//! The tester program's report upload.
//!
//! The tester journal in the interface records what the tester did and what
//! failed, collects the tester's notes and pictures, and hands them to
//! [`send_tester_report`]. This module checks the account is enrolled, removes
//! anything that names the person or the machine a second time (the interface
//! does it first), checks sizes and picture types the way the auth service
//! does, and uploads the report with the session's own token, which never
//! reaches the interface. The auth service forwards it to the founder's chat.

use crate::auth_session::{self, AuthState};
use base64::Engine;
use regex::Regex;
use reqwest::blocking::multipart::{Form, Part};
use serde::Serialize;
use serde_json::Value;
use std::sync::LazyLock;
use std::time::Duration;
use tauri::AppHandle;

// The same bounds as services/auth-worker/src/tester.mjs.
const MAX_IMAGES: usize = 6;
const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;
const MAX_REPORT_BYTES: usize = 512 * 1024;
/// Six pictures on a slow connection; the regular auth timeout is 12 seconds.
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(120);
const REDACTED: &str = "•••";

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TesterReportReceipt {
    pub report_id: String,
}

#[derive(Debug, PartialEq, Eq)]
struct Picture {
    bytes: Vec<u8>,
    mime: &'static str,
    file_name: &'static str,
}

static WINDOWS_PROFILE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)\b[a-z]:[\\/]+users[\\/]+[^\\/"\r\n]+"#).expect("pattern"));
static UNIX_HOME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"/(?:Users|home)/[^/"\r\n]+"#).expect("pattern"));
static EMAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}")
        .expect("pattern")
});
/// Steam IDs, phone numbers and other long numbers.
static LONG_NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d{9,}").expect("pattern"));

fn redact_text(text: &str) -> String {
    let text = WINDOWS_PROFILE.replace_all(text, "%USERPROFILE%");
    let text = UNIX_HOME.replace_all(&text, "~");
    let text = EMAIL.replace_all(&text, "•••@•••");
    LONG_NUMBER.replace_all(&text, REDACTED).into_owned()
}

/// Every string in the report, and every number long enough to be an ID.
fn redact(value: &mut Value) {
    match value {
        Value::String(text) => *text = redact_text(text),
        Value::Number(number) => {
            let large = number
                .as_f64()
                .is_some_and(|value| value.abs() >= 1_000_000_000.0);
            if large {
                *value = Value::String(REDACTED.to_string());
            }
        }
        Value::Array(items) => items.iter_mut().for_each(redact),
        Value::Object(fields) => fields.values_mut().for_each(redact),
        Value::Bool(_) | Value::Null => {}
    }
}

fn picture(base64_text: &str) -> Result<Picture, String> {
    if base64_text.len() > MAX_IMAGE_BYTES * 4 / 3 + 8 {
        return Err("tester_image_too_large".to_string());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_text)
        .map_err(|_| "tester_image_invalid".to_string())?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err("tester_image_too_large".to_string());
    }
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Ok(Picture {
            bytes,
            mime: "image/png",
            file_name: "picture.png",
        })
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Ok(Picture {
            bytes,
            mime: "image/jpeg",
            file_name: "picture.jpg",
        })
    } else {
        Err("tester_image_invalid".to_string())
    }
}

/// The report as it will be sent, and its pictures.
fn prepare(report: &str, images: &[String]) -> Result<(String, Vec<Picture>), String> {
    if report.len() > MAX_REPORT_BYTES * 4 || images.len() > MAX_IMAGES {
        return Err("tester_report_too_large".to_string());
    }
    let mut value: Value =
        serde_json::from_str(report).map_err(|_| "tester_report_invalid".to_string())?;
    let listed = value.get("images").and_then(Value::as_array).map(Vec::len);
    if value.get("schema") != Some(&Value::from(1)) || listed != Some(images.len()) {
        return Err("tester_report_invalid".to_string());
    }
    redact(&mut value);
    let text = serde_json::to_string(&value).map_err(|_| "tester_report_invalid".to_string())?;
    if text.len() > MAX_REPORT_BYTES {
        return Err("tester_report_too_large".to_string());
    }
    let pictures = images
        .iter()
        .map(|image| picture(image))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((text, pictures))
}

fn form(report: &str, pictures: &[Picture]) -> Result<Form, String> {
    let report_part = Part::text(report.to_string())
        .mime_str("application/json")
        .map_err(|_| "tester_report_invalid".to_string())?;
    pictures
        .iter()
        .try_fold(Form::new().part("report", report_part), |form, picture| {
            Part::bytes(picture.bytes.clone())
                .file_name(picture.file_name)
                .mime_str(picture.mime)
                .map(|part| form.part("image", part))
                .map_err(|_| "tester_image_invalid".to_string())
        })
}

fn upload_error(status: u16) -> &'static str {
    match status {
        401 => "auth_session_expired",
        403 => "tester_not_enrolled",
        400 => "tester_report_invalid",
        413 => "tester_report_too_large",
        429 => "tester_rate_limited",
        502 | 503 => "tester_delivery_failed",
        _ => "tester_service_unavailable",
    }
}

fn send_blocking(
    state: &AuthState,
    report: String,
    images: Vec<String>,
) -> Result<TesterReportReceipt, String> {
    let (report, pictures) = prepare(&report, &images)?;
    if !auth_session::auth_profile_blocking(state)?.tester {
        return Err("tester_not_enrolled".to_string());
    }
    // Built once here so a malformed part fails before any request.
    form(&report, &pictures)?;
    let response = auth_session::authenticated_request_with(
        state,
        reqwest::Method::POST,
        "/v1/tester/reports",
        |request| {
            let body = form(&report, &pictures).expect("checked above");
            request.timeout(UPLOAD_TIMEOUT).multipart(body)
        },
    )?;
    if !response.status().is_success() {
        return Err(upload_error(response.status().as_u16()).to_string());
    }
    let payload = response
        .json::<Value>()
        .map_err(|_| "tester_service_unavailable".to_string())?;
    let report_id = payload
        .get("reportId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= 64)
        .ok_or_else(|| "tester_service_unavailable".to_string())?;
    Ok(TesterReportReceipt {
        report_id: report_id.to_string(),
    })
}

/// Sends the tester's report. Only an enrolled account may; the auth service
/// checks the same again.
#[tauri::command]
pub async fn send_tester_report(
    app: AppHandle,
    report: String,
    images: Vec<String>,
) -> Result<TesterReportReceipt, String> {
    auth_session::run_blocking(app, move |state| send_blocking(state, report, images)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0];
    const JPEG: &[u8] = &[0xff, 0xd8, 0xff, 0xe0];

    fn encode(bytes: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    fn report(images: usize, events: Value) -> String {
        serde_json::json!({
            "schema": 1,
            "appVersion": "0.6.0",
            "images": (0..images).map(|_| serde_json::json!({ "kind": "snapshot", "note": null })).collect::<Vec<_>>(),
            "events": events,
        })
        .to_string()
    }

    #[test]
    fn names_of_the_person_and_machine_are_removed() {
        assert_eq!(
            redact_text(r"C:\Users\Иван Петров\AppData\Roaming\app.betterfy.desktop"),
            r"%USERPROFILE%\AppData\Roaming\app.betterfy.desktop"
        );
        assert_eq!(
            redact_text("d:/USERS/zori/Desktop"),
            "%USERPROFILE%/Desktop"
        );
        assert_eq!(redact_text("/Users/zori/Library"), "~/Library");
        assert_eq!(
            redact_text("write to player.one@example.com now"),
            "write to •••@••• now"
        );
        assert_eq!(redact_text("steam 76561198000000000"), "steam •••");
        assert_eq!(
            redact_text("2026-10-19T17:14:00Z · build 26100 · 0.6.0"),
            "2026-10-19T17:14:00Z · build 26100 · 0.6.0",
            "times, builds and versions stay"
        );
    }

    #[test]
    fn the_whole_report_is_redacted_before_it_leaves() {
        let events = serde_json::json!([
            { "type": "failure", "detail": r"C:\Users\zori\x", "account": 76561198000000000u64, "ms": 1834 },
        ]);
        let (text, pictures) = prepare(&report(1, events), &[encode(PNG)]).expect("prepared");
        assert!(!text.contains("zori"), "{text}");
        assert!(!text.contains("76561198000000000"), "{text}");
        assert!(text.contains("\"ms\":1834"));
        assert_eq!(pictures[0].mime, "image/png");
    }

    #[test]
    fn malformed_reports_and_pictures_are_refused() {
        let events = serde_json::json!([]);
        assert_eq!(
            prepare("{", &[]).err().as_deref(),
            Some("tester_report_invalid")
        );
        assert_eq!(
            prepare(&report(1, events.clone()), &[]).err().as_deref(),
            Some("tester_report_invalid"),
            "a listed picture is missing"
        );
        assert_eq!(
            prepare(&report(1, events.clone()), &[encode(b"GIF89a")])
                .err()
                .as_deref(),
            Some("tester_image_invalid")
        );
        assert_eq!(
            prepare(
                &report(1, events.clone()),
                &[encode(&vec![0xff; MAX_IMAGE_BYTES + 1])]
            )
            .err()
            .as_deref(),
            Some("tester_image_too_large")
        );
        assert_eq!(
            prepare(&report(7, events.clone()), &vec![encode(JPEG); 7])
                .err()
                .as_deref(),
            Some("tester_report_too_large")
        );
        let huge = serde_json::json!(vec!["x".repeat(1000); 600]);
        assert_eq!(
            prepare(&report(0, huge), &[]).err().as_deref(),
            Some("tester_report_too_large")
        );
        let (_, pictures) =
            prepare(&report(2, events), &[encode(PNG), encode(JPEG)]).expect("two pictures");
        assert_eq!(pictures[1].file_name, "picture.jpg");
        assert!(form("{}", &pictures).is_ok());
    }

    #[test]
    fn service_answers_map_to_interface_codes() {
        assert_eq!(upload_error(403), "tester_not_enrolled");
        assert_eq!(upload_error(429), "tester_rate_limited");
        assert_eq!(upload_error(502), "tester_delivery_failed");
        assert_eq!(upload_error(500), "tester_service_unavailable");
    }
}

//! Retained Windows-test evidence reports.
//!
//! Each saved report is a new, pretty-printed JSON file under
//! `<app_data>/reports/`. A report is written to a temporary file in the same
//! directory, synced, and then published under its final name with a
//! hard link, which fails instead of replacing an existing file. Only the
//! most recent `MAX_REPORTS` files (by name, which sorts by UTC timestamp)
//! are kept.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_REPORTS: usize = 50;
const REPORT_PREFIX: &str = "evidence-";
const REPORT_SUFFIX: &str = ".json";
const MAX_NAME_ATTEMPTS: usize = 100;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SavedEvidence {
    pub file_name: String,
    pub entries: usize,
}

fn reject_symlink(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err("report_path_invalid".to_string()),
        _ => Ok(()),
    }
}

pub fn reports_dir(app_data_root: &Path) -> Result<PathBuf, String> {
    reject_symlink(app_data_root)?;
    fs::create_dir_all(app_data_root).map_err(|_| "report_write_failed".to_string())?;
    let dir = app_data_root.join("reports");
    reject_symlink(&dir)?;
    fs::create_dir_all(&dir).map_err(|_| "report_write_failed".to_string())?;
    reject_symlink(&dir)?;
    Ok(dir)
}

pub fn save_report<T: Serialize>(
    app_data_root: &Path,
    report: &T,
    entries: usize,
    now: SystemTime,
) -> Result<SavedEvidence, String> {
    let dir = reports_dir(app_data_root)?;
    let mut bytes =
        serde_json::to_vec_pretty(report).map_err(|_| "report_serialize_failed".to_string())?;
    bytes.push(b'\n');
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let stem = format!("{REPORT_PREFIX}{}-{}", utc_stamp(now), &digest[..8]);

    let temp = dir.join(format!(
        ".{stem}-{}-{}.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    if let Err(code) = write_synced(&temp, &bytes) {
        let _ = fs::remove_file(&temp);
        return Err(code);
    }
    let published = publish_without_overwrite(&dir, &stem, &temp);
    let _ = fs::remove_file(&temp);
    let file_name = published?;
    prune(&dir, MAX_REPORTS);
    Ok(SavedEvidence { file_name, entries })
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|_| "report_write_failed".to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "report_write_failed".to_string())
}

fn publish_without_overwrite(dir: &Path, stem: &str, temp: &Path) -> Result<String, String> {
    for attempt in 0..MAX_NAME_ATTEMPTS {
        let name = if attempt == 0 {
            format!("{stem}{REPORT_SUFFIX}")
        } else {
            format!("{stem}-{attempt}{REPORT_SUFFIX}")
        };
        let target = dir.join(&name);
        match fs::hard_link(temp, &target) {
            Ok(()) => return Ok(name),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(_) => {
                // File systems without hard links: fall back to a rename,
                // still refusing to replace an existing report.
                if fs::symlink_metadata(&target).is_ok() {
                    continue;
                }
                return fs::rename(temp, &target)
                    .map(|_| name)
                    .map_err(|_| "report_write_failed".to_string());
            }
        }
    }
    Err("report_write_failed".to_string())
}

fn prune(dir: &Path, keep: usize) {
    let Ok(listing) = fs::read_dir(dir) else {
        return;
    };
    let mut reports = listing
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with(REPORT_PREFIX) && name.ends_with(REPORT_SUFFIX))
        .collect::<Vec<_>>();
    if reports.len() <= keep {
        return;
    }
    reports.sort();
    let excess = reports.len() - keep;
    for name in reports.into_iter().take(excess) {
        let _ = fs::remove_file(dir.join(name));
    }
}

fn utc_stamp(now: SystemTime) -> String {
    let seconds = now
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    let rest = seconds % 86_400;
    format!(
        "{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z",
        rest / 3_600,
        rest % 3_600 / 60,
        rest % 60
    )
}

/// Days since 1970-01-01 to a proleptic Gregorian (year, month, day).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "betterfy-reports-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn at(seconds: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(seconds)
    }

    #[test]
    fn utc_stamps_match_known_dates() {
        assert_eq!(utc_stamp(at(0)), "19700101T000000Z");
        assert_eq!(utc_stamp(at(951_868_800)), "20000301T000000Z");
        assert_eq!(utc_stamp(at(1_709_210_096)), "20240229T123456Z");
        assert_eq!(utc_stamp(at(4_102_444_799)), "20991231T235959Z");
    }

    #[test]
    fn saves_pretty_json_and_never_overwrites_an_existing_report() {
        let root = temp_root("no-overwrite");
        let report = serde_json::json!({ "schemaVersion": 1, "entries": [] });
        let first = save_report(&root, &report, 0, at(1_709_210_096)).expect("first save");
        assert!(first.file_name.starts_with("evidence-20240229T123456Z-"));
        assert!(first.file_name.ends_with(".json"));
        let dir = root.join("reports");
        let first_path = dir.join(&first.file_name);
        fs::write(&first_path, b"founder notes").expect("simulate edited report");

        let second = save_report(&root, &report, 0, at(1_709_210_096)).expect("second save");
        assert_ne!(first.file_name, second.file_name);
        assert_eq!(fs::read(&first_path).expect("first kept"), b"founder notes");
        let text = fs::read_to_string(dir.join(&second.file_name)).expect("second");
        assert!(text.contains("\n  \"schemaVersion\": 1"));
        let leftovers = fs::read_dir(&dir)
            .expect("list")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn keeps_only_the_most_recent_reports() {
        let root = temp_root("retention");
        let dir = reports_dir(&root).expect("dir");
        for index in 0..(MAX_REPORTS + 5) {
            fs::write(
                dir.join(format!("evidence-20200101T00{index:04}Z-0.json")),
                b"{}",
            )
            .expect("seed");
        }
        fs::write(dir.join("notes.txt"), b"keep").expect("unrelated file");
        let saved = save_report(&root, &serde_json::json!({}), 0, at(1_709_210_096)).expect("save");
        let mut names = fs::read_dir(&dir)
            .expect("list")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().into_string().expect("utf8"))
            .filter(|name| name.starts_with(REPORT_PREFIX))
            .collect::<Vec<_>>();
        names.sort();
        assert_eq!(names.len(), MAX_REPORTS);
        assert_eq!(names.last(), Some(&saved.file_name));
        assert!(!names.contains(&"evidence-20200101T000005Z-0.json".to_string()));
        assert!(names.contains(&"evidence-20200101T000006Z-0.json".to_string()));
        assert!(dir.join("notes.txt").exists());
        let _ = fs::remove_dir_all(root);
    }
}

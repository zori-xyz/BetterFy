//! Signed remote package catalog.
//!
//! The catalog is `index.json` plus `index.json.sig`, produced by
//! `npm run catalog:publish` and signed in the minisign format of
//! `tauri signer` with a key that is separate from the updater key. The
//! desktop accepts a catalog only when:
//!
//! - the signature over the exact bytes verifies against the embedded public
//!   key;
//! - its `sequence` is not lower than the last catalog accepted on this device
//!   (a replayed older catalog cannot roll packages back);
//! - it has not expired and was not issued in the future;
//! - every manifest passes the same validation as the embedded manifests,
//!   which also pins the source repository, so even a leaked signing key cannot
//!   point BetterFy at another host or repository.
//!
//! An accepted catalog is cached in app data together with its signature and
//! re-verified on every load. Without a usable catalog the embedded manifests
//! stay active; a network or signature failure never removes a package.

use crate::package_registry::{self, PackageSummary};
use base64::Engine;
use minisign_verify::{PublicKey, Signature};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CATALOG_URL: &str = "https://betterfy-auth.zori-xyz.workers.dev/catalog/index.json";
/// Catalog signing public key (key ID 5847CC846725027), not the updater key.
const CATALOG_PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDU4NDdDQzg0NjcyNTAyNwpSV1FuVUhKR3lIeUVCZFk3RmVkMnhpUDAwSkh2eEx3Sy9LTG5zeHFab3Y4UVZMbk1VWXIwOTNQRAo=";
const MAX_INDEX_BYTES: usize = 1024 * 1024;
const MAX_SIGNATURE_BYTES: usize = 4 * 1024;
/// Tolerated clock skew for `issuedAt` in the future.
const MAX_CLOCK_SKEW_SECONDS: i64 = 24 * 60 * 60;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CatalogIndex {
    schema_version: u32,
    sequence: u64,
    issued_at: String,
    expires_at: String,
    packages: Vec<serde_json::Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogStatus {
    /// `embedded`, `cache` or `remote`: where the active package set came from.
    source: &'static str,
    sequence: Option<u64>,
    packages: Vec<PackageSummary>,
    /// Why the remote catalog was not used this time, if it was not.
    error: Option<String>,
}

fn decode_base64_text(value: &str) -> Result<String, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value.trim())
        .map_err(|_| "catalog_signature_invalid".to_string())?;
    String::from_utf8(bytes).map_err(|_| "catalog_signature_invalid".to_string())
}

fn verify_signature(index: &[u8], signature_b64: &str, public_key_b64: &str) -> Result<(), String> {
    let public_key = PublicKey::decode(&decode_base64_text(public_key_b64)?)
        .map_err(|_| "catalog_key_invalid".to_string())?;
    let signature = Signature::decode(&decode_base64_text(signature_b64)?)
        .map_err(|_| "catalog_signature_invalid".to_string())?;
    public_key
        .verify(index, &signature, false)
        .map_err(|_| "catalog_signature_invalid".to_string())
}

/// `YYYY-MM-DDTHH:MM:SSZ` to Unix seconds. Only the exact UTC form written by
/// the publishing script is accepted.
fn parse_utc(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return None;
    }
    let number = |range: std::ops::Range<usize>| value.get(range)?.parse::<i64>().ok();
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    // Days from civil date (proleptic Gregorian), Howard Hinnant's algorithm.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// Everything except the signature: schema, freshness, anti-rollback and
/// manifest validation. Returns the merged package set to activate.
fn accept(
    index: &CatalogIndex,
    last_sequence: Option<u64>,
    now: i64,
) -> Result<Vec<package_registry::PackageManifest>, String> {
    if index.schema_version != 1 || index.sequence == 0 {
        return Err("catalog_invalid".to_string());
    }
    if last_sequence.is_some_and(|last| index.sequence < last) {
        return Err("catalog_rollback".to_string());
    }
    let issued = parse_utc(&index.issued_at).ok_or_else(|| "catalog_invalid".to_string())?;
    let expires = parse_utc(&index.expires_at).ok_or_else(|| "catalog_invalid".to_string())?;
    if issued > now + MAX_CLOCK_SKEW_SECONDS || expires <= issued {
        return Err("catalog_invalid".to_string());
    }
    if expires <= now {
        return Err("catalog_expired".to_string());
    }
    if index.packages.is_empty() {
        return Err("catalog_invalid".to_string());
    }
    package_registry::merge_remote(&index.packages)
}

fn parse_and_accept(
    index_bytes: &[u8],
    signature: &str,
    last_sequence: Option<u64>,
    now: i64,
) -> Result<(CatalogIndex, Vec<package_registry::PackageManifest>), String> {
    if index_bytes.len() > MAX_INDEX_BYTES || signature.len() > MAX_SIGNATURE_BYTES {
        return Err("catalog_too_large".to_string());
    }
    verify_signature(index_bytes, signature, CATALOG_PUBLIC_KEY)?;
    let index: CatalogIndex =
        serde_json::from_slice(index_bytes).map_err(|_| "catalog_invalid".to_string())?;
    let merged = accept(&index, last_sequence, now)?;
    Ok((index, merged))
}

fn cache_dir(app_data_root: &Path) -> Result<PathBuf, String> {
    let dir = app_data_root.join("engine-v1").join("catalog");
    for path in [app_data_root.join("engine-v1"), dir.clone()] {
        if fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return Err("catalog_cache_invalid".to_string());
        }
    }
    fs::create_dir_all(&dir).map_err(|_| "catalog_cache_invalid".to_string())?;
    Ok(dir)
}

fn read_bounded(path: &Path, limit: usize) -> Option<Vec<u8>> {
    let meta = fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || meta.len() > limit as u64 {
        return None;
    }
    fs::read(path).ok()
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension("tmp");
    let _ = fs::remove_file(&temporary);
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|_| "catalog_cache_invalid".to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "catalog_cache_invalid".to_string())?;
    drop(file);
    fs::rename(&temporary, path).map_err(|_| {
        let _ = fs::remove_file(&temporary);
        "catalog_cache_invalid".to_string()
    })
}

struct Cached {
    index: Vec<u8>,
    signature: String,
}

fn read_cache(dir: &Path) -> Option<Cached> {
    Some(Cached {
        index: read_bounded(&dir.join("index.json"), MAX_INDEX_BYTES)?,
        signature: String::from_utf8(read_bounded(
            &dir.join("index.json.sig"),
            MAX_SIGNATURE_BYTES,
        )?)
        .ok()?,
    })
}

/// The signature is written first: a crash between the two writes leaves a
/// pair that fails verification and is ignored, never a trusted mismatch.
fn write_cache(dir: &Path, index: &[u8], signature: &str) -> Result<(), String> {
    write_atomic(&dir.join("index.json.sig"), signature.as_bytes())?;
    write_atomic(&dir.join("index.json"), index)
}

fn fetch_bounded(client: &Client, url: &str, limit: usize) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .send()
        .map_err(|_| "catalog_unavailable".to_string())?;
    if !response.status().is_success() {
        return Err("catalog_unavailable".to_string());
    }
    let mut bytes = Vec::new();
    response
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "catalog_unavailable".to_string())?;
    if bytes.len() > limit {
        return Err("catalog_too_large".to_string());
    }
    Ok(bytes)
}

fn fetch_remote() -> Result<(Vec<u8>, String), String> {
    let client = Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .user_agent("BetterFy-catalog/1")
        .build()
        .map_err(|_| "catalog_unavailable".to_string())?;
    let index = fetch_bounded(&client, CATALOG_URL, MAX_INDEX_BYTES)?;
    let signature = fetch_bounded(&client, &format!("{CATALOG_URL}.sig"), MAX_SIGNATURE_BYTES)?;
    let signature =
        String::from_utf8(signature).map_err(|_| "catalog_signature_invalid".to_string())?;
    Ok((index, signature))
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

fn status(source: &'static str, sequence: Option<u64>, error: Option<String>) -> CatalogStatus {
    CatalogStatus {
        source,
        sequence,
        packages: package_registry::summaries().unwrap_or_default(),
        error,
    }
}

/// Activates the cached catalog if it still verifies, then tries the remote
/// one. Every failure leaves the previously active set in place.
pub fn refresh(app_data_root: &Path) -> CatalogStatus {
    let now = now_seconds();
    let dir = match cache_dir(app_data_root) {
        Ok(dir) => dir,
        Err(error) => return status("embedded", None, Some(error)),
    };
    let mut source = "embedded";
    let mut sequence = None;
    if let Some(cached) = read_cache(&dir) {
        if let Ok((index, merged)) = parse_and_accept(&cached.index, &cached.signature, None, now) {
            if package_registry::activate(merged).is_ok() {
                source = "cache";
                sequence = Some(index.sequence);
            }
        }
    }
    let remote = fetch_remote().and_then(|(bytes, signature)| {
        let (index, merged) = parse_and_accept(&bytes, &signature, sequence, now)?;
        if Some(index.sequence) != sequence {
            write_cache(&dir, &bytes, &signature)?;
        }
        package_registry::activate(merged)?;
        Ok(index.sequence)
    });
    match remote {
        Ok(accepted) => status("remote", Some(accepted), None),
        Err(error) => status(source, sequence, Some(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUBLISHED_INDEX: &[u8] = include_bytes!("../../website/public/bot/catalog/index.json");
    const PUBLISHED_SIGNATURE: &str =
        include_str!("../../website/public/bot/catalog/index.json.sig");
    const UPDATER_PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDI5RjYxQTkzNTQxRDZFOTAKUldTUWJoMVVreHIyS2ZaRWFUVllzVGVUUElzM3UzU3pROXRRM0RsUGVzSGRUNXJBOWY1Q041c2UK";

    fn index(sequence: u64, issued: &str, expires: &str) -> CatalogIndex {
        let published: serde_json::Value =
            serde_json::from_slice(PUBLISHED_INDEX).expect("published index");
        CatalogIndex {
            schema_version: 1,
            sequence,
            issued_at: issued.to_string(),
            expires_at: expires.to_string(),
            packages: published["packages"].as_array().expect("packages").clone(),
        }
    }

    #[test]
    fn the_published_catalog_is_signed_by_the_catalog_key() {
        verify_signature(PUBLISHED_INDEX, PUBLISHED_SIGNATURE, CATALOG_PUBLIC_KEY)
            .expect("published catalog must verify; run `npm run catalog:publish`");
        let parsed: CatalogIndex = serde_json::from_slice(PUBLISHED_INDEX).expect("schema");
        assert_eq!(parsed.schema_version, 1);
        package_registry::merge_remote(&parsed.packages).expect("published manifests are valid");
    }

    #[test]
    fn tampering_or_a_different_key_is_rejected() {
        let mut tampered = PUBLISHED_INDEX.to_vec();
        let position = tampered
            .iter()
            .position(|byte| *byte == b'1')
            .expect("digit");
        tampered[position] = b'2';
        assert_eq!(
            verify_signature(&tampered, PUBLISHED_SIGNATURE, CATALOG_PUBLIC_KEY)
                .err()
                .as_deref(),
            Some("catalog_signature_invalid")
        );
        assert!(
            verify_signature(PUBLISHED_INDEX, PUBLISHED_SIGNATURE, UPDATER_PUBLIC_KEY).is_err()
        );
        assert!(verify_signature(PUBLISHED_INDEX, "not base64!", CATALOG_PUBLIC_KEY).is_err());
    }

    #[test]
    fn freshness_and_rollback_rules() {
        let now = parse_utc("2026-10-01T00:00:00Z").unwrap();
        let ok = index(5, "2026-09-30T00:00:00Z", "2027-03-01T00:00:00Z");
        assert!(
            accept(&ok, Some(5), now).is_ok(),
            "same sequence is a re-download"
        );
        assert!(accept(&ok, Some(4), now).is_ok());
        assert_eq!(
            accept(&ok, Some(6), now).err().as_deref(),
            Some("catalog_rollback")
        );
        let expired = index(5, "2026-01-01T00:00:00Z", "2026-06-01T00:00:00Z");
        assert_eq!(
            accept(&expired, None, now).err().as_deref(),
            Some("catalog_expired")
        );
        let future = index(5, "2026-12-01T00:00:00Z", "2027-06-01T00:00:00Z");
        assert_eq!(
            accept(&future, None, now).err().as_deref(),
            Some("catalog_invalid")
        );
        let zero = index(0, "2026-09-30T00:00:00Z", "2027-03-01T00:00:00Z");
        assert!(accept(&zero, None, now).is_err());
    }

    #[test]
    fn remote_manifests_cannot_leave_the_trusted_repository_or_collide() {
        let now = parse_utc("2026-10-01T00:00:00Z").unwrap();
        let mut hostile = index(2, "2026-09-30T00:00:00Z", "2027-03-01T00:00:00Z");
        hostile.packages[0]["source"]["repository"] = "attacker/mods".into();
        assert!(accept(&hostile, None, now).is_err());
        let mut colliding = index(2, "2026-09-30T00:00:00Z", "2027-03-01T00:00:00Z");
        colliding.packages[0]["id"] = "minify.renamed".into();
        assert_eq!(
            accept(&colliding, None, now).err().as_deref(),
            Some("package_manifest_invalid:catalog_id_conflict")
        );
    }

    #[test]
    fn utc_parsing_is_strict() {
        assert_eq!(parse_utc("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_utc("2000-03-01T00:00:00Z"), Some(951_868_800));
        assert_eq!(parse_utc("2026-09-30T17:15:00.000Z"), None);
        assert_eq!(parse_utc("2026-13-01T00:00:00Z"), None);
        assert_eq!(parse_utc("2026-09-30 17:15:00Z"), None);
    }

    #[test]
    fn a_cache_pair_that_does_not_verify_is_ignored() {
        let root = std::env::temp_dir().join(format!(
            "betterfy-catalog-cache-{}-{}",
            std::process::id(),
            now_seconds()
        ));
        let dir = cache_dir(&root).expect("cache dir");
        write_cache(&dir, PUBLISHED_INDEX, PUBLISHED_SIGNATURE).expect("write");
        let cached = read_cache(&dir).expect("read back");
        assert_eq!(cached.index, PUBLISHED_INDEX);
        fs::write(dir.join("index.json"), b"{}").expect("corrupt");
        let corrupted = read_cache(&dir).expect("read corrupted");
        assert!(
            parse_and_accept(&corrupted.index, &corrupted.signature, None, now_seconds()).is_err()
        );
        let _ = fs::remove_dir_all(root);
    }
}

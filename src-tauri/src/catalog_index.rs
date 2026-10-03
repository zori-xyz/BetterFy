//! Signed remote package catalog.
//!
//! The catalog is `index.json` plus `index.json.sig`, produced by
//! `npm run catalog:publish` and signed in the minisign format of
//! `tauri signer` with a key that is separate from the updater key. The
//! desktop accepts a remote catalog only when:
//!
//! - the signature over the exact bytes verifies against the embedded public
//!   key;
//! - its `sequence` is at least the highest one this device has seen and at
//!   least the one committed with this build (`BETTERFY_MIN_CATALOG_SEQUENCE`),
//!   and a reused sequence carries the same bytes;
//! - it is unexpired, not issued in the future, and valid for at most 200 days;
//! - every manifest passes the embedded validation (trusted repository, pinned
//!   commit, data-only extensions, bounded sizes) and no package ID changes its
//!   contract: neither a package this build ships with nor one accepted
//!   earlier on this device.
//!
//! A leaked signing key is therefore limited to adding packages built from
//! data files at some commit reachable through the trusted repository path
//! (GitHub may also serve fork commits there), and to metadata. It cannot
//! change what an existing package ID installs, write outside the language
//! folder, or use another host.
//!
//! An accepted catalog is cached with its signature and re-verified on load.
//! An expired cached catalog stays usable for packages it already provided, so
//! installed builds stay verifiable offline. Every failure keeps the current
//! set; a catalog never removes a package.

use crate::package_registry::{self, PackageSummary};
use base64::Engine;
use minisign_verify::{PublicKey, Signature};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CATALOG_URL: &str = "https://betterfy-auth.zori-xyz.workers.dev/catalog/index.json";
/// Catalog signing public key (key ID 5847CC846725027), not the updater key.
const CATALOG_PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDU4NDdDQzg0NjcyNTAyNwpSV1FuVUhKR3lIeUVCZFk3RmVkMnhpUDAwSkh2eEx3Sy9LTG5zeHFab3Y4UVZMbk1VWXIwOTNQRAo=";
const MAX_INDEX_BYTES: usize = 1024 * 1024;
const MAX_SIGNATURE_BYTES: usize = 4 * 1024;
/// Tolerated clock skew for `issuedAt` in the future.
const MAX_CLOCK_SKEW_SECONDS: i64 = 24 * 60 * 60;
const MAX_VALIDITY_SECONDS: i64 = 200 * 24 * 60 * 60;
/// A single catalog cannot jump the sequence far enough to lock out every
/// later legitimate catalog.
const MAX_SEQUENCE_JUMP: u64 = 1_000_000;

static REFRESH_LOCK: Mutex<()> = Mutex::new(());
/// Sequence and byte hash of the catalog currently active in this process.
static ACTIVE: Mutex<Option<(u64, String)>> = Mutex::new(None);

fn min_sequence() -> u64 {
    env!("BETTERFY_MIN_CATALOG_SEQUENCE").parse().unwrap_or(1)
}

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

/// Signature, size and schema. Freshness and sequence are checked separately
/// because a cached catalog is still a valid baseline after it expires.
fn verify_and_parse(index_bytes: &[u8], signature: &str) -> Result<CatalogIndex, String> {
    if index_bytes.len() > MAX_INDEX_BYTES || signature.len() > MAX_SIGNATURE_BYTES {
        return Err("catalog_too_large".to_string());
    }
    verify_signature(index_bytes, signature, CATALOG_PUBLIC_KEY)?;
    let index: CatalogIndex =
        serde_json::from_slice(index_bytes).map_err(|_| "catalog_invalid".to_string())?;
    if index.schema_version != 1 || index.sequence == 0 || index.packages.is_empty() {
        return Err("catalog_invalid".to_string());
    }
    Ok(index)
}

fn check_time(index: &CatalogIndex, now: i64, allow_expired: bool) -> Result<(), String> {
    let issued = parse_utc(&index.issued_at).ok_or_else(|| "catalog_invalid".to_string())?;
    let expires = parse_utc(&index.expires_at).ok_or_else(|| "catalog_invalid".to_string())?;
    if issued > now + MAX_CLOCK_SKEW_SECONDS
        || expires <= issued
        || expires - issued > MAX_VALIDITY_SECONDS
    {
        return Err("catalog_invalid".to_string());
    }
    if !allow_expired && expires <= now {
        return Err("catalog_expired".to_string());
    }
    Ok(())
}

fn check_sequence(sequence: u64, baseline: u64) -> Result<(), String> {
    if sequence < baseline {
        return Err("catalog_rollback".to_string());
    }
    if sequence > baseline.saturating_add(MAX_SEQUENCE_JUMP) {
        return Err("catalog_invalid".to_string());
    }
    Ok(())
}

/// A package ID keeps the contract it had when this device first accepted it.
fn check_contracts(
    merged: &[package_registry::PackageManifest],
    known: &BTreeMap<String, String>,
) -> Result<(), String> {
    for manifest in merged {
        if known
            .get(&manifest.id)
            .is_some_and(|hash| *hash != manifest.contract_hash())
        {
            return Err("catalog_contract_changed".to_string());
        }
    }
    Ok(())
}

fn byte_hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
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
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
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

fn read_contracts(dir: &Path) -> BTreeMap<String, String> {
    read_bounded(&dir.join("contracts.json"), MAX_INDEX_BYTES)
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_contracts(
    dir: &Path,
    known: &mut BTreeMap<String, String>,
    merged: &[package_registry::PackageManifest],
) -> Result<(), String> {
    let before = known.len();
    for manifest in merged {
        known
            .entry(manifest.id.clone())
            .or_insert_with(|| manifest.contract_hash());
    }
    if known.len() == before {
        return Ok(());
    }
    let bytes =
        serde_json::to_vec_pretty(known).map_err(|_| "catalog_cache_invalid".to_string())?;
    write_atomic(&dir.join("contracts.json"), &bytes)
}

/// Activates a set unless the same catalog, or a newer one, is already active
/// in this process. Repeated refreshes do not re-activate (or leak) anything.
fn activate_if_newer(
    sequence: u64,
    hash: String,
    merged: Vec<package_registry::PackageManifest>,
) -> Result<(), String> {
    let mut active = ACTIVE
        .lock()
        .map_err(|_| "package_registry_invalid".to_string())?;
    if active
        .as_ref()
        .is_some_and(|(current, current_hash)| *current > sequence || *current_hash == hash)
    {
        return Ok(());
    }
    package_registry::activate(merged)?;
    *active = Some((sequence, hash));
    Ok(())
}

/// Decides whether a fetched catalog may replace the current one. `cached`
/// is the verified cached catalog's sequence and byte hash, if any.
fn evaluate_remote(
    bytes: &[u8],
    signature: &str,
    baseline: u64,
    cached: Option<&(u64, String)>,
    known: &BTreeMap<String, String>,
    now: i64,
) -> Result<(CatalogIndex, String, Vec<package_registry::PackageManifest>), String> {
    let index = verify_and_parse(bytes, signature)?;
    check_sequence(index.sequence, baseline)?;
    let hash = byte_hash(bytes);
    // A reused sequence is the cached catalog's own number with different
    // bytes. The baseline also includes this build's floor, which a cache
    // from an older catalog sits below; comparing with the baseline would
    // reject the legitimate catalog published at the floor.
    if cached.is_some_and(|(cached_sequence, cached_hash)| {
        *cached_sequence == index.sequence && *cached_hash != hash
    }) {
        return Err("catalog_sequence_reused".to_string());
    }
    check_time(&index, now, false)?;
    let merged = package_registry::merge_remote(&index.packages)?;
    check_contracts(&merged, known)?;
    Ok((index, hash, merged))
}

/// Activates the cached catalog if it still verifies, then tries the remote
/// one. Serialized per process; every failure leaves the current set in place.
pub fn refresh(app_data_root: &Path) -> CatalogStatus {
    let Ok(_guard) = REFRESH_LOCK.lock() else {
        return status("embedded", None, Some("catalog_busy".to_string()));
    };
    let now = now_seconds();
    let dir = match cache_dir(app_data_root) {
        Ok(dir) => dir,
        Err(error) => return status("embedded", None, Some(error)),
    };
    let mut known = read_contracts(&dir);
    let mut baseline = min_sequence();
    let mut cached_catalog: Option<(u64, String)> = None;
    let mut source = "embedded";
    let mut sequence = None;
    if let Some(cached) = read_cache(&dir) {
        if let Ok(index) = verify_and_parse(&cached.index, &cached.signature) {
            // A verified cached catalog raises the baseline even if it can no
            // longer be used, so losing usability never re-opens rollback.
            baseline = baseline.max(index.sequence);
            let hash = byte_hash(&cached.index);
            let usable = (index.sequence >= min_sequence()
                && check_time(&index, now, true).is_ok())
            .then(|| package_registry::merge_remote(&index.packages).ok())
            .flatten()
            .filter(|merged| check_contracts(merged, &known).is_ok());
            if let Some(merged) = usable {
                if activate_if_newer(index.sequence, hash.clone(), merged).is_ok() {
                    source = "cache";
                    sequence = Some(index.sequence);
                }
            }
            cached_catalog = Some((index.sequence, hash));
        }
    }
    let remote = fetch_remote().and_then(|(bytes, signature)| {
        let (index, hash, merged) = evaluate_remote(
            &bytes,
            &signature,
            baseline,
            cached_catalog.as_ref(),
            &known,
            now,
        )?;
        write_contracts(&dir, &mut known, &merged)?;
        if cached_catalog.as_ref().map(|(_, cached_hash)| cached_hash) != Some(&hash) {
            write_cache(&dir, &bytes, &signature)?;
        }
        activate_if_newer(index.sequence, hash, merged)?;
        Ok(index.sequence)
    });
    let result = match remote {
        Ok(accepted) => status("remote", Some(accepted), None),
        Err(error) => status(source, sequence, Some(error)),
    };
    record_refresh(&dir, &result, now);
    result
}

/// The outcome of the last refresh, kept next to the cache so a rejected or
/// unreachable catalog is visible on the machine. Best effort: failing to
/// write it never changes the outcome.
fn record_refresh(dir: &Path, status: &CatalogStatus, now: i64) {
    let record = serde_json::json!({
        "checkedAt": now,
        "source": status.source,
        "sequence": status.sequence,
        "error": status.error,
    });
    if let Ok(bytes) = serde_json::to_vec_pretty(&record) {
        let _ = write_atomic(&dir.join("last-refresh.json"), &bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUBLISHED_INDEX: &[u8] = include_bytes!("../../website/public/bot/catalog/index.json");
    /// Sequence 2 as published on 2026-09-30, the cache found on the test PC.
    const OLD_INDEX: &[u8] = include_bytes!("../tests/fixtures/catalog_sequence_2.json");
    const OLD_SIGNATURE: &str = include_str!("../tests/fixtures/catalog_sequence_2.json.sig");
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
        let parsed = verify_and_parse(PUBLISHED_INDEX, PUBLISHED_SIGNATURE)
            .expect("published catalog must verify; run `npm run catalog:publish`");
        assert_eq!(
            parsed.sequence,
            min_sequence(),
            "build.rs floor follows the committed index"
        );
        let merged = package_registry::merge_remote(&parsed.packages).expect("valid manifests");
        assert!(check_contracts(&merged, &BTreeMap::new()).is_ok());
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
            verify_and_parse(&tampered, PUBLISHED_SIGNATURE)
                .err()
                .as_deref(),
            Some("catalog_signature_invalid")
        );
        assert!(
            verify_signature(PUBLISHED_INDEX, PUBLISHED_SIGNATURE, UPDATER_PUBLIC_KEY).is_err()
        );
        assert!(verify_and_parse(PUBLISHED_INDEX, "not base64!").is_err());
        // Line endings rewritten by a checkout must not verify either.
        let crlf = String::from_utf8(PUBLISHED_INDEX.to_vec())
            .unwrap()
            .replace('\n', "\r\n");
        assert!(verify_and_parse(crlf.as_bytes(), PUBLISHED_SIGNATURE).is_err());
    }

    #[test]
    fn freshness_rules() {
        let now = parse_utc("2026-10-01T00:00:00Z").unwrap();
        let ok = index(5, "2026-09-30T00:00:00Z", "2027-03-01T00:00:00Z");
        assert!(check_time(&ok, now, false).is_ok());
        let expired = index(5, "2026-01-01T00:00:00Z", "2026-06-01T00:00:00Z");
        assert_eq!(
            check_time(&expired, now, false).err().as_deref(),
            Some("catalog_expired")
        );
        assert!(
            check_time(&expired, now, true).is_ok(),
            "an expired cache stays a baseline"
        );
        let future = index(5, "2026-12-01T00:00:00Z", "2027-06-01T00:00:00Z");
        assert!(check_time(&future, now, false).is_err());
        let forever = index(5, "2026-09-30T00:00:00Z", "9999-01-01T00:00:00Z");
        assert!(
            check_time(&forever, now, false).is_err(),
            "validity is capped"
        );
    }

    #[test]
    fn a_cache_below_the_build_floor_does_not_block_the_catalog_at_the_floor() {
        // EA.24/EA.25 Windows pass: the device cache held sequence 2 while
        // this build's floor and the Worker were at the published sequence;
        // refresh rejected it as catalog_sequence_reused.
        let old: CatalogIndex = verify_and_parse(OLD_INDEX, OLD_SIGNATURE).expect("old catalog");
        let published =
            verify_and_parse(PUBLISHED_INDEX, PUBLISHED_SIGNATURE).expect("published catalog");
        assert!(old.sequence < published.sequence);
        assert_eq!(published.sequence, min_sequence());
        let cached = (old.sequence, byte_hash(OLD_INDEX));
        let baseline = min_sequence().max(old.sequence);
        let issued = parse_utc(&published.issued_at).expect("issued");
        let (accepted, hash, _) = evaluate_remote(
            PUBLISHED_INDEX,
            PUBLISHED_SIGNATURE,
            baseline,
            Some(&cached),
            &BTreeMap::new(),
            issued + 60,
        )
        .expect("the published catalog is accepted over an older cache");
        assert_eq!(accepted.sequence, published.sequence);
        // Same sequence as the cache with different bytes is still a reuse.
        assert_eq!(
            evaluate_remote(
                PUBLISHED_INDEX,
                PUBLISHED_SIGNATURE,
                baseline,
                Some(&(published.sequence, "0".repeat(64))),
                &BTreeMap::new(),
                issued + 60,
            )
            .err()
            .as_deref(),
            Some("catalog_sequence_reused")
        );
        // The identical cached catalog is accepted again.
        assert!(evaluate_remote(
            PUBLISHED_INDEX,
            PUBLISHED_SIGNATURE,
            baseline,
            Some(&(published.sequence, hash)),
            &BTreeMap::new(),
            issued + 60,
        )
        .is_ok());
    }

    #[test]
    fn sequence_rules() {
        assert!(check_sequence(5, 5).is_ok());
        assert!(check_sequence(6, 5).is_ok());
        assert_eq!(
            check_sequence(4, 5).err().as_deref(),
            Some("catalog_rollback")
        );
        assert!(check_sequence(u64::MAX, 5).is_err(), "no lock-out jump");
    }

    #[test]
    fn a_package_id_never_changes_what_it_installs() {
        let published: serde_json::Value = serde_json::from_slice(PUBLISHED_INDEX).unwrap();
        let mut packages = published["packages"].as_array().unwrap().clone();
        // Metadata may change.
        let tree = packages
            .iter_mut()
            .find(|p| p["id"] == "minify.tree-mod")
            .unwrap();
        tree["name"]["en"] = "Tree Mod (renamed)".into();
        assert!(package_registry::merge_remote(&packages).is_ok());
        // The contract of a shipped package may not.
        let tree = packages
            .iter_mut()
            .find(|p| p["id"] == "minify.tree-mod")
            .unwrap();
        tree["resources"][0]["sha256"] = "0".repeat(64).into();
        assert_eq!(
            package_registry::merge_remote(&packages).err().as_deref(),
            Some("catalog_contract_changed")
        );
        // Nor the contract of a package accepted earlier on this device.
        let merged =
            package_registry::merge_remote(&published["packages"].as_array().unwrap().clone())
                .unwrap();
        let mut known = BTreeMap::new();
        known.insert("minify.remove-river".to_string(), "different".to_string());
        assert_eq!(
            check_contracts(&merged, &known).err().as_deref(),
            Some("catalog_contract_changed")
        );
    }

    #[test]
    fn remote_manifests_cannot_leave_the_trusted_repository_or_collide() {
        let mut hostile = index(2, "2026-09-30T00:00:00Z", "2027-03-01T00:00:00Z");
        hostile.packages[0]["source"]["repository"] = "attacker/mods".into();
        assert!(package_registry::merge_remote(&hostile.packages).is_err());
        let mut script = index(2, "2026-09-30T00:00:00Z", "2027-03-01T00:00:00Z");
        script.packages[0]["id"] = "minify.new-package".into();
        script.packages[0]["catalogId"] = "minify-new-package".into();
        script.packages[0]["resources"][0]["path"] = "panorama/scripts/hud.vjs_c".into();
        assert!(
            package_registry::merge_remote(&script.packages).is_err(),
            "scripts are not data"
        );
        let mut colliding = index(2, "2026-09-30T00:00:00Z", "2027-03-01T00:00:00Z");
        colliding.packages[0]["id"] = "minify.renamed".into();
        assert_eq!(
            package_registry::merge_remote(&colliding.packages)
                .err()
                .as_deref(),
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
    fn cache_and_contract_files_round_trip_and_corruption_is_ignored() {
        let root = std::env::temp_dir().join(format!(
            "betterfy-catalog-cache-{}-{}",
            std::process::id(),
            now_seconds()
        ));
        let dir = cache_dir(&root).expect("cache dir");
        write_cache(&dir, PUBLISHED_INDEX, PUBLISHED_SIGNATURE).expect("write");
        assert_eq!(read_cache(&dir).expect("read back").index, PUBLISHED_INDEX);
        fs::write(dir.join("index.json"), b"{}").expect("corrupt");
        let corrupted = read_cache(&dir).expect("read corrupted");
        assert!(verify_and_parse(&corrupted.index, &corrupted.signature).is_err());
        let parsed = verify_and_parse(PUBLISHED_INDEX, PUBLISHED_SIGNATURE).unwrap();
        let merged = package_registry::merge_remote(&parsed.packages).unwrap();
        let mut known = BTreeMap::new();
        write_contracts(&dir, &mut known, &merged).expect("contracts");
        assert_eq!(read_contracts(&dir).len(), merged.len());
        let _ = fs::remove_dir_all(root);
    }
}

//! Installed profile: one derived record of what BetterFy last installed in
//! the game.
//!
//! The record lives at `<app_data>/engine-v1/installed-profile.json`. It is
//! informational only. The game-deployment ownership record and journals
//! stay the source of truth: a profile whose operation, hash, plan, language
//! or package order does not match the current owned deployment is treated
//! as stale and ignored. Writing or clearing the profile never changes the
//! result of the install or rollback it describes.
//!
//! The file contains opaque operation IDs, hashes, the package order, the
//! language, the Steam build number, the app version and a UTC timestamp. It
//! never contains filesystem paths, Steam IDs or account names.

use serde::{Deserialize, Serialize};
#[cfg(not(target_os = "windows"))]
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::evidence_reports::civil_from_days;
use crate::game_deployment::DeploymentReceipt;
use crate::game_language::GameLanguage;

pub const SCHEMA_VERSION: u32 = 1;
const FILE_NAME: &str = "installed-profile.json";
const MAX_PROFILE_BYTES: u64 = 64 * 1024;
const MAX_PACKAGES: usize = 64;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstalledProfile {
    pub schema_version: u32,
    pub deployment_operation_id: String,
    pub plan_id: Option<String>,
    pub package_ids: Vec<String>,
    pub language: GameLanguage,
    pub installed_sha256: String,
    pub installed_at: String,
    pub dota_build_at_install: Option<String>,
    pub steam_operation_id: Option<String>,
    pub app_version: String,
}

impl InstalledProfile {
    pub fn from_receipt(
        receipt: &DeploymentReceipt,
        dota_build_at_install: Option<String>,
        app_version: &str,
        now: SystemTime,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            deployment_operation_id: receipt.operation_id.clone(),
            plan_id: receipt.bundle_plan_id.clone(),
            package_ids: receipt.package_ids.clone(),
            language: receipt.language,
            installed_sha256: receipt.installed_sha256.clone(),
            installed_at: rfc3339_utc(now),
            dota_build_at_install,
            steam_operation_id: None,
            app_version: app_version.to_string(),
        }
    }

    /// Whether this profile describes exactly the given owned deployment.
    pub fn matches(&self, receipt: &DeploymentReceipt) -> bool {
        self.deployment_operation_id == receipt.operation_id
            && self.installed_sha256 == receipt.installed_sha256
            && self.plan_id == receipt.bundle_plan_id
            && self.package_ids == receipt.package_ids
            && self.language == receipt.language
    }
}

fn reject_symlink(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err("installed_profile_path_unsafe".to_string())
        }
        _ => Ok(()),
    }
}

fn profile_path(app_data_root: &Path) -> Result<PathBuf, String> {
    reject_symlink(app_data_root)?;
    fs::create_dir_all(app_data_root).map_err(|_| "installed_profile_write_failed".to_string())?;
    let root = app_data_root.join("engine-v1");
    reject_symlink(&root)?;
    fs::create_dir_all(&root).map_err(|_| "installed_profile_write_failed".to_string())?;
    reject_symlink(&root)?;
    let path = root.join(FILE_NAME);
    reject_symlink(&path)?;
    Ok(path)
}

fn is_opaque_id(value: &str, prefix: &str) -> bool {
    value.len() > prefix.len()
        && value.len() <= 96
        && value.starts_with(prefix)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate(profile: &InstalledProfile) -> Result<(), String> {
    let invalid = || Err("installed_profile_invalid".to_string());
    if profile.schema_version != SCHEMA_VERSION
        || !is_opaque_id(&profile.deployment_operation_id, "deploy-op-")
        || !is_sha256(&profile.installed_sha256)
        || profile.package_ids.len() > MAX_PACKAGES
        || profile.installed_at.len() != 20
        || profile.app_version.is_empty()
        || profile.app_version.len() > 64
    {
        return invalid();
    }
    if profile
        .plan_id
        .as_deref()
        .is_some_and(|plan| plan.is_empty() || plan.len() > 128 || !plan.is_ascii())
        || profile.package_ids.iter().any(|id| {
            id.is_empty()
                || id.len() > 96
                || !id.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_.".contains(&byte)
                })
        })
        || profile
            .dota_build_at_install
            .as_deref()
            .is_some_and(|build| {
                build.is_empty() || build.len() > 12 || !build.bytes().all(|b| b.is_ascii_digit())
            })
        || profile
            .steam_operation_id
            .as_deref()
            .is_some_and(|id| !is_opaque_id(id, "steam-op-"))
        || !profile
            .app_version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-+".contains(&byte))
    {
        return invalid();
    }
    Ok(())
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|_| "installed_profile_write_failed".to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "installed_profile_write_failed".to_string())
}

/// Replace the profile atomically: a synced temporary file in the same
/// directory is renamed over the old one (`MoveFileExW` with
/// `MOVEFILE_REPLACE_EXISTING` on Windows, `rename(2)` elsewhere).
pub fn write(app_data_root: &Path, profile: &InstalledProfile) -> Result<(), String> {
    validate(profile)?;
    let _guard = WRITE_LOCK
        .lock()
        .map_err(|_| "installed_profile_write_failed".to_string())?;
    write_locked(app_data_root, profile)
}

fn write_locked(app_data_root: &Path, profile: &InstalledProfile) -> Result<(), String> {
    let path = profile_path(app_data_root)?;
    let directory = path
        .parent()
        .ok_or_else(|| "installed_profile_write_failed".to_string())?;
    let mut bytes =
        serde_json::to_vec_pretty(profile).map_err(|_| "installed_profile_invalid".to_string())?;
    bytes.push(b'\n');
    let temporary = directory.join(format!(
        ".installed-profile-{}-{}.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    reject_symlink(&temporary)?;
    if let Err(code) = write_synced(&temporary, &bytes) {
        let _ = fs::remove_file(&temporary);
        return Err(code);
    }
    reject_symlink(&path)?;
    if fs::rename(&temporary, &path).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err("installed_profile_write_failed".to_string());
    }
    #[cfg(not(target_os = "windows"))]
    File::open(directory)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| "installed_profile_write_failed".to_string())?;
    Ok(())
}

/// Read the profile if one exists. A malformed or oversized file is an error.
pub fn read(app_data_root: &Path) -> Result<Option<InstalledProfile>, String> {
    reject_symlink(app_data_root)?;
    let root = app_data_root.join("engine-v1");
    reject_symlink(&root)?;
    let path = root.join(FILE_NAME);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(None),
    };
    if metadata.file_type().is_symlink() {
        return Err("installed_profile_path_unsafe".to_string());
    }
    if !metadata.is_file() || metadata.len() > MAX_PROFILE_BYTES {
        return Err("installed_profile_invalid".to_string());
    }
    let bytes = fs::read(&path).map_err(|_| "installed_profile_invalid".to_string())?;
    let profile: InstalledProfile =
        serde_json::from_slice(&bytes).map_err(|_| "installed_profile_invalid".to_string())?;
    validate(&profile)?;
    Ok(Some(profile))
}

/// Remove the profile. Removing an absent profile succeeds.
pub fn clear(app_data_root: &Path) -> Result<(), String> {
    let _guard = WRITE_LOCK
        .lock()
        .map_err(|_| "installed_profile_write_failed".to_string())?;
    let path = profile_path(app_data_root)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("installed_profile_write_failed".to_string()),
    }
}

/// The profile for the current owned deployment, or `None` when there is no
/// profile, it cannot be read, or it describes a different deployment.
pub fn current_for(app_data_root: &Path, receipt: &DeploymentReceipt) -> Option<InstalledProfile> {
    read(app_data_root)
        .ok()
        .flatten()
        .filter(|profile| profile.matches(receipt))
}

/// Record a successful install and return the install result unchanged.
/// A failed profile write is ignored: the profile is a derived record.
pub fn record_install(
    app_data_root: &Path,
    result: Result<DeploymentReceipt, String>,
    dota_build: impl FnOnce() -> Option<String>,
    app_version: &str,
    now: SystemTime,
) -> Result<DeploymentReceipt, String> {
    if let Ok(receipt) = &result {
        if receipt.committed && !receipt.rolled_back {
            let profile = InstalledProfile::from_receipt(receipt, dota_build(), app_version, now);
            let _ = write(app_data_root, &profile);
        }
    }
    result
}

/// Attach the Steam operation that activated this deployment. Ignored when
/// the profile is absent or describes another deployment.
pub fn record_steam_operation(
    app_data_root: &Path,
    deployment_operation_id: &str,
    steam_operation_id: &str,
) {
    let Ok(_guard) = WRITE_LOCK.lock() else {
        return;
    };
    let Ok(Some(mut profile)) = read(app_data_root) else {
        return;
    };
    if profile.deployment_operation_id != deployment_operation_id {
        return;
    }
    profile.steam_operation_id = Some(steam_operation_id.to_string());
    if validate(&profile).is_ok() {
        let _ = write_locked(app_data_root, &profile);
    }
}

/// Drop a Steam operation that has been rolled back from the profile.
pub fn forget_steam_operation(app_data_root: &Path, steam_operation_id: &str) {
    let Ok(_guard) = WRITE_LOCK.lock() else {
        return;
    };
    let Ok(Some(mut profile)) = read(app_data_root) else {
        return;
    };
    if profile.steam_operation_id.as_deref() != Some(steam_operation_id) {
        return;
    }
    profile.steam_operation_id = None;
    let _ = write_locked(app_data_root, &profile);
}

/// Clear the profile after a successful rollback and return the rollback
/// result unchanged.
pub fn record_rollback(
    app_data_root: &Path,
    result: Result<DeploymentReceipt, String>,
) -> Result<DeploymentReceipt, String> {
    if result.as_ref().is_ok_and(|receipt| receipt.rolled_back) {
        let _ = clear(app_data_root);
    }
    result
}

/// True only when both builds are known and differ.
pub fn dota_patched(profile: Option<&InstalledProfile>, current_build: Option<&str>) -> bool {
    match (
        profile.and_then(|profile| profile.dota_build_at_install.as_deref()),
        current_build,
    ) {
        (Some(installed), Some(current)) => installed != current,
        _ => false,
    }
}

fn rfc3339_utc(now: SystemTime) -> String {
    let seconds = now
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    let rest = seconds % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3_600,
        rest % 3_600 / 60,
        rest % 60
    )
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
        let path = std::env::temp_dir().join(format!(
            "betterfy-profile-{label}-{}-{nonce}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("root");
        path
    }

    fn receipt(operation_id: &str) -> DeploymentReceipt {
        DeploymentReceipt {
            operation_id: operation_id.to_string(),
            language: GameLanguage::Russian,
            before_sha256: None,
            installed_sha256: "a".repeat(64),
            backup_verified: true,
            committed: true,
            rolled_back: false,
            bundle_plan_id: Some(format!("sha256:{}", "d".repeat(64))),
            package_ids: vec![
                "minify.tree-mod".to_string(),
                "minify.show-networth".to_string(),
            ],
        }
    }

    fn at(seconds: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(seconds)
    }

    fn profile_file(root: &Path) -> PathBuf {
        root.join("engine-v1").join(FILE_NAME)
    }

    #[test]
    fn timestamps_are_rfc3339_utc() {
        assert_eq!(rfc3339_utc(at(0)), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339_utc(at(1_709_210_096)), "2024-02-29T12:34:56Z");
    }

    #[test]
    fn writes_reads_replaces_and_clears_atomically() {
        let root = temp_root("roundtrip");
        assert_eq!(read(&root).expect("read"), None);

        let first = InstalledProfile::from_receipt(
            &receipt("deploy-op-1-1-0"),
            Some("12345".to_string()),
            "0.1.4",
            at(1_709_210_096),
        );
        write(&root, &first).expect("write");
        assert_eq!(read(&root).expect("read"), Some(first.clone()));
        assert_eq!(first.installed_at, "2024-02-29T12:34:56Z");

        let second = InstalledProfile::from_receipt(
            &receipt("deploy-op-2-1-0"),
            None,
            "0.1.4",
            at(1_709_210_100),
        );
        write(&root, &second).expect("replace");
        assert_eq!(read(&root).expect("read"), Some(second));

        let leftovers = fs::read_dir(root.join("engine-v1"))
            .expect("list")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);

        clear(&root).expect("clear");
        assert_eq!(read(&root).expect("read"), None);
        clear(&root).expect("clearing twice is fine");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn rejects_malformed_and_unknown_fields() {
        let root = temp_root("malformed");
        fs::create_dir_all(root.join("engine-v1")).expect("dir");
        fs::write(profile_file(&root), b"{\"schemaVersion\":1}").expect("write");
        assert!(read(&root).is_err());
        let mut value = serde_json::to_value(InstalledProfile::from_receipt(
            &receipt("deploy-op-1-1-0"),
            None,
            "0.1.4",
            at(0),
        ))
        .expect("value");
        value["gamePath"] = serde_json::json!("C:/Games");
        fs::write(
            profile_file(&root),
            serde_json::to_vec(&value).expect("json"),
        )
        .expect("write");
        assert!(read(&root).is_err());
        let mut bad = InstalledProfile::from_receipt(
            &receipt("deploy-op-1-1-0"),
            Some("12a".to_string()),
            "0.1.4",
            at(0),
        );
        assert!(write(&root, &bad).is_err());
        bad.dota_build_at_install = None;
        bad.deployment_operation_id = "../escape".to_string();
        assert!(write(&root, &bad).is_err());
        fs::remove_dir_all(root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_profile() {
        let root = temp_root("symlink");
        let outside = temp_root("symlink-outside").join("elsewhere.json");
        fs::write(&outside, b"{}").expect("outside");
        fs::create_dir_all(root.join("engine-v1")).expect("dir");
        std::os::unix::fs::symlink(&outside, profile_file(&root)).expect("symlink");
        assert!(read(&root).is_err());
        let profile =
            InstalledProfile::from_receipt(&receipt("deploy-op-1-1-0"), None, "0.1.4", at(0));
        assert!(write(&root, &profile).is_err());
        assert_eq!(fs::read(&outside).expect("outside"), b"{}");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn a_profile_for_another_deployment_is_stale() {
        let root = temp_root("stale");
        let installed = receipt("deploy-op-1-1-0");
        record_install(&root, Ok(installed.clone()), || None, "0.1.4", at(0)).expect("install");
        assert!(current_for(&root, &installed).is_some());

        assert!(current_for(&root, &receipt("deploy-op-2-1-0")).is_none());
        let mut rehashed = installed.clone();
        rehashed.installed_sha256 = "b".repeat(64);
        assert!(current_for(&root, &rehashed).is_none());
        let mut reordered = installed.clone();
        reordered.package_ids.reverse();
        assert!(current_for(&root, &reordered).is_none());
        let mut relanguaged = installed;
        relanguaged.language = GameLanguage::Dutch;
        assert!(current_for(&root, &relanguaged).is_none());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn serialized_profile_contains_no_paths() {
        let root = temp_root("privacy");
        let mut installed = receipt("deploy-op-1-1-0");
        installed.before_sha256 = Some("c".repeat(64));
        record_install(
            &root,
            Ok(installed),
            || Some("20000001".to_string()),
            "0.1.4",
            at(0),
        )
        .expect("install");
        record_steam_operation(&root, "deploy-op-1-1-0", "steam-op-1-1-0");
        let text = fs::read_to_string(profile_file(&root)).expect("profile");
        let root_text = root.to_string_lossy();
        let temp_text = std::env::temp_dir().to_string_lossy().to_string();
        assert!(!text.contains(root_text.as_ref()));
        assert!(!text.contains(temp_text.trim_end_matches(['/', '\\'])));
        assert!(!text.contains('\\'));
        assert!(!text.contains('/'));
        assert!(text.contains("\"steamOperationId\": \"steam-op-1-1-0\""));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn a_failed_profile_write_does_not_change_the_install_result() {
        let root = temp_root("write-fails");
        // A regular file where the engine directory should be makes every
        // profile write fail.
        fs::write(root.join("engine-v1"), b"not a directory").expect("blocker");
        let installed = receipt("deploy-op-1-1-0");
        let expected = serde_json::to_value(&installed).expect("value");
        let result = record_install(
            &root,
            Ok(installed),
            || Some("1".to_string()),
            "0.1.4",
            at(0),
        )
        .expect("install result survives");
        assert_eq!(serde_json::to_value(&result).expect("value"), expected);

        let failed = record_install(
            &root,
            Err("deployment_conflict".to_string()),
            || panic!("no build lookup after a failed install"),
            "0.1.4",
            at(0),
        );
        assert_eq!(failed.unwrap_err(), "deployment_conflict");

        let mut rolled_back = receipt("deploy-op-1-1-0");
        rolled_back.rolled_back = true;
        let expected = serde_json::to_value(&rolled_back).expect("value");
        let result = record_rollback(&root, Ok(rolled_back)).expect("rollback result survives");
        assert_eq!(serde_json::to_value(&result).expect("value"), expected);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn rollback_clears_and_steam_rollback_forgets() {
        let root = temp_root("rollback");
        let installed = receipt("deploy-op-1-1-0");
        record_install(&root, Ok(installed.clone()), || None, "0.1.4", at(0)).expect("install");
        record_steam_operation(&root, "deploy-op-other-1-0", "steam-op-1-1-0");
        assert_eq!(read(&root).expect("read").unwrap().steam_operation_id, None);
        record_steam_operation(&root, "deploy-op-1-1-0", "steam-op-1-1-0");
        forget_steam_operation(&root, "steam-op-2-1-0");
        assert!(read(&root)
            .expect("read")
            .unwrap()
            .steam_operation_id
            .is_some());
        forget_steam_operation(&root, "steam-op-1-1-0");
        assert_eq!(read(&root).expect("read").unwrap().steam_operation_id, None);

        record_rollback(&root, Err("rollback_failed".to_string())).unwrap_err();
        assert!(read(&root).expect("read").is_some());
        let mut rolled_back = installed;
        rolled_back.rolled_back = true;
        record_rollback(&root, Ok(rolled_back)).expect("rollback");
        assert_eq!(read(&root).expect("read"), None);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn patched_only_when_both_builds_are_known_and_differ() {
        let mut profile = InstalledProfile::from_receipt(
            &receipt("deploy-op-1-1-0"),
            Some("100".to_string()),
            "0.1.4",
            at(0),
        );
        assert!(!dota_patched(Some(&profile), Some("100")));
        assert!(dota_patched(Some(&profile), Some("101")));
        assert!(!dota_patched(Some(&profile), None));
        assert!(!dota_patched(None, Some("101")));
        profile.dota_build_at_install = None;
        assert!(!dota_patched(Some(&profile), Some("101")));
    }
}

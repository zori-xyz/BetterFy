use crate::steam_config::{plan_managed_launch_option, LaunchOptionPlan};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const JOURNAL_SCHEMA_VERSION: u32 = 1;
const MAX_LOCALCONFIG_BYTES: u64 = 16 * 1024 * 1024;
const PROFILE_TOKEN_PREFIX: &str = "steam-profile-v1:";
const CONFIRMATION_DOMAIN: &[u8] = b"betterfy-steam-launch-options-v1";
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
struct SteamProfile {
    token: String,
    localconfig_path: PathBuf,
    modified_ms: u128,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SteamProfileStatus {
    Ready,
    AlreadyManaged,
    LaunchOptionConflict,
    InvalidConfig,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamProfileSummary {
    profile_token: String,
    profile_index: usize,
    status: SteamProfileStatus,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamLaunchOptionPreview {
    profile_token: String,
    changed: bool,
    before_sha256: String,
    after_sha256: String,
    confirmation_token: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplySteamLaunchOptionRequest {
    pub profile_token: String,
    pub confirmation_token: String,
    pub confirmed: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SteamConfigOperationRequest {
    pub operation_id: String,
    pub confirmed: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamConfigReceipt {
    operation_id: Option<String>,
    profile_token: String,
    changed: bool,
    before_sha256: String,
    after_sha256: String,
    backup_verified: bool,
    committed: bool,
    rolled_back: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SteamConfigPhase {
    BackedUp,
    Prepared,
    Committed,
    RolledBack,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SteamConfigJournal {
    schema_version: u32,
    operation_id: String,
    profile_token: String,
    before_sha256: String,
    after_sha256: String,
    phase: SteamConfigPhase,
    created_at_ms: u128,
    updated_at_ms: u128,
    backup_relative_path: String,
    error_code: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailurePoint {
    None,
    AfterPreparedJournal,
    AfterReplace,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn now_ms() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|_| "steam_config_write_failed".to_string())
}

fn new_operation_id() -> Result<String, String> {
    let count = OPERATION_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(format!(
        "steam-op-{}-{}-{count}",
        now_ms()?,
        std::process::id()
    ))
}

fn validate_profile_token(token: &str) -> Result<(), String> {
    let Some(digest) = token.strip_prefix(PROFILE_TOKEN_PREFIX) else {
        return Err("steam_profile_not_found".to_string());
    };
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("steam_profile_not_found".to_string());
    }
    Ok(())
}

fn validate_operation_id(operation_id: &str) -> Result<(), String> {
    if operation_id.len() < 16
        || operation_id.len() > 96
        || !operation_id.starts_with("steam-op-")
        || !operation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err("steam_operation_not_found".to_string());
    }
    Ok(())
}

fn reject_symlink(path: &Path, code: &str) -> Result<(), String> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err(code.to_string());
        }
    }
    Ok(())
}

fn canonical_directory(path: &Path, code: &str) -> Result<PathBuf, String> {
    reject_symlink(path, code)?;
    let canonical = path.canonicalize().map_err(|_| code.to_string())?;
    if !canonical.is_dir() {
        return Err(code.to_string());
    }
    Ok(canonical)
}

fn profile_token(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"betterfy-steam-profile-path-v1\0");
    hasher.update(path.to_string_lossy().as_bytes());
    format!("{PROFILE_TOKEN_PREFIX}{:x}", hasher.finalize())
}

fn modified_ms(path: &Path) -> u128 {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_millis())
}

fn checked_localconfig(path: &Path) -> Result<PathBuf, String> {
    reject_symlink(path, "steam_profile_invalid")?;
    let metadata = fs::metadata(path).map_err(|_| "steam_profile_invalid".to_string())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_LOCALCONFIG_BYTES {
        return Err("steam_profile_invalid".to_string());
    }
    path.canonicalize()
        .map_err(|_| "steam_profile_invalid".to_string())
}

fn discover_profiles_in_roots(roots: &[PathBuf]) -> Result<Vec<SteamProfile>, String> {
    let mut profiles = Vec::new();
    let mut seen = HashSet::new();
    for root in roots {
        let Ok(root) = canonical_directory(root, "steam_root_invalid") else {
            continue;
        };
        let userdata_path = root.join("userdata");
        let Ok(userdata) = canonical_directory(&userdata_path, "steam_root_invalid") else {
            continue;
        };
        if !userdata.starts_with(&root) {
            continue;
        }
        let Ok(entries) = fs::read_dir(&userdata) else {
            continue;
        };
        for entry in entries.flatten() {
            let account_name = entry.file_name();
            let Some(account_name) = account_name.to_str() else {
                continue;
            };
            if account_name.is_empty()
                || account_name.len() > 20
                || !account_name.bytes().all(|byte| byte.is_ascii_digit())
            {
                continue;
            }
            let account_path = entry.path();
            let Ok(account) = canonical_directory(&account_path, "steam_profile_invalid") else {
                continue;
            };
            if account.parent() != Some(userdata.as_path()) {
                continue;
            }
            let candidate = account.join("config").join("localconfig.vdf");
            let Ok(localconfig) = checked_localconfig(&candidate) else {
                continue;
            };
            if !localconfig.starts_with(&account) || !seen.insert(localconfig.clone()) {
                continue;
            }
            profiles.push(SteamProfile {
                token: profile_token(&localconfig),
                modified_ms: modified_ms(&localconfig),
                localconfig_path: localconfig,
            });
        }
    }
    profiles.sort_by(|left, right| {
        right
            .modified_ms
            .cmp(&left.modified_ms)
            .then(left.token.cmp(&right.token))
    });
    Ok(profiles)
}

#[cfg(target_os = "windows")]
fn platform_steam_roots() -> Vec<PathBuf> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY};
    use winreg::RegKey;

    let candidates = [
        (
            HKEY_CURRENT_USER,
            "Software\\Valve\\Steam",
            "SteamPath",
            KEY_READ,
        ),
        (
            HKEY_LOCAL_MACHINE,
            "Software\\Valve\\Steam",
            "InstallPath",
            KEY_READ | KEY_WOW64_32KEY,
        ),
        (
            HKEY_LOCAL_MACHINE,
            "Software\\WOW6432Node\\Valve\\Steam",
            "InstallPath",
            KEY_READ,
        ),
    ];
    let mut roots = Vec::new();
    for (hive, key_path, value_name, flags) in candidates {
        let hive = RegKey::predef(hive);
        if let Ok(key) = hive.open_subkey_with_flags(key_path, flags) {
            if let Ok(value) = key.get_value::<String, _>(value_name) {
                if !value.trim().is_empty() {
                    roots.push(PathBuf::from(value.trim()));
                }
            }
        }
    }
    roots
}

#[cfg(target_os = "macos")]
fn platform_steam_roots() -> Vec<PathBuf> {
    std::env::var_os("HOME")
        .map(|home| {
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Steam")
        })
        .into_iter()
        .collect()
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn platform_steam_roots() -> Vec<PathBuf> {
    Vec::new()
}

fn read_localconfig(path: &Path) -> Result<String, String> {
    let checked = checked_localconfig(path)?;
    if checked != path {
        return Err("steam_profile_invalid".to_string());
    }
    let metadata = fs::metadata(path).map_err(|_| "steam_config_read_failed".to_string())?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(|_| "steam_config_read_failed".to_string())?;
    String::from_utf8(bytes).map_err(|_| "steam_config_invalid".to_string())
}

fn status_for_contents(contents: &str) -> SteamProfileStatus {
    match plan_managed_launch_option(contents) {
        Ok(plan) if plan.changed => SteamProfileStatus::Ready,
        Ok(_) => SteamProfileStatus::AlreadyManaged,
        Err(code) if code == "launch_option_conflict" => SteamProfileStatus::LaunchOptionConflict,
        Err(_) => SteamProfileStatus::InvalidConfig,
    }
}

fn resolve_profile(roots: &[PathBuf], token: &str) -> Result<SteamProfile, String> {
    validate_profile_token(token)?;
    discover_profiles_in_roots(roots)?
        .into_iter()
        .find(|profile| profile.token == token)
        .ok_or_else(|| "steam_profile_not_found".to_string())
}

fn confirmation_token(profile_token: &str, plan: &LaunchOptionPlan) -> String {
    let mut hasher = Sha256::new();
    hasher.update(CONFIRMATION_DOMAIN);
    hasher.update([0]);
    hasher.update(profile_token.as_bytes());
    hasher.update([0]);
    hasher.update(plan.before_sha256.as_bytes());
    hasher.update([0]);
    hasher.update(plan.after_sha256.as_bytes());
    format!("confirm-v1:{:x}", hasher.finalize())
}

pub fn list_platform_profiles() -> Result<Vec<SteamProfileSummary>, String> {
    list_profiles(&platform_steam_roots())
}

fn list_profiles(roots: &[PathBuf]) -> Result<Vec<SteamProfileSummary>, String> {
    discover_profiles_in_roots(roots)?
        .into_iter()
        .enumerate()
        .map(|(index, profile)| {
            let contents = read_localconfig(&profile.localconfig_path)?;
            Ok(SteamProfileSummary {
                profile_token: profile.token,
                profile_index: index + 1,
                status: status_for_contents(&contents),
            })
        })
        .collect()
}

pub fn preview_platform_profile(token: &str) -> Result<SteamLaunchOptionPreview, String> {
    preview_profile(&platform_steam_roots(), token)
}

fn preview_profile(roots: &[PathBuf], token: &str) -> Result<SteamLaunchOptionPreview, String> {
    let profile = resolve_profile(roots, token)?;
    let contents = read_localconfig(&profile.localconfig_path)?;
    let plan = plan_managed_launch_option(&contents)?;
    Ok(SteamLaunchOptionPreview {
        profile_token: profile.token.clone(),
        changed: plan.changed,
        before_sha256: plan.before_sha256.clone(),
        after_sha256: plan.after_sha256.clone(),
        confirmation_token: confirmation_token(&profile.token, &plan),
    })
}

fn transaction_roots(app_data_root: &Path) -> Result<(PathBuf, PathBuf), String> {
    reject_symlink(app_data_root, "steam_journal_invalid")?;
    fs::create_dir_all(app_data_root).map_err(|_| "backup_failed".to_string())?;
    let root = app_data_root.join("engine-v1").join("steam-config");
    let operations = root.join("operations");
    let journals = root.join("journals");
    for path in [&root, &operations, &journals] {
        reject_symlink(path, "steam_journal_invalid")?;
        fs::create_dir_all(path).map_err(|_| "backup_failed".to_string())?;
        reject_symlink(path, "steam_journal_invalid")?;
    }
    Ok((operations, journals))
}

#[cfg(target_os = "windows")]
struct TransactionLock {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(target_os = "windows")]
impl Drop for TransactionLock {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

#[cfg(target_os = "windows")]
fn acquire_transaction_lock(app_data_root: &Path) -> Result<TransactionLock, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_HIDDEN, OPEN_ALWAYS,
    };

    let lock_path = app_data_root
        .join("engine-v1")
        .join("steam-config")
        .join("write.lock");
    reject_symlink(&lock_path, "steam_config_locked")?;
    let lock_path = lock_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let handle = unsafe {
        CreateFileW(
            lock_path.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            std::ptr::null(),
            OPEN_ALWAYS,
            FILE_ATTRIBUTE_HIDDEN,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err("steam_config_locked".to_string());
    }
    Ok(TransactionLock { handle })
}

#[cfg(not(target_os = "windows"))]
static TRANSACTION_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(not(target_os = "windows"))]
struct TransactionLock {
    _guard: std::sync::MutexGuard<'static, ()>,
}

#[cfg(not(target_os = "windows"))]
fn acquire_transaction_lock(_app_data_root: &Path) -> Result<TransactionLock, String> {
    TRANSACTION_LOCK
        .lock()
        .map(|guard| TransactionLock { _guard: guard })
        .map_err(|_| "steam_config_locked".to_string())
}

#[cfg(unix)]
fn private_new_file(path: &Path) -> Result<File, String> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| "steam_config_write_failed".to_string())
}

#[cfg(not(unix))]
fn private_new_file(path: &Path) -> Result<File, String> {
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|_| "steam_config_write_failed".to_string())
}

fn write_new_synced(path: &Path, bytes: &[u8], code: &str) -> Result<(), String> {
    reject_symlink(path, code)?;
    let mut file = private_new_file(path).map_err(|_| code.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| code.to_string())
}

fn journal_sidecars(path: &Path) -> (PathBuf, PathBuf) {
    (
        path.with_extension("json.tmp"),
        path.with_extension("json.bak"),
    )
}

fn recover_journal_files(path: &Path) -> Result<(), String> {
    let (temporary, backup) = journal_sidecars(path);
    for candidate in [path, temporary.as_path(), backup.as_path()] {
        reject_symlink(candidate, "steam_journal_invalid")?;
    }
    if path.exists() {
        if backup.exists() {
            fs::remove_file(&backup).map_err(|_| "steam_recovery_required".to_string())?;
        }
        if temporary.exists() {
            fs::remove_file(&temporary).map_err(|_| "steam_recovery_required".to_string())?;
        }
        return Ok(());
    }
    if backup.exists() {
        fs::rename(&backup, path).map_err(|_| "steam_recovery_required".to_string())?;
        if temporary.exists() {
            fs::remove_file(&temporary).map_err(|_| "steam_recovery_required".to_string())?;
        }
        return Ok(());
    }
    if temporary.exists() {
        let journal: SteamConfigJournal = serde_json::from_slice(
            &fs::read(&temporary).map_err(|_| "steam_recovery_required".to_string())?,
        )
        .map_err(|_| "steam_recovery_required".to_string())?;
        if journal.schema_version != JOURNAL_SCHEMA_VERSION {
            return Err("steam_recovery_required".to_string());
        }
        fs::rename(temporary, path).map_err(|_| "steam_recovery_required".to_string())?;
    }
    Ok(())
}

fn write_journal(path: &Path, journal: &SteamConfigJournal) -> Result<(), String> {
    recover_journal_files(path)?;
    let (temporary, backup) = journal_sidecars(path);
    let bytes =
        serde_json::to_vec_pretty(journal).map_err(|_| "steam_journal_invalid".to_string())?;
    write_new_synced(&temporary, &bytes, "steam_journal_invalid")?;
    if path.exists() {
        fs::rename(path, &backup).map_err(|_| "steam_journal_invalid".to_string())?;
    }
    if fs::rename(&temporary, path).is_err() {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        let _ = fs::remove_file(&temporary);
        return Err("steam_journal_invalid".to_string());
    }
    if backup.exists() {
        fs::remove_file(backup).map_err(|_| "steam_journal_invalid".to_string())?;
    }
    Ok(())
}

fn read_journal(path: &Path) -> Result<SteamConfigJournal, String> {
    recover_journal_files(path)?;
    reject_symlink(path, "steam_journal_invalid")?;
    let metadata = fs::metadata(path).map_err(|_| "steam_operation_not_found".to_string())?;
    if !metadata.is_file() || metadata.len() > 64 * 1024 {
        return Err("steam_journal_invalid".to_string());
    }
    let journal: SteamConfigJournal =
        serde_json::from_slice(&fs::read(path).map_err(|_| "steam_journal_invalid".to_string())?)
            .map_err(|_| "steam_journal_invalid".to_string())?;
    if journal.schema_version != JOURNAL_SCHEMA_VERSION {
        return Err("steam_journal_invalid".to_string());
    }
    validate_operation_id(&journal.operation_id)?;
    validate_profile_token(&journal.profile_token)?;
    Ok(journal)
}

fn recover_journal_sidecars(journals_root: &Path) -> Result<(), String> {
    let entries = fs::read_dir(journals_root)
        .map_err(|_| "steam_journal_invalid".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "steam_journal_invalid".to_string())?;
    for entry in entries {
        let path = entry.path();
        if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("tmp" | "bak")
        ) {
            recover_journal_files(&path.with_extension(""))?;
        }
    }
    Ok(())
}

fn pending_operations(journals_root: &Path) -> Result<Vec<String>, String> {
    recover_journal_sidecars(journals_root)?;
    let mut pending = Vec::new();
    for entry in fs::read_dir(journals_root).map_err(|_| "steam_journal_invalid".to_string())? {
        let entry = entry.map_err(|_| "steam_journal_invalid".to_string())?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let journal = read_journal(&path)?;
        if matches!(
            journal.phase,
            SteamConfigPhase::BackedUp | SteamConfigPhase::Prepared
        ) {
            pending.push(journal.operation_id);
        }
    }
    pending.sort();
    Ok(pending)
}

fn cleanup_orphan_operations(operations_root: &Path, journals_root: &Path) -> Result<(), String> {
    recover_journal_sidecars(journals_root)?;
    let journal_ids = fs::read_dir(journals_root)
        .map_err(|_| "steam_journal_invalid".to_string())?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|value| value.to_str()) == Some("json"))
                .then(|| path.file_stem()?.to_str().map(str::to_string))
                .flatten()
        })
        .collect::<HashSet<_>>();
    let canonical_operations = operations_root
        .canonicalize()
        .map_err(|_| "steam_journal_invalid".to_string())?;
    for entry in fs::read_dir(operations_root).map_err(|_| "steam_journal_invalid".to_string())? {
        let entry = entry.map_err(|_| "steam_journal_invalid".to_string())?;
        let Some(operation_id) = entry.file_name().to_str().map(str::to_string) else {
            return Err("steam_journal_invalid".to_string());
        };
        validate_operation_id(&operation_id)?;
        if journal_ids.contains(&operation_id) {
            continue;
        }
        let path = entry.path();
        reject_symlink(&path, "steam_journal_invalid")?;
        let canonical = path
            .canonicalize()
            .map_err(|_| "steam_journal_invalid".to_string())?;
        if canonical == canonical_operations || !canonical.starts_with(&canonical_operations) {
            return Err("steam_journal_invalid".to_string());
        }
        fs::remove_dir_all(canonical).map_err(|_| "steam_journal_invalid".to_string())?;
    }
    Ok(())
}

fn temp_path(target: &Path, operation_id: &str) -> Result<PathBuf, String> {
    validate_operation_id(operation_id)?;
    let parent = target
        .parent()
        .ok_or_else(|| "steam_profile_invalid".to_string())?;
    Ok(parent.join(format!(".betterfy-{operation_id}.tmp")))
}

#[cfg(target_os = "windows")]
fn atomic_replace(target: &Path, replacement: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{ReplaceFileW, REPLACEFILE_WRITE_THROUGH};

    let target_wide = target
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let replacement_wide = replacement
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let replaced = unsafe {
        ReplaceFileW(
            target_wide.as_ptr(),
            replacement_wide.as_ptr(),
            std::ptr::null(),
            REPLACEFILE_WRITE_THROUGH,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if replaced == 0 {
        return Err("steam_config_commit_failed".to_string());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn atomic_replace(target: &Path, replacement: &Path) -> Result<(), String> {
    fs::rename(replacement, target).map_err(|_| "steam_config_commit_failed".to_string())?;
    if let Some(parent) = target.parent() {
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| "steam_config_commit_failed".to_string())?;
    }
    Ok(())
}

fn restore_bytes(target: &Path, operation_id: &str, bytes: &[u8]) -> Result<(), String> {
    reject_symlink(target, "steam_profile_invalid")?;
    let temporary = temp_path(target, operation_id)?;
    if temporary.exists() {
        reject_symlink(&temporary, "steam_config_commit_failed")?;
        fs::remove_file(&temporary).map_err(|_| "steam_config_commit_failed".to_string())?;
    }
    write_new_synced(&temporary, bytes, "steam_config_write_failed")?;
    if sha256(&fs::read(&temporary).map_err(|_| "steam_config_write_failed".to_string())?)
        != sha256(bytes)
    {
        let _ = fs::remove_file(&temporary);
        return Err("verification_failed".to_string());
    }
    atomic_replace(target, &temporary)
}

pub fn apply_platform_profile(
    app_data_root: &Path,
    request: ApplySteamLaunchOptionRequest,
) -> Result<SteamConfigReceipt, String> {
    apply_profile_with_failure(
        app_data_root,
        &platform_steam_roots(),
        request,
        FailurePoint::None,
    )
}

fn apply_profile_with_failure(
    app_data_root: &Path,
    roots: &[PathBuf],
    request: ApplySteamLaunchOptionRequest,
    failure: FailurePoint,
) -> Result<SteamConfigReceipt, String> {
    if !request.confirmed {
        return Err("steam_config_confirmation_required".to_string());
    }
    let (operations_root, journals_root) = transaction_roots(app_data_root)?;
    let _lock = acquire_transaction_lock(app_data_root)?;
    cleanup_orphan_operations(&operations_root, &journals_root)?;
    if !pending_operations(&journals_root)?.is_empty() {
        return Err("steam_recovery_required".to_string());
    }
    let profile = resolve_profile(roots, &request.profile_token)?;
    let contents = read_localconfig(&profile.localconfig_path)?;
    let plan = plan_managed_launch_option(&contents)?;
    if request.confirmation_token != confirmation_token(&profile.token, &plan) {
        return Err("steam_config_plan_stale".to_string());
    }
    if !plan.changed {
        return Ok(SteamConfigReceipt {
            operation_id: None,
            profile_token: profile.token,
            changed: false,
            before_sha256: plan.before_sha256,
            after_sha256: plan.after_sha256,
            backup_verified: false,
            committed: true,
            rolled_back: false,
        });
    }

    let locked_contents = read_localconfig(&profile.localconfig_path)?;
    if sha256(locked_contents.as_bytes()) != plan.before_sha256 {
        return Err("steam_config_plan_stale".to_string());
    }
    let operation_id = new_operation_id()?;
    let operation_root = operations_root.join(&operation_id);
    fs::create_dir(&operation_root).map_err(|_| "backup_failed".to_string())?;
    reject_symlink(&operation_root, "backup_failed")?;
    let backup_relative_path = format!("operations/{operation_id}/localconfig.vdf.before");
    let backup_path = operation_root.join("localconfig.vdf.before");
    let before_bytes = locked_contents.as_bytes();
    if let Err(code) = write_new_synced(&backup_path, before_bytes, "backup_failed") {
        let _ = fs::remove_dir_all(&operation_root);
        return Err(code);
    }
    let backup_bytes = match fs::read(&backup_path) {
        Ok(bytes) => bytes,
        Err(_) => {
            let _ = fs::remove_dir_all(&operation_root);
            return Err("backup_failed".to_string());
        }
    };
    if sha256(&backup_bytes) != plan.before_sha256 {
        let _ = fs::remove_dir_all(&operation_root);
        return Err("backup_verification_failed".to_string());
    }

    let timestamp = now_ms()?;
    let journal_path = journals_root.join(format!("{operation_id}.json"));
    let mut journal = SteamConfigJournal {
        schema_version: JOURNAL_SCHEMA_VERSION,
        operation_id: operation_id.clone(),
        profile_token: profile.token.clone(),
        before_sha256: plan.before_sha256.clone(),
        after_sha256: plan.after_sha256.clone(),
        phase: SteamConfigPhase::BackedUp,
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
        backup_relative_path,
        error_code: None,
    };
    if let Err(code) = write_journal(&journal_path, &journal) {
        let _ = fs::remove_dir_all(&operation_root);
        return Err(code);
    }

    let result = (|| {
        let temporary = temp_path(&profile.localconfig_path, &operation_id)?;
        write_new_synced(
            &temporary,
            plan.updated_contents.as_bytes(),
            "steam_config_write_failed",
        )?;
        let prepared = read_localconfig_unchecked(&temporary)?;
        let verified_plan = plan_managed_launch_option(&prepared)?;
        if sha256(prepared.as_bytes()) != plan.after_sha256 || verified_plan.changed {
            return Err("verification_failed".to_string());
        }
        journal.phase = SteamConfigPhase::Prepared;
        journal.updated_at_ms = now_ms()?;
        write_journal(&journal_path, &journal)?;
        if failure == FailurePoint::AfterPreparedJournal {
            return Err("injected_failure".to_string());
        }

        reject_symlink(&profile.localconfig_path, "steam_profile_invalid")?;
        let current = read_localconfig(&profile.localconfig_path)?;
        if sha256(current.as_bytes()) != plan.before_sha256 {
            return Err("steam_config_plan_stale".to_string());
        }
        atomic_replace(&profile.localconfig_path, &temporary)?;
        if failure == FailurePoint::AfterReplace {
            return Err("injected_failure".to_string());
        }
        let committed = read_localconfig(&profile.localconfig_path)?;
        if sha256(committed.as_bytes()) != plan.after_sha256 {
            restore_bytes(&profile.localconfig_path, &operation_id, &backup_bytes)?;
            return Err("verification_failed".to_string());
        }
        let mut committed_journal = journal.clone();
        committed_journal.phase = SteamConfigPhase::Committed;
        committed_journal.updated_at_ms = now_ms()?;
        write_journal(&journal_path, &committed_journal)?;
        journal = committed_journal;
        Ok(())
    })();

    if let Err(code) = result {
        if let Ok(temporary) = temp_path(&profile.localconfig_path, &operation_id) {
            if temporary.exists() && reject_symlink(&temporary, "steam_config_write_failed").is_ok()
            {
                let _ = fs::remove_file(temporary);
            }
        }
        journal.error_code = Some(code.clone());
        journal.updated_at_ms = now_ms().unwrap_or(journal.updated_at_ms);
        if journal.phase != SteamConfigPhase::Prepared {
            journal.phase = SteamConfigPhase::Failed;
        }
        let _ = write_journal(&journal_path, &journal);
        return Err(code);
    }

    Ok(SteamConfigReceipt {
        operation_id: Some(operation_id),
        profile_token: profile.token,
        changed: true,
        before_sha256: plan.before_sha256,
        after_sha256: plan.after_sha256,
        backup_verified: true,
        committed: true,
        rolled_back: false,
    })
}

fn read_localconfig_unchecked(path: &Path) -> Result<String, String> {
    reject_symlink(path, "steam_config_invalid")?;
    let metadata = fs::metadata(path).map_err(|_| "steam_config_invalid".to_string())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_LOCALCONFIG_BYTES {
        return Err("steam_config_invalid".to_string());
    }
    String::from_utf8(fs::read(path).map_err(|_| "steam_config_invalid".to_string())?)
        .map_err(|_| "steam_config_invalid".to_string())
}

fn backup_path(
    app_data_root: &Path,
    operations_root: &Path,
    journal: &SteamConfigJournal,
) -> Result<PathBuf, String> {
    let expected = format!("operations/{}/localconfig.vdf.before", journal.operation_id);
    if journal.backup_relative_path != expected {
        return Err("steam_journal_invalid".to_string());
    }
    let path = operations_root
        .join(&journal.operation_id)
        .join("localconfig.vdf.before");
    reject_symlink(&path, "steam_journal_invalid")?;
    let canonical_operations = operations_root
        .canonicalize()
        .map_err(|_| "steam_journal_invalid".to_string())?;
    let canonical_app_data = app_data_root
        .canonicalize()
        .map_err(|_| "steam_journal_invalid".to_string())?;
    let canonical = path
        .canonicalize()
        .map_err(|_| "steam_journal_invalid".to_string())?;
    if !canonical.starts_with(&canonical_operations) || !canonical.starts_with(&canonical_app_data)
    {
        return Err("steam_journal_invalid".to_string());
    }
    Ok(canonical)
}

pub fn rollback_platform_operation(
    app_data_root: &Path,
    request: SteamConfigOperationRequest,
) -> Result<SteamConfigReceipt, String> {
    rollback_operation(app_data_root, &platform_steam_roots(), request)
}

fn rollback_operation(
    app_data_root: &Path,
    roots: &[PathBuf],
    request: SteamConfigOperationRequest,
) -> Result<SteamConfigReceipt, String> {
    transaction_roots(app_data_root)?;
    let _lock = acquire_transaction_lock(app_data_root)?;
    rollback_operation_locked(app_data_root, roots, request)
}

fn rollback_operation_locked(
    app_data_root: &Path,
    roots: &[PathBuf],
    request: SteamConfigOperationRequest,
) -> Result<SteamConfigReceipt, String> {
    if !request.confirmed {
        return Err("steam_config_confirmation_required".to_string());
    }
    validate_operation_id(&request.operation_id)?;
    let (operations_root, journals_root) = transaction_roots(app_data_root)?;
    let journal_path = journals_root.join(format!("{}.json", request.operation_id));
    let mut journal = read_journal(&journal_path)?;
    if journal.operation_id != request.operation_id {
        return Err("steam_journal_invalid".to_string());
    }
    let profile = resolve_profile(roots, &journal.profile_token)?;
    let backup = backup_path(app_data_root, &operations_root, &journal)?;
    let backup_bytes = fs::read(backup).map_err(|_| "backup_failed".to_string())?;
    if sha256(&backup_bytes) != journal.before_sha256 {
        return Err("backup_verification_failed".to_string());
    }
    let current = read_localconfig(&profile.localconfig_path)?;
    let current_hash = sha256(current.as_bytes());
    if journal.phase == SteamConfigPhase::RolledBack && current_hash == journal.before_sha256 {
        return Ok(receipt_from_journal(&journal, true));
    }
    if current_hash != journal.after_sha256 && current_hash != journal.before_sha256 {
        return Err("steam_config_rollback_conflict".to_string());
    }
    if current_hash == journal.after_sha256 {
        restore_bytes(
            &profile.localconfig_path,
            &journal.operation_id,
            &backup_bytes,
        )?;
    }
    let temporary = temp_path(&profile.localconfig_path, &journal.operation_id)?;
    if temporary.exists() {
        reject_symlink(&temporary, "rollback_failed")?;
        fs::remove_file(&temporary).map_err(|_| "rollback_failed".to_string())?;
    }
    let restored = read_localconfig(&profile.localconfig_path)?;
    if sha256(restored.as_bytes()) != journal.before_sha256 {
        return Err("rollback_failed".to_string());
    }
    journal.phase = SteamConfigPhase::RolledBack;
    journal.updated_at_ms = now_ms()?;
    journal.error_code = None;
    write_journal(&journal_path, &journal)?;
    Ok(receipt_from_journal(&journal, true))
}

fn receipt_from_journal(journal: &SteamConfigJournal, rolled_back: bool) -> SteamConfigReceipt {
    SteamConfigReceipt {
        operation_id: Some(journal.operation_id.clone()),
        profile_token: journal.profile_token.clone(),
        changed: true,
        before_sha256: journal.before_sha256.clone(),
        after_sha256: journal.after_sha256.clone(),
        backup_verified: true,
        committed: journal.phase == SteamConfigPhase::Committed,
        rolled_back,
    }
}

pub fn recover_platform_operations(
    app_data_root: &Path,
) -> Result<Vec<SteamConfigReceipt>, String> {
    recover_operations(app_data_root, &platform_steam_roots())
}

fn recover_operations(
    app_data_root: &Path,
    roots: &[PathBuf],
) -> Result<Vec<SteamConfigReceipt>, String> {
    let (operations_root, journals_root) = transaction_roots(app_data_root)?;
    let _lock = acquire_transaction_lock(app_data_root)?;
    cleanup_orphan_operations(&operations_root, &journals_root)?;
    let operation_ids = pending_operations(&journals_root)?;
    let mut receipts = Vec::new();
    for operation_id in operation_ids {
        receipts.push(rollback_operation_locked(
            app_data_root,
            roots,
            SteamConfigOperationRequest {
                operation_id,
                confirmed: true,
            },
        )?);
    }
    Ok(receipts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "betterfy-steam-accounts-{name}-{}-{}",
            now_ms().expect("time"),
            OPERATION_COUNTER.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn local_config(options: &str) -> String {
        format!(
            "\"UserLocalConfigStore\"\n{{\n\t\"Software\"\n\t{{\n\t\t\"Valve\"\n\t\t{{\n\t\t\t\"Steam\"\n\t\t\t{{\n\t\t\t\t\"apps\"\n\t\t\t\t{{\n\t\t\t\t\t\"570\"\n\t\t\t\t\t{{\n\t\t\t\t\t\t\"LaunchOptions\"\t\t\"{options}\"\n\t\t\t\t\t}}\n\t\t\t\t}}\n\t\t\t}}\n\t\t}}\n\t}}\n}}\n"
        )
    }

    fn fixture(name: &str, account: &str, contents: &str) -> (PathBuf, PathBuf, PathBuf) {
        let base = temp_root(name);
        let steam = base.join("Steam");
        let localconfig = steam
            .join("userdata")
            .join(account)
            .join("config")
            .join("localconfig.vdf");
        fs::create_dir_all(localconfig.parent().expect("config parent")).expect("profile dirs");
        fs::write(&localconfig, contents).expect("localconfig");
        let app_data = base.join("app-data");
        (base, steam, app_data)
    }

    fn request_for(steam: &Path) -> ApplySteamLaunchOptionRequest {
        let roots = vec![steam.to_path_buf()];
        let profiles = discover_profiles_in_roots(&roots).expect("profiles");
        let preview = preview_profile(&roots, &profiles[0].token).expect("preview");
        ApplySteamLaunchOptionRequest {
            profile_token: preview.profile_token,
            confirmation_token: preview.confirmation_token,
            confirmed: true,
        }
    }

    #[test]
    fn discovers_only_numeric_real_profiles_without_exposing_ids() {
        let (base, steam, _) = fixture("discover", "76561198000000001", &local_config("-novid"));
        let ignored = steam.join("userdata").join("not-an-account").join("config");
        fs::create_dir_all(&ignored).expect("ignored dirs");
        fs::write(ignored.join("localconfig.vdf"), local_config("")).expect("ignored config");
        let summaries = list_profiles(&[steam]).expect("summaries");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].profile_index, 1);
        assert_eq!(summaries[0].status, SteamProfileStatus::Ready);
        let serialized = serde_json::to_string(&summaries).expect("serialize");
        assert!(!serialized.contains("76561198000000001"));
        assert!(!serialized.contains("localconfig"));
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn backs_up_commits_and_rolls_back_exact_bytes() {
        let original = local_config("-novid +exec autoexec.cfg");
        let (base, steam, app_data) = fixture("commit", "42", &original);
        let request = request_for(&steam);
        let receipt = apply_profile_with_failure(
            &app_data,
            std::slice::from_ref(&steam),
            request,
            FailurePoint::None,
        )
        .expect("apply");
        assert!(receipt.committed);
        let operation_id = receipt.operation_id.clone().expect("operation");
        let target = steam
            .join("userdata")
            .join("42")
            .join("config")
            .join("localconfig.vdf");
        let changed = fs::read_to_string(&target).expect("changed");
        assert!(changed.contains("-novid +exec autoexec.cfg -language dutch"));

        let journal_path = app_data
            .join("engine-v1/steam-config/journals")
            .join(format!("{operation_id}.json"));
        let journal_text = fs::read_to_string(&journal_path).expect("journal");
        assert!(!journal_text.contains("userdata"));
        assert!(!journal_text.contains("\"42\""));

        let rollback = rollback_operation(
            &app_data,
            std::slice::from_ref(&steam),
            SteamConfigOperationRequest {
                operation_id: operation_id.clone(),
                confirmed: true,
            },
        )
        .expect("rollback");
        assert!(rollback.rolled_back);
        assert_eq!(fs::read_to_string(&target).expect("restored"), original);
        rollback_operation(
            &app_data,
            &[steam],
            SteamConfigOperationRequest {
                operation_id,
                confirmed: true,
            },
        )
        .expect("idempotent rollback");
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn rejects_missing_confirmation_and_a_stale_plan_without_writing() {
        let original = local_config("-novid");
        let (base, steam, app_data) = fixture("stale", "99", &original);
        let mut request = request_for(&steam);
        request.confirmed = false;
        assert_eq!(
            apply_profile_with_failure(
                &app_data,
                std::slice::from_ref(&steam),
                request.clone(),
                FailurePoint::None,
            )
            .err()
            .as_deref(),
            Some("steam_config_confirmation_required")
        );
        request.confirmed = true;
        request.confirmation_token.push('0');
        assert_eq!(
            apply_profile_with_failure(
                &app_data,
                std::slice::from_ref(&steam),
                request,
                FailurePoint::None,
            )
            .err()
            .as_deref(),
            Some("steam_config_plan_stale")
        );
        let target = steam.join("userdata/99/config/localconfig.vdf");
        assert_eq!(fs::read_to_string(target).expect("unchanged"), original);
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn recovery_handles_interruptions_on_both_sides_of_replace() {
        for (name, failure) in [
            ("before-replace", FailurePoint::AfterPreparedJournal),
            ("after-replace", FailurePoint::AfterReplace),
        ] {
            let original = local_config("-novid");
            let (base, steam, app_data) = fixture(name, "7", &original);
            let request = request_for(&steam);
            assert_eq!(
                apply_profile_with_failure(
                    &app_data,
                    std::slice::from_ref(&steam),
                    request.clone(),
                    failure,
                )
                .err()
                .as_deref(),
                Some("injected_failure")
            );
            assert_eq!(
                apply_profile_with_failure(
                    &app_data,
                    std::slice::from_ref(&steam),
                    request,
                    FailurePoint::None,
                )
                .err()
                .as_deref(),
                Some("steam_recovery_required")
            );
            let receipts =
                recover_operations(&app_data, std::slice::from_ref(&steam)).expect("recover");
            assert_eq!(receipts.len(), 1);
            let target = steam.join("userdata/7/config/localconfig.vdf");
            assert_eq!(fs::read_to_string(target).expect("restored"), original);
            assert!(recover_operations(&app_data, &[steam])
                .expect("idempotent")
                .is_empty());
            fs::remove_dir_all(base).expect("cleanup");
        }
    }

    #[test]
    fn recovery_removes_only_orphaned_betterfy_operation_directories() {
        let (base, steam, app_data) = fixture("orphan", "8", &local_config("-novid"));
        let (operations, _) = transaction_roots(&app_data).expect("transaction roots");
        let orphan = operations.join("steam-op-1234567890-1-0");
        fs::create_dir(&orphan).expect("orphan operation");
        fs::write(orphan.join("localconfig.vdf.before"), b"orphaned backup")
            .expect("orphan backup");
        assert!(recover_operations(&app_data, &[steam])
            .expect("recover")
            .is_empty());
        assert!(!orphan.exists());
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_profile_files() {
        use std::os::unix::fs::symlink;

        let base = temp_root("symlink");
        let steam = base.join("Steam");
        let config = steam.join("userdata/12/config");
        fs::create_dir_all(&config).expect("config");
        let outside = base.join("outside.vdf");
        fs::write(&outside, local_config("-novid")).expect("outside");
        symlink(&outside, config.join("localconfig.vdf")).expect("symlink");
        assert!(discover_profiles_in_roots(&[steam])
            .expect("discovery")
            .is_empty());
        fs::remove_dir_all(base).expect("cleanup");
    }
}

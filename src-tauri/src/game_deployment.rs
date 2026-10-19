use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(not(target_os = "windows"))]
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::game_language::GameLanguage;
use crate::map_package;
use crate::vpk;

const SCHEMA_VERSION: u32 = 1;
/// The name every record written before the file name became a choice refers
/// to. Records without a name field mean this file.
const LEGACY_VPK_NAME: &str = "pak66_dir.vpk";
/// Numbers BetterFy picks from for a new install, in order of preference. 66 is
/// Minify's fixed output name and is never chosen. Dota loads `pak01`..`pak99`
/// and the lower number wins on a conflict (see ENGINE_ARCHITECTURE.md).
const PREFERRED_SLOT_NUMBERS: [u8; 16] = [
    65, 64, 63, 62, 61, 60, 59, 58, 57, 56, 55, 54, 53, 52, 51, 50,
];
/// Minify writes its output as `pak66_dir.vpk` plus `pak66_000.vpk`, ...
const MINIFY_SLOT_NUMBER: u8 = 66;
const MAX_SLOT_FILES: usize = 64;
const MAX_PACKAGE_BYTES: usize = 512 * 1024 * 1024;
/// Where a map package goes, below the language folder: `maps/dota.vpk`.
const MAPS_FOLDER: &str = "maps";
const MAP_FILE: &str = "dota.vpk";
const PAK_BACKUP: &str = "before.vpk";
const MAP_BACKUP: &str = "before-map.vpk";
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeployStagedVpkRequest {
    pub game_path: String,
    pub staged_operation_id: String,
    pub expected_plan_id: String,
    pub confirmed: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeploymentOperationRequest {
    pub game_path: String,
    pub operation_id: String,
    pub confirmed: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeploymentRecoveryRequest {
    pub game_path: String,
    pub confirmed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentPhase {
    BackedUp,
    Prepared,
    Committed,
    RolledBack,
    Failed,
}

/// What an operation did to `maps/dota.vpk`, as a receipt shows it.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MapReceipt {
    /// The path below the language folder, always `maps/dota.vpk`.
    pub file: &'static str,
    /// The map after the operation; `None` when the operation removed
    /// BetterFy's map.
    pub installed_sha256: Option<String>,
    pub before_sha256: Option<String>,
    pub backup_verified: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentReceipt {
    pub operation_id: String,
    pub language: GameLanguage,
    /// The previous `pakNN_dir.vpk` (the map has its own in `map`).
    pub before_sha256: Option<String>,
    /// The hash that identifies what was installed: the `pakNN_dir.vpk` when
    /// the build has one, otherwise the map package (a map-only build).
    pub installed_sha256: String,
    pub backup_verified: bool,
    pub committed: bool,
    pub rolled_back: bool,
    pub bundle_plan_id: Option<String>,
    pub package_ids: Vec<String>,
    /// The file BetterFy owns in the language folder, e.g. `pak65_dir.vpk`.
    /// Empty for a map-only build.
    pub vpk_name: String,
    /// Whether the build writes a `pakNN_dir.vpk` at all.
    pub has_pak: bool,
    /// The map package, when the operation involved one.
    pub map: Option<MapReceipt>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryReceipt {
    pub inspected: usize,
    pub rolled_back: usize,
    pub marked_failed: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentStressCapabilities {
    pub enabled: bool,
    pub failure_points: Vec<&'static str>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStressFailurePoint {
    AfterPrepared,
    AfterReplace,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapEvidence {
    pub file: &'static str,
    pub before_sha256: Option<String>,
    pub installed_sha256: Option<String>,
    pub backup_verified: bool,
    pub rollback_verified: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentEvidenceEntry {
    pub sequence: usize,
    pub language: GameLanguage,
    pub phase: DeploymentPhase,
    pub before_sha256: Option<String>,
    pub installed_sha256: String,
    pub backup_verified: bool,
    pub rollback_verified: bool,
    pub active: bool,
    pub bundle_plan_id: Option<String>,
    pub package_ids: Vec<String>,
    pub created_at_ms: u128,
    pub updated_at_ms: u128,
    pub error_code: Option<String>,
    pub vpk_name: String,
    pub has_pak: bool,
    pub map: Option<MapEvidence>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentEvidenceReport {
    pub schema_version: u32,
    pub app_version: String,
    pub platform: &'static str,
    pub generated_at_ms: u128,
    pub entries: Vec<DeploymentEvidenceEntry>,
}

#[derive(Clone, Debug)]
pub struct DeploymentDiagnosticCounts {
    pub total: usize,
    pub recoverable: usize,
}

/// What an operation did to the map file. Present only on operations that
/// involve one, so every journal written without a map is unchanged.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MapStep {
    /// The map after the operation. `None`: the operation removed the map
    /// BetterFy had installed (the new build has none).
    installed_sha256: Option<String>,
    /// The map BetterFy owned before the operation.
    before_sha256: Option<String>,
    /// `before-map.vpk` in the operation folder when there was a map before.
    backup_relative_path: Option<String>,
    /// This operation created the `maps` folder (it did not exist).
    #[serde(default)]
    maps_directory_created: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DeploymentJournal {
    schema_version: u32,
    operation_id: String,
    target_identity: String,
    #[serde(default)]
    language: GameLanguage,
    before_sha256: Option<String>,
    before_operation_id: Option<String>,
    /// The `pakNN_dir.vpk` after the operation. Empty when the build has none
    /// (`no_pak`).
    installed_sha256: String,
    #[serde(default)]
    bundle_plan_id: Option<String>,
    #[serde(default)]
    package_ids: Vec<String>,
    phase: DeploymentPhase,
    created_at_ms: u128,
    updated_at_ms: u128,
    backup_relative_path: Option<String>,
    #[serde(default)]
    rollback_verified: bool,
    #[serde(default)]
    locale_directory_created: bool,
    error_code: Option<String>,
    /// The pak file this operation wrote or removed. Empty when the operation
    /// never involved one (a map-only install on nothing).
    #[serde(default = "default_vpk_name")]
    vpk_name: String,
    /// The build has no `pakNN_dir.vpk`. Absent from every record of a build
    /// that has one.
    #[serde(default, skip_serializing_if = "is_false")]
    no_pak: bool,
    /// The map step; absent from every record of a build without a map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    map: Option<MapStep>,
}

fn default_vpk_name() -> String {
    LEGACY_VPK_NAME.to_string()
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OwnedMap {
    installed_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OwnershipState {
    schema_version: u32,
    target_identity: String,
    #[serde(default)]
    language: GameLanguage,
    /// The `pakNN_dir.vpk` BetterFy owns; empty when it owns only a map.
    installed_sha256: String,
    operation_id: String,
    #[serde(default)]
    bundle_plan_id: Option<String>,
    #[serde(default)]
    package_ids: Vec<String>,
    #[serde(default = "default_vpk_name")]
    vpk_name: String,
    #[serde(default, skip_serializing_if = "is_false")]
    no_pak: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    map: Option<OwnedMap>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DeploymentBundleIdentity {
    pub plan_id: Option<String>,
    pub package_ids: Vec<String>,
}

/// One verified file to place, with the hash it must have.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PayloadFile<'a> {
    pub bytes: &'a [u8],
    pub sha256: &'a str,
}

/// What an install writes: a `pakNN_dir.vpk`, a `maps/dota.vpk`, or both. A
/// part that is `None` is not part of the build; if BetterFy owned that file
/// from an earlier build, the install removes it (with a backup, like any
/// change).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DeploymentPayload<'a> {
    pub pak: Option<PayloadFile<'a>>,
    pub map: Option<PayloadFile<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailurePoint {
    None,
    AfterPrepared,
    AfterReplace,
    /// Test only: the `pakNN_dir.vpk` is published, the map is not yet.
    #[cfg(test)]
    BetweenPublishes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Pak,
    Map,
}

impl Kind {
    fn backup_name(self) -> &'static str {
        match self {
            Self::Pak => PAK_BACKUP,
            Self::Map => MAP_BACKUP,
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Self::Pak => "",
            Self::Map => "-map",
        }
    }
}

/// One file an operation changes: where it is, what BetterFy owned before
/// (the backup) and what the operation leaves there (`None`: nothing).
#[derive(Clone, Debug)]
struct Part {
    kind: Kind,
    target: PathBuf,
    before: Option<String>,
    after: Option<String>,
}

/// Where a file stands against an operation's before and after.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stand {
    /// As before the operation (or absent, when there was nothing before).
    Before,
    /// As the operation leaves it.
    After,
    /// Neither: someone else changed it.
    Other,
}

fn stand(current: &Option<String>, part: &Part) -> Stand {
    if *current == part.before {
        Stand::Before
    } else if *current == part.after {
        Stand::After
    } else {
        Stand::Other
    }
}

fn now_ms() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .map_err(|_| "deployment_failed".to_string())
}

fn new_operation_id() -> Result<String, String> {
    let count = OPERATION_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(format!(
        "deploy-op-{}-{}-{count}",
        now_ms()?,
        std::process::id()
    ))
}

fn validate_operation_id(value: &str) -> Result<(), String> {
    if value.len() < 10
        || value.len() > 96
        || !value.starts_with("deploy-op-")
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err("deployment_not_found".to_string());
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn reject_symlink(path: &Path) -> Result<(), String> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err("deployment_path_unsafe".to_string());
        }
    }
    Ok(())
}

/// Stricter than [`reject_symlink`]: also refuses a Windows reparse point (a
/// junction or any other redirect). Used for the `maps` folder and the map
/// file, where a redirect would send a whole-file write somewhere else.
fn reject_link(path: &Path) -> Result<(), String> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err("deployment_path_unsafe".to_string());
        }
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::MetadataExt;
            const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
            if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                return Err("deployment_path_unsafe".to_string());
            }
        }
    }
    Ok(())
}

fn owned_roots(app_data_root: &Path) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    reject_symlink(app_data_root)?;
    fs::create_dir_all(app_data_root).map_err(|_| "deployment_failed".to_string())?;
    let root = app_data_root.join("engine-v1").join("game-deployment");
    let operations = root.join("operations");
    let journals = root.join("journals");
    for path in [&root, &operations, &journals] {
        reject_symlink(path)?;
        fs::create_dir_all(path).map_err(|_| "deployment_failed".to_string())?;
        reject_symlink(path)?;
    }
    Ok((root, operations, journals))
}

/// `pakNN_dir.vpk` with a two-digit number: the only shape BetterFy ever
/// writes, and the only one a record may name. Anything else in a record is
/// refused so a damaged journal cannot point BetterFy at another file.
fn validate_vpk_name(name: &str) -> Result<(), String> {
    let bytes = name.as_bytes();
    if bytes.len() == 13
        && name.starts_with("pak")
        && name.ends_with("_dir.vpk")
        && bytes[3].is_ascii_digit()
        && bytes[4].is_ascii_digit()
    {
        Ok(())
    } else {
        Err("deployment_journal_invalid".to_string())
    }
}

/// The number in `pakNN_dir.vpk`, `!pakNN_dir.vpk` or `pakNN_<digits>.vpk`
/// (the chunk files of a split archive), compared case-insensitively because
/// Windows file names are. `None` for every other name.
fn slot_number(name: &str) -> Option<u8> {
    let lower = name.to_ascii_lowercase();
    let rest = lower.strip_prefix('!').unwrap_or(&lower);
    let rest = rest.strip_prefix("pak")?;
    let digits = rest.get(..2)?;
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let tail = rest.get(2..)?.strip_prefix('_')?.strip_suffix(".vpk")?;
    let is_dir = tail == "dir";
    let is_chunk = !tail.is_empty() && tail.bytes().all(|byte| byte.is_ascii_digit());
    if is_dir || is_chunk {
        digits.parse().ok()
    } else {
        None
    }
}

/// Numbers used by any entry of the language folder that Dota would load
/// (directories included, so nothing is ever written over a name in use).
fn taken_slot_numbers(locale: &Path) -> Result<std::collections::BTreeSet<u8>, String> {
    let mut taken = std::collections::BTreeSet::new();
    if !locale.is_dir() {
        return Ok(taken);
    }
    for entry in fs::read_dir(locale).map_err(|_| "deployment_failed".to_string())? {
        let entry = entry.map_err(|_| "deployment_failed".to_string())?;
        if let Some(number) = entry.file_name().to_str().and_then(slot_number) {
            taken.insert(number);
        }
    }
    Ok(taken)
}

/// The file name for a new install: the first number of the preference list
/// that nothing in the folder uses. `slot_full` when none is free.
fn allocate_vpk_name(locale: &Path) -> Result<String, String> {
    let taken = taken_slot_numbers(locale)?;
    PREFERRED_SLOT_NUMBERS
        .iter()
        .find(|number| !taken.contains(number))
        .map(|number| format!("pak{number:02}_dir.vpk"))
        .ok_or_else(|| "slot_full".to_string())
}

fn locale_for(dota_root: &Path, language: GameLanguage) -> Result<PathBuf, String> {
    reject_symlink(dota_root)?;
    let canonical = dota_root
        .canonicalize()
        .map_err(|_| "invalid_game_path".to_string())?;
    let executable = if cfg!(target_os = "windows") {
        canonical.join("game/bin/win64/dota2.exe")
    } else if cfg!(target_os = "macos") {
        canonical.join("game/bin/osx64/dota2")
    } else {
        canonical.join("game/bin/linuxsteamrt64/dota2")
    };
    if !canonical.is_dir()
        || canonical
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| !value.eq_ignore_ascii_case("dota 2 beta"))
            .unwrap_or(true)
        || !canonical.join("game/dota/pak01_dir.vpk").is_file()
        || !executable.is_file()
    {
        return Err("invalid_game_path".to_string());
    }
    let game = canonical.join("game");
    reject_symlink(&game)?;
    let locale = game.join(format!("dota_{}", language.suffix()));
    reject_symlink(&locale)?;
    Ok(locale)
}

fn target_for(dota_root: &Path, language: GameLanguage, vpk_name: &str) -> Result<PathBuf, String> {
    validate_vpk_name(vpk_name)?;
    let target = locale_for(dota_root, language)?.join(vpk_name);
    reject_symlink(&target)?;
    Ok(target)
}

/// The `maps` folder and `maps/dota.vpk` of a language folder. Neither is
/// created; a link or redirect at either is refused.
fn map_paths(dota_root: &Path, language: GameLanguage) -> Result<(PathBuf, PathBuf), String> {
    let folder = locale_for(dota_root, language)?.join(MAPS_FOLDER);
    reject_link(&folder)?;
    let file = folder.join(MAP_FILE);
    reject_link(&file)?;
    Ok((folder, file))
}

fn target_identity(dota_root: &Path) -> Result<String, String> {
    let canonical = dota_root
        .canonicalize()
        .map_err(|_| "invalid_game_path".to_string())?;
    Ok(format!(
        "sha256:{}",
        sha256(canonical.to_string_lossy().as_bytes())
    ))
}

fn write_new_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|_| "deployment_write_failed".to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "deployment_write_failed".to_string())
}

fn atomic_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    reject_symlink(path)?;
    let temporary = path.with_extension("json.tmp");
    reject_symlink(&temporary)?;
    if temporary.exists() {
        fs::remove_file(&temporary).map_err(|_| "deployment_failed".to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| "deployment_failed".to_string())?;
    write_new_synced(&temporary, &bytes)?;
    publish(path, &temporary, path.exists()).map_err(|_| "deployment_failed".to_string())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    reject_symlink(path)?;
    let metadata = fs::metadata(path).map_err(|_| "deployment_not_found".to_string())?;
    if metadata.len() > 1024 * 1024 {
        return Err("deployment_journal_invalid".to_string());
    }
    serde_json::from_slice(&fs::read(path).map_err(|_| "deployment_journal_invalid".to_string())?)
        .map_err(|_| "deployment_journal_invalid".to_string())
}

#[cfg(target_os = "windows")]
struct TransactionLock {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(target_os = "windows")]
impl Drop for TransactionLock {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.handle) };
    }
}

#[cfg(target_os = "windows")]
fn transaction_lock(root: &Path) -> Result<TransactionLock, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_HIDDEN, OPEN_ALWAYS,
    };
    let path = root
        .join("write.lock")
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            std::ptr::null(),
            OPEN_ALWAYS,
            FILE_ATTRIBUTE_HIDDEN,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err("deployment_locked".to_string());
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
fn transaction_lock(_root: &Path) -> Result<TransactionLock, String> {
    TRANSACTION_LOCK
        .lock()
        .map(|guard| TransactionLock { _guard: guard })
        .map_err(|_| "deployment_locked".to_string())
}

#[cfg(target_os = "windows")]
fn publish(target: &Path, replacement: &Path, target_exists: bool) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, ReplaceFileW, MOVEFILE_WRITE_THROUGH, REPLACEFILE_WRITE_THROUGH,
    };
    let target = target
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let replacement = replacement
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        if target_exists {
            ReplaceFileW(
                target.as_ptr(),
                replacement.as_ptr(),
                std::ptr::null(),
                REPLACEFILE_WRITE_THROUGH,
                std::ptr::null(),
                std::ptr::null(),
            )
        } else {
            MoveFileExW(
                replacement.as_ptr(),
                target.as_ptr(),
                MOVEFILE_WRITE_THROUGH,
            )
        }
    };
    if result == 0 {
        return Err("deployment_commit_failed".to_string());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn publish(target: &Path, replacement: &Path, _target_exists: bool) -> Result<(), String> {
    fs::rename(replacement, target).map_err(|_| "deployment_commit_failed".to_string())?;
    if let Some(parent) = target.parent() {
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| "deployment_commit_failed".to_string())?;
    }
    Ok(())
}

fn verify_package(bytes: &[u8], expected_sha256: &str) -> Result<String, String> {
    if bytes.is_empty() || bytes.len() > MAX_PACKAGE_BYTES || expected_sha256.len() != 64 {
        return Err("package_invalid".to_string());
    }
    vpk::inspect(bytes).map_err(|_| "package_invalid".to_string())?;
    let actual = sha256(bytes);
    if actual != expected_sha256 {
        return Err("package_hash_mismatch".to_string());
    }
    Ok(actual)
}

/// A map package: the right hash, and a well-formed whole-map VPK (see
/// `map_package`). Anything else is refused before the game folder is read.
fn verify_map(bytes: &[u8], expected_sha256: &str) -> Result<String, String> {
    if bytes.is_empty() || bytes.len() > map_package::MAX_MAP_BYTES || expected_sha256.len() != 64 {
        return Err("map_package_invalid".to_string());
    }
    let actual = sha256(bytes);
    if actual != expected_sha256 {
        return Err("map_hash_mismatch".to_string());
    }
    map_package::analyze(bytes).map_err(String::from)?;
    Ok(actual)
}

fn verify_kind(kind: Kind, bytes: &[u8], expected_sha256: &str) -> Result<String, String> {
    match kind {
        Kind::Pak => verify_package(bytes, expected_sha256),
        Kind::Map => verify_map(bytes, expected_sha256),
    }
}

fn ownership_path(root: &Path) -> PathBuf {
    root.join("ownership.json")
}

fn remove_dir_if_empty(dir: &Path) -> Result<(), String> {
    reject_link(dir)?;
    if !dir.is_dir() {
        return Ok(());
    }
    let mut entries = fs::read_dir(dir).map_err(|_| "rollback_failed".to_string())?;
    if entries.next().is_none() {
        fs::remove_dir(dir).map_err(|_| "rollback_failed".to_string())?;
    }
    Ok(())
}

fn remove_locale_if_empty(locale: &Path) -> Result<(), String> {
    reject_symlink(locale)?;
    remove_dir_if_empty(locale)
}

/// What sits at a path BetterFy may own.
enum Probe {
    Absent,
    File(Vec<u8>),
    /// A directory, a device or a file too large to be one of BetterFy's.
    Other,
}

fn probe(path: &Path, limit: usize) -> Result<Probe, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Probe::Absent),
        Err(_) => return Err("deployment_failed".to_string()),
    };
    if metadata.file_type().is_symlink() {
        return Err("deployment_path_unsafe".to_string());
    }
    if !metadata.is_file() || metadata.len() > limit as u64 {
        return Ok(Probe::Other);
    }
    fs::read(path)
        .map(Probe::File)
        .map_err(|_| "deployment_failed".to_string())
}

fn limit_of(kind: Kind) -> usize {
    match kind {
        Kind::Pak => MAX_PACKAGE_BYTES,
        Kind::Map => map_package::MAX_MAP_BYTES,
    }
}

/// The hash of a part's file now, with anything that is not a plain file
/// counted as "someone else's".
fn current_hash(part: &Part) -> Result<Option<Option<String>>, String> {
    match probe(&part.target, limit_of(part.kind))? {
        Probe::Absent => Ok(Some(None)),
        Probe::File(bytes) => Ok(Some(Some(sha256(&bytes)))),
        Probe::Other => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------

impl DeploymentJournal {
    /// The `pakNN_dir.vpk` this operation leaves in the folder.
    fn pak_after(&self) -> Option<(&str, &str)> {
        (!self.no_pak).then_some((self.vpk_name.as_str(), self.installed_sha256.as_str()))
    }

    /// The `pakNN_dir.vpk` BetterFy owned before this operation.
    fn pak_before(&self) -> Option<(&str, &str)> {
        self.before_sha256
            .as_deref()
            .map(|sha| (self.vpk_name.as_str(), sha))
    }

    fn map_after(&self) -> Option<&str> {
        self.map
            .as_ref()
            .and_then(|step| step.installed_sha256.as_deref())
    }

    fn map_before(&self) -> Option<&str> {
        self.map
            .as_ref()
            .and_then(|step| step.before_sha256.as_deref())
    }

    /// The hash that identifies what the operation installed.
    fn primary_sha256(&self) -> String {
        match self.pak_after() {
            Some((_, sha)) => sha.to_string(),
            None => self.map_after().unwrap_or_default().to_string(),
        }
    }

    /// Refuses a record whose fields contradict each other, so a damaged or
    /// hand-edited journal cannot point BetterFy at a file it never wrote.
    fn validate_shape(&self) -> Result<(), String> {
        let invalid = || Err("deployment_journal_invalid".to_string());
        let pak_involved = self.pak_after().is_some() || self.before_sha256.is_some();
        if pak_involved {
            validate_vpk_name(&self.vpk_name)?;
        } else if !self.vpk_name.is_empty() {
            return invalid();
        }
        if self.no_pak && !self.installed_sha256.is_empty() {
            return invalid();
        }
        if self.before_sha256.is_some()
            != (self.backup_relative_path.as_deref() == Some(PAK_BACKUP))
            || (self.backup_relative_path.is_some() && self.before_sha256.is_none())
        {
            return invalid();
        }
        if let Some(step) = &self.map {
            if step.installed_sha256.is_none() && step.before_sha256.is_none() {
                return invalid();
            }
            if step.before_sha256.is_some()
                != (step.backup_relative_path.as_deref() == Some(MAP_BACKUP))
                || (step.backup_relative_path.is_some() && step.before_sha256.is_none())
            {
                return invalid();
            }
        }
        // Something owned before the operation exists exactly when the
        // operation replaced an earlier one.
        let any_before = self.before_sha256.is_some() || self.map_before().is_some();
        if any_before != self.before_operation_id.is_some() {
            return invalid();
        }
        if self.pak_after().is_none() && self.map_after().is_none() {
            return invalid();
        }
        Ok(())
    }

    /// The files this operation changes.
    fn parts(&self, dota_root: &Path) -> Result<Vec<Part>, String> {
        self.validate_shape()?;
        let mut parts = Vec::new();
        if self.pak_after().is_some() || self.before_sha256.is_some() {
            parts.push(Part {
                kind: Kind::Pak,
                target: target_for(dota_root, self.language, &self.vpk_name)?,
                before: self.before_sha256.clone(),
                after: self.pak_after().map(|(_, sha)| sha.to_string()),
            });
        }
        if let Some(step) = &self.map {
            parts.push(Part {
                kind: Kind::Map,
                target: map_paths(dota_root, self.language)?.1,
                before: step.before_sha256.clone(),
                after: step.installed_sha256.clone(),
            });
        }
        Ok(parts)
    }

    fn receipt(
        &self,
        committed: bool,
        rolled_back: bool,
        backup_verified: bool,
    ) -> DeploymentReceipt {
        DeploymentReceipt {
            operation_id: self.operation_id.clone(),
            language: self.language,
            before_sha256: self.before_sha256.clone(),
            installed_sha256: self.primary_sha256(),
            backup_verified,
            committed,
            rolled_back,
            bundle_plan_id: self.bundle_plan_id.clone(),
            package_ids: self.package_ids.clone(),
            vpk_name: self
                .pak_after()
                .map(|(name, _)| name.to_string())
                .unwrap_or_default(),
            has_pak: !self.no_pak,
            map: self.map.as_ref().map(|step| MapReceipt {
                file: map_package::MAP_RELATIVE_PATH,
                installed_sha256: step.installed_sha256.clone(),
                before_sha256: step.before_sha256.clone(),
                backup_verified,
            }),
        }
    }
}

impl OwnershipState {
    fn pak(&self) -> Option<(&str, &str)> {
        (!self.no_pak).then_some((self.vpk_name.as_str(), self.installed_sha256.as_str()))
    }

    fn map_sha256(&self) -> Option<&str> {
        self.map.as_ref().map(|map| map.installed_sha256.as_str())
    }

    fn validate(&self, identity: &str) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION || self.target_identity != identity {
            return Err("deployment_journal_invalid".to_string());
        }
        if self.no_pak {
            if !self.vpk_name.is_empty() || !self.installed_sha256.is_empty() || self.map.is_none()
            {
                return Err("deployment_journal_invalid".to_string());
            }
        } else {
            validate_vpk_name(&self.vpk_name)?;
        }
        Ok(())
    }

    /// The state a committed journal leaves behind.
    fn from_journal(journal: &DeploymentJournal) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            target_identity: journal.target_identity.clone(),
            language: journal.language,
            installed_sha256: journal.installed_sha256.clone(),
            operation_id: journal.operation_id.clone(),
            bundle_plan_id: journal.bundle_plan_id.clone(),
            package_ids: journal.package_ids.clone(),
            vpk_name: if journal.no_pak {
                String::new()
            } else {
                journal.vpk_name.clone()
            },
            no_pak: journal.no_pak,
            map: journal.map_after().map(|sha| OwnedMap {
                installed_sha256: sha.to_string(),
            }),
        }
    }
}

/// The map and pak targets of the install BetterFy owns, for every check that
/// compares the folder with the record.
fn owned_parts(dota_root: &Path, state: &OwnershipState) -> Result<Vec<Part>, String> {
    let mut parts = Vec::new();
    if let Some((name, sha)) = state.pak() {
        parts.push(Part {
            kind: Kind::Pak,
            target: target_for(dota_root, state.language, name)?,
            before: Some(sha.to_string()),
            after: Some(sha.to_string()),
        });
    }
    if let Some(sha) = state.map_sha256() {
        parts.push(Part {
            kind: Kind::Map,
            target: map_paths(dota_root, state.language)?.1,
            before: Some(sha.to_string()),
            after: Some(sha.to_string()),
        });
    }
    Ok(parts)
}

// ---------------------------------------------------------------------------
// Install
// ---------------------------------------------------------------------------

/// Everything the install learned about one file before it changes anything.
struct Planned {
    part: Part,
    /// The owned bytes to back up.
    backup: Option<Vec<u8>>,
    /// The file to place; `None` removes the owned file.
    payload: Option<Vec<u8>>,
}

fn deploy_payload_with_failure(
    app_data_root: &Path,
    dota_root: &Path,
    payload: DeploymentPayload<'_>,
    language: GameLanguage,
    bundle: DeploymentBundleIdentity,
    failure: FailurePoint,
) -> Result<DeploymentReceipt, String> {
    if payload.pak.is_none() && payload.map.is_none() {
        return Err("package_invalid".to_string());
    }
    let pak_sha = payload
        .pak
        .map(|file| verify_package(file.bytes, file.sha256))
        .transpose()?;
    let map_sha = payload
        .map
        .map(|file| verify_map(file.bytes, file.sha256))
        .transpose()?;
    let locale_path = locale_for(dota_root, language)?;
    let identity = target_identity(dota_root)?;
    let (root, operations, journals) = owned_roots(app_data_root)?;
    let _lock = transaction_lock(&root)?;

    // An install BetterFy already owns keeps its file names. A new install
    // takes the first free number for its pak, so a file another program put
    // in the folder (Minify's pak66) is never the one BetterFy writes over.
    let owned = if ownership_path(&root).exists() {
        let state: OwnershipState = read_json(&ownership_path(&root))?;
        state.validate(&identity)?;
        if state.language != language {
            return Err("deployment_language_change_requires_restore".to_string());
        }
        Some(state)
    } else {
        None
    };
    let owned_pak = owned.as_ref().and_then(|state| state.pak());
    let owned_map = owned.as_ref().and_then(|state| state.map_sha256());

    // The pak file: BetterFy's recorded one, or the first free name.
    let vpk_name = match (owned_pak, payload.pak) {
        (Some((name, _)), _) => name.to_string(),
        (None, Some(_)) => allocate_vpk_name(&locale_path)?,
        (None, None) => String::new(),
    };
    let mut planned: Vec<Planned> = Vec::new();
    if owned_pak.is_some() || payload.pak.is_some() {
        let target = target_for(dota_root, language, &vpk_name)?;
        let before = match owned_pak {
            Some((_, owned_sha)) => {
                if !target.is_file() {
                    return Err("deployment_conflict".to_string());
                }
                let bytes = fs::read(&target).map_err(|_| "deployment_failed".to_string())?;
                if sha256(&bytes) != owned_sha {
                    return Err("deployment_target_foreign".to_string());
                }
                Some(bytes)
            }
            None => None,
        };
        planned.push(Planned {
            part: Part {
                kind: Kind::Pak,
                target,
                before: owned_pak.map(|(_, sha)| sha.to_string()),
                after: pak_sha.clone(),
            },
            backup: before,
            payload: payload.pak.map(|file| file.bytes.to_vec()),
        });
    }

    // The map file. BetterFy writes `maps/dota.vpk` only when it is absent or
    // is the exact file BetterFy wrote before; any other file there (Valve's,
    // another tool's, the player's) stops the install, because replacing it
    // could not be undone with a proven result.
    if owned_map.is_some() || payload.map.is_some() {
        let (maps_dir, map_target) = map_paths(dota_root, language)?;
        if maps_dir.exists() && !maps_dir.is_dir() {
            return Err("map_folder_invalid".to_string());
        }
        let before = match owned_map {
            Some(owned_sha) => match probe(&map_target, map_package::MAX_MAP_BYTES)? {
                Probe::File(bytes) if sha256(&bytes) == owned_sha => Some(bytes),
                _ => return Err("map_conflict".to_string()),
            },
            None => {
                match probe(&map_target, map_package::MAX_MAP_BYTES)? {
                    Probe::Absent => {}
                    _ => return Err("map_target_foreign".to_string()),
                }
                None
            }
        };
        planned.push(Planned {
            part: Part {
                kind: Kind::Map,
                target: map_target,
                before: owned_map.map(str::to_string),
                after: map_sha.clone(),
            },
            backup: before,
            payload: payload.map.map(|file| file.bytes.to_vec()),
        });
    }

    let locale = locale_path.as_path();
    reject_symlink(locale)?;
    let locale_directory_created = !locale.exists();
    fs::create_dir_all(locale).map_err(|_| "deployment_failed".to_string())?;
    reject_symlink(locale)?;
    let mut maps_directory_created = false;
    if payload.map.is_some() {
        let maps_dir = locale.join(MAPS_FOLDER);
        reject_link(&maps_dir)?;
        if !maps_dir.exists() {
            maps_directory_created = true;
            fs::create_dir(&maps_dir).map_err(|_| "deployment_failed".to_string())?;
        }
        reject_link(&maps_dir)?;
    }
    for item in &planned {
        reject_symlink(&item.part.target)?;
    }

    let operation_id = new_operation_id()?;
    let operation_root = operations.join(&operation_id);
    fs::create_dir(&operation_root).map_err(|_| "deployment_failed".to_string())?;
    // Backups first, each read back and compared, before any journal says the
    // operation exists.
    for item in &planned {
        if let (Some(bytes), Some(hash)) = (&item.backup, &item.part.before) {
            let backup_path = operation_root.join(item.part.kind.backup_name());
            write_new_synced(&backup_path, bytes)?;
            if sha256(&fs::read(&backup_path).map_err(|_| "backup_failed".to_string())?) != *hash {
                return Err("backup_verification_failed".to_string());
            }
        }
    }
    let part_of = |kind: Kind| planned.iter().find(|item| item.part.kind == kind);
    let pak_before = part_of(Kind::Pak).and_then(|item| item.part.before.clone());
    let map_before = part_of(Kind::Map).and_then(|item| item.part.before.clone());
    let timestamp = now_ms()?;
    let journal_path = journals.join(format!("{operation_id}.json"));
    let mut journal = DeploymentJournal {
        schema_version: SCHEMA_VERSION,
        operation_id: operation_id.clone(),
        target_identity: identity.clone(),
        language,
        before_sha256: pak_before.clone(),
        before_operation_id: owned.as_ref().map(|state| state.operation_id.clone()),
        installed_sha256: pak_sha.clone().unwrap_or_default(),
        bundle_plan_id: bundle.plan_id.clone(),
        package_ids: bundle.package_ids.clone(),
        phase: DeploymentPhase::BackedUp,
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
        backup_relative_path: pak_before.as_ref().map(|_| PAK_BACKUP.to_string()),
        rollback_verified: false,
        locale_directory_created,
        error_code: None,
        vpk_name: vpk_name.clone(),
        no_pak: pak_sha.is_none(),
        map: (owned_map.is_some() || payload.map.is_some()).then(|| MapStep {
            installed_sha256: map_sha.clone(),
            before_sha256: map_before.clone(),
            backup_relative_path: map_before.as_ref().map(|_| MAP_BACKUP.to_string()),
            maps_directory_created,
        }),
    };
    journal.validate_shape()?;
    atomic_json(&journal_path, &journal)?;

    // Temporary copies next to each target, written, read back and verified.
    let mut temporaries: Vec<(usize, PathBuf)> = Vec::new();
    for (index, item) in planned.iter().enumerate() {
        let Some(bytes) = &item.payload else { continue };
        let expected = item.part.after.as_deref().unwrap_or_default();
        let temporary = item
            .part
            .target
            .parent()
            .ok_or_else(|| "deployment_failed".to_string())?
            .join(format!(
                ".betterfy-{operation_id}{}.tmp",
                item.part.kind.tag()
            ));
        write_new_synced(&temporary, bytes)?;
        temporaries.push((index, temporary.clone()));
        let written = fs::read(&temporary).map_err(|_| "deployment_write_failed".to_string());
        if written
            .and_then(|written| verify_kind(item.part.kind, &written, expected))
            .is_err()
        {
            for (_, temporary) in &temporaries {
                let _ = fs::remove_file(temporary);
            }
            return Err("verification_failed".to_string());
        }
    }
    journal.phase = DeploymentPhase::Prepared;
    journal.updated_at_ms = now_ms()?;
    atomic_json(&journal_path, &journal)?;
    if failure == FailurePoint::AfterPrepared {
        for (_, temporary) in &temporaries {
            let _ = fs::remove_file(temporary);
        }
        return Err("injected_failure".to_string());
    }
    // Publish: the pak first, then the map. A stop between the two leaves the
    // journal in `Prepared`, and recovery puts both files back.
    for (position, item) in planned.iter().enumerate() {
        match temporaries.iter().find(|(index, _)| *index == position) {
            Some((_, temporary)) => {
                publish(&item.part.target, temporary, item.part.before.is_some())?
            }
            None => fs::remove_file(&item.part.target)
                .map_err(|_| "deployment_commit_failed".to_string())?,
        }
        #[cfg(test)]
        if failure == FailurePoint::BetweenPublishes && item.part.kind == Kind::Pak {
            return Err("injected_failure".to_string());
        }
    }
    if failure == FailurePoint::AfterReplace {
        return Err("injected_failure".to_string());
    }
    for item in &planned {
        match &item.part.after {
            Some(expected) => {
                verify_kind(
                    item.part.kind,
                    &fs::read(&item.part.target).map_err(|_| "verification_failed".to_string())?,
                    expected,
                )?;
            }
            None => {
                if item.part.target.exists() {
                    return Err("verification_failed".to_string());
                }
            }
        }
    }
    atomic_json(
        &ownership_path(&root),
        &OwnershipState::from_journal(&journal),
    )?;
    journal.phase = DeploymentPhase::Committed;
    journal.updated_at_ms = now_ms()?;
    atomic_json(&journal_path, &journal)?;
    Ok(journal.receipt(true, false, true))
}

#[cfg(test)]
fn deploy_with_failure(
    app_data_root: &Path,
    dota_root: &Path,
    package: &[u8],
    expected_sha256: &str,
    language: GameLanguage,
    bundle: DeploymentBundleIdentity,
    failure: FailurePoint,
) -> Result<DeploymentReceipt, String> {
    deploy_payload_with_failure(
        app_data_root,
        dota_root,
        DeploymentPayload {
            pak: Some(PayloadFile {
                bytes: package,
                sha256: expected_sha256,
            }),
            map: None,
        },
        language,
        bundle,
        failure,
    )
}

pub(crate) fn deploy_verified_vpk(
    app_data_root: &Path,
    dota_root: &Path,
    package: &[u8],
    expected_sha256: &str,
) -> Result<DeploymentReceipt, String> {
    deploy_verified_payload_for_language(
        app_data_root,
        dota_root,
        DeploymentPayload {
            pak: Some(PayloadFile {
                bytes: package,
                sha256: expected_sha256,
            }),
            map: None,
        },
        GameLanguage::Dutch,
        DeploymentBundleIdentity::default(),
    )
}

pub(crate) fn verify_language_folder(
    dota_root: &Path,
    language: GameLanguage,
) -> Result<(), String> {
    let folder = dota_root
        .join("game")
        .join(format!("dota_{}", language.suffix()));
    reject_symlink(&folder)?;
    if folder.exists() && !folder.is_dir() {
        return Err("language_folder_unavailable".to_string());
    }
    Ok(())
}

/// Read-only: the checks an install would make against the folder, run before
/// Steam and Dota are shut down. `with_map` says whether the build carries a
/// map. Creates nothing.
pub(crate) fn preview_destination(
    app_data_root: &Path,
    dota_root: &Path,
    language: GameLanguage,
    with_map: bool,
) -> Result<(), String> {
    verify_language_folder(dota_root, language)?;
    let locale = locale_for(dota_root, language)?;
    let identity = target_identity(dota_root)?;
    let (root, _, _) = owned_roots(app_data_root)?;
    let _lock = transaction_lock(&root)?;
    let ownership_file = ownership_path(&root);
    let mut owns_map = false;
    if ownership_file.exists() {
        let state: OwnershipState = read_json(&ownership_file)?;
        state.validate(&identity)?;
        if state.language != language {
            return Err("deployment_language_change_requires_restore".to_string());
        }
        if let Some((name, owned_sha)) = state.pak() {
            let target = target_for(dota_root, language, name)?;
            if !target.is_file()
                || sha256(&fs::read(&target).map_err(|_| "deployment_conflict".to_string())?)
                    != owned_sha
            {
                return Err("deployment_conflict".to_string());
            }
        }
        if let Some(owned_sha) = state.map_sha256() {
            owns_map = true;
            let (_, map_target) = map_paths(dota_root, language)?;
            match probe(&map_target, map_package::MAX_MAP_BYTES)? {
                Probe::File(bytes) if sha256(&bytes) == owned_sha => {}
                _ => return Err("map_conflict".to_string()),
            }
        }
        if state.pak().is_none() {
            // A pak the build adds still needs a free name.
            allocate_vpk_name(&locale)?;
        }
    } else {
        // A new install never needs a file another program owns; it only
        // needs a free name (`slot_full` when there is none).
        allocate_vpk_name(&locale)?;
    }
    if with_map && !owns_map {
        let (maps_dir, map_target) = map_paths(dota_root, language)?;
        if maps_dir.exists() && !maps_dir.is_dir() {
            return Err("map_folder_invalid".to_string());
        }
        if !matches!(
            probe(&map_target, map_package::MAX_MAP_BYTES)?,
            Probe::Absent
        ) {
            return Err("map_target_foreign".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn preview_language_destination(
    app_data_root: &Path,
    dota_root: &Path,
    language: GameLanguage,
) -> Result<(), String> {
    preview_destination(app_data_root, dota_root, language, false)
}

pub(crate) fn deploy_verified_payload_for_language(
    app_data_root: &Path,
    dota_root: &Path,
    payload: DeploymentPayload<'_>,
    language: GameLanguage,
    bundle: DeploymentBundleIdentity,
) -> Result<DeploymentReceipt, String> {
    verify_language_folder(dota_root, language)?;
    deploy_payload_with_failure(
        app_data_root,
        dota_root,
        payload,
        language,
        bundle,
        FailurePoint::None,
    )
}

#[cfg(test)]
pub(crate) fn deploy_verified_bundle_for_language(
    app_data_root: &Path,
    dota_root: &Path,
    package: &[u8],
    expected_sha256: &str,
    language: GameLanguage,
    bundle: DeploymentBundleIdentity,
) -> Result<DeploymentReceipt, String> {
    deploy_verified_payload_for_language(
        app_data_root,
        dota_root,
        DeploymentPayload {
            pak: Some(PayloadFile {
                bytes: package,
                sha256: expected_sha256,
            }),
            map: None,
        },
        language,
        bundle,
    )
}

/// Stress controls need both the compiled stop points and an account the auth
/// service marks as a developer.
pub(crate) fn deployment_stress_capabilities(developer: bool) -> DeploymentStressCapabilities {
    let enabled = cfg!(feature = "internal-stress-test") && developer;
    DeploymentStressCapabilities {
        enabled,
        failure_points: if enabled {
            vec!["after_prepared", "after_replace"]
        } else {
            Vec::new()
        },
    }
}

pub(crate) fn deploy_verified_payload_for_language_stress(
    app_data_root: &Path,
    dota_root: &Path,
    payload: DeploymentPayload<'_>,
    language: GameLanguage,
    bundle: DeploymentBundleIdentity,
    failure: DeploymentStressFailurePoint,
) -> Result<DeploymentReceipt, String> {
    #[cfg(feature = "internal-stress-test")]
    {
        verify_language_folder(dota_root, language)?;
        let failure = match failure {
            DeploymentStressFailurePoint::AfterPrepared => FailurePoint::AfterPrepared,
            DeploymentStressFailurePoint::AfterReplace => FailurePoint::AfterReplace,
        };
        deploy_payload_with_failure(app_data_root, dota_root, payload, language, bundle, failure)
    }
    #[cfg(not(feature = "internal-stress-test"))]
    {
        let _ = (app_data_root, dota_root, payload, language, bundle, failure);
        Err("stress_test_disabled".to_string())
    }
}

#[cfg(test)]
pub(crate) fn deploy_verified_vpk_for_language(
    app_data_root: &Path,
    dota_root: &Path,
    package: &[u8],
    expected_sha256: &str,
    language: GameLanguage,
) -> Result<DeploymentReceipt, String> {
    deploy_verified_bundle_for_language(
        app_data_root,
        dota_root,
        package,
        expected_sha256,
        language,
        DeploymentBundleIdentity::default(),
    )
}

#[cfg(test)]
pub(crate) fn current_verified_deployment(
    app_data_root: &Path,
    dota_root: &Path,
    expected_sha256: &str,
) -> Result<Option<DeploymentReceipt>, String> {
    current_owned_deployment(app_data_root, dota_root)
        .map(|receipt| receipt.filter(|receipt| receipt.installed_sha256 == expected_sha256))
}

pub(crate) fn current_owned_deployment(
    app_data_root: &Path,
    dota_root: &Path,
) -> Result<Option<DeploymentReceipt>, String> {
    let identity = target_identity(dota_root)?;
    let (root, operations, journals) = owned_roots(app_data_root)?;
    let _lock = transaction_lock(&root)?;
    let ownership_file = ownership_path(&root);
    if !ownership_file.exists() {
        return Ok(None);
    }
    let ownership: OwnershipState = read_json(&ownership_file)?;
    ownership.validate(&identity)?;
    validate_operation_id(&ownership.operation_id)?;
    let journal: DeploymentJournal =
        read_json(&journals.join(format!("{}.json", ownership.operation_id)))?;
    journal.validate_shape()?;
    let from_journal = OwnershipState::from_journal(&journal);
    if journal.schema_version != SCHEMA_VERSION
        || journal.operation_id != ownership.operation_id
        || journal.target_identity != identity
        || journal.language != ownership.language
        || from_journal.installed_sha256 != ownership.installed_sha256
        || from_journal.vpk_name != ownership.vpk_name
        || from_journal.no_pak != ownership.no_pak
        || from_journal.map != ownership.map
        || journal.bundle_plan_id != ownership.bundle_plan_id
        || journal.package_ids != ownership.package_ids
        || journal.phase != DeploymentPhase::Committed
    {
        return Err("deployment_journal_invalid".to_string());
    }
    for part in owned_parts(dota_root, &ownership)? {
        let current = fs::read(&part.target).map_err(|_| "deployment_conflict".to_string())?;
        if Some(sha256(&current)) != part.after {
            return Err("deployment_conflict".to_string());
        }
    }
    // Every backup the operation relies on still matches the file it saved.
    for (kind, before, relative) in [
        (
            Kind::Pak,
            journal.before_sha256.as_ref(),
            journal.backup_relative_path.as_deref(),
        ),
        (
            Kind::Map,
            journal
                .map
                .as_ref()
                .and_then(|step| step.before_sha256.as_ref()),
            journal
                .map
                .as_ref()
                .and_then(|step| step.backup_relative_path.as_deref()),
        ),
    ] {
        match (before, relative) {
            (Some(expected), Some(name)) if name == kind.backup_name() => {
                let backup = operations.join(&ownership.operation_id).join(name);
                reject_symlink(&backup)?;
                if sha256(&fs::read(&backup).map_err(|_| "backup_failed".to_string())?) != *expected
                {
                    return Err("backup_verification_failed".to_string());
                }
            }
            (None, None) => {}
            _ => return Err("deployment_journal_invalid".to_string()),
        }
    }
    Ok(Some(journal.receipt(true, false, true)))
}

/// Why BetterFy's record of an installation no longer matches the game folder.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StaleReason {
    /// The file BetterFy wrote is gone.
    Missing,
    /// Another program (a Dota update, a manual edit) replaced the file.
    Changed,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StaleRelease {
    pub operation_id: String,
    pub language: GameLanguage,
    pub reason: StaleReason,
}

/// Lets go of an installation whose file no longer matches the record.
///
/// Nothing in the game folder is touched: the file stays exactly as it is and
/// BetterFy's old backup is never copied over it, because the backup describes
/// a file that is no longer there and restoring it could undo a game update.
/// The ownership record is renamed, not deleted, so the evidence stays. An
/// installation that still matches is refused, so this cannot be used to
/// abandon a healthy install. With a map in the record, the install is stale
/// when any file it owns (the pak or the map) no longer matches.
pub(crate) fn release_stale_deployment(
    app_data_root: &Path,
    dota_root: &Path,
) -> Result<StaleRelease, String> {
    let identity = target_identity(dota_root)?;
    let (root, _, journals) = owned_roots(app_data_root)?;
    let _lock = transaction_lock(&root)?;
    let ownership_file = ownership_path(&root);
    if !ownership_file.exists() {
        return Err("deployment_not_found".to_string());
    }
    let state: OwnershipState = read_json(&ownership_file)?;
    state.validate(&identity)?;
    validate_operation_id(&state.operation_id)?;
    let mut reason: Option<StaleReason> = None;
    for part in owned_parts(dota_root, &state)? {
        reject_symlink(&part.target)?;
        let found = if !part.target.exists() {
            Some(StaleReason::Missing)
        } else if !part.target.is_file() {
            Some(StaleReason::Changed)
        } else {
            let current = fs::read(&part.target).map_err(|_| "deployment_conflict".to_string())?;
            (Some(sha256(&current)) != part.after).then_some(StaleReason::Changed)
        };
        // A file that was changed outranks one that is gone.
        reason = match (reason, found) {
            (Some(StaleReason::Changed), _) | (_, Some(StaleReason::Changed)) => {
                Some(StaleReason::Changed)
            }
            (previous, found) => found.or(previous),
        };
    }
    let Some(reason) = reason else {
        return Err("deployment_not_stale".to_string());
    };
    let released = root.join(format!("ownership.released-{}.json", now_ms()?));
    reject_symlink(&released)?;
    fs::rename(&ownership_file, &released).map_err(|_| "deployment_failed".to_string())?;
    let journal_path = journals.join(format!("{}.json", state.operation_id));
    if let Ok(mut journal) = read_json::<DeploymentJournal>(&journal_path) {
        journal.error_code = Some("released_stale".to_string());
        journal.updated_at_ms = now_ms()?;
        let _ = atomic_json(&journal_path, &journal);
    }
    Ok(StaleRelease {
        operation_id: state.operation_id,
        language: state.language,
        reason,
    })
}

// ---------------------------------------------------------------------------
// Rollback and recovery
// ---------------------------------------------------------------------------

/// How strictly a rollback checks the folder.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rollback {
    /// A committed operation: every file must be as the operation left it
    /// (or, for a file that had an owned version, already put back by an
    /// earlier attempt that stopped before the journal was updated).
    Committed,
    /// Recovery of an operation that never committed: a file the operation had
    /// not yet changed is simply left as it is.
    Interrupted,
}

pub(crate) fn rollback(
    app_data_root: &Path,
    dota_root: &Path,
    operation_id: &str,
) -> Result<DeploymentReceipt, String> {
    rollback_with(app_data_root, dota_root, operation_id, Rollback::Committed)
}

fn rollback_with(
    app_data_root: &Path,
    dota_root: &Path,
    operation_id: &str,
    mode: Rollback,
) -> Result<DeploymentReceipt, String> {
    validate_operation_id(operation_id)?;
    let identity = target_identity(dota_root)?;
    let (root, operations, journals) = owned_roots(app_data_root)?;
    let _lock = transaction_lock(&root)?;
    let journal_path = journals.join(format!("{operation_id}.json"));
    let mut journal: DeploymentJournal = read_json(&journal_path)?;
    if journal.schema_version != SCHEMA_VERSION
        || journal.operation_id != operation_id
        || journal.target_identity != identity
    {
        return Err("deployment_journal_invalid".to_string());
    }
    let parts = journal.parts(dota_root)?;
    if journal.phase == DeploymentPhase::RolledBack {
        return Ok(journal.receipt(false, true, true));
    }
    // Where each file stands now. Nothing is changed until every file has been
    // checked, so a conflict leaves the folder exactly as it was.
    let mut standing = Vec::with_capacity(parts.len());
    for part in &parts {
        let current = current_hash(part)?.ok_or_else(|| "rollback_conflict".to_string())?;
        let state = stand(&current, part);
        let acceptable = match (mode, state) {
            (_, Stand::After) => true,
            // An earlier rollback attempt already put an owned file back.
            (Rollback::Committed, Stand::Before) => part.before.is_some(),
            (Rollback::Interrupted, Stand::Before) => true,
            (_, Stand::Other) => false,
        };
        if !acceptable {
            return Err("rollback_conflict".to_string());
        }
        standing.push(state);
    }
    // The bytes to put back, read and checked before anything is touched.
    let mut backups: Vec<Option<Vec<u8>>> = Vec::with_capacity(parts.len());
    for part in &parts {
        match &part.before {
            Some(expected) => {
                let backup = operations.join(operation_id).join(part.kind.backup_name());
                reject_symlink(&backup)?;
                let bytes = fs::read(&backup).map_err(|_| "backup_failed".to_string())?;
                if sha256(&bytes) != *expected {
                    return Err("backup_verification_failed".to_string());
                }
                backups.push(Some(bytes));
            }
            None => backups.push(None),
        }
    }
    // The earlier build this operation replaced must still be on record.
    let previous_state = match &journal.before_operation_id {
        Some(previous_operation) => {
            validate_operation_id(previous_operation)?;
            let previous: DeploymentJournal =
                read_json(&journals.join(format!("{previous_operation}.json")))?;
            previous.validate_shape()?;
            if previous.operation_id != *previous_operation
                || previous.target_identity != identity
                || previous.language != journal.language
                || previous.pak_after() != journal.pak_before()
                || previous.map_after() != journal.map_before()
                || previous.phase != DeploymentPhase::Committed
            {
                return Err("deployment_journal_invalid".to_string());
            }
            Some(OwnershipState::from_journal(&previous))
        }
        None => None,
    };

    // Put the files back, the map before the pak.
    for index in (0..parts.len()).rev() {
        if standing[index] != Stand::After {
            continue;
        }
        let part = &parts[index];
        let parent = part
            .target
            .parent()
            .ok_or_else(|| "rollback_failed".to_string())?;
        match &backups[index] {
            Some(bytes) => {
                if !parent.exists() {
                    fs::create_dir_all(parent).map_err(|_| "rollback_failed".to_string())?;
                }
                reject_link(parent)?;
                let replacement = parent.join(format!(
                    ".betterfy-rollback-{operation_id}{}.tmp",
                    part.kind.tag()
                ));
                if replacement.exists() {
                    reject_symlink(&replacement)?;
                    fs::remove_file(&replacement).map_err(|_| "rollback_failed".to_string())?;
                }
                write_new_synced(&replacement, bytes)?;
                publish(&part.target, &replacement, part.target.exists())?;
            }
            None => {
                if part.target.exists() {
                    fs::remove_file(&part.target).map_err(|_| "rollback_failed".to_string())?;
                }
            }
        }
    }
    match previous_state {
        Some(state) => atomic_json(&ownership_path(&root), &state)?,
        None => {
            let ownership = ownership_path(&root);
            if ownership.exists() {
                fs::remove_file(ownership).map_err(|_| "rollback_failed".to_string())?;
            }
        }
    }
    // A temporary copy written for a file the interrupted operation never got
    // to publish is BetterFy's own and goes away with the operation.
    for part in &parts {
        if let Some(parent) = part.target.parent() {
            let temporary = parent.join(format!(".betterfy-{operation_id}{}.tmp", part.kind.tag()));
            if temporary.exists() {
                reject_symlink(&temporary)?;
                fs::remove_file(&temporary).map_err(|_| "rollback_failed".to_string())?;
            }
        }
    }
    // Folders this operation created go away only if they are empty.
    if journal
        .map
        .as_ref()
        .is_some_and(|step| step.maps_directory_created)
    {
        if let Some(maps_dir) = parts
            .iter()
            .find(|part| part.kind == Kind::Map)
            .and_then(|part| part.target.parent())
        {
            remove_dir_if_empty(maps_dir)?;
        }
    }
    if journal.locale_directory_created {
        if let Some(locale) = parts.first().and_then(|part| {
            let parent = part.target.parent()?;
            if part.kind == Kind::Map {
                parent.parent()
            } else {
                Some(parent)
            }
        }) {
            remove_locale_if_empty(locale)?;
        }
    }
    journal.phase = DeploymentPhase::RolledBack;
    journal.updated_at_ms = now_ms()?;
    journal.rollback_verified = true;
    for part in &parts {
        let restored = current_hash(part)?.is_some_and(|current| current == part.before);
        if !restored {
            journal.rollback_verified = false;
        }
    }
    if !journal.rollback_verified {
        return Err("rollback_verification_failed".to_string());
    }
    journal.error_code = None;
    atomic_json(&journal_path, &journal)?;
    Ok(journal.receipt(false, true, true))
}

pub(crate) fn recover_pending(
    app_data_root: &Path,
    dota_root: &Path,
) -> Result<RecoveryReceipt, String> {
    let identity = target_identity(dota_root)?;
    let (root, _, journals) = owned_roots(app_data_root)?;
    let mut rollback_ids = Vec::new();
    let mut marked_failed = 0usize;
    let mut inspected = 0usize;
    {
        let _lock = transaction_lock(&root)?;
        for entry in
            fs::read_dir(&journals).map_err(|_| "deployment_journal_invalid".to_string())?
        {
            let path = entry
                .map_err(|_| "deployment_journal_invalid".to_string())?
                .path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let mut journal: DeploymentJournal = read_json(&path)?;
            if journal.schema_version != SCHEMA_VERSION || journal.target_identity != identity {
                continue;
            }
            if !matches!(
                journal.phase,
                DeploymentPhase::BackedUp | DeploymentPhase::Prepared
            ) {
                continue;
            }
            let parts = journal.parts(dota_root)?;
            inspected += 1;
            // Each file is either as it was, or already as the operation
            // leaves it; anything else is not BetterFy's to touch.
            let mut published = false;
            for part in &parts {
                let current = current_hash(part)?
                    .ok_or_else(|| "deployment_recovery_conflict".to_string())?;
                match stand(&current, part) {
                    Stand::Before => {}
                    Stand::After => published = true,
                    Stand::Other => return Err("deployment_recovery_conflict".to_string()),
                }
            }
            if published {
                rollback_ids.push(journal.operation_id.clone());
                continue;
            }
            for part in &parts {
                let temporary = part
                    .target
                    .parent()
                    .ok_or_else(|| "deployment_recovery_conflict".to_string())?
                    .join(format!(
                        ".betterfy-{}{}.tmp",
                        journal.operation_id,
                        part.kind.tag()
                    ));
                if temporary.exists() {
                    reject_symlink(&temporary)?;
                    fs::remove_file(temporary)
                        .map_err(|_| "deployment_recovery_conflict".to_string())?;
                }
            }
            if journal
                .map
                .as_ref()
                .is_some_and(|step| step.maps_directory_created)
            {
                if let Some(maps_dir) = parts
                    .iter()
                    .find(|part| part.kind == Kind::Map)
                    .and_then(|part| part.target.parent())
                {
                    remove_dir_if_empty(maps_dir)
                        .map_err(|_| "deployment_recovery_conflict".to_string())?;
                }
            }
            if journal.locale_directory_created {
                let locale = locale_for(dota_root, journal.language)?;
                remove_locale_if_empty(&locale)
                    .map_err(|_| "deployment_recovery_conflict".to_string())?;
            }
            journal.phase = DeploymentPhase::Failed;
            journal.updated_at_ms = now_ms()?;
            journal.error_code = Some("interrupted_before_commit".to_string());
            atomic_json(&path, &journal)?;
            marked_failed += 1;
        }
    }
    let mut rolled_back = 0usize;
    for operation_id in rollback_ids {
        rollback_with(
            app_data_root,
            dota_root,
            &operation_id,
            Rollback::Interrupted,
        )?;
        rolled_back += 1;
    }
    Ok(RecoveryReceipt {
        inspected,
        rolled_back,
        marked_failed,
    })
}

pub(crate) fn diagnostic_counts(
    app_data_root: &Path,
) -> Result<DeploymentDiagnosticCounts, String> {
    let (_, _, journals) = owned_roots(app_data_root)?;
    let mut total = 0usize;
    let mut recoverable = 0usize;
    for entry in fs::read_dir(journals).map_err(|_| "deployment_journal_invalid".to_string())? {
        let path = entry
            .map_err(|_| "deployment_journal_invalid".to_string())?
            .path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let journal: DeploymentJournal = read_json(&path)?;
        if journal.schema_version != SCHEMA_VERSION {
            return Err("deployment_journal_invalid".to_string());
        }
        total += 1;
        if matches!(
            journal.phase,
            DeploymentPhase::BackedUp | DeploymentPhase::Prepared
        ) {
            recoverable += 1;
        }
    }
    Ok(DeploymentDiagnosticCounts { total, recoverable })
}

pub(crate) fn collect_evidence(
    app_data_root: &Path,
    dota_root: &Path,
    app_version: String,
) -> Result<DeploymentEvidenceReport, String> {
    let identity = target_identity(dota_root)?;
    let (root, operations, journals) = owned_roots(app_data_root)?;
    let _lock = transaction_lock(&root)?;
    let active_operation = if ownership_path(&root).exists() {
        let ownership: OwnershipState = read_json(&ownership_path(&root))?;
        ownership.validate(&identity)?;
        Some(ownership.operation_id)
    } else {
        None
    };
    let mut journals_for_target = Vec::new();
    for entry in fs::read_dir(journals).map_err(|_| "deployment_journal_invalid".to_string())? {
        let path = entry
            .map_err(|_| "deployment_journal_invalid".to_string())?
            .path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let journal: DeploymentJournal = read_json(&path)?;
        if journal.schema_version != SCHEMA_VERSION || journal.target_identity != identity {
            continue;
        }
        let backup_ok = |before: Option<&String>, relative: Option<&String>, kind: Kind| match (
            before, relative,
        ) {
            (None, None) => true,
            (Some(expected), Some(relative)) if relative == kind.backup_name() => {
                let backup = operations.join(&journal.operation_id).join(relative);
                if reject_symlink(&backup).is_err() {
                    return false;
                }
                fs::read(&backup)
                    .map(|bytes| sha256(&bytes) == *expected)
                    .unwrap_or(false)
            }
            _ => false,
        };
        let pak_backup = backup_ok(
            journal.before_sha256.as_ref(),
            journal.backup_relative_path.as_ref(),
            Kind::Pak,
        );
        let map_backup = journal.map.as_ref().is_none_or(|step| {
            backup_ok(
                step.before_sha256.as_ref(),
                step.backup_relative_path.as_ref(),
                Kind::Map,
            )
        });
        journals_for_target.push((journal, pak_backup && map_backup));
    }
    journals_for_target.sort_by_key(|(journal, _)| journal.created_at_ms);
    let entries = journals_for_target
        .into_iter()
        .rev()
        .take(100)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .enumerate()
        .map(|(index, (journal, backup_verified))| {
            let rolled_back_ok = journal.rollback_verified;
            DeploymentEvidenceEntry {
                sequence: index + 1,
                language: journal.language,
                phase: journal.phase.clone(),
                before_sha256: journal.before_sha256.clone(),
                installed_sha256: journal.primary_sha256(),
                backup_verified,
                rollback_verified: journal.rollback_verified,
                active: active_operation.as_deref() == Some(journal.operation_id.as_str()),
                bundle_plan_id: journal.bundle_plan_id.clone(),
                package_ids: journal.package_ids.clone(),
                created_at_ms: journal.created_at_ms,
                updated_at_ms: journal.updated_at_ms,
                error_code: journal.error_code.clone(),
                vpk_name: journal
                    .pak_after()
                    .map(|(name, _)| name.to_string())
                    .unwrap_or_else(|| journal.vpk_name.clone()),
                has_pak: !journal.no_pak,
                map: journal.map.as_ref().map(|step| MapEvidence {
                    file: map_package::MAP_RELATIVE_PATH,
                    before_sha256: step.before_sha256.clone(),
                    installed_sha256: step.installed_sha256.clone(),
                    backup_verified,
                    rollback_verified: rolled_back_ok,
                }),
            }
        })
        .collect();
    Ok(DeploymentEvidenceReport {
        schema_version: 1,
        app_version,
        platform: if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "other"
        },
        generated_at_ms: now_ms()?,
        entries,
    })
}

/// Whose a `pak*` file in a language folder looks like.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SlotFileOwner {
    /// The file BetterFy wrote: named in its ownership record, same hash.
    Betterfy,
    /// Named like Minify's output (`pak66_dir.vpk`, `pak66_000.vpk`, ...).
    /// Only the name is known; the contents are never read.
    MinifyLike,
    Unknown,
}

/// What became of the file BetterFy's record points at.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OwnedFileState {
    Intact,
    Missing,
    /// Present, but not the bytes BetterFy wrote.
    Replaced,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SlotFile {
    pub name: String,
    pub bytes: u64,
    pub modified_ms: Option<u64>,
    pub owner: SlotFileOwner,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OwnedSlotFile {
    pub name: String,
    pub state: OwnedFileState,
}

/// `maps/dota.vpk` in the language folder, by size and time only.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MapSlotFile {
    pub bytes: u64,
    pub modified_ms: Option<u64>,
    /// `betterfy` only when it is the file BetterFy's record names, with the
    /// same hash; everything else is `unknown`.
    pub owner: SlotFileOwner,
}

/// A read-only summary of one language folder. It carries file names, sizes
/// and modification times only: no paths and no contents.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LanguageSlotReport {
    pub language: GameLanguage,
    pub folder_exists: bool,
    /// Every `pak*.vpk` entry of the folder, by name (at most 64).
    pub files: Vec<SlotFile>,
    pub files_truncated: bool,
    /// BetterFy's recorded file in this folder, if it has an install here.
    pub owned: Option<OwnedSlotFile>,
    /// The name BetterFy would pick for a new install right now.
    pub next_name: Option<String>,
    pub slot_full: bool,
    /// `maps/dota.vpk`, when something is there.
    pub map: Option<MapSlotFile>,
    /// BetterFy's recorded map, if it has one.
    pub owned_map: Option<OwnedFileState>,
}

fn is_minify_like(name: &str) -> bool {
    slot_number(name) == Some(MINIFY_SLOT_NUMBER) && !name.starts_with('!')
}

fn is_pak_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let lower = lower.strip_prefix('!').unwrap_or(&lower);
    lower.starts_with("pak") && lower.ends_with(".vpk")
}

fn owned_state(target: &Path, expected: &str, limit: usize) -> Result<OwnedFileState, String> {
    reject_link(target)?;
    Ok(match fs::metadata(target) {
        Err(_) => OwnedFileState::Missing,
        Ok(metadata) if metadata.is_file() && metadata.len() <= limit as u64 => {
            match fs::read(target) {
                Ok(bytes) if sha256(&bytes) == expected => OwnedFileState::Intact,
                _ => OwnedFileState::Replaced,
            }
        }
        Ok(_) => OwnedFileState::Replaced,
    })
}

/// Describes the language folder BetterFy installs into, for explaining a
/// refusal. Reads names, sizes and times, and hashes only BetterFy's own
/// recorded files. With an existing install the folder of that install is
/// described, whatever `language` says: that is where the question is.
pub(crate) fn inspect_language_slot(
    app_data_root: &Path,
    dota_root: &Path,
    language: Option<GameLanguage>,
) -> Result<LanguageSlotReport, String> {
    let identity = target_identity(dota_root)?;
    let (root, _, _) = owned_roots(app_data_root)?;
    let _lock = transaction_lock(&root)?;
    let ownership = if ownership_path(&root).exists() {
        read_json::<OwnershipState>(&ownership_path(&root))
            .ok()
            .filter(|state| state.validate(&identity).is_ok())
    } else {
        None
    };
    let language = ownership
        .as_ref()
        .map(|state| state.language)
        .or(language)
        .ok_or_else(|| "deployment_not_found".to_string())?;
    let locale = locale_for(dota_root, language)?;

    let owned = match ownership.as_ref().and_then(|state| state.pak()) {
        Some((name, expected)) => {
            let target = locale.join(name);
            reject_symlink(&target)?;
            Some(OwnedSlotFile {
                name: name.to_string(),
                state: owned_state(&target, expected, MAX_PACKAGE_BYTES)?,
            })
        }
        None => None,
    };

    let mut files = Vec::new();
    let mut files_truncated = false;
    if locale.is_dir() {
        let mut names = Vec::new();
        for entry in fs::read_dir(&locale).map_err(|_| "deployment_failed".to_string())? {
            let entry = entry.map_err(|_| "deployment_failed".to_string())?;
            if let Some(name) = entry.file_name().to_str() {
                if is_pak_name(name) && name.len() <= 80 {
                    names.push(name.to_string());
                }
            }
        }
        names.sort();
        files_truncated = names.len() > MAX_SLOT_FILES;
        for name in names.into_iter().take(MAX_SLOT_FILES) {
            let metadata = fs::symlink_metadata(locale.join(&name))
                .map_err(|_| "deployment_failed".to_string())?;
            let betterfy = owned
                .as_ref()
                .is_some_and(|file| file.state == OwnedFileState::Intact && file.name == name);
            let owner = if betterfy {
                SlotFileOwner::Betterfy
            } else if is_minify_like(&name) {
                SlotFileOwner::MinifyLike
            } else {
                SlotFileOwner::Unknown
            };
            files.push(SlotFile {
                bytes: metadata.len(),
                modified_ms: metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_millis() as u64),
                name,
                owner,
            });
        }
    }

    let (_, map_target) = map_paths(dota_root, language)?;
    let owned_map = match ownership.as_ref().and_then(|state| state.map_sha256()) {
        Some(expected) => Some(owned_state(
            &map_target,
            expected,
            map_package::MAX_MAP_BYTES,
        )?),
        None => None,
    };
    let map = match fs::symlink_metadata(&map_target) {
        Ok(metadata) => Some(MapSlotFile {
            bytes: metadata.len(),
            modified_ms: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis() as u64),
            owner: if owned_map == Some(OwnedFileState::Intact) {
                SlotFileOwner::Betterfy
            } else {
                SlotFileOwner::Unknown
            },
        }),
        Err(_) => None,
    };
    let (next_name, slot_full) = match allocate_vpk_name(&locale) {
        Ok(name) => (Some(name), false),
        Err(code) if code == "slot_full" => (None, true),
        Err(code) => return Err(code),
    };
    Ok(LanguageSlotReport {
        language,
        folder_exists: locale.is_dir(),
        files,
        files_truncated,
        owned,
        next_name,
        slot_full,
        map,
        owned_map,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWNED_VPK_NAME: &str = LEGACY_VPK_NAME;

    #[test]
    fn stress_controls_require_a_developer_account() {
        let signed_out = deployment_stress_capabilities(false);
        assert!(!signed_out.enabled);
        assert!(signed_out.failure_points.is_empty());
        let developer = deployment_stress_capabilities(true);
        assert_eq!(developer.enabled, cfg!(feature = "internal-stress-test"));
        assert_eq!(developer.failure_points.is_empty(), !developer.enabled);
    }

    fn root(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "betterfy-deploy-{label}-{}-{}",
            std::process::id(),
            OPERATION_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("root");
        path
    }

    fn game(root: &Path) -> PathBuf {
        let dota = root.join("dota 2 beta");
        fs::create_dir_all(dota.join("game/dota")).expect("game");
        fs::write(dota.join("game/dota/pak01_dir.vpk"), b"official-marker").expect("marker");
        let executable = if cfg!(target_os = "windows") {
            dota.join("game/bin/win64/dota2.exe")
        } else if cfg!(target_os = "macos") {
            dota.join("game/bin/osx64/dota2")
        } else {
            dota.join("game/bin/linuxsteamrt64/dota2")
        };
        fs::create_dir_all(executable.parent().expect("executable parent"))
            .expect("executable directory");
        fs::write(executable, b"executable-marker").expect("executable marker");
        dota
    }

    #[test]
    fn bundle_identity_survives_install_restart_and_rollback() {
        let base = root("bundle-identity");
        let app = base.join("app");
        let dota = game(&base);
        let first = package(b"first bundle");
        let first_identity = DeploymentBundleIdentity {
            plan_id: Some("sha256:first-plan".to_string()),
            package_ids: vec!["minify.tree-mod".to_string()],
        };
        let first_receipt = deploy_verified_bundle_for_language(
            &app,
            &dota,
            &first,
            &sha256(&first),
            GameLanguage::Dutch,
            first_identity.clone(),
        )
        .expect("first bundle");
        let second = package(b"second bundle");
        let second_identity = DeploymentBundleIdentity {
            plan_id: Some("sha256:second-plan".to_string()),
            package_ids: vec![
                "minify.show-networth".to_string(),
                "minify.tree-mod".to_string(),
            ],
        };
        let second_receipt = deploy_verified_bundle_for_language(
            &app,
            &dota,
            &second,
            &sha256(&second),
            GameLanguage::Dutch,
            second_identity.clone(),
        )
        .expect("second bundle");
        let current = current_owned_deployment(&app, &dota)
            .expect("current state")
            .expect("owned state");
        assert_eq!(current.bundle_plan_id, second_identity.plan_id);
        assert_eq!(current.package_ids, second_identity.package_ids);
        rollback(&app, &dota, &second_receipt.operation_id).expect("restore first bundle");
        let restored = current_owned_deployment(&app, &dota)
            .expect("restored state")
            .expect("restored ownership");
        assert_eq!(restored.operation_id, first_receipt.operation_id);
        assert_eq!(restored.bundle_plan_id, first_identity.plan_id);
        assert_eq!(restored.package_ids, first_identity.package_ids);
        let _ = fs::remove_dir_all(base);
    }

    fn package(label: &'static [u8]) -> Vec<u8> {
        vpk::build(vec![vpk::VpkInput {
            path: "models/props_tree/tree_oak_01.vmdl_c",
            bytes: label,
        }])
        .expect("package")
    }

    #[test]
    fn deploys_verifies_and_removes_an_initial_install_on_rollback() {
        let base = root("commit");
        let app = base.join("app");
        let dota = game(&base);
        let bytes = package(b"tree-one");
        let receipt = deploy_verified_vpk(&app, &dota, &bytes, &sha256(&bytes)).expect("deploy");
        let target = dota.join("game/dota_dutch/pak65_dir.vpk");
        assert_eq!(fs::read(&target).expect("installed"), bytes);
        assert!(receipt.committed);
        let current = current_verified_deployment(&app, &dota, &sha256(&bytes))
            .expect("read current deployment")
            .expect("owned deployment");
        assert_eq!(current.operation_id, receipt.operation_id);
        assert!(current_verified_deployment(&app, &dota, &sha256(b"other"))
            .expect("different package is not current")
            .is_none());
        let rolled_back = rollback(&app, &dota, &receipt.operation_id).expect("rollback");
        assert!(rolled_back.rolled_back);
        assert!(!target.exists());
        assert!(current_verified_deployment(&app, &dota, &sha256(&bytes))
            .expect("no current deployment after rollback")
            .is_none());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn selected_language_is_bound_to_target_receipt_and_rollback() {
        let base = root("russian-language");
        let app = base.join("app");
        let dota = game(&base);
        let bytes = package(b"russian-tree");
        let locale = dota.join("game/dota_russian");
        assert!(!locale.exists());
        preview_language_destination(&app, &dota, GameLanguage::Russian)
            .expect("destination ready before process shutdown");
        assert!(!locale.exists(), "preview must remain read-only");
        let receipt = deploy_verified_vpk_for_language(
            &app,
            &dota,
            &bytes,
            &sha256(&bytes),
            GameLanguage::Russian,
        )
        .expect("deploy Russian");
        assert_eq!(receipt.language, GameLanguage::Russian);
        assert_eq!(
            fs::read(locale.join(&receipt.vpk_name)).expect("installed"),
            bytes
        );
        let current = current_verified_deployment(&app, &dota, &sha256(&bytes))
            .expect("current")
            .expect("owned");
        assert_eq!(current.language, GameLanguage::Russian);
        assert_eq!(
            deploy_verified_vpk_for_language(
                &app,
                &dota,
                &bytes,
                &sha256(&bytes),
                GameLanguage::Dutch,
            )
            .err()
            .as_deref(),
            Some("deployment_language_change_requires_restore")
        );
        assert_eq!(
            preview_language_destination(&app, &dota, GameLanguage::Dutch)
                .err()
                .as_deref(),
            Some("deployment_language_change_requires_restore")
        );
        rollback(&app, &dota, &receipt.operation_id).expect("rollback Russian");
        assert!(
            !locale.exists(),
            "restore removes an empty BetterFy-created folder"
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn betterfy_slot_is_created_owned_and_removed_on_restore() {
        let base = root("betterfy-slot");
        let app = base.join("app");
        let dota = game(&base);
        let locale = dota.join("game/dota_betterfy");
        // Minify's former English slot is someone else's folder.
        let minify = dota.join("game/dota_minify").join(OWNED_VPK_NAME);
        fs::create_dir_all(minify.parent().expect("minify parent")).expect("minify folder");
        fs::write(&minify, b"minify data").expect("minify file");
        preview_language_destination(&app, &dota, GameLanguage::Betterfy).expect("preview");
        assert!(!locale.exists(), "preview must remain read-only");
        let bytes = package(b"english tree");
        let receipt = deploy_verified_vpk_for_language(
            &app,
            &dota,
            &bytes,
            &sha256(&bytes),
            GameLanguage::Betterfy,
        )
        .expect("deploy into the BetterFy slot");
        assert_eq!(receipt.language, GameLanguage::Betterfy);
        assert_eq!(
            fs::read(locale.join(&receipt.vpk_name)).expect("installed"),
            bytes
        );
        assert_eq!(
            deploy_verified_vpk_for_language(
                &app,
                &dota,
                &bytes,
                &sha256(&bytes),
                GameLanguage::Russian,
            )
            .err()
            .as_deref(),
            Some("deployment_language_change_requires_restore")
        );
        rollback(&app, &dota, &receipt.operation_id).expect("rollback");
        assert!(!locale.exists(), "restore removes the folder BetterFy made");
        assert_eq!(fs::read(&minify).expect("minify unchanged"), b"minify data");

        // A VPK BetterFy did not write is not replaced: the next install
        // takes another name next to it.
        fs::create_dir_all(&locale).expect("foreign slot");
        fs::write(locale.join(OWNED_VPK_NAME), b"foreign").expect("foreign file");
        preview_language_destination(&app, &dota, GameLanguage::Betterfy)
            .expect("a free name exists next to the foreign file");
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn preview_does_not_create_a_dutch_language_folder() {
        let base = root("preview-no-game-write");
        let app = base.join("app");
        let dota = game(&base);
        let locale = dota.join("game/dota_dutch");
        assert!(!locale.exists());
        preview_language_destination(&app, &dota, GameLanguage::Dutch).expect("preview is safe");
        assert!(!locale.exists(), "preview must not write inside the game");
        let bytes = package(b"tree");
        let receipt = deploy_verified_vpk_for_language(
            &app,
            &dota,
            &bytes,
            &sha256(&bytes),
            GameLanguage::Dutch,
        )
        .expect("confirmed deployment creates its destination");
        assert!(locale.join(&receipt.vpk_name).is_file());
        rollback(&app, &dota, &receipt.operation_id).expect("rollback");
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn owned_install_remains_discoverable_without_source_cache() {
        let base = root("owned-without-cache");
        let app = base.join("app");
        let dota = game(&base);
        let bytes = package(b"tree");
        let receipt = deploy_verified_vpk(&app, &dota, &bytes, &sha256(&bytes))
            .expect("deploy without a content cache");
        let current = current_owned_deployment(&app, &dota)
            .expect("read ownership")
            .expect("installed operation");
        assert_eq!(current.operation_id, receipt.operation_id);
        assert_eq!(current.installed_sha256, sha256(&bytes));
        fs::write(dota.join("game/dota_dutch/pak65_dir.vpk"), b"external edit")
            .expect("tamper target");
        assert_eq!(
            current_owned_deployment(&app, &dota).err().as_deref(),
            Some("deployment_conflict")
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn missing_backup_is_not_reported_as_restorable() {
        let base = root("lost-backup");
        let app = base.join("app");
        let dota = game(&base);
        let first = package(b"first");
        deploy_verified_vpk(&app, &dota, &first, &sha256(&first)).expect("first install");
        let second = package(b"second");
        let receipt = deploy_verified_vpk(&app, &dota, &second, &sha256(&second))
            .expect("update with backup");
        let backup = app
            .join("engine-v1/game-deployment/operations")
            .join(&receipt.operation_id)
            .join("before.vpk");
        fs::remove_file(backup).expect("simulate lost backup");
        assert_eq!(
            current_owned_deployment(&app, &dota).err().as_deref(),
            Some("backup_failed")
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn every_existing_language_folder_uses_only_its_own_slot() {
        for language in GameLanguage::ALL {
            let base = root(language.suffix());
            let app = base.join("app");
            let dota = game(&base);
            let selected = dota
                .join("game")
                .join(format!("dota_{}", language.suffix()));
            fs::create_dir_all(&selected).expect("language folder");
            if language != GameLanguage::Dutch {
                fs::write(selected.join("gameinfo.gi"), b"fixture").expect("language marker");
            }
            let foreign = dota.join("game/dota_other/pak66_dir.vpk");
            fs::create_dir_all(foreign.parent().expect("foreign parent")).expect("foreign folder");
            fs::write(&foreign, b"foreign data").expect("foreign file");
            let bytes = package(language.suffix().as_bytes());
            let receipt =
                deploy_verified_vpk_for_language(&app, &dota, &bytes, &sha256(&bytes), language)
                    .expect("selected language deployment");
            assert_eq!(receipt.language, language);
            assert_eq!(
                fs::read(selected.join(&receipt.vpk_name)).expect("installed"),
                bytes
            );
            assert_eq!(
                fs::read(&foreign).expect("foreign unchanged"),
                b"foreign data"
            );
            rollback(&app, &dota, &receipt.operation_id).expect("restore selected slot");
            assert!(!selected.join(&receipt.vpk_name).exists());
            assert_eq!(
                fs::read(&foreign).expect("foreign unchanged"),
                b"foreign data"
            );
            let _ = fs::remove_dir_all(base);
        }
    }

    #[test]
    fn interrupted_selected_language_deployment_recovers_without_touching_other_folders() {
        let base = root("russian-recovery");
        let app = base.join("app");
        let dota = game(&base);
        let russian = dota.join("game/dota_russian");
        let dutch = dota.join("game/dota_dutch");
        fs::create_dir_all(&russian).expect("Russian folder");
        fs::create_dir_all(&dutch).expect("Dutch folder");
        fs::write(russian.join("gameinfo.gi"), b"fixture").expect("language marker");
        fs::write(dutch.join(OWNED_VPK_NAME), b"other mod").expect("foreign file");
        let bytes = package(b"interrupted Russian tree");
        assert_eq!(
            deploy_with_failure(
                &app,
                &dota,
                &bytes,
                &sha256(&bytes),
                GameLanguage::Russian,
                DeploymentBundleIdentity::default(),
                FailurePoint::AfterReplace,
            )
            .err()
            .as_deref(),
            Some("injected_failure")
        );
        assert_eq!(
            fs::read(russian.join("pak65_dir.vpk")).expect("published before failure"),
            bytes
        );
        let recovered = recover_pending(&app, &dota).expect("recover Russian operation");
        assert_eq!(recovered.rolled_back, 1);
        assert!(!russian.join("pak65_dir.vpk").exists());
        assert_eq!(
            fs::read(dutch.join(OWNED_VPK_NAME)).expect("other language unchanged"),
            b"other mod"
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn legacy_dutch_journal_without_language_remains_restorable() {
        let base = root("legacy-dutch");
        let app = base.join("app");
        let dota = game(&base);
        let bytes = package(b"old-tree");
        let receipt =
            deploy_verified_vpk(&app, &dota, &bytes, &sha256(&bytes)).expect("Dutch deployment");
        let root = app.join("engine-v1/game-deployment");
        for path in [
            root.join("ownership.json"),
            root.join("journals")
                .join(format!("{}.json", receipt.operation_id)),
        ] {
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).expect("record")).expect("valid JSON");
            value
                .as_object_mut()
                .expect("record object")
                .remove("language");
            // Records written before the file name was a choice have no name
            // and describe pak66_dir.vpk.
            value
                .as_object_mut()
                .expect("record object")
                .remove("vpkName");
            fs::write(&path, serde_json::to_vec(&value).expect("JSON bytes"))
                .expect("legacy record");
        }
        let locale = dota.join("game/dota_dutch");
        fs::rename(locale.join(&receipt.vpk_name), locale.join(LEGACY_VPK_NAME))
            .expect("old installs live in pak66_dir.vpk");
        let current = current_verified_deployment(&app, &dota, &sha256(&bytes))
            .expect("read old deployment")
            .expect("owned Dutch deployment");
        assert_eq!(current.language, GameLanguage::Dutch);
        assert_eq!(current.vpk_name, LEGACY_VPK_NAME);
        // An update of the old install stays on its recorded name.
        let next = package(b"newer-tree");
        let updated =
            deploy_verified_vpk(&app, &dota, &next, &sha256(&next)).expect("update old install");
        assert_eq!(updated.vpk_name, LEGACY_VPK_NAME);
        assert_eq!(
            fs::read(locale.join(LEGACY_VPK_NAME)).expect("updated"),
            next
        );
        assert!(!locale.join("pak65_dir.vpk").exists());
        rollback(&app, &dota, &updated.operation_id).expect("restore previous bytes");
        assert_eq!(
            fs::read(locale.join(LEGACY_VPK_NAME)).expect("restored"),
            bytes
        );
        rollback(&app, &dota, &receipt.operation_id).expect("restore old deployment");
        assert!(!locale.join(LEGACY_VPK_NAME).exists());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn updates_only_an_owned_slot_and_restores_exact_bytes() {
        let base = root("owned-update");
        let app = base.join("app");
        let dota = game(&base);
        let first = package(b"tree-one");
        let first_receipt =
            deploy_verified_vpk(&app, &dota, &first, &sha256(&first)).expect("first");
        let second = package(b"tree-two");
        let second_receipt =
            deploy_verified_vpk(&app, &dota, &second, &sha256(&second)).expect("second");
        rollback(&app, &dota, &second_receipt.operation_id).expect("rollback second");
        assert_eq!(
            fs::read(dota.join("game/dota_dutch/pak65_dir.vpk")).expect("restored"),
            first
        );
        rollback(&app, &dota, &first_receipt.operation_id).expect("rollback first");
        assert!(!dota.join("game/dota_dutch/pak65_dir.vpk").exists());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn refuses_foreign_targets_and_external_edits() {
        let base = root("foreign");
        let app = base.join("app");
        let dota = game(&base);
        let locale = dota.join("game/dota_dutch");
        fs::create_dir_all(&locale).expect("locale");
        fs::write(locale.join("pak65_dir.vpk"), b"foreign").expect("foreign");
        let bytes = package(b"tree");
        preview_language_destination(&app, &dota, GameLanguage::Dutch)
            .expect("a free name exists next to the foreign file");
        let receipt = deploy_verified_vpk(&app, &dota, &bytes, &sha256(&bytes)).expect("deploy");
        assert_eq!(receipt.vpk_name, "pak64_dir.vpk");
        assert_eq!(
            fs::read(locale.join("pak65_dir.vpk")).expect("foreign kept"),
            b"foreign"
        );
        // An edit of BetterFy's own file is still a hard conflict.
        fs::write(locale.join(&receipt.vpk_name), b"external-edit").expect("edit");
        assert_eq!(
            rollback(&app, &dota, &receipt.operation_id)
                .err()
                .as_deref(),
            Some("rollback_conflict")
        );
        assert_eq!(
            deploy_verified_vpk(&app, &dota, &bytes, &sha256(&bytes))
                .err()
                .as_deref(),
            Some("deployment_target_foreign")
        );
        assert_eq!(
            fs::read(locale.join("pak65_dir.vpk")).expect("foreign kept"),
            b"foreign"
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn releases_only_an_installation_whose_file_changed_or_vanished() {
        let base = root("stale");
        let app = base.join("app");
        let dota = game(&base);
        let bytes = package(b"tree");
        let receipt = deploy_verified_vpk(&app, &dota, &bytes, &sha256(&bytes)).expect("deploy");
        let target = dota.join("game/dota_dutch/pak65_dir.vpk");
        // A healthy install is never released.
        assert_eq!(
            release_stale_deployment(&app, &dota).err().as_deref(),
            Some("deployment_not_stale")
        );
        // A game update replaces the file: both install and restore are
        // refused, and releasing leaves the new file alone.
        fs::write(&target, b"game-update").expect("replace");
        assert_eq!(
            rollback(&app, &dota, &receipt.operation_id)
                .err()
                .as_deref(),
            Some("rollback_conflict")
        );
        let released = release_stale_deployment(&app, &dota).expect("release");
        assert_eq!(released.reason, StaleReason::Changed);
        assert_eq!(released.operation_id, receipt.operation_id);
        assert_eq!(fs::read(&target).expect("kept"), b"game-update");
        // Nothing is owned any more, and the game's file is not overwritten.
        assert_eq!(
            release_stale_deployment(&app, &dota).err().as_deref(),
            Some("deployment_not_found")
        );
        // A fresh install does not touch the file the game left there: it
        // takes the next free name beside it.
        let fresh =
            deploy_verified_vpk(&app, &dota, &bytes, &sha256(&bytes)).expect("fresh install");
        assert_eq!(fresh.vpk_name, "pak64_dir.vpk");
        assert_eq!(fs::read(&target).expect("kept"), b"game-update");
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn a_vanished_file_is_released_and_a_fresh_install_works() {
        let base = root("stale-missing");
        let app = base.join("app");
        let dota = game(&base);
        let bytes = package(b"tree");
        deploy_verified_vpk(&app, &dota, &bytes, &sha256(&bytes)).expect("deploy");
        fs::remove_file(dota.join("game/dota_dutch/pak65_dir.vpk")).expect("remove");
        let released = release_stale_deployment(&app, &dota).expect("release");
        assert_eq!(released.reason, StaleReason::Missing);
        deploy_verified_vpk(&app, &dota, &bytes, &sha256(&bytes)).expect("fresh install");
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn failures_before_and_after_publish_preserve_recoverable_evidence() {
        let base = root("failure");
        let app = base.join("app");
        let dota = game(&base);
        let first = package(b"first");
        assert_eq!(
            deploy_with_failure(
                &app,
                &dota,
                &first,
                &sha256(&first),
                GameLanguage::Dutch,
                DeploymentBundleIdentity::default(),
                FailurePoint::AfterPrepared,
            )
            .err()
            .as_deref(),
            Some("injected_failure")
        );
        assert!(!dota.join("game/dota_dutch/pak65_dir.vpk").exists());
        let first_recovery = recover_pending(&app, &dota).expect("recover prepared");
        assert_eq!(first_recovery.marked_failed, 1);
        let second = package(b"second");
        assert_eq!(
            deploy_with_failure(
                &app,
                &dota,
                &second,
                &sha256(&second),
                GameLanguage::Dutch,
                DeploymentBundleIdentity::default(),
                FailurePoint::AfterReplace,
            )
            .err()
            .as_deref(),
            Some("injected_failure")
        );
        assert_eq!(
            fs::read(dota.join("game/dota_dutch/pak65_dir.vpk")).expect("published"),
            second
        );
        let pending = diagnostic_counts(&app).expect("diagnostics");
        assert_eq!(pending.total, 2);
        assert_eq!(pending.recoverable, 1);
        let second_recovery = recover_pending(&app, &dota).expect("recover published");
        assert_eq!(second_recovery.rolled_back, 1);
        assert!(!dota.join("game/dota_dutch/pak65_dir.vpk").exists());
        let report =
            collect_evidence(&app, &dota, "0.1-test".to_string()).expect("privacy-safe evidence");
        assert_eq!(report.entries.len(), 2);
        assert_eq!(report.entries[0].phase, DeploymentPhase::Failed);
        assert_eq!(
            report.entries[0].error_code.as_deref(),
            Some("interrupted_before_commit")
        );
        assert_eq!(report.entries[1].phase, DeploymentPhase::RolledBack);
        assert!(report.entries[1].rollback_verified);
        assert!(!report.entries[1].active);
        let serialized = serde_json::to_string(&report).expect("serialize evidence");
        assert!(!serialized.contains("targetIdentity"));
        assert!(!serialized.contains("gamePath"));
        assert!(!serialized.contains(&dota.to_string_lossy().to_string()));
        assert_eq!(report.entries[0].vpk_name, "pak65_dir.vpk");
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn slot_numbers_follow_the_names_dota_loads() {
        for (name, expected) in [
            ("pak66_dir.vpk", Some(66)),
            ("PAK66_DIR.VPK", Some(66)),
            ("!pak65_dir.vpk", Some(65)),
            ("pak66_000.vpk", Some(66)),
            ("pak66_123.vpk", Some(66)),
            ("pak01_dir.vpk", Some(1)),
            ("pak100_dir.vpk", None),
            ("pak6_dir.vpk", None),
            ("pak66_.vpk", None),
            ("pak66_abc.vpk", None),
            ("pak66_dir.vpk.bak", None),
            ("gameinfo.gi", None),
            (".betterfy-deploy-op-1.tmp", None),
        ] {
            assert_eq!(slot_number(name), expected, "{name}");
        }
        assert!(validate_vpk_name("pak65_dir.vpk").is_ok());
        for bad in [
            "pak65_000.vpk",
            "!pak65_dir.vpk",
            "../pak65_dir.vpk",
            "pak6x_dir.vpk",
            "pak065_dir.vpk",
            "",
        ] {
            assert!(validate_vpk_name(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn installs_beside_minify_and_leaves_its_files_untouched() {
        let base = root("minify-coexist");
        let app = base.join("app");
        let dota = game(&base);
        let locale = dota.join("game/dota_russian");
        fs::create_dir_all(&locale).expect("locale");
        fs::write(locale.join("gameinfo.gi"), b"fixture").expect("language marker");
        fs::write(locale.join("pak66_dir.vpk"), b"minify directory").expect("minify dir");
        fs::write(locale.join("pak66_000.vpk"), b"minify chunk").expect("minify chunk");
        let bytes = package(b"tree beside minify");
        preview_language_destination(&app, &dota, GameLanguage::Russian).expect("preview");
        let receipt = deploy_verified_vpk_for_language(
            &app,
            &dota,
            &bytes,
            &sha256(&bytes),
            GameLanguage::Russian,
        )
        .expect("install beside Minify");
        assert_eq!(receipt.vpk_name, "pak65_dir.vpk");
        assert_eq!(fs::read(locale.join("pak65_dir.vpk")).expect("ours"), bytes);
        assert_eq!(
            fs::read(locale.join("pak66_dir.vpk")).expect("minify dir"),
            b"minify directory"
        );
        assert_eq!(
            fs::read(locale.join("pak66_000.vpk")).expect("minify chunk"),
            b"minify chunk"
        );
        // An update of the owned install keeps its name.
        let next = package(b"newer tree");
        let updated = deploy_verified_vpk_for_language(
            &app,
            &dota,
            &next,
            &sha256(&next),
            GameLanguage::Russian,
        )
        .expect("update");
        assert_eq!(updated.vpk_name, "pak65_dir.vpk");
        rollback(&app, &dota, &updated.operation_id).expect("restore previous");
        assert_eq!(
            fs::read(locale.join("pak65_dir.vpk")).expect("previous"),
            bytes
        );
        rollback(&app, &dota, &receipt.operation_id).expect("restore");
        assert!(!locale.join("pak65_dir.vpk").exists());
        assert_eq!(
            fs::read(locale.join("pak66_dir.vpk")).expect("minify dir"),
            b"minify directory"
        );
        assert_eq!(
            fs::read(locale.join("pak66_000.vpk")).expect("minify chunk"),
            b"minify chunk"
        );
        assert!(locale.join("gameinfo.gi").is_file());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn names_taken_by_any_pak_file_are_skipped() {
        let base = root("allocation");
        let app = base.join("app");
        let dota = game(&base);
        let locale = dota.join("game/dota_german");
        fs::create_dir_all(&locale).expect("locale");
        // A priority-prefixed directory file, a chunk-only number and a
        // directory all take their numbers.
        fs::write(locale.join("!pak65_dir.vpk"), b"a").expect("prefixed");
        fs::write(locale.join("pak64_007.vpk"), b"b").expect("chunk");
        fs::create_dir(locale.join("pak63_dir.vpk")).expect("directory");
        fs::write(locale.join("pak62_dir.vpk.bak"), b"c").expect("backup copy");
        let bytes = package(b"allocation");
        let receipt = deploy_verified_vpk_for_language(
            &app,
            &dota,
            &bytes,
            &sha256(&bytes),
            GameLanguage::German,
        )
        .expect("install");
        assert_eq!(receipt.vpk_name, "pak62_dir.vpk");
        for (name, bytes) in [("!pak65_dir.vpk", b"a"), ("pak64_007.vpk", b"b")] {
            assert_eq!(fs::read(locale.join(name)).expect("kept"), bytes);
        }
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn a_full_slot_is_reported_and_nothing_is_written() {
        let base = root("slot-full");
        let app = base.join("app");
        let dota = game(&base);
        let locale = dota.join("game/dota_polish");
        fs::create_dir_all(&locale).expect("locale");
        for number in PREFERRED_SLOT_NUMBERS {
            fs::write(locale.join(format!("pak{number:02}_dir.vpk")), b"other").expect("taken");
        }
        let bytes = package(b"nowhere to go");
        assert_eq!(
            preview_language_destination(&app, &dota, GameLanguage::Polish)
                .err()
                .as_deref(),
            Some("slot_full")
        );
        assert_eq!(
            deploy_verified_vpk_for_language(
                &app,
                &dota,
                &bytes,
                &sha256(&bytes),
                GameLanguage::Polish,
            )
            .err()
            .as_deref(),
            Some("slot_full")
        );
        let report = inspect_language_slot(&app, &dota, Some(GameLanguage::Polish))
            .expect("inspect a full slot");
        assert!(report.slot_full);
        assert!(report.next_name.is_none());
        assert_eq!(
            fs::read_dir(&locale).expect("listing").count(),
            PREFERRED_SLOT_NUMBERS.len()
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn minify_replacing_the_old_pak66_install_is_explained_and_recoverable() {
        // The reported case: BetterFy installed to pak66 (every build before
        // the name was a choice), then Minify wrote its own pak66.
        let base = root("minify-replaced");
        let app = base.join("app");
        let dota = game(&base);
        let locale = dota.join("game/dota_russian");
        let bytes = package(b"old install");
        let receipt = deploy_verified_vpk_for_language(
            &app,
            &dota,
            &bytes,
            &sha256(&bytes),
            GameLanguage::Russian,
        )
        .expect("install");
        let root_dir = app.join("engine-v1/game-deployment");
        for path in [
            root_dir.join("ownership.json"),
            root_dir
                .join("journals")
                .join(format!("{}.json", receipt.operation_id)),
        ] {
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).expect("record")).expect("valid JSON");
            value["vpkName"] = serde_json::Value::String(LEGACY_VPK_NAME.to_string());
            fs::write(&path, serde_json::to_vec(&value).expect("JSON bytes")).expect("record");
        }
        fs::rename(locale.join("pak65_dir.vpk"), locale.join(LEGACY_VPK_NAME)).expect("move");
        fs::write(locale.join(LEGACY_VPK_NAME), b"minify output").expect("minify overwrites");
        fs::write(locale.join("pak66_000.vpk"), b"minify chunk").expect("minify chunk");

        let report = inspect_language_slot(&app, &dota, None).expect("inspect");
        assert_eq!(report.language, GameLanguage::Russian);
        assert_eq!(
            report.owned,
            Some(OwnedSlotFile {
                name: LEGACY_VPK_NAME.to_string(),
                state: OwnedFileState::Replaced,
            })
        );
        assert_eq!(report.next_name.as_deref(), Some("pak65_dir.vpk"));
        assert_eq!(report.files.len(), 2);
        assert!(report
            .files
            .iter()
            .all(|file| file.owner == SlotFileOwner::MinifyLike));
        assert_eq!(report.files[0].name, "pak66_000.vpk");
        assert_eq!(report.files[1].bytes, b"minify output".len() as u64);
        let serialized = serde_json::to_string(&report).expect("serialize");
        assert!(!serialized.contains(&dota.to_string_lossy().to_string()));
        assert!(!serialized.contains("minify output"));

        assert_eq!(
            rollback(&app, &dota, &receipt.operation_id)
                .err()
                .as_deref(),
            Some("rollback_conflict")
        );
        assert_eq!(
            deploy_verified_vpk_for_language(
                &app,
                &dota,
                &bytes,
                &sha256(&bytes),
                GameLanguage::Russian,
            )
            .err()
            .as_deref(),
            Some("deployment_target_foreign")
        );
        release_stale_deployment(&app, &dota).expect("let go");
        let fresh = deploy_verified_vpk_for_language(
            &app,
            &dota,
            &bytes,
            &sha256(&bytes),
            GameLanguage::Russian,
        )
        .expect("install beside Minify");
        assert_eq!(fresh.vpk_name, "pak65_dir.vpk");
        assert_eq!(
            fs::read(locale.join(LEGACY_VPK_NAME)).expect("minify kept"),
            b"minify output"
        );
        assert_eq!(
            fs::read(locale.join("pak66_000.vpk")).expect("chunk kept"),
            b"minify chunk"
        );
        let healthy = inspect_language_slot(&app, &dota, None).expect("inspect again");
        assert_eq!(
            healthy.owned.as_ref().map(|file| file.state),
            Some(OwnedFileState::Intact)
        );
        assert!(healthy
            .files
            .iter()
            .any(|file| file.name == "pak65_dir.vpk" && file.owner == SlotFileOwner::Betterfy));
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn inspecting_without_an_install_needs_a_language_and_reads_nothing_else() {
        let base = root("inspect-empty");
        let app = base.join("app");
        let dota = game(&base);
        assert_eq!(
            inspect_language_slot(&app, &dota, None).err().as_deref(),
            Some("deployment_not_found")
        );
        let locale = dota.join("game/dota_dutch");
        fs::create_dir_all(&locale).expect("locale");
        fs::write(locale.join("pak22_dir.vpk"), b"another mod").expect("other mod");
        fs::write(locale.join("maps.vpk"), b"not a pak").expect("not listed");
        let report =
            inspect_language_slot(&app, &dota, Some(GameLanguage::Dutch)).expect("inspect");
        assert!(report.folder_exists);
        assert!(report.owned.is_none());
        assert_eq!(report.next_name.as_deref(), Some("pak65_dir.vpk"));
        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].owner, SlotFileOwner::Unknown);
        let missing = inspect_language_slot(&app, &dota, Some(GameLanguage::Czech))
            .expect("inspect a folder that does not exist");
        assert!(!missing.folder_exists);
        assert!(missing.files.is_empty());
        assert!(
            !dota.join("game/dota_czech").exists(),
            "inspecting must not create the folder"
        );
        let _ = fs::remove_dir_all(base);
    }

    // -- map terrain (`maps/dota.vpk`) as a second owned target ---------------

    /// A well-formed whole-map package (VPK version 2, like the real ones).
    fn map_vpk(label: &'static [u8]) -> Vec<u8> {
        vpk::build_unchecked(
            2,
            &[
                ("maps/dota.vmap_c", label),
                ("maps/dota.vhcg", b"hcg".as_slice()),
                ("maps/dota/world_visibility.vvis_c", b"vis".as_slice()),
            ],
        )
    }

    /// The bytes of a build with their hashes, so a test can borrow a payload.
    struct Files {
        pak: Option<(Vec<u8>, String)>,
        map: Option<(Vec<u8>, String)>,
    }

    impl Files {
        fn new(pak: Option<Vec<u8>>, map: Option<Vec<u8>>) -> Self {
            let with_hash = |bytes: Vec<u8>| {
                let hash = sha256(&bytes);
                (bytes, hash)
            };
            Self {
                pak: pak.map(with_hash),
                map: map.map(with_hash),
            }
        }

        fn payload(&self) -> DeploymentPayload<'_> {
            DeploymentPayload {
                pak: self
                    .pak
                    .as_ref()
                    .map(|(bytes, sha256)| PayloadFile { bytes, sha256 }),
                map: self
                    .map
                    .as_ref()
                    .map(|(bytes, sha256)| PayloadFile { bytes, sha256 }),
            }
        }
    }

    fn bundle_of(label: &str, ids: &[&str]) -> DeploymentBundleIdentity {
        DeploymentBundleIdentity {
            plan_id: Some(format!("sha256:{label}")),
            package_ids: ids.iter().map(|id| id.to_string()).collect(),
        }
    }

    fn install_files(
        app: &Path,
        dota: &Path,
        files: &Files,
        label: &str,
    ) -> Result<DeploymentReceipt, String> {
        deploy_verified_payload_for_language(
            app,
            dota,
            files.payload(),
            GameLanguage::Dutch,
            bundle_of(label, &["map.test", "wardrobe.test"]),
        )
    }

    fn map_file(dota: &Path) -> PathBuf {
        dota.join("game/dota_dutch/maps/dota.vpk")
    }

    fn pak_file(dota: &Path, name: &str) -> PathBuf {
        dota.join("game/dota_dutch").join(name)
    }

    #[test]
    fn a_map_alone_is_installed_owned_verified_and_removed_with_its_folders() {
        let base = root("map-alone");
        let app = base.join("app");
        let dota = game(&base);
        let files = Files::new(None, Some(map_vpk(b"ti5")));
        let map_hash = files.map.as_ref().expect("map").1.clone();
        let receipt = install_files(&app, &dota, &files, "one").expect("install a map");
        assert!(receipt.committed && receipt.backup_verified);
        assert!(!receipt.has_pak);
        assert_eq!(receipt.vpk_name, "");
        assert_eq!(
            receipt.installed_sha256, map_hash,
            "the map is the identity"
        );
        let map = receipt.map.clone().expect("map receipt");
        assert_eq!(map.file, "maps/dota.vpk");
        assert_eq!(map.installed_sha256.as_deref(), Some(map_hash.as_str()));
        assert_eq!(map.before_sha256, None);
        assert_eq!(
            fs::read(map_file(&dota)).expect("installed"),
            files.map.as_ref().expect("map").0
        );
        // No pak file was written for a map-only build.
        let names = fs::read_dir(dota.join("game/dota_dutch"))
            .expect("locale")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["maps"], "no pak file and no temporary file");
        let current = current_owned_deployment(&app, &dota)
            .expect("current")
            .expect("owned");
        assert_eq!(current.operation_id, receipt.operation_id);
        assert_eq!(current.map, receipt.map);
        assert_eq!(current.package_ids, vec!["map.test", "wardrobe.test"]);
        // BetterFy created `dota_dutch` and `maps`; restoring removes both.
        let rolled = rollback(&app, &dota, &receipt.operation_id).expect("restore");
        assert!(rolled.rolled_back && !rolled.committed);
        assert!(!dota.join("game/dota_dutch").exists());
        assert!(current_owned_deployment(&app, &dota)
            .expect("nothing owned")
            .is_none());
        // Restoring again is a no-op that reports the same result.
        assert!(
            rollback(&app, &dota, &receipt.operation_id)
                .expect("repeat")
                .rolled_back
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn a_pak_and_a_map_install_together_and_restore_together() {
        let base = root("map-and-pak");
        let app = base.join("app");
        let dota = game(&base);
        let files = Files::new(Some(package(b"hero skin").to_vec()), Some(map_vpk(b"ti6")));
        let receipt = install_files(&app, &dota, &files, "both").expect("install both");
        assert!(receipt.has_pak);
        assert_eq!(receipt.vpk_name, "pak65_dir.vpk");
        assert_eq!(
            receipt.installed_sha256,
            files.pak.as_ref().expect("pak").1,
            "with a pak the pak identifies the build"
        );
        assert_eq!(
            fs::read(pak_file(&dota, "pak65_dir.vpk")).expect("pak"),
            files.pak.as_ref().expect("pak").0
        );
        assert_eq!(
            fs::read(map_file(&dota)).expect("map"),
            files.map.as_ref().expect("map").0
        );
        // The evidence report names both files and carries no path.
        let report = collect_evidence(&app, &dota, "0.1-test".to_string()).expect("evidence");
        let entry = &report.entries[0];
        assert!(entry.has_pak && entry.active);
        let evidence_map = entry.map.as_ref().expect("map evidence");
        assert_eq!(evidence_map.file, "maps/dota.vpk");
        assert!(evidence_map.backup_verified);
        let serialized = serde_json::to_string(&report).expect("serialize");
        assert!(serialized.contains("maps/dota.vpk"));
        assert!(!serialized.contains(&dota.to_string_lossy().to_string()));
        rollback(&app, &dota, &receipt.operation_id).expect("restore");
        assert!(!dota.join("game/dota_dutch").exists());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn updates_add_change_and_drop_the_map_and_each_step_restores_exactly() {
        let base = root("map-chain");
        let app = base.join("app");
        let dota = game(&base);
        // 1. a pak alone
        let one = Files::new(Some(package(b"skin one").to_vec()), None);
        let first = install_files(&app, &dota, &one, "one").expect("pak only");
        assert!(first.map.is_none());
        // 2. the same install gains a map: the pak is replaced, the map is new
        let two = Files::new(Some(package(b"skin two").to_vec()), Some(map_vpk(b"ti7")));
        let second = install_files(&app, &dota, &two, "two").expect("add a map");
        assert_eq!(
            second.vpk_name, first.vpk_name,
            "an owned install keeps its name"
        );
        // 3. the map changes, the pak stays
        let three = Files::new(Some(package(b"skin two").to_vec()), Some(map_vpk(b"ti8")));
        let third = install_files(&app, &dota, &three, "three").expect("change the map");
        assert_eq!(
            third.map.as_ref().expect("map").before_sha256,
            second
                .map
                .as_ref()
                .and_then(|map| map.installed_sha256.clone())
        );
        // 4. the map is dropped: BetterFy removes its own file, with a backup
        let four = Files::new(Some(package(b"skin two").to_vec()), None);
        let fourth = install_files(&app, &dota, &four, "four").expect("drop the map");
        assert!(!map_file(&dota).exists());
        assert_eq!(
            fourth.map.as_ref().expect("map step").installed_sha256,
            None
        );
        assert!(fourth.map.as_ref().expect("map step").backup_verified);
        // 5. the pak is dropped for a map alone (the pak is removed)
        let five = Files::new(None, Some(map_vpk(b"ti9")));
        let fifth = install_files(&app, &dota, &five, "five").expect("map only");
        assert!(!pak_file(&dota, "pak65_dir.vpk").exists());
        assert!(!fifth.has_pak);

        // Restoring walks the chain back, exactly.
        let back_to_four = rollback(&app, &dota, &fifth.operation_id).expect("undo five");
        assert!(back_to_four.rolled_back);
        assert_eq!(
            fs::read(pak_file(&dota, "pak65_dir.vpk")).expect("pak is back"),
            four.pak.as_ref().expect("pak").0
        );
        assert!(!map_file(&dota).exists());
        let owned = current_owned_deployment(&app, &dota)
            .expect("current")
            .expect("owned");
        assert_eq!(owned.operation_id, fourth.operation_id);
        rollback(&app, &dota, &fourth.operation_id).expect("undo four");
        assert_eq!(
            fs::read(map_file(&dota)).expect("map is back"),
            three.map.as_ref().expect("map").0
        );
        rollback(&app, &dota, &third.operation_id).expect("undo three");
        assert_eq!(
            fs::read(map_file(&dota)).expect("earlier map"),
            two.map.as_ref().expect("map").0
        );
        rollback(&app, &dota, &second.operation_id).expect("undo two");
        assert!(!map_file(&dota).exists(), "the map was new in step two");
        assert!(
            !dota.join("game/dota_dutch/maps").exists(),
            "BetterFy created the empty maps folder and removes it"
        );
        assert_eq!(
            fs::read(pak_file(&dota, "pak65_dir.vpk")).expect("first pak"),
            one.pak.as_ref().expect("pak").0
        );
        rollback(&app, &dota, &first.operation_id).expect("undo one");
        assert!(!dota.join("game/dota_dutch").exists());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn a_map_file_that_betterfy_did_not_write_stops_the_install_untouched() {
        let base = root("map-foreign");
        let app = base.join("app");
        let dota = game(&base);
        let maps = dota.join("game/dota_dutch/maps");
        fs::create_dir_all(&maps).expect("maps folder");
        fs::write(maps.join("dota.vpk"), b"somebody else's map").expect("foreign map");
        let files = Files::new(Some(package(b"skin").to_vec()), Some(map_vpk(b"ti5")));
        assert_eq!(
            preview_destination(&app, &dota, GameLanguage::Dutch, true)
                .err()
                .as_deref(),
            Some("map_target_foreign"),
            "the refusal comes before Steam and Dota are closed"
        );
        preview_destination(&app, &dota, GameLanguage::Dutch, false)
            .expect("a build without a map is not affected");
        assert_eq!(
            install_files(&app, &dota, &files, "foreign")
                .err()
                .as_deref(),
            Some("map_target_foreign")
        );
        assert_eq!(
            fs::read(maps.join("dota.vpk")).expect("kept"),
            b"somebody else's map"
        );
        assert!(
            !pak_file(&dota, "pak65_dir.vpk").exists(),
            "nothing is written when the map cannot be"
        );
        assert!(current_owned_deployment(&app, &dota)
            .expect("nothing owned")
            .is_none());
        let counts = diagnostic_counts(&app).expect("counts");
        assert_eq!(counts.total, 0, "no operation was started");
        // The same file is refused even when it is byte-identical to the new
        // map: BetterFy cannot prove it wrote it.
        fs::write(maps.join("dota.vpk"), &files.map.as_ref().expect("map").0).expect("same bytes");
        assert_eq!(
            install_files(&app, &dota, &files, "same").err().as_deref(),
            Some("map_target_foreign")
        );
        // A directory where the file belongs is refused as well.
        fs::remove_file(maps.join("dota.vpk")).expect("remove");
        fs::create_dir(maps.join("dota.vpk")).expect("dir in the way");
        assert_eq!(
            install_files(&app, &dota, &files, "dir").err().as_deref(),
            Some("map_target_foreign")
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn a_maps_path_that_is_not_a_plain_folder_is_refused() {
        let base = root("map-folder");
        let app = base.join("app");
        let dota = game(&base);
        let locale = dota.join("game/dota_dutch");
        fs::create_dir_all(&locale).expect("locale");
        fs::write(locale.join("maps"), b"a file named maps").expect("file");
        let files = Files::new(None, Some(map_vpk(b"ti5")));
        assert_eq!(
            install_files(&app, &dota, &files, "file").err().as_deref(),
            Some("map_folder_invalid")
        );
        assert_eq!(
            preview_destination(&app, &dota, GameLanguage::Dutch, true)
                .err()
                .as_deref(),
            Some("map_folder_invalid")
        );
        assert_eq!(
            fs::read(locale.join("maps")).expect("kept"),
            b"a file named maps"
        );
        let _ = fs::remove_dir_all(base);
    }

    #[cfg(unix)]
    #[test]
    fn links_in_the_map_path_are_refused() {
        use std::os::unix::fs::symlink;
        let base = root("map-links");
        let app = base.join("app");
        let dota = game(&base);
        let locale = dota.join("game/dota_dutch");
        let elsewhere = base.join("elsewhere");
        fs::create_dir_all(&locale).expect("locale");
        fs::create_dir_all(&elsewhere).expect("elsewhere");
        let files = Files::new(None, Some(map_vpk(b"ti5")));
        // A link where the folder belongs would send the write elsewhere.
        symlink(&elsewhere, locale.join("maps")).expect("folder link");
        assert_eq!(
            install_files(&app, &dota, &files, "folder")
                .err()
                .as_deref(),
            Some("deployment_path_unsafe")
        );
        assert!(fs::read_dir(&elsewhere)
            .expect("elsewhere")
            .next()
            .is_none());
        fs::remove_file(locale.join("maps")).expect("remove link");
        // A link where the file belongs is refused too.
        fs::create_dir(locale.join("maps")).expect("maps");
        fs::write(elsewhere.join("target.vpk"), b"target").expect("target");
        symlink(elsewhere.join("target.vpk"), locale.join("maps/dota.vpk")).expect("file link");
        assert_eq!(
            install_files(&app, &dota, &files, "file").err().as_deref(),
            Some("deployment_path_unsafe")
        );
        assert_eq!(
            fs::read(elsewhere.join("target.vpk")).expect("kept"),
            b"target"
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn a_map_folder_betterfy_did_not_create_stays_and_so_do_other_files_in_it() {
        let base = root("map-folder-kept");
        let app = base.join("app");
        let dota = game(&base);
        let maps = dota.join("game/dota_dutch/maps");
        fs::create_dir_all(&maps).expect("existing maps folder");
        let files = Files::new(None, Some(map_vpk(b"ti5")));
        let receipt = install_files(&app, &dota, &files, "kept").expect("install");
        fs::write(maps.join("notes.txt"), b"mine").expect("another file");
        rollback(&app, &dota, &receipt.operation_id).expect("restore");
        assert!(maps.is_dir(), "a folder BetterFy did not create stays");
        assert!(!maps.join("dota.vpk").exists());
        assert_eq!(fs::read(maps.join("notes.txt")).expect("kept"), b"mine");
        // And a folder BetterFy created stays too while it holds another file.
        fs::remove_file(maps.join("notes.txt")).expect("remove");
        fs::remove_dir(&maps).expect("remove folder");
        let second = install_files(&app, &dota, &files, "created").expect("install");
        fs::write(maps.join("notes.txt"), b"mine").expect("another file");
        rollback(&app, &dota, &second.operation_id).expect("restore");
        assert!(maps.is_dir() && maps.join("notes.txt").is_file());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn an_owned_map_that_changed_or_vanished_blocks_updates_and_can_be_let_go() {
        let base = root("map-stale");
        let app = base.join("app");
        let dota = game(&base);
        let files = Files::new(Some(package(b"skin").to_vec()), Some(map_vpk(b"ti5")));
        let receipt = install_files(&app, &dota, &files, "stale").expect("install");
        assert_eq!(
            release_stale_deployment(&app, &dota).err().as_deref(),
            Some("deployment_not_stale"),
            "a healthy install with a map is never released"
        );
        // A game update replaces the map.
        fs::write(map_file(&dota), b"valve map").expect("replace");
        assert_eq!(
            preview_destination(&app, &dota, GameLanguage::Dutch, true)
                .err()
                .as_deref(),
            Some("map_conflict")
        );
        assert_eq!(
            install_files(&app, &dota, &files, "again").err().as_deref(),
            Some("map_conflict")
        );
        assert_eq!(
            rollback(&app, &dota, &receipt.operation_id)
                .err()
                .as_deref(),
            Some("rollback_conflict")
        );
        assert_eq!(
            fs::read(pak_file(&dota, "pak65_dir.vpk")).expect("pak untouched"),
            files.pak.as_ref().expect("pak").0,
            "a conflict on one file leaves the other alone"
        );
        let report = inspect_language_slot(&app, &dota, None).expect("inspect");
        assert_eq!(report.owned_map, Some(OwnedFileState::Replaced));
        assert_eq!(
            report.map.as_ref().map(|map| map.owner),
            Some(SlotFileOwner::Unknown)
        );
        assert_eq!(
            current_owned_deployment(&app, &dota).err().as_deref(),
            Some("deployment_conflict")
        );
        let released = release_stale_deployment(&app, &dota).expect("release");
        assert_eq!(released.reason, StaleReason::Changed);
        assert_eq!(fs::read(map_file(&dota)).expect("kept"), b"valve map");
        // The valve map is now a foreign file: a new install refuses it.
        assert_eq!(
            install_files(&app, &dota, &files, "fresh").err().as_deref(),
            Some("map_target_foreign")
        );
        // A vanished map is reported as missing.
        fs::remove_file(map_file(&dota)).expect("remove");
        let fresh = install_files(&app, &dota, &files, "fresh").expect("fresh install");
        fs::remove_file(map_file(&dota)).expect("vanish");
        let missing = inspect_language_slot(&app, &dota, None).expect("inspect");
        assert_eq!(missing.owned_map, Some(OwnedFileState::Missing));
        assert_eq!(
            release_stale_deployment(&app, &dota)
                .expect("release")
                .reason,
            StaleReason::Missing
        );
        let _ = fresh;
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn bytes_that_are_not_a_proven_map_package_are_refused_before_anything_is_touched() {
        let base = root("map-invalid");
        let app = base.join("app");
        let dota = game(&base);
        // A script riding in the package.
        let script = vpk::build_unchecked(
            2,
            &[
                ("maps/dota.vmap_c", b"map".as_slice()),
                ("maps/dota/scripts/run.vjs_c", b"alert(1)".as_slice()),
            ],
        );
        let scripted = Files::new(None, Some(script));
        assert_eq!(
            install_files(&app, &dota, &scripted, "script")
                .err()
                .as_deref(),
            Some("map_entry_script")
        );
        // Right bytes, wrong hash.
        let good = map_vpk(b"ti5");
        let payload = DeploymentPayload {
            pak: None,
            map: Some(PayloadFile {
                bytes: &good,
                sha256: &"0".repeat(64),
            }),
        };
        assert_eq!(
            deploy_verified_payload_for_language(
                &app,
                &dota,
                payload,
                GameLanguage::Dutch,
                DeploymentBundleIdentity::default(),
            )
            .err()
            .as_deref(),
            Some("map_hash_mismatch")
        );
        // Nothing to install at all.
        assert_eq!(
            deploy_verified_payload_for_language(
                &app,
                &dota,
                DeploymentPayload::default(),
                GameLanguage::Dutch,
                DeploymentBundleIdentity::default(),
            )
            .err()
            .as_deref(),
            Some("package_invalid")
        );
        assert!(!dota.join("game/dota_dutch").exists());
        assert_eq!(diagnostic_counts(&app).expect("counts").total, 0);
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn a_stop_before_publish_leaves_both_files_alone_and_cleans_up() {
        let base = root("map-before-publish");
        let app = base.join("app");
        let dota = game(&base);
        let files = Files::new(Some(package(b"skin").to_vec()), Some(map_vpk(b"ti5")));
        assert_eq!(
            deploy_payload_with_failure(
                &app,
                &dota,
                files.payload(),
                GameLanguage::Dutch,
                DeploymentBundleIdentity::default(),
                FailurePoint::AfterPrepared,
            )
            .err()
            .as_deref(),
            Some("injected_failure")
        );
        assert!(!pak_file(&dota, "pak65_dir.vpk").exists());
        assert!(!map_file(&dota).exists());
        let recovery = recover_pending(&app, &dota).expect("recover");
        assert_eq!((recovery.marked_failed, recovery.rolled_back), (1, 0));
        assert!(
            !dota.join("game/dota_dutch").exists(),
            "the folders the operation created are removed"
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn a_stop_between_the_two_publishes_is_rolled_back_whole() {
        let base = root("map-between");
        let app = base.join("app");
        let dota = game(&base);
        let files = Files::new(Some(package(b"skin").to_vec()), Some(map_vpk(b"ti5")));
        assert_eq!(
            deploy_payload_with_failure(
                &app,
                &dota,
                files.payload(),
                GameLanguage::Dutch,
                DeploymentBundleIdentity::default(),
                FailurePoint::BetweenPublishes,
            )
            .err()
            .as_deref(),
            Some("injected_failure")
        );
        assert!(pak_file(&dota, "pak65_dir.vpk").is_file(), "the pak is out");
        assert!(!map_file(&dota).exists(), "the map is not");
        assert!(current_owned_deployment(&app, &dota)
            .expect("no ownership yet")
            .is_none());
        let recovery = recover_pending(&app, &dota).expect("recover");
        assert_eq!((recovery.rolled_back, recovery.marked_failed), (1, 0));
        assert!(!dota.join("game/dota_dutch").exists());
        let report = collect_evidence(&app, &dota, "0.1-test".to_string()).expect("evidence");
        assert_eq!(report.entries[0].phase, DeploymentPhase::RolledBack);
        assert!(report.entries[0].rollback_verified);
        assert!(!report.entries[0].active);
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn a_stop_after_both_publishes_is_rolled_back_to_the_previous_build() {
        let base = root("map-after");
        let app = base.join("app");
        let dota = game(&base);
        let before = Files::new(Some(package(b"skin one").to_vec()), Some(map_vpk(b"ti5")));
        let first = install_files(&app, &dota, &before, "first").expect("first");
        let after = Files::new(Some(package(b"skin two").to_vec()), Some(map_vpk(b"ti6")));
        assert_eq!(
            deploy_payload_with_failure(
                &app,
                &dota,
                after.payload(),
                GameLanguage::Dutch,
                DeploymentBundleIdentity::default(),
                FailurePoint::AfterReplace,
            )
            .err()
            .as_deref(),
            Some("injected_failure")
        );
        assert_eq!(
            fs::read(map_file(&dota)).expect("new map is out"),
            after.map.as_ref().expect("map").0
        );
        let recovery = recover_pending(&app, &dota).expect("recover");
        assert_eq!(recovery.rolled_back, 1);
        assert_eq!(
            fs::read(map_file(&dota)).expect("old map is back"),
            before.map.as_ref().expect("map").0
        );
        assert_eq!(
            fs::read(pak_file(&dota, "pak65_dir.vpk")).expect("old pak is back"),
            before.pak.as_ref().expect("pak").0
        );
        let owned = current_owned_deployment(&app, &dota)
            .expect("current")
            .expect("owned");
        assert_eq!(owned.operation_id, first.operation_id);
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn a_restore_that_stopped_halfway_is_finished_by_the_next_one() {
        let base = root("map-resume");
        let app = base.join("app");
        let dota = game(&base);
        let before = Files::new(Some(package(b"skin one").to_vec()), Some(map_vpk(b"ti5")));
        install_files(&app, &dota, &before, "first").expect("first");
        let after = Files::new(Some(package(b"skin two").to_vec()), Some(map_vpk(b"ti6")));
        let second = install_files(&app, &dota, &after, "second").expect("second");
        // The map is already back, the pak is not (the app stopped between).
        fs::write(map_file(&dota), &before.map.as_ref().expect("map").0).expect("half restore");
        let rolled = rollback(&app, &dota, &second.operation_id).expect("finish the restore");
        assert!(rolled.rolled_back);
        assert_eq!(
            fs::read(pak_file(&dota, "pak65_dir.vpk")).expect("pak"),
            before.pak.as_ref().expect("pak").0
        );
        assert_eq!(
            fs::read(map_file(&dota)).expect("map"),
            before.map.as_ref().expect("map").0
        );
        // A file that is neither state is still a conflict, and nothing moves.
        let third = install_files(&app, &dota, &after, "third").expect("third");
        fs::write(map_file(&dota), b"edited by hand").expect("edit");
        assert_eq!(
            rollback(&app, &dota, &third.operation_id).err().as_deref(),
            Some("rollback_conflict")
        );
        assert_eq!(
            fs::read(pak_file(&dota, "pak65_dir.vpk")).expect("pak untouched"),
            after.pak.as_ref().expect("pak").0
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn records_of_a_build_without_a_map_are_unchanged_on_disk() {
        let base = root("map-compat");
        let app = base.join("app");
        let dota = game(&base);
        let bytes = package(b"skin");
        let receipt = deploy_verified_vpk(&app, &dota, &bytes, &sha256(&bytes)).expect("deploy");
        let engine = app.join("engine-v1/game-deployment");
        for path in [
            engine.join("ownership.json"),
            engine
                .join("journals")
                .join(format!("{}.json", receipt.operation_id)),
        ] {
            let text = fs::read_to_string(&path).expect("record");
            assert!(!text.contains("noPak"), "{text}");
            assert!(!text.contains("\"map\""), "{text}");
        }
        assert!(receipt.map.is_none() && receipt.has_pak);
        // A record that carries a map is refused when it contradicts itself.
        let journal_path = engine
            .join("journals")
            .join(format!("{}.json", receipt.operation_id));
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&journal_path).expect("journal")).expect("json");
        value["noPak"] = serde_json::Value::Bool(true);
        fs::write(&journal_path, serde_json::to_vec(&value).expect("bytes")).expect("write");
        assert_eq!(
            rollback(&app, &dota, &receipt.operation_id)
                .err()
                .as_deref(),
            Some("deployment_journal_invalid"),
            "no pak and no map leaves nothing installed"
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn the_slot_report_describes_the_map_by_name_size_and_owner_only() {
        let base = root("map-inspect");
        let app = base.join("app");
        let dota = game(&base);
        let maps = dota.join("game/dota_dutch/maps");
        fs::create_dir_all(&maps).expect("maps");
        fs::write(maps.join("dota.vpk"), b"somebody's map").expect("foreign");
        let report =
            inspect_language_slot(&app, &dota, Some(GameLanguage::Dutch)).expect("inspect");
        let map = report.map.expect("map entry");
        assert_eq!(map.bytes, b"somebody's map".len() as u64);
        assert_eq!(map.owner, SlotFileOwner::Unknown);
        assert_eq!(report.owned_map, None);
        let serialized = serde_json::to_string(
            &inspect_language_slot(&app, &dota, Some(GameLanguage::Dutch)).expect("inspect"),
        )
        .expect("serialize");
        assert!(!serialized.contains("somebody's map"));
        assert!(!serialized.contains(&dota.to_string_lossy().to_string()));
        fs::remove_file(maps.join("dota.vpk")).expect("remove");
        let files = Files::new(None, Some(map_vpk(b"ti5")));
        install_files(&app, &dota, &files, "mine").expect("install");
        let mine = inspect_language_slot(&app, &dota, None).expect("inspect");
        assert_eq!(mine.owned_map, Some(OwnedFileState::Intact));
        assert_eq!(mine.map.expect("map").owner, SlotFileOwner::Betterfy);
        assert!(mine.owned.is_none(), "a map-only install owns no pak file");
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn a_map_installs_into_any_language_slot_and_never_across_slots() {
        let base = root("map-language");
        let app = base.join("app");
        let dota = game(&base);
        let files = Files::new(None, Some(map_vpk(b"ti5")));
        let receipt = deploy_verified_payload_for_language(
            &app,
            &dota,
            files.payload(),
            GameLanguage::Russian,
            DeploymentBundleIdentity::default(),
        )
        .expect("install into Russian");
        assert!(dota.join("game/dota_russian/maps/dota.vpk").is_file());
        assert_eq!(
            deploy_verified_payload_for_language(
                &app,
                &dota,
                files.payload(),
                GameLanguage::Dutch,
                DeploymentBundleIdentity::default(),
            )
            .err()
            .as_deref(),
            Some("deployment_language_change_requires_restore")
        );
        assert!(!dota.join("game/dota_dutch").exists());
        rollback(&app, &dota, &receipt.operation_id).expect("restore");
        assert!(!dota.join("game/dota_russian").exists());
        let _ = fs::remove_dir_all(base);
    }
}

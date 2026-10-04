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
use crate::vpk;

const SCHEMA_VERSION: u32 = 1;
const OWNED_VPK_NAME: &str = "pak66_dir.vpk";
const MAX_PACKAGE_BYTES: usize = 256 * 1024 * 1024;
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentReceipt {
    pub operation_id: String,
    pub language: GameLanguage,
    pub before_sha256: Option<String>,
    pub installed_sha256: String,
    pub backup_verified: bool,
    pub committed: bool,
    pub rolled_back: bool,
    pub bundle_plan_id: Option<String>,
    pub package_ids: Vec<String>,
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OwnershipState {
    schema_version: u32,
    target_identity: String,
    #[serde(default)]
    language: GameLanguage,
    installed_sha256: String,
    operation_id: String,
    #[serde(default)]
    bundle_plan_id: Option<String>,
    #[serde(default)]
    package_ids: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DeploymentBundleIdentity {
    pub plan_id: Option<String>,
    pub package_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailurePoint {
    None,
    AfterPrepared,
    AfterReplace,
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

fn target_for(dota_root: &Path, language: GameLanguage) -> Result<PathBuf, String> {
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
    let target = locale.join(OWNED_VPK_NAME);
    reject_symlink(&target)?;
    Ok(target)
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

fn ownership_path(root: &Path) -> PathBuf {
    root.join("ownership.json")
}

fn remove_locale_if_empty(locale: &Path) -> Result<(), String> {
    reject_symlink(locale)?;
    if !locale.is_dir() {
        return Ok(());
    }
    let mut entries = fs::read_dir(locale).map_err(|_| "rollback_failed".to_string())?;
    if entries.next().is_none() {
        fs::remove_dir(locale).map_err(|_| "rollback_failed".to_string())?;
    }
    Ok(())
}

fn deploy_with_failure(
    app_data_root: &Path,
    dota_root: &Path,
    package: &[u8],
    expected_sha256: &str,
    language: GameLanguage,
    bundle: DeploymentBundleIdentity,
    failure: FailurePoint,
) -> Result<DeploymentReceipt, String> {
    let installed_sha256 = verify_package(package, expected_sha256)?;
    let target = target_for(dota_root, language)?;
    let identity = target_identity(dota_root)?;
    let (root, operations, journals) = owned_roots(app_data_root)?;
    let _lock = transaction_lock(&root)?;

    if ownership_path(&root).exists() {
        let state: OwnershipState = read_json(&ownership_path(&root))?;
        if state.schema_version != SCHEMA_VERSION || state.target_identity != identity {
            return Err("deployment_journal_invalid".to_string());
        }
        if state.language != language {
            return Err("deployment_language_change_requires_restore".to_string());
        }
        if !target.is_file() {
            return Err("deployment_conflict".to_string());
        }
    }

    let before = if target.exists() {
        let bytes = fs::read(&target).map_err(|_| "deployment_failed".to_string())?;
        let before_hash = sha256(&bytes);
        let state: OwnershipState = read_json(&ownership_path(&root))
            .map_err(|_| "deployment_target_foreign".to_string())?;
        if state.schema_version != SCHEMA_VERSION
            || state.target_identity != identity
            || state.language != language
            || state.installed_sha256 != before_hash
        {
            return Err("deployment_target_foreign".to_string());
        }
        Some((bytes, before_hash, state.operation_id))
    } else {
        None
    };

    let locale = target
        .parent()
        .ok_or_else(|| "deployment_failed".to_string())?;
    reject_symlink(locale)?;
    let locale_directory_created = !locale.exists();
    fs::create_dir_all(locale).map_err(|_| "deployment_failed".to_string())?;
    reject_symlink(locale)?;
    reject_symlink(&target)?;

    let operation_id = new_operation_id()?;
    let operation_root = operations.join(&operation_id);
    fs::create_dir(&operation_root).map_err(|_| "deployment_failed".to_string())?;
    let backup_path = operation_root.join("before.vpk");
    let before_sha256 = before.as_ref().map(|(_, hash, _)| hash.clone());
    let before_operation_id = before
        .as_ref()
        .map(|(_, _, operation_id)| operation_id.clone());
    if let Some((bytes, hash, _)) = &before {
        write_new_synced(&backup_path, bytes)?;
        if sha256(&fs::read(&backup_path).map_err(|_| "backup_failed".to_string())?) != *hash {
            return Err("backup_verification_failed".to_string());
        }
    }
    let timestamp = now_ms()?;
    let journal_path = journals.join(format!("{operation_id}.json"));
    let mut journal = DeploymentJournal {
        schema_version: SCHEMA_VERSION,
        operation_id: operation_id.clone(),
        target_identity: identity.clone(),
        language,
        before_sha256: before_sha256.clone(),
        before_operation_id,
        installed_sha256: installed_sha256.clone(),
        bundle_plan_id: bundle.plan_id.clone(),
        package_ids: bundle.package_ids.clone(),
        phase: DeploymentPhase::BackedUp,
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
        backup_relative_path: before.as_ref().map(|_| "before.vpk".to_string()),
        rollback_verified: false,
        locale_directory_created,
        error_code: None,
    };
    atomic_json(&journal_path, &journal)?;

    let temporary = target
        .parent()
        .ok_or_else(|| "deployment_failed".to_string())?
        .join(format!(".betterfy-{operation_id}.tmp"));
    write_new_synced(&temporary, package)?;
    if verify_package(
        &fs::read(&temporary).map_err(|_| "deployment_write_failed".to_string())?,
        expected_sha256,
    )
    .is_err()
    {
        let _ = fs::remove_file(&temporary);
        return Err("verification_failed".to_string());
    }
    journal.phase = DeploymentPhase::Prepared;
    journal.updated_at_ms = now_ms()?;
    atomic_json(&journal_path, &journal)?;
    if failure == FailurePoint::AfterPrepared {
        let _ = fs::remove_file(&temporary);
        return Err("injected_failure".to_string());
    }
    publish(&target, &temporary, before.is_some())?;
    if failure == FailurePoint::AfterReplace {
        return Err("injected_failure".to_string());
    }
    verify_package(
        &fs::read(&target).map_err(|_| "verification_failed".to_string())?,
        expected_sha256,
    )?;
    atomic_json(
        &ownership_path(&root),
        &OwnershipState {
            schema_version: SCHEMA_VERSION,
            target_identity: identity,
            language,
            installed_sha256: installed_sha256.clone(),
            operation_id: operation_id.clone(),
            bundle_plan_id: bundle.plan_id.clone(),
            package_ids: bundle.package_ids.clone(),
        },
    )?;
    journal.phase = DeploymentPhase::Committed;
    journal.updated_at_ms = now_ms()?;
    atomic_json(&journal_path, &journal)?;
    Ok(DeploymentReceipt {
        operation_id,
        language,
        before_sha256,
        installed_sha256,
        backup_verified: before.is_none() || backup_path.is_file(),
        committed: true,
        rolled_back: false,
        bundle_plan_id: bundle.plan_id,
        package_ids: bundle.package_ids,
    })
}

pub(crate) fn deploy_verified_vpk(
    app_data_root: &Path,
    dota_root: &Path,
    package: &[u8],
    expected_sha256: &str,
) -> Result<DeploymentReceipt, String> {
    deploy_with_failure(
        app_data_root,
        dota_root,
        package,
        expected_sha256,
        GameLanguage::Dutch,
        DeploymentBundleIdentity::default(),
        FailurePoint::None,
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

pub(crate) fn preview_language_destination(
    app_data_root: &Path,
    dota_root: &Path,
    language: GameLanguage,
) -> Result<(), String> {
    verify_language_folder(dota_root, language)?;
    let target = target_for(dota_root, language)?;
    let identity = target_identity(dota_root)?;
    let (root, _, _) = owned_roots(app_data_root)?;
    let _lock = transaction_lock(&root)?;
    let ownership_file = ownership_path(&root);
    if ownership_file.exists() {
        let state: OwnershipState = read_json(&ownership_file)?;
        if state.schema_version != SCHEMA_VERSION || state.target_identity != identity {
            return Err("deployment_journal_invalid".to_string());
        }
        if state.language != language {
            return Err("deployment_language_change_requires_restore".to_string());
        }
        if !target.is_file()
            || sha256(&fs::read(&target).map_err(|_| "deployment_conflict".to_string())?)
                != state.installed_sha256
        {
            return Err("deployment_conflict".to_string());
        }
    } else if target.exists() {
        return Err("deployment_target_foreign".to_string());
    }
    Ok(())
}

pub(crate) fn deploy_verified_bundle_for_language(
    app_data_root: &Path,
    dota_root: &Path,
    package: &[u8],
    expected_sha256: &str,
    language: GameLanguage,
    bundle: DeploymentBundleIdentity,
) -> Result<DeploymentReceipt, String> {
    verify_language_folder(dota_root, language)?;
    deploy_with_failure(
        app_data_root,
        dota_root,
        package,
        expected_sha256,
        language,
        bundle,
        FailurePoint::None,
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

pub(crate) fn deploy_verified_bundle_for_language_stress(
    app_data_root: &Path,
    dota_root: &Path,
    package: &[u8],
    expected_sha256: &str,
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
        deploy_with_failure(
            app_data_root,
            dota_root,
            package,
            expected_sha256,
            language,
            bundle,
            failure,
        )
    }
    #[cfg(not(feature = "internal-stress-test"))]
    {
        let _ = (
            app_data_root,
            dota_root,
            package,
            expected_sha256,
            language,
            bundle,
            failure,
        );
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
    if ownership.schema_version != SCHEMA_VERSION || ownership.target_identity != identity {
        return Err("deployment_journal_invalid".to_string());
    }
    validate_operation_id(&ownership.operation_id)?;
    let target = target_for(dota_root, ownership.language)?;
    let journal: DeploymentJournal =
        read_json(&journals.join(format!("{}.json", ownership.operation_id)))?;
    if journal.schema_version != SCHEMA_VERSION
        || journal.operation_id != ownership.operation_id
        || journal.target_identity != identity
        || journal.language != ownership.language
        || journal.installed_sha256 != ownership.installed_sha256
        || journal.bundle_plan_id != ownership.bundle_plan_id
        || journal.package_ids != ownership.package_ids
        || journal.phase != DeploymentPhase::Committed
    {
        return Err("deployment_journal_invalid".to_string());
    }
    let current = fs::read(&target).map_err(|_| "deployment_conflict".to_string())?;
    if sha256(&current) != ownership.installed_sha256 {
        return Err("deployment_conflict".to_string());
    }
    if let Some(expected) = &journal.before_sha256 {
        if journal.backup_relative_path.as_deref() != Some("before.vpk") {
            return Err("deployment_journal_invalid".to_string());
        }
        let backup = operations.join(&ownership.operation_id).join("before.vpk");
        reject_symlink(&backup)?;
        if sha256(&fs::read(&backup).map_err(|_| "backup_failed".to_string())?) != *expected {
            return Err("backup_verification_failed".to_string());
        }
    } else if journal.backup_relative_path.is_some() {
        return Err("deployment_journal_invalid".to_string());
    }
    Ok(Some(DeploymentReceipt {
        operation_id: ownership.operation_id,
        language: ownership.language,
        before_sha256: journal.before_sha256,
        installed_sha256: ownership.installed_sha256,
        backup_verified: true,
        committed: true,
        rolled_back: false,
        bundle_plan_id: ownership.bundle_plan_id,
        package_ids: ownership.package_ids,
    }))
}

pub(crate) fn rollback(
    app_data_root: &Path,
    dota_root: &Path,
    operation_id: &str,
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
    let target = target_for(dota_root, journal.language)?;
    if journal.phase == DeploymentPhase::RolledBack {
        return Ok(DeploymentReceipt {
            operation_id: operation_id.to_string(),
            language: journal.language,
            before_sha256: journal.before_sha256,
            installed_sha256: journal.installed_sha256,
            backup_verified: true,
            committed: false,
            rolled_back: true,
            bundle_plan_id: journal.bundle_plan_id,
            package_ids: journal.package_ids,
        });
    }
    let current = fs::read(&target).map_err(|_| "rollback_conflict".to_string())?;
    if sha256(&current) != journal.installed_sha256 {
        return Err("rollback_conflict".to_string());
    }
    match (
        &journal.before_sha256,
        &journal.before_operation_id,
        &journal.backup_relative_path,
    ) {
        (Some(expected), Some(previous_operation), Some(relative)) if relative == "before.vpk" => {
            let backup = operations.join(operation_id).join(relative);
            reject_symlink(&backup)?;
            let bytes = fs::read(&backup).map_err(|_| "backup_failed".to_string())?;
            if sha256(&bytes) != *expected {
                return Err("backup_verification_failed".to_string());
            }
            let replacement = target
                .parent()
                .ok_or_else(|| "rollback_failed".to_string())?
                .join(format!(".betterfy-rollback-{operation_id}.tmp"));
            write_new_synced(&replacement, &bytes)?;
            publish(&target, &replacement, true)?;
            let previous: DeploymentJournal =
                read_json(&journals.join(format!("{previous_operation}.json")))?;
            if previous.operation_id != *previous_operation
                || previous.target_identity != identity
                || previous.language != journal.language
                || previous.installed_sha256 != *expected
                || previous.phase != DeploymentPhase::Committed
            {
                return Err("deployment_journal_invalid".to_string());
            }
            atomic_json(
                &ownership_path(&root),
                &OwnershipState {
                    schema_version: SCHEMA_VERSION,
                    target_identity: identity.clone(),
                    language: journal.language,
                    installed_sha256: expected.clone(),
                    operation_id: previous_operation.clone(),
                    bundle_plan_id: previous.bundle_plan_id,
                    package_ids: previous.package_ids,
                },
            )?;
        }
        (None, None, None) => {
            fs::remove_file(&target).map_err(|_| "rollback_failed".to_string())?;
            let ownership = ownership_path(&root);
            if ownership.exists() {
                fs::remove_file(ownership).map_err(|_| "rollback_failed".to_string())?;
            }
            if journal.locale_directory_created {
                let locale = target
                    .parent()
                    .ok_or_else(|| "rollback_failed".to_string())?;
                remove_locale_if_empty(locale)?;
            }
        }
        _ => return Err("deployment_journal_invalid".to_string()),
    }
    journal.phase = DeploymentPhase::RolledBack;
    journal.updated_at_ms = now_ms()?;
    journal.rollback_verified = match &journal.before_sha256 {
        Some(expected) => {
            target.is_file()
                && sha256(&fs::read(&target).map_err(|_| "rollback_failed".to_string())?)
                    == *expected
        }
        None => !target.exists(),
    };
    if !journal.rollback_verified {
        return Err("rollback_verification_failed".to_string());
    }
    journal.error_code = None;
    atomic_json(&journal_path, &journal)?;
    Ok(DeploymentReceipt {
        operation_id: operation_id.to_string(),
        language: journal.language,
        before_sha256: journal.before_sha256,
        installed_sha256: journal.installed_sha256,
        backup_verified: true,
        committed: false,
        rolled_back: true,
        bundle_plan_id: journal.bundle_plan_id,
        package_ids: journal.package_ids,
    })
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
            let target = target_for(dota_root, journal.language)?;
            inspected += 1;
            let current = target
                .is_file()
                .then(|| fs::read(&target))
                .transpose()
                .map_err(|_| "deployment_recovery_conflict".to_string())?;
            let current_hash = current.as_deref().map(sha256);
            if current_hash.as_deref() == Some(journal.installed_sha256.as_str()) {
                rollback_ids.push(journal.operation_id.clone());
                continue;
            }
            if current_hash != journal.before_sha256 {
                return Err("deployment_recovery_conflict".to_string());
            }
            let temporary = target
                .parent()
                .ok_or_else(|| "deployment_recovery_conflict".to_string())?
                .join(format!(".betterfy-{}.tmp", journal.operation_id));
            if temporary.exists() {
                reject_symlink(&temporary)?;
                fs::remove_file(temporary)
                    .map_err(|_| "deployment_recovery_conflict".to_string())?;
            }
            if journal.locale_directory_created {
                let locale = target
                    .parent()
                    .ok_or_else(|| "deployment_recovery_conflict".to_string())?;
                remove_locale_if_empty(locale)
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
        rollback(app_data_root, dota_root, &operation_id)?;
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
        if ownership.schema_version != SCHEMA_VERSION || ownership.target_identity != identity {
            return Err("deployment_journal_invalid".to_string());
        }
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
        let backup_verified = match (&journal.before_sha256, &journal.backup_relative_path) {
            (None, None) => true,
            (Some(expected), Some(relative)) if relative == "before.vpk" => {
                let backup = operations.join(&journal.operation_id).join(relative);
                reject_symlink(&backup)?;
                fs::read(&backup)
                    .map(|bytes| sha256(&bytes) == *expected)
                    .unwrap_or(false)
            }
            _ => false,
        };
        journals_for_target.push((journal, backup_verified));
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
        .map(
            |(index, (journal, backup_verified))| DeploymentEvidenceEntry {
                sequence: index + 1,
                language: journal.language,
                phase: journal.phase,
                before_sha256: journal.before_sha256,
                installed_sha256: journal.installed_sha256,
                backup_verified,
                rollback_verified: journal.rollback_verified,
                active: active_operation.as_deref() == Some(journal.operation_id.as_str()),
                bundle_plan_id: journal.bundle_plan_id,
                package_ids: journal.package_ids,
                created_at_ms: journal.created_at_ms,
                updated_at_ms: journal.updated_at_ms,
                error_code: journal.error_code,
            },
        )
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let target = dota.join("game/dota_dutch/pak66_dir.vpk");
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
            fs::read(locale.join(OWNED_VPK_NAME)).expect("installed"),
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
            fs::read(locale.join(OWNED_VPK_NAME)).expect("installed"),
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

        // A VPK BetterFy did not write blocks the slot instead of being replaced.
        fs::create_dir_all(&locale).expect("foreign slot");
        fs::write(locale.join(OWNED_VPK_NAME), b"foreign").expect("foreign file");
        assert_eq!(
            preview_language_destination(&app, &dota, GameLanguage::Betterfy)
                .err()
                .as_deref(),
            Some("deployment_target_foreign")
        );
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
        assert!(locale.join(OWNED_VPK_NAME).is_file());
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
        fs::write(dota.join("game/dota_dutch/pak66_dir.vpk"), b"external edit")
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
                fs::read(selected.join(OWNED_VPK_NAME)).expect("installed"),
                bytes
            );
            assert_eq!(
                fs::read(&foreign).expect("foreign unchanged"),
                b"foreign data"
            );
            rollback(&app, &dota, &receipt.operation_id).expect("restore selected slot");
            assert!(!selected.join(OWNED_VPK_NAME).exists());
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
            fs::read(russian.join(OWNED_VPK_NAME)).expect("published before failure"),
            bytes
        );
        let recovered = recover_pending(&app, &dota).expect("recover Russian operation");
        assert_eq!(recovered.rolled_back, 1);
        assert!(!russian.join(OWNED_VPK_NAME).exists());
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
            fs::write(&path, serde_json::to_vec(&value).expect("JSON bytes"))
                .expect("legacy record");
        }
        let current = current_verified_deployment(&app, &dota, &sha256(&bytes))
            .expect("read old deployment")
            .expect("owned Dutch deployment");
        assert_eq!(current.language, GameLanguage::Dutch);
        rollback(&app, &dota, &receipt.operation_id).expect("restore old deployment");
        assert!(!dota.join("game/dota_dutch/pak66_dir.vpk").exists());
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
            fs::read(dota.join("game/dota_dutch/pak66_dir.vpk")).expect("restored"),
            first
        );
        rollback(&app, &dota, &first_receipt.operation_id).expect("rollback first");
        assert!(!dota.join("game/dota_dutch/pak66_dir.vpk").exists());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn refuses_foreign_targets_and_external_edits() {
        let base = root("foreign");
        let app = base.join("app");
        let dota = game(&base);
        fs::create_dir_all(dota.join("game/dota_dutch")).expect("locale");
        fs::write(dota.join("game/dota_dutch/pak66_dir.vpk"), b"foreign").expect("foreign");
        let bytes = package(b"tree");
        assert_eq!(
            preview_language_destination(&app, &dota, GameLanguage::Dutch)
                .err()
                .as_deref(),
            Some("deployment_target_foreign")
        );
        assert_eq!(
            deploy_verified_vpk(&app, &dota, &bytes, &sha256(&bytes))
                .err()
                .as_deref(),
            Some("deployment_target_foreign")
        );
        fs::remove_file(dota.join("game/dota_dutch/pak66_dir.vpk")).expect("remove");
        let receipt = deploy_verified_vpk(&app, &dota, &bytes, &sha256(&bytes)).expect("deploy");
        fs::write(dota.join("game/dota_dutch/pak66_dir.vpk"), b"external-edit").expect("edit");
        assert_eq!(
            rollback(&app, &dota, &receipt.operation_id)
                .err()
                .as_deref(),
            Some("rollback_conflict")
        );
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
        assert!(!dota.join("game/dota_dutch/pak66_dir.vpk").exists());
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
            fs::read(dota.join("game/dota_dutch/pak66_dir.vpk")).expect("published"),
            second
        );
        let pending = diagnostic_counts(&app).expect("diagnostics");
        assert_eq!(pending.total, 2);
        assert_eq!(pending.recoverable, 1);
        let second_recovery = recover_pending(&app, &dota).expect("recover published");
        assert_eq!(second_recovery.rolled_back, 1);
        assert!(!dota.join("game/dota_dutch/pak66_dir.vpk").exists());
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
        let _ = fs::remove_dir_all(base);
    }
}

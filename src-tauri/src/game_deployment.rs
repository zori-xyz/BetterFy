use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(not(target_os = "windows"))]
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

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
    pub before_sha256: Option<String>,
    pub installed_sha256: String,
    pub backup_verified: bool,
    pub committed: bool,
    pub rolled_back: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryReceipt {
    pub inspected: usize,
    pub rolled_back: usize,
    pub marked_failed: usize,
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
    before_sha256: Option<String>,
    before_operation_id: Option<String>,
    installed_sha256: String,
    phase: DeploymentPhase,
    created_at_ms: u128,
    updated_at_ms: u128,
    backup_relative_path: Option<String>,
    error_code: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OwnershipState {
    schema_version: u32,
    target_identity: String,
    installed_sha256: String,
    operation_id: String,
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

fn target_for(dota_root: &Path) -> Result<PathBuf, String> {
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
    let locale = game.join("dota_dutch");
    reject_symlink(&locale)?;
    fs::create_dir_all(&locale).map_err(|_| "deployment_failed".to_string())?;
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

fn deploy_with_failure(
    app_data_root: &Path,
    dota_root: &Path,
    package: &[u8],
    expected_sha256: &str,
    failure: FailurePoint,
) -> Result<DeploymentReceipt, String> {
    let installed_sha256 = verify_package(package, expected_sha256)?;
    let target = target_for(dota_root)?;
    let identity = target_identity(dota_root)?;
    let (root, operations, journals) = owned_roots(app_data_root)?;
    let _lock = transaction_lock(&root)?;

    let before = if target.exists() {
        let bytes = fs::read(&target).map_err(|_| "deployment_failed".to_string())?;
        let before_hash = sha256(&bytes);
        let state: OwnershipState = read_json(&ownership_path(&root))
            .map_err(|_| "deployment_target_foreign".to_string())?;
        if state.schema_version != SCHEMA_VERSION
            || state.target_identity != identity
            || state.installed_sha256 != before_hash
        {
            return Err("deployment_target_foreign".to_string());
        }
        Some((bytes, before_hash, state.operation_id))
    } else {
        None
    };

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
        before_sha256: before_sha256.clone(),
        before_operation_id,
        installed_sha256: installed_sha256.clone(),
        phase: DeploymentPhase::BackedUp,
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
        backup_relative_path: before.as_ref().map(|_| "before.vpk".to_string()),
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
            installed_sha256: installed_sha256.clone(),
            operation_id: operation_id.clone(),
        },
    )?;
    journal.phase = DeploymentPhase::Committed;
    journal.updated_at_ms = now_ms()?;
    atomic_json(&journal_path, &journal)?;
    Ok(DeploymentReceipt {
        operation_id,
        before_sha256,
        installed_sha256,
        backup_verified: before.is_none() || backup_path.is_file(),
        committed: true,
        rolled_back: false,
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
        FailurePoint::None,
    )
}

pub(crate) fn rollback(
    app_data_root: &Path,
    dota_root: &Path,
    operation_id: &str,
) -> Result<DeploymentReceipt, String> {
    validate_operation_id(operation_id)?;
    let target = target_for(dota_root)?;
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
    if journal.phase == DeploymentPhase::RolledBack {
        return Ok(DeploymentReceipt {
            operation_id: operation_id.to_string(),
            before_sha256: journal.before_sha256,
            installed_sha256: journal.installed_sha256,
            backup_verified: true,
            committed: false,
            rolled_back: true,
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
            atomic_json(
                &ownership_path(&root),
                &OwnershipState {
                    schema_version: SCHEMA_VERSION,
                    target_identity: identity.clone(),
                    installed_sha256: expected.clone(),
                    operation_id: previous_operation.clone(),
                },
            )?;
        }
        (None, None, None) => {
            fs::remove_file(&target).map_err(|_| "rollback_failed".to_string())?;
            let ownership = ownership_path(&root);
            if ownership.exists() {
                fs::remove_file(ownership).map_err(|_| "rollback_failed".to_string())?;
            }
        }
        _ => return Err("deployment_journal_invalid".to_string()),
    }
    journal.phase = DeploymentPhase::RolledBack;
    journal.updated_at_ms = now_ms()?;
    journal.error_code = None;
    atomic_json(&journal_path, &journal)?;
    Ok(DeploymentReceipt {
        operation_id: operation_id.to_string(),
        before_sha256: journal.before_sha256,
        installed_sha256: journal.installed_sha256,
        backup_verified: true,
        committed: false,
        rolled_back: true,
    })
}

pub(crate) fn recover_pending(
    app_data_root: &Path,
    dota_root: &Path,
) -> Result<RecoveryReceipt, String> {
    let target = target_for(dota_root)?;
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let rolled_back = rollback(&app, &dota, &receipt.operation_id).expect("rollback");
        assert!(rolled_back.rolled_back);
        assert!(!target.exists());
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
        let _ = fs::remove_dir_all(base);
    }
}

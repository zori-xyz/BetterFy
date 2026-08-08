use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::content_store::{self, ContentIntakeRequest, FixturePackageContract};

const ENGINE_SCHEMA_VERSION: u32 = 1;
const MAX_OPERATION_FILES: usize = 256;
const MAX_STAGED_BYTES: u64 = 64 * 1024 * 1024;
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureManifest {
    id: String,
    name: String,
    version: String,
    files: Vec<FixtureFile>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureFile {
    source: String,
    destination: String,
    size: u64,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildPlanRequest {
    pub mod_ids: Vec<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecuteBuildRequest {
    pub mod_ids: Vec<String>,
    pub expected_plan_id: String,
    pub confirmed: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanInput {
    id: String,
    name: String,
    version: String,
    content_identity: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanOperation {
    owner_id: String,
    source: String,
    destination: String,
    size: u64,
    expected_sha256: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanConflict {
    destination: String,
    contenders: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildPlan {
    plan_id: String,
    dry_run: bool,
    inputs: Vec<PlanInput>,
    operations: Vec<PlanOperation>,
    conflicts: Vec<PlanConflict>,
    space_estimate: u64,
    executable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationPhase {
    Staging,
    Verifying,
    Ready,
    Failed,
    RolledBack,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct JournalFile {
    owner_id: String,
    destination: String,
    expected_sha256: String,
    actual_sha256: Option<String>,
    size: u64,
    staged: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BuildJournal {
    schema_version: u32,
    operation_id: String,
    plan_id: String,
    phase: OperationPhase,
    created_at_ms: u128,
    updated_at_ms: u128,
    staged_root: String,
    files: Vec<JournalFile>,
    error_code: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildReceipt {
    operation_id: String,
    plan_id: String,
    phase: OperationPhase,
    staged_root: String,
    staged_files: usize,
    staged_bytes: u64,
    checksums_verified: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationSummary {
    operation_id: String,
    plan_id: String,
    phase: OperationPhase,
    created_at_ms: u128,
    staged_files: usize,
}

#[derive(Clone, Debug)]
pub struct OperationDiagnosticCounts {
    pub total: usize,
    pub recoverable: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackReceipt {
    operation_id: String,
    phase: OperationPhase,
    removed_staging: bool,
}

fn fixture_manifests() -> Result<Vec<FixtureManifest>, String> {
    [
        include_str!("../fixtures/mods/ambient-violet.json"),
        include_str!("../fixtures/mods/ambient-clean.json"),
    ]
    .iter()
    .map(|contents| serde_json::from_str(contents).map_err(|_| "catalog_invalid".to_string()))
    .collect()
}

fn fixture_payload(source: &str) -> Option<&'static [u8]> {
    match source {
        "panorama/styles/ambient-violet.css" => {
            Some(include_bytes!("../fixtures/payloads/ambient-violet.css"))
        }
        "panorama/styles/ambient-clean.css" => {
            Some(include_bytes!("../fixtures/payloads/ambient-clean.css"))
        }
        _ => None,
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn validate_relative_path(value: &str) -> Result<(), String> {
    if value.is_empty() || value.contains('\\') || value.contains(':') {
        return Err("catalog_invalid".to_string());
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("catalog_invalid".to_string());
    }
    Ok(())
}

fn validate_recipe_contract(
    manifest: &FixtureManifest,
    contract: &FixturePackageContract,
) -> Result<(), String> {
    if manifest.version != contract.version
        || contract.recipe_version != 1
        || manifest.files.len() != 1
    {
        return Err("catalog_invalid".to_string());
    }
    let file = &manifest.files[0];
    let source_name = Path::new(&file.source)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "catalog_invalid".to_string())?;
    if source_name != contract.file_name || file.size != contract.size {
        return Err("catalog_invalid".to_string());
    }
    let payload = fixture_payload(&file.source).ok_or_else(|| "catalog_invalid".to_string())?;
    if payload.len() as u64 != contract.size || sha256(payload) != contract.sha256 {
        return Err("catalog_invalid".to_string());
    }
    Ok(())
}

pub fn create_build_plan(request: BuildPlanRequest) -> Result<BuildPlan, String> {
    let available = fixture_manifests()?;
    let requested: BTreeSet<String> = request
        .mod_ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();
    let mut selected = Vec::new();
    for requested_id in requested {
        let manifest = available
            .iter()
            .find(|manifest| manifest.id == requested_id)
            .ok_or_else(|| "catalog_invalid".to_string())?;
        let contract = content_store::fixture_package_contract(&requested_id)
            .map_err(|_| "catalog_invalid".to_string())?;
        validate_recipe_contract(manifest, &contract)?;
        selected.push((manifest, contract));
    }

    let mut operations = Vec::new();
    let mut destinations: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (manifest, contract) in &selected {
        for file in &manifest.files {
            validate_relative_path(&file.destination)?;
            destinations
                .entry(file.destination.clone())
                .or_default()
                .push(manifest.id.clone());
            operations.push(PlanOperation {
                owner_id: manifest.id.clone(),
                source: file.source.clone(),
                destination: file.destination.clone(),
                size: file.size,
                expected_sha256: contract.sha256.clone(),
            });
        }
    }

    if operations.len() > MAX_OPERATION_FILES {
        return Err("catalog_invalid".to_string());
    }
    operations.sort_by(|left, right| {
        left.destination
            .cmp(&right.destination)
            .then(left.owner_id.cmp(&right.owner_id))
    });
    let conflicts: Vec<PlanConflict> = destinations
        .into_iter()
        .filter_map(|(destination, mut contenders)| {
            contenders.sort();
            (contenders.len() > 1).then_some(PlanConflict {
                destination,
                contenders,
            })
        })
        .collect();
    let space_estimate: u64 = operations.iter().map(|operation| operation.size).sum();
    if space_estimate > MAX_STAGED_BYTES {
        return Err("catalog_invalid".to_string());
    }
    let inputs = selected
        .iter()
        .map(|(manifest, contract)| PlanInput {
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            content_identity: format!("sha256:{}", contract.sha256),
        })
        .collect::<Vec<_>>();

    let mut plan_hasher = Sha256::new();
    for input in &inputs {
        plan_hasher.update(input.id.as_bytes());
        plan_hasher.update([0]);
        plan_hasher.update(input.version.as_bytes());
        plan_hasher.update([0]);
        plan_hasher.update(input.content_identity.as_bytes());
        plan_hasher.update([0]);
    }
    for operation in &operations {
        plan_hasher.update(operation.destination.as_bytes());
        plan_hasher.update([0]);
        plan_hasher.update(operation.expected_sha256.as_bytes());
        plan_hasher.update([0]);
    }

    Ok(BuildPlan {
        plan_id: format!("sha256:{:x}", plan_hasher.finalize()),
        dry_run: true,
        inputs,
        operations,
        executable: conflicts.is_empty() && !selected.is_empty(),
        conflicts,
        space_estimate,
    })
}

fn now_ms() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|_| "build_failed".to_string())
}

fn new_operation_id() -> Result<String, String> {
    let count = OPERATION_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(format!("op-{}-{}-{count}", now_ms()?, std::process::id()))
}

fn engine_paths(app_data_root: &Path) -> (PathBuf, PathBuf) {
    let root = app_data_root.join("engine-v1");
    (root.join("operations"), root.join("journals"))
}

fn reject_symlink(path: &Path) -> Result<(), String> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err("build_failed".to_string());
        }
    }
    Ok(())
}

fn prepare_owned_roots(app_data_root: &Path) -> Result<(PathBuf, PathBuf), String> {
    reject_symlink(app_data_root)?;
    fs::create_dir_all(app_data_root).map_err(|_| "build_failed".to_string())?;
    reject_symlink(app_data_root)?;
    let engine_root = app_data_root.join("engine-v1");
    reject_symlink(&engine_root)?;
    fs::create_dir(&engine_root)
        .or_else(|error| {
            (error.kind() == std::io::ErrorKind::AlreadyExists)
                .then_some(())
                .ok_or(error)
        })
        .map_err(|_| "build_failed".to_string())?;
    reject_symlink(&engine_root)?;
    let (operations, journals) = engine_paths(app_data_root);
    for path in [&operations, &journals] {
        reject_symlink(path)?;
        fs::create_dir_all(path).map_err(|_| "build_failed".to_string())?;
        reject_symlink(path)?;
    }
    Ok((operations, journals))
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
        reject_symlink(candidate)?;
    }
    if path.exists() {
        if backup.exists() {
            fs::remove_file(&backup).map_err(|_| "journal_recovery_required".to_string())?;
        }
        if temporary.exists() {
            fs::remove_file(&temporary).map_err(|_| "journal_recovery_required".to_string())?;
        }
        return Ok(());
    }
    if backup.exists() {
        fs::rename(&backup, path).map_err(|_| "journal_recovery_required".to_string())?;
        if temporary.exists() {
            fs::remove_file(&temporary).map_err(|_| "journal_recovery_required".to_string())?;
        }
        return Ok(());
    }
    if temporary.exists() {
        let bytes = fs::read(&temporary).map_err(|_| "journal_recovery_required".to_string())?;
        let candidate: BuildJournal =
            serde_json::from_slice(&bytes).map_err(|_| "journal_recovery_required".to_string())?;
        if candidate.schema_version != ENGINE_SCHEMA_VERSION {
            return Err("journal_recovery_required".to_string());
        }
        fs::rename(&temporary, path).map_err(|_| "journal_recovery_required".to_string())?;
    }
    Ok(())
}

fn atomic_write_json(path: &Path, value: &BuildJournal) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| "build_failed".to_string())?;
    reject_symlink(parent)?;
    recover_journal_files(path)?;
    let (temporary, backup) = journal_sidecars(path);
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| "build_failed".to_string())?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|_| "build_failed".to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "build_failed".to_string())?;

    if path.exists() {
        reject_symlink(path)?;
        fs::rename(path, &backup).map_err(|_| "build_failed".to_string())?;
    }
    if fs::rename(&temporary, path).is_err() {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        let _ = fs::remove_file(&temporary);
        return Err("build_failed".to_string());
    }
    if backup.exists() {
        fs::remove_file(backup).map_err(|_| "build_failed".to_string())?;
    }
    Ok(())
}

fn read_journal(path: &Path) -> Result<BuildJournal, String> {
    recover_journal_files(path)?;
    reject_symlink(path)?;
    let metadata = fs::metadata(path).map_err(|_| "operation_not_found".to_string())?;
    if metadata.len() > 1024 * 1024 {
        return Err("journal_invalid".to_string());
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(|_| "journal_invalid".to_string())?;
    let journal: BuildJournal =
        serde_json::from_slice(&bytes).map_err(|_| "journal_invalid".to_string())?;
    if journal.schema_version != ENGINE_SCHEMA_VERSION {
        return Err("journal_invalid".to_string());
    }
    Ok(journal)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FailurePoint {
    None,
    AfterFirstWrite,
    BeforeVerification,
}

pub fn execute_build(
    app_data_root: &Path,
    request: ExecuteBuildRequest,
) -> Result<BuildReceipt, String> {
    execute_build_with_failure(app_data_root, request, FailurePoint::None)
}

fn execute_build_with_failure(
    app_data_root: &Path,
    request: ExecuteBuildRequest,
    failure: FailurePoint,
) -> Result<BuildReceipt, String> {
    if !request.confirmed {
        return Err("build_confirmation_required".to_string());
    }
    let plan = create_build_plan(BuildPlanRequest {
        mod_ids: request.mod_ids,
    })?;
    if request.expected_plan_id != plan.plan_id {
        return Err("build_plan_stale".to_string());
    }
    if !plan.executable {
        return Err("conflict_unresolved".to_string());
    }
    content_store::intake_fixture_content(
        app_data_root,
        ContentIntakeRequest {
            package_ids: plan.inputs.iter().map(|input| input.id.clone()).collect(),
        },
    )?;
    let (operations_root, journals_root) = prepare_owned_roots(app_data_root)?;
    let operation_id = new_operation_id()?;
    let operation_root = operations_root.join(&operation_id);
    fs::create_dir(&operation_root).map_err(|_| "build_failed".to_string())?;
    reject_symlink(&operation_root)?;
    let staging_root = operation_root.join("staging");
    fs::create_dir(&staging_root).map_err(|_| "build_failed".to_string())?;
    let journal_path = journals_root.join(format!("{operation_id}.json"));
    let timestamp = now_ms()?;
    let mut journal = BuildJournal {
        schema_version: ENGINE_SCHEMA_VERSION,
        operation_id: operation_id.clone(),
        plan_id: plan.plan_id.clone(),
        phase: OperationPhase::Staging,
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
        staged_root: staging_root.to_string_lossy().into_owned(),
        files: plan
            .operations
            .iter()
            .map(|operation| JournalFile {
                owner_id: operation.owner_id.clone(),
                destination: operation.destination.clone(),
                expected_sha256: operation.expected_sha256.clone(),
                actual_sha256: None,
                size: operation.size,
                staged: false,
            })
            .collect(),
        error_code: None,
    };
    atomic_write_json(&journal_path, &journal)?;

    let result = (|| {
        for (index, operation) in plan.operations.iter().enumerate() {
            validate_relative_path(&operation.destination)?;
            let payload =
                content_store::read_verified_fixture_artifact(app_data_root, &operation.owner_id)?;
            if payload.len() as u64 != operation.size
                || sha256(&payload) != operation.expected_sha256
            {
                return Err("content_store_corrupt".to_string());
            }
            let target = staging_root.join(&operation.destination);
            let parent = target.parent().ok_or_else(|| "build_failed".to_string())?;
            fs::create_dir_all(parent).map_err(|_| "build_failed".to_string())?;
            reject_symlink(parent)?;
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&target)
                .map_err(|_| "build_failed".to_string())?;
            file.write_all(&payload)
                .and_then(|_| file.sync_all())
                .map_err(|_| "build_failed".to_string())?;
            journal.files[index].staged = true;
            journal.updated_at_ms = now_ms()?;
            atomic_write_json(&journal_path, &journal)?;
            if failure == FailurePoint::AfterFirstWrite && index == 0 {
                return Err("injected_failure".to_string());
            }
        }

        journal.phase = OperationPhase::Verifying;
        journal.updated_at_ms = now_ms()?;
        atomic_write_json(&journal_path, &journal)?;
        if failure == FailurePoint::BeforeVerification {
            return Err("injected_failure".to_string());
        }

        for file in &mut journal.files {
            let target = staging_root.join(&file.destination);
            reject_symlink(&target)?;
            let bytes = fs::read(target).map_err(|_| "verification_failed".to_string())?;
            let actual = sha256(&bytes);
            if bytes.len() as u64 != file.size || actual != file.expected_sha256 {
                return Err("verification_failed".to_string());
            }
            file.actual_sha256 = Some(actual);
        }
        journal.phase = OperationPhase::Ready;
        journal.updated_at_ms = now_ms()?;
        atomic_write_json(&journal_path, &journal)?;
        Ok(())
    })();

    if let Err(code) = result {
        journal.phase = OperationPhase::Failed;
        journal.error_code = Some(code.clone());
        journal.updated_at_ms = now_ms().unwrap_or(journal.updated_at_ms);
        let _ = atomic_write_json(&journal_path, &journal);
        return Err(code);
    }

    Ok(BuildReceipt {
        operation_id,
        plan_id: plan.plan_id,
        phase: OperationPhase::Ready,
        staged_root: staging_root.to_string_lossy().into_owned(),
        staged_files: plan.operations.len(),
        staged_bytes: plan.space_estimate,
        checksums_verified: true,
    })
}

fn validate_operation_id(operation_id: &str) -> Result<(), String> {
    if operation_id.len() < 4
        || operation_id.len() > 96
        || !operation_id.starts_with("op-")
        || !operation_id
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || value == b'-')
    {
        return Err("operation_not_found".to_string());
    }
    Ok(())
}

pub fn rollback_operation(
    app_data_root: &Path,
    operation_id: &str,
) -> Result<RollbackReceipt, String> {
    validate_operation_id(operation_id)?;
    let (operations_root, journals_root) = prepare_owned_roots(app_data_root)?;
    let journal_path = journals_root.join(format!("{operation_id}.json"));
    let mut journal = read_journal(&journal_path)?;
    if journal.operation_id != operation_id {
        return Err("journal_invalid".to_string());
    }
    let operation_root = operations_root.join(operation_id);
    let mut removed_staging = false;
    if operation_root.exists() {
        reject_symlink(&operation_root)?;
        let canonical_operations = operations_root
            .canonicalize()
            .map_err(|_| "rollback_failed".to_string())?;
        let canonical_operation = operation_root
            .canonicalize()
            .map_err(|_| "rollback_failed".to_string())?;
        if canonical_operation == canonical_operations
            || !canonical_operation.starts_with(&canonical_operations)
        {
            return Err("rollback_failed".to_string());
        }
        fs::remove_dir_all(&canonical_operation).map_err(|_| "rollback_failed".to_string())?;
        removed_staging = true;
    }
    journal.phase = OperationPhase::RolledBack;
    journal.updated_at_ms = now_ms()?;
    journal.error_code = None;
    atomic_write_json(&journal_path, &journal)?;
    Ok(RollbackReceipt {
        operation_id: operation_id.to_string(),
        phase: OperationPhase::RolledBack,
        removed_staging,
    })
}

pub fn list_operations(app_data_root: &Path) -> Result<Vec<OperationSummary>, String> {
    let (_, journals_root) = prepare_owned_roots(app_data_root)?;
    let recovery_entries = fs::read_dir(&journals_root)
        .map_err(|_| "journal_invalid".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "journal_invalid".to_string())?;
    for entry in recovery_entries {
        let path = entry.path();
        if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("tmp" | "bak")
        ) {
            recover_journal_files(&path.with_extension(""))?;
        }
    }
    let mut summaries = Vec::new();
    for entry in fs::read_dir(journals_root).map_err(|_| "journal_invalid".to_string())? {
        let entry = entry.map_err(|_| "journal_invalid".to_string())?;
        if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let journal = read_journal(&entry.path())?;
        summaries.push(OperationSummary {
            operation_id: journal.operation_id,
            plan_id: journal.plan_id,
            phase: journal.phase,
            created_at_ms: journal.created_at_ms,
            staged_files: journal.files.iter().filter(|file| file.staged).count(),
        });
    }
    summaries.sort_by_key(|summary| std::cmp::Reverse(summary.created_at_ms));
    Ok(summaries)
}

pub fn operation_diagnostic_counts(
    app_data_root: &Path,
) -> Result<OperationDiagnosticCounts, String> {
    let operations = list_operations(app_data_root)?;
    Ok(OperationDiagnosticCounts {
        total: operations.len(),
        recoverable: operations
            .iter()
            .filter(|operation| {
                matches!(
                    operation.phase,
                    OperationPhase::Staging | OperationPhase::Verifying | OperationPhase::Failed
                )
            })
            .count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "betterfy-engine-{name}-{}",
            now_ms().expect("time")
        ))
    }

    fn one_mod() -> BuildPlanRequest {
        BuildPlanRequest {
            mod_ids: vec!["fixture.ambient-violet".to_string()],
        }
    }

    fn confirmed_build() -> ExecuteBuildRequest {
        let request = one_mod();
        let plan = create_build_plan(request.clone()).expect("plan");
        ExecuteBuildRequest {
            mod_ids: request.mod_ids,
            expected_plan_id: plan.plan_id,
            confirmed: true,
        }
    }

    #[test]
    fn plan_is_deterministic_and_hashes_payloads() {
        let first = create_build_plan(one_mod()).expect("first plan");
        let second = create_build_plan(one_mod()).expect("second plan");
        assert_eq!(first.plan_id, second.plan_id);
        assert_eq!(first.operations.len(), 1);
        assert_eq!(first.operations[0].expected_sha256.len(), 64);
        assert_eq!(
            first.inputs[0].content_identity,
            format!("sha256:{}", first.operations[0].expected_sha256)
        );
        assert!(first.executable);
    }

    #[test]
    fn recipe_must_match_the_trusted_package_contract() {
        let manifest = fixture_manifests().expect("fixtures").remove(0);
        let contract = content_store::fixture_package_contract(&manifest.id).expect("contract");
        assert!(validate_recipe_contract(&manifest, &contract).is_ok());

        let mut wrong_version = contract.clone();
        wrong_version.version = "9.9.9".to_string();
        assert_eq!(
            validate_recipe_contract(&manifest, &wrong_version)
                .err()
                .as_deref(),
            Some("catalog_invalid")
        );

        let mut wrong_hash = contract;
        wrong_hash.sha256 = "0".repeat(64);
        assert_eq!(
            validate_recipe_contract(&manifest, &wrong_hash)
                .err()
                .as_deref(),
            Some("catalog_invalid")
        );
    }

    #[test]
    fn plan_reports_conflicts() {
        let plan = create_build_plan(BuildPlanRequest {
            mod_ids: vec![
                "fixture.ambient-violet".to_string(),
                "fixture.ambient-clean".to_string(),
            ],
        })
        .expect("plan");
        assert_eq!(plan.conflicts.len(), 1);
        assert!(!plan.executable);
    }

    #[test]
    fn plan_rejects_unknown_ids_and_deduplicates_known_ids() {
        assert_eq!(
            create_build_plan(BuildPlanRequest {
                mod_ids: vec!["fixture.unknown".to_string()],
            })
            .err()
            .as_deref(),
            Some("catalog_invalid")
        );
        let plan = create_build_plan(BuildPlanRequest {
            mod_ids: vec![
                "fixture.ambient-clean".to_string(),
                "fixture.ambient-clean".to_string(),
            ],
        })
        .expect("deduplicated plan");
        assert_eq!(plan.inputs.len(), 1);
        assert_eq!(plan.operations.len(), 1);
    }

    #[test]
    fn stages_verifies_journals_and_rolls_back_idempotently() {
        let root = temp_root("success");
        let receipt = execute_build(&root, confirmed_build()).expect("execute");
        assert!(receipt.checksums_verified);
        assert!(Path::new(&receipt.staged_root).is_dir());
        let operations = list_operations(&root).expect("list");
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].phase, OperationPhase::Ready);

        let first = rollback_operation(&root, &receipt.operation_id).expect("rollback");
        assert!(first.removed_staging);
        let second = rollback_operation(&root, &receipt.operation_id).expect("repeat rollback");
        assert!(!second.removed_staging);
        assert_eq!(second.phase, OperationPhase::RolledBack);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn failure_is_persisted_and_recoverable() {
        let root = temp_root("failure");
        assert_eq!(
            execute_build_with_failure(&root, confirmed_build(), FailurePoint::AfterFirstWrite)
                .err()
                .as_deref(),
            Some("injected_failure")
        );
        let operations = list_operations(&root).expect("list");
        assert_eq!(operations[0].phase, OperationPhase::Failed);
        let rollback =
            rollback_operation(&root, &operations[0].operation_id).expect("rollback failed op");
        assert!(rollback.removed_staging);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn failure_before_verification_keeps_a_recoverable_journal() {
        let root = temp_root("pre-verification-failure");
        assert_eq!(
            execute_build_with_failure(&root, confirmed_build(), FailurePoint::BeforeVerification,)
                .err()
                .as_deref(),
            Some("injected_failure")
        );
        let operations = list_operations(&root).expect("list");
        assert_eq!(operations[0].phase, OperationPhase::Failed);
        rollback_operation(&root, &operations[0].operation_id).expect("rollback");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn build_refuses_a_tampered_content_object() {
        let root = temp_root("tampered-content");
        content_store::intake_fixture_content(
            &root,
            ContentIntakeRequest {
                package_ids: vec!["fixture.ambient-violet".to_string()],
            },
        )
        .expect("content intake");
        let object = root
            .join("content-v1")
            .join("objects")
            .join("sha256")
            .join("e2c06353f3a5c99162512e40c6a2e318778d1c73bc0ba79126df5a4beff13c64");
        fs::write(object, b"tampered").expect("tamper content object");
        assert_eq!(
            execute_build(&root, confirmed_build()).err().as_deref(),
            Some("content_store_corrupt")
        );
        assert!(list_operations(&root).expect("operations").is_empty());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn recovers_an_interrupted_atomic_journal_commit() {
        let root = temp_root("journal-recovery");
        let receipt = execute_build(&root, confirmed_build()).expect("execute");
        let (_, journals) = engine_paths(&root);
        let journal = journals.join(format!("{}.json", receipt.operation_id));
        let temporary = journal.with_extension("json.tmp");
        fs::rename(&journal, &temporary).expect("simulate interrupted rename");

        let operations = list_operations(&root).expect("recover and list");
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].phase, OperationPhase::Ready);
        assert!(journal.is_file());
        assert!(!temporary.exists());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn rejects_traversal_and_invalid_operation_ids() {
        for path in ["../escape", "/absolute", "C:/absolute", "folder\\escape"] {
            assert_eq!(
                validate_relative_path(path).err().as_deref(),
                Some("catalog_invalid")
            );
        }
        let root = temp_root("invalid-operation");
        assert_eq!(
            rollback_operation(&root, "../escape").err().as_deref(),
            Some("operation_not_found")
        );
    }

    #[test]
    fn execution_requires_confirmation_and_the_exact_reviewed_plan() {
        let root = temp_root("plan-confirmation");
        let mut request = confirmed_build();
        request.confirmed = false;
        assert_eq!(
            execute_build(&root, request).err().as_deref(),
            Some("build_confirmation_required")
        );

        let mut request = confirmed_build();
        request.expected_plan_id = format!("{}-changed", request.expected_plan_id);
        assert_eq!(
            execute_build(&root, request).err().as_deref(),
            Some("build_plan_stale")
        );
        assert!(list_operations(&root).expect("operations").is_empty());
        if root.exists() {
            fs::remove_dir_all(root).expect("cleanup");
        }
    }

    #[cfg(unix)]
    #[test]
    fn execution_rejects_a_symlinked_engine_root_component() {
        use std::os::unix::fs::symlink;

        let root = temp_root("symlink-engine-root");
        let outside = temp_root("symlink-engine-outside");
        fs::create_dir_all(&root).expect("root");
        fs::create_dir_all(&outside).expect("outside");
        symlink(&outside, root.join("engine-v1")).expect("symlink");
        assert_eq!(
            execute_build(&root, confirmed_build()).err().as_deref(),
            Some("build_failed")
        );
        assert!(fs::read_dir(&outside)
            .expect("outside contents")
            .next()
            .is_none());
        fs::remove_file(root.join("engine-v1")).expect("remove symlink");
        fs::remove_dir_all(root).expect("cleanup root");
        fs::remove_dir_all(outside).expect("cleanup outside");
    }
}

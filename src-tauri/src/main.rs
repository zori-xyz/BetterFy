#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod archive_inspector;
mod auth_session;
mod blacklist;
mod build_engine;
mod catalog_index;
mod content_store;
mod evidence_reports;
mod game_deployment;
mod game_language;
mod installed_profile;
mod mod_bundle;
mod package_registry;
mod panorama;
mod presets;
mod remote_intake;
mod runtime_control;
mod steam_accounts;
pub mod steam_config;
mod system_diagnostics;
mod tree_pilot;
mod vpk;

use auth_session::{
    auth_begin_device_challenge, auth_begin_email, auth_cancel_device_challenge,
    auth_email_identity, auth_fetch_avatar, auth_id_login, auth_id_register_start,
    auth_id_register_verify, auth_link_email_start, auth_link_email_verify, auth_list_sessions,
    auth_logout, auth_poll_device_challenge, auth_profile, auth_restore_session,
    auth_revoke_device, auth_verify_code, auth_verify_email, AuthState,
};
use build_engine::{
    create_build_plan, execute_build as execute_staged_build,
    list_operations as list_staged_operations, operation_diagnostic_counts, rollback_operation,
    BuildPlan, BuildPlanRequest, BuildReceipt, ExecuteBuildRequest, OperationSummary,
    RollbackReceipt,
};
use content_store::{ContentIntakeRequest, ContentReceipt};
use game_deployment::{
    DeployStagedVpkRequest, DeploymentEvidenceReport, DeploymentOperationRequest,
    DeploymentReceipt, DeploymentRecoveryRequest, DeploymentStressCapabilities,
    DeploymentStressFailurePoint, RecoveryReceipt,
};
use game_language::GameLanguage;
use presets::{delete_preset, export_preset, import_preset, list_presets, save_preset};
use remote_intake::ContentDownloadStatus;
use runtime_control::{RuntimePrepareRequest, RuntimeState, SteamStartRequest};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use steam_accounts::{
    ApplySteamLaunchOptionRequest, SteamConfigOperationRequest, SteamConfigReceipt,
    SteamLaunchOptionPreview, SteamProfileSummary,
};
use system_diagnostics::SystemDiagnosticReport;
use tauri::{AppHandle, Manager};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StartSteamAfterProfileRequest {
    profile_token: String,
    operation_id: Option<String>,
    #[serde(default)]
    language: GameLanguage,
    confirmed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstallTreePilotRequest {
    game_path: String,
    package_ids: Vec<String>,
    expected_plan_id: String,
    language: GameLanguage,
    confirmed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstallTreePilotStressRequest {
    game_path: String,
    package_ids: Vec<String>,
    expected_plan_id: String,
    language: GameLanguage,
    failure_point: DeploymentStressFailurePoint,
    confirmed: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TreeCurrentState {
    #[serde(flatten)]
    receipt: DeploymentReceipt,
    package_verified: bool,
    steam_operation_id: Option<String>,
    steam_profile_token: Option<String>,
    steam_recovery_required: bool,
    /// Derived record of the install; absent when missing or stale.
    profile: Option<installed_profile::InstalledProfile>,
    /// Steam's Dota build differs from the one recorded at install time.
    dota_patched: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TreePilotEvidenceReport {
    #[serde(flatten)]
    deployment: DeploymentEvidenceReport,
    steam_entries: Vec<steam_accounts::SteamEvidenceEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplyTreeSteamRequest {
    game_path: String,
    deployment_operation_id: String,
    profile_token: String,
    confirmation_token: String,
    language: GameLanguage,
    confirmed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplyTreeSteamStressRequest {
    game_path: String,
    deployment_operation_id: String,
    profile_token: String,
    confirmation_token: String,
    language: GameLanguage,
    failure_point: steam_accounts::SteamStressFailurePoint,
    confirmed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StartSteamAfterTreePilotRequest {
    game_path: String,
    operation_id: String,
    profile_token: Option<String>,
    steam_operation_id: Option<String>,
    confirmed: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GameInstallation {
    path: String,
    executable_path: String,
    steam_library: String,
    client: String,
    source: &'static str,
    verified: bool,
}

/// `canonicalize` on Windows returns extended-length paths (`\\?\C:\...`).
/// They are valid for the OS but confusing in the interface, and some tools
/// reject them. Short drive and UNC paths are shown in their ordinary form.
/// The deployment code canonicalizes again, so its identities are unaffected.
fn simplified_path(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if text.len() >= 260 {
        return path;
    }
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = text.strip_prefix(r"\\?\") {
        let bytes = rest.as_bytes();
        if bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && bytes[2] == b'\\'
        {
            return PathBuf::from(rest);
        }
    }
    path
}

fn validate_candidate(path: &Path, source: &'static str) -> Result<GameInstallation, String> {
    let canonical = simplified_path(
        path.canonicalize()
            .map_err(|_| "game_not_found".to_string())?,
    );
    if !canonical.is_dir() {
        return Err("invalid_game_path".to_string());
    }

    let folder_name = canonical
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if folder_name != "dota 2 beta" {
        return Err("invalid_game_path".to_string());
    }

    let pak = canonical.join("game").join("dota").join("pak01_dir.vpk");
    let executable = if cfg!(target_os = "windows") {
        canonical
            .join("game")
            .join("bin")
            .join("win64")
            .join("dota2.exe")
    } else {
        canonical
            .join("game")
            .join("bin")
            .join("osx64")
            .join("dota2")
    };
    if !pak.is_file() || !executable.is_file() {
        return Err("invalid_game_path".to_string());
    }

    let steam_library = canonical
        .ancestors()
        .nth(4)
        .unwrap_or(&canonical)
        .to_string_lossy()
        .into_owned();

    Ok(GameInstallation {
        path: canonical.to_string_lossy().into_owned(),
        executable_path: executable.to_string_lossy().into_owned(),
        steam_library,
        client: "Steam".to_string(),
        source,
        verified: true,
    })
}

/// Steam's build number for the installed Dota 2, from the library's
/// `appmanifest_570.acf`. Read-only; used to notice that the game was patched
/// after a build was installed, because a patch can break an older HUD mod.
fn dota_build_id(game_root: &Path) -> Option<String> {
    let manifest = game_root.parent()?.parent()?.join("appmanifest_570.acf");
    if fs::metadata(&manifest).ok()?.len() > 256 * 1024 {
        return None;
    }
    parse_build_id(&fs::read_to_string(manifest).ok()?)
}

fn parse_build_id(contents: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let quoted: Vec<&str> = line.split('"').skip(1).step_by(2).collect();
        match quoted.as_slice() {
            ["buildid", value]
                if !value.is_empty()
                    && value.len() <= 12
                    && value.bytes().all(|byte| byte.is_ascii_digit()) =>
            {
                Some((*value).to_string())
            }
            _ => None,
        }
    })
}

/// Loads the cached signed catalog and checks the remote one. Network and
/// signature failures are reported but never remove a package.
#[tauri::command]
async fn refresh_catalog(app: AppHandle) -> Result<catalog_index::CatalogStatus, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "catalog_cache_invalid".to_string())?;
    tauri::async_runtime::spawn_blocking(move || catalog_index::refresh(&app_data))
        .await
        .map_err(|_| "runtime_worker_failed".to_string())
}

/// Packages that can be installed now: the embedded manifests, merged with a
/// verified signed catalog once `refresh_catalog` has accepted one.
#[tauri::command]
fn list_installable_packages() -> Result<Vec<package_registry::PackageSummary>, String> {
    package_registry::summaries()
}

/// Best-effort Dota build lookup for the installed profile.
fn current_dota_build(game_path: &str) -> Option<String> {
    let installation = validate_candidate(Path::new(game_path), "manual").ok()?;
    dota_build_id(Path::new(&installation.path))
}

/// Attach a committed Tree pilot Steam operation to the installed profile.
/// Never changes the Steam result.
fn record_tree_steam_operation(
    app_data: &Path,
    deployment_id: Option<&str>,
    result: &Result<SteamConfigReceipt, String>,
) {
    if let (Some(deployment_id), Ok(receipt)) = (deployment_id, result) {
        if let Some(steam_operation_id) = receipt.committed_operation_id() {
            installed_profile::record_steam_operation(app_data, deployment_id, steam_operation_id);
        }
    }
}

fn discovery_candidates() -> Vec<PathBuf> {
    let mut steam_roots = Vec::new();

    #[cfg(target_os = "windows")]
    {
        steam_roots.extend(windows_registry_steam_roots());
        for key in ["PROGRAMFILES(X86)", "PROGRAMFILES"] {
            if let Some(base) = std::env::var_os(key) {
                steam_roots.push(PathBuf::from(base).join("Steam"));
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            steam_roots.push(
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("Steam"),
            );
        }
    }

    let mut libraries = steam_roots.clone();
    for root in &steam_roots {
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(contents) = fs::read_to_string(vdf) {
            libraries.extend(parse_library_paths(&contents));
        }
    }

    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    for library in libraries {
        let candidate = library.join("steamapps").join("common").join("dota 2 beta");
        if seen.insert(candidate.clone()) {
            candidates.push(candidate);
        }
    }
    candidates
}

#[cfg(target_os = "windows")]
fn windows_registry_steam_roots() -> Vec<PathBuf> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY};
    use winreg::RegKey;

    let mut roots = Vec::new();
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
    for (hive, key_path, value_name, flags) in candidates {
        let hive = RegKey::predef(hive);
        if let Ok(key) = hive.open_subkey_with_flags(key_path, flags) {
            if let Ok(value) = key.get_value::<String, _>(value_name) {
                let value = value.trim();
                if !value.is_empty() {
                    roots.push(PathBuf::from(value));
                }
            }
        }
    }
    roots
}

fn parse_library_paths(contents: &str) -> Vec<PathBuf> {
    contents
        .lines()
        .filter_map(|line| {
            let quoted: Vec<&str> = line.split('"').skip(1).step_by(2).collect();
            if quoted.first().copied() != Some("path") {
                return None;
            }
            quoted
                .get(1)
                .map(|value| PathBuf::from(value.replace("\\\\", "\\")))
        })
        .collect()
}

// Discovery and diagnostics read the registry, Steam library files and the
// process list. Synchronous Tauri commands run on the main thread, so these
// run on a blocking worker to keep the window responsive.
#[tauri::command]
async fn discover_game() -> Result<Vec<GameInstallation>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        discovery_candidates()
            .iter()
            .filter_map(|path| validate_candidate(path, "auto").ok())
            .collect()
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())
}

#[tauri::command(async)]
fn validate_game_path(path: String) -> Result<GameInstallation, String> {
    validate_candidate(Path::new(&path), "manual")
}

#[tauri::command]
async fn collect_system_diagnostics(
    app: AppHandle,
    game_path: Option<String>,
) -> Result<SystemDiagnosticReport, String> {
    tauri::async_runtime::spawn_blocking(move || system_diagnostics_report(&app, game_path))
        .await
        .map_err(|_| "runtime_worker_failed".to_string())
}

fn system_diagnostics_report(app: &AppHandle, game_path: Option<String>) -> SystemDiagnosticReport {
    let stored_game_verified = game_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .map(|path| validate_candidate(Path::new(path), "manual").is_ok())
        .unwrap_or(false);
    let game_verified = stored_game_verified
        || discovery_candidates()
            .iter()
            .any(|path| validate_candidate(path, "auto").is_ok());
    let app_data = app.path().app_data_dir();
    let staging = app_data
        .as_deref()
        .map(|root| {
            let mut counts = operation_diagnostic_counts(root)?;
            let deployments = game_deployment::diagnostic_counts(root)?;
            counts.total = counts.total.saturating_add(deployments.total);
            counts.recoverable = counts.recoverable.saturating_add(deployments.recoverable);
            Ok(counts)
        })
        .unwrap_or_else(|_| Err("diagnostics_unavailable".to_string()));
    let content = app_data
        .as_deref()
        .map(content_store::content_diagnostic_counts)
        .unwrap_or_else(|_| Err("diagnostics_unavailable".to_string()));

    system_diagnostics::assemble_report(
        cfg!(target_os = "windows"),
        game_verified,
        runtime_control::inspect_runtime(),
        steam_accounts::platform_profile_diagnostic_counts(),
        staging,
        content,
    )
}

#[tauri::command]
fn intake_fixture_content(
    app: AppHandle,
    request: ContentIntakeRequest,
) -> Result<Vec<ContentReceipt>, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "content_store_unavailable".to_string())?;
    content_store::intake_fixture_content(&app_data, request)
}

#[tauri::command]
fn begin_content_download(
    app: AppHandle,
    package_id: String,
) -> Result<ContentDownloadStatus, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "content_store_unavailable".to_string())?;
    remote_intake::begin(app_data, package_id)
}

#[tauri::command]
fn content_download_status(operation_id: String) -> Result<ContentDownloadStatus, String> {
    remote_intake::status(&operation_id)
}

#[tauri::command]
fn cancel_content_download(operation_id: String) -> Result<ContentDownloadStatus, String> {
    remote_intake::cancel(&operation_id)
}

#[tauri::command]
fn plan_build(request: BuildPlanRequest) -> Result<BuildPlan, String> {
    create_build_plan(request)
}

#[tauri::command(async)]
fn execute_build(app: AppHandle, request: ExecuteBuildRequest) -> Result<BuildReceipt, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "build_failed".to_string())?;
    execute_staged_build(&app_data, request)
}

#[tauri::command(async)]
fn inspect_runtime() -> Result<RuntimeState, String> {
    runtime_control::inspect_runtime()
}

#[tauri::command]
async fn prepare_runtime_for_patch(request: RuntimePrepareRequest) -> Result<RuntimeState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        runtime_control::prepare_runtime_for_patch(request)
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command(async)]
fn list_engine_operations(app: AppHandle) -> Result<Vec<OperationSummary>, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "journal_invalid".to_string())?;
    list_staged_operations(&app_data)
}

#[tauri::command(async)]
fn rollback_engine_operation(
    app: AppHandle,
    operation_id: String,
) -> Result<RollbackReceipt, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "rollback_failed".to_string())?;
    rollback_operation(&app_data, &operation_id)
}

#[tauri::command]
async fn list_steam_profiles() -> Result<Vec<SteamProfileSummary>, String> {
    tauri::async_runtime::spawn_blocking(steam_accounts::list_platform_profiles)
        .await
        .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
async fn preview_steam_launch_options(
    profile_token: String,
    language: Option<GameLanguage>,
) -> Result<SteamLaunchOptionPreview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        steam_accounts::preview_platform_profile(&profile_token, language.unwrap_or_default())
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

fn require_patch_ready_runtime() -> Result<(), String> {
    if !cfg!(target_os = "windows") {
        return Err("platform_not_supported".to_string());
    }
    if !runtime_control::inspect_runtime()?.patch_ready {
        return Err("runtime_busy".to_string());
    }
    Ok(())
}

#[tauri::command]
async fn apply_steam_launch_options(
    app: AppHandle,
    request: ApplySteamLaunchOptionRequest,
) -> Result<SteamConfigReceipt, String> {
    if request.linked_deployment_id.is_some() {
        return Err("steam_activation_not_ready".to_string());
    }
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "backup_failed".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        require_patch_ready_runtime()?;
        steam_accounts::apply_platform_profile(&app_data, request)
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
async fn rollback_steam_launch_options(
    app: AppHandle,
    request: SteamConfigOperationRequest,
) -> Result<SteamConfigReceipt, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "rollback_failed".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        require_patch_ready_runtime()?;
        let result = steam_accounts::rollback_platform_operation(&app_data, request);
        if let Some(operation_id) = result
            .as_ref()
            .ok()
            .and_then(|receipt| receipt.rolled_back_operation_id())
        {
            installed_profile::forget_steam_operation(&app_data, operation_id);
        }
        result
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
async fn recover_steam_launch_options(
    app: AppHandle,
    confirmed: bool,
) -> Result<Vec<SteamConfigReceipt>, String> {
    if !confirmed {
        return Err("steam_config_confirmation_required".to_string());
    }
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "rollback_failed".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        require_patch_ready_runtime()?;
        steam_accounts::recover_platform_operations(&app_data)
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
async fn deploy_staged_vpk(
    app: AppHandle,
    request: DeployStagedVpkRequest,
) -> Result<DeploymentReceipt, String> {
    if !cfg!(debug_assertions) {
        return Err("deployment_not_enabled".to_string());
    }
    if !request.confirmed {
        return Err("deployment_confirmation_required".to_string());
    }
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "deployment_failed".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        require_patch_ready_runtime()?;
        let staged = build_engine::verified_staged_vpk(
            &app_data,
            &request.staged_operation_id,
            &request.expected_plan_id,
        )?;
        game_deployment::deploy_verified_vpk(
            &app_data,
            Path::new(&request.game_path),
            &staged.bytes,
            &staged.sha256,
        )
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
fn begin_tree_pilot_download(
    app: AppHandle,
    package_ids: Vec<String>,
    game_path: Option<String>,
) -> Result<tree_pilot::TreeDownloadStatus, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "build_failed".to_string())?;
    // Blacklists are expanded against this installation's own archives.
    let game_path = match game_path {
        Some(path) => {
            validate_candidate(Path::new(&path), "manual")?;
            Some(std::path::PathBuf::from(path))
        }
        None => None,
    };
    tree_pilot::begin_download(app_data, package_ids, game_path)
}

#[tauri::command]
fn tree_pilot_download_status() -> Result<Option<tree_pilot::TreeDownloadStatus>, String> {
    tree_pilot::download_status()
}

#[tauri::command]
fn cancel_tree_pilot_download() -> Result<tree_pilot::TreeDownloadStatus, String> {
    tree_pilot::cancel_download()
}

fn build_verified_tree_pilot(
    app_data: &Path,
    request: &InstallTreePilotRequest,
) -> Result<tree_pilot::VerifiedTreeVpk, String> {
    require_patch_ready_runtime()?;
    validate_candidate(Path::new(&request.game_path), "manual")?;
    let verified = tree_pilot::build_from_verified_store(app_data, &request.package_ids)?;
    if request.expected_plan_id != verified.plan_id() {
        return Err("build_plan_stale".to_string());
    }
    let staged = tree_pilot::stage_from_verified_store(
        app_data,
        &request.expected_plan_id,
        &request.package_ids,
        true,
    )?;
    let reopened = build_engine::verified_staged_vpk(
        app_data,
        &staged.operation_id,
        &request.expected_plan_id,
    )?;
    if reopened.sha256 != verified.sha256() || reopened.bytes != verified.bytes() {
        return Err("verification_failed".to_string());
    }
    require_patch_ready_runtime()?;
    Ok(verified)
}

#[tauri::command]
async fn install_tree_pilot(
    app: AppHandle,
    request: InstallTreePilotRequest,
) -> Result<DeploymentReceipt, String> {
    if !request.confirmed {
        return Err("deployment_confirmation_required".to_string());
    }
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "deployment_failed".to_string())?;
    let app_version = app.package_info().version.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let verified = build_verified_tree_pilot(&app_data, &request)?;
        let result = game_deployment::deploy_verified_bundle_for_language(
            &app_data,
            Path::new(&request.game_path),
            verified.bytes(),
            verified.sha256(),
            request.language,
            game_deployment::DeploymentBundleIdentity {
                plan_id: Some(verified.plan_id().to_string()),
                package_ids: verified.bundle_plan().package_ids.clone(),
            },
        );
        installed_profile::record_install(
            &app_data,
            result,
            || current_dota_build(&request.game_path),
            &app_version,
            std::time::SystemTime::now(),
        )
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
fn tree_pilot_stress_capabilities() -> DeploymentStressCapabilities {
    game_deployment::deployment_stress_capabilities()
}

#[tauri::command]
async fn install_tree_pilot_stress(
    app: AppHandle,
    request: InstallTreePilotStressRequest,
) -> Result<DeploymentReceipt, String> {
    if !request.confirmed {
        return Err("deployment_confirmation_required".to_string());
    }
    if !game_deployment::deployment_stress_capabilities().enabled {
        return Err("stress_test_disabled".to_string());
    }
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "deployment_failed".to_string())?;
    let app_version = app.package_info().version.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let install = InstallTreePilotRequest {
            game_path: request.game_path,
            package_ids: request.package_ids,
            expected_plan_id: request.expected_plan_id,
            language: request.language,
            confirmed: request.confirmed,
        };
        let verified = build_verified_tree_pilot(&app_data, &install)?;
        let result = game_deployment::deploy_verified_bundle_for_language_stress(
            &app_data,
            Path::new(&install.game_path),
            verified.bytes(),
            verified.sha256(),
            install.language,
            game_deployment::DeploymentBundleIdentity {
                plan_id: Some(verified.plan_id().to_string()),
                package_ids: verified.bundle_plan().package_ids.clone(),
            },
            request.failure_point,
        );
        installed_profile::record_install(
            &app_data,
            result,
            || current_dota_build(&install.game_path),
            &app_version,
            std::time::SystemTime::now(),
        )
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
async fn collect_tree_pilot_evidence(
    app: AppHandle,
    game_path: String,
) -> Result<TreePilotEvidenceReport, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "deployment_failed".to_string())?;
    let app_version = app.package_info().version.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        tree_pilot_evidence_report(&app_data, &game_path, app_version)
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

fn tree_pilot_evidence_report(
    app_data: &Path,
    game_path: &str,
    app_version: String,
) -> Result<TreePilotEvidenceReport, String> {
    validate_candidate(Path::new(game_path), "manual")?;
    Ok(TreePilotEvidenceReport {
        deployment: game_deployment::collect_evidence(app_data, Path::new(game_path), app_version)?,
        steam_entries: steam_accounts::collect_platform_evidence(app_data)?,
    })
}

fn save_tree_pilot_evidence_report(
    app_data: &Path,
    game_path: &str,
    app_version: String,
) -> Result<evidence_reports::SavedEvidence, String> {
    let report = tree_pilot_evidence_report(app_data, game_path, app_version)?;
    let entries = report.deployment.entries.len() + report.steam_entries.len();
    evidence_reports::save_report(app_data, &report, entries, std::time::SystemTime::now())
}

#[tauri::command]
async fn save_tree_pilot_evidence(
    app: AppHandle,
    game_path: String,
) -> Result<evidence_reports::SavedEvidence, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "report_write_failed".to_string())?;
    let app_version = app.package_info().version.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        save_tree_pilot_evidence_report(&app_data, &game_path, app_version)
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
async fn open_reports_folder(app: AppHandle) -> Result<(), String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "report_write_failed".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let dir = evidence_reports::reports_dir(&app_data)?;
        tauri_plugin_opener::open_path(&dir, None::<&str>)
            .map_err(|_| "reports_folder_open_failed".to_string())
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
async fn current_tree_pilot(
    app: AppHandle,
    game_path: String,
) -> Result<Option<TreeCurrentState>, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "deployment_failed".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        if !cfg!(target_os = "windows") {
            return Err("platform_not_supported".to_string());
        }
        let installation = validate_candidate(Path::new(&game_path), "manual")?;
        let Some(receipt) =
            game_deployment::current_owned_deployment(&app_data, Path::new(&game_path))?
        else {
            return Ok(None);
        };
        let profile = installed_profile::current_for(&app_data, &receipt);
        let current_build = profile
            .as_ref()
            .and_then(|profile| profile.dota_build_at_install.as_ref())
            .and_then(|_| dota_build_id(Path::new(&installation.path)));
        let dota_patched =
            installed_profile::dota_patched(profile.as_ref(), current_build.as_deref());
        let package_ids = if receipt.package_ids.is_empty() {
            vec![tree_pilot::PACKAGE_ID.to_string()]
        } else {
            receipt.package_ids.clone()
        };
        let package_verified = tree_pilot::build_from_verified_store(&app_data, &package_ids)
            .map(|verified| {
                format!("{:x}", sha2::Sha256::digest(verified.bytes())) == receipt.installed_sha256
                    && receipt
                        .bundle_plan_id
                        .as_deref()
                        .is_none_or(|plan| plan == verified.plan_id())
            })
            .unwrap_or(false);
        let (linked, steam_recovery_required) =
            match steam_accounts::linked_platform_operation(&app_data, &receipt.operation_id) {
                Ok(linked) => (linked, false),
                Err(code) if code == "steam_recovery_required" => (None, true),
                Err(code) => return Err(code),
            };
        if linked
            .as_ref()
            .is_some_and(|operation| operation.language != receipt.language)
        {
            return Err("steam_journal_invalid".to_string());
        }
        Ok(Some(TreeCurrentState {
            receipt,
            package_verified,
            steam_operation_id: linked
                .as_ref()
                .map(|operation| operation.operation_id.clone()),
            steam_profile_token: linked.map(|operation| operation.profile_token),
            steam_recovery_required,
            profile,
            dota_patched,
        }))
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
async fn apply_tree_steam_launch_options(
    app: AppHandle,
    request: ApplyTreeSteamRequest,
) -> Result<SteamConfigReceipt, String> {
    if !request.confirmed {
        return Err("steam_config_confirmation_required".to_string());
    }
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "backup_failed".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let profile_request = validate_tree_steam_request(&app_data, request)?;
        let deployment_id = profile_request.linked_deployment_id.clone();
        let result = steam_accounts::apply_platform_profile(&app_data, profile_request);
        record_tree_steam_operation(&app_data, deployment_id.as_deref(), &result);
        result
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

fn validate_tree_steam_request(
    app_data: &Path,
    request: ApplyTreeSteamRequest,
) -> Result<ApplySteamLaunchOptionRequest, String> {
    require_patch_ready_runtime()?;
    validate_candidate(Path::new(&request.game_path), "manual")?;
    let receipt =
        game_deployment::current_owned_deployment(app_data, Path::new(&request.game_path))?
            .ok_or_else(|| "deployment_not_found".to_string())?;
    let package_ids = if receipt.package_ids.is_empty() {
        vec![tree_pilot::PACKAGE_ID.to_string()]
    } else {
        receipt.package_ids.clone()
    };
    let verified = tree_pilot::build_from_verified_store(app_data, &package_ids)?;
    if receipt.installed_sha256 != format!("{:x}", sha2::Sha256::digest(verified.bytes()))
        || receipt
            .bundle_plan_id
            .as_deref()
            .is_some_and(|plan| plan != verified.plan_id())
        || receipt.operation_id != request.deployment_operation_id
        || receipt.language != request.language
    {
        return Err("deployment_conflict".to_string());
    }
    Ok(ApplySteamLaunchOptionRequest {
        profile_token: request.profile_token,
        confirmation_token: request.confirmation_token,
        language: request.language,
        confirmed: true,
        linked_deployment_id: Some(receipt.operation_id),
    })
}

#[tauri::command]
async fn apply_tree_steam_launch_options_stress(
    app: AppHandle,
    request: ApplyTreeSteamStressRequest,
) -> Result<SteamConfigReceipt, String> {
    if !request.confirmed {
        return Err("steam_config_confirmation_required".to_string());
    }
    if !game_deployment::deployment_stress_capabilities().enabled {
        return Err("stress_test_disabled".to_string());
    }
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "backup_failed".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let failure_point = request.failure_point;
        let profile_request = validate_tree_steam_request(
            &app_data,
            ApplyTreeSteamRequest {
                game_path: request.game_path,
                deployment_operation_id: request.deployment_operation_id,
                profile_token: request.profile_token,
                confirmation_token: request.confirmation_token,
                language: request.language,
                confirmed: request.confirmed,
            },
        )?;
        let deployment_id = profile_request.linked_deployment_id.clone();
        let result = steam_accounts::apply_platform_profile_stress(
            &app_data,
            profile_request,
            failure_point,
        );
        record_tree_steam_operation(&app_data, deployment_id.as_deref(), &result);
        result
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
async fn preview_tree_language(
    app: AppHandle,
    game_path: String,
    language: GameLanguage,
) -> Result<(), String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "deployment_failed".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        if !cfg!(target_os = "windows") {
            return Err("platform_not_supported".to_string());
        }
        validate_candidate(Path::new(&game_path), "manual")?;
        game_deployment::preview_language_destination(&app_data, Path::new(&game_path), language)
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
async fn start_steam_after_tree_pilot(
    app: AppHandle,
    request: StartSteamAfterTreePilotRequest,
) -> Result<RuntimeState, String> {
    if !request.confirmed {
        return Err("runtime_confirmation_required".to_string());
    }
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "deployment_failed".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        if !cfg!(target_os = "windows") {
            return Err("platform_not_supported".to_string());
        }
        require_patch_ready_runtime()?;
        validate_candidate(Path::new(&request.game_path), "manual")?;
        let receipt =
            game_deployment::current_owned_deployment(&app_data, Path::new(&request.game_path))?
                .ok_or_else(|| "deployment_not_found".to_string())?;
        let package_ids = if receipt.package_ids.is_empty() {
            vec![tree_pilot::PACKAGE_ID.to_string()]
        } else {
            receipt.package_ids.clone()
        };
        let verified = tree_pilot::build_from_verified_store(&app_data, &package_ids)?;
        if receipt.installed_sha256 != format!("{:x}", sha2::Sha256::digest(verified.bytes()))
            || receipt
                .bundle_plan_id
                .as_deref()
                .is_some_and(|plan| plan != verified.plan_id())
        {
            return Err("deployment_conflict".to_string());
        }
        if receipt.operation_id != request.operation_id {
            return Err("deployment_conflict".to_string());
        }
        match (
            request.profile_token.as_deref(),
            request.steam_operation_id.as_deref(),
        ) {
            (Some(profile), steam_operation) => {
                steam_accounts::verify_platform_activation(
                    &app_data,
                    profile,
                    steam_operation,
                    receipt.language,
                )?;
            }
            (None, None) => {}
            (None, Some(_)) => return Err("steam_activation_not_ready".to_string()),
        }
        runtime_control::start_steam(SteamStartRequest { confirmed: true })
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
async fn rollback_game_deployment(
    app: AppHandle,
    request: DeploymentOperationRequest,
) -> Result<DeploymentReceipt, String> {
    if !request.confirmed {
        return Err("deployment_confirmation_required".to_string());
    }
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "rollback_failed".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        require_patch_ready_runtime()?;
        let result = game_deployment::rollback(
            &app_data,
            Path::new(&request.game_path),
            &request.operation_id,
        );
        installed_profile::record_rollback(&app_data, result)
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
async fn recover_game_deployments(
    app: AppHandle,
    request: DeploymentRecoveryRequest,
) -> Result<RecoveryReceipt, String> {
    if !request.confirmed {
        return Err("deployment_confirmation_required".to_string());
    }
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "rollback_failed".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        require_patch_ready_runtime()?;
        game_deployment::recover_pending(&app_data, Path::new(&request.game_path))
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
async fn start_steam_after_profile(
    app: AppHandle,
    request: StartSteamAfterProfileRequest,
) -> Result<RuntimeState, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "steam_activation_not_ready".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        require_patch_ready_runtime()?;
        steam_accounts::verify_platform_activation(
            &app_data,
            &request.profile_token,
            request.operation_id.as_deref(),
            request.language,
        )?;
        runtime_control::start_steam(SteamStartRequest {
            confirmed: request.confirmed,
        })
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

#[tauri::command]
async fn start_steam_after_restore(request: SteamStartRequest) -> Result<RuntimeState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        require_patch_ready_runtime()?;
        runtime_control::start_steam(request)
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

fn main() {
    tauri::Builder::default()
        .manage(AuthState::default())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            discover_game,
            list_installable_packages,
            refresh_catalog,
            validate_game_path,
            collect_system_diagnostics,
            intake_fixture_content,
            begin_content_download,
            content_download_status,
            cancel_content_download,
            plan_build,
            execute_build,
            inspect_runtime,
            prepare_runtime_for_patch,
            list_engine_operations,
            rollback_engine_operation,
            list_steam_profiles,
            preview_steam_launch_options,
            apply_steam_launch_options,
            apply_tree_steam_launch_options,
            apply_tree_steam_launch_options_stress,
            rollback_steam_launch_options,
            recover_steam_launch_options,
            deploy_staged_vpk,
            begin_tree_pilot_download,
            tree_pilot_download_status,
            cancel_tree_pilot_download,
            install_tree_pilot,
            tree_pilot_stress_capabilities,
            install_tree_pilot_stress,
            collect_tree_pilot_evidence,
            save_tree_pilot_evidence,
            open_reports_folder,
            current_tree_pilot,
            preview_tree_language,
            start_steam_after_tree_pilot,
            rollback_game_deployment,
            recover_game_deployments,
            start_steam_after_profile,
            start_steam_after_restore,
            list_presets,
            save_preset,
            delete_preset,
            export_preset,
            import_preset,
            auth_verify_code,
            auth_begin_email,
            auth_verify_email,
            auth_id_login,
            auth_id_register_start,
            auth_id_register_verify,
            auth_link_email_start,
            auth_link_email_verify,
            auth_email_identity,
            auth_begin_device_challenge,
            auth_poll_device_challenge,
            auth_cancel_device_challenge,
            auth_restore_session,
            auth_fetch_avatar,
            auth_profile,
            auth_list_sessions,
            auth_revoke_device,
            auth_logout
        ])
        .run(tauri::generate_context!())
        .expect("error while running BetterFy");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dota_build_id_is_read_from_the_app_manifest() {
        let manifest = "\"AppState\"\n{\n\t\"appid\"\t\t\"570\"\n\t\"buildid\"\t\t\"20512345\"\n\t\"UserConfig\"\n\t{\n\t\t\"language\"\t\t\"russian\"\n\t}\n}\n";
        assert_eq!(parse_build_id(manifest).as_deref(), Some("20512345"));
        assert_eq!(parse_build_id("\"buildid\"\t\t\"12a\""), None);
        assert_eq!(parse_build_id("\"TargetBuildID\"\t\t\"1\""), None);

        let library = unique_temp("dota-build");
        let game = library.join("steamapps").join("common").join("dota 2 beta");
        fs::create_dir_all(&game).expect("game dir");
        fs::write(
            library.join("steamapps").join("appmanifest_570.acf"),
            manifest,
        )
        .expect("manifest");
        assert_eq!(dota_build_id(&game).as_deref(), Some("20512345"));
        let _ = fs::remove_dir_all(&library);
    }

    #[test]
    fn extended_length_prefixes_are_hidden_from_the_interface() {
        let drive = simplified_path(PathBuf::from(r"\\?\C:\Steam\steamapps\common\dota 2 beta"));
        assert_eq!(
            drive,
            PathBuf::from(r"C:\Steam\steamapps\common\dota 2 beta")
        );
        let unc = simplified_path(PathBuf::from(r"\\?\UNC\server\games\dota 2 beta"));
        assert_eq!(unc, PathBuf::from(r"\\server\games\dota 2 beta"));
        let device = PathBuf::from(r"\\?\Volume{0}\dota 2 beta");
        assert_eq!(simplified_path(device.clone()), device);
        let long = PathBuf::from(format!(r"\\?\C:\{}", "a".repeat(300)));
        assert_eq!(simplified_path(long.clone()), long);
    }
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("betterfy-{name}-{nonce}"))
    }

    fn create_valid_fixture(root: &Path) {
        let pak = root.join("game").join("dota").join("pak01_dir.vpk");
        let executable = if cfg!(target_os = "windows") {
            root.join("game")
                .join("bin")
                .join("win64")
                .join("dota2.exe")
        } else {
            root.join("game").join("bin").join("osx64").join("dota2")
        };
        fs::create_dir_all(pak.parent().expect("pak parent")).expect("pak dirs");
        fs::create_dir_all(executable.parent().expect("exe parent")).expect("exe dirs");
        fs::write(pak, b"fixture").expect("pak");
        fs::write(executable, b"fixture").expect("exe");
    }

    #[test]
    fn parses_only_vdf_path_entries() {
        let vdf = r#"
            "0"
            {
                "path" "C:\\Program Files (x86)\\Steam"
            }
            "1"
            {
                "path" "D:\\Games\\Steam"
            }
            "contentid" "123"
        "#;
        let paths = parse_library_paths(vdf);
        assert_eq!(paths.len(), 2);
        assert!(paths[0].to_string_lossy().contains("Program Files"));
        assert!(paths[1].to_string_lossy().contains("Games"));
    }

    #[test]
    fn accepts_only_a_marked_dota_installation() {
        let base = unique_temp("valid");
        let root = base.join("dota 2 beta");
        create_valid_fixture(&root);
        let result = validate_candidate(&root, "manual").expect("valid installation");
        assert!(result.verified);
        assert_eq!(result.source, "manual");
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn rejects_wrong_folder_name() {
        let base = unique_temp("wrong-name");
        let root = base.join("not-dota");
        create_valid_fixture(&root);
        assert_eq!(
            validate_candidate(&root, "manual").err().as_deref(),
            Some("invalid_game_path")
        );
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn rejects_missing_marker_files() {
        let base = unique_temp("missing-marker");
        let root = base.join("dota 2 beta");
        fs::create_dir_all(&root).expect("fixture");
        assert_eq!(
            validate_candidate(&root, "manual").err().as_deref(),
            Some("invalid_game_path")
        );
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn saved_evidence_report_contains_no_local_paths() {
        let base = unique_temp("saved-evidence");
        let app_data = base.join("app-data");
        let root = base.join("dota 2 beta");
        create_valid_fixture(&root);
        let package = vpk::build(vec![vpk::VpkInput {
            path: "models/props_tree/tree_oak_01.vmdl_c",
            bytes: b"evidence-fixture",
        }])
        .expect("package");
        let checksum = format!("{:x}", sha2::Sha256::digest(&package));
        game_deployment::deploy_verified_vpk(&app_data, &root, &package, &checksum)
            .expect("deploy");

        let saved = save_tree_pilot_evidence_report(
            &app_data,
            &root.to_string_lossy(),
            "0.1-test".to_string(),
        )
        .expect("save evidence");
        assert_eq!(saved.entries, 1);
        assert!(saved.file_name.starts_with("evidence-"));
        let text = fs::read_to_string(app_data.join("reports").join(&saved.file_name))
            .expect("saved report");
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(parsed["schemaVersion"], 1);
        assert_eq!(parsed["entries"][0]["installedSha256"], checksum);

        let mut forbidden = vec![
            app_data.to_string_lossy().to_string(),
            base.to_string_lossy().to_string(),
            root.to_string_lossy().to_string(),
        ];
        if let Ok(canonical) = app_data.canonicalize() {
            forbidden.push(canonical.to_string_lossy().to_string());
        }
        if let Ok(canonical) = root.canonicalize() {
            forbidden.push(canonical.to_string_lossy().to_string());
        }
        // JSON escapes backslashes, so also check the escaped Windows form.
        let escaped = forbidden
            .iter()
            .map(|path| path.replace('\\', "\\\\"))
            .collect::<Vec<_>>();
        for path in forbidden.iter().chain(escaped.iter()) {
            assert!(!text.contains(path.as_str()), "report leaks {path}");
        }
        assert!(!text.contains("targetIdentity"));
        assert!(!text.contains("profileToken"));
        let _ = fs::remove_dir_all(base);
    }
}

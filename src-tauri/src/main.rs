#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod archive_inspector;
mod auth_session;
mod build_engine;
mod content_store;
mod presets;
mod remote_intake;
mod runtime_control;
mod steam_accounts;
pub mod steam_config;
mod system_diagnostics;

use auth_session::{
    auth_fetch_avatar, auth_list_sessions, auth_logout, auth_restore_session, auth_revoke_device,
    auth_verify_code, AuthState,
};
use build_engine::{
    create_build_plan, execute_build as execute_staged_build,
    list_operations as list_staged_operations, operation_diagnostic_counts, rollback_operation,
    BuildPlan, BuildPlanRequest, BuildReceipt, ExecuteBuildRequest, OperationSummary,
    RollbackReceipt,
};
use content_store::{ContentIntakeRequest, ContentReceipt};
use presets::{delete_preset, export_preset, import_preset, list_presets, save_preset};
use remote_intake::ContentDownloadStatus;
use runtime_control::{RuntimePrepareRequest, RuntimeState, SteamStartRequest};
use serde::{Deserialize, Serialize};
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

fn validate_candidate(path: &Path, source: &'static str) -> Result<GameInstallation, String> {
    let canonical = path
        .canonicalize()
        .map_err(|_| "game_not_found".to_string())?;
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

#[tauri::command]
fn discover_game() -> Vec<GameInstallation> {
    discovery_candidates()
        .iter()
        .filter_map(|path| validate_candidate(path, "auto").ok())
        .collect()
}

#[tauri::command]
fn validate_game_path(path: String) -> Result<GameInstallation, String> {
    validate_candidate(Path::new(&path), "manual")
}

#[tauri::command]
fn collect_system_diagnostics(app: AppHandle, game_path: Option<String>) -> SystemDiagnosticReport {
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
        .map(operation_diagnostic_counts)
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

#[tauri::command]
fn execute_build(app: AppHandle, request: ExecuteBuildRequest) -> Result<BuildReceipt, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "build_failed".to_string())?;
    execute_staged_build(&app_data, request)
}

#[tauri::command]
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

#[tauri::command]
fn list_engine_operations(app: AppHandle) -> Result<Vec<OperationSummary>, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_| "journal_invalid".to_string())?;
    list_staged_operations(&app_data)
}

#[tauri::command]
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
fn list_steam_profiles() -> Result<Vec<SteamProfileSummary>, String> {
    steam_accounts::list_platform_profiles()
}

#[tauri::command]
fn preview_steam_launch_options(profile_token: String) -> Result<SteamLaunchOptionPreview, String> {
    steam_accounts::preview_platform_profile(&profile_token)
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
        steam_accounts::rollback_platform_operation(&app_data, request)
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
        )?;
        runtime_control::start_steam(SteamStartRequest {
            confirmed: request.confirmed,
        })
    })
    .await
    .map_err(|_| "runtime_worker_failed".to_string())?
}

fn main() {
    tauri::Builder::default()
        .manage(AuthState::default())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            discover_game,
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
            rollback_steam_launch_options,
            recover_steam_launch_options,
            start_steam_after_profile,
            list_presets,
            save_preset,
            delete_preset,
            export_preset,
            import_preset,
            auth_verify_code,
            auth_restore_session,
            auth_fetch_avatar,
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
}

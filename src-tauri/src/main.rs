#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod presets;

use presets::{delete_preset, export_preset, import_preset, list_presets, save_preset};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureManifest {
    id: String,
    name: String,
    version: String,
    files: Vec<FixtureFile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureFile {
    source: String,
    destination: String,
    size: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildPlanRequest {
    mod_ids: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanInput {
    id: String,
    name: String,
    version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanOperation {
    owner_id: String,
    source: String,
    destination: String,
    size: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanConflict {
    destination: String,
    contenders: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildPlan {
    plan_id: &'static str,
    dry_run: bool,
    inputs: Vec<PlanInput>,
    operations: Vec<PlanOperation>,
    conflicts: Vec<PlanConflict>,
    space_estimate: u64,
    executable: bool,
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

fn fixture_manifests() -> Result<Vec<FixtureManifest>, String> {
    [
        include_str!("../fixtures/mods/ambient-violet.json"),
        include_str!("../fixtures/mods/ambient-clean.json"),
    ]
    .iter()
    .map(|contents| serde_json::from_str(contents).map_err(|_| "catalog_invalid".to_string()))
    .collect()
}

fn create_build_plan(request: BuildPlanRequest) -> Result<BuildPlan, String> {
    let available = fixture_manifests()?;
    let mut selected = Vec::new();
    let mut selected_ids = HashSet::new();
    for requested in &request.mod_ids {
        if !selected_ids.insert(requested.clone()) {
            continue;
        }
        let manifest = available
            .iter()
            .find(|manifest| &manifest.id == requested)
            .ok_or_else(|| "catalog_invalid".to_string())?;
        selected.push(manifest);
    }

    let mut operations = Vec::new();
    let mut destinations: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for manifest in &selected {
        for file in &manifest.files {
            if Path::new(&file.destination).is_absolute()
                || file.destination.split('/').any(|part| part == "..")
            {
                return Err("catalog_invalid".to_string());
            }
            destinations
                .entry(file.destination.clone())
                .or_default()
                .push(manifest.id.clone());
            operations.push(PlanOperation {
                owner_id: manifest.id.clone(),
                source: file.source.clone(),
                destination: file.destination.clone(),
                size: file.size,
            });
        }
    }

    operations.sort_by(|left, right| {
        left.destination
            .cmp(&right.destination)
            .then(left.owner_id.cmp(&right.owner_id))
    });
    let conflicts: Vec<PlanConflict> = destinations
        .into_iter()
        .filter_map(|(destination, contenders)| {
            (contenders.len() > 1).then_some(PlanConflict {
                destination,
                contenders,
            })
        })
        .collect();
    let space_estimate = operations.iter().map(|operation| operation.size).sum();
    let inputs = selected
        .iter()
        .map(|manifest| PlanInput {
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
        })
        .collect();

    Ok(BuildPlan {
        plan_id: "fixture-plan-v1",
        dry_run: true,
        inputs,
        operations,
        executable: conflicts.is_empty() && !selected.is_empty(),
        conflicts,
        space_estimate,
    })
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
fn plan_build(request: BuildPlanRequest) -> Result<BuildPlan, String> {
    create_build_plan(request)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            discover_game,
            validate_game_path,
            plan_build,
            list_presets,
            save_preset,
            delete_preset,
            export_preset,
            import_preset
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

    #[test]
    fn fixture_plan_is_deterministic_and_reports_conflict() {
        let request = BuildPlanRequest {
            mod_ids: vec![
                "fixture.ambient-violet".to_string(),
                "fixture.ambient-clean".to_string(),
            ],
        };
        let plan = create_build_plan(request).expect("plan");
        assert_eq!(plan.inputs.len(), 2);
        assert_eq!(plan.operations.len(), 2);
        assert_eq!(plan.conflicts.len(), 1);
        assert_eq!(
            plan.conflicts[0].destination,
            "game/dota/pak01_dir/panorama/styles/betterfy-theme.css"
        );
        assert!(!plan.executable);
    }

    #[test]
    fn fixture_plan_rejects_unknown_mod() {
        let request = BuildPlanRequest {
            mod_ids: vec!["fixture.unknown".to_string()],
        };
        assert_eq!(
            create_build_plan(request).err().as_deref(),
            Some("catalog_invalid")
        );
    }

    #[test]
    fn fixture_plan_deduplicates_repeated_mod_ids() {
        let request = BuildPlanRequest {
            mod_ids: vec![
                "fixture.ambient-violet".to_string(),
                "fixture.ambient-violet".to_string(),
            ],
        };
        let plan = create_build_plan(request).expect("plan");
        assert_eq!(plan.inputs.len(), 1);
        assert_eq!(plan.operations.len(), 1);
        assert!(plan.conflicts.is_empty());
        assert!(plan.executable);
    }
}

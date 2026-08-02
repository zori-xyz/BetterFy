use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

const SCHEMA_VERSION: u8 = 1;
const MAX_ITEMS: usize = 256;
const MAX_PRESET_FILES: usize = 512;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresetRecord {
    schema_version: u8,
    id: String,
    name: String,
    description: String,
    author: String,
    version: String,
    mod_ids: Vec<String>,
    wardrobe_ids: Vec<String>,
    source: PresetSource,
    read_only: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum PresetSource {
    Local,
    Betterfy,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavePresetRequest {
    id: Option<String>,
    name: String,
    description: String,
    mod_ids: Vec<String>,
    #[serde(default)]
    wardrobe_ids: Vec<String>,
}

fn stable_now() -> Result<String, String> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "preset_clock_invalid".to_string())?
        .as_millis();
    Ok(format!("unix:{millis}"))
}

fn is_safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn normalized_ids(values: Vec<String>) -> Result<Vec<String>, String> {
    if values.len() > MAX_ITEMS {
        return Err("preset_too_large".to_string());
    }
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let value = value.trim().to_string();
        if !is_safe_identifier(&value) {
            return Err("preset_invalid".to_string());
        }
        if seen.insert(value.clone()) {
            normalized.push(value);
        }
    }
    Ok(normalized)
}

fn validate_text(value: &str, max: usize, required: bool) -> Result<String, String> {
    let value = value.trim();
    let length = value.chars().count();
    if (required && value.is_empty()) || length > max {
        return Err("preset_invalid".to_string());
    }
    Ok(value.to_string())
}

fn validate_record(record: &PresetRecord) -> Result<(), String> {
    if record.schema_version != SCHEMA_VERSION || !is_safe_identifier(&record.id) {
        return Err("preset_invalid".to_string());
    }
    validate_text(&record.name, 64, true)?;
    validate_text(&record.description, 280, false)?;
    validate_text(&record.author, 64, true)?;
    validate_text(&record.version, 24, true)?;
    normalized_ids(record.mod_ids.clone())?;
    normalized_ids(record.wardrobe_ids.clone())?;
    if record.mod_ids.len() + record.wardrobe_ids.len() == 0 {
        return Err("preset_invalid".to_string());
    }
    Ok(())
}

fn presets_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|_| "preset_store_unavailable".to_string())?
        .join("presets");
    fs::create_dir_all(&directory).map_err(|_| "preset_store_unavailable".to_string())?;
    let metadata =
        fs::symlink_metadata(&directory).map_err(|_| "preset_store_unavailable".to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("preset_store_unavailable".to_string());
    }
    Ok(directory)
}

fn local_path(directory: &Path, id: &str) -> Result<PathBuf, String> {
    if !id.starts_with("local.") || !is_safe_identifier(id) {
        return Err("preset_invalid".to_string());
    }
    Ok(directory.join(format!("{id}.json")))
}

fn read_local_presets(directory: &Path) -> Result<Vec<PresetRecord>, String> {
    let mut entries = fs::read_dir(directory)
        .map_err(|_| "preset_store_unavailable".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "preset_store_unavailable".to_string())?;
    if entries.len() > MAX_PRESET_FILES {
        return Err("preset_store_invalid".to_string());
    }
    entries.sort_by_key(|entry| entry.file_name());
    let mut presets = Vec::new();
    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let file_type = entry
            .file_type()
            .map_err(|_| "preset_store_invalid".to_string())?;
        if file_type.is_symlink() || !file_type.is_file() {
            return Err("preset_store_invalid".to_string());
        }
        let bytes = fs::read(&path).map_err(|_| "preset_store_invalid".to_string())?;
        if bytes.len() > 64 * 1024 {
            return Err("preset_store_invalid".to_string());
        }
        let preset: PresetRecord =
            serde_json::from_slice(&bytes).map_err(|_| "preset_store_invalid".to_string())?;
        validate_record(&preset).map_err(|_| "preset_store_invalid".to_string())?;
        if preset.source != PresetSource::Local || preset.read_only {
            return Err("preset_store_invalid".to_string());
        }
        presets.push(preset);
    }
    Ok(presets)
}

fn atomic_write_owned(path: &Path, contents: &[u8]) -> Result<(), String> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("preset_store_invalid".to_string());
        }
    }
    let temporary = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");
    if temporary.exists() || backup.exists() {
        return Err("preset_store_recovery_required".to_string());
    }
    fs::write(&temporary, contents).map_err(|_| "preset_store_unavailable".to_string())?;

    let had_previous = path.exists();
    if had_previous {
        fs::rename(path, &backup).map_err(|_| "preset_store_unavailable".to_string())?;
    }
    if fs::rename(&temporary, path).is_err() {
        let _ = fs::remove_file(&temporary);
        if had_previous {
            let _ = fs::rename(&backup, path);
        }
        return Err("preset_store_unavailable".to_string());
    }
    if had_previous {
        fs::remove_file(&backup).map_err(|_| "preset_store_recovery_required".to_string())?;
    }
    Ok(())
}

fn save_preset_to_dir(
    directory: &Path,
    request: SavePresetRequest,
) -> Result<PresetRecord, String> {
    let local = read_local_presets(directory)?;
    let existing = match request.id.as_deref() {
        Some(id) => Some(
            local
                .iter()
                .find(|preset| preset.id == id)
                .ok_or_else(|| "preset_not_found".to_string())?,
        ),
        None => None,
    };
    let timestamp = stable_now()?;
    let id = existing.map(|preset| preset.id.clone()).unwrap_or_else(|| {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        format!("local.{nonce:x}")
    });
    let record = PresetRecord {
        schema_version: SCHEMA_VERSION,
        id: id.clone(),
        name: validate_text(&request.name, 64, true)?,
        description: validate_text(&request.description, 280, false)?,
        author: "Local profile".to_string(),
        version: existing
            .map(|preset| preset.version.clone())
            .unwrap_or_else(|| "1.0.0".to_string()),
        mod_ids: normalized_ids(request.mod_ids)?,
        wardrobe_ids: normalized_ids(request.wardrobe_ids)?,
        source: PresetSource::Local,
        read_only: false,
        created_at: existing
            .map(|preset| preset.created_at.clone())
            .unwrap_or_else(|| timestamp.clone()),
        updated_at: timestamp,
    };
    validate_record(&record)?;
    let bytes = serde_json::to_vec_pretty(&record).map_err(|_| "preset_invalid".to_string())?;
    atomic_write_owned(&local_path(directory, &id)?, &bytes)?;
    Ok(record)
}

fn workshop_presets() -> Vec<PresetRecord> {
    let make = |id: &str, name: &str, description: &str, mod_ids: &[&str]| PresetRecord {
        schema_version: SCHEMA_VERSION,
        id: id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        author: "BetterFy".to_string(),
        version: "1.0.0".to_string(),
        mod_ids: mod_ids.iter().map(|value| value.to_string()).collect(),
        wardrobe_ids: Vec::new(),
        source: PresetSource::Betterfy,
        read_only: true,
        created_at: "2026-08-02T00:00:00.000Z".to_string(),
        updated_at: "2026-08-02T00:00:00.000Z".to_string(),
    };
    vec![
        make(
            "betterfy.focus-performance",
            "Focus / Performance",
            "A focused local composition built from the imported Minify snapshot.",
            &[
                "minify-misc-optimization",
                "minify-remove-foilage",
                "minify-remove-river",
                "minify-remove-weather-effects",
                "minify-remove-hero-renders",
            ],
        ),
        make(
            "betterfy.clean-interface",
            "Clean Interface",
            "A calm interface composition built from the imported Minify snapshot.",
            &[
                "minify-remove-main-menu-background",
                "minify-remove-showcases",
                "minify-transparent-hud",
                "minify-revamp-hero-grid-layout",
                "minify-show-networth",
            ],
        ),
        make(
            "betterfy.quiet-match",
            "Quiet Match",
            "An audio-light composition built from the imported Minify snapshot.",
            &[
                "minify-mute-ambient-sounds",
                "minify-mute-taunt-sounds",
                "minify-mute-voice-line-sounds",
            ],
        ),
    ]
}

#[tauri::command]
pub fn list_presets(app: tauri::AppHandle) -> Result<Vec<PresetRecord>, String> {
    let mut presets = read_local_presets(&presets_dir(&app)?)?;
    presets.extend(workshop_presets());
    Ok(presets)
}

#[tauri::command]
pub fn save_preset(
    app: tauri::AppHandle,
    request: SavePresetRequest,
) -> Result<PresetRecord, String> {
    save_preset_to_dir(&presets_dir(&app)?, request)
}

#[tauri::command]
pub fn delete_preset(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let path = local_path(&presets_dir(&app)?, &id)?;
    let metadata = fs::symlink_metadata(&path).map_err(|_| "preset_not_found".to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("preset_store_invalid".to_string());
    }
    fs::remove_file(path).map_err(|_| "preset_store_unavailable".to_string())
}

#[tauri::command]
pub fn export_preset(app: tauri::AppHandle, id: String) -> Result<String, String> {
    let mut presets = read_local_presets(&presets_dir(&app)?)?;
    presets.extend(workshop_presets());
    let preset = presets
        .into_iter()
        .find(|preset| preset.id == id)
        .ok_or_else(|| "preset_not_found".to_string())?;
    serde_json::to_string_pretty(&preset).map_err(|_| "preset_invalid".to_string())
}

#[tauri::command]
pub fn import_preset(app: tauri::AppHandle, serialized: String) -> Result<PresetRecord, String> {
    if serialized.len() > 64 * 1024 {
        return Err("preset_too_large".to_string());
    }
    let imported: PresetRecord =
        serde_json::from_str(&serialized).map_err(|_| "preset_invalid".to_string())?;
    validate_record(&imported)?;
    save_preset_to_dir(
        &presets_dir(&app)?,
        SavePresetRequest {
            id: None,
            name: imported.name,
            description: imported.description,
            mod_ids: imported.mod_ids,
            wardrobe_ids: imported.wardrobe_ids,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_directory(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("betterfy-presets-{name}-{nonce}"));
        fs::create_dir_all(&path).expect("temp directory");
        path
    }

    fn request() -> SavePresetRequest {
        SavePresetRequest {
            id: None,
            name: "Clean HUD".to_string(),
            description: "Local test".to_string(),
            mod_ids: vec!["minify-transparent-hud".to_string()],
            wardrobe_ids: Vec::new(),
        }
    }

    #[test]
    fn saves_and_reads_a_local_preset() {
        let directory = temporary_directory("roundtrip");
        let saved = save_preset_to_dir(&directory, request()).expect("save");
        let loaded = read_local_presets(&directory).expect("read");
        assert_eq!(loaded, vec![saved]);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn rejects_unknown_manifest_fields() {
        let invalid = r#"{
          "schemaVersion":1,"id":"local.safe","name":"Safe","description":"",
          "author":"Local profile","version":"1.0.0","modIds":["safe-mod"],
          "wardrobeIds":[],"source":"local","readOnly":false,
          "createdAt":"unix:1","updatedAt":"unix:1","unexpected":true
        }"#;
        assert!(serde_json::from_str::<PresetRecord>(invalid).is_err());
    }

    #[test]
    fn rejects_traversing_or_excessive_ids() {
        assert!(normalized_ids(vec!["../escape".to_string()]).is_err());
        assert!(normalized_ids(vec!["ok".to_string(); MAX_ITEMS + 1]).is_err());
        assert!(local_path(Path::new("/tmp"), "betterfy.readonly").is_err());
    }

    #[test]
    fn rejects_symlinked_local_records() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let directory = temporary_directory("symlink");
            let outside = directory.with_extension("outside");
            fs::write(&outside, b"{}").expect("outside");
            symlink(&outside, directory.join("local.bad.json")).expect("symlink");
            assert_eq!(
                read_local_presets(&directory).err().as_deref(),
                Some("preset_store_invalid")
            );
            fs::remove_dir_all(directory).expect("cleanup");
            fs::remove_file(outside).expect("cleanup outside");
        }
    }
}

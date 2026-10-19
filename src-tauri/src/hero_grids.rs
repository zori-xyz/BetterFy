//! Minify's "Custom Hero Grids": ready-made hero grids added to the player's
//! own, in every Steam account's `570/remote/cfg/hero_grid_config.json`.
//!
//! Minify fetches the latest grids of `Egezenn/dota2-precompiled-grids` and
//! appends them, named with a suffix, after removing the ones it added
//! before. BetterFy does the same with one release of those grids compiled
//! into the app (2026-10-19, MIT), named `... #BetterFy`, as a small
//! transaction linked to the deployment operation of the build:
//!
//! * the player's own grids are never touched; only entries named with the
//!   suffix are added or removed;
//! * the bytes of every file are kept in the content store before it is
//!   written, and the journal names them;
//! * restore puts a file back byte for byte when nobody changed it since;
//!   when the player edited their grids in the game meanwhile, it removes only
//!   the grids BetterFy added and keeps the edits;
//! * a write that stops half way is undone by recovery.

use crate::content_store;
use crate::game_deployment::{publish, reject_link, validate_operation_id};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub(crate) const PACKAGE_ID: &str = "minify.custom-hero-grids";
pub(crate) const CATALOG_ID: &str = "minify-custom-hero-grids";
const GRIDS: &str = include_str!("../herogrids/hero_grid_config.json");
#[cfg(test)]
const INDEX: &str = include_str!("../herogrids/index.json");
const SUFFIX: &str = " #BetterFy";
const FILE_NAME: &str = "hero_grid_config.json";
const SCHEMA_VERSION: u32 = 1;
const MAX_GRID_BYTES: usize = 4 * 1024 * 1024;
const MAX_RECORD_BYTES: u64 = 1024 * 1024;

static LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Stored {
    sha256: String,
    bytes: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Entry {
    /// The account's `hero_grid_config.json`.
    path: PathBuf,
    before: Option<Stored>,
    after: Option<Stored>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Prepared,
    Committed,
    RolledBack,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Journal {
    schema_version: u32,
    operation_id: String,
    phase: Phase,
    /// The files that held BetterFy's grids before and after the operation.
    owned_before: BTreeSet<PathBuf>,
    owned_after: BTreeSet<PathBuf>,
    entries: Vec<Entry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Ownership {
    schema_version: u32,
    operation_id: String,
    files: BTreeSet<PathBuf>,
}

/// The grids BetterFy adds, as Dota's `configs` entries with the suffix.
pub(crate) fn pinned_configs() -> Result<Vec<Value>, String> {
    let value: Value = serde_json::from_str(GRIDS).map_err(|_| "hero_grids_invalid".to_string())?;
    let configs = value["configs"]
        .as_array()
        .ok_or_else(|| "hero_grids_invalid".to_string())?;
    configs
        .iter()
        .map(|config| {
            let mut config = config.clone();
            let name = config["config_name"]
                .as_str()
                .ok_or_else(|| "hero_grids_invalid".to_string())?
                .to_string();
            config["config_name"] = Value::String(format!("{name}{SUFFIX}"));
            Ok(config)
        })
        .collect()
}

/// The text BetterFy puts into the build's VPK so the reviewed plan is bound
/// to these exact grids; the game never reads it.
pub(crate) fn marker() -> (String, Vec<u8>) {
    (
        format!("betterfy/{CATALOG_ID}.txt"),
        format!(
            "{PACKAGE_ID}\nhero_grid_config.json {} {}\n",
            GRIDS.len(),
            sha256(GRIDS.as_bytes())
        )
        .into_bytes(),
    )
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_ours(config: &Value) -> bool {
    config["config_name"]
        .as_str()
        .is_some_and(|name| name.ends_with(SUFFIX))
}

/// The player's file without BetterFy's grids, plus them when `add`. `None`
/// when the file is not a grid file BetterFy can read: it is then left alone.
fn rewrite(current: Option<&[u8]>, add: bool) -> Option<Vec<u8>> {
    let mut value: Value = match current {
        None => serde_json::json!({ "version": 3, "configs": [] }),
        Some(bytes) => serde_json::from_slice(bytes).ok()?,
    };
    let object = value.as_object_mut()?;
    let configs = object
        .entry("configs")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()?;
    configs.retain(|config| !is_ours(config));
    if add {
        configs.extend(pinned_configs().ok()?);
    }
    object.entry("version").or_insert_with(|| Value::from(3));
    serde_json::to_vec(&value).ok()
}

fn records_root(app_data_root: &Path) -> Result<PathBuf, String> {
    reject_link(app_data_root)?;
    let root = app_data_root.join("engine-v1").join("hero-grids");
    for path in [
        app_data_root.join("engine-v1"),
        root.clone(),
        root.join("journals"),
    ] {
        reject_link(&path)?;
        fs::create_dir_all(&path).map_err(|_| "hero_grids_failed".to_string())?;
        reject_link(&path)?;
    }
    Ok(root)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    reject_link(path)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "hero_grids_failed".to_string())?;
    let temporary = path.with_file_name(format!(".betterfy-{name}.tmp"));
    reject_link(&temporary)?;
    if temporary.exists() {
        fs::remove_file(&temporary).map_err(|_| "hero_grids_failed".to_string())?;
    }
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|_| "hero_grids_failed".to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "hero_grids_failed".to_string())?;
    drop(file);
    publish(path, &temporary, path.exists()).map_err(|_| {
        let _ = fs::remove_file(&temporary);
        "hero_grids_failed".to_string()
    })
}

fn write_record<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| "hero_grids_failed".to_string())?;
    write_atomic(path, &bytes)
}

fn read_record<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Option<T>, String> {
    reject_link(path)?;
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() && metadata.len() <= MAX_RECORD_BYTES => {}
        Ok(_) => return Err("hero_grids_record_invalid".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("hero_grids_record_invalid".to_string()),
    }
    let bytes = fs::read(path).map_err(|_| "hero_grids_record_invalid".to_string())?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| "hero_grids_record_invalid".to_string())
}

fn current(path: &Path) -> Result<Option<Vec<u8>>, String> {
    reject_link(path)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && metadata.len() <= MAX_GRID_BYTES as u64 => {
            fs::read(path)
                .map(Some)
                .map_err(|_| "hero_grids_failed".to_string())
        }
        Ok(_) => Err("hero_grids_target_unsafe".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("hero_grids_failed".to_string()),
    }
}

fn stored(bytes: &Option<Vec<u8>>) -> Option<Stored> {
    bytes.as_ref().map(|bytes| Stored {
        sha256: sha256(bytes),
        bytes: bytes.len(),
    })
}

fn keep(app_data_root: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() {
        return Ok(());
    }
    content_store::store_pinned_resource(app_data_root, bytes.len(), &sha256(bytes), bytes)
        .map_err(|_| "hero_grids_backup_failed".to_string())
}

fn read_kept(app_data_root: &Path, stored: &Stored) -> Result<Vec<u8>, String> {
    if stored.bytes == 0 {
        return Ok(Vec::new());
    }
    content_store::read_pinned_resource(app_data_root, stored.bytes, &stored.sha256)
        .map_err(|_| "hero_grids_backup_missing".to_string())
}

/// Puts each entry back to `before` when the file is still what the
/// operation left, and otherwise removes only BetterFy's grids from it.
fn restore(app_data_root: &Path, entries: &[Entry]) -> Result<(), String> {
    for entry in entries {
        let now = current(&entry.path)?;
        if stored(&now) == entry.before {
            continue;
        }
        if stored(&now) == entry.after {
            match &entry.before {
                Some(before) => write_atomic(&entry.path, &read_kept(app_data_root, before)?)?,
                None => {
                    fs::remove_file(&entry.path).map_err(|_| "hero_grids_failed".to_string())?
                }
            }
        } else if let Some(stripped) = rewrite(now.as_deref(), false) {
            if Some(&stripped) != now.as_ref() {
                write_atomic(&entry.path, &stripped)?;
            }
        }
    }
    Ok(())
}

fn pending(root: &Path) -> Result<Vec<Journal>, String> {
    let mut journals = Vec::new();
    for entry in fs::read_dir(root.join("journals")).map_err(|_| "hero_grids_failed".to_string())? {
        let path = entry.map_err(|_| "hero_grids_failed".to_string())?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        if let Some(journal) = read_record::<Journal>(&path)? {
            if journal.phase == Phase::Prepared {
                journals.push(journal);
            }
        }
    }
    Ok(journals)
}

fn save_ownership(
    root: &Path,
    operation_id: &str,
    files: &BTreeSet<PathBuf>,
) -> Result<(), String> {
    let path = root.join("ownership.json");
    if files.is_empty() {
        return match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("hero_grids_failed".to_string()),
        };
    }
    write_record(
        &path,
        &Ownership {
            schema_version: SCHEMA_VERSION,
            operation_id: operation_id.to_string(),
            files: files.clone(),
        },
    )
}

/// Adds BetterFy's grids to every account folder in `cfg_folders` (`add`),
/// or takes them out of the files that have them (a build without the mod).
/// Returns how many files hold BetterFy's grids afterwards.
pub(crate) fn apply(
    app_data_root: &Path,
    operation_id: &str,
    add: bool,
    cfg_folders: &[PathBuf],
) -> Result<usize, String> {
    validate_operation_id(operation_id).map_err(|_| "hero_grids_failed".to_string())?;
    let _lock = LOCK.lock().map_err(|_| "hero_grids_locked".to_string())?;
    let root = records_root(app_data_root)?;
    if !pending(&root)?.is_empty() {
        return Err("hero_grids_recovery_required".to_string());
    }
    let owned_before = read_record::<Ownership>(&root.join("ownership.json"))?
        .map(|ownership| ownership.files)
        .unwrap_or_default();
    let mut targets = owned_before.clone();
    if add {
        for folder in cfg_folders {
            reject_link(folder)?;
            targets.insert(folder.join(FILE_NAME));
        }
        if targets.is_empty() {
            return Err("hero_grids_no_profile".to_string());
        }
    }
    if targets.is_empty() {
        return Ok(0);
    }
    let mut entries = Vec::new();
    let mut owned_after = BTreeSet::new();
    for path in &targets {
        let now = current(path)?;
        let Some(next) = rewrite(now.as_deref(), add) else {
            // Not a grid file BetterFy can read: left as it is.
            continue;
        };
        if let Some(bytes) = &now {
            keep(app_data_root, bytes)?;
        }
        keep(app_data_root, &next)?;
        if add {
            owned_after.insert(path.clone());
        }
        entries.push(Entry {
            path: path.clone(),
            before: stored(&now),
            after: stored(&Some(next)),
        });
    }
    let mut journal = Journal {
        schema_version: SCHEMA_VERSION,
        operation_id: operation_id.to_string(),
        phase: Phase::Prepared,
        owned_before: owned_before.clone(),
        owned_after: owned_after.clone(),
        entries,
    };
    let record = root.join("journals").join(format!("{operation_id}.json"));
    if record.exists() {
        return Err("hero_grids_failed".to_string());
    }
    write_record(&record, &journal)?;
    let written = journal.entries.iter().try_for_each(|entry| {
        if entry.before == entry.after {
            return Ok(());
        }
        let after = entry
            .after
            .as_ref()
            .ok_or_else(|| "hero_grids_failed".to_string())?;
        write_atomic(&entry.path, &read_kept(app_data_root, after)?)?;
        if stored(&current(&entry.path)?) != entry.after {
            return Err("hero_grids_verify_failed".to_string());
        }
        Ok(())
    });
    if let Err(code) = written {
        if restore(app_data_root, &journal.entries).is_ok() {
            journal.phase = Phase::Failed;
            let _ = write_record(&record, &journal);
        }
        return Err(code);
    }
    save_ownership(&root, operation_id, &owned_after)?;
    journal.phase = Phase::Committed;
    write_record(&record, &journal)?;
    Ok(owned_after.len())
}

/// Restores what the build of `operation_id` changed. `Ok(false)` when that
/// operation changed no grid file.
pub(crate) fn rollback(app_data_root: &Path, operation_id: &str) -> Result<bool, String> {
    validate_operation_id(operation_id).map_err(|_| "hero_grids_failed".to_string())?;
    let _lock = LOCK.lock().map_err(|_| "hero_grids_locked".to_string())?;
    let root = records_root(app_data_root)?;
    let record = root.join("journals").join(format!("{operation_id}.json"));
    let Some(mut journal) = read_record::<Journal>(&record)? else {
        return Ok(false);
    };
    if !matches!(journal.phase, Phase::Committed | Phase::Prepared) {
        return Ok(false);
    }
    restore(app_data_root, &journal.entries)?;
    save_ownership(&root, operation_id, &journal.owned_before)?;
    journal.phase = Phase::RolledBack;
    write_record(&record, &journal)?;
    Ok(true)
}

/// Finishes every operation that stopped before it committed.
pub(crate) fn recover_pending(app_data_root: &Path) -> Result<usize, String> {
    let _lock = LOCK.lock().map_err(|_| "hero_grids_locked".to_string())?;
    let root = records_root(app_data_root)?;
    let journals = pending(&root)?;
    for mut journal in journals.iter().cloned() {
        restore(app_data_root, &journal.entries)?;
        save_ownership(&root, &journal.operation_id, &journal.owned_before)?;
        journal.phase = Phase::RolledBack;
        write_record(
            &root
                .join("journals")
                .join(format!("{}.json", journal.operation_id)),
            &journal,
        )?;
    }
    Ok(journals.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn root(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "betterfy-hero-grids-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("root");
        path
    }

    fn names(path: &Path) -> Vec<String> {
        let value: Value = serde_json::from_slice(&fs::read(path).expect("file")).expect("json");
        value["configs"]
            .as_array()
            .expect("configs")
            .iter()
            .map(|config| config["config_name"].as_str().expect("name").to_string())
            .collect()
    }

    #[test]
    fn the_pinned_grids_and_the_interface_entry_are_valid() {
        let configs = pinned_configs().expect("grids");
        assert_eq!(configs.len(), 3);
        assert!(configs.iter().all(is_ours));
        let index: Value = serde_json::from_str(INDEX).expect("index");
        assert_eq!(index[0]["id"], PACKAGE_ID);
        assert_eq!(index[0]["catalogId"], CATALOG_ID);
    }

    #[test]
    fn grids_are_added_beside_the_players_and_restored_exactly() {
        let base = root("chain");
        let app = base.join("app");
        let cfg = base.join("userdata/1/570/remote/cfg");
        fs::create_dir_all(&cfg).expect("cfg");
        let file = cfg.join(FILE_NAME);
        let player = br#"{"version":3,"configs":[{"config_name":"Mine","categories":[]}]}"#;
        fs::write(&file, player).expect("player grids");
        assert_eq!(
            apply(&app, "deploy-op-1-1-1", true, std::slice::from_ref(&cfg)).expect("add"),
            1
        );
        let after = names(&file);
        assert_eq!(after[0], "Mine");
        assert_eq!(after.len(), 4);
        assert!(after[1..].iter().all(|name| name.ends_with(SUFFIX)));
        // Installing again does not add them twice.
        assert_eq!(
            apply(&app, "deploy-op-2-1-1", true, std::slice::from_ref(&cfg)).expect("again"),
            1
        );
        assert_eq!(names(&file).len(), 4);
        // Restoring the second build leaves the first build's grids.
        assert!(rollback(&app, "deploy-op-2-1-1").expect("rollback 2"));
        assert_eq!(names(&file).len(), 4);
        // Restoring the first gives back the player's file byte for byte.
        assert!(rollback(&app, "deploy-op-1-1-1").expect("rollback 1"));
        assert_eq!(fs::read(&file).expect("file"), player);
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn the_players_edits_survive_restore_and_a_build_without_the_mod_removes_ours() {
        let base = root("edits");
        let app = base.join("app");
        let cfg = base.join("userdata/2/570/remote/cfg");
        fs::create_dir_all(&cfg).expect("cfg");
        let file = cfg.join(FILE_NAME);
        // No grid file yet: one is created with BetterFy's grids.
        apply(&app, "deploy-op-1-1-1", true, std::slice::from_ref(&cfg)).expect("add");
        assert_eq!(names(&file).len(), 3);
        // The player adds a grid in the game.
        let mut value: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        value["configs"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"config_name": "New", "categories": []}));
        fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
        // A build without the mod takes out only BetterFy's grids.
        assert_eq!(
            apply(&app, "deploy-op-2-1-1", false, &[]).expect("remove"),
            0
        );
        assert_eq!(names(&file), vec!["New".to_string()]);
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn an_unreadable_grid_file_is_left_alone_and_no_account_is_an_error() {
        let base = root("unreadable");
        let app = base.join("app");
        let cfg = base.join("userdata/3/570/remote/cfg");
        fs::create_dir_all(&cfg).expect("cfg");
        fs::write(cfg.join(FILE_NAME), b"not json").expect("garbage");
        assert_eq!(
            apply(&app, "deploy-op-1-1-1", true, std::slice::from_ref(&cfg)).expect("apply"),
            0
        );
        assert_eq!(fs::read(cfg.join(FILE_NAME)).unwrap(), b"not json");
        let other = root("none");
        assert_eq!(
            apply(&other.join("app"), "deploy-op-1-1-1", true, &[])
                .err()
                .as_deref(),
            Some("hero_grids_no_profile")
        );
        fs::remove_dir_all(base).expect("cleanup");
        fs::remove_dir_all(other).expect("cleanup");
    }
}

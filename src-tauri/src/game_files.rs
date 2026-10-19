//! The game's own loose files a build replaces: cursors, Panorama fonts and
//! item builds.
//!
//! Dota keeps these as plain files under `game/dota` (`resource/cursor`,
//! `panorama/fonts`, `itembuilds`), not in a VPK, and the catalog's cursor and
//! font packs and Minify's guide and font mods install by writing over them.
//! BetterFy does the same, as a transaction of its own that is linked to the
//! deployment operation of the build:
//!
//! * only files directly inside those three folders, with plain lower-case
//!   names, are ever touched, and the folders are never created;
//! * before anything is written, the bytes at every path the build touches are
//!   kept in the content store, and the journal names them;
//! * the original file of the game (or its absence) is remembered per path,
//!   across builds, so a later build without that file puts the game's own
//!   file back, and restoring the last build puts back exactly what was there
//!   before it;
//! * a write that stops half way is undone from the journal by recovery;
//! * a file the game itself replaced since (a Dota update, Steam's "verify
//!   files") is never overwritten on restore: it is the game's file now.

use crate::content_store;
use crate::game_deployment::{
    game_dota_folder, publish, reject_link, target_identity, validate_operation_id,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const SCHEMA_VERSION: u32 = 1;
/// Folders below `game/dota` whose files a build may replace.
pub(crate) const FOLDERS: [&str; 3] = ["resource/cursor/", "panorama/fonts/", "itembuilds/"];
pub(crate) const MAX_FILES: usize = 512;
/// The largest of Dota's own fonts (`pw_title.uifont`) is about 14 MB.
pub(crate) const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;
const MAX_NAME_BYTES: usize = 96;
const MAX_RECORD_BYTES: u64 = 4 * 1024 * 1024;

static LOCK: Mutex<()> = Mutex::new(());

/// A stored version of a file: its size and SHA-256 in the content store.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Stored {
    sha256: String,
    bytes: usize,
}

/// One path BetterFy owns: what it put there (`None`: it removed the game's
/// file, as a font pack does with the fonts it does not ship), and the game's
/// file it replaced (`None`: there was none).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OwnedFile {
    installed: Option<Stored>,
    original: Option<Stored>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Ownership {
    schema_version: u32,
    target_identity: String,
    /// The deployment operation of the build these files belong to.
    operation_id: String,
    files: BTreeMap<String, OwnedFile>,
}

/// What one path is before and after the operation (`None`: no file).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Entry {
    path: String,
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
    target_identity: String,
    operation_id: String,
    phase: Phase,
    /// Ownership before and after the operation.
    previous: Option<Ownership>,
    next: Option<Ownership>,
    entries: Vec<Entry>,
}

/// What an install or restore did to the game's loose files.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GameFilesReceipt {
    pub operation_id: String,
    /// Files BetterFy owns after the operation.
    pub files: usize,
    /// Files written or removed.
    pub changed: usize,
    /// Files left alone on restore because the game had replaced them.
    pub left_to_game: usize,
}

/// How BetterFy's files look in the game now.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GameFilesState {
    pub operation_id: String,
    pub files: usize,
    /// Exactly as BetterFy wrote them.
    pub intact: usize,
    /// Changed or put back by the game (an update, a file check).
    pub replaced: usize,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// A path BetterFy may own: directly inside one of [`FOLDERS`], a plain
/// lower-case name with an extension.
pub(crate) fn validate_path(path: &str) -> Result<(), String> {
    let invalid = || Err("game_file_path_invalid".to_string());
    let Some(name) = FOLDERS.iter().find_map(|folder| path.strip_prefix(folder)) else {
        return invalid();
    };
    if name.is_empty()
        || name.len() > MAX_NAME_BYTES
        || name.starts_with('.')
        || name.ends_with('.')
        || !name.contains('.')
        || !name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
        })
    {
        return invalid();
    }
    Ok(())
}

fn records_root(app_data_root: &Path) -> Result<PathBuf, String> {
    reject_link(app_data_root)?;
    let root = app_data_root.join("engine-v1").join("game-files");
    let journals = root.join("journals");
    for path in [app_data_root.join("engine-v1"), root.clone(), journals] {
        reject_link(&path)?;
        fs::create_dir_all(&path).map_err(|_| "game_files_failed".to_string())?;
        reject_link(&path)?;
    }
    Ok(root)
}

fn journal_path(root: &Path, operation_id: &str) -> PathBuf {
    root.join("journals").join(format!("{operation_id}.json"))
}

fn ownership_path(root: &Path) -> PathBuf {
    root.join("ownership.json")
}

fn write_record<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    reject_link(path)?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| "game_files_failed".to_string())?;
    let temporary = path.with_extension("json.tmp");
    reject_link(&temporary)?;
    if temporary.exists() {
        fs::remove_file(&temporary).map_err(|_| "game_files_failed".to_string())?;
    }
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|_| "game_files_failed".to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "game_files_failed".to_string())?;
    drop(file);
    publish(path, &temporary, path.exists()).map_err(|_| "game_files_failed".to_string())
}

fn read_record<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Option<T>, String> {
    reject_link(path)?;
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("game_files_record_invalid".to_string()),
    };
    if !metadata.is_file() || metadata.len() > MAX_RECORD_BYTES {
        return Err("game_files_record_invalid".to_string());
    }
    let bytes = fs::read(path).map_err(|_| "game_files_record_invalid".to_string())?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| "game_files_record_invalid".to_string())
}

fn valid_stored(stored: &Stored) -> bool {
    stored.bytes <= MAX_FILE_BYTES
        && stored.sha256.len() == 64
        && stored
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl Ownership {
    fn validate(&self, identity: &str) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION
            || self.target_identity != identity
            || validate_operation_id(&self.operation_id).is_err()
            || self.files.is_empty()
            || self.files.len() > MAX_FILES
        {
            return Err("game_files_record_invalid".to_string());
        }
        for (path, file) in &self.files {
            validate_path(path)?;
            if (file.installed.is_none() && file.original.is_none())
                || [&file.installed, &file.original]
                    .into_iter()
                    .flatten()
                    .any(|stored| !valid_stored(stored))
            {
                return Err("game_files_record_invalid".to_string());
            }
        }
        Ok(())
    }
}

impl Journal {
    fn validate(&self, identity: &str) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION
            || self.target_identity != identity
            || validate_operation_id(&self.operation_id).is_err()
            || self.entries.len() > 2 * MAX_FILES
        {
            return Err("game_files_record_invalid".to_string());
        }
        for ownership in [&self.previous, &self.next].into_iter().flatten() {
            ownership.validate(identity)?;
        }
        let mut seen = BTreeSet::new();
        for entry in &self.entries {
            validate_path(&entry.path)?;
            if !seen.insert(entry.path.as_str())
                || [&entry.before, &entry.after]
                    .into_iter()
                    .flatten()
                    .any(|stored| !valid_stored(stored))
            {
                return Err("game_files_record_invalid".to_string());
            }
        }
        Ok(())
    }
}

/// What is at a path now: `Ok(None)` when there is no file.
fn current(target: &Path) -> Result<Option<Vec<u8>>, String> {
    reject_link(target)?;
    let metadata = match fs::symlink_metadata(target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("game_files_failed".to_string()),
    };
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES as u64 {
        return Err("game_file_target_unsafe".to_string());
    }
    fs::read(target)
        .map(Some)
        .map_err(|_| "game_files_failed".to_string())
}

fn current_stored(target: &Path) -> Result<Option<Stored>, String> {
    Ok(current(target)?.map(|bytes| Stored {
        sha256: sha256(&bytes),
        bytes: bytes.len(),
    }))
}

/// Keeps bytes in the content store and describes them.
fn keep(app_data_root: &Path, bytes: &[u8]) -> Result<Stored, String> {
    let stored = Stored {
        sha256: sha256(bytes),
        bytes: bytes.len(),
    };
    // The content store holds no empty object; an empty file is described
    // by its size alone.
    if !bytes.is_empty() {
        content_store::store_pinned_resource(app_data_root, bytes.len(), &stored.sha256, bytes)
            .map_err(|_| "game_files_backup_failed".to_string())?;
    }
    Ok(stored)
}

fn read_kept(app_data_root: &Path, stored: &Stored) -> Result<Vec<u8>, String> {
    if stored.bytes == 0 {
        return Ok(Vec::new());
    }
    content_store::read_pinned_resource(app_data_root, stored.bytes, &stored.sha256)
        .map_err(|_| "game_files_backup_missing".to_string())
}

/// The file below `game/dota`, refusing a link at the folder or the file.
/// The folder must exist: BetterFy never creates one.
fn target(game: &Path, path: &str) -> Result<PathBuf, String> {
    validate_path(path)?;
    let target = game.join(path);
    let folder = target
        .parent()
        .ok_or_else(|| "game_file_path_invalid".to_string())?;
    reject_link(folder)?;
    if !folder.is_dir() {
        return Err("game_file_folder_missing".to_string());
    }
    reject_link(&target)?;
    Ok(target)
}

/// Puts `stored` (or no file) at `target`: a new file beside it, then an
/// atomic replace, so the game never sees half a file.
fn place(app_data_root: &Path, target: &Path, stored: Option<&Stored>) -> Result<(), String> {
    match stored {
        None => match fs::remove_file(target) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("game_file_write_failed".to_string()),
        },
        Some(stored) => {
            let bytes = read_kept(app_data_root, stored)?;
            let name = target
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| "game_file_path_invalid".to_string())?;
            let temporary = target.with_file_name(format!(".betterfy-{name}.tmp"));
            reject_link(&temporary)?;
            if temporary.exists() {
                fs::remove_file(&temporary).map_err(|_| "game_file_write_failed".to_string())?;
            }
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .map_err(|_| "game_file_write_failed".to_string())?;
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| "game_file_write_failed".to_string())?;
            drop(file);
            publish(target, &temporary, target.exists()).map_err(|_| {
                let _ = fs::remove_file(&temporary);
                "game_file_write_failed".to_string()
            })
        }
    }
}

/// Puts every entry back to `before` where the file is still what the
/// operation left (or meant to leave), and leaves a file anyone else changed
/// since. Returns (changed, left alone).
fn restore(app_data_root: &Path, game: &Path, entries: &[Entry]) -> Result<(usize, usize), String> {
    let mut changed = 0;
    let mut left = 0;
    for entry in entries {
        let target = target(game, &entry.path)?;
        let now = current_stored(&target)?;
        if now == entry.before {
            continue;
        }
        if now != entry.after {
            left += 1;
            continue;
        }
        place(app_data_root, &target, entry.before.as_ref())?;
        if current_stored(&target)? != entry.before {
            return Err("game_files_restore_failed".to_string());
        }
        changed += 1;
    }
    Ok((changed, left))
}

fn load_ownership(root: &Path, identity: &str) -> Result<Option<Ownership>, String> {
    let ownership: Option<Ownership> = read_record(&ownership_path(root))?;
    if let Some(ownership) = &ownership {
        ownership.validate(identity)?;
    }
    Ok(ownership)
}

fn save_ownership(root: &Path, ownership: Option<&Ownership>) -> Result<(), String> {
    let path = ownership_path(root);
    match ownership {
        Some(ownership) => write_record(&path, ownership),
        None => match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("game_files_failed".to_string()),
        },
    }
}

fn pending_journals(root: &Path, identity: &str) -> Result<Vec<Journal>, String> {
    let mut pending = Vec::new();
    for entry in fs::read_dir(root.join("journals")).map_err(|_| "game_files_failed".to_string())? {
        let path = entry.map_err(|_| "game_files_failed".to_string())?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        if let Some(journal) = read_record::<Journal>(&path)? {
            if journal.target_identity == identity && journal.phase == Phase::Prepared {
                journal.validate(identity)?;
                pending.push(journal);
            }
        }
    }
    Ok(pending)
}

/// Makes the game's loose files what `desired` says for the build of
/// deployment operation `operation_id` (path below `game/dota` to bytes).
/// Paths BetterFy owned that `desired` no longer has get the game's original
/// back. An empty `desired` with nothing owned does nothing.
pub(crate) fn apply(
    app_data_root: &Path,
    dota_root: &Path,
    operation_id: &str,
    desired: &BTreeMap<String, Option<Vec<u8>>>,
) -> Result<GameFilesReceipt, String> {
    validate_operation_id(operation_id).map_err(|_| "game_files_failed".to_string())?;
    if desired.len() > MAX_FILES
        || desired
            .values()
            .flatten()
            .any(|bytes| bytes.len() > MAX_FILE_BYTES)
    {
        return Err("game_files_too_large".to_string());
    }
    for path in desired.keys() {
        validate_path(path)?;
    }
    let _lock = LOCK.lock().map_err(|_| "game_files_locked".to_string())?;
    let root = records_root(app_data_root)?;
    let identity = target_identity(dota_root)?;
    if !pending_journals(&root, &identity)?.is_empty() {
        return Err("game_files_recovery_required".to_string());
    }
    let previous = load_ownership(&root, &identity)?;
    if previous.is_none() && desired.is_empty() {
        return Ok(GameFilesReceipt {
            operation_id: operation_id.to_string(),
            files: 0,
            changed: 0,
            left_to_game: 0,
        });
    }
    let game = game_dota_folder(dota_root)?;
    let paths = previous
        .iter()
        .flat_map(|ownership| ownership.files.keys().cloned())
        .chain(desired.keys().cloned())
        .collect::<BTreeSet<_>>();
    let mut entries = Vec::new();
    let mut next_files = BTreeMap::new();
    for path in &paths {
        let target = target(&game, path)?;
        let now_bytes = current(&target)?;
        let before = match &now_bytes {
            Some(bytes) => Some(keep(app_data_root, bytes)?),
            None => None,
        };
        let owned = previous
            .as_ref()
            .and_then(|ownership| ownership.files.get(path));
        // The game's own file: what BetterFy found the first time, unless the
        // game has replaced BetterFy's file since; then that is the game's.
        let original = match owned {
            Some(owned) if before == owned.installed => owned.original.clone(),
            _ => before.clone(),
        };
        let after = match desired.get(path) {
            Some(wanted) => {
                let stored = match wanted {
                    Some(bytes) => Some(keep(app_data_root, bytes)?),
                    None => None,
                };
                // Removing a file the game does not have changes nothing and
                // is not worth owning.
                if stored.is_some() || original.is_some() {
                    next_files.insert(
                        path.clone(),
                        OwnedFile {
                            installed: stored.clone(),
                            original: original.clone(),
                        },
                    );
                }
                stored
            }
            None => original,
        };
        entries.push(Entry {
            path: path.clone(),
            before,
            after,
        });
    }
    let next = (!next_files.is_empty()).then(|| Ownership {
        schema_version: SCHEMA_VERSION,
        target_identity: identity.clone(),
        operation_id: operation_id.to_string(),
        files: next_files,
    });
    let mut journal = Journal {
        schema_version: SCHEMA_VERSION,
        target_identity: identity.clone(),
        operation_id: operation_id.to_string(),
        phase: Phase::Prepared,
        previous: previous.clone(),
        next: next.clone(),
        entries,
    };
    journal.validate(&identity)?;
    let record = journal_path(&root, operation_id);
    if record.exists() {
        return Err("game_files_failed".to_string());
    }
    write_record(&record, &journal)?;

    let written = (|| -> Result<usize, String> {
        let mut changed = 0;
        for entry in &journal.entries {
            if entry.before == entry.after {
                continue;
            }
            let target = target(&game, &entry.path)?;
            place(app_data_root, &target, entry.after.as_ref())?;
            changed += 1;
        }
        for entry in &journal.entries {
            if current_stored(&target(&game, &entry.path)?)? != entry.after {
                return Err("game_files_verify_failed".to_string());
            }
        }
        Ok(changed)
    })();
    let changed = match written {
        Ok(changed) => changed,
        Err(code) => {
            // Undo what was written, keep the record if that fails too, so
            // recovery can finish it.
            if restore(app_data_root, &game, &journal.entries).is_ok() {
                journal.phase = Phase::Failed;
                let _ = write_record(&record, &journal);
            }
            return Err(code);
        }
    };
    save_ownership(&root, next.as_ref())?;
    journal.phase = Phase::Committed;
    write_record(&record, &journal)?;
    Ok(GameFilesReceipt {
        operation_id: operation_id.to_string(),
        files: next.as_ref().map_or(0, |ownership| ownership.files.len()),
        changed,
        left_to_game: 0,
    })
}

/// Restores what the build of deployment operation `operation_id` changed:
/// every file it wrote goes back to what was there before, unless the game
/// replaced it since. `Ok(None)` when that operation changed no loose file.
pub(crate) fn rollback(
    app_data_root: &Path,
    dota_root: &Path,
    operation_id: &str,
) -> Result<Option<GameFilesReceipt>, String> {
    validate_operation_id(operation_id).map_err(|_| "game_files_failed".to_string())?;
    let _lock = LOCK.lock().map_err(|_| "game_files_locked".to_string())?;
    let root = records_root(app_data_root)?;
    let identity = target_identity(dota_root)?;
    let record = journal_path(&root, operation_id);
    let Some(mut journal) = read_record::<Journal>(&record)? else {
        return Ok(None);
    };
    journal.validate(&identity)?;
    match journal.phase {
        Phase::Committed | Phase::Prepared => {}
        Phase::RolledBack | Phase::Failed => return Ok(None),
    }
    // Only the latest build can be restored; an older one was replaced.
    let owned = load_ownership(&root, &identity)?;
    if journal.phase == Phase::Committed && owned != journal.next {
        return Err("game_files_not_latest".to_string());
    }
    let game = game_dota_folder(dota_root)?;
    let (changed, left) = restore(app_data_root, &game, &journal.entries)?;
    save_ownership(&root, journal.previous.as_ref())?;
    journal.phase = Phase::RolledBack;
    write_record(&record, &journal)?;
    Ok(Some(GameFilesReceipt {
        operation_id: operation_id.to_string(),
        files: journal
            .previous
            .as_ref()
            .map_or(0, |ownership| ownership.files.len()),
        changed,
        left_to_game: left,
    }))
}

/// Finishes every operation that stopped before it committed: its files go
/// back to what was there before. Returns how many operations it finished.
pub(crate) fn recover_pending(app_data_root: &Path, dota_root: &Path) -> Result<usize, String> {
    let _lock = LOCK.lock().map_err(|_| "game_files_locked".to_string())?;
    let root = records_root(app_data_root)?;
    let identity = target_identity(dota_root)?;
    let pending = pending_journals(&root, &identity)?;
    if pending.is_empty() {
        return Ok(0);
    }
    let game = game_dota_folder(dota_root)?;
    for mut journal in pending.iter().cloned() {
        restore(app_data_root, &game, &journal.entries)?;
        save_ownership(&root, journal.previous.as_ref())?;
        journal.phase = Phase::RolledBack;
        write_record(&journal_path(&root, &journal.operation_id), &journal)?;
    }
    Ok(pending.len())
}

/// BetterFy's loose files as they are in the game now; `None` when it owns
/// none.
pub(crate) fn state(
    app_data_root: &Path,
    dota_root: &Path,
) -> Result<Option<GameFilesState>, String> {
    let _lock = LOCK.lock().map_err(|_| "game_files_locked".to_string())?;
    let root = records_root(app_data_root)?;
    let identity = target_identity(dota_root)?;
    let Some(owned) = load_ownership(&root, &identity)? else {
        return Ok(None);
    };
    let game = game_dota_folder(dota_root)?;
    let mut intact = 0;
    for (path, file) in &owned.files {
        if current_stored(&target(&game, path)?)? == file.installed {
            intact += 1;
        }
    }
    Ok(Some(GameFilesState {
        operation_id: owned.operation_id.clone(),
        files: owned.files.len(),
        intact,
        replaced: owned.files.len() - intact,
    }))
}

/// Whether recovery has something to finish.
pub(crate) fn recovery_required(app_data_root: &Path, dota_root: &Path) -> Result<bool, String> {
    let _lock = LOCK.lock().map_err(|_| "game_files_locked".to_string())?;
    let root = records_root(app_data_root)?;
    let identity = target_identity(dota_root)?;
    Ok(!pending_journals(&root, &identity)?.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn root(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "betterfy-game-files-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("root");
        path
    }

    /// A fake installation with Valve's cursor and font folders.
    fn game(root: &Path) -> PathBuf {
        let dota = root.join("dota 2 beta");
        for folder in [
            "game/dota/resource/cursor",
            "game/dota/panorama/fonts",
            "game/dota/itembuilds",
        ] {
            fs::create_dir_all(dota.join(folder)).expect("folder");
        }
        fs::write(dota.join("game/dota/pak01_dir.vpk"), b"official").expect("marker");
        let executable = if cfg!(target_os = "windows") {
            dota.join("game/bin/win64/dota2.exe")
        } else if cfg!(target_os = "macos") {
            dota.join("game/bin/osx64/dota2")
        } else {
            dota.join("game/bin/linuxsteamrt64/dota2")
        };
        fs::create_dir_all(executable.parent().expect("parent")).expect("bin");
        fs::write(executable, b"exe").expect("exe");
        fs::write(
            dota.join("game/dota/resource/cursor/cursor.res"),
            b"valve cursor res",
        )
        .expect("valve file");
        fs::write(
            dota.join("game/dota/panorama/fonts/radiance-regular.otf"),
            b"valve font",
        )
        .expect("valve font");
        dota
    }

    fn files(pairs: &[(&str, &[u8])]) -> BTreeMap<String, Option<Vec<u8>>> {
        pairs
            .iter()
            .map(|(path, bytes)| (path.to_string(), Some(bytes.to_vec())))
            .collect()
    }

    fn read(dota: &Path, path: &str) -> Option<Vec<u8>> {
        fs::read(dota.join("game/dota").join(path)).ok()
    }

    #[test]
    fn paths_stay_inside_the_three_folders() {
        for good in [
            "resource/cursor/cursor.res",
            "resource/cursor/cursor_attack_default_vsz0.bmp",
            "panorama/fonts/radiance-regular.otf",
            "itembuilds/default_antimage.txt",
        ] {
            assert!(validate_path(good).is_ok(), "{good}");
        }
        for bad in [
            "resource/cursor/../../gameinfo.gi",
            "resource/cursor/sub/a.bmp",
            "resource/cursor/",
            "resource/cursor/Cursor.res",
            "resource/cursor/.hidden",
            "resource/cursor/noext",
            "panorama/styles/x.vcss_c",
            "itembuilds\\default.txt",
            "gameinfo.gi",
        ] {
            assert!(validate_path(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn install_replace_and_restore_put_the_game_back_exactly() {
        let base = root("chain");
        let app = base.join("app");
        let dota = game(&base);
        // First build: a cursor pack (one Valve file replaced, one new file).
        let first = apply(
            &app,
            &dota,
            "deploy-op-1-1-1",
            &files(&[
                ("resource/cursor/cursor.res", b"pack cursor res"),
                ("resource/cursor/cursor_new.ani", b"RIFF new"),
            ]),
        )
        .expect("first");
        assert_eq!((first.files, first.changed), (2, 2));
        assert_eq!(
            read(&dota, "resource/cursor/cursor.res").unwrap(),
            b"pack cursor res"
        );
        // Second build: a font instead of the cursor pack.
        let second = apply(
            &app,
            &dota,
            "deploy-op-2-1-1",
            &files(&[("panorama/fonts/radiance-regular.otf", b"pack font")]),
        )
        .expect("second");
        assert_eq!(second.files, 1);
        assert_eq!(
            read(&dota, "resource/cursor/cursor.res").unwrap(),
            b"valve cursor res"
        );
        assert_eq!(read(&dota, "resource/cursor/cursor_new.ani"), None);
        assert_eq!(
            read(&dota, "panorama/fonts/radiance-regular.otf").unwrap(),
            b"pack font"
        );
        // Only the latest build can be restored.
        assert_eq!(
            rollback(&app, &dota, "deploy-op-1-1-1").err().as_deref(),
            Some("game_files_not_latest")
        );
        // Restoring the second build brings the first back.
        rollback(&app, &dota, "deploy-op-2-1-1")
            .expect("rollback second")
            .expect("had files");
        assert_eq!(
            read(&dota, "resource/cursor/cursor.res").unwrap(),
            b"pack cursor res"
        );
        assert_eq!(
            read(&dota, "panorama/fonts/radiance-regular.otf").unwrap(),
            b"valve font"
        );
        // Restoring the first leaves the game exactly as Valve shipped it.
        rollback(&app, &dota, "deploy-op-1-1-1")
            .expect("rollback first")
            .expect("had files");
        assert_eq!(
            read(&dota, "resource/cursor/cursor.res").unwrap(),
            b"valve cursor res"
        );
        assert_eq!(read(&dota, "resource/cursor/cursor_new.ani"), None);
        assert_eq!(state(&app, &dota).expect("state"), None);
        // A build without loose files on top of nothing changes nothing.
        let none = apply(&app, &dota, "deploy-op-3-1-1", &BTreeMap::new()).expect("none");
        assert_eq!((none.files, none.changed), (0, 0));
        assert_eq!(
            rollback(&app, &dota, "deploy-op-3-1-1").expect("no record"),
            None
        );
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn a_font_pack_can_remove_the_games_fonts_and_restore_them() {
        let base = root("remove");
        let app = base.join("app");
        let dota = game(&base);
        let mut desired = files(&[("panorama/fonts/reaver-regular.otf", b"pack reaver")]);
        desired.insert("panorama/fonts/radiance-regular.otf".to_string(), None);
        // A listed font the game does not have is not owned.
        desired.insert("panorama/fonts/thai.uifont".to_string(), None);
        let receipt = apply(&app, &dota, "deploy-op-1-1-1", &desired).expect("install");
        assert_eq!(receipt.files, 2);
        assert_eq!(read(&dota, "panorama/fonts/radiance-regular.otf"), None);
        assert_eq!(
            read(&dota, "panorama/fonts/reaver-regular.otf").unwrap(),
            b"pack reaver"
        );
        let now = state(&app, &dota).expect("state").expect("owned");
        assert_eq!((now.intact, now.replaced), (2, 0));
        rollback(&app, &dota, "deploy-op-1-1-1")
            .expect("rollback")
            .expect("had files");
        assert_eq!(
            read(&dota, "panorama/fonts/radiance-regular.otf").unwrap(),
            b"valve font"
        );
        assert_eq!(read(&dota, "panorama/fonts/reaver-regular.otf"), None);
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn a_file_the_game_replaced_is_left_to_the_game() {
        let base = root("game-replaced");
        let app = base.join("app");
        let dota = game(&base);
        apply(
            &app,
            &dota,
            "deploy-op-1-1-1",
            &files(&[("resource/cursor/cursor.res", b"pack cursor res")]),
        )
        .expect("install");
        // A Dota update ships a new cursor.res.
        fs::write(
            dota.join("game/dota/resource/cursor/cursor.res"),
            b"updated valve res",
        )
        .expect("update");
        let state_now = state(&app, &dota).expect("state").expect("owned");
        assert_eq!((state_now.intact, state_now.replaced), (0, 1));
        let restored = rollback(&app, &dota, "deploy-op-1-1-1")
            .expect("rollback")
            .expect("had files");
        assert_eq!((restored.changed, restored.left_to_game), (0, 1));
        assert_eq!(
            read(&dota, "resource/cursor/cursor.res").unwrap(),
            b"updated valve res"
        );
        // Installing again on top keeps the updated file as the game's own.
        apply(
            &app,
            &dota,
            "deploy-op-2-1-1",
            &files(&[("resource/cursor/cursor.res", b"pack cursor res")]),
        )
        .expect("again");
        rollback(&app, &dota, "deploy-op-2-1-1")
            .expect("rollback")
            .expect("had files");
        assert_eq!(
            read(&dota, "resource/cursor/cursor.res").unwrap(),
            b"updated valve res"
        );
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn an_interrupted_install_is_finished_by_recovery() {
        let base = root("recovery");
        let app = base.join("app");
        let dota = game(&base);
        apply(
            &app,
            &dota,
            "deploy-op-1-1-1",
            &files(&[("resource/cursor/cursor.res", b"first")]),
        )
        .expect("first");
        // Simulate a crash after the journal and half the writes of a second
        // build: write its journal by hand and one of its files.
        let records = records_root(&app).expect("records");
        let identity = target_identity(&dota).expect("identity");
        let previous = load_ownership(&records, &identity).expect("owned");
        let pack = keep(&app, b"second").expect("keep");
        let font = keep(&app, b"second font").expect("keep font");
        let first = keep(&app, b"first").expect("keep first");
        let valve_font = keep(&app, b"valve font").expect("keep valve");
        let journal = Journal {
            schema_version: SCHEMA_VERSION,
            target_identity: identity.clone(),
            operation_id: "deploy-op-2-1-1".to_string(),
            phase: Phase::Prepared,
            previous: previous.clone(),
            next: None,
            entries: vec![
                Entry {
                    path: "panorama/fonts/radiance-regular.otf".to_string(),
                    before: Some(valve_font),
                    after: Some(font),
                },
                Entry {
                    path: "resource/cursor/cursor.res".to_string(),
                    before: Some(first),
                    after: Some(pack),
                },
            ],
        };
        write_record(&journal_path(&records, "deploy-op-2-1-1"), &journal).expect("journal");
        fs::write(dota.join("game/dota/resource/cursor/cursor.res"), b"second").expect("half");
        assert!(recovery_required(&app, &dota).expect("check"));
        assert_eq!(
            apply(&app, &dota, "deploy-op-3-1-1", &BTreeMap::new())
                .err()
                .as_deref(),
            Some("game_files_recovery_required")
        );
        assert_eq!(recover_pending(&app, &dota).expect("recover"), 1);
        assert_eq!(read(&dota, "resource/cursor/cursor.res").unwrap(), b"first");
        assert_eq!(
            read(&dota, "panorama/fonts/radiance-regular.otf").unwrap(),
            b"valve font"
        );
        assert!(!recovery_required(&app, &dota).expect("check"));
        // The first build is still the one BetterFy owns and can restore.
        rollback(&app, &dota, "deploy-op-1-1-1")
            .expect("rollback")
            .expect("had files");
        assert_eq!(
            read(&dota, "resource/cursor/cursor.res").unwrap(),
            b"valve cursor res"
        );
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn missing_folders_and_unsafe_targets_are_refused_before_any_write() {
        let base = root("refused");
        let app = base.join("app");
        let dota = game(&base);
        fs::remove_dir_all(dota.join("game/dota/itembuilds")).expect("remove folder");
        assert_eq!(
            apply(
                &app,
                &dota,
                "deploy-op-1-1-1",
                &files(&[
                    ("resource/cursor/cursor.res", b"pack"),
                    ("itembuilds/default_axe.txt", b"guide"),
                ]),
            )
            .err()
            .as_deref(),
            Some("game_file_folder_missing")
        );
        assert_eq!(
            read(&dota, "resource/cursor/cursor.res").unwrap(),
            b"valve cursor res"
        );
        fs::create_dir_all(dota.join("game/dota/resource/cursor/cursor_x.bmp")).expect("dir");
        assert_eq!(
            apply(
                &app,
                &dota,
                "deploy-op-2-1-1",
                &files(&[("resource/cursor/cursor_x.bmp", b"BM")]),
            )
            .err()
            .as_deref(),
            Some("game_file_target_unsafe")
        );
        assert_eq!(
            apply(
                &app,
                &dota,
                "deploy-op-3-1-1",
                &files(&[("gameinfo.gi", b"x")])
            )
            .err()
            .as_deref(),
            Some("game_file_path_invalid")
        );
        fs::remove_dir_all(base).expect("cleanup");
    }
}

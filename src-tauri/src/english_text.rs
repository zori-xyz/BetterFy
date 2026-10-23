//! The English text layer.
//!
//! Dota ignores language folders that Valve does not ship (`dota_betterfy`,
//! `dota_minify`, ...), so an English-language player cannot load mods through
//! a folder of their own. A real folder works, so BetterFy installs into
//! `game/dota_dutch`, starts Dota with `-language dutch`, and puts the
//! player's own English text into that folder under the Dutch names. The
//! interface then stays English.
//!
//! The layer is derived from the player's own `game/dota/pak01_dir.vpk` and
//! its numbered side archives: every path ending `_english.txt` or
//! `_english.vtt` is read (CRC-checked) and added to the merged VPK with
//! `_english` renamed to `_dutch`. Nothing is downloaded and nothing from the
//! game is kept in the repository. The result is stored in BetterFy's own
//! app-data folder, so later rebuilds of the same reviewed plan are
//! byte-identical until the player prepares the build again.
//!
//! This module reads the game and writes only BetterFy's own files.

use crate::vpk::{self, VpkInput};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(crate) const PACKAGE_ID: &str = "betterfy.english-text";
const GAME_PAK: &str = "game/dota/pak01_dir.vpk";
/// The folder name of the language the layer is written for.
const LOCALE: &str = "dutch";
/// Bounds on the layer. They are estimates, not measured on a real game: a
/// layer beyond them is refused rather than truncated. The merged VPK's own
/// limits (65,536 entries, 512 MiB) apply on top.
pub(crate) const MAX_FILES: usize = 32_768;
pub(crate) const MAX_BYTES: usize = 128 * 1024 * 1024;
const STORE_FOLDER: &str = "english_text";
const SCHEMA_VERSION: u32 = 1;
const MAX_RECORD_BYTES: u64 = 64 * 1024;

/// What the plan shows: how much English text comes from the player's game.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EnglishTextPlanItem {
    pub files: usize,
    pub bytes: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    schema_version: u32,
    vpk_sha256: String,
    files: usize,
    bytes: usize,
}

pub(crate) struct Layer {
    pub resources: BTreeMap<String, Vec<u8>>,
    pub item: EnglishTextPlanItem,
}

/// `dota_english.txt` -> `dota_dutch.txt`; `None` for any other path.
pub(crate) fn dutch_path(path: &str) -> Option<String> {
    for extension in [".txt", ".vtt"] {
        if let Some(stem) = path.strip_suffix(extension) {
            if let Some(base) = stem.strip_suffix("_english") {
                return Some(format!("{base}_{LOCALE}{extension}"));
            }
        }
    }
    None
}

/// Reads the English text of the game at `game_path`.
pub(crate) fn collect(game_path: &Path) -> Result<Layer, String> {
    collect_with_limits(game_path, MAX_FILES, MAX_BYTES)
}

fn collect_with_limits(
    game_path: &Path,
    max_files: usize,
    max_bytes: usize,
) -> Result<Layer, String> {
    let dir_path = game_path.join(GAME_PAK);
    let directory = std::fs::read(&dir_path).map_err(|_| "game_archive_unreadable".to_string())?;
    let wanted = vpk::list_directory_paths(&directory)?
        .into_iter()
        .filter(|path| dutch_path(path).is_some())
        .collect::<BTreeSet<_>>();
    drop(directory);
    if wanted.is_empty() {
        return Err("english_text_missing".to_string());
    }
    if wanted.len() > max_files {
        return Err("english_text_too_large".to_string());
    }
    let found = vpk::read_game_resources(&dir_path, &wanted)?;
    let mut resources = BTreeMap::new();
    let mut bytes = 0usize;
    for (path, data) in found {
        let renamed = dutch_path(&path).ok_or_else(|| "english_text_invalid".to_string())?;
        vpk::validate_path(&renamed).map_err(|_| "english_text_invalid".to_string())?;
        bytes = bytes
            .checked_add(data.len())
            .filter(|total| *total <= max_bytes)
            .ok_or_else(|| "english_text_too_large".to_string())?;
        resources.insert(renamed, data);
    }
    if resources.is_empty() {
        return Err("english_text_missing".to_string());
    }
    Ok(Layer {
        item: EnglishTextPlanItem {
            files: resources.len(),
            bytes,
        },
        resources,
    })
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_atomically(target: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = target.with_extension("tmp");
    std::fs::write(&temporary, bytes).map_err(|_| "english_text_store_failed".to_string())?;
    std::fs::rename(&temporary, target).map_err(|_| "english_text_store_failed".to_string())
}

/// Reads the player's game and stores the layer for the builds that follow.
pub(crate) fn resolve(
    app_data_root: &Path,
    game_path: &Path,
) -> Result<EnglishTextPlanItem, String> {
    store(app_data_root, collect(game_path)?)
}

fn store(app_data_root: &Path, layer: Layer) -> Result<EnglishTextPlanItem, String> {
    let inputs = layer
        .resources
        .iter()
        .map(|(path, bytes)| VpkInput { path, bytes })
        .collect::<Vec<_>>();
    let packed = vpk::build(inputs)?;
    let folder = app_data_root.join(STORE_FOLDER);
    std::fs::create_dir_all(&folder).map_err(|_| "english_text_store_failed".to_string())?;
    write_atomically(&folder.join("layer.vpk"), &packed)?;
    let record = Record {
        schema_version: SCHEMA_VERSION,
        vpk_sha256: sha256(&packed),
        files: layer.item.files,
        bytes: layer.item.bytes,
    };
    let json = serde_json::to_vec(&record).map_err(|_| "english_text_store_failed".to_string())?;
    write_atomically(&folder.join("layer.json"), &json)?;
    Ok(layer.item)
}

/// The stored layer, re-checked against its record.
pub(crate) fn load(app_data_root: &Path) -> Result<Layer, String> {
    let missing = || "english_text_missing".to_string();
    let folder = app_data_root.join(STORE_FOLDER);
    let json_path = folder.join("layer.json");
    let metadata = std::fs::metadata(&json_path).map_err(|_| missing())?;
    if !metadata.is_file() || metadata.len() > MAX_RECORD_BYTES {
        return Err(missing());
    }
    let record: Record = serde_json::from_slice(&std::fs::read(&json_path).map_err(|_| missing())?)
        .map_err(|_| missing())?;
    let packed = std::fs::read(folder.join("layer.vpk")).map_err(|_| missing())?;
    if record.schema_version != SCHEMA_VERSION || sha256(&packed) != record.vpk_sha256 {
        return Err(missing());
    }
    let resources = vpk::extract_embedded(&packed).map_err(|_| missing())?;
    let bytes = resources.values().map(Vec::len).sum::<usize>();
    if resources.len() != record.files
        || bytes != record.bytes
        || resources.len() > MAX_FILES
        || bytes > MAX_BYTES
    {
        return Err(missing());
    }
    Ok(Layer {
        item: EnglishTextPlanItem {
            files: resources.len(),
            bytes,
        },
        resources,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::fs;

    /// Writes a synthetic game under `root`: `game/dota/pak01_dir.vpk` plus
    /// numbered side archives, as Valve lays them out.
    pub(crate) fn game(root: &Path, entries: &[(&str, &[u8], u16)]) {
        fs::create_dir_all(root.join("game/dota")).expect("dirs");
        vpk::write_test_game_pak(&root.join("game/dota"), entries);
    }

    pub(crate) fn sample() -> Vec<(&'static str, &'static [u8], u16)> {
        vec![
            (
                "resource/localization/dota_english.txt",
                b"\"english\" one",
                0,
            ),
            (
                "resource/localization/abilities_english.txt",
                b"abilities",
                1,
            ),
            (
                "resource/subtitles/subtitles_english.vtt",
                b"WEBVTT en",
                0x7fff,
            ),
            ("resource/localization/dota_russian.txt", b"russian", 0),
            ("resource/localization/dota_dutch.txt", b"real dutch", 1),
            (
                "resource/localization/english_notes.txt",
                b"not a locale file",
                0,
            ),
            (
                "resource/localization/dota_english.bin",
                b"wrong extension",
                1,
            ),
            ("materials/skin.vmat_c", b"unrelated", 0),
        ]
    }

    pub(crate) fn temp(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "betterfy-english-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn only_english_text_and_subtitle_paths_are_renamed() {
        assert_eq!(
            dutch_path("resource/localization/dota_english.txt").as_deref(),
            Some("resource/localization/dota_dutch.txt")
        );
        assert_eq!(
            dutch_path("subtitles_english.vtt").as_deref(),
            Some("subtitles_dutch.vtt")
        );
        for other in [
            "dota_russian.txt",
            "dota_english.bin",
            "english.txt",
            "dota_english_extra.txt",
            "dota_english.txt.bak",
        ] {
            assert_eq!(dutch_path(other), None, "{other}");
        }
    }

    #[test]
    fn reads_across_the_directory_file_and_side_archives() {
        let root = temp("collect");
        game(&root, &sample());
        let layer = collect(&root).expect("layer");
        assert_eq!(
            layer
                .resources
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "resource/localization/abilities_dutch.txt",
                "resource/localization/dota_dutch.txt",
                "resource/subtitles/subtitles_dutch.vtt",
            ]
        );
        assert_eq!(
            layer.resources["resource/localization/dota_dutch.txt"],
            b"\"english\" one"
        );
        assert_eq!(
            layer.resources["resource/localization/abilities_dutch.txt"],
            b"abilities"
        );
        assert_eq!(layer.item.files, 3);
        assert_eq!(layer.item.bytes, 13 + 9 + 9);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn the_same_game_gives_the_same_stored_layer() {
        let first = temp("det-a");
        let second = temp("det-b");
        game(&first, &sample());
        let mut reordered = sample();
        reordered.reverse();
        game(&second, &reordered);
        let data_a = temp("det-data-a");
        let data_b = temp("det-data-b");
        resolve(&data_a, &first).expect("a");
        resolve(&data_b, &second).expect("b");
        assert_eq!(
            fs::read(data_a.join("english_text/layer.vpk")).unwrap(),
            fs::read(data_b.join("english_text/layer.vpk")).unwrap()
        );
        let loaded = load(&data_a).expect("load");
        assert_eq!(loaded.resources, collect(&first).unwrap().resources);
        for root in [first, second, data_a, data_b] {
            fs::remove_dir_all(root).ok();
        }
    }

    #[test]
    fn a_changed_game_changes_the_layer() {
        let before = temp("upd-a");
        let after = temp("upd-b");
        game(&before, &sample());
        let mut updated = sample();
        updated[0].1 = b"\"english\" two";
        game(&after, &updated);
        let one = collect(&before).unwrap();
        let two = collect(&after).unwrap();
        assert_ne!(one.resources, two.resources);
        fs::remove_dir_all(before).ok();
        fs::remove_dir_all(after).ok();
    }

    #[test]
    fn a_layer_over_the_limits_is_refused_not_cut() {
        let root = temp("limits");
        game(&root, &sample());
        assert_eq!(
            collect_with_limits(&root, 2, MAX_BYTES).err().as_deref(),
            Some("english_text_too_large")
        );
        assert_eq!(
            collect_with_limits(&root, MAX_FILES, 30).err().as_deref(),
            Some("english_text_too_large")
        );
        assert!(collect_with_limits(&root, 3, 31).is_ok());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn a_game_without_english_text_or_a_pak_is_an_error() {
        let root = temp("none");
        game(&root, &[("materials/skin.vmat_c", b"x", 0)]);
        assert_eq!(
            collect(&root).err().as_deref(),
            Some("english_text_missing")
        );
        let empty = temp("nopak");
        assert_eq!(
            collect(&empty).err().as_deref(),
            Some("game_archive_unreadable")
        );
        assert_eq!(load(&empty).err().as_deref(), Some("english_text_missing"));
        fs::remove_dir_all(root).ok();
        fs::remove_dir_all(empty).ok();
    }

    #[test]
    fn a_tampered_store_is_refused() {
        let root = temp("tamper-game");
        let data = temp("tamper-data");
        game(&root, &sample());
        resolve(&data, &root).expect("store");
        let packed = data.join("english_text/layer.vpk");
        let mut bytes = fs::read(&packed).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        fs::write(&packed, bytes).unwrap();
        assert_eq!(load(&data).err().as_deref(), Some("english_text_missing"));
        fs::remove_dir_all(root).ok();
        fs::remove_dir_all(data).ok();
    }
}

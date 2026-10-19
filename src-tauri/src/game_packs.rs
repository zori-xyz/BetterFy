//! Cursor and font packs of the catalog: the game's own loose files, replaced
//! the way the packs' own guides say.
//!
//! A cursor pack is a ZIP of `<Name>/cursor/*` (the `.ani`, `.bmp` and
//! `cursor.res` files of `game/dota/resource/cursor`) and an `Install.bat`
//! that copies them over the game's. A font pack is a ZIP of
//! `<Name>/assets/custom/*` (the pack's fonts) and `<Name>/assets/default/*`
//! (a copy of the game's own `game/dota/panorama/fonts`); its guide says to
//! delete the game's fonts and put the pack's in their place. BetterFy does
//! exactly that through `game_files`, so every replaced or removed file of the
//! game is backed up and comes back on restore. Nothing of the pack is ever
//! run: the scripts are not even extracted.
//!
//! An item installs only when it has an entry in the allowlist compiled into
//! the app (`src-tauri/gamefiles/manifests.json`, written by
//! `generate_game_packs` from a local archive cache). The entry pins the ZIP
//! and every file taken from it by size and SHA-256, and names where each
//! goes; for a font pack, also which of the game's fonts it removes (the
//! pack's own list of the game's fonts, minus the ones it replaces). Every
//! file's content is checked again on each build (`check_content`).

use crate::content_store;
use crate::game_files;
use crate::package_registry::LocalizedName;
use crate::wardrobe::{archive_tail, TRUSTED_ARCHIVE_PREFIXES};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::OnceLock;

const MANIFESTS: &str = include_str!("../gamefiles/manifests.json");
#[cfg(test)]
const INDEX: &str = include_str!("../gamefiles/index.json");

/// IDs share the package ID shape (`namespace.name`) under their own
/// namespace, which `package_registry` reserves.
pub(crate) const NAMESPACE: &str = "gamefiles";
const SCHEMA_VERSION: u32 = 1;
const MAX_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ZIP_ENTRIES: usize = 256;
const MAX_CURSOR_RES_BYTES: usize = 256 * 1024;
const MAX_ITEM_BUILD_BYTES: usize = 64 * 1024;
/// OpenDotaGuides publishes its guides as a dated release asset.
const GUIDES_RELEASE_PREFIX: &str = "https://github.com/Egezenn/OpenDotaGuides/releases/download/";
const GUIDES_ASSET: &str = "/itembuilds.zip";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PackCategory {
    Cursors,
    Fonts,
    /// Item build guides (`default_<hero>.txt`), as Minify's OpenDotaGuides
    /// mod installs them.
    ItemBuilds,
}

impl PackCategory {
    fn folder(self) -> &'static str {
        match self {
            Self::Cursors => "resource/cursor/",
            Self::Fonts => "panorama/fonts/",
            Self::ItemBuilds => "itembuilds/",
        }
    }

    fn catalog_prefix(self) -> &'static str {
        match self {
            Self::Cursors => "cursors-",
            Self::Fonts => "fonts-",
            Self::ItemBuilds => "minify-opendotaguides",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackSource {
    pub urls: Vec<String>,
    pub catalog_snapshot: String,
    pub permission: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackArchive {
    pub zip_bytes: u64,
    pub zip_sha256: String,
    /// Entries of the ZIP, folders included.
    pub entries: usize,
    /// The pack's own install scripts (`Install.bat`), by exact entry name.
    /// They are part of the pinned ZIP and are never extracted or run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scripts: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackFile {
    /// The entry in the ZIP.
    pub from: String,
    /// Where it goes, below `game/dota`.
    pub to: String,
    pub bytes: usize,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackManifest {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub catalog_id: String,
    pub category: PackCategory,
    pub name: LocalizedName,
    pub author: String,
    pub distribution: String,
    pub source: PackSource,
    pub archive: PackArchive,
    pub files: Vec<PackFile>,
    /// The game's files the pack removes (font packs only).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removes: Vec<String>,
}

/// The interface's copy of one manifest (`index.json`).
#[cfg(test)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IndexEntry {
    pub id: String,
    pub catalog_id: String,
    pub kind: String,
    pub category: PackCategory,
    pub name: LocalizedName,
    pub author: String,
    pub download_bytes: u64,
    pub install_files: usize,
    pub install_bytes: u64,
}

#[cfg(test)]
pub(crate) fn index_entry(manifest: &PackManifest) -> IndexEntry {
    IndexEntry {
        id: manifest.id.clone(),
        catalog_id: manifest.catalog_id.clone(),
        kind: manifest.kind.clone(),
        category: manifest.category,
        name: manifest.name.clone(),
        author: manifest.author.clone(),
        download_bytes: manifest.archive.zip_bytes,
        install_files: manifest.files.len(),
        install_bytes: manifest.files.iter().map(|file| file.bytes as u64).sum(),
    }
}

/// One pack in a reviewed build.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackPlanItem {
    pub package_id: String,
    pub catalog_id: String,
    pub category: PackCategory,
    /// Files the pack puts into the game.
    pub files: usize,
    pub bytes: u64,
    /// Files of the game it removes.
    pub removes: usize,
}

pub(crate) struct LoadedPack {
    /// Path below `game/dota` to its bytes, or `None` for a removed file.
    pub files: BTreeMap<String, Option<Vec<u8>>>,
    pub item: PackPlanItem,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// The one URL of a guides pack: a dated release (`YYYY-MM-DD`) of
/// OpenDotaGuides.
fn valid_guides_url(urls: &[String]) -> bool {
    let [url] = urls else {
        return false;
    };
    url.strip_prefix(GUIDES_RELEASE_PREFIX)
        .and_then(|rest| rest.strip_suffix(GUIDES_ASSET))
        .is_some_and(|tag| {
            tag.len() == 10
                && tag.bytes().enumerate().all(|(index, byte)| {
                    if index == 4 || index == 7 {
                        byte == b'-'
                    } else {
                        byte.is_ascii_digit()
                    }
                })
        })
}

fn valid_archive_urls(urls: &[String]) -> bool {
    if urls.is_empty() || urls.len() > TRUSTED_ARCHIVE_PREFIXES.len() {
        return false;
    }
    let Some(first) = archive_tail(&urls[0], ".zip") else {
        return false;
    };
    let mut prefixes = BTreeSet::new();
    urls.iter().all(|url| {
        archive_tail(url, ".zip") == Some(first)
            && TRUSTED_ARCHIVE_PREFIXES
                .iter()
                .find(|prefix| url.starts_with(**prefix))
                .is_some_and(|prefix| prefixes.insert(*prefix))
    })
}

/// A ZIP entry name the pack's files may come from: relative, no traversal,
/// no backslash or drive, printable.
fn valid_entry_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 260
        && !name.starts_with('/')
        && !name.contains(['\\', ':'])
        && !name.chars().any(char::is_control)
        && name
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn extension(path: &str) -> &str {
    path.rsplit_once('.').map_or("", |(_, extension)| extension)
}

/// What a file of a pack must look like for where it goes. Cursors are
/// Windows animated cursors (`RIFF`/`ACON`), bitmaps (`BM`) and the
/// `cursor.res` text that names them; fonts are OpenType or TrueType files.
/// Nothing else is installed.
pub(crate) fn check_content(to: &str, bytes: &[u8]) -> Result<(), String> {
    let invalid = || Err("game_pack_content_invalid".to_string());
    if bytes.is_empty() || bytes.len() > game_files::MAX_FILE_BYTES {
        return invalid();
    }
    if let Some(name) = to.strip_prefix(PackCategory::Cursors.folder()) {
        return match extension(name) {
            "ani" if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"ACON" => {
                Ok(())
            }
            "bmp" if bytes.len() >= 14 && &bytes[..2] == b"BM" => Ok(()),
            "res"
                if name == "cursor.res"
                    && bytes.len() <= MAX_CURSOR_RES_BYTES
                    && std::str::from_utf8(bytes).is_ok_and(|text| {
                        !text.chars().any(|c| c.is_control() && !c.is_whitespace())
                    }) =>
            {
                Ok(())
            }
            _ => invalid(),
        };
    }
    if let Some(name) = to.strip_prefix(PackCategory::ItemBuilds.folder()) {
        let hero = name
            .strip_prefix("default_")
            .and_then(|rest| rest.strip_suffix(".txt"));
        return match (hero, std::str::from_utf8(bytes)) {
            (Some(hero), Ok(text))
                if !hero.is_empty()
                    && hero.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
                    })
                    && bytes.len() <= MAX_ITEM_BUILD_BYTES
                    && !text.chars().any(|c| c.is_control() && !c.is_whitespace())
                    && text
                        .trim_start_matches('\u{feff}')
                        .trim_start()
                        .starts_with("\"itembuilds\"") =>
            {
                Ok(())
            }
            _ => invalid(),
        };
    }
    if let Some(name) = to.strip_prefix(PackCategory::Fonts.folder()) {
        let sfnt = bytes.len() >= 12 && matches!(&bytes[..4], [0, 1, 0, 0] | b"OTTO" | b"true");
        return if matches!(extension(name), "otf" | "ttf") && sfnt {
            Ok(())
        } else {
            invalid()
        };
    }
    invalid()
}

pub(crate) fn validate(manifest: &PackManifest) -> Result<(), String> {
    let invalid = |reason: &str| Err(format!("game_pack_invalid:{}:{reason}", manifest.id));
    if manifest.schema_version != SCHEMA_VERSION || manifest.kind != "gamefiles" {
        return invalid("schema");
    }
    if manifest.distribution != "internal_pilot" {
        return invalid("distribution");
    }
    let category = manifest.category;
    if manifest.id != format!("{NAMESPACE}.{}", manifest.catalog_id)
        || !slug(&manifest.catalog_id)
        || !manifest.catalog_id.starts_with(category.catalog_prefix())
    {
        return invalid("id");
    }
    if manifest.name.ru.trim().is_empty()
        || manifest.name.en.trim().is_empty()
        || manifest.author.trim().is_empty()
        || manifest.source.permission.trim().is_empty()
        || manifest.source.catalog_snapshot.trim().is_empty()
    {
        return invalid("name");
    }
    let urls_ok = match category {
        PackCategory::ItemBuilds => valid_guides_url(&manifest.source.urls),
        _ => valid_archive_urls(&manifest.source.urls),
    };
    if !urls_ok {
        return invalid("source");
    }
    let archive = &manifest.archive;
    if archive.zip_bytes == 0
        || archive.zip_bytes > MAX_ARCHIVE_BYTES
        || archive.entries == 0
        || archive.entries > MAX_ZIP_ENTRIES
        || !valid_sha256(&archive.zip_sha256)
        || archive.scripts.len() > 4
        || archive.scripts.iter().any(|name| {
            !valid_entry_name(name)
                || !name.to_ascii_lowercase().ends_with(".bat")
                || manifest.files.iter().any(|file| file.from == *name)
        })
    {
        return invalid("archive");
    }
    if manifest.files.is_empty() || manifest.files.len() > game_files::MAX_FILES {
        return invalid("files");
    }
    let mut targets = BTreeSet::new();
    let mut sources = BTreeSet::new();
    for file in &manifest.files {
        if !valid_entry_name(&file.from)
            || game_files::validate_path(&file.to).is_err()
            || !file.to.starts_with(category.folder())
            || file.bytes == 0
            || file.bytes > game_files::MAX_FILE_BYTES
            || !valid_sha256(&file.sha256)
            || !targets.insert(file.to.as_str())
            || !sources.insert(file.from.as_str())
        {
            return invalid("file");
        }
    }
    if category != PackCategory::Fonts && !manifest.removes.is_empty() {
        return invalid("removes");
    }
    let mut removed = BTreeSet::new();
    for path in &manifest.removes {
        if game_files::validate_path(path).is_err()
            || !path.starts_with(category.folder())
            || targets.contains(path.as_str())
            || !removed.insert(path.as_str())
        {
            return invalid("removes");
        }
    }
    if manifest.files.len() + manifest.removes.len() > game_files::MAX_FILES {
        return invalid("files");
    }
    Ok(())
}

fn parse_all(source: &str) -> Result<Vec<PackManifest>, String> {
    let manifests: Vec<PackManifest> =
        serde_json::from_str(source).map_err(|error| format!("game_pack_invalid:parse:{error}"))?;
    let mut ids = BTreeSet::new();
    for manifest in &manifests {
        validate(manifest)?;
        if !ids.insert(manifest.id.as_str()) {
            return Err(format!("game_pack_invalid:{}:duplicate", manifest.id));
        }
    }
    Ok(manifests)
}

struct Registry {
    manifests: Vec<PackManifest>,
    by_id: BTreeMap<String, usize>,
}

fn embedded() -> &'static Result<Registry, String> {
    static EMBEDDED: OnceLock<Result<Registry, String>> = OnceLock::new();
    EMBEDDED.get_or_init(|| {
        let manifests = parse_all(MANIFESTS)?;
        let mut by_id = BTreeMap::new();
        for (index, manifest) in manifests.iter().enumerate() {
            by_id.insert(manifest.id.clone(), index);
            by_id.insert(manifest.catalog_id.clone(), index);
        }
        Ok(Registry { manifests, by_id })
    })
}

#[cfg(test)]
pub(crate) fn manifests() -> Result<&'static [PackManifest], String> {
    embedded()
        .as_ref()
        .map(|registry| registry.manifests.as_slice())
        .map_err(|_| "game_pack_registry_invalid".to_string())
}

/// Resolves the engine ID (`gamefiles.cursors-ti8-cursor`) or the catalog ID
/// (`cursors-ti8-cursor`). `Ok(None)`: not a pack.
pub(crate) fn find(id: &str) -> Result<Option<&'static PackManifest>, String> {
    #[cfg(test)]
    if let Some(item) = TEST_ITEMS
        .lock()
        .expect("test registry")
        .iter()
        .find(|manifest| manifest.id == id || manifest.catalog_id == id)
    {
        return Ok(Some(*item));
    }
    let registry = embedded()
        .as_ref()
        .map_err(|_| "game_pack_registry_invalid".to_string())?;
    Ok(registry
        .by_id
        .get(id)
        .map(|index| &registry.manifests[*index]))
}

/// Whether a signed package catalog would reuse this pack's catalog ID.
pub(crate) fn catalog_id_taken(catalog_id: &str) -> bool {
    embedded().as_ref().is_ok_and(|registry| {
        registry
            .by_id
            .get(catalog_id)
            .is_some_and(|index| registry.manifests[*index].catalog_id == catalog_id)
    })
}

#[cfg(test)]
static TEST_ITEMS: std::sync::Mutex<Vec<&'static PackManifest>> = std::sync::Mutex::new(Vec::new());

#[cfg(test)]
pub(crate) fn register_for_test(manifest: PackManifest) -> &'static PackManifest {
    validate(&manifest).expect("valid test pack");
    let leaked: &'static PackManifest = Box::leak(Box::new(manifest));
    TEST_ITEMS.lock().expect("test registry").push(leaked);
    leaked
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Reads every pinned file out of the pack's ZIP and checks it. The ZIP is
/// inspected first (no traversal, links, encrypted or executable entries,
/// bounded sizes) and must have exactly the pinned number of entries; each
/// file is read through a hard length bound.
pub(crate) fn extract_pinned_files(
    zip_bytes: &[u8],
    manifest: &PackManifest,
) -> Result<Vec<Vec<u8>>, String> {
    if zip_bytes.len() as u64 != manifest.archive.zip_bytes
        || sha256_hex(zip_bytes) != manifest.archive.zip_sha256
    {
        return Err("game_pack_hash_mismatch".to_string());
    }
    let report = crate::archive_inspector::inspect_zip_bytes_tolerating(
        zip_bytes,
        &manifest.archive.scripts,
    )?;
    if report.entries != manifest.archive.entries {
        return Err("game_pack_unexpected".to_string());
    }
    let mut archive = zip::read::ZipArchive::new(Cursor::new(zip_bytes))
        .map_err(|_| "archive_format_invalid".to_string())?;
    manifest
        .files
        .iter()
        .map(|file| {
            let mut entry = archive
                .by_name(&file.from)
                .map_err(|_| "game_pack_unexpected".to_string())?;
            if !entry.is_file() || entry.size() != file.bytes as u64 {
                return Err("game_pack_unexpected".to_string());
            }
            let mut bytes = Vec::with_capacity(file.bytes);
            entry
                .by_ref()
                .take(file.bytes as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "game_pack_unreadable".to_string())?;
            if bytes.len() != file.bytes || sha256_hex(&bytes) != file.sha256 {
                return Err("game_pack_hash_mismatch".to_string());
            }
            check_content(&file.to, &bytes)?;
            Ok(bytes)
        })
        .collect()
}

/// Makes sure every file of the pack is in the content store, downloading
/// and checking the pinned ZIP when one is missing. Only the checked files
/// are stored, never the ZIP or its scripts.
pub(crate) fn acquire(
    app_data_root: &Path,
    manifest: &PackManifest,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    let stored = manifest.files.iter().all(|file| {
        content_store::read_pinned_resource(app_data_root, file.bytes, &file.sha256).is_ok()
    });
    if stored {
        return Ok(());
    }
    let zip = crate::remote_intake::fetch_pinned_game_pack(manifest, cancelled)?;
    let files = extract_pinned_files(&zip, manifest)?;
    for (file, bytes) in manifest.files.iter().zip(files) {
        content_store::store_pinned_resource(app_data_root, file.bytes, &file.sha256, &bytes)?;
    }
    Ok(())
}

/// The pack's files from the content store, checked again, and its removals.
pub(crate) fn load(app_data_root: &Path, manifest: &PackManifest) -> Result<LoadedPack, String> {
    let mut files = BTreeMap::new();
    for file in &manifest.files {
        let bytes = content_store::read_pinned_resource(app_data_root, file.bytes, &file.sha256)
            .map_err(|_| "tree_resource_unverified".to_string())?;
        check_content(&file.to, &bytes)?;
        files.insert(file.to.clone(), Some(bytes));
    }
    for path in &manifest.removes {
        files.insert(path.clone(), None);
    }
    Ok(LoadedPack {
        files,
        item: PackPlanItem {
            package_id: manifest.id.clone(),
            catalog_id: manifest.catalog_id.clone(),
            category: manifest.category,
            files: manifest.files.len(),
            bytes: manifest.files.iter().map(|file| file.bytes as u64).sum(),
            removes: manifest.removes.len(),
        },
    })
}

/// The text a pack puts into the build's VPK so the reviewed build is bound
/// to the exact files it changes in the game (`betterfy/<id>.txt`); the game
/// never reads it.
pub(crate) fn marker(manifest: &PackManifest) -> (String, Vec<u8>) {
    let mut text = format!("{}\n", manifest.id);
    for file in &manifest.files {
        text.push_str(&format!(
            "write {} {} {}\n",
            file.to, file.bytes, file.sha256
        ));
    }
    for path in &manifest.removes {
        text.push_str(&format!("remove {path}\n"));
    }
    (
        format!("betterfy/{}.txt", manifest.catalog_id),
        text.into_bytes(),
    )
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use std::io::Write;

    /// A cursor pack ZIP with a `cursor.res`, one `.ani`, one `.bmp`, a guide
    /// and an install script, and its manifest.
    pub(crate) fn cursor_pack(slug: &str, label: &[u8]) -> (PackManifest, Vec<u8>) {
        let res = [b"cursor ".as_slice(), label].concat();
        let ani = [b"RIFF\x10\0\0\0ACON".as_slice(), label].concat();
        let bmp = [b"BM\0\0\0\0\0\0\0\0\0\0\0\0".as_slice(), label].concat();
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        let entries: [(&str, &[u8]); 5] = [
            ("Pack/cursor/cursor.res", &res),
            ("Pack/cursor/cursor_default.ani", &ani),
            ("Pack/cursor/cursor_default_vsz0.bmp", &bmp),
            ("Pack/guide.txt", b"guide"),
            ("Pack/Install.bat", b"@echo off"),
        ];
        for (name, bytes) in entries {
            zip.start_file(name, options).expect("entry");
            zip.write_all(bytes).expect("write");
        }
        let zip = zip.finish().expect("zip").into_inner();
        let file = |from: &str, to: &str, bytes: &[u8]| PackFile {
            from: from.to_string(),
            to: to.to_string(),
            bytes: bytes.len(),
            sha256: sha256_hex(bytes),
        };
        let catalog_id = format!("cursors-{slug}");
        let manifest = PackManifest {
            schema_version: SCHEMA_VERSION,
            kind: "gamefiles".to_string(),
            id: format!("{NAMESPACE}.{catalog_id}"),
            catalog_id,
            category: PackCategory::Cursors,
            name: LocalizedName {
                ru: "Test Cursor".to_string(),
                en: "Test Cursor".to_string(),
            },
            author: "tester".to_string(),
            distribution: "internal_pilot".to_string(),
            source: PackSource {
                urls: vec![format!(
                    "{}cursors/Test%20Cursor.zip",
                    TRUSTED_ARCHIVE_PREFIXES[0]
                )],
                catalog_snapshot: "test".to_string(),
                permission: "founder_reported_author_consent".to_string(),
            },
            archive: PackArchive {
                zip_bytes: zip.len() as u64,
                zip_sha256: sha256_hex(&zip),
                entries: 5,
                scripts: vec!["Pack/Install.bat".to_string()],
            },
            files: vec![
                file("Pack/cursor/cursor.res", "resource/cursor/cursor.res", &res),
                file(
                    "Pack/cursor/cursor_default.ani",
                    "resource/cursor/cursor_default.ani",
                    &ani,
                ),
                file(
                    "Pack/cursor/cursor_default_vsz0.bmp",
                    "resource/cursor/cursor_default_vsz0.bmp",
                    &bmp,
                ),
            ],
            removes: Vec::new(),
        };
        (manifest, zip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_allowlist_and_its_interface_copy_agree() {
        let manifests = manifests().expect("valid allowlist");
        let index: Vec<IndexEntry> = serde_json::from_str(INDEX).expect("index json");
        let derived = manifests.iter().map(index_entry).collect::<Vec<_>>();
        assert_eq!(index, derived);
        for manifest in manifests {
            assert!(!crate::wardrobe::catalog_id_taken(&manifest.catalog_id));
            assert!(!crate::map_terrain::catalog_id_taken(&manifest.catalog_id));
        }
    }

    #[test]
    fn content_checks_follow_the_target_folder() {
        assert!(check_content("resource/cursor/a.ani", b"RIFF\0\0\0\0ACONxx").is_ok());
        assert!(check_content("resource/cursor/a.ani", b"MZ\0\0\0\0\0\0ACONxx").is_err());
        assert!(check_content("resource/cursor/a.bmp", b"BM0123456789abcd").is_ok());
        assert!(check_content("resource/cursor/cursor.res", b"\"cursors\" {}\r\n").is_ok());
        assert!(check_content("resource/cursor/other.res", b"\"cursors\" {}").is_err());
        assert!(check_content("resource/cursor/cursor.res", b"\x00\x01binary").is_err());
        assert!(check_content("resource/cursor/a.png", b"\x89PNG....").is_err());
        assert!(check_content("panorama/fonts/a.otf", b"OTTO\0\0\0\0\0\0\0\0").is_ok());
        assert!(check_content("panorama/fonts/a.ttf", b"\0\x01\0\0\0\0\0\0\0\0\0\0").is_ok());
        assert!(check_content("panorama/fonts/a.otf", b"wOFF\0\0\0\0\0\0\0\0").is_err());
        assert!(check_content("panorama/fonts/a.uifont", b"OTTO\0\0\0\0\0\0\0\0").is_err());
        assert!(check_content("itembuilds/default_axe.txt", b"x").is_err());
        assert!(check_content(
            "itembuilds/default_axe.txt",
            b"\"itembuilds\"\r\n{\r\n\t\"Author\" \"x\"\r\n}\r\n"
        )
        .is_ok());
        assert!(check_content("itembuilds/axe.txt", b"\"itembuilds\" {}").is_err());
        assert!(check_content("itembuilds/default_axe.txt", b"\"other\" {}").is_err());
        assert!(valid_guides_url(&[format!(
            "{GUIDES_RELEASE_PREFIX}2026-10-19{GUIDES_ASSET}"
        )]));
        assert!(!valid_guides_url(&[format!(
            "{GUIDES_RELEASE_PREFIX}latest{GUIDES_ASSET}"
        )]));
        assert!(!valid_guides_url(&[format!(
            "https://github.com/other/OpenDotaGuides/releases/download/2026-10-19{GUIDES_ASSET}"
        )]));
    }

    #[test]
    fn a_pack_is_extracted_checked_and_refused_when_anything_differs() {
        let (manifest, zip) = fixtures::cursor_pack("extract", b"one");
        validate(&manifest).expect("valid");
        let files = extract_pinned_files(&zip, &manifest).expect("extracts");
        assert_eq!(files.len(), 3);
        let mut tampered = zip.clone();
        let last = tampered.len() - 30;
        tampered[last] ^= 1;
        assert!(extract_pinned_files(&tampered, &manifest).is_err());
        let mut wrong = manifest.clone();
        wrong.files[0].sha256 = "0".repeat(64);
        assert_eq!(
            extract_pinned_files(&zip, &wrong).err().as_deref(),
            Some("game_pack_hash_mismatch")
        );
        let (marker_path, text) = marker(&manifest);
        assert_eq!(marker_path, "betterfy/cursors-extract.txt");
        assert!(String::from_utf8(text)
            .unwrap()
            .contains("write resource/cursor/cursor.res"));
    }

    #[test]
    fn hostile_manifests_are_refused() {
        let (good, _) = fixtures::cursor_pack("hostile", b"x");
        type Edit = Box<dyn Fn(&mut PackManifest)>;
        let cases: Vec<Edit> = vec![
            Box::new(|m| m.files[0].to = "resource/cursor/../../gameinfo.gi".to_string()),
            Box::new(|m| m.files[0].to = "panorama/fonts/a.otf".to_string()),
            Box::new(|m| m.files[0].to = "gameinfo.gi".to_string()),
            Box::new(|m| m.files[0].from = "../escape".to_string()),
            Box::new(|m| m.files[0].from = "C:/x".to_string()),
            Box::new(|m| m.files[1].to = m.files[0].to.clone()),
            Box::new(|m| m.removes = vec!["resource/cursor/cursor.res".to_string()]),
            Box::new(|m| m.source.urls = vec!["https://evil.example/a.zip".to_string()]),
            Box::new(|m| m.id = "gamefiles.other".to_string()),
            Box::new(|m| m.catalog_id = "fonts-hostile".to_string()),
            Box::new(|m| m.files[0].sha256 = "00".to_string()),
        ];
        for (index, edit) in cases.iter().enumerate() {
            let mut manifest = good.clone();
            edit(&mut manifest);
            assert!(validate(&manifest).is_err(), "case {index} accepted");
        }
    }
}

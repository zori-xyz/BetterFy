//! Map terrains: whole-map packages installed through the same verified
//! pipeline as the hero skins, but as their own file.
//!
//! A "standard" terrain of the Dota2PornFx catalog is a ZIP that holds
//! `maps/dota.vpk` (a complete Valve map package, 34-53 MB) and a `Guide.txt`.
//! Upstream's install guide says to put the `maps` folder into the language
//! folder, `game/dota_<language>/maps/dota.vpk`, without moving or renaming
//! `dota.vpk`. BetterFy installs that one file, byte for byte, as a second
//! owned target of the deployment transaction (see `game_deployment.rs`); it is
//! never merged into the `pakNN_dir.vpk`.
//!
//! An item is installable only when it has an entry in the allowlist compiled
//! into the app (`src-tauri/maps/manifests.json`, one generated JSON array; a
//! signed catalog cannot add one). An entry pins
//!
//! * the downloaded ZIP and the `maps/dota.vpk` inside it by size and SHA-256,
//!   the `Guide.txt` next to it, and the ordered list of mirrors the ZIP may
//!   come from (the same trusted hosts as the skins);
//! * what the map analyzer (`map_package.rs`) found in the VPK: entry count,
//!   file types, contents identity. Every build re-runs the analyzer on the
//!   stored bytes and must reproduce the audit field for field.
//!
//! The Valve-list fields record, at audit time, how many of the map's paths
//! Valve's own `pak01_dir.vpk` file list also has. The shared-path logic of the
//! skin analyzer does not apply to a map package: it is a separate package,
//! not a set of files layered over `pak01`. What it replaces is the game's map
//! package as a whole.
//!
//! `src-tauri/maps/index.json` is the interface's copy of the list; a test
//! requires it to be exactly what the manifests derive.

use crate::content_store;
use crate::map_package::{self, MapReport};
use crate::package_registry::LocalizedName;
use crate::wardrobe::{archive_tail, TRUSTED_ARCHIVE_PREFIXES};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::OnceLock;

const MANIFESTS: &str = include_str!("../maps/manifests.json");
#[cfg(test)]
const INDEX: &str = include_str!("../maps/index.json");

/// Map IDs share the package ID shape (`namespace.name`) under their own
/// namespace, which `package_registry` reserves.
pub(crate) const NAMESPACE: &str = "map";
const SCHEMA_VERSION: u32 = 1;
const MAX_ARCHIVE_BYTES: u64 = content_store::MAX_ARTIFACT_BYTES;
/// The only entries a map ZIP may hold.
const GUIDE_ENTRY: &str = "Guide.txt";
const MAX_GUIDE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapSource {
    /// Mirrors of the same ZIP, tried in order, each under a trusted prefix.
    pub urls: Vec<String>,
    pub catalog_snapshot: String,
    /// How permission to use the item was established, as recorded. These are
    /// Valve's own maps, mirrored by the catalog project: BetterFy does not
    /// host them and has not established a licence for them.
    pub permission: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapArchivePin {
    pub zip_bytes: u64,
    pub zip_sha256: String,
    /// Always `maps/dota.vpk`.
    pub inner_path: String,
    pub inner_bytes: usize,
    pub inner_sha256: String,
    /// The `Guide.txt` that ships beside it (not installed).
    pub guide_bytes: usize,
    pub guide_sha256: String,
}

/// What the analyzer found when a person audited this archive.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapAudit {
    pub report: MapReport,
    /// SHA-256 and entry count of Valve's file list the overlap was computed
    /// against (SteamDatabase `game/dota/pak01_dir.txt`).
    pub valve_list_sha256: String,
    pub valve_list_entries: usize,
    /// How many of the map's entry paths Valve's file list also has. A map
    /// package is not expected to share any.
    pub valve_overlap: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapManifest {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub catalog_id: String,
    pub name: LocalizedName,
    pub distribution: String,
    pub source: MapSource,
    pub archive: MapArchivePin,
    pub audited: MapAudit,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verified_languages: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_note: Option<String>,
}

/// The interface's copy of one manifest (`index.json`): what a card and the
/// build summary need.
#[cfg(test)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IndexEntry {
    pub id: String,
    pub catalog_id: String,
    pub kind: String,
    pub name: LocalizedName,
    pub author: String,
    /// Size of the map package that is written to the game.
    pub install_bytes: u64,
    pub install_files: usize,
}

#[cfg(test)]
pub(crate) fn index_entry(manifest: &MapManifest) -> IndexEntry {
    IndexEntry {
        id: manifest.id.clone(),
        catalog_id: manifest.catalog_id.clone(),
        kind: manifest.kind.clone(),
        name: manifest.name.clone(),
        // The catalog credits no author for a standard terrain; the field is
        // here so the interface can treat every installable item alike.
        author: "Dota 2".to_string(),
        install_bytes: manifest.archive.inner_bytes as u64,
        install_files: manifest.audited.report.entries,
    }
}

/// One map in a reviewed build plan: what the analyzer found and what the
/// install writes.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MapPlanItem {
    pub package_id: String,
    pub catalog_id: String,
    /// Where the file goes, below the language folder.
    pub file: &'static str,
    pub bytes: usize,
    pub sha256: String,
    pub source_urls: Vec<String>,
    pub archive_sha256: String,
    pub catalog_snapshot: String,
    pub permission: String,
    pub verified_languages: Vec<String>,
    pub compatibility_note: Option<String>,
    pub valve_list_entries: usize,
    pub valve_overlap: usize,
    pub report: MapReport,
}

pub(crate) struct LoadedMap {
    pub bytes: Vec<u8>,
    pub sha256: String,
    pub item: MapPlanItem,
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

fn valid_archive_urls(urls: &[String]) -> bool {
    if urls.is_empty() || urls.len() > TRUSTED_ARCHIVE_PREFIXES.len() {
        return false;
    }
    let Some(first) = archive_tail(&urls[0]) else {
        return false;
    };
    let mut prefixes = BTreeSet::new();
    urls.iter().all(|url| {
        archive_tail(url) == Some(first)
            && TRUSTED_ARCHIVE_PREFIXES
                .iter()
                .find(|prefix| url.starts_with(**prefix))
                .is_some_and(|prefix| prefixes.insert(*prefix))
    })
}

pub(crate) fn validate(manifest: &MapManifest) -> Result<(), String> {
    let invalid = |reason: &str| Err(format!("map_manifest_invalid:{}:{reason}", manifest.id));
    if manifest.schema_version != SCHEMA_VERSION || manifest.kind != "map" {
        return invalid("schema");
    }
    if manifest.distribution != "internal_pilot" {
        return invalid("distribution");
    }
    let name = manifest.id.strip_prefix("map.");
    if !name.is_some_and(slug)
        || !slug(&manifest.catalog_id)
        || !manifest.catalog_id.starts_with("terrains-")
        || name.is_some_and(|name| manifest.catalog_id != format!("terrains-{name}"))
    {
        return invalid("id");
    }
    if manifest.name.ru.trim().is_empty()
        || manifest.name.en.trim().is_empty()
        || manifest.source.permission.trim().is_empty()
        || manifest.source.catalog_snapshot.trim().is_empty()
    {
        return invalid("name");
    }
    if !valid_archive_urls(&manifest.source.urls) {
        return invalid("source");
    }
    let pin = &manifest.archive;
    if pin.zip_bytes == 0
        || pin.zip_bytes > MAX_ARCHIVE_BYTES
        || pin.inner_path != map_package::MAP_RELATIVE_PATH
        || pin.inner_bytes == 0
        || pin.inner_bytes > map_package::MAX_MAP_BYTES
        || pin.guide_bytes == 0
        || pin.guide_bytes > MAX_GUIDE_BYTES
        || !valid_sha256(&pin.zip_sha256)
        || !valid_sha256(&pin.inner_sha256)
        || !valid_sha256(&pin.guide_sha256)
    {
        return invalid("archive");
    }
    let audit = &manifest.audited;
    let report = &audit.report;
    if !matches!(report.vpk_version, 1 | 2)
        || report.entries == 0
        || report.total_bytes != pin.inner_bytes as u64
        || report.payload_bytes > report.total_bytes
        || report.zero_length > report.entries
        || !report.main_map
        || !valid_sha256(&report.identity)
        || report.extensions.values().sum::<usize>() != report.entries
        || report.subfolders.values().sum::<usize>() != report.entries
        || report
            .extensions
            .keys()
            .any(|extension| !map_package::ALLOWED_EXTENSIONS.contains(&extension.as_str()))
        || !valid_sha256(&audit.valve_list_sha256)
        || audit.valve_list_entries == 0
        || audit.valve_overlap > report.entries
    {
        return invalid("audit");
    }
    Ok(())
}

fn parse_all(source: &str) -> Result<Vec<MapManifest>, String> {
    let manifests: Vec<MapManifest> = serde_json::from_str(source)
        .map_err(|error| format!("map_manifest_invalid:parse:{error}"))?;
    let mut ids = BTreeSet::new();
    let mut catalog_ids = BTreeSet::new();
    for manifest in &manifests {
        validate(manifest)?;
        if !ids.insert(manifest.id.as_str()) || !catalog_ids.insert(manifest.catalog_id.as_str()) {
            return Err(format!("map_manifest_invalid:{}:duplicate", manifest.id));
        }
    }
    Ok(manifests)
}

struct Registry {
    manifests: Vec<MapManifest>,
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
pub(crate) fn manifests() -> Result<&'static [MapManifest], String> {
    embedded()
        .as_ref()
        .map(|registry| registry.manifests.as_slice())
        .map_err(|_| "map_registry_invalid".to_string())
}

/// Resolves either the engine ID (`map.ti5-desert-terrain`) or the catalog ID
/// (`terrains-ti5-desert-terrain`). `Ok(None)` means the ID is not a map; an
/// invalid registry is an error, not `None`.
pub(crate) fn find(id: &str) -> Result<Option<&'static MapManifest>, String> {
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
        .map_err(|_| "map_registry_invalid".to_string())?;
    Ok(registry
        .by_id
        .get(id)
        .map(|index| &registry.manifests[*index]))
}

/// Whether the catalog ID belongs to the map allowlist; a signed package
/// catalog may not reuse it.
pub(crate) fn catalog_id_taken(catalog_id: &str) -> bool {
    embedded().as_ref().is_ok_and(|registry| {
        registry
            .by_id
            .get(catalog_id)
            .is_some_and(|index| registry.manifests[*index].catalog_id == catalog_id)
    })
}

/// Test-only registry of synthetic maps, so build tests can exercise the whole
/// pipeline without a real archive. Never compiled into the app.
#[cfg(test)]
static TEST_ITEMS: std::sync::Mutex<Vec<&'static MapManifest>> = std::sync::Mutex::new(Vec::new());

#[cfg(test)]
pub(crate) fn register_for_test(manifest: MapManifest) -> &'static MapManifest {
    validate(&manifest).expect("valid test map");
    let leaked: &'static MapManifest = Box::leak(Box::new(manifest));
    TEST_ITEMS.lock().expect("test registry").push(leaked);
    leaked
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Reads the two entries a map ZIP holds and checks them against the pin. The
/// ZIP is inspected first (no traversal, links, encrypted or executable
/// entries, bounded sizes) and must hold exactly `maps/dota.vpk` and
/// `Guide.txt`; each is read through a hard length bound, not the size it
/// declares. Returns the map package.
pub(crate) fn extract_pinned_map(zip_bytes: &[u8], pin: &MapArchivePin) -> Result<Vec<u8>, String> {
    let report = crate::archive_inspector::inspect_zip_bytes(zip_bytes)?;
    if report.entries != 2 || report.files != 2 {
        return Err("map_archive_unexpected".to_string());
    }
    let mut archive = zip::read::ZipArchive::new(Cursor::new(zip_bytes))
        .map_err(|_| "archive_format_invalid".to_string())?;
    let mut read_entry = |name: &str, expected_bytes: usize, expected_sha256: &str| {
        let mut entry = archive
            .by_name(name)
            .map_err(|_| "map_archive_unexpected".to_string())?;
        if !entry.is_file() || entry.size() != expected_bytes as u64 {
            return Err("map_archive_unexpected".to_string());
        }
        let mut bytes = Vec::with_capacity(expected_bytes);
        entry
            .by_ref()
            .take(expected_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "map_archive_unreadable".to_string())?;
        if bytes.len() != expected_bytes || sha256_hex(&bytes) != expected_sha256 {
            return Err("map_archive_hash_mismatch".to_string());
        }
        Ok(bytes)
    };
    let guide = read_entry(GUIDE_ENTRY, pin.guide_bytes, &pin.guide_sha256)?;
    drop(guide);
    read_entry(&pin.inner_path, pin.inner_bytes, &pin.inner_sha256)
}

/// Makes sure the map package is in the content store, downloading and
/// verifying the pinned ZIP when it is not. Only the verified `maps/dota.vpk`
/// is stored.
pub(crate) fn acquire(
    app_data_root: &Path,
    manifest: &MapManifest,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    let pin = &manifest.archive;
    if content_store::read_pinned_resource(app_data_root, pin.inner_bytes, &pin.inner_sha256)
        .is_ok()
    {
        return Ok(());
    }
    let zip = crate::remote_intake::fetch_pinned_map_archive(manifest, cancelled)?;
    let inner = extract_pinned_map(&zip, pin)?;
    content_store::store_pinned_resource(app_data_root, pin.inner_bytes, &pin.inner_sha256, &inner)
}

/// The analyzer must reproduce the audit, field for field.
fn matches_audit(manifest: &MapManifest, report: &MapReport) -> bool {
    *report == manifest.audited.report
}

/// Reads the map package back from the content store (size and SHA-256 are
/// checked again), analyzes it and returns the bytes to install.
pub(crate) fn load(app_data_root: &Path, manifest: &MapManifest) -> Result<LoadedMap, String> {
    let pin = &manifest.archive;
    let bytes =
        content_store::read_pinned_resource(app_data_root, pin.inner_bytes, &pin.inner_sha256)?;
    let report = map_package::analyze(&bytes).map_err(String::from)?;
    if !matches_audit(manifest, &report) {
        return Err("map_audit_mismatch".to_string());
    }
    Ok(LoadedMap {
        sha256: pin.inner_sha256.clone(),
        item: MapPlanItem {
            package_id: manifest.id.clone(),
            catalog_id: manifest.catalog_id.clone(),
            file: map_package::MAP_RELATIVE_PATH,
            bytes: bytes.len(),
            sha256: pin.inner_sha256.clone(),
            source_urls: manifest.source.urls.clone(),
            archive_sha256: pin.zip_sha256.clone(),
            catalog_snapshot: manifest.source.catalog_snapshot.clone(),
            permission: manifest.source.permission.clone(),
            verified_languages: manifest.verified_languages.clone(),
            compatibility_note: manifest.compatibility_note.clone(),
            valve_list_entries: manifest.audited.valve_list_entries,
            valve_overlap: manifest.audited.valve_overlap,
            report,
        },
        bytes,
    })
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    /// The entries of a small synthetic map package.
    pub(crate) fn map_entries(label: &'static [u8]) -> Vec<(&'static str, &'static [u8])> {
        vec![
            ("maps/dota.vmap_c", label),
            ("maps/dota.vhcg", b"hcg".as_slice()),
            (
                "maps/dota/world_visibility.vvis_c",
                b"visibility".as_slice(),
            ),
            ("maps/dota/models/tree_01.vmdl_c", b"tree".as_slice()),
        ]
    }

    /// A ZIP that holds `maps/dota.vpk` and a `Guide.txt`, like the real ones.
    pub(crate) fn zip_of(vpk: &[u8], guide: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut writer = ZipWriter::new(Cursor::new(&mut bytes));
            writer
                .start_file("maps/dota.vpk", SimpleFileOptions::default())
                .expect("start map");
            writer.write_all(vpk).expect("write map");
            writer
                .start_file("Guide.txt", SimpleFileOptions::default())
                .expect("start guide");
            writer.write_all(guide).expect("write guide");
            writer.finish().expect("finish");
        }
        bytes
    }

    /// A manifest value for a synthetic map, with the audit the analyzer
    /// produces for it, and the ZIP and VPK bytes it pins.
    pub(crate) fn synthetic_value(
        slug: &str,
        label: &'static [u8],
    ) -> (serde_json::Value, Vec<u8>, Vec<u8>) {
        let vpk = crate::vpk::build_unchecked(2, &map_entries(label));
        let guide = b"Move the maps folder to your language folder.".to_vec();
        let zip = zip_of(&vpk, &guide);
        let report = map_package::analyze(&vpk).expect("synthetic map");
        let value = serde_json::json!({
            "schemaVersion": SCHEMA_VERSION,
            "kind": "map",
            "id": format!("map.{slug}"),
            "catalogId": format!("terrains-{slug}"),
            "name": { "ru": "Test Map", "en": "Test Map" },
            "distribution": "internal_pilot",
            "source": {
                "urls": [
                    "https://raw.githubusercontent.com/h6rd/Dota2PornFxWeb/main/assets/files/terrains/Test%20Map.zip",
                    "https://huggingface.co/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/terrains/Test%20Map.zip"
                ],
                "catalogSnapshot": "test",
                "permission": "valve_content_mirrored_by_catalog"
            },
            "archive": {
                "zipBytes": zip.len(),
                "zipSha256": sha256_hex(&zip),
                "innerPath": "maps/dota.vpk",
                "innerBytes": vpk.len(),
                "innerSha256": sha256_hex(&vpk),
                "guideBytes": guide.len(),
                "guideSha256": sha256_hex(&guide)
            },
            "audited": {
                "report": report,
                "valveListSha256": "2".repeat(64),
                "valveListEntries": 388116,
                "valveOverlap": 0
            }
        });
        (value, zip, vpk)
    }

    pub(crate) fn synthetic_item(
        slug: &str,
        label: &'static [u8],
    ) -> (MapManifest, Vec<u8>, Vec<u8>) {
        let (value, zip, vpk) = synthetic_value(slug, label);
        let manifest: MapManifest = serde_json::from_value(value).expect("manifest");
        validate(&manifest).expect("valid");
        (manifest, zip, vpk)
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    fn temp_root(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "betterfy-map-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        root
    }

    fn array_with(edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let (mut value, _, _) = synthetic_value("base-map", b"base");
        edit(&mut value);
        serde_json::Value::Array(vec![value]).to_string()
    }

    #[test]
    fn the_embedded_allowlist_validates_and_the_files_are_canonical() {
        let all = parse_all(MANIFESTS).expect("every embedded manifest is valid");
        let mut previous = "";
        for item in &all {
            // sorted by engine id, so the generated file is deterministic
            assert!(previous < item.id.as_str(), "{} is out of order", item.id);
            previous = &item.id;
            assert!(valid_archive_urls(&item.source.urls), "{}", item.id);
            assert_eq!(item.archive.inner_path, "maps/dota.vpk");
            assert_eq!(item.audited.valve_overlap, 0, "{}", item.id);
            assert!(item.audited.report.main_map);
        }
        // Compact and re-serializable: what is embedded is what the generator
        // would write for these manifests.
        assert_eq!(
            serde_json::to_string(&all).expect("serializes"),
            MANIFESTS.trim_end(),
            "maps/manifests.json is not the canonical compact form; regenerate it"
        );
        let index = all.iter().map(index_entry).collect::<Vec<_>>();
        assert_eq!(
            serde_json::to_string(&index).expect("serializes"),
            INDEX.trim_end(),
            "maps/index.json is not derived from manifests.json; regenerate it"
        );
        // Only the two generated files live in the folder.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("maps");
        let mut names = std::fs::read_dir(dir)
            .expect("maps dir")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        names.sort();
        assert_eq!(names, vec!["index.json", "manifests.json"]);
    }

    #[test]
    fn ids_resolve_both_ways_and_stay_apart_from_other_namespaces() {
        let all = manifests().expect("registry parses");
        let first = all.first().expect("at least one map");
        assert_eq!(
            find(&first.id).expect("registry").map(|item| &item.id),
            Some(&first.id)
        );
        assert_eq!(
            find(&first.catalog_id)
                .expect("registry")
                .map(|item| &item.id),
            Some(&first.id)
        );
        assert!(find("wardrobe.scarlet-keeper-of-the-light")
            .expect("registry")
            .is_none());
        assert!(find("minify-tree-mod").expect("registry").is_none());
        assert!(catalog_id_taken(&first.catalog_id));
        assert!(!catalog_id_taken(&first.id));
        assert!(!catalog_id_taken("heroes-scarlet-keeper-of-the-light"));
    }

    #[test]
    fn hostile_or_malformed_entries_are_rejected() {
        assert!(parse_all(&array_with(|_| {})).is_ok());
        let github = |tail: &str| {
            format!(
                "https://raw.githubusercontent.com/h6rd/Dota2PornFxWeb/main/assets/files/{tail}"
            )
        };
        let cases = [
            array_with(|v| v["kind"] = "wardrobe".into()),
            array_with(|v| v["schemaVersion"] = 2.into()),
            array_with(|v| v["distribution"] = "public".into()),
            array_with(|v| v["id"] = "wardrobe.base-map".into()),
            array_with(|v| v["id"] = "map.Base-Map".into()),
            array_with(|v| v["catalogId"] = "heroes-base-map".into()),
            array_with(|v| v["catalogId"] = "terrains-other".into()),
            array_with(|v| v["source"]["urls"] = serde_json::json!([])),
            array_with(|v| {
                v["source"]["urls"] =
                    serde_json::json!([github("terrains/Test%20Map.zip").replace("https", "http")])
            }),
            array_with(|v| {
                v["source"]["urls"] = serde_json::json!([github("terrains/../Test%20Map.zip")])
            }),
            array_with(|v| {
                v["source"]["urls"] = serde_json::json!([
                    github("terrains/Test%20Map.zip"),
                    github("terrains/Other.zip")
                ])
            }),
            array_with(|v| v["source"]["urls"] = serde_json::json!(["https://example.com/a.zip"])),
            array_with(|v| v["archive"]["innerPath"] = "maps/other.vpk".into()),
            array_with(|v| v["archive"]["innerPath"] = "../maps/dota.vpk".into()),
            array_with(|v| v["archive"]["zipBytes"] = 0.into()),
            array_with(|v| v["archive"]["innerBytes"] = 500_000_000.into()),
            array_with(|v| v["archive"]["innerSha256"] = "xyz".into()),
            array_with(|v| v["archive"]["guideBytes"] = 0.into()),
            array_with(|v| v["audited"]["report"]["mainMap"] = false.into()),
            array_with(|v| v["audited"]["report"]["entries"] = 0.into()),
            array_with(|v| v["audited"]["report"]["totalBytes"] = 1.into()),
            array_with(|v| v["audited"]["report"]["extensions"]["exe"] = 1.into()),
            array_with(|v| v["audited"]["report"]["extensions"]["vmdl_c"] = 99.into()),
            array_with(|v| v["audited"]["report"]["vpkVersion"] = 3.into()),
            array_with(|v| v["audited"]["valveOverlap"] = 5000.into()),
            array_with(|v| v["audited"]["extra"] = 1.into()),
            array_with(|v| v["stripPaths"] = serde_json::json!([])),
        ];
        for (index, case) in cases.iter().enumerate() {
            assert!(
                parse_all(case).is_err(),
                "case {index} was accepted: {case}"
            );
        }
        // Two entries with the same id are one too many.
        let (value, _, _) = synthetic_value("dup", b"dup");
        let twice = serde_json::Value::Array(vec![value.clone(), value]).to_string();
        assert!(parse_all(&twice).is_err());
    }

    #[test]
    fn the_zip_must_hold_exactly_the_pinned_map_and_guide() {
        let (manifest, zip, vpk) = synthetic_item("zip-shape", b"shape");
        let pin = &manifest.archive;
        assert_eq!(extract_pinned_map(&zip, pin).expect("pinned entry"), vpk);
        // An extra entry, a missing guide, other bytes, other names.
        let with_extra = {
            let mut bytes = Vec::new();
            {
                use std::io::Write;
                let mut writer = zip::write::ZipWriter::new(Cursor::new(&mut bytes));
                for (name, data) in [
                    ("maps/dota.vpk", vpk.as_slice()),
                    (
                        "Guide.txt",
                        b"Move the maps folder to your language folder.".as_slice(),
                    ),
                    ("extra.txt", b"x".as_slice()),
                ] {
                    writer
                        .start_file(name, zip::write::SimpleFileOptions::default())
                        .expect("start");
                    writer.write_all(data).expect("write");
                }
                writer.finish().expect("finish");
            }
            bytes
        };
        for hostile in [
            with_extra,
            zip_of(&vpk, b"another guide"),
            zip_of(b"not the pinned map bytes of the same length..", b"x"),
        ] {
            assert!(extract_pinned_map(&hostile, pin).is_err());
        }
        let mut wrong_name = pin.clone();
        wrong_name.inner_path = "maps/other.vpk".to_string();
        assert!(extract_pinned_map(&zip, &wrong_name).is_err());
    }

    #[test]
    fn load_reads_the_store_and_reproduces_the_audit() {
        let root = temp_root("load");
        let (manifest, _, vpk) = synthetic_item("load-map", b"load");
        let pin = &manifest.archive;
        assert!(
            load(&root, &manifest).is_err(),
            "an empty store has nothing to load"
        );
        content_store::store_pinned_resource(&root, pin.inner_bytes, &pin.inner_sha256, &vpk)
            .expect("store");
        let loaded = load(&root, &manifest).expect("load");
        assert_eq!(loaded.bytes, vpk);
        assert_eq!(loaded.sha256, pin.inner_sha256);
        assert_eq!(loaded.item.file, "maps/dota.vpk");
        assert_eq!(loaded.item.report, manifest.audited.report);
        assert_eq!(loaded.item.valve_overlap, 0);
        // An audit that does not match what the bytes contain is refused.
        let mut drifted = manifest.clone();
        drifted.audited.report.identity = "f".repeat(64);
        assert_eq!(
            load(&root, &drifted).err().as_deref(),
            Some("map_audit_mismatch")
        );
        let _ = std::fs::remove_dir_all(root);
    }
}

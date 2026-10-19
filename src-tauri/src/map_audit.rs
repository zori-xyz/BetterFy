//! Local-only generator of the map-terrain allowlist (`#[ignore]`d tests).
//!
//! It turns a cache of downloaded archives into the embedded manifests. It
//! never runs in CI and never touches the network: everything it reads is a
//! local file named by an environment variable.
//!
//! ```text
//! BETTERFY_MAP_ZIPS     directory holding <catalogId>.zip for every item
//! BETTERFY_MAP_FETCH    fetch_report.json: {catalogId: {file, bytes, sha256,
//!                       mirrors: {github: [status, length], huggingface: [..]},
//!                       mirror_sha256: {github: sha, huggingface: sha}}}
//!                       (scripts/map-terrains/fetch_maps.py writes it;
//!                       mirror_sha256 only with --verify-mirrors)
//! BETTERFY_MAP_CATALOG  src/webCatalog.json
//! BETTERFY_MAP_VALVE    Valve's pak01_dir.txt (SteamDatabase/GameTracking-Dota2)
//! BETTERFY_MAP_WRITE    "1" to write src-tauri/maps/{manifests,index}.json and
//!                       docs/MAP_AUDIT_REPORT.md (a dry run otherwise)
//! cargo test --release generate_map_allowlist -- --ignored --nocapture
//! ```
//!
//! For every `terrains` catalog item whose file is a ZIP it opens the cached
//! archive, reads `maps/dota.vpk` and `Guide.txt`, runs the map analyzer,
//! counts how many of the map's paths Valve's file list also has, lists only
//! the mirrors that answered 200 with the pinned size, validates the entry and
//! lets the app's own load path reproduce the audit. Anything that cannot be
//! audited cleanly is not emitted and is listed with its reason in the report.
//! The output is sorted and carries no timestamp, so the same inputs always
//! produce the same bytes.

use crate::content_store;
use crate::map_package;
use crate::map_terrain::{self, MapArchivePin, MapAudit, MapManifest, MapSource};
use crate::package_registry::LocalizedName;
use crate::vpk;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::Read;
use std::path::{Path, PathBuf};

const GITHUB: &str =
    "https://raw.githubusercontent.com/h6rd/Dota2PornFxWeb/main/assets/files/terrains/";
const HUGGING_FACE: &str =
    "https://huggingface.co/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/terrains/";

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var(name).ok().map(PathBuf::from)
}

/// Percent-encodes a file name the way the catalog's mirrors expect (spaces as
/// `%20`, letters, digits and `._-~` kept).
fn encode(name: &str) -> String {
    name.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'~') {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

struct CatalogItem {
    id: String,
    name: String,
    file: String,
}

fn read_catalog(path: &Path) -> (String, Vec<CatalogItem>, usize) {
    let value: Value =
        serde_json::from_slice(&std::fs::read(path).expect("catalog")).expect("json");
    let snapshot = value["snapshot"].as_str().expect("snapshot").to_string();
    let terrains = value["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter(|item| item["category"] == "terrains")
        .collect::<Vec<_>>();
    let total = terrains.len();
    let zips = terrains
        .into_iter()
        .filter(|item| {
            item["file"]
                .as_str()
                .is_some_and(|file| file.to_ascii_lowercase().ends_with(".zip"))
        })
        .map(|item| CatalogItem {
            id: item["id"].as_str().expect("id").to_string(),
            name: item["name"].as_str().expect("name").trim().to_string(),
            file: item["file"].as_str().expect("file").to_string(),
        })
        .collect();
    (snapshot, zips, total)
}

/// Valve's file list: `path CRC:xxxx size:n` per line, lower-cased.
fn read_valve_list(path: &Path) -> (BTreeSet<String>, String, usize) {
    let bytes = std::fs::read(path).expect("valve list");
    let sha = sha256_hex(&bytes);
    let text = String::from_utf8_lossy(&bytes);
    let paths = text
        .lines()
        .filter_map(|line| line.split_once(" CRC:").map(|(path, _)| path.trim()))
        .map(str::to_ascii_lowercase)
        .collect::<BTreeSet<_>>();
    let entries = text.lines().filter(|line| line.contains(" CRC:")).count();
    (paths, sha, entries)
}

struct Audited {
    manifest: MapManifest,
    guide_text: String,
}

enum Outcome {
    Emitted(Box<Audited>),
    Skipped(String, String),
}

/// A mirror is listed when it answered 200 with the archive's size and, when
/// the fetch report carries the SHA-256 of what that mirror served
/// (`--verify-mirrors`), the same bytes.
fn mirror_ok(report: &Value, host: &str, bytes: u64, sha256: &str) -> bool {
    let row = &report["mirrors"][host];
    let served = &report["mirror_sha256"][host];
    row[0].as_u64() == Some(200)
        && row[1].as_u64() == Some(bytes)
        && (served.is_null() || served.as_str() == Some(sha256))
}

#[allow(clippy::too_many_arguments)]
fn audit_item(
    item: &CatalogItem,
    snapshot: &str,
    zips: &Path,
    fetch: &Value,
    valve: &BTreeSet<String>,
    valve_sha: &str,
    valve_entries: usize,
) -> Outcome {
    let skip = |reason: &str| Outcome::Skipped(item.id.clone(), reason.to_string());
    let Ok(zip) = std::fs::read(zips.join(format!("{}.zip", item.id))) else {
        return skip("no archive was downloaded");
    };
    let report = &fetch[item.id.as_str()];
    let zip_sha = sha256_hex(&zip);
    if report["sha256"].as_str() != Some(zip_sha.as_str())
        || report["bytes"].as_u64() != Some(zip.len() as u64)
    {
        return skip("the cached archive does not match the fetch report");
    }
    let mut urls = Vec::new();
    let encoded = encode(&item.file);
    if mirror_ok(report, "github", zip.len() as u64, &zip_sha) {
        urls.push(format!("{GITHUB}{encoded}"));
    }
    if mirror_ok(report, "huggingface", zip.len() as u64, &zip_sha) {
        urls.push(format!("{HUGGING_FACE}{encoded}"));
    }
    if urls.is_empty() {
        return skip("no mirror answered 200 with the archive's size");
    }
    let inspected = match crate::archive_inspector::inspect_zip_bytes(&zip) {
        Ok(inspected) => inspected,
        Err(code) => return skip(&format!("the ZIP preflight refused it ({code})")),
    };
    if inspected.entries != 2 || inspected.files != 2 {
        return skip("the ZIP does not hold exactly maps/dota.vpk and Guide.txt");
    }
    let mut archive = match zip::read::ZipArchive::new(std::io::Cursor::new(&zip)) {
        Ok(archive) => archive,
        Err(_) => return skip("the ZIP cannot be opened"),
    };
    let mut read = |name: &str| -> Result<Vec<u8>, String> {
        let mut entry = archive
            .by_name(name)
            .map_err(|_| format!("the ZIP has no {name}"))?;
        let mut bytes = Vec::new();
        entry
            .by_ref()
            .take(map_package::MAX_MAP_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| format!("{name} cannot be read"))?;
        Ok(bytes)
    };
    let inner = match read("maps/dota.vpk") {
        Ok(bytes) => bytes,
        Err(reason) => return skip(&reason),
    };
    let guide = match read("Guide.txt") {
        Ok(bytes) => bytes,
        Err(reason) => return skip(&reason),
    };
    let analysis = match map_package::analyze(&inner) {
        Ok(analysis) => analysis,
        Err(rejection) => {
            return skip(&format!(
                "the map analyzer refused it ({}{})",
                rejection.code,
                rejection
                    .path
                    .map(|path| format!(": {path}"))
                    .unwrap_or_default()
            ))
        }
    };
    let overlap = match vpk::read_embedded_directory(&inner) {
        Ok(entries) => entries
            .iter()
            .filter(|entry| valve.contains(&entry.path.to_ascii_lowercase()))
            .count(),
        Err(code) => return skip(&format!("the map directory cannot be read again ({code})")),
    };
    let manifest = MapManifest {
        schema_version: 1,
        kind: "map".to_string(),
        id: format!("map.{}", item.id.trim_start_matches("terrains-")),
        catalog_id: item.id.clone(),
        name: LocalizedName {
            ru: item.name.clone(),
            en: item.name.clone(),
        },
        distribution: "internal_pilot".to_string(),
        source: MapSource {
            urls,
            catalog_snapshot: snapshot.to_string(),
            permission: "valve_content_mirrored_by_catalog".to_string(),
        },
        archive: MapArchivePin {
            zip_bytes: zip.len() as u64,
            zip_sha256: zip_sha,
            inner_path: "maps/dota.vpk".to_string(),
            inner_bytes: inner.len(),
            inner_sha256: sha256_hex(&inner),
            guide_bytes: guide.len(),
            guide_sha256: sha256_hex(&guide),
        },
        audited: MapAudit {
            report: analysis,
            valve_list_sha256: valve_sha.to_string(),
            valve_list_entries: valve_entries,
            valve_overlap: overlap,
        },
        verified_languages: Vec::new(),
        compatibility_note: None,
    };
    if let Err(reason) = map_terrain::validate(&manifest) {
        return skip(&format!("its manifest does not validate ({reason})"));
    }
    // The app's own load path must reproduce the audit.
    match map_terrain::extract_pinned_map(&zip, &manifest.archive) {
        Ok(extracted) if extracted == inner => {}
        _ => return skip("the app's extraction does not reproduce the pinned bytes"),
    }
    Outcome::Emitted(Box::new(Audited {
        guide_text: String::from_utf8_lossy(&guide).into_owned(),
        manifest,
    }))
}

fn report_markdown(
    audited: &[Audited],
    skipped: &[(String, String)],
    snapshot: &str,
    terrains_total: usize,
    valve_sha: &str,
    valve_entries: usize,
) -> String {
    let mut text = String::new();
    let _ = writeln!(text, "# Map terrain audit report\n");
    let _ = writeln!(
        text,
        "Written by `cargo test generate_map_allowlist -- --ignored` from archives cached on a Mac. It carries no timestamp: the same inputs give the same file. The audited facts are pinned in [`src-tauri/maps/manifests.json`](../src-tauri/maps/manifests.json), and every build re-runs the map analyzer (`src-tauri/src/map_package.rs`) on the stored bytes and must reproduce them. A mirror is listed only when it answered 200 with the archive's size and, when the fetch report carries it, served bytes with the same SHA-256.\n"
    );
    let _ = writeln!(text, "| What | Value |\n| --- | --- |");
    let _ = writeln!(
        text,
        "| Catalog | `src/webCatalog.json`, snapshot {snapshot} |"
    );
    let _ = writeln!(text, "| `terrains` catalog items | {terrains_total} |");
    let _ = writeln!(
        text,
        "| ZIP items (`maps/dota.vpk` + `Guide.txt`) | {} |",
        audited.len() + skipped.len()
    );
    let _ = writeln!(text, "| Audited and allowlisted | {} |", audited.len());
    let _ = writeln!(text, "| Not installable | {} |", skipped.len());
    let _ = writeln!(
        text,
        "| Valve file list | `game/dota/pak01_dir.txt` (SteamDatabase), SHA-256 `{valve_sha}`, {valve_entries} entries |"
    );
    let _ = writeln!(
        text,
        "| Raw `pakNN_dir.vpk` terrains | {} catalog items, not handled here (ordinary VPKs, installed through the generic catalog path) |\n",
        terrains_total - audited.len() - skipped.len()
    );
    let _ = writeln!(text, "## Allowlisted maps\n");
    let _ = writeln!(
        text,
        "| Engine ID | ZIP (bytes) | ZIP SHA-256 | `maps/dota.vpk` (bytes) | VPK SHA-256 | Entries | Payload bytes | Mirrors |"
    );
    let _ = writeln!(
        text,
        "| --- | ---: | --- | ---: | --- | ---: | ---: | --- |"
    );
    for item in audited {
        let m = &item.manifest;
        let mirrors = m
            .source
            .urls
            .iter()
            .map(|url| {
                if url.starts_with(GITHUB) {
                    "GitHub"
                } else {
                    "Hugging Face"
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            text,
            "| `{}` | {} | `{}` | {} | `{}` | {} | {} | {} |",
            m.id,
            m.archive.zip_bytes,
            m.archive.zip_sha256,
            m.archive.inner_bytes,
            m.archive.inner_sha256,
            m.audited.report.entries,
            m.audited.report.payload_bytes,
            mirrors
        );
    }
    let _ = writeln!(text, "\n## What each map package contains\n");
    let _ = writeln!(
        text,
        "| Engine ID | VPK | Types (entries) | Under `maps/` | Contents identity | Paths also in Valve's list |"
    );
    let _ = writeln!(text, "| --- | --- | --- | --- | --- | ---: |");
    for item in audited {
        let m = &item.manifest;
        let types = m
            .audited
            .report
            .extensions
            .iter()
            .map(|(extension, count)| format!("{extension} {count}"))
            .collect::<Vec<_>>()
            .join(", ");
        let folders = m
            .audited
            .report
            .subfolders
            .iter()
            .map(|(folder, count)| format!("{folder} {count}"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            text,
            "| `{}` | v{} | {} | {} | `{}` | {} |",
            m.id,
            m.audited.report.vpk_version,
            types,
            folders,
            m.audited.report.identity,
            m.audited.valve_overlap
        );
    }
    let guides = audited
        .iter()
        .map(|item| (&item.manifest.archive.guide_sha256, &item.guide_text))
        .collect::<std::collections::BTreeMap<_, _>>();
    let _ = writeln!(text, "\n## Guide.txt\n");
    let _ = writeln!(
        text,
        "Every ZIP carries a `Guide.txt`. {} distinct file(s) among the {} archives, pinned by size and SHA-256 in each manifest:\n",
        guides.len(),
        audited.len()
    );
    for (sha, guide) in &guides {
        let count = audited
            .iter()
            .filter(|item| &item.manifest.archive.guide_sha256 == *sha)
            .count();
        let _ = writeln!(
            text,
            "- `{sha}`, {} bytes, in {count} archive(s). The English part says: create `dota_<name>` in `steamapps/common/dota 2 beta/game/`, move the `maps` folder into it, and add `-language <name>` to the launch options; for Minify, put `maps` in `dota_minify` with `-language minify`; for another language, put it in that language's `dota_<language>` folder. It also has Russian and Ukrainian sections with the same instructions. It names no map-specific note.",
            guide.len()
        );
    }
    let _ = writeln!(text, "\n## Not installable\n");
    if skipped.is_empty() {
        let _ = writeln!(text, "None: every ZIP item of the catalog passed.\n");
    } else {
        let _ = writeln!(text, "| Catalog ID | Reason |\n| --- | --- |");
        for (id, reason) in skipped {
            let _ = writeln!(text, "| `{id}` | {reason} |");
        }
    }
    text
}

#[test]
#[ignore = "local-only generator; needs the BETTERFY_MAP_* environment variables"]
fn generate_map_allowlist() {
    let zips = env_path("BETTERFY_MAP_ZIPS").expect("BETTERFY_MAP_ZIPS");
    let fetch: Value = serde_json::from_slice(
        &std::fs::read(env_path("BETTERFY_MAP_FETCH").expect("BETTERFY_MAP_FETCH")).expect("fetch"),
    )
    .expect("fetch json");
    let (snapshot, items, terrains_total) =
        read_catalog(&env_path("BETTERFY_MAP_CATALOG").expect("BETTERFY_MAP_CATALOG"));
    let (valve, valve_sha, valve_entries) =
        read_valve_list(&env_path("BETTERFY_MAP_VALVE").expect("BETTERFY_MAP_VALVE"));
    let mut audited = Vec::new();
    let mut skipped = Vec::new();
    for item in &items {
        match audit_item(
            item,
            &snapshot,
            &zips,
            &fetch,
            &valve,
            &valve_sha,
            valve_entries,
        ) {
            Outcome::Emitted(entry) => audited.push(*entry),
            Outcome::Skipped(id, reason) => skipped.push((id, reason)),
        }
    }
    audited.sort_by(|left, right| left.manifest.id.cmp(&right.manifest.id));
    skipped.sort();
    println!(
        "audited {} of {} ZIP items ({} terrains in the catalog)",
        audited.len(),
        items.len(),
        terrains_total
    );
    for (id, reason) in &skipped {
        println!("skipped {id}: {reason}");
    }
    let manifests = audited
        .iter()
        .map(|entry| entry.manifest.clone())
        .collect::<Vec<_>>();
    let manifests_json = serde_json::to_string(&manifests).expect("serializes") + "\n";
    let index = manifests
        .iter()
        .map(map_terrain::index_entry)
        .collect::<Vec<_>>();
    let index_json = serde_json::to_string(&index).expect("serializes") + "\n";
    let report = report_markdown(
        &audited,
        &skipped,
        &snapshot,
        terrains_total,
        &valve_sha,
        valve_entries,
    );
    if std::env::var("BETTERFY_MAP_WRITE").as_deref() == Ok("1") {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        std::fs::write(root.join("maps/manifests.json"), manifests_json).expect("manifests");
        std::fs::write(root.join("maps/index.json"), index_json).expect("index");
        std::fs::write(root.join("../docs/MAP_AUDIT_REPORT.md"), report).expect("report");
        println!("written");
    } else {
        println!("dry run: {} bytes of manifests", manifests_json.len());
    }
}

/// Unpacks the cached ZIPs of the embedded maps exactly as the app does after
/// a download, stores them in a temporary content store and builds real plans
/// from them (a map alone, and a map beside a hero skin when a skin archive is
/// given in `BETTERFY_MAP_SKIN`, a wardrobe catalog ID with its archive under
/// `BETTERFY_AUDIT_ZIPS`; `BETTERFY_MAP_WITH` picks the map for that pairing).
/// Prints real numbers; nothing leaves the temporary folder.
#[test]
#[ignore = "local-only spot check; needs BETTERFY_MAP_ZIPS (and optionally BETTERFY_MAP_SKIN)"]
fn spot_check_embedded_maps() {
    let zips = env_path("BETTERFY_MAP_ZIPS").expect("BETTERFY_MAP_ZIPS");
    let root = std::env::temp_dir().join(format!("betterfy-map-spot-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("temp root");
    let all = map_terrain::manifests().expect("registry");
    assert!(!all.is_empty());
    for item in all {
        let zip = std::fs::read(zips.join(format!("{}.zip", item.catalog_id))).expect("zip");
        assert_eq!(sha256_hex(&zip), item.archive.zip_sha256, "{}", item.id);
        // Exactly what `map_terrain::acquire` does after the download.
        let inner = map_terrain::extract_pinned_map(&zip, &item.archive).expect("extracts");
        content_store::store_pinned_resource(
            &root,
            item.archive.inner_bytes,
            &item.archive.inner_sha256,
            &inner,
        )
        .expect("stores");
        let started = std::time::Instant::now();
        let ids = vec![item.id.clone()];
        let plan = crate::tree_pilot::plan_from_verified_store(&root, &ids).expect("plan");
        let again = crate::tree_pilot::plan_from_verified_store(&root, &ids).expect("plan again");
        let plan = serde_json::to_value(&plan).expect("json");
        let again = serde_json::to_value(&again).expect("json");
        assert_eq!(plan["planId"], again["planId"], "deterministic");
        println!(
            "{}: map {} bytes sha256 {} entries {} plan {} has_pak {} ({:?})",
            item.id,
            plan["map"]["bytes"],
            plan["map"]["sha256"],
            plan["map"]["report"]["entries"],
            plan["planId"],
            plan["hasPak"],
            started.elapsed()
        );
        assert_eq!(plan["hasPak"], false);
        assert_eq!(plan["map"]["sha256"], item.archive.inner_sha256.as_str());
    }
    // A map beside a hero skin.
    let paired = std::env::var("BETTERFY_MAP_WITH")
        .ok()
        .and_then(|catalog_id| all.iter().find(|item| item.catalog_id == catalog_id))
        .or_else(|| all.first());
    if let (Ok(skin), Some(first)) = (std::env::var("BETTERFY_MAP_SKIN"), paired) {
        let skin = crate::wardrobe::find(&skin)
            .expect("registry")
            .unwrap_or_else(|| panic!("{skin} is not in the wardrobe allowlist"));
        let skins = env_path("BETTERFY_AUDIT_ZIPS").expect("BETTERFY_AUDIT_ZIPS");
        let zip = std::fs::read(skins.join(format!("{}.zip", skin.catalog_id))).expect("skin zip");
        let inner = crate::wardrobe::extract_pinned_entry(
            &zip,
            &skin.archive.inner_path,
            skin.archive.inner_bytes,
            &skin.archive.inner_sha256,
        )
        .expect("skin extracts");
        content_store::store_pinned_resource(
            &root,
            skin.archive.inner_bytes,
            &skin.archive.inner_sha256,
            &inner,
        )
        .expect("stores skin");
        let ids = vec![skin.id.clone(), first.id.clone()];
        let plan = crate::tree_pilot::plan_from_verified_store(&root, &ids).expect("plan");
        let plan = serde_json::to_value(&plan).expect("json");
        println!(
            "{} + {}: pak {} bytes ({} files), map {} bytes, packages {}, plan {}",
            skin.id,
            first.id,
            plan["vpkBytes"],
            plan["resourceCount"],
            plan["map"]["bytes"],
            plan["packageCount"],
            plan["planId"]
        );
        assert_eq!(plan["hasPak"], true);
        // The same pak alone has a different plan: the map is bound into it.
        let alone = crate::tree_pilot::plan_from_verified_store(&root, &ids[..1]).expect("alone");
        let alone = serde_json::to_value(&alone).expect("json");
        assert_ne!(alone["planId"], plan["planId"]);
        assert_eq!(
            alone["vpkSha256"], plan["vpkSha256"],
            "the pak is the same file"
        );
        // Two maps are refused.
        if let Some(second) = all.get(1) {
            let both = vec![first.id.clone(), second.id.clone()];
            println!(
                "two maps: {:?}",
                crate::tree_pilot::plan_from_verified_store(&root, &both).err()
            );
        }
    }
    std::fs::remove_dir_all(root).expect("cleanup");
}

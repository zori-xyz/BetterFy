//! Local-only generator of the cursor and font pack allowlist (`#[ignore]`d
//! test). It never runs in CI and never touches the network.
//!
//! ```text
//! BETTERFY_PACKS_ZIPS     directory holding <catalogId>.zip for every pack
//! BETTERFY_PACKS_MIRRORS  json: { catalogId: [urls that answered 200 with the
//!                         file's exact size] }
//! BETTERFY_AUDIT_CATALOG  src/webCatalog.json
//! BETTERFY_AUDIT_WRITE    "1" to write src-tauri/gamefiles/{manifests,index}.json
//!                         and docs/GAME_PACKS_AUDIT.md (dry run otherwise)
//! cargo test --release generate_game_packs -- --ignored --nocapture
//! ```
//!
//! A cursor pack installs every `<Name>/cursor/<file>` whose content passes
//! `game_packs::check_content` into `resource/cursor/`. A font pack installs
//! `<Name>/assets/custom/<font>` into `panorama/fonts/` and removes every
//! font of the pack's own copy of the game's folder (`assets/default`) that it
//! does not replace, as the pack's guide says. The `.bat` scripts are pinned
//! by name and never extracted; every other entry is left in the ZIP and
//! listed in the report.

use crate::game_packs::{
    self, check_content, index_entry, PackArchive, PackCategory, PackFile, PackManifest, PackSource,
};
use crate::package_registry::LocalizedName;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{Cursor, Read};
use std::path::PathBuf;

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("{name}")))
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

struct Item {
    id: String,
    name: String,
    category: PackCategory,
    author: String,
}

fn author_of(item: &Value) -> String {
    item["links"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|link| link["type"] == "author" || link["type"] == "modded")
        .filter_map(|link| link["url"].as_str())
        .map(|url| {
            url.trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or(url)
                .to_string()
        })
        .next()
        .unwrap_or_else(|| "BetterFy Web".to_string())
}

#[test]
#[ignore = "local-only generator; needs the archive cache named by BETTERFY_PACKS_* variables"]
fn generate_game_packs() {
    let zips = env_path("BETTERFY_PACKS_ZIPS");
    let mirrors: BTreeMap<String, Vec<String>> = serde_json::from_slice(
        &std::fs::read(env_path("BETTERFY_PACKS_MIRRORS")).expect("mirrors"),
    )
    .expect("mirrors json");
    let catalog: Value = serde_json::from_slice(
        &std::fs::read(env_path("BETTERFY_AUDIT_CATALOG")).expect("catalog"),
    )
    .expect("catalog json");
    let write = std::env::var("BETTERFY_AUDIT_WRITE").is_ok_and(|value| value == "1");
    let snapshot = catalog["snapshot"].as_str().expect("snapshot").to_string();
    let items = catalog["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter(|item| item["type"] == "mod")
        .filter_map(|item| {
            let category = match item["category"].as_str()? {
                "cursors" => PackCategory::Cursors,
                "fonts" => PackCategory::Fonts,
                _ => return None,
            };
            Some(Item {
                id: item["id"].as_str()?.to_string(),
                name: item["name"].as_str()?.trim().to_string(),
                category,
                author: author_of(item),
            })
        })
        .collect::<Vec<_>>();

    let mut manifests = Vec::new();
    let mut report = String::new();
    let mut skipped = Vec::new();
    for item in &items {
        let zip = match std::fs::read(zips.join(format!("{}.zip", item.id))) {
            Ok(zip) => zip,
            Err(_) => {
                skipped.push((item.id.clone(), "no archive in the cache".to_string()));
                continue;
            }
        };
        let urls = mirrors.get(&item.id).cloned().unwrap_or_default();
        if urls.is_empty() {
            skipped.push((item.id.clone(), "no mirror serves the file".to_string()));
            continue;
        }
        let mut archive = zip::ZipArchive::new(Cursor::new(&zip[..])).expect("zip");
        let mut files = Vec::new();
        let mut defaults = Vec::new();
        let mut scripts = Vec::new();
        let mut left = Vec::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).expect("entry");
            if entry.is_dir() {
                continue;
            }
            let name = entry.name().to_string();
            let parts = name.split('/').collect::<Vec<_>>();
            let file_name = parts.last().copied().unwrap_or_default();
            if file_name.to_ascii_lowercase().ends_with(".bat") {
                scripts.push(name.clone());
                continue;
            }
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).expect("read");
            let folder = parts[1..parts.len() - 1].join("/");
            let target = match (item.category, folder.as_str()) {
                (PackCategory::Cursors, "cursor") => Some(format!(
                    "resource/cursor/{}",
                    file_name.to_ascii_lowercase()
                )),
                (PackCategory::Fonts, "assets/custom") => {
                    Some(format!("panorama/fonts/{}", file_name.to_ascii_lowercase()))
                }
                (PackCategory::Fonts, "assets/default") => {
                    defaults.push(format!("panorama/fonts/{}", file_name.to_ascii_lowercase()));
                    None
                }
                _ => None,
            };
            match target {
                Some(to)
                    if crate::game_files::validate_path(&to).is_ok()
                        && check_content(&to, &bytes).is_ok() =>
                {
                    files.push(PackFile {
                        from: name,
                        to,
                        bytes: bytes.len(),
                        sha256: sha256_hex(&bytes),
                    });
                }
                _ => left.push(name),
            }
        }
        files.sort_by(|a, b| a.to.cmp(&b.to));
        let mut removes = defaults
            .into_iter()
            .filter(|path| !files.iter().any(|file| file.to == *path))
            .filter(|path| crate::game_files::validate_path(path).is_ok())
            .collect::<Vec<_>>();
        removes.sort();
        scripts.sort();
        left.sort();
        let manifest = PackManifest {
            schema_version: 1,
            kind: "gamefiles".to_string(),
            id: format!("{}.{}", game_packs::NAMESPACE, item.id),
            catalog_id: item.id.clone(),
            category: item.category,
            name: LocalizedName {
                ru: item.name.clone(),
                en: item.name.clone(),
            },
            author: item.author.clone(),
            distribution: "internal_pilot".to_string(),
            source: PackSource {
                urls,
                catalog_snapshot: format!("Dota2PornFxWeb {snapshot}"),
                permission: "founder_reported_author_consent".to_string(),
            },
            archive: PackArchive {
                zip_bytes: zip.len() as u64,
                zip_sha256: sha256_hex(&zip),
                entries: archive.len(),
                scripts,
            },
            files,
            removes,
        };
        if let Err(reason) = game_packs::validate(&manifest) {
            skipped.push((item.id.clone(), reason));
            continue;
        }
        // The extraction the app runs must reproduce the audit.
        game_packs::extract_pinned_files(&zip, &manifest).expect("extracts as audited");
        let _ = writeln!(
            report,
            "| `{}` | {} | {} | {:.1} | {} | {} |",
            manifest.catalog_id,
            match manifest.category {
                PackCategory::Cursors => "cursors",
                PackCategory::Fonts => "fonts",
            },
            manifest.files.len(),
            manifest.files.iter().map(|file| file.bytes).sum::<usize>() as f64 / 1_048_576.0,
            manifest.removes.len(),
            if left.is_empty() {
                "none".to_string()
            } else {
                left.iter()
                    .map(|name| format!("`{name}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        );
        manifests.push(manifest);
    }
    manifests.sort_by(|a, b| a.id.cmp(&b.id));
    println!("emitted {} of {}", manifests.len(), items.len());
    for (id, reason) in &skipped {
        println!("skipped {id}: {reason}");
    }
    if !write {
        return;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let json = serde_json::to_string_pretty(&manifests).expect("json") + "\n";
    std::fs::write(root.join("gamefiles/manifests.json"), json).expect("manifests");
    let index = manifests.iter().map(index_entry).collect::<Vec<_>>();
    let json = serde_json::to_string_pretty(&index).expect("json") + "\n";
    std::fs::write(root.join("gamefiles/index.json"), json).expect("index");
    let mut doc = String::new();
    doc.push_str("# Cursor and font pack audit\n\n");
    doc.push_str("Written by the local-only test `game_packs_audit::generate_game_packs` (`src-tauri/src/game_packs_audit.rs`); do not edit by hand. It is the audit trail for `src-tauri/gamefiles/manifests.json`. The run is a Mac run over archives downloaded to a local cache: nothing here was run on Windows or in Dota.\n\n");
    let _ = writeln!(
        doc,
        "Catalog snapshot {snapshot}. Emitted {} of {} items. Every pack's `Install.bat` (and `Uninstall.bat`) is pinned by name and never extracted.\n",
        manifests.len(),
        items.len()
    );
    doc.push_str("| Item | Category | Files written | MiB | Game files removed | Entries left in the ZIP |\n| --- | --- | ---: | ---: | ---: | --- |\n");
    doc.push_str(&report);
    if !skipped.is_empty() {
        doc.push_str("\n## Skipped\n\n");
        for (id, reason) in &skipped {
            let _ = writeln!(doc, "- `{id}`: {reason}");
        }
    }
    std::fs::write(root.join("../docs/GAME_PACKS_AUDIT.md"), doc).expect("report");
    println!("written");
}

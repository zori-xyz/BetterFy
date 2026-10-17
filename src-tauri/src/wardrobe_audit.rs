//! Local-only generator of the wardrobe allowlist (`#[ignore]`d test).
//!
//! It turns a cache of downloaded archives into the embedded manifests. It
//! never runs in CI and never touches the network: everything it reads is a
//! local file named by an environment variable.
//!
//! ```text
//! BETTERFY_AUDIT_ZIPS      directory holding <catalogId>.zip for every item
//! BETTERFY_AUDIT_FETCHED   fetched.json: [[id, source, sha256, size, [urls]], ..]
//! BETTERFY_AUDIT_MIRRORS   (optional) mirrors.json: which candidate URLs answer
//!                          200 with the pinned size; without it every candidate
//!                          URL of fetched.json is listed
//! BETTERFY_AUDIT_CATALOG   src/webCatalog.json
//! BETTERFY_AUDIT_VALVE     Valve's pak01_dir.txt (SteamDatabase/GameTracking-Dota2)
//! BETTERFY_AUDIT_WRITE     "1" to write src-tauri/wardrobe/{manifests,index}.json
//!                          and docs/WARDROBE_AUDIT_REPORT.md (dry run otherwise)
//! cargo test --release generate_wardrobe_allowlist -- --ignored --nocapture
//! ```
//!
//! For every `heroes` and `hero-items` catalog item it opens the cached ZIP,
//! reads its one VPK, infers the hero names from the paths that exist in
//! Valve's file list, computes the shared paths with the same hero rules the
//! analyzer uses, runs the analyzer and, when the result is clean, emits a
//! manifest. Anything that cannot be audited cleanly is not emitted and is
//! listed with its reason in the report. The output is sorted and carries no
//! timestamp, so the same inputs always produce the same bytes.

use crate::content_store;
use crate::package_registry::LocalizedName;
use crate::skin_archive::{self, Limits, RejectReason, ScopeSpec, SharedPolicy, SkinReport};
use crate::wardrobe::{self, ArchivePin, AuditedResult, WardrobeManifest, WardrobeSource};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Read;
use std::path::{Path, PathBuf};

/// A hero name counts as an inferred internal name of the archive when it has
/// at least this share (percent) of the top candidate's Valve-present paths
/// and at least `SECOND_NAME_MIN_PATHS` of them. Valve spells one hero several
/// ways (`shadow_fiend` models, `nevermore` particles, `queen_of_pain` and
/// `queenofpain`, `life_stealer` and `lifestealer`), and in a skin the second
/// spelling often has only a few percent of the paths; a stray single file of
/// another hero stays below the bar.
const SECOND_NAME_PERCENT: usize = 5;
const SECOND_NAME_MIN_PATHS: usize = 3;

struct CatalogItem {
    id: String,
    name: String,
    category: String,
    author: String,
}

#[derive(Clone, Debug)]
struct ItemStats {
    heroes: Vec<String>,
    hero_files: usize,
    author_files: usize,
    install_files: usize,
    install_bytes: u64,
    shared: usize,
    strip_bytes: u64,
    rejected: usize,
    renamed: usize,
    mirrors: usize,
    rejected_reasons: BTreeMap<String, usize>,
    stripped_dirs: BTreeMap<String, usize>,
}

enum Outcome {
    Emitted(Box<WardrobeManifest>, ItemStats),
    Skipped(String, String),
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var(name).ok().map(PathBuf::from)
}

fn read_catalog(path: &Path) -> (String, Vec<CatalogItem>) {
    let value: Value =
        serde_json::from_slice(&std::fs::read(path).expect("catalog")).expect("json");
    let snapshot = value["snapshot"].as_str().expect("snapshot").to_string();
    let items = value["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter(|item| matches!(item["category"].as_str(), Some("heroes" | "hero-items")))
        .map(|item| {
            let authors = item["links"]
                .as_array()
                .map(|links| {
                    links
                        .iter()
                        .filter(|link| link["type"] == "author")
                        .filter_map(|link| link["url"].as_str())
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            CatalogItem {
                id: item["id"].as_str().expect("id").to_string(),
                name: item["name"].as_str().expect("name").trim().to_string(),
                category: item["category"].as_str().expect("category").to_string(),
                author: if authors.is_empty() {
                    "Not credited in the catalog".to_string()
                } else {
                    authors.join(", ")
                },
            }
        })
        .collect();
    (snapshot, items)
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

/// The hero names a set of archive paths points at, from the paths Valve's
/// list also has, most frequent first.
fn hero_candidates(paths: &[String], valve: &BTreeSet<String>) -> Vec<(String, usize)> {
    let mut counts = BTreeMap::<String, usize>::new();
    for path in paths.iter().filter(|path| valve.contains(*path)) {
        let segments = path.split('/').collect::<Vec<_>>();
        if let Some(hero) = skin_archive::hero_folder(&segments[..segments.len() - 1]) {
            *counts.entry(hero.to_string()).or_default() += 1;
        }
    }
    let mut counts = counts.into_iter().collect::<Vec<_>>();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    counts
}

fn heroes_of(candidates: &[(String, usize)]) -> Vec<String> {
    let Some((_, top)) = candidates.first() else {
        return Vec::new();
    };
    // The top name is always kept, however few paths an archive has.
    candidates
        .iter()
        .enumerate()
        .filter(|(index, (_, count))| {
            *index == 0
                || (count * 100 >= top * SECOND_NAME_PERCENT && *count >= SECOND_NAME_MIN_PATHS)
        })
        .map(|(_, (name, _))| name.clone())
        .collect()
}

fn skip(reason: &str, detail: impl Into<String>) -> Outcome {
    Outcome::Skipped(reason.to_string(), detail.into())
}

fn engine_name(catalog_id: &str) -> String {
    catalog_id
        .strip_prefix("heroes-")
        .unwrap_or(catalog_id)
        .to_string()
}

struct Inputs {
    zips: PathBuf,
    fetched: BTreeMap<String, (Vec<String>, String, u64)>,
    mirrors: Option<BTreeMap<String, Vec<String>>>,
    snapshot: String,
    valve: BTreeSet<String>,
    valve_sha: String,
    valve_entries: usize,
}

fn mirror_urls(inputs: &Inputs, id: &str, candidates: &[String]) -> Vec<String> {
    let Some(mirrors) = &inputs.mirrors else {
        return candidates.to_vec();
    };
    let working = mirrors.get(id).cloned().unwrap_or_default();
    candidates
        .iter()
        .filter(|url| working.contains(url))
        .cloned()
        .collect()
}

fn audit_item(inputs: &Inputs, item: &CatalogItem) -> Outcome {
    let Some((candidates, zip_sha, zip_size)) = inputs.fetched.get(&item.id) else {
        return skip(
            "no archive in the cache",
            "never fetched: the survey could not read its ZIP directory (no end-of-central-directory record)",
        );
    };
    let zip_path = inputs.zips.join(format!("{}.zip", item.id));
    let Ok(zip) = std::fs::read(&zip_path) else {
        return skip("no archive in the cache", "file missing");
    };
    assert_eq!(
        (&sha256_hex(&zip), zip.len() as u64),
        (zip_sha, *zip_size),
        "cache differs from fetched.json for {}",
        item.id
    );
    if zip.len() as u64 > content_store::MAX_ARTIFACT_BYTES {
        return skip(
            "archive over the 64 MiB limit",
            format!("ZIP is {} bytes", zip.len()),
        );
    }
    if let Err(code) = crate::archive_inspector::inspect_zip_bytes(&zip) {
        // The inspector's size bound is on the unpacked VPK.
        if code == "archive_size_invalid" {
            return skip(
                "archive over the 64 MiB limit",
                "the VPK inside the ZIP is over the unpacked-size bound of the archive inspector",
            );
        }
        return skip("ZIP refused by the archive inspector", code);
    }
    let mut archive = match zip::read::ZipArchive::new(std::io::Cursor::new(&zip[..])) {
        Ok(archive) => archive,
        Err(_) => return skip("ZIP refused by the archive inspector", "unreadable"),
    };
    if archive.len() != 1 {
        return skip(
            "ZIP does not hold exactly one pakNN_dir.vpk",
            format!("{} entries", archive.len()),
        );
    }
    let mut entry = archive.by_index(0).expect("entry");
    let inner_path = entry.name().to_string();
    let well_formed_name = inner_path.ends_with("_dir.vpk")
        && inner_path.len() <= 64
        && inner_path
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.'));
    if !entry.is_file() || !well_formed_name {
        return skip(
            "ZIP does not hold exactly one pakNN_dir.vpk",
            format!("entry {inner_path:?}"),
        );
    }
    if entry.size() > content_store::MAX_ARTIFACT_BYTES {
        return skip(
            "archive over the 64 MiB limit",
            format!("inner VPK is {} bytes", entry.size()),
        );
    }
    let mut inner = Vec::new();
    entry
        .by_ref()
        .take(content_store::MAX_ARTIFACT_BYTES + 1)
        .read_to_end(&mut inner)
        .expect("inner");
    drop(entry);
    drop(archive);
    let inner_sha = sha256_hex(&inner);
    // The same function the app uses to read it back.
    wardrobe::extract_pinned_entry(&zip, &inner_path, inner.len(), &inner_sha)
        .expect("pinned entry extracts");

    let directory = match crate::vpk::read_embedded_directory(&inner) {
        Ok(directory) => directory,
        Err(code) => return skip("VPK container unreadable", code),
    };
    let paths = directory
        .iter()
        .map(|entry| entry.path.to_ascii_lowercase())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    drop(directory);
    let candidates_by_count = hero_candidates(&paths, &inputs.valve);
    let heroes = heroes_of(&candidates_by_count);
    if heroes.is_empty() {
        return skip("no hero found in paths that exist in Valve's file list", "");
    }
    if heroes.len() > skin_archive::MAX_HEROES {
        return skip(
            "more hero names than one skin may carry",
            format!("{} names above {SECOND_NAME_PERCENT}%", heroes.len()),
        );
    }

    // Pass one lists every archive path that Valve ships and is not the
    // heroes'; the analyzer keeps the ones it actually classifies as shared
    // (right type, safe path). Pass two is the audited result.
    let probe = ScopeSpec {
        heroes: heroes.clone(),
        shared_paths: paths
            .iter()
            .filter(|path| {
                inputs.valve.contains(*path)
                    && skin_archive::path_is_safe(path)
                    && !skin_archive::hero_scoped(path, &heroes)
            })
            .cloned()
            .collect(),
        shared_policy: SharedPolicy::Install,
    };
    let probe_report = match skin_archive::analyze(&inner, &probe, &Limits::default()) {
        Ok(analysis) => analysis.report,
        Err(code) => return skip("analyzer refused the archive", code),
    };
    let spec = ScopeSpec {
        shared_paths: probe_report
            .shared_paths
            .iter()
            .map(|entry| entry.path.clone())
            .collect(),
        ..probe
    };
    let report = match skin_archive::analyze(&inner, &spec, &Limits::default()) {
        Ok(analysis) => analysis.report,
        Err(code) => return skip("analyzer refused the archive", code),
    };
    if let Some(code) = &report.install.blocked {
        let mut detail = report
            .rejected_paths
            .iter()
            .filter(|entry| entry.reason.is_hostile())
            .fold(BTreeMap::<String, usize>::new(), |mut acc, entry| {
                *acc.entry(format!("{:?}", entry.reason)).or_default() += 1;
                acc
            })
            .into_iter()
            .map(|(reason, count)| format!("{reason} x{count}"))
            .collect::<Vec<_>>()
            .join(", ");
        if detail.is_empty() {
            detail = code.clone();
        }
        return skip(&format!("analyzer blocked the install ({code})"), detail);
    }

    let audited = match wardrobe::audit_both_modes(
        &inner,
        &heroes,
        &spec.shared_paths,
        &inputs.valve_sha,
        inputs.valve_entries,
    ) {
        Ok(audited) => audited,
        Err(code) => return skip("analyzer refused the archive", code),
    };
    let name = engine_name(&item.id);
    let urls = mirror_urls(inputs, &item.id, candidates);
    if urls.is_empty() {
        return skip(
            "no mirror answers with the pinned size",
            "checked by HEAD request",
        );
    }
    let manifest = WardrobeManifest {
        schema_version: 2,
        kind: "wardrobe".to_string(),
        id: format!("{}.{name}", wardrobe::NAMESPACE),
        catalog_id: item.id.clone(),
        name: LocalizedName {
            ru: item.name.clone(),
            en: item.name.clone(),
        },
        author: item.author.clone(),
        heroes: heroes.clone(),
        shared_paths: spec.shared_paths.clone(),
        distribution: "internal_pilot".to_string(),
        source: WardrobeSource {
            urls: urls.clone(),
            catalog_snapshot: format!("Dota2PornFxWeb {}", inputs.snapshot),
            permission: "founder_reported_author_consent".to_string(),
        },
        archive: ArchivePin {
            zip_bytes: zip.len() as u64,
            zip_sha256: zip_sha.clone(),
            inner_path,
            inner_bytes: inner.len(),
            inner_sha256: inner_sha,
        },
        audited,
        verified_languages: Vec::new(),
        compatibility_note: None,
    };
    if let Err(code) = wardrobe::validate(&manifest) {
        return skip("manifest does not validate", code);
    }
    // The app's own load path must reproduce the audit from the stored bytes.
    let root = std::env::temp_dir().join(format!(
        "betterfy-audit-{}-{}",
        std::process::id(),
        item.id.len()
    ));
    std::fs::create_dir_all(&root).expect("temp root");
    content_store::store_pinned_resource(
        &root,
        manifest.archive.inner_bytes,
        &manifest.archive.inner_sha256,
        &inner,
    )
    .expect("stores");
    let loaded = wardrobe::load(&root, &manifest, SharedPolicy::Install).and_then(|install| {
        wardrobe::load(&root, &manifest, SharedPolicy::Strip).map(|strip| (install, strip))
    });
    std::fs::remove_dir_all(&root).expect("cleanup");
    let (install, strip) = match loaded {
        Ok(loaded) => loaded,
        Err(code) => return skip("the app's load path disagrees with the audit", code),
    };
    assert_eq!(install.resources.len(), manifest.audited.install_files);
    assert_eq!(strip.resources.len(), manifest.audited.strip_files);

    let stats = stats_of(&heroes, &urls, &report, &manifest.audited);
    Outcome::Emitted(Box::new(manifest), stats)
}

fn stats_of(
    heroes: &[String],
    urls: &[String],
    report: &SkinReport,
    audited: &AuditedResult,
) -> ItemStats {
    let mut rejected_reasons = BTreeMap::new();
    for entry in &report.rejected_paths {
        *rejected_reasons
            .entry(reason_name(entry.reason))
            .or_default() += 1;
    }
    let mut stripped_dirs = BTreeMap::new();
    for entry in &report.shared_paths {
        let directories = entry.path.rsplit_once('/').map_or("", |(dir, _)| dir);
        let prefix = directories.split('/').take(3).collect::<Vec<_>>().join("/");
        *stripped_dirs.entry(prefix).or_default() += 1;
    }
    ItemStats {
        heroes: heroes.to_vec(),
        hero_files: report.hero_scoped.files,
        author_files: report.author_owned.files,
        install_files: report.install.files,
        install_bytes: report.install.bytes,
        shared: report.shared.files,
        strip_bytes: audited.strip_bytes,
        rejected: report.rejected.files,
        renamed: report.renamed,
        mirrors: urls.len(),
        rejected_reasons,
        stripped_dirs,
    }
}

fn reason_name(reason: RejectReason) -> String {
    serde_json::to_value(reason)
        .expect("reason")
        .as_str()
        .expect("string")
        .to_string()
}

fn mib(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1_048_576.0)
}

fn percentile(sorted: &[u64], percent: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    sorted[((sorted.len() - 1) * percent) / 100]
}

struct Report {
    markdown: String,
    summary: String,
}

fn render_report(
    inputs: &Inputs,
    catalog: &[CatalogItem],
    emitted: &[(WardrobeManifest, ItemStats)],
    skipped: &[(String, String, String)],
) -> Report {
    let mut out = String::new();
    let heroes_total = catalog.iter().filter(|i| i.category == "heroes").count();
    let items_total = catalog.len() - heroes_total;
    let mut summary = String::new();
    macro_rules! both {
        ($($arg:tt)*) => {{ let line = format!($($arg)*); writeln!(out, "{line}").unwrap(); writeln!(summary, "{line}").unwrap(); }};
    }
    writeln!(out, "# Wardrobe catalog audit report\n").unwrap();
    writeln!(
        out,
        "Written by the local-only test `wardrobe_audit::generate_wardrobe_allowlist` \
         (`src-tauri/src/wardrobe_audit.rs`); do not edit by hand. It is the audit trail \
         for `src-tauri/wardrobe/manifests.json`. The run is a Mac run over archives \
         downloaded to a local cache: nothing here was run on Windows or in Dota, and \
         no archive bytes are part of the repository.\n"
    )
    .unwrap();
    writeln!(out, "## Inputs\n").unwrap();
    writeln!(out, "| Input | Value |\n| --- | --- |").unwrap();
    writeln!(
        out,
        "| Catalog | `src/webCatalog.json`, snapshot {} |",
        inputs.snapshot
    )
    .unwrap();
    writeln!(
        out,
        "| Items audited | {} (`heroes` {heroes_total}, `hero-items` {items_total}) |",
        catalog.len()
    )
    .unwrap();
    writeln!(
        out,
        "| Valve file list | SteamDatabase/GameTracking-Dota2 `game/dota/pak01_dir.txt`, {} entries, SHA-256 `{}` |",
        inputs.valve_entries, inputs.valve_sha
    )
    .unwrap();
    writeln!(
        out,
        "| Hero inference | folders of Valve-present paths: `models/heroes/<h>`, `models/items/<h>`, `particles/units/heroes/hero_<h>`, `particles/econ/items/<h>`, `materials/models/(heroes\\|items)/<h>`; every name with at least {SECOND_NAME_PERCENT}% of the top name's paths is kept (at most {}) |",
        skin_archive::MAX_HEROES
    )
    .unwrap();
    writeln!(
        out,
        "| Mirrors | {} |\n",
        if inputs.mirrors.is_some() {
            "candidate URLs checked by HEAD request: kept only when they answer 200 with the pinned ZIP size"
        } else {
            "candidate URLs of the fetch run, not individually checked"
        }
    )
    .unwrap();

    both!("Emitted {} of {} items.", emitted.len(), catalog.len());
    both!("Skipped {} of {} items.", skipped.len(), catalog.len());
    writeln!(out).unwrap();

    // Skipped, by reason.
    writeln!(out, "## Items that are not in the allowlist\n").unwrap();
    let mut by_reason = BTreeMap::<&str, Vec<&(String, String, String)>>::new();
    for row in skipped {
        by_reason.entry(row.1.as_str()).or_default().push(row);
    }
    writeln!(out, "| Reason | Items |\n| --- | ---: |").unwrap();
    for (reason, rows) in &by_reason {
        writeln!(out, "| {reason} | {} |", rows.len()).unwrap();
        summary.push_str(&format!("  skipped: {reason}: {}\n", rows.len()));
    }
    writeln!(out).unwrap();
    for (reason, rows) in &by_reason {
        writeln!(out, "### {reason} ({})\n", rows.len()).unwrap();
        for row in rows {
            if row.2.is_empty() {
                writeln!(out, "- `{}`", row.0).unwrap();
            } else {
                writeln!(out, "- `{}`: {}", row.0, row.2).unwrap();
            }
        }
        writeln!(out).unwrap();
    }

    // Emitted, in numbers.
    writeln!(out, "## Emitted items in numbers\n").unwrap();
    let mut bytes = emitted
        .iter()
        .map(|(_, s)| s.install_bytes)
        .collect::<Vec<_>>();
    bytes.sort_unstable();
    let total: u64 = bytes.iter().sum();
    let biggest = emitted
        .iter()
        .max_by_key(|(_, s)| s.install_bytes)
        .map(|(m, s)| (m.catalog_id.clone(), s.install_bytes));
    let multi = emitted.iter().filter(|(_, s)| s.heroes.len() > 1).count();
    let zero_shared = emitted.iter().filter(|(_, s)| s.shared == 0).count();
    let over_100 = emitted.iter().filter(|(_, s)| s.shared > 100).count();
    let renamed_items = emitted.iter().filter(|(_, s)| s.renamed > 0).count();
    let renamed_files: usize = emitted.iter().map(|(_, s)| s.renamed).sum();
    let two_mirrors = emitted.iter().filter(|(_, s)| s.mirrors == 2).count();
    let shared_total: usize = emitted.iter().map(|(_, s)| s.shared).sum();
    let strip_total: u64 = emitted.iter().map(|(_, s)| s.strip_bytes).sum();
    let files_total: usize = emitted.iter().map(|(_, s)| s.install_files).sum();
    let hero_files: usize = emitted.iter().map(|(_, s)| s.hero_files).sum();
    let author_files: usize = emitted.iter().map(|(_, s)| s.author_files).sum();
    both!(
        "Total install bytes of all emitted items in the default mode: {} MiB ({files_total} files); with the shared files left out (opt-in): {} MiB.",
        mib(total),
        mib(strip_total)
    );
    if let Some((id, size)) = biggest {
        both!("Biggest install: `{id}`, {} MiB.", mib(size));
    }
    both!(
        "Install size per item (MiB): median {}, p90 {}, p99 {}.",
        mib(percentile(&bytes, 50)),
        mib(percentile(&bytes, 90)),
        mib(percentile(&bytes, 99))
    );
    both!(
        "Items that override no shared Valve file: {zero_shared}; that override more than 100: {over_100}; shared files overridden in total: {shared_total}."
    );
    both!(
        "Hero-name ambiguity: {multi} items carry more than one internal hero name (a second name with at least {SECOND_NAME_PERCENT}% of the top name's Valve-present paths)."
    );
    both!("Paths folded to lower case: {renamed_files} files in {renamed_items} items.");
    both!(
        "Classes over all emitted items: {hero_files} hero-scoped files, {author_files} author-owned files (files the game does not ship, installed)."
    );
    both!(
        "Items with two verified mirrors: {two_mirrors}; with one: {}.",
        emitted.len() - two_mirrors
    );
    let mut reasons = BTreeMap::<String, usize>::new();
    for (_, s) in emitted {
        for (reason, count) in &s.rejected_reasons {
            *reasons.entry(reason.clone()).or_default() += count;
        }
    }
    writeln!(
        out,
        "\nDropped files by reason (not installed, listed per item in the plan):\n"
    )
    .unwrap();
    writeln!(out, "| Reason | Files |\n| --- | ---: |").unwrap();
    for (reason, count) in &reasons {
        writeln!(out, "| {reason} | {count} |").unwrap();
        summary.push_str(&format!("  dropped: {reason}: {count}\n"));
    }
    let mut dirs = BTreeMap::<String, usize>::new();
    for (_, s) in emitted {
        for (dir, count) in &s.stripped_dirs {
            *dirs.entry(dir.clone()).or_default() += count;
        }
    }
    let mut dirs = dirs.into_iter().collect::<Vec<_>>();
    dirs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    writeln!(
        out,
        "\nFolders where skins override shared Valve files most often (first three path segments):\n"
    )
    .unwrap();
    writeln!(out, "| Folder | Files |\n| --- | ---: |").unwrap();
    for (dir, count) in dirs.iter().take(25) {
        writeln!(out, "| `{dir}` | {count} |").unwrap();
    }

    writeln!(out, "\n## Emitted items\n").unwrap();
    writeln!(
        out,
        "Heroes are the inferred internal names. Files are what goes into the build; \
         shared is how many of the files override a shared Valve file (the opt-in mode leaves them out); dropped is files rejected by type.\n"
    )
    .unwrap();
    writeln!(
        out,
        "| Catalog ID | Heroes | Files | MiB | Shared | Dropped | Folded |\n| --- | --- | ---: | ---: | ---: | ---: | ---: |"
    )
    .unwrap();
    for (manifest, s) in emitted {
        writeln!(
            out,
            "| `{}` | {} | {} | {} | {} | {} | {} |",
            manifest.catalog_id,
            s.heroes.join(", "),
            s.install_files,
            mib(s.install_bytes),
            s.shared,
            s.rejected,
            s.renamed
        )
        .unwrap();
    }
    Report {
        markdown: out,
        summary,
    }
}

#[test]
#[ignore = "local-only generator; needs the archive cache named by BETTERFY_AUDIT_* variables"]
fn generate_wardrobe_allowlist() {
    let zips = env_path("BETTERFY_AUDIT_ZIPS").expect("BETTERFY_AUDIT_ZIPS");
    let fetched_path = env_path("BETTERFY_AUDIT_FETCHED").expect("BETTERFY_AUDIT_FETCHED");
    let catalog_path = env_path("BETTERFY_AUDIT_CATALOG").expect("BETTERFY_AUDIT_CATALOG");
    let valve_path = env_path("BETTERFY_AUDIT_VALVE").expect("BETTERFY_AUDIT_VALVE");
    let write = std::env::var("BETTERFY_AUDIT_WRITE").is_ok_and(|value| value == "1");

    let fetched_rows: Vec<Value> =
        serde_json::from_slice(&std::fs::read(fetched_path).expect("fetched")).expect("json");
    let fetched = fetched_rows
        .iter()
        .map(|row| {
            let id = row[0].as_str().expect("id").to_string();
            let urls = row[4]
                .as_array()
                .expect("urls")
                .iter()
                .map(|url| url.as_str().expect("url").to_string())
                .collect::<Vec<_>>();
            (
                id,
                (
                    urls,
                    row[2].as_str().expect("sha").to_string(),
                    row[3].as_u64().expect("size"),
                ),
            )
        })
        .collect();
    let mirrors = env_path("BETTERFY_AUDIT_MIRRORS").map(|path| {
        let value: Value =
            serde_json::from_slice(&std::fs::read(path).expect("mirrors")).expect("mirrors json");
        value
            .as_object()
            .expect("object")
            .iter()
            .map(|(id, entry)| {
                let size = entry["size"].as_u64().expect("size");
                let working = entry["mirrors"]
                    .as_array()
                    .expect("mirrors")
                    .iter()
                    .filter(|row| row[1] == 200 && row[2].as_u64() == Some(size))
                    .map(|row| row[0].as_str().expect("url").to_string())
                    .collect::<Vec<_>>();
                (id.clone(), working)
            })
            .collect::<BTreeMap<_, _>>()
    });
    let (snapshot, catalog) = read_catalog(&catalog_path);
    let (valve, valve_sha, valve_entries) = read_valve_list(&valve_path);
    let inputs = Inputs {
        zips,
        fetched,
        mirrors,
        snapshot,
        valve,
        valve_sha,
        valve_entries,
    };

    let mut emitted = Vec::new();
    let mut skipped = Vec::new();
    for (index, item) in catalog.iter().enumerate() {
        match audit_item(&inputs, item) {
            Outcome::Emitted(manifest, stats) => emitted.push((*manifest, stats)),
            Outcome::Skipped(reason, detail) => skipped.push((item.id.clone(), reason, detail)),
        }
        if index % 50 == 0 {
            println!("audited {index} / {}", catalog.len());
        }
    }
    emitted.sort_by(|a, b| a.0.id.cmp(&b.0.id));
    skipped.sort();

    // The ids must be unique after the `heroes-` prefix is dropped.
    let mut seen = BTreeSet::new();
    for (manifest, _) in &emitted {
        assert!(seen.insert(manifest.id.clone()), "{} twice", manifest.id);
    }

    let manifests = emitted.iter().map(|(m, _)| m.clone()).collect::<Vec<_>>();
    let manifests_json = serde_json::to_string(&manifests).expect("manifests json");
    let index = manifests
        .iter()
        .map(wardrobe::index_entry)
        .collect::<Vec<_>>();
    let index_json = serde_json::to_string(&index).expect("index json");
    let report = render_report(&inputs, &catalog, &emitted, &skipped);
    println!("{}", report.summary);
    println!(
        "manifests.json {} bytes, index.json {} bytes, report {} bytes",
        manifests_json.len(),
        index_json.len(),
        report.markdown.len()
    );
    if write {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        std::fs::write(root.join("wardrobe/manifests.json"), manifests_json).expect("write");
        std::fs::write(root.join("wardrobe/index.json"), index_json).expect("write");
        std::fs::write(
            root.join("../docs/WARDROBE_AUDIT_REPORT.md"),
            report.markdown,
        )
        .expect("write");
        println!("written");
    }
}

/// Local-only spot check of the whole path from the embedded allowlist to a
/// plan: for each catalog ID in `BETTERFY_AUDIT_SPOT` (comma separated) the
/// cached ZIP is unpacked exactly as the app does, stored, and built into a
/// plan in both modes; then all of them are built together, which shows the
/// conflicts between skins and the size budget. Nothing is written outside a
/// temporary folder.
#[test]
#[ignore = "local-only spot check; needs BETTERFY_AUDIT_ZIPS and BETTERFY_AUDIT_SPOT"]
fn spot_check_embedded_items() {
    let zips = env_path("BETTERFY_AUDIT_ZIPS").expect("BETTERFY_AUDIT_ZIPS");
    let spot = std::env::var("BETTERFY_AUDIT_SPOT").expect("BETTERFY_AUDIT_SPOT");
    let root = std::env::temp_dir().join(format!("betterfy-spot-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("temp root");
    let mut ids = Vec::new();
    for catalog_id in spot.split(',') {
        let item = wardrobe::find(catalog_id)
            .expect("registry")
            .unwrap_or_else(|| panic!("{catalog_id} is not in the allowlist"));
        let zip = std::fs::read(zips.join(format!("{catalog_id}.zip"))).expect("zip");
        assert_eq!(sha256_hex(&zip), item.archive.zip_sha256);
        // Exactly what `wardrobe::acquire` does after the download.
        let inner = wardrobe::extract_pinned_entry(
            &zip,
            &item.archive.inner_path,
            item.archive.inner_bytes,
            &item.archive.inner_sha256,
        )
        .expect("extracts");
        content_store::store_pinned_resource(
            &root,
            item.archive.inner_bytes,
            &item.archive.inner_sha256,
            &inner,
        )
        .expect("stores");
        for suffix in ["", ".strip"] {
            let id = format!("{}{suffix}", item.id);
            let started = std::time::Instant::now();
            let plan =
                crate::tree_pilot::plan_from_verified_store(&root, std::slice::from_ref(&id))
                    .expect("plan");
            let plan = serde_json::to_value(&plan).expect("json");
            println!(
                "{id}: {} resources, {} payload bytes, vpk {} bytes sha256 {} plan {}, shared {} (left out {}), dropped {}, conflicts {}, {:?}",
                plan["resourceCount"],
                plan["resourceBytes"],
                plan["vpkBytes"],
                plan["vpkSha256"],
                plan["planId"],
                plan["wardrobe"][0]["report"]["shared"]["files"],
                plan["wardrobe"][0]["report"]["install"]["strippedShared"],
                plan["wardrobe"][0]["report"]["rejected"]["files"],
                plan["wardrobeConflicts"].as_array().map_or(0, Vec::len),
                started.elapsed()
            );
            assert_eq!(plan["wardrobe"][0]["packageId"], id.as_str());
        }
        ids.push(item.id.clone());
    }
    if ids.len() > 1 {
        match crate::tree_pilot::plan_from_verified_store(&root, &ids) {
            Ok(plan) => {
                let plan = serde_json::to_value(&plan).expect("json");
                println!(
                    "together: {} resources, {} payload bytes, vpk {} bytes, overrides {}, wardrobe conflicts {}",
                    plan["resourceCount"],
                    plan["resourceBytes"],
                    plan["vpkBytes"],
                    plan["overriddenResources"],
                    plan["wardrobeConflicts"].as_array().map_or(0, Vec::len)
                );
            }
            Err(code) => println!("together: refused with {code}"),
        }
        let (files, bytes) = wardrobe::install_footprint(&ids);
        println!("footprint of the selection: {files} files, {bytes} bytes");
    }
    std::fs::remove_dir_all(root).expect("cleanup");
}

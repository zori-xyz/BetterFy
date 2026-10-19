//! Local-only generator of the wardrobe allowlist (`#[ignore]`d test).
//!
//! It turns a cache of downloaded archives into the embedded manifests. It
//! never runs in CI and never touches the network: everything it reads is a
//! local file named by an environment variable.
//!
//! ```text
//! BETTERFY_AUDIT_ZIPS      directory holding <catalogId>.zip (a ZIP item) or
//!                          <catalogId>.vpk (a raw VPK item) for every item
//! BETTERFY_AUDIT_FETCHED   fetched.json files, comma separated:
//!                          [[id, source, sha256, size, [urls]], ..]
//! BETTERFY_AUDIT_MIRRORS   (optional) mirrors json files, comma separated: which
//!                          candidate URLs answer 200 with the pinned size;
//!                          without it every candidate URL is listed
//! BETTERFY_AUDIT_CATALOG   src/webCatalog.json
//! BETTERFY_AUDIT_VALVE     Valve's pak01_dir.txt (SteamDatabase/GameTracking-Dota2)
//! BETTERFY_AUDIT_WRITE     "1" to write src-tauri/wardrobe/{manifests*,index}.json
//!                          and docs/WARDROBE_AUDIT_REPORT.md (dry run otherwise)
//! cargo test --release generate_wardrobe_allowlist -- --ignored --nocapture
//! ```
//!
//! For every catalog item of an enabled category (`skin_archive::CATEGORIES`)
//! it opens the cached file (a ZIP holding one VPK, or the raw VPK), computes
//! the shared paths against Valve's file list (for a hero skin after inferring
//! the hero names from the paths that exist there), runs the analyzer and, when
//! the result is clean, emits a manifest. Anything that cannot be audited
//! cleanly is not emitted and is listed with its reason in the report. The
//! output is sorted and carries no timestamp, so the same inputs always produce
//! the same bytes.

use crate::content_store;
use crate::package_registry::LocalizedName;
use crate::skin_archive::{
    self, Limits, RejectReason, ScopeKind, ScopeSpec, SharedPolicy, SkinReport,
};
use crate::wardrobe::{self, ArchivePin, SourceKind, WardrobeManifest, WardrobeSource};
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

/// Catalog categories that are not installable, and why. The report lists each
/// with its item count; `docs/CATALOG_CATEGORIES.md` carries the same reasons.
/// A category is "not enabled" when it is neither here nor in
/// `skin_archive::CATEGORIES` only by mistake, and the generator refuses to run
/// then.
const NOT_ENABLED: &[(&str, &str)] = &[
    (
        "backgrounds",
        "Panorama layouts, styles and video (vxml_c, vcss_c, webm): the layout proof covers plain image wrappers only; a separate proof is in progress",
    ),
    (
        "huds",
        "Panorama layouts and styles (vcss_c with textures): not covered by the layout proof",
    ),
    (
        "versus-screens",
        "Panorama layouts (vxml_c): not covered by the layout proof",
    ),
    (
        "cursors",
        "loose cursor files in a ZIP (no VPK): no verified install target",
    ),
    (
        "fonts",
        "loose font files in a ZIP (no VPK): no verified install target",
    ),
    (
        "tools",
        "programs and scripts, not game content: BetterFy installs none",
    ),
    ("guides", "reading material, not game content"),
    ("news", "links, not game content"),
    ("sites", "links, not game content"),
    (
        "other",
        "scripts and text (scripts/*.txt, unproven layouts): not installed",
    ),
    (
        "pedestal",
        "the files do not exist on either mirror (HTTP 404 on Hugging Face, absent from the GitHub tree)",
    ),
    (
        "tormentor",
        "the files do not exist on either mirror (HTTP 404 on Hugging Face, absent from the GitHub tree)",
    ),
    (
        "packs",
        "collections of other items, with no file of their own",
    ),
];

struct CatalogItem {
    id: String,
    name: String,
    category: String,
    author: String,
    file: Option<String>,
}

#[derive(Clone, Debug)]
struct ItemStats {
    category: String,
    scope: ScopeKind,
    heroes: Vec<String>,
    download_bytes: u64,
    hero_files: usize,
    author_files: usize,
    install_files: usize,
    install_bytes: u64,
    shared: usize,
    strip_bytes: u64,
    rejected: usize,
    ignored: usize,
    renamed: usize,
    mirrors: usize,
    rejected_reasons: BTreeMap<String, usize>,
    /// Every file really dropped (not an ignored one): path, reason, bytes.
    dropped: Vec<(String, RejectReason, u64)>,
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

fn env_paths(name: &str) -> Option<Vec<PathBuf>> {
    std::env::var(name)
        .ok()
        .map(|value| value.split(',').map(PathBuf::from).collect())
}

fn read_catalog(path: &Path) -> (String, Vec<CatalogItem>, BTreeMap<String, usize>) {
    let value: Value =
        serde_json::from_slice(&std::fs::read(path).expect("catalog")).expect("json");
    let snapshot = value["snapshot"].as_str().expect("snapshot").to_string();
    let mut all_categories = BTreeMap::<String, usize>::new();
    let mut items = Vec::new();
    for item in value["items"].as_array().expect("items") {
        let category = item["category"].as_str().expect("category").to_string();
        *all_categories.entry(category.clone()).or_default() += 1;
        if skin_archive::category_rule(&category).is_none() {
            continue;
        }
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
        items.push(CatalogItem {
            id: item["id"].as_str().expect("id").to_string(),
            name: item["name"].as_str().expect("name").trim().to_string(),
            category,
            author: if authors.is_empty() {
                "Not credited in the catalog".to_string()
            } else {
                authors.join(", ")
            },
            file: item["file"].as_str().map(str::to_string),
        });
    }
    (snapshot, items, all_categories)
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

/// The engine name: hero skins drop the `heroes-` prefix (as the first
/// allowlist always did); every other catalog ID is kept whole, so two
/// categories can never collide. A trailing hyphen in the upstream ID is not a
/// valid engine name and is trimmed.
fn engine_name(catalog_id: &str) -> String {
    catalog_id
        .strip_prefix("heroes-")
        .unwrap_or(catalog_id)
        .trim_end_matches('-')
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
    let rule = skin_archive::category_rule(&item.category).expect("enabled category");
    let Some(file) = item.file.as_deref() else {
        return skip("the catalog item has no file", "");
    };
    if file.starts_with("http") {
        return skip(
            "the catalog's file is an external link, not a mirrored file",
            file,
        );
    }
    let kind = if file.to_ascii_lowercase().ends_with(".vpk") {
        SourceKind::Vpk
    } else if file.to_ascii_lowercase().ends_with(".zip") {
        SourceKind::Zip
    } else {
        return skip("the catalog file is neither a ZIP nor a VPK", file);
    };
    let Some((candidates, download_sha, download_size)) = inputs.fetched.get(&item.id) else {
        return skip(
            "no archive in the cache",
            "never fetched: the survey could not read its ZIP directory or no mirror had the file",
        );
    };
    let extension = if kind == SourceKind::Vpk {
        "vpk"
    } else {
        "zip"
    };
    let cached_path = inputs.zips.join(format!("{}.{extension}", item.id));
    let Ok(downloaded) = std::fs::read(&cached_path) else {
        return skip("no archive in the cache", "file missing");
    };
    assert_eq!(
        (&sha256_hex(&downloaded), downloaded.len() as u64),
        (download_sha, *download_size),
        "cache differs from fetched.json for {}",
        item.id
    );
    if downloaded.len() as u64 > content_store::MAX_ARTIFACT_BYTES {
        return skip(
            "archive over the 256 MiB limit",
            format!("the file is {} bytes", downloaded.len()),
        );
    }
    let (inner_path, inner) = match kind {
        SourceKind::Vpk => (String::new(), downloaded.clone()),
        SourceKind::Zip => {
            if let Err(code) = crate::archive_inspector::inspect_zip_bytes(&downloaded) {
                // The inspector's size bound is on the unpacked VPK.
                if code == "archive_size_invalid" {
                    return skip(
                        "archive over the 256 MiB limit",
                        "the VPK inside the ZIP is over the unpacked-size bound of the archive inspector",
                    );
                }
                return skip("ZIP refused by the archive inspector", code);
            }
            let mut archive =
                match zip::read::ZipArchive::new(std::io::Cursor::new(&downloaded[..])) {
                    Ok(archive) => archive,
                    Err(_) => return skip("ZIP refused by the archive inspector", "unreadable"),
                };
            let entry_names = archive
                .file_names()
                .map(str::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            if archive.len() != 1 {
                return skip(
                    "ZIP does not hold exactly one pakNN_dir.vpk",
                    format!("{} entries: {entry_names}", archive.len()),
                );
            }
            let mut entry = archive.by_index(0).expect("entry");
            let inner_path = entry.name().to_string();
            let well_formed_name = inner_path.ends_with("_dir.vpk")
                && inner_path.len() <= 64
                && inner_path.bytes().all(|b| {
                    b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.')
                });
            if !entry.is_file() || !well_formed_name {
                return skip(
                    "ZIP does not hold exactly one pakNN_dir.vpk",
                    format!("entry {inner_path:?}"),
                );
            }
            if entry.size() > content_store::MAX_ARTIFACT_BYTES {
                return skip(
                    "archive over the 256 MiB limit",
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
            wardrobe::extract_pinned_entry(&downloaded, &inner_path, inner.len(), &inner_sha)
                .expect("pinned entry extracts");
            (inner_path, inner)
        }
    };
    let inner_sha = sha256_hex(&inner);

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
    let heroes = match rule.scope {
        ScopeKind::Hero => {
            let heroes = heroes_of(&hero_candidates(&paths, &inputs.valve));
            if heroes.is_empty() {
                return skip("no hero found in paths that exist in Valve's file list", "");
            }
            if heroes.len() > skin_archive::MAX_HEROES {
                return skip(
                    "more hero names than one skin may carry",
                    format!("{} names above {SECOND_NAME_PERCENT}%", heroes.len()),
                );
            }
            heroes
        }
        ScopeKind::None => Vec::new(),
    };

    // Pass one lists every archive path that Valve ships and is not the
    // heroes'; the analyzer keeps the ones it actually classifies as shared
    // (right type, safe path). Pass two is the audited result.
    let probe = ScopeSpec {
        kind: rule.scope,
        types: rule.types,
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

    let audited =
        match wardrobe::audit_archive(&inner, &spec, &inputs.valve_sha, inputs.valve_entries) {
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
    let archive = match kind {
        SourceKind::Zip => ArchivePin {
            kind,
            zip_bytes: downloaded.len() as u64,
            zip_sha256: download_sha.clone(),
            inner_path,
            inner_bytes: inner.len(),
            inner_sha256: inner_sha,
        },
        SourceKind::Vpk => ArchivePin {
            kind,
            zip_bytes: 0,
            zip_sha256: String::new(),
            inner_path: String::new(),
            inner_bytes: inner.len(),
            inner_sha256: inner_sha,
        },
    };
    let manifest = WardrobeManifest {
        schema_version: 3,
        kind: "wardrobe".to_string(),
        id: format!("{}.{name}", wardrobe::NAMESPACE),
        catalog_id: item.id.clone(),
        category: item.category.clone(),
        scope: rule.scope,
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
        archive,
        audited,
        verified_languages: Vec::new(),
        compatibility_note: None,
    };
    if let Err(code) = wardrobe::validate(&manifest) {
        return skip("manifest does not validate", code);
    }
    // The app's own load path must reproduce the audit from the stored bytes.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "betterfy-audit-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).expect("temp root");
    content_store::store_pinned_resource(
        &root,
        manifest.archive.inner_bytes,
        &manifest.archive.inner_sha256,
        &inner,
    )
    .expect("stores");
    let install = wardrobe::load(&root, &manifest, SharedPolicy::Install);
    let strip = match rule.scope {
        ScopeKind::Hero => Some(wardrobe::load(&root, &manifest, SharedPolicy::Strip)),
        ScopeKind::None => None,
    };
    std::fs::remove_dir_all(&root).expect("cleanup");
    let install = match install {
        Ok(loaded) => loaded,
        Err(code) => return skip("the app's load path disagrees with the audit", code),
    };
    assert_eq!(install.resources.len(), manifest.audited.install_files);
    if let Some(strip) = strip {
        match strip {
            Ok(strip) => assert_eq!(strip.resources.len(), manifest.audited.strip_files),
            Err(code) => return skip("the app's load path disagrees with the audit", code),
        }
    }

    let stats = stats_of(&manifest, &urls, &report);
    Outcome::Emitted(Box::new(manifest), stats)
}

fn stats_of(manifest: &WardrobeManifest, urls: &[String], report: &SkinReport) -> ItemStats {
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
        category: manifest.category.clone(),
        scope: manifest.scope,
        heroes: manifest.heroes.clone(),
        download_bytes: manifest.archive.download_bytes(),
        hero_files: report.hero_scoped.files,
        author_files: report.author_owned.files,
        install_files: report.install.files,
        install_bytes: report.install.bytes,
        shared: report.shared.files,
        strip_bytes: manifest.audited.strip_bytes,
        rejected: report.rejected.files,
        ignored: report.ignored.files,
        renamed: report.renamed,
        mirrors: urls.len(),
        rejected_reasons,
        dropped: report
            .rejected_paths
            .iter()
            .filter(|entry| !entry.reason.is_ignored())
            .map(|entry| (entry.path.clone(), entry.reason, entry.bytes))
            .collect(),
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

fn type_policy_name(types: skin_archive::TypePolicy) -> &'static str {
    match types {
        skin_archive::TypePolicy::Resources => "compiled resources",
        skin_archive::TypePolicy::Sounds => "sounds",
        skin_archive::TypePolicy::Textures => "panorama textures",
        skin_archive::TypePolicy::Materials => "materials and textures",
    }
}

fn render_report(
    inputs: &Inputs,
    catalog: &[CatalogItem],
    all_categories: &BTreeMap<String, usize>,
    emitted: &[(WardrobeManifest, ItemStats)],
    skipped: &[(String, String, String)],
    shard_sizes: &[(String, usize)],
    index_len: usize,
) -> Report {
    let mut out = String::new();
    let mut summary = String::new();
    macro_rules! both {
        ($($arg:tt)*) => {{ let line = format!($($arg)*); writeln!(out, "{line}").unwrap(); writeln!(summary, "{line}").unwrap(); }};
    }
    writeln!(out, "# Wardrobe catalog audit report\n").unwrap();
    writeln!(
        out,
        "Written by the local-only test `wardrobe_audit::generate_wardrobe_allowlist` \
         (`src-tauri/src/wardrobe_audit.rs`); do not edit by hand. It is the audit trail \
         for `src-tauri/wardrobe/manifests*.json`. The run is a Mac run over archives \
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
        "| Items audited | {} in {} enabled categories |",
        catalog.len(),
        skin_archive::CATEGORIES.len()
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
        "| Hero inference (hero skins only) | folders of Valve-present paths: `models/heroes/<h>`, `models/items/<h>`, `particles/units/heroes/hero_<h>`, `particles/econ/items/<h>`, `materials/models/(heroes\\|items)/<h>`; every name with at least {SECOND_NAME_PERCENT}% of the top name's paths is kept (at most {}) |",
        skin_archive::MAX_HEROES
    )
    .unwrap();
    writeln!(
        out,
        "| Other categories | no hero: every accepted file installs; \"overrides a Valve file\" and \"new file\" are computed against the same list and pinned (information, not a policy) |"
    )
    .unwrap();
    writeln!(
        out,
        "| Mirrors | {} |",
        if inputs.mirrors.is_some() {
            "candidate URLs checked by HEAD request: kept only when they answer 200 with the pinned size"
        } else {
            "candidate URLs of the fetch run, not individually checked"
        }
    )
    .unwrap();
    let embedded_total: usize = shard_sizes.iter().map(|(_, size)| size).sum();
    writeln!(
        out,
        "| Embedded allowlist | {} files, {:.2} MB in all ({}); interface copy `index.json` {:.2} MB |\n",
        shard_sizes.len(),
        embedded_total as f64 / 1_000_000.0,
        shard_sizes
            .iter()
            .map(|(name, size)| format!("`{name}` {:.2} MB", *size as f64 / 1_000_000.0))
            .collect::<Vec<_>>()
            .join(", "),
        index_len as f64 / 1_000_000.0
    )
    .unwrap();

    both!("Emitted {} of {} items.", emitted.len(), catalog.len());
    both!("Skipped {} of {} items.", skipped.len(), catalog.len());
    writeln!(out).unwrap();

    // Categories at a glance.
    writeln!(out, "## Categories\n").unwrap();
    writeln!(
        out,
        "| Category | Scope | Types | Items | Emitted | Skipped | Install MiB | Overrides a Valve file | New files |\n| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |"
    )
    .unwrap();
    for rule in skin_archive::CATEGORIES {
        let in_catalog = catalog.iter().filter(|i| i.category == rule.name).count();
        let rows = emitted
            .iter()
            .filter(|(_, s)| s.category == rule.name)
            .collect::<Vec<_>>();
        let bytes: u64 = rows.iter().map(|(_, s)| s.install_bytes).sum();
        let shared: usize = rows.iter().map(|(_, s)| s.shared).sum();
        let new: usize = rows.iter().map(|(_, s)| s.author_files).sum();
        writeln!(
            out,
            "| `{}` | {} | {} | {in_catalog} | {} | {} | {} | {shared} | {new} |",
            rule.name,
            if rule.scope == ScopeKind::Hero {
                "hero"
            } else {
                "none"
            },
            type_policy_name(rule.types),
            rows.len(),
            in_catalog - rows.len(),
            mib(bytes)
        )
        .unwrap();
    }
    writeln!(out).unwrap();
    writeln!(out, "## Categories that are not enabled\n").unwrap();
    writeln!(out, "| Category | Items | Why |\n| --- | ---: | --- |").unwrap();
    for (category, reason) in NOT_ENABLED {
        let items = all_categories.get(*category).copied().unwrap_or(0);
        writeln!(out, "| `{category}` | {items} | {reason} |").unwrap();
    }
    // `terrains` is enabled for raw VPKs only.
    let terrain_zips = skipped
        .iter()
        .filter(|row| row.0.starts_with("terrains-") && row.1.starts_with("ZIP does not hold"))
        .count();
    writeln!(
        out,
        "| `terrains` (ZIP items) | {terrain_zips} | each ZIP holds `maps/dota.vpk` and a guide, not a `pakNN_dir.vpk`: a whole-map replacement, installed through the separate map allowlist (`src-tauri/maps`), not this one |"
    )
    .unwrap();
    for category in all_categories.keys() {
        assert!(
            skin_archive::category_rule(category).is_some()
                || NOT_ENABLED.iter().any(|(name, _)| name == category),
            "category {category} is neither enabled nor listed as not enabled"
        );
    }
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
    let download_total: u64 = emitted.iter().map(|(_, s)| s.download_bytes).sum();
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
    let strip_total: u64 = emitted
        .iter()
        .filter(|(_, s)| s.scope == ScopeKind::Hero)
        .map(|(_, s)| s.strip_bytes)
        .sum();
    let hero_install_total: u64 = emitted
        .iter()
        .filter(|(_, s)| s.scope == ScopeKind::Hero)
        .map(|(_, s)| s.install_bytes)
        .sum();
    let files_total: usize = emitted.iter().map(|(_, s)| s.install_files).sum();
    let hero_files: usize = emitted.iter().map(|(_, s)| s.hero_files).sum();
    let author_files: usize = emitted.iter().map(|(_, s)| s.author_files).sum();
    both!(
        "Total install bytes of all emitted items: {} MiB ({files_total} files) from {} MiB of downloads; hero skins with the shared files left out (opt-in): {} MiB of {} MiB.",
        mib(total),
        mib(download_total),
        mib(strip_total),
        mib(hero_install_total)
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
        "Items that override no Valve file: {zero_shared}; that override more than 100: {over_100}; Valve files overridden in total: {shared_total}."
    );
    both!(
        "Hero-name ambiguity: {multi} items carry more than one internal hero name (a second name with at least {SECOND_NAME_PERCENT}% of the top name's Valve-present paths)."
    );
    both!("Paths folded to lower case: {renamed_files} files in {renamed_items} items.");
    both!(
        "Classes over all emitted items: {hero_files} hero-scoped files, {author_files} files the game does not ship (installed), {shared_total} files that override a Valve file (installed)."
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
    let ignored_reasons = ["uncompiled_source", "not_game_file"];
    let ignored_total: usize = emitted.iter().map(|(_, s)| s.ignored).sum();
    let dropped_total: usize = emitted.iter().map(|(_, s)| s.rejected).sum();
    both!(
        "Files left out of the installs: {dropped_total} dropped (game files or unprovable ones BetterFy does not install), {ignored_total} ignored (not game files)."
    );
    writeln!(
        out,
        "\nFiles ignored because they are not game files (uncompiled sources, readmes and tool leftovers; the game loads none of them from a VPK, so leaving them out changes nothing in game):\n"
    )
    .unwrap();
    writeln!(out, "| Reason | Files |\n| --- | ---: |").unwrap();
    for (reason, count) in reasons
        .iter()
        .filter(|(reason, _)| ignored_reasons.contains(&reason.as_str()))
    {
        writeln!(out, "| {reason} | {count} |").unwrap();
        summary.push_str(&format!("  ignored: {reason}: {count}\n"));
    }
    writeln!(
        out,
        "\nFiles dropped for real (not installed, listed per item in the plan):\n"
    )
    .unwrap();
    writeln!(out, "| Reason | Files |\n| --- | ---: |").unwrap();
    for (reason, count) in reasons
        .iter()
        .filter(|(reason, _)| !ignored_reasons.contains(&reason.as_str()))
    {
        writeln!(out, "| {reason} | {count} |").unwrap();
        summary.push_str(&format!("  dropped: {reason}: {count}\n"));
    }
    // Dropped for real, by reason and file type.
    let mut by_type = BTreeMap::<(String, String), (usize, u64, BTreeSet<&str>)>::new();
    for (manifest, s) in emitted {
        for (path, reason, bytes) in &s.dropped {
            let name = path.rsplit('/').next().unwrap_or(path);
            let extension = name.rsplit_once('.').map_or("", |(_, extension)| extension);
            let entry = by_type
                .entry((reason_name(*reason), extension.to_ascii_lowercase()))
                .or_default();
            entry.0 += 1;
            entry.1 += bytes;
            entry.2.insert(manifest.catalog_id.as_str());
        }
    }
    writeln!(
        out,
        "\nFiles dropped for real, by reason and file type:\n\n| Reason | Type | Files | MiB | Items |\n| --- | --- | ---: | ---: | ---: |"
    )
    .unwrap();
    for ((reason, extension), (files, bytes, items)) in &by_type {
        writeln!(
            out,
            "| {reason} | `{extension}` | {files} | {} | {} |",
            mib(*bytes),
            items.len()
        )
        .unwrap();
    }
    // Everything that is not a plain unsupported type is listed path by path:
    // these are the files a reader should look at.
    writeln!(
        out,
        "\nDropped for a reason other than an unsupported type, listed by item:\n"
    )
    .unwrap();
    for (manifest, s) in emitted {
        for (path, reason, bytes) in &s.dropped {
            if *reason != RejectReason::TypeNotAllowed {
                writeln!(
                    out,
                    "- `{}`: `{path}` ({bytes} bytes): {}",
                    manifest.catalog_id,
                    reason_name(*reason)
                )
                .unwrap();
            }
        }
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
        "\nFolders where items override Valve files most often (first three path segments):\n"
    )
    .unwrap();
    writeln!(out, "| Folder | Files |\n| --- | ---: |").unwrap();
    for (dir, count) in dirs.iter().take(25) {
        writeln!(out, "| `{dir}` | {count} |").unwrap();
    }

    // One section per category.
    writeln!(out, "\n## Per category\n").unwrap();
    for rule in skin_archive::CATEGORIES {
        let rows = emitted
            .iter()
            .filter(|(_, s)| s.category == rule.name)
            .collect::<Vec<_>>();
        let skipped_rows = skipped
            .iter()
            .filter(|row| row.0.starts_with(&format!("{}-", rule.name)))
            .filter(|row| {
                catalog
                    .iter()
                    .any(|item| item.id == row.0 && item.category == rule.name)
            })
            .collect::<Vec<_>>();
        writeln!(out, "### `{}`\n", rule.name).unwrap();
        if rows.is_empty() {
            writeln!(out, "No item emitted ({} skipped).\n", skipped_rows.len()).unwrap();
        } else {
            let mut sizes = rows
                .iter()
                .map(|(_, s)| s.install_bytes)
                .collect::<Vec<_>>();
            sizes.sort_unstable();
            let total: u64 = sizes.iter().sum();
            let files: usize = rows.iter().map(|(_, s)| s.install_files).sum();
            let shared: usize = rows.iter().map(|(_, s)| s.shared).sum();
            let new: usize = rows.iter().map(|(_, s)| s.author_files).sum();
            let dropped: usize = rows.iter().map(|(_, s)| s.rejected).sum();
            let ignored: usize = rows.iter().map(|(_, s)| s.ignored).sum();
            let download: u64 = rows.iter().map(|(_, s)| s.download_bytes).sum();
            let kinds = rows
                .iter()
                .map(|(m, _)| format!("{:?}", m.archive.kind).to_ascii_lowercase())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(", ");
            let biggest = rows
                .iter()
                .max_by_key(|(_, s)| s.install_bytes)
                .map(|(m, s)| format!("`{}` {} MiB", m.catalog_id, mib(s.install_bytes)))
                .unwrap_or_default();
            writeln!(
                out,
                "- Emitted {} of {} ({} skipped); source: {kinds}; downloads {} MiB.",
                rows.len(),
                rows.len() + skipped_rows.len(),
                skipped_rows.len(),
                mib(download)
            )
            .unwrap();
            writeln!(
                out,
                "- Install: {files} files, {} MiB in all; per item median {} MiB, largest {biggest}.",
                mib(total),
                mib(percentile(&sizes, 50))
            )
            .unwrap();
            writeln!(
                out,
                "- Overrides a Valve file: {shared} files; new files (the game does not ship them): {new}; dropped: {dropped}; ignored: {ignored}."
            )
            .unwrap();
        }
        for row in &skipped_rows {
            if row.2.is_empty() {
                writeln!(out, "- skipped `{}`: {}", row.0, row.1).unwrap();
            } else {
                writeln!(out, "- skipped `{}`: {} ({})", row.0, row.1, row.2).unwrap();
            }
        }
        writeln!(out).unwrap();
    }

    writeln!(out, "## Emitted items\n").unwrap();
    writeln!(
        out,
        "Heroes are the inferred internal names (hero skins only). Files are what goes into the build; \
         overrides is how many of the files override a Valve file (for a hero skin, the opt-in mode leaves them out); dropped is files really left out and ignored is files that are not game files (see above).\n"
    )
    .unwrap();
    writeln!(
        out,
        "| Catalog ID | Heroes | Files | MiB | Overrides | Dropped | Ignored | Folded |\n| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |"
    )
    .unwrap();
    for (manifest, s) in emitted {
        writeln!(
            out,
            "| `{}` | {} | {} | {} | {} | {} | {} | {} |",
            manifest.catalog_id,
            s.heroes.join(", "),
            s.install_files,
            mib(s.install_bytes),
            s.shared,
            s.rejected,
            s.ignored,
            s.renamed
        )
        .unwrap();
    }
    Report {
        markdown: out,
        summary,
    }
}

fn read_pairs(paths: &[PathBuf]) -> Vec<Value> {
    paths
        .iter()
        .flat_map(|path| {
            let value: Value =
                serde_json::from_slice(&std::fs::read(path).expect("json file")).expect("json");
            match value {
                Value::Array(rows) => rows,
                other => vec![other],
            }
        })
        .collect()
}

#[test]
#[ignore = "local-only generator; needs the archive cache named by BETTERFY_AUDIT_* variables"]
fn generate_wardrobe_allowlist() {
    let zips = env_path("BETTERFY_AUDIT_ZIPS").expect("BETTERFY_AUDIT_ZIPS");
    let fetched_paths = env_paths("BETTERFY_AUDIT_FETCHED").expect("BETTERFY_AUDIT_FETCHED");
    let catalog_path = env_path("BETTERFY_AUDIT_CATALOG").expect("BETTERFY_AUDIT_CATALOG");
    let valve_path = env_path("BETTERFY_AUDIT_VALVE").expect("BETTERFY_AUDIT_VALVE");
    let write = std::env::var("BETTERFY_AUDIT_WRITE").is_ok_and(|value| value == "1");

    let fetched = read_pairs(&fetched_paths)
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
    let mirrors = env_paths("BETTERFY_AUDIT_MIRRORS").map(|paths| {
        let mut all = BTreeMap::new();
        for path in paths {
            let value: Value = serde_json::from_slice(&std::fs::read(path).expect("mirrors"))
                .expect("mirrors json");
            for (id, entry) in value.as_object().expect("object") {
                let size = entry["size"].as_u64().expect("size");
                // The size is the downloaded file's (0 when nothing answered).
                let working = entry["mirrors"]
                    .as_array()
                    .expect("mirrors")
                    .iter()
                    .filter(|row| row[1] == 200 && row[2].as_u64() == Some(size))
                    .map(|row| row[0].as_str().expect("url").to_string())
                    .collect::<Vec<_>>();
                all.insert(id.clone(), working);
            }
        }
        all
    });
    let (snapshot, catalog, all_categories) = read_catalog(&catalog_path);
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

    // One file per shard (see `wardrobe::SHARDS`).
    let mut shard_sizes = Vec::new();
    let mut shard_json = Vec::new();
    for shard in wardrobe::SHARDS {
        let manifests = emitted
            .iter()
            .filter(|(m, _)| shard.categories.contains(&m.category.as_str()))
            .map(|(m, _)| m.clone())
            .collect::<Vec<_>>();
        let json = serde_json::to_string(&manifests).expect("manifests json");
        shard_sizes.push((shard.file.to_string(), json.len()));
        shard_json.push((shard.file, json));
    }
    let covered: usize = shard_json
        .iter()
        .map(|(_, json)| {
            serde_json::from_str::<Vec<Value>>(json)
                .expect("json")
                .len()
        })
        .sum();
    assert_eq!(covered, emitted.len(), "an emitted category has no shard");
    let manifests = emitted.iter().map(|(m, _)| m.clone()).collect::<Vec<_>>();
    let index = manifests
        .iter()
        .map(wardrobe::index_entry)
        .collect::<Vec<_>>();
    let index_json = serde_json::to_string(&index).expect("index json");
    let report = render_report(
        &inputs,
        &catalog,
        &all_categories,
        &emitted,
        &skipped,
        &shard_sizes,
        index_json.len(),
    );
    println!("{}", report.summary);
    println!(
        "shards {shard_sizes:?}, index.json {} bytes, report {} bytes",
        index_json.len(),
        report.markdown.len()
    );
    if write {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for (file, json) in shard_json {
            std::fs::write(root.join("wardrobe").join(file), json).expect("write");
        }
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
/// cached file is unpacked exactly as the app does, stored, and built into a
/// plan twice (the two builds must be byte for byte identical; hero skins also
/// in the opt-in mode); then all of them are built together, which shows the
/// conflicts between items and the size budget. Nothing is written outside a
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
        // Exactly what `wardrobe::acquire` does after the download.
        let inner = match item.archive.kind {
            SourceKind::Zip => {
                let zip = std::fs::read(zips.join(format!("{catalog_id}.zip"))).expect("zip");
                assert_eq!(sha256_hex(&zip), item.archive.zip_sha256);
                wardrobe::extract_pinned_entry(
                    &zip,
                    &item.archive.inner_path,
                    item.archive.inner_bytes,
                    &item.archive.inner_sha256,
                )
                .expect("extracts")
            }
            SourceKind::Vpk => {
                let vpk = std::fs::read(zips.join(format!("{catalog_id}.vpk"))).expect("vpk");
                assert_eq!(sha256_hex(&vpk), item.archive.inner_sha256);
                assert_eq!(vpk.len(), item.archive.inner_bytes);
                vpk
            }
        };
        content_store::store_pinned_resource(
            &root,
            item.archive.inner_bytes,
            &item.archive.inner_sha256,
            &inner,
        )
        .expect("stores");
        let suffixes: &[&str] = match item.scope {
            ScopeKind::Hero => &["", ".strip"],
            ScopeKind::None => &[""],
        };
        for suffix in suffixes {
            let id = format!("{}{suffix}", item.id);
            let started = std::time::Instant::now();
            let plan =
                crate::tree_pilot::plan_from_verified_store(&root, std::slice::from_ref(&id))
                    .expect("plan");
            let again =
                crate::tree_pilot::plan_from_verified_store(&root, std::slice::from_ref(&id))
                    .expect("second plan");
            let plan = serde_json::to_value(&plan).expect("json");
            let again = serde_json::to_value(&again).expect("json");
            assert_eq!(
                plan["vpkSha256"], again["vpkSha256"],
                "{id} is not deterministic"
            );
            assert_eq!(plan["planId"], again["planId"], "{id} plan id differs");
            println!(
                "{id} [{}]: {} resources, {} payload bytes, vpk {} bytes sha256 {} plan {}, overrides {} (left out {}), dropped {}, ignored {}, conflicts {}, deterministic, {:?}",
                item.category,
                plan["resourceCount"],
                plan["resourceBytes"],
                plan["vpkBytes"],
                plan["vpkSha256"],
                plan["planId"],
                plan["wardrobe"][0]["report"]["shared"]["files"],
                plan["wardrobe"][0]["report"]["install"]["strippedShared"],
                plan["wardrobe"][0]["report"]["rejected"]["files"],
                plan["wardrobe"][0]["report"]["ignored"]["files"],
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
                let again =
                    crate::tree_pilot::plan_from_verified_store(&root, &ids).expect("second plan");
                let plan = serde_json::to_value(&plan).expect("json");
                let again = serde_json::to_value(&again).expect("json");
                assert_eq!(
                    plan["vpkSha256"], again["vpkSha256"],
                    "together: not deterministic"
                );
                println!(
                    "together: {} resources, {} payload bytes, vpk {} bytes sha256 {}, overrides {}, wardrobe conflicts {}, deterministic",
                    plan["resourceCount"],
                    plan["resourceBytes"],
                    plan["vpkBytes"],
                    plan["vpkSha256"],
                    plan["overriddenResources"],
                    plan["wardrobeConflicts"].as_array().map_or(0, Vec::len)
                );
            }
            Err(code) => println!("together: refused with {code}"),
        }
        let (files, bytes) = wardrobe::install_footprint(&ids);
        println!("footprint of the selection: {files} files, {bytes} bytes");
        // A selection over the merged VPK's bound is refused before anything is
        // downloaded (the footprint is known from the audit alone).
        if bytes > crate::mod_bundle::MAX_PAYLOAD_BYTES as u64 {
            let refused = crate::tree_pilot::begin_download(root.clone(), ids.clone(), None);
            println!("begin_download: {:?}", refused.as_ref().err());
            assert_eq!(refused.err().as_deref(), Some("bundle_limit_exceeded"));
        }
    }
    std::fs::remove_dir_all(root).expect("cleanup");
}

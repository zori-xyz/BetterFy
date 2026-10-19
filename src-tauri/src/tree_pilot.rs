//! Download, verification and bundling for the pinned, data-only packages
//! described by `package_registry` manifests.

use crate::game_deployment::{DeploymentPayload, DeploymentReceipt, PayloadFile};
use crate::map_terrain::{self, LoadedMap, MapManifest, MapPlanItem};
use crate::mod_bundle::{
    self, BundleContribution, BundleDuplicate, BundleOverride, BundlePackage, BundlePlan,
};
use crate::package_registry::{self, PackageManifest, Resource};
use crate::skin_archive::SharedPolicy;
use crate::vpk;
use crate::wardrobe::{self, WardrobeConflict, WardrobeManifest, WardrobePlanItem};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

static DOWNLOAD: OnceLock<Mutex<Option<Arc<TreeDownload>>>> = OnceLock::new();

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TreeDownloadStatus {
    phase: &'static str,
    verified_resources: usize,
    total_resources: usize,
    error_code: Option<String>,
    plan: Option<TreePilotPlan>,
}

struct TreeDownload {
    status: Mutex<TreeDownloadStatus>,
    cancelled: AtomicBool,
    app_data_root: std::path::PathBuf,
    package_ids: Vec<String>,
}

/// Where the expansion of each package's blacklist against the player's game
/// is kept. Rebuilding the same bundle later (verification, Steam setup,
/// restore) reads this file, so a Dota update does not change what BetterFy
/// considers installed.
fn blacklist_resolution_path(app_data_root: &Path, package_id: &str) -> std::path::PathBuf {
    app_data_root
        .join("engine-v1")
        .join("blacklist")
        .join(format!("{package_id}.json"))
}

#[derive(serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BlacklistResolution {
    package_id: String,
    blacklist_sha256: String,
    expansion: crate::blacklist::Expansion,
}

fn read_game_resource_paths(game_path: &Path) -> Result<BTreeSet<String>, String> {
    let mut paths = BTreeSet::new();
    for archive in ["game/dota/pak01_dir.vpk", "game/core/pak01_dir.vpk"] {
        let bytes = std::fs::read(game_path.join(archive))
            .map_err(|_| "game_archive_unreadable".to_string())?;
        paths.extend(vpk::list_directory_paths(&bytes)?);
    }
    Ok(paths)
}

/// Expands every selected blacklist against the game at `game_path` and
/// stores the result. Packages without a blacklist need no game path.
fn resolve_blacklists(
    app_data_root: &Path,
    package_ids: &[String],
    game_path: Option<&Path>,
) -> Result<(), String> {
    let mut game_paths: Option<BTreeSet<String>> = None;
    for package_id in package_ids {
        let Some(manifest) = tuning(package_id)? else {
            continue;
        };
        let Some(blacklist) = &manifest.blacklist else {
            continue;
        };
        let game_path = game_path.ok_or_else(|| "game_path_required".to_string())?;
        if game_paths.is_none() {
            game_paths = Some(read_game_resource_paths(game_path)?);
        }
        let text = crate::content_store::read_pinned_resource(
            app_data_root,
            blacklist.bytes,
            &blacklist.sha256,
        )?;
        let text = String::from_utf8(text).map_err(|_| "blacklist_invalid".to_string())?;
        let blanks = blacklist.blanks.keys().cloned().collect::<BTreeSet<_>>();
        let expansion =
            crate::blacklist::expand(&text, game_paths.as_ref().expect("read above"), &blanks)?;
        let resolution = BlacklistResolution {
            package_id: package_id.clone(),
            blacklist_sha256: blacklist.sha256.clone(),
            expansion,
        };
        let target = blacklist_resolution_path(app_data_root, package_id);
        let parent = target
            .parent()
            .ok_or_else(|| "blacklist_store_failed".to_string())?;
        std::fs::create_dir_all(parent).map_err(|_| "blacklist_store_failed".to_string())?;
        let bytes =
            serde_json::to_vec(&resolution).map_err(|_| "blacklist_store_failed".to_string())?;
        let temporary = target.with_extension("json.tmp");
        std::fs::write(&temporary, bytes).map_err(|_| "blacklist_store_failed".to_string())?;
        std::fs::rename(&temporary, &target).map_err(|_| "blacklist_store_failed".to_string())?;
    }
    Ok(())
}

/// The file list of the player's own `game/dota/pak01_dir.vpk`, lower case.
/// Read-only and bounded; any problem is an error the caller turns into "not
/// checked".
fn read_dota_pak_paths(game_path: &Path) -> Result<BTreeSet<String>, String> {
    const MAX_DIRECTORY_FILE_BYTES: u64 = 512 * 1024 * 1024;
    let file = game_path.join("game/dota/pak01_dir.vpk");
    let metadata = std::fs::metadata(&file).map_err(|_| "game_archive_unreadable".to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_DIRECTORY_FILE_BYTES {
        return Err("game_archive_unreadable".to_string());
    }
    let bytes = std::fs::read(&file).map_err(|_| "game_archive_unreadable".to_string())?;
    Ok(vpk::list_directory_paths(&bytes)?.into_iter().collect())
}

/// Compares every selected wardrobe item with the player's own game and
/// stores, per item, which of the skin's own files the game ships now
/// (added by Valve since the audit). Those are left out of the build and
/// listed in the plan. When there is no game folder, or its file list cannot
/// be read, the items are recorded as "not checked" and the plan says so: the
/// game never makes the plan fail. Later rebuilds read the stored result, as
/// for blacklists, so a Dota update does not change what is installed.
fn resolve_wardrobe_game_checks(
    app_data_root: &Path,
    package_ids: &[String],
    game_path: Option<&Path>,
) -> Result<(), String> {
    let mut listing: Option<Option<BTreeSet<String>>> = None;
    for package_id in package_ids {
        let Contract::Wardrobe(item, _) = resolve(package_id)? else {
            continue;
        };
        let game_paths = listing
            .get_or_insert_with(|| game_path.and_then(|path| read_dota_pak_paths(path).ok()))
            .as_ref();
        wardrobe::record_game_check(app_data_root, item, game_paths)?;
    }
    Ok(())
}

fn style_resolution_path(app_data_root: &Path, package_id: &str) -> std::path::PathBuf {
    app_data_root
        .join("engine-v1")
        .join("panorama")
        .join(format!("{package_id}.json"))
}

#[derive(serde::Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct StoredOriginal {
    bytes: usize,
    sha256: String,
}

/// The game's own compiled styles a package's `styling.css` extends, captured
/// when the build is prepared. Keys are `game:path` or `core:path`.
#[derive(serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StyleResolution {
    package_id: String,
    styling_sha256: String,
    originals: BTreeMap<String, StoredOriginal>,
    missing: Vec<String>,
}

fn write_json_atomically(target: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = target
        .parent()
        .ok_or_else(|| "engine_store_failed".to_string())?;
    std::fs::create_dir_all(parent).map_err(|_| "engine_store_failed".to_string())?;
    let bytes = serde_json::to_vec(value).map_err(|_| "engine_store_failed".to_string())?;
    let temporary = target.with_extension("json.tmp");
    std::fs::write(&temporary, bytes).map_err(|_| "engine_store_failed".to_string())?;
    std::fs::rename(&temporary, target).map_err(|_| "engine_store_failed".to_string())
}

fn read_styling(
    app_data_root: &Path,
    manifest: &PackageManifest,
    package_id: &str,
) -> Result<Option<BTreeMap<String, String>>, String> {
    let Some(styles) = &manifest.panorama_styles else {
        return Ok(None);
    };
    let bytes =
        crate::content_store::read_pinned_resource(app_data_root, styles.bytes, &styles.sha256)?;
    let text = String::from_utf8(bytes).map_err(|_| "panorama_styling_invalid".to_string())?;
    // User Styles installs only the snippets its ID names.
    let text = if manifest.id == crate::user_styles::PACKAGE_ID {
        let (_, mask) = crate::user_styles::parse_variant(package_id)
            .ok_or_else(|| "user_styles_selection_required".to_string())?;
        crate::user_styles::select(&text, mask)?
    } else {
        text
    };
    crate::panorama::split_styling(&text).map(Some)
}

/// Captures the game styles every selected `styling.css` extends.
fn resolve_styles(
    app_data_root: &Path,
    package_ids: &[String],
    game_path: Option<&Path>,
) -> Result<(), String> {
    for package_id in package_ids {
        let Some(manifest) = tuning(package_id)? else {
            continue;
        };
        let Some(sections) = read_styling(app_data_root, manifest, package_id)? else {
            continue;
        };
        let game_path = game_path.ok_or_else(|| "game_path_required".to_string())?;
        let mut originals = BTreeMap::new();
        let mut missing = Vec::new();
        for archive in ["game", "core"] {
            let wanted = sections
                .keys()
                .filter_map(|key| key.strip_prefix(&format!("{archive}:")))
                .map(String::from)
                .collect::<BTreeSet<_>>();
            if wanted.is_empty() {
                continue;
            }
            let directory = if archive == "game" { "dota" } else { "core" };
            let found = vpk::read_game_resources(
                &game_path.join("game").join(directory).join("pak01_dir.vpk"),
                &wanted,
            )?;
            for path in wanted {
                let key = format!("{archive}:{path}");
                match found.get(&path) {
                    Some(bytes) => {
                        crate::panorama::style_text(bytes)?;
                        let sha256 = format!("{:x}", Sha256::digest(bytes));
                        crate::content_store::store_pinned_resource(
                            app_data_root,
                            bytes.len(),
                            &sha256,
                            bytes,
                        )?;
                        originals.insert(
                            key,
                            StoredOriginal {
                                bytes: bytes.len(),
                                sha256,
                            },
                        );
                    }
                    None => missing.push(key),
                }
            }
        }
        let styles = manifest
            .panorama_styles
            .as_ref()
            .ok_or_else(|| "panorama_styling_invalid".to_string())?;
        write_json_atomically(
            &style_resolution_path(app_data_root, package_id),
            &StyleResolution {
                package_id: package_id.clone(),
                styling_sha256: styles.sha256.clone(),
                originals,
                missing,
            },
        )?;
    }
    Ok(())
}

/// Extends the captured game styles with every selected package's CSS. When
/// several packages extend the same style their CSS is joined lowest priority
/// first, so the highest-priority package's rules come last and win; the
/// result belongs to that package in the bundle.
fn apply_panorama_styles(
    app_data_root: &Path,
    package_ids: &[String],
    packages: &mut [BundlePackage],
) -> Result<(), String> {
    let mut targets: BTreeMap<String, (StoredOriginal, Vec<(usize, String)>)> = BTreeMap::new();
    for (index, package_id) in package_ids.iter().enumerate() {
        let Some(manifest) = tuning(package_id)? else {
            continue;
        };
        let Some(sections) = read_styling(app_data_root, manifest, package_id)? else {
            continue;
        };
        let styles = manifest
            .panorama_styles
            .as_ref()
            .ok_or_else(|| "panorama_styling_invalid".to_string())?;
        let bytes = std::fs::read(style_resolution_path(app_data_root, package_id))
            .map_err(|_| "panorama_resolution_missing".to_string())?;
        let resolution: StyleResolution = serde_json::from_slice(&bytes)
            .map_err(|_| "panorama_resolution_invalid".to_string())?;
        if resolution.package_id != *package_id || resolution.styling_sha256 != styles.sha256 {
            return Err("panorama_resolution_stale".to_string());
        }
        for (key, css) in sections {
            let Some(original) = resolution.originals.get(&key) else {
                continue;
            };
            let path = key
                .split_once(':')
                .map(|(_, path)| path.to_string())
                .ok_or_else(|| "panorama_resolution_invalid".to_string())?;
            let entry = targets
                .entry(path)
                .or_insert_with(|| (original.clone(), Vec::new()));
            if entry.0.sha256 != original.sha256 {
                return Err("panorama_resolution_invalid".to_string());
            }
            entry.1.push((index, css));
        }
    }
    for (path, (original, mut additions)) in targets {
        let bytes = crate::content_store::read_pinned_resource(
            app_data_root,
            original.bytes,
            &original.sha256,
        )?;
        if bytes.len() != original.bytes
            || format!("{:x}", Sha256::digest(&bytes)) != original.sha256
        {
            return Err("tree_resource_unverified".to_string());
        }
        additions.sort_by_key(|(index, _)| std::cmp::Reverse(*index));
        let owner = additions.last().map(|(index, _)| *index).unwrap_or(0);
        let css = additions
            .iter()
            .map(|(_, css)| css.trim())
            .collect::<Vec<_>>()
            .join("\n\n");
        let patched = crate::panorama::append_style(&bytes, &css)?;
        packages
            .get_mut(owner)
            .ok_or_else(|| "panorama_resolution_invalid".to_string())?
            .resources
            .insert(path, patched);
    }
    Ok(())
}

fn layout_resolution_path(app_data_root: &Path, package_id: &str) -> std::path::PathBuf {
    app_data_root
        .join("engine-v1")
        .join("layout")
        .join(format!("{package_id}.json"))
}

/// The game's own compiled layouts a package's `xml.json` (and settings menu)
/// edits, captured when the build is prepared.
#[derive(serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LayoutResolution {
    package_id: String,
    edits_sha256: String,
    menu_sha256: Option<String>,
    originals: BTreeMap<String, StoredOriginal>,
}

struct PackageLayouts {
    edits: BTreeMap<String, Vec<crate::panorama_layout::Edit>>,
    menu: Option<String>,
}

fn read_layouts(
    app_data_root: &Path,
    manifest: &PackageManifest,
) -> Result<Option<PackageLayouts>, String> {
    let Some(layouts) = &manifest.panorama_layouts else {
        return Ok(None);
    };
    let text =
        crate::content_store::read_pinned_resource(app_data_root, layouts.bytes, &layouts.sha256)?;
    let text = String::from_utf8(text).map_err(|_| "panorama_layout_invalid".to_string())?;
    let mut edits = crate::panorama_layout::parse_edits(&text)?;
    let menu = match &layouts.menu {
        Some(menu) => {
            let text = crate::content_store::read_pinned_resource(
                app_data_root,
                menu.bytes,
                &menu.sha256,
            )?;
            edits
                .entry(crate::panorama_layout::MENU_TARGET.to_string())
                .or_default();
            Some(String::from_utf8(text).map_err(|_| "panorama_layout_invalid".to_string())?)
        }
        None => None,
    };
    Ok(Some(PackageLayouts { edits, menu }))
}

fn edit_layout(
    bytes: &[u8],
    edits: &[(&[crate::panorama_layout::Edit], Option<&String>)],
) -> Result<Vec<u8>, String> {
    let mut layout = crate::panorama_layout::read(bytes)?;
    let mut menus = Vec::new();
    for (actions, menu) in edits {
        for action in *actions {
            crate::panorama_layout::apply(&mut layout.root, action)?;
        }
        menus.extend(menu.cloned());
    }
    crate::panorama_layout::apply_menus(&mut layout.root, &menus)?;
    crate::panorama_layout::write(&layout)
}

/// Captures the game layouts every selected `xml.json` edits and checks that
/// each package's edits apply to them, so a Dota update that moved a target
/// fails here with a clear code instead of producing a half-edited layout.
fn resolve_layouts(
    app_data_root: &Path,
    package_ids: &[String],
    game_path: Option<&Path>,
) -> Result<(), String> {
    for package_id in package_ids {
        let Some(manifest) = tuning(package_id)? else {
            continue;
        };
        let Some(layouts) = read_layouts(app_data_root, manifest)? else {
            continue;
        };
        let spec = manifest
            .panorama_layouts
            .as_ref()
            .ok_or_else(|| "panorama_layout_invalid".to_string())?;
        let game_path = game_path.ok_or_else(|| "game_path_required".to_string())?;
        let wanted = layouts.edits.keys().cloned().collect::<BTreeSet<_>>();
        let found = vpk::read_game_resources(
            &game_path.join("game").join("dota").join("pak01_dir.vpk"),
            &wanted,
        )?;
        let mut originals = BTreeMap::new();
        for (path, actions) in &layouts.edits {
            let bytes = found
                .get(path)
                .ok_or_else(|| "panorama_layout_target_missing".to_string())?;
            let menu = (path == crate::panorama_layout::MENU_TARGET)
                .then_some(layouts.menu.as_ref())
                .flatten();
            edit_layout(bytes, &[(actions.as_slice(), menu)])?;
            let sha256 = format!("{:x}", Sha256::digest(bytes));
            crate::content_store::store_pinned_resource(
                app_data_root,
                bytes.len(),
                &sha256,
                bytes,
            )?;
            originals.insert(
                path.clone(),
                StoredOriginal {
                    bytes: bytes.len(),
                    sha256,
                },
            );
        }
        write_json_atomically(
            &layout_resolution_path(app_data_root, package_id),
            &LayoutResolution {
                package_id: package_id.clone(),
                edits_sha256: spec.sha256.clone(),
                menu_sha256: spec.menu.as_ref().map(|menu| menu.sha256.clone()),
                originals,
            },
        )?;
    }
    Ok(())
}

/// Applies every selected package's layout edits to the captured game
/// layouts, lowest priority first, so the highest-priority package edits
/// last; the result belongs to that package in the bundle.
fn apply_panorama_layouts(
    app_data_root: &Path,
    package_ids: &[String],
    packages: &mut [BundlePackage],
) -> Result<(), String> {
    let mut selected = Vec::new();
    for (index, package_id) in package_ids.iter().enumerate() {
        let Some(manifest) = tuning(package_id)? else {
            continue;
        };
        let Some(layouts) = read_layouts(app_data_root, manifest)? else {
            continue;
        };
        let spec = manifest
            .panorama_layouts
            .as_ref()
            .ok_or_else(|| "panorama_layout_invalid".to_string())?;
        let bytes = std::fs::read(layout_resolution_path(app_data_root, package_id))
            .map_err(|_| "panorama_resolution_missing".to_string())?;
        let resolution: LayoutResolution = serde_json::from_slice(&bytes)
            .map_err(|_| "panorama_resolution_invalid".to_string())?;
        if resolution.package_id != *package_id
            || resolution.edits_sha256 != spec.sha256
            || resolution.menu_sha256 != spec.menu.as_ref().map(|menu| menu.sha256.clone())
        {
            return Err("panorama_resolution_stale".to_string());
        }
        if resolution.originals.keys().ne(layouts.edits.keys()) {
            return Err("panorama_resolution_invalid".to_string());
        }
        selected.push((index, layouts, resolution));
    }
    let mut targets: BTreeMap<String, (StoredOriginal, Vec<usize>)> = BTreeMap::new();
    for (position, (_, _, resolution)) in selected.iter().enumerate() {
        for (path, original) in &resolution.originals {
            let entry = targets
                .entry(path.clone())
                .or_insert_with(|| (original.clone(), Vec::new()));
            if entry.0.sha256 != original.sha256 {
                return Err("panorama_resolution_invalid".to_string());
            }
            entry.1.push(position);
        }
    }
    for (path, (original, mut users)) in targets {
        let bytes = crate::content_store::read_pinned_resource(
            app_data_root,
            original.bytes,
            &original.sha256,
        )?;
        if bytes.len() != original.bytes
            || format!("{:x}", Sha256::digest(&bytes)) != original.sha256
        {
            return Err("tree_resource_unverified".to_string());
        }
        // `selected` is in priority order; apply from the lowest priority.
        users.reverse();
        let edits = users
            .iter()
            .map(|position| {
                let (_, layouts, _) = &selected[*position];
                let menu = (path == crate::panorama_layout::MENU_TARGET)
                    .then_some(layouts.menu.as_ref())
                    .flatten();
                (layouts.edits[&path].as_slice(), menu)
            })
            .collect::<Vec<_>>();
        let patched = edit_layout(&bytes, &edits)?;
        let owner = selected[*users
            .last()
            .ok_or_else(|| "panorama_resolution_invalid".to_string())?]
        .0;
        packages
            .get_mut(owner)
            .ok_or_else(|| "panorama_resolution_invalid".to_string())?
            .resources
            .insert(path, patched);
    }
    Ok(())
}

fn stored_expansion(
    app_data_root: &Path,
    manifest: &PackageManifest,
) -> Result<Option<crate::blacklist::Expansion>, String> {
    let Some(blacklist) = &manifest.blacklist else {
        return Ok(None);
    };
    let bytes = std::fs::read(blacklist_resolution_path(app_data_root, &manifest.id))
        .map_err(|_| "blacklist_resolution_missing".to_string())?;
    let resolution: BlacklistResolution =
        serde_json::from_slice(&bytes).map_err(|_| "blacklist_resolution_invalid".to_string())?;
    if resolution.package_id != manifest.id || resolution.blacklist_sha256 != blacklist.sha256 {
        return Err("blacklist_resolution_stale".to_string());
    }
    if resolution.expansion.paths.len() > crate::blacklist::MAX_EXPANDED
        || resolution.expansion.paths.iter().any(|(path, extension)| {
            !blacklist.blanks.contains_key(extension) || !path.ends_with(&format!(".{extension}"))
        })
    {
        return Err("blacklist_resolution_invalid".to_string());
    }
    Ok(Some(resolution.expansion))
}

fn download_slot() -> &'static Mutex<Option<Arc<TreeDownload>>> {
    DOWNLOAD.get_or_init(|| Mutex::new(None))
}

/// The first pilot package. Older journals written before bundles recorded
/// their package list are read as this single package.
pub(crate) const PACKAGE_ID: &str = "minify.tree-mod";
pub(crate) const TARGET_FILE: &str = "pak66_dir.vpk";
/// Upper bound on packages merged into one VPK.
const MAX_BUNDLE_PACKAGES: usize = 32;

fn normalize_package_ids(ids: &[String]) -> Result<Vec<String>, String> {
    if ids.is_empty() || ids.len() > MAX_BUNDLE_PACKAGES {
        return Err("pilot_package_unsupported".to_string());
    }
    let mut seen = BTreeSet::new();
    let ids = ids
        .iter()
        .map(|id| {
            let contract = resolve(id)?;
            // A skin and its `.strip` mode are the same skin: once per build.
            if !seen.insert(contract.base_id().to_string()) {
                return Err("pilot_package_invalid".to_string());
            }
            Ok(contract.engine_id())
        })
        .collect::<Result<Vec<_>, String>>()?;
    // A map terrain replaces the game's map package as a whole, so two of them
    // cannot be installed together: the plan says so before anything is
    // downloaded.
    let mut maps = 0usize;
    for id in &ids {
        if matches!(resolve(id)?, Contract::Map(_)) {
            maps += 1;
        }
    }
    if maps > 1 {
        return Err("map_terrain_conflict".to_string());
    }
    // User Styles installs only the snippets the player picked; without a
    // pick it has nothing to install.
    for id in &ids {
        if id == crate::user_styles::PACKAGE_ID {
            return Err("user_styles_selection_required".to_string());
        }
    }
    // A cursor or font pack replaces the whole set of the game's cursors or
    // fonts: one of each per build.
    let mut pack_categories = BTreeSet::new();
    for id in &ids {
        if let Contract::Pack(item) = resolve(id)? {
            if !pack_categories.insert(item.category) {
                return Err("game_pack_conflict".to_string());
            }
        }
    }
    // Minify's own conflicts and dependencies, checked before anything is
    // downloaded or built. A wardrobe item declares neither.
    for id in &ids {
        let Some(manifest) = tuning(id)? else {
            continue;
        };
        if manifest.conflicts.iter().any(|other| seen.contains(other)) {
            return Err("package_conflict".to_string());
        }
        if manifest.requires.iter().any(|other| !seen.contains(other)) {
            return Err("package_dependency_missing".to_string());
        }
    }
    Ok(ids)
}

/// What a package ID resolves to: an audited tuning package, or a wardrobe
/// item from the embedded allowlist.
enum Contract {
    Tuning(&'static PackageManifest),
    /// An item and how it is installed: everything the analyzer accepts (the
    /// default) or, for the `.strip` ID, without the shared game files.
    Wardrobe(&'static WardrobeManifest, SharedPolicy),
    /// A whole-map package installed as its own file, `maps/dota.vpk`.
    Map(&'static MapManifest),
    /// A cursor or font pack: the game's own loose files (`game_files`).
    Pack(&'static crate::game_packs::PackManifest),
    /// Minify's User Styles with the snippets the player picked (a bit mask).
    UserStyles(&'static PackageManifest, u32),
    /// Minify's Custom Hero Grids: grids compiled into the app, added to the
    /// Steam accounts' own (`hero_grids`).
    HeroGrids,
    /// Minify's Custom Fonts: the font file the player picked (its SHA-256
    /// and size), stored by `custom_fonts::store`.
    CustomFont(String, usize),
}

impl Contract {
    fn engine_id(&self) -> String {
        match self {
            Self::Tuning(manifest) => manifest.id.clone(),
            Self::Wardrobe(item, policy) => wardrobe::variant_id(item, *policy),
            Self::Map(item) => item.id.clone(),
            Self::Pack(item) => item.id.clone(),
            Self::UserStyles(_, mask) => crate::user_styles::variant_id(*mask),
            Self::HeroGrids => crate::hero_grids::PACKAGE_ID.to_string(),
            Self::CustomFont(sha, bytes) => crate::custom_fonts::variant_id(sha, *bytes),
        }
    }

    /// The ID of what is installed, without the install mode: two modes of the
    /// same skin are one skin, and a build holds it once.
    fn base_id(&self) -> &str {
        match self {
            Self::Tuning(manifest) => &manifest.id,
            Self::Wardrobe(item, _) => &item.id,
            Self::Map(item) => &item.id,
            Self::Pack(item) => &item.id,
            Self::UserStyles(manifest, _) => &manifest.id,
            Self::HeroGrids => crate::hero_grids::PACKAGE_ID,
            Self::CustomFont(..) => crate::custom_fonts::PACKAGE_ID,
        }
    }
}

fn resolve(id: &str) -> Result<Contract, String> {
    if let Some((item, policy)) = wardrobe::find_variant(id)? {
        return Ok(Contract::Wardrobe(item, policy));
    }
    if let Some(item) = map_terrain::find(id)? {
        return Ok(Contract::Map(item));
    }
    if let Some(item) = crate::game_packs::find(id)? {
        return Ok(Contract::Pack(item));
    }
    if id == crate::hero_grids::PACKAGE_ID || id == crate::hero_grids::CATALOG_ID {
        return Ok(Contract::HeroGrids);
    }
    if id == crate::custom_fonts::PACKAGE_ID || id == crate::custom_fonts::CATALOG_ID {
        return Err("custom_font_required".to_string());
    }
    if let Some((sha, bytes)) = crate::custom_fonts::parse_variant(id) {
        return Ok(Contract::CustomFont(sha, bytes));
    }
    if let Some((base, mask)) = crate::user_styles::parse_variant(id) {
        return package_registry::find(base).map(|manifest| Contract::UserStyles(manifest, mask));
    }
    package_registry::find(id).map(Contract::Tuning)
}

/// The tuning manifest behind an ID, or `None` for a wardrobe item, which has
/// no blacklist, style or layout edits.
fn tuning(id: &str) -> Result<Option<&'static PackageManifest>, String> {
    match resolve(id)? {
        Contract::Tuning(manifest) | Contract::UserStyles(manifest, _) => Ok(Some(manifest)),
        Contract::Wardrobe(..)
        | Contract::Map(_)
        | Contract::Pack(_)
        | Contract::HeroGrids
        | Contract::CustomFont(..) => Ok(None),
    }
}

/// A tuning package only; wardrobe items have no pinned per-file contract.
fn contract(id: &str) -> Result<&'static PackageManifest, String> {
    tuning(id)?.ok_or_else(|| "pilot_package_unsupported".to_string())
}

pub(crate) struct VerifiedTreeVpk {
    /// The merged `pakNN_dir.vpk`; empty for a map-only build.
    bytes: Vec<u8>,
    sha256: String,
    /// The plan of the merged packages. A map-only build has an empty one.
    bundle_plan: BundlePlan,
    /// The map terrain of the build, installed as `maps/dota.vpk` beside (never
    /// inside) the merged VPK.
    map: Option<LoadedMap>,
    /// Every package of the build in selection order, map included.
    package_ids: Vec<String>,
    /// The identity of the reviewed build: the bundle plan's when there is no
    /// map (unchanged for every build that had none), otherwise a hash that
    /// also binds the map package and the full ordered package list.
    plan_id: String,
    /// What the analyzer found for each selected wardrobe item, in selection
    /// order. Empty for a build of tuning packages only.
    wardrobe: Vec<WardrobePlanItem>,
    /// Paths a wardrobe item writes that another package writes differently.
    wardrobe_conflicts: Vec<WardrobeConflict>,
    /// The game's own loose files the build replaces (`None`: removes),
    /// below `game/dota`. Their list is inside the VPK, so the plan ID binds
    /// them.
    game_files: BTreeMap<String, Option<Vec<u8>>>,
    /// The cursor and font packs of the build, in selection order.
    packs: Vec<crate::game_packs::PackPlanItem>,
    /// The build adds the pinned hero grids to the Steam accounts' own.
    hero_grids: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TreePilotPlan {
    plan_id: String,
    package_id: String,
    package_ids: Vec<String>,
    source_commit: String,
    target_file: &'static str,
    resource_count: usize,
    resource_bytes: usize,
    vpk_bytes: usize,
    vpk_sha256: String,
    /// Whether the build writes a `pakNN_dir.vpk` at all (a map-only build
    /// does not).
    has_pak: bool,
    /// The map terrain of the build, written as `maps/dota.vpk`.
    map: Option<MapPlanItem>,
    bundle_plan_id: String,
    package_count: usize,
    duplicate_resources: usize,
    overridden_resources: usize,
    duplicates: Vec<BundleDuplicate>,
    overrides: Vec<BundleOverride>,
    contributions: Vec<BundleContribution>,
    /// One entry per selected wardrobe item: the analyzer report, with its
    /// size, class counts and every stripped or rejected path.
    wardrobe: Vec<WardrobePlanItem>,
    wardrobe_conflicts: Vec<WardrobeConflict>,
    /// Cursor and font packs: the game's own files they replace or remove.
    packs: Vec<crate::game_packs::PackPlanItem>,
    compatibility: &'static str,
    distribution: &'static str,
    deploy_enabled: bool,
}

impl VerifiedTreeVpk {
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn sha256(&self) -> &str {
        &self.sha256
    }

    pub(crate) fn plan_id(&self) -> &str {
        &self.plan_id
    }

    pub(crate) fn bundle_plan(&self) -> &BundlePlan {
        &self.bundle_plan
    }

    /// Every package of the build in selection order, map included; this is
    /// the list the deployment records.
    pub(crate) fn package_ids(&self) -> &[String] {
        &self.package_ids
    }

    /// The game's own loose files the build replaces (`None`: removes).
    pub(crate) fn game_files(&self) -> &BTreeMap<String, Option<Vec<u8>>> {
        &self.game_files
    }

    /// Whether the build adds the pinned hero grids.
    pub(crate) fn hero_grids(&self) -> bool {
        self.hero_grids
    }

    pub(crate) fn has_pak(&self) -> bool {
        !self.bytes.is_empty()
    }

    pub(crate) fn map(&self) -> Option<&LoadedMap> {
        self.map.as_ref()
    }

    /// The files an install writes.
    pub(crate) fn payload(&self) -> DeploymentPayload<'_> {
        DeploymentPayload {
            pak: self.has_pak().then_some(PayloadFile {
                bytes: &self.bytes,
                sha256: &self.sha256,
            }),
            map: self.map.as_ref().map(|map| PayloadFile {
                bytes: &map.bytes,
                sha256: &map.sha256,
            }),
        }
    }

    /// The hash that identifies what the build installs: the merged VPK's, or
    /// the map's for a map-only build.
    pub(crate) fn primary_sha256(&self) -> &str {
        if self.has_pak() {
            &self.sha256
        } else {
            self.map.as_ref().map_or("", |map| map.sha256.as_str())
        }
    }

    /// Whether an installed deployment is exactly this build: same files, same
    /// reviewed plan. A record from before plans were bound is compared by
    /// its hashes alone.
    pub(crate) fn matches_receipt(&self, receipt: &DeploymentReceipt) -> bool {
        receipt.has_pak == self.has_pak()
            && receipt.installed_sha256 == self.primary_sha256()
            && receipt
                .map
                .as_ref()
                .and_then(|map| map.installed_sha256.as_deref())
                == self.map.as_ref().map(|map| map.sha256.as_str())
            && receipt
                .bundle_plan_id
                .as_deref()
                .is_none_or(|plan| plan == self.plan_id())
    }

    /// A build with a wardrobe path conflict installs only after the person
    /// has been shown the conflicts and said so.
    pub(crate) fn require_acknowledged_conflicts(&self, acknowledged: bool) -> Result<(), String> {
        if self.wardrobe_conflicts.is_empty() || acknowledged {
            Ok(())
        } else {
            Err("wardrobe_conflict_unacknowledged".to_string())
        }
    }
}

#[cfg(test)]
pub(crate) fn verified_test_vpk() -> VerifiedTreeVpk {
    let bundle = mod_bundle::build(vec![BundlePackage {
        package_id: PACKAGE_ID.to_string(),
        resources: BTreeMap::from([(
            "models/props_tree/test_tree.vmdl_c".to_string(),
            b"verified-test-tree".to_vec(),
        )]),
    }])
    .expect("test VPK bundle");
    VerifiedTreeVpk {
        bytes: bundle.bytes().to_vec(),
        sha256: bundle.sha256().to_string(),
        bundle_plan: bundle.plan().clone(),
        map: None,
        package_ids: bundle.plan().package_ids.clone(),
        plan_id: bundle.plan().plan_id.clone(),
        wardrobe: Vec::new(),
        wardrobe_conflicts: Vec::new(),
        game_files: BTreeMap::new(),
        packs: Vec::new(),
        hero_grids: false,
    }
}

/// The only URL a pilot resource may be downloaded from: the manifest's
/// pinned commit, for a resource that is exactly in that manifest.
pub(crate) fn pinned_url(package_id: &str, resource: &Resource) -> Result<String, String> {
    let manifest = contract(package_id)?;
    if !manifest.downloads().iter().any(|expected| {
        expected.path == resource.path
            && expected.bytes == resource.bytes
            && expected.sha256 == resource.sha256
            && expected.from == resource.from
    }) {
        return Err("download_contract_invalid".to_string());
    }
    Ok(manifest.resource_url(resource))
}

pub(crate) fn source_commit(package_id: &str) -> Result<String, String> {
    Ok(contract(package_id)?.source.commit.clone())
}

#[cfg(test)]
pub(crate) fn acquire_verified_resources(
    app_data_root: &Path,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    acquire_verified_resources_with_progress(
        app_data_root,
        &[PACKAGE_ID.to_string()],
        cancelled,
        |_| {},
    )
}

fn acquire_verified_resources_with_progress(
    app_data_root: &Path,
    package_ids: &[String],
    cancelled: &AtomicBool,
    mut progress: impl FnMut(usize),
) -> Result<(), String> {
    let contracts = package_ids
        .iter()
        .map(|id| resolve(id))
        .collect::<Result<Vec<_>, _>>()?;
    let mut index = 0usize;
    for (package_id, contract) in package_ids.iter().zip(contracts) {
        let manifest = match contract {
            Contract::Tuning(manifest) | Contract::UserStyles(manifest, _) => manifest,
            // One pinned archive: fetched, verified and stored as a unit.
            Contract::Wardrobe(item, _) => {
                if cancelled.load(Ordering::Relaxed) {
                    return Err("download_cancelled".to_string());
                }
                wardrobe::acquire(app_data_root, item, cancelled)?;
                index += 1;
                progress(index);
                continue;
            }
            // One pinned archive as well: its map package is fetched,
            // verified and stored as a unit.
            Contract::Map(item) => {
                if cancelled.load(Ordering::Relaxed) {
                    return Err("download_cancelled".to_string());
                }
                map_terrain::acquire(app_data_root, item, cancelled)?;
                index += 1;
                progress(index);
                continue;
            }
            // The player's font is already stored; nothing to download.
            Contract::CustomFont(sha, bytes) => {
                crate::content_store::read_pinned_resource(app_data_root, bytes, &sha)
                    .map_err(|_| "custom_font_missing".to_string())?;
                index += 1;
                progress(index);
                continue;
            }
            // The grids are compiled into the app: nothing to download.
            Contract::HeroGrids => {
                index += 1;
                progress(index);
                continue;
            }
            // One pinned ZIP; only its checked files are stored.
            Contract::Pack(item) => {
                if cancelled.load(Ordering::Relaxed) {
                    return Err("download_cancelled".to_string());
                }
                crate::game_packs::acquire(app_data_root, item, cancelled)?;
                index += 1;
                progress(index);
                continue;
            }
        };
        for resource in &manifest.downloads() {
            if cancelled.load(Ordering::Relaxed) {
                return Err("download_cancelled".to_string());
            }
            // Zero-length resources are fully described by the manifest.
            if resource.is_empty() {
                index += 1;
                progress(index);
                continue;
            }
            if crate::content_store::read_pinned_resource(
                app_data_root,
                resource.bytes,
                &resource.sha256,
            )
            .is_ok()
            {
                index += 1;
                progress(index);
                continue;
            }
            let bytes =
                crate::remote_intake::fetch_pinned_pilot_resource(package_id, resource, cancelled)?;
            if cancelled.load(Ordering::Relaxed) {
                return Err("download_cancelled".to_string());
            }
            crate::content_store::store_pinned_resource(
                app_data_root,
                resource.bytes,
                &resource.sha256,
                &bytes,
            )?;
            index += 1;
            progress(index);
        }
    }
    Ok(())
}

pub(crate) fn begin_download(
    app_data_root: std::path::PathBuf,
    requested_package_ids: Vec<String>,
    game_path: Option<std::path::PathBuf>,
) -> Result<TreeDownloadStatus, String> {
    let package_ids = normalize_package_ids(&requested_package_ids)?;
    // The merged VPK has a hard size and entry limit. The audited install
    // sizes of the selected skins are known up front, so a selection that
    // cannot fit is refused before anything is downloaded.
    let (skin_files, skin_bytes) = wardrobe::install_footprint(&package_ids);
    if skin_bytes > mod_bundle::MAX_PAYLOAD_BYTES as u64 || skin_files > mod_bundle::MAX_RESOURCES {
        return Err("bundle_limit_exceeded".to_string());
    }
    // A wardrobe item is one archive.
    let total_resources = package_ids.iter().try_fold(0usize, |total, id| {
        Ok::<_, String>(total + tuning(id)?.map_or(1, |manifest| manifest.downloads().len()))
    })?;
    if game_path.is_none()
        && package_ids.iter().any(|id| {
            contract(id).is_ok_and(|manifest| {
                manifest.blacklist.is_some()
                    || manifest.panorama_styles.is_some()
                    || manifest.panorama_layouts.is_some()
            })
        })
    {
        return Err("game_path_required".to_string());
    }
    let mut slot = download_slot()
        .lock()
        .map_err(|_| "download_failed".to_string())?;
    if let Some(current) = slot.as_ref() {
        let status = current
            .status
            .lock()
            .map_err(|_| "download_failed".to_string())?
            .clone();
        if matches!(status.phase, "downloading" | "verifying")
            || (current.package_ids == package_ids
                && status.phase == "ready"
                && plan_from_verified_store(&app_data_root, &package_ids).is_ok())
        {
            return Ok(status);
        }
    }
    let operation = Arc::new(TreeDownload {
        status: Mutex::new(TreeDownloadStatus {
            phase: "downloading",
            verified_resources: 0,
            total_resources,
            error_code: None,
            plan: None,
        }),
        cancelled: AtomicBool::new(false),
        app_data_root: app_data_root.clone(),
        package_ids: package_ids.clone(),
    });
    *slot = Some(operation.clone());
    std::thread::spawn(move || {
        let result = acquire_verified_resources_with_progress(
            &app_data_root,
            &package_ids,
            &operation.cancelled,
            |count| {
                if let Ok(mut status) = operation.status.lock() {
                    status.verified_resources = count;
                }
            },
        )
        .and_then(|_| {
            if operation.cancelled.load(Ordering::Relaxed) {
                return Err("download_cancelled".to_string());
            }
            if let Ok(mut status) = operation.status.lock() {
                status.phase = "verifying";
            }
            resolve_blacklists(&app_data_root, &package_ids, game_path.as_deref())?;
            resolve_wardrobe_game_checks(&app_data_root, &package_ids, game_path.as_deref())?;
            resolve_styles(&app_data_root, &package_ids, game_path.as_deref())?;
            resolve_layouts(&app_data_root, &package_ids, game_path.as_deref())?;
            plan_from_verified_store(&app_data_root, &package_ids)
        });
        if let Ok(mut status) = operation.status.lock() {
            match result {
                Ok(plan) => {
                    status.phase = "ready";
                    status.plan = Some(plan);
                }
                Err(code) => {
                    status.phase = if code == "download_cancelled" {
                        "cancelled"
                    } else {
                        "failed"
                    };
                    status.error_code = Some(code);
                }
            }
        }
    });
    Ok(TreeDownloadStatus {
        phase: "downloading",
        verified_resources: 0,
        total_resources,
        error_code: None,
        plan: None,
    })
}

pub(crate) fn download_status() -> Result<Option<TreeDownloadStatus>, String> {
    let slot = download_slot()
        .lock()
        .map_err(|_| "download_failed".to_string())?;
    let Some(operation) = slot.as_ref() else {
        return Ok(None);
    };
    let mut status = operation
        .status
        .lock()
        .map_err(|_| "download_failed".to_string())?;
    if status.phase == "ready"
        && plan_from_verified_store(&operation.app_data_root, &operation.package_ids).is_err()
    {
        status.phase = "failed";
        status.plan = None;
        status.error_code = Some("content_store_invalid".to_string());
    }
    Ok(Some(status.clone()))
}

pub(crate) fn cancel_download() -> Result<TreeDownloadStatus, String> {
    let slot = download_slot()
        .lock()
        .map_err(|_| "download_failed".to_string())?;
    let operation = slot
        .as_ref()
        .ok_or_else(|| "download_not_found".to_string())?;
    operation.cancelled.store(true, Ordering::Relaxed);
    operation
        .status
        .lock()
        .map(|status| status.clone())
        .map_err(|_| "download_failed".to_string())
}

pub(crate) fn build_from_verified_store(
    app_data_root: &Path,
    requested_package_ids: &[String],
) -> Result<VerifiedTreeVpk, String> {
    let package_ids = normalize_package_ids(requested_package_ids)?;
    // The map is not merged into the VPK: it is read on its own, and the
    // merged packages are everything else, in selection order.
    let mut map = None;
    let mut bundle_ids = Vec::new();
    for package_id in &package_ids {
        match resolve(package_id)? {
            Contract::Map(item) => map = Some(map_terrain::load(app_data_root, item)?),
            _ => bundle_ids.push(package_id.clone()),
        }
    }
    let mut wardrobe_items = Vec::new();
    let mut game_files = BTreeMap::new();
    let mut packs = Vec::new();
    let mut hero_grids = false;
    let packages = bundle_ids
        .iter()
        .map(|package_id| {
            let manifest = match resolve(package_id)? {
                Contract::Map(_) => return Err("pilot_package_invalid".to_string()),
                Contract::Tuning(manifest) | Contract::UserStyles(manifest, _) => manifest,
                // The pack's files go into the game folder on their own; the
                // VPK carries the list of them, which binds the reviewed build
                // to those exact files.
                Contract::Pack(item) => {
                    let loaded = crate::game_packs::load(app_data_root, item)?;
                    for (path, bytes) in loaded.files {
                        if game_files.insert(path, bytes).is_some() {
                            return Err("game_pack_conflict".to_string());
                        }
                    }
                    packs.push(loaded.item);
                    let (marker, text) = crate::game_packs::marker(item);
                    return Ok(BundlePackage {
                        package_id: package_id.clone(),
                        resources: BTreeMap::from([(marker, text)]),
                    });
                }
                Contract::CustomFont(sha, bytes) => {
                    for (path, font) in crate::custom_fonts::load(app_data_root, &sha, bytes)? {
                        if game_files.insert(path, font).is_some() {
                            return Err("game_pack_conflict".to_string());
                        }
                    }
                    let (marker, text) = crate::custom_fonts::marker(&sha, bytes);
                    return Ok(BundlePackage {
                        package_id: package_id.clone(),
                        resources: BTreeMap::from([(marker, text)]),
                    });
                }
                Contract::HeroGrids => {
                    hero_grids = true;
                    let (marker, text) = crate::hero_grids::marker();
                    return Ok(BundlePackage {
                        package_id: package_id.clone(),
                        resources: BTreeMap::from([(marker, text)]),
                    });
                }
                // The analyzer re-checks the stored archive against its audit
                // and returns only the files that audit allows.
                Contract::Wardrobe(item, policy) => {
                    let loaded = wardrobe::load(app_data_root, item, policy)?;
                    wardrobe_items.push(loaded.item);
                    return Ok(BundlePackage {
                        package_id: package_id.clone(),
                        resources: loaded.resources,
                    });
                }
            };
            let mut resources = manifest
                .resources
                .iter()
                .map(|resource| {
                    if resource.is_empty() {
                        return Ok((resource.path.clone(), Vec::new()));
                    }
                    crate::content_store::read_pinned_resource(
                        app_data_root,
                        resource.bytes,
                        &resource.sha256,
                    )
                    .map(|bytes| (resource.path.clone(), bytes))
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?;
            verify_resources(&manifest.resources, &resources)?;
            // Blacklisted game resources get the placeholder of their type.
            // A file the package ships itself always wins over a blank.
            if let (Some(blacklist), Some(expansion)) = (
                &manifest.blacklist,
                stored_expansion(app_data_root, manifest)?,
            ) {
                let mut blanks = BTreeMap::new();
                for (extension, blank) in &blacklist.blanks {
                    let bytes = crate::content_store::read_pinned_resource(
                        app_data_root,
                        blank.bytes,
                        &blank.sha256,
                    )?;
                    if bytes.len() != blank.bytes
                        || format!("{:x}", Sha256::digest(&bytes)) != blank.sha256
                    {
                        return Err("tree_resource_unverified".to_string());
                    }
                    blanks.insert(extension.clone(), bytes);
                }
                for (path, extension) in expansion.paths {
                    if let std::collections::btree_map::Entry::Vacant(slot) = resources.entry(path)
                    {
                        let bytes = blanks
                            .get(&extension)
                            .ok_or_else(|| "blacklist_resolution_invalid".to_string())?;
                        slot.insert(bytes.clone());
                    }
                }
            }
            Ok(BundlePackage {
                package_id: package_id.clone(),
                resources,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut packages = packages;
    apply_panorama_styles(app_data_root, &bundle_ids, &mut packages)?;
    apply_panorama_layouts(app_data_root, &bundle_ids, &mut packages)?;
    let mut verified = assemble(package_ids, packages, wardrobe_items, map)?;
    verified.game_files = game_files;
    verified.packs = packs;
    verified.hero_grids = hero_grids;
    Ok(verified)
}

/// The identity of a build that carries a map: the merged packages' plan (if
/// any), the full ordered package list and the map package itself (its hash
/// and the analyzer's contents identity). A build without a map keeps its
/// bundle plan's identity unchanged.
fn map_plan_id(bundle: &BundlePlan, package_ids: &[String], map: &LoadedMap) -> String {
    let mut hasher = Sha256::new();
    for part in [
        "betterfy-map-plan-v1",
        bundle.plan_id.as_str(),
        &package_ids.join("\n"),
        map.item.package_id.as_str(),
        map.sha256.as_str(),
        map.item.report.identity.as_str(),
    ] {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

/// Joins the merged packages and the map into one reviewed build.
fn assemble(
    package_ids: Vec<String>,
    packages: Vec<BundlePackage>,
    wardrobe: Vec<WardrobePlanItem>,
    map: Option<LoadedMap>,
) -> Result<VerifiedTreeVpk, String> {
    let mut verified = if packages.is_empty() {
        // A map and nothing to merge: there is no `pakNN_dir.vpk`.
        if map.is_none() {
            return Err("pilot_package_unsupported".to_string());
        }
        VerifiedTreeVpk {
            bytes: Vec::new(),
            sha256: String::new(),
            bundle_plan: BundlePlan {
                plan_id: String::new(),
                package_ids: Vec::new(),
                resource_count: 0,
                payload_bytes: 0,
                duplicates: Vec::new(),
                overrides: Vec::new(),
                contributions: Vec::new(),
            },
            map: None,
            package_ids: Vec::new(),
            plan_id: String::new(),
            wardrobe,
            wardrobe_conflicts: Vec::new(),
            game_files: BTreeMap::new(),
            packs: Vec::new(),
            hero_grids: false,
        }
    } else {
        build_verified_bundle(packages, wardrobe)?
    };
    if let Some(map) = map {
        verified.plan_id = map_plan_id(&verified.bundle_plan, &package_ids, &map);
        verified.map = Some(map);
    }
    verified.package_ids = package_ids;
    Ok(verified)
}

fn plan_for(verified: &VerifiedTreeVpk) -> TreePilotPlan {
    let hash = verified.sha256().to_string();
    let commits = verified
        .package_ids()
        .iter()
        .filter_map(|id| source_commit(id).ok())
        .collect::<BTreeSet<_>>();
    TreePilotPlan {
        plan_id: verified.plan_id().to_string(),
        package_id: verified.package_ids().join("+"),
        package_ids: verified.package_ids().to_vec(),
        source_commit: commits.into_iter().collect::<Vec<_>>().join(","),
        target_file: TARGET_FILE,
        resource_count: verified.bundle_plan().resource_count,
        resource_bytes: verified.bundle_plan().payload_bytes,
        vpk_bytes: verified.bytes().len(),
        vpk_sha256: hash,
        has_pak: verified.has_pak(),
        map: verified.map().map(|map| map.item.clone()),
        bundle_plan_id: verified.bundle_plan().plan_id.clone(),
        package_count: verified.package_ids().len(),
        duplicate_resources: verified.bundle_plan().duplicates.len(),
        overridden_resources: verified.bundle_plan().overrides.len(),
        duplicates: verified.bundle_plan().duplicates.clone(),
        overrides: verified.bundle_plan().overrides.clone(),
        contributions: verified.bundle_plan().contributions.clone(),
        wardrobe: verified.wardrobe.clone(),
        wardrobe_conflicts: verified.wardrobe_conflicts.clone(),
        packs: verified.packs.clone(),
        compatibility: "unknown",
        distribution: "internal_pilot",
        deploy_enabled: cfg!(target_os = "windows"),
    }
}

pub(crate) fn plan_from_verified_store(
    app_data_root: &Path,
    package_ids: &[String],
) -> Result<TreePilotPlan, String> {
    build_from_verified_store(app_data_root, package_ids).map(|verified| plan_for(&verified))
}

pub(crate) fn stage_from_verified_store(
    app_data_root: &Path,
    expected_plan_id: &str,
    package_ids: &[String],
    confirmed: bool,
) -> Result<crate::build_engine::BuildReceipt, String> {
    if !confirmed {
        return Err("build_confirmation_required".to_string());
    }
    let verified = build_from_verified_store(app_data_root, package_ids)?;
    crate::build_engine::stage_verified_vpk(app_data_root, &verified, expected_plan_id, confirmed)
}

fn verify_resources(
    contract: &[Resource],
    resources: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    if resources.len() != contract.len() {
        return Err("tree_resource_set_invalid".to_string());
    }
    let mut seen = BTreeSet::new();
    for expected in contract {
        if !seen.insert(expected.path.to_ascii_lowercase()) {
            return Err("tree_contract_invalid".to_string());
        }
        let bytes = resources
            .get(&expected.path)
            .ok_or_else(|| "tree_resource_set_invalid".to_string())?;
        if bytes.len() != expected.bytes
            || format!("{:x}", Sha256::digest(bytes)) != expected.sha256
        {
            return Err("tree_resource_unverified".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn tree_resources() -> Vec<Resource> {
    contract(PACKAGE_ID)
        .expect("tree manifest")
        .resources
        .clone()
}

#[cfg(test)]
pub(crate) fn build_verified_vpk(
    resources: &BTreeMap<String, Vec<u8>>,
) -> Result<VerifiedTreeVpk, String> {
    verify_resources(&tree_resources(), resources)?;
    build_verified_bundle(
        vec![BundlePackage {
            package_id: PACKAGE_ID.to_string(),
            resources: resources.clone(),
        }],
        Vec::new(),
    )
}

fn build_verified_bundle(
    packages: Vec<BundlePackage>,
    wardrobe: Vec<WardrobePlanItem>,
) -> Result<VerifiedTreeVpk, String> {
    let bundle = mod_bundle::build(packages)?;
    let report = vpk::inspect(bundle.bytes())?;
    if report.entries != bundle.plan().resource_count {
        return Err("tree_vpk_invalid".to_string());
    }
    // The bundle already records every path two packages write differently
    // (the first package wins). Those involving a wardrobe item are surfaced
    // on their own: a skin must never lose or win a path unseen.
    let items = wardrobe
        .iter()
        .map(|item| item.package_id.as_str())
        .collect::<BTreeSet<_>>();
    let wardrobe_conflicts = bundle
        .plan()
        .overrides
        .iter()
        .filter(|item| {
            items.contains(item.winner_package_id.as_str())
                || item
                    .shadowed_package_ids
                    .iter()
                    .any(|id| items.contains(id.as_str()))
        })
        .map(|item| WardrobeConflict {
            path: item.path.clone(),
            winner_package_id: item.winner_package_id.clone(),
            shadowed_package_ids: item.shadowed_package_ids.clone(),
        })
        .collect();
    Ok(VerifiedTreeVpk {
        bytes: bundle.bytes().to_vec(),
        sha256: bundle.sha256().to_string(),
        bundle_plan: bundle.plan().clone(),
        map: None,
        package_ids: bundle.plan().package_ids.clone(),
        plan_id: bundle.plan().plan_id.clone(),
        wardrobe,
        wardrobe_conflicts,
        game_files: BTreeMap::new(),
        packs: Vec::new(),
        hero_grids: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHOW_NETWORTH_PACKAGE_ID: &str = "minify.show-networth";
    const UNIT_QUERY_HUD_PACKAGE_ID: &str = "minify.repopulate-unit-query-hud";

    #[test]
    fn ledger_is_exact_and_bounded() {
        assert_eq!(PACKAGE_ID, "minify.tree-mod");
        let tree = contract(PACKAGE_ID).expect("tree manifest");
        assert_eq!(tree.source.commit.len(), 40);
        assert_eq!(TARGET_FILE, "pak66_dir.vpk");
        assert_eq!(tree.resources.len(), 21);
        assert!(tree.resources.iter().map(|item| item.bytes).sum::<usize>() < 1024 * 1024);
        assert!(tree.resources.iter().all(|item| item.sha256.len() == 64));
        let ledger = include_str!("../../docs/TREE_MOD_PILOT.md");
        for item in &tree.resources {
            assert!(ledger.contains(&format!(
                "| `{}` | {} | `{}` |",
                item.path, item.bytes, item.sha256
            )));
        }
    }

    #[test]
    fn verified_pilot_allowlist_preserves_selected_priority() {
        let ids = vec![
            "minify-show-networth".to_string(),
            "minify-tree-mod".to_string(),
        ];
        assert_eq!(
            normalize_package_ids(&ids).expect("known pilot packages"),
            vec![SHOW_NETWORTH_PACKAGE_ID.to_string(), PACKAGE_ID.to_string()]
        );
        let networth = &contract(SHOW_NETWORTH_PACKAGE_ID)
            .expect("manifest")
            .resources;
        assert_eq!(networth.len(), 1);
        assert_eq!(networth[0].bytes, 2705);
        assert_eq!(networth[0].sha256.len(), 64);
        assert!(pinned_url(SHOW_NETWORTH_PACKAGE_ID, &networth[0])
            .expect("pinned URL")
            .contains("/Show%20NetWorth/files/"));
        assert_eq!(
            normalize_package_ids(&["unknown".to_string()])
                .err()
                .as_deref(),
            Some("pilot_package_unsupported")
        );
    }

    #[test]
    #[ignore = "requires pinned upstream resources outside the repository"]
    fn downloads_and_builds_the_multi_package_pilot_in_selected_order() {
        let root =
            std::env::temp_dir().join(format!("betterfy-two-package-pilot-{}", std::process::id()));
        let ids = vec![
            PACKAGE_ID.to_string(),
            SHOW_NETWORTH_PACKAGE_ID.to_string(),
            UNIT_QUERY_HUD_PACKAGE_ID.to_string(),
        ];
        let cancelled = AtomicBool::new(false);
        acquire_verified_resources_with_progress(&root, &ids, &cancelled, |_| {})
            .expect("download both pinned contracts");
        let first = build_from_verified_store(&root, &ids).expect("build ordered bundle");
        assert_eq!(first.bundle_plan().package_ids, ids);
        assert_eq!(first.bundle_plan().resource_count, 25);
        let reversed = vec![
            UNIT_QUERY_HUD_PACKAGE_ID.to_string(),
            SHOW_NETWORTH_PACKAGE_ID.to_string(),
            PACKAGE_ID.to_string(),
        ];
        let second = build_from_verified_store(&root, &reversed).expect("build reversed bundle");
        assert_eq!(second.bundle_plan().package_ids, reversed);
        assert_ne!(first.plan_id(), second.plan_id());
        assert_eq!(first.sha256(), second.sha256());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn only_complete_untampered_resources_can_enter_vpk_writer() {
        assert_eq!(
            build_verified_vpk(&BTreeMap::new()).err().as_deref(),
            Some("tree_resource_set_invalid")
        );
        let contract = [Resource {
            path: "models/props_tree/tree_oak_01.vmdl_c".to_string(),
            bytes: 4,
            sha256: "dc9c5edb8b2d479e697b4b0b8ab874f32b325138598ce9e7b759eb8292110622".to_string(),
            from: None,
        }];
        let mut resources = BTreeMap::from([(contract[0].path.clone(), b"tree".to_vec())]);
        assert!(verify_resources(&contract, &resources).is_ok());
        assert!(mod_bundle::build(vec![BundlePackage {
            package_id: "test.tree".to_string(),
            resources: resources.clone(),
        }])
        .is_ok());
        resources.get_mut(&contract[0].path).unwrap()[0] ^= 1;
        assert_eq!(
            verify_resources(&contract, &resources).err().as_deref(),
            Some("tree_resource_unverified")
        );
        resources.remove(&contract[0].path);
        assert_eq!(
            verify_resources(&contract, &resources).err().as_deref(),
            Some("tree_resource_set_invalid")
        );
        resources.insert("../escape.vmdl_c".to_string(), b"tree".to_vec());
        assert_eq!(
            verify_resources(&contract, &resources).err().as_deref(),
            Some("tree_resource_set_invalid")
        );
    }

    #[test]
    fn cancelled_intake_does_not_start_a_download() {
        let cancelled = AtomicBool::new(true);
        let root = std::env::temp_dir().join("betterfy-tree-cancel-no-write");
        assert_eq!(
            acquire_verified_resources(&root, &cancelled)
                .err()
                .as_deref(),
            Some("download_cancelled")
        );
    }

    #[test]
    #[ignore = "requires the 21 pinned upstream resources outside the repository"]
    fn builds_the_real_pinned_tree_vpk_without_touching_dota() {
        let root = std::env::var_os("BETTERFY_TREE_RESOURCE_ROOT")
            .expect("set BETTERFY_TREE_RESOURCE_ROOT to a temporary resource directory");
        let resources = tree_resources()
            .iter()
            .map(|resource| {
                let bytes = std::fs::read(std::path::Path::new(&root).join(&resource.path))
                    .expect("read pinned resource");
                (resource.path.clone(), bytes)
            })
            .collect::<BTreeMap<_, _>>();
        let first = build_verified_vpk(&resources).expect("build exact pinned resources");
        let second = build_verified_vpk(&resources).expect("build again");
        assert_eq!(first.bytes(), second.bytes());
        let report = vpk::inspect(first.bytes()).expect("reopen VPK");
        assert_eq!(report.entries, tree_resources().len());
        assert_eq!(
            report.payload_bytes,
            tree_resources()
                .iter()
                .map(|entry| entry.bytes as u64)
                .sum::<u64>()
        );
        let app_data = std::path::Path::new(&root).join("betterfy-test-app-data");
        for resource in tree_resources() {
            crate::content_store::store_pinned_resource(
                &app_data,
                resource.bytes,
                &resource.sha256,
                resources.get(&resource.path).unwrap(),
            )
            .expect("publish verified pinned resource");
        }
        acquire_verified_resources(&app_data, &AtomicBool::new(false))
            .expect("reuse verified cache without network");
        let ids = vec![PACKAGE_ID.to_string()];
        let from_store =
            build_from_verified_store(&app_data, &ids).expect("build from immutable store");
        assert_eq!(from_store.bytes(), first.bytes());
        let plan = plan_from_verified_store(&app_data, &ids).expect("reviewed plan");
        assert_eq!(plan.resource_count, 21);
        assert_eq!(plan.deploy_enabled, cfg!(target_os = "windows"));
        let plan_id = plan.plan_id;
        assert_eq!(
            stage_from_verified_store(&app_data, &plan_id, &ids, false)
                .err()
                .as_deref(),
            Some("build_confirmation_required")
        );
        assert_eq!(
            stage_from_verified_store(&app_data, "sha256:stale", &ids, true)
                .err()
                .as_deref(),
            Some("build_plan_stale")
        );
        let receipt =
            stage_from_verified_store(&app_data, &plan_id, &ids, true).expect("stage verified VPK");
        let operation_id = serde_json::to_value(&receipt)
            .expect("receipt")
            .get("operationId")
            .and_then(|value| value.as_str())
            .expect("operation ID")
            .to_string();
        let staged = crate::build_engine::verified_staged_vpk(&app_data, &operation_id, &plan_id)
            .expect("reopen staged artifact");
        assert_eq!(staged.bytes, first.bytes());
        crate::build_engine::rollback_operation(&app_data, &operation_id)
            .expect("rollback staged artifact");
        let mut known = BTreeSet::from([operation_id]);
        for failure in [
            crate::build_engine::FailurePoint::AfterFirstWrite,
            crate::build_engine::FailurePoint::BeforeVerification,
        ] {
            assert_eq!(
                crate::build_engine::stage_verified_vpk_with_failure(
                    &app_data, &first, &plan_id, true, failure
                )
                .err()
                .as_deref(),
                Some("injected_failure")
            );
            let failed = crate::build_engine::list_operations(&app_data)
                .expect("read journals")
                .into_iter()
                .map(|operation| serde_json::to_value(operation).expect("summary"))
                .find(|operation| {
                    operation.get("phase").and_then(|value| value.as_str()) == Some("failed")
                        && operation
                            .get("operationId")
                            .and_then(|value| value.as_str())
                            .is_some_and(|id| !known.contains(id))
                })
                .expect("failed journal");
            let id = failed
                .get("operationId")
                .and_then(|value| value.as_str())
                .expect("failed operation ID")
                .to_string();
            let rollback = crate::build_engine::rollback_operation(&app_data, &id)
                .expect("rollback interrupted staging");
            assert_eq!(
                serde_json::to_value(rollback).unwrap()["removedStaging"],
                true
            );
            known.insert(id);
        }
    }

    #[test]
    #[ignore = "requires HTTPS access to the pinned upstream repository"]
    fn rust_intake_downloads_and_reuses_the_pinned_resources() {
        let root = std::env::temp_dir().join(format!(
            "betterfy-tree-rust-intake-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let cancelled = AtomicBool::new(false);
        begin_download(root.clone(), vec![PACKAGE_ID.to_string()], None)
            .expect("start pinned download");
        let mut completed = None;
        for _ in 0..1200 {
            let status = download_status()
                .expect("read download status")
                .expect("operation");
            if matches!(status.phase, "ready" | "failed" | "cancelled") {
                completed = Some(status);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let status = completed.expect("download completes within a minute");
        assert_eq!(
            status.phase, "ready",
            "download failed: {:?}",
            status.error_code
        );
        assert_eq!(status.verified_resources, tree_resources().len());
        assert!(status.plan.is_some());
        let ids = vec![PACKAGE_ID.to_string()];
        let first = build_from_verified_store(&root, &ids).expect("build from cache");
        acquire_verified_resources(&root, &cancelled).expect("idempotent cache read");
        let second = build_from_verified_store(&root, &ids).expect("rebuild from cache");
        assert_eq!(first.bytes(), second.bytes());
        std::fs::remove_dir_all(root).expect("clean generated test cache");
    }

    #[test]
    fn user_styles_install_only_with_a_pick_and_keep_it_in_the_id() {
        assert_eq!(
            normalize_package_ids(&["minify.user-styles".to_string()])
                .err()
                .as_deref(),
            Some("user_styles_selection_required")
        );
        assert_eq!(
            normalize_package_ids(&["minify-user-styles".to_string()])
                .err()
                .as_deref(),
            Some("user_styles_selection_required")
        );
        assert_eq!(
            normalize_package_ids(&["minify.user-styles.pick-5".to_string()]).expect("pick"),
            vec!["minify.user-styles.pick-5".to_string()]
        );
        // Two picks are still one package.
        assert!(normalize_package_ids(&[
            "minify.user-styles.pick-5".to_string(),
            "minify.user-styles.pick-6".to_string()
        ])
        .is_err());
        let manifest = contract("minify.user-styles.pick-5").expect("tuning contract");
        assert_eq!(manifest.id, "minify.user-styles");
    }

    #[test]
    fn a_custom_font_builds_as_every_renamed_weight() {
        use base64::Engine;
        let root = wardrobe_root("custom-font");
        assert_eq!(
            normalize_package_ids(&["minify-custom-fonts".to_string()])
                .err()
                .as_deref(),
            Some("custom_font_required")
        );
        let font = crate::font_names::test_font();
        let info = crate::custom_fonts::store(
            &root,
            &base64::engine::general_purpose::STANDARD.encode(&font),
        )
        .expect("stores");
        assert_eq!(info.family, "Original");
        let verified = build_from_verified_store(&root, std::slice::from_ref(&info.package_id))
            .expect("builds");
        assert_eq!(verified.game_files().len(), 29);
        let bold = verified
            .game_files()
            .get("panorama/fonts/radiance-bold.otf")
            .and_then(Option::as_ref)
            .expect("bold");
        assert_eq!(
            crate::font_names::family_name(bold).expect("family"),
            "Radiance"
        );
        // A font pack and a custom font both write the game's fonts.
        assert!(verified.has_pak());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_cursor_pack_builds_with_its_game_files_bound_to_the_vpk() {
        let root = wardrobe_root("cursor-pack");
        let (manifest, zip) = crate::game_packs::fixtures::cursor_pack("build-pack", b"x");
        let item = crate::game_packs::register_for_test(manifest);
        let files = crate::game_packs::extract_pinned_files(&zip, item).expect("extracts");
        for (file, bytes) in item.files.iter().zip(files) {
            crate::content_store::store_pinned_resource(&root, file.bytes, &file.sha256, &bytes)
                .expect("stores");
        }
        let verified = build_from_verified_store(&root, std::slice::from_ref(&item.catalog_id))
            .expect("builds");
        assert!(verified.has_pak(), "the file list goes into the VPK");
        assert_eq!(verified.game_files().len(), 3);
        assert!(verified
            .game_files()
            .get("resource/cursor/cursor.res")
            .is_some_and(Option::is_some));
        let plan = plan_for(&verified);
        assert_eq!(plan.packs.len(), 1);
        assert_eq!(plan.packs[0].files, 3);
        // The same pack with other bytes is another build.
        let (other, other_zip) = crate::game_packs::fixtures::cursor_pack("build-pack-2", b"y");
        let other = crate::game_packs::register_for_test(other);
        let files = crate::game_packs::extract_pinned_files(&other_zip, other).expect("extracts");
        for (file, bytes) in other.files.iter().zip(files) {
            crate::content_store::store_pinned_resource(&root, file.bytes, &file.sha256, &bytes)
                .expect("stores");
        }
        let second = build_from_verified_store(&root, std::slice::from_ref(&other.catalog_id))
            .expect("builds");
        assert_ne!(second.plan_id(), verified.plan_id());
        // Two cursor packs cannot go into one build.
        assert_eq!(
            normalize_package_ids(&[item.id.clone(), other.id.clone()])
                .err()
                .as_deref(),
            Some("game_pack_conflict")
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    fn wardrobe_root(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "betterfy-wardrobe-build-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        root
    }

    /// Registers a synthetic wardrobe item and stores its archive.
    fn stored_item(
        root: &Path,
        id: &str,
        hood: &'static [u8],
    ) -> &'static crate::wardrobe::WardrobeManifest {
        let (manifest, archive) = crate::wardrobe::synthetic_item(
            id,
            &[
                ("models/heroes/keeper_of_the_light/kotl_hood.vmdl_c", hood),
                (
                    "particles/units/heroes/hero_keeper_of_the_light/attack.vpcf_c",
                    b"attack",
                ),
                ("darkness/materials/body.vmat_c", b"body"),
                ("particles/basic_ambient/basic_ambient.vpcf_c", b"shared"),
                ("particles/darkness/flame.vpcf", b"source"),
            ],
            |_| {},
        );
        crate::content_store::store_pinned_resource(
            root,
            manifest.archive.inner_bytes,
            &manifest.archive.inner_sha256,
            &archive,
        )
        .expect("store archive");
        crate::wardrobe::register_for_test(manifest)
    }

    #[test]
    fn a_wardrobe_item_builds_into_the_bundle_with_its_report() {
        let root = wardrobe_root("single");
        let item = stored_item(&root, "build-single", b"hood");
        let ids = vec![item.catalog_id.clone()];
        // The catalog ID resolves to the engine ID, and a repeat is refused.
        assert_eq!(
            normalize_package_ids(&ids).expect("resolves"),
            vec![item.id.clone()]
        );
        assert_eq!(
            normalize_package_ids(&[item.id.clone(), item.catalog_id.clone()])
                .err()
                .as_deref(),
            Some("pilot_package_invalid")
        );
        // Everything is already in the store, so acquiring needs no network.
        let mut seen = 0;
        acquire_verified_resources_with_progress(&root, &ids, &AtomicBool::new(false), |count| {
            seen = count
        })
        .expect("stored archive is reused");
        assert_eq!(seen, 1);
        assert_eq!(
            acquire_verified_resources_with_progress(&root, &ids, &AtomicBool::new(true), |_| {})
                .err()
                .as_deref(),
            Some("download_cancelled")
        );

        let verified = build_from_verified_store(&root, &ids).expect("builds");
        assert_eq!(verified.bundle_plan().package_ids, vec![item.id.clone()]);
        // hero file, hero particle, author file and the one file that overrides
        // a Valve file; only the source file is out.
        assert_eq!(verified.bundle_plan().resource_count, 4);
        let report = &verified.wardrobe[0].report;
        assert_eq!(report.install.files, 4);
        assert_eq!(report.install.stripped_shared, 0);
        assert_eq!(report.shared.files, 1);
        assert_eq!(report.rejected.files, 0);
        assert_eq!(report.ignored.files, 1);
        assert!(verified.wardrobe_conflicts.is_empty());
        verified
            .require_acknowledged_conflicts(false)
            .expect("nothing to acknowledge");
        let reopened = vpk::extract_embedded(verified.bytes()).expect("reopen");
        assert!(reopened.contains_key("darkness/materials/body.vmat_c"));
        assert!(reopened.contains_key("particles/basic_ambient/basic_ambient.vpcf_c"));

        let plan = plan_for(&verified);
        let json = serde_json::to_value(&plan).expect("plan json");
        assert_eq!(json["wardrobe"][0]["packageId"], item.id.as_str());
        assert_eq!(json["wardrobe"][0]["report"]["install"]["files"], 4);
        assert_eq!(json["wardrobe"][0]["sharedPolicy"], "install");
        assert_eq!(
            json["wardrobe"][0]["report"]["install"]["strippedShared"],
            0
        );
        assert_eq!(
            json["wardrobe"][0]["report"]["sharedPaths"][0]["path"],
            "particles/basic_ambient/basic_ambient.vpcf_c"
        );
        assert_eq!(json["wardrobeConflicts"], serde_json::json!([]));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn two_items_writing_the_same_path_are_a_reviewed_conflict() {
        let root = wardrobe_root("conflict");
        let first = stored_item(&root, "build-first", b"first-hood");
        let second = stored_item(&root, "build-second", b"second-hood");
        let ids = vec![first.id.clone(), second.id.clone()];
        let verified = build_from_verified_store(&root, &ids).expect("builds");
        // Only the hood differs; the two identical files are duplicates.
        assert_eq!(
            verified.wardrobe_conflicts,
            vec![WardrobeConflict {
                path: "models/heroes/keeper_of_the_light/kotl_hood.vmdl_c".to_string(),
                winner_package_id: first.id.clone(),
                shadowed_package_ids: vec![second.id.clone()],
            }]
        );
        assert_eq!(verified.bundle_plan().duplicates.len(), 3);
        assert_eq!(
            verified
                .require_acknowledged_conflicts(false)
                .err()
                .as_deref(),
            Some("wardrobe_conflict_unacknowledged")
        );
        verified
            .require_acknowledged_conflicts(true)
            .expect("acknowledged");
        // The first item in the selection wins, and the order is part of the plan.
        let reopened = vpk::extract_embedded(verified.bytes()).expect("reopen");
        assert_eq!(
            reopened["models/heroes/keeper_of_the_light/kotl_hood.vmdl_c"],
            b"first-hood"
        );
        let reversed = build_from_verified_store(&root, &[second.id.clone(), first.id.clone()])
            .expect("builds reversed");
        assert_ne!(verified.plan_id(), reversed.plan_id());
        assert_eq!(reversed.wardrobe_conflicts[0].winner_package_id, second.id);
        let plan = serde_json::to_value(plan_for(&verified)).expect("plan json");
        assert_eq!(
            plan["wardrobeConflicts"][0]["path"],
            "models/heroes/keeper_of_the_light/kotl_hood.vmdl_c"
        );
        assert_eq!(plan["overriddenResources"], 1);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    /// Registers a synthetic pack with no hero (a raw VPK) and stores it.
    fn stored_pack(
        root: &Path,
        category: &str,
        id: &str,
        files: &[(&str, &[u8])],
    ) -> &'static crate::wardrobe::WardrobeManifest {
        let (manifest, archive) =
            crate::wardrobe::synthetic_pack(category, id, crate::wardrobe::SourceKind::Vpk, files);
        crate::content_store::store_pinned_resource(
            root,
            manifest.archive.inner_bytes,
            &manifest.archive.inner_sha256,
            &archive,
        )
        .expect("store archive");
        crate::wardrobe::register_for_test(manifest)
    }

    #[test]
    fn a_pack_with_no_hero_mixes_with_a_skin_and_shares_the_conflict_review() {
        let root = wardrobe_root("mixed");
        let skin = stored_item(&root, "build-mixed-skin", b"skin-hood");
        let courier = stored_pack(
            &root,
            "couriers",
            "mixed",
            &[
                ("models/courier/mixed/body.vmdl_c", b"body"),
                // the same path as the skin's hood, other bytes
                (
                    "models/heroes/keeper_of_the_light/kotl_hood.vmdl_c",
                    b"courier-hood",
                ),
            ],
        );
        let sounds = stored_pack(
            &root,
            "mega-kill",
            "mixed-a",
            &[("sounds/vo/announcer/kill.vsnd_c", b"one")],
        );
        let more_sounds = stored_pack(
            &root,
            "mega-kill",
            "mixed-b",
            &[("sounds/vo/announcer/kill.vsnd_c", b"two")],
        );
        let ids = vec![
            skin.id.clone(),
            courier.catalog_id.clone(),
            sounds.id.clone(),
            more_sounds.id.clone(),
        ];
        // A pack has no `.strip` mode: asking for one is not an item.
        assert!(normalize_package_ids(&[format!("{}.strip", courier.id)]).is_err());
        let verified = build_from_verified_store(&root, &ids).expect("builds");
        assert_eq!(verified.wardrobe.len(), 4);
        // two conflicts: the hood (skin over courier), the announcer line
        // (the first pack over the second)
        let mut conflicts = verified
            .wardrobe_conflicts
            .iter()
            .map(|c| (c.path.clone(), c.winner_package_id.clone()))
            .collect::<Vec<_>>();
        conflicts.sort();
        assert_eq!(
            conflicts,
            vec![
                (
                    "models/heroes/keeper_of_the_light/kotl_hood.vmdl_c".to_string(),
                    skin.id.clone()
                ),
                (
                    "sounds/vo/announcer/kill.vsnd_c".to_string(),
                    sounds.id.clone()
                ),
            ]
        );
        assert_eq!(
            verified
                .require_acknowledged_conflicts(false)
                .err()
                .as_deref(),
            Some("wardrobe_conflict_unacknowledged")
        );
        let reopened = vpk::extract_embedded(verified.bytes()).expect("reopen");
        assert_eq!(reopened["sounds/vo/announcer/kill.vsnd_c"], b"one");
        let plan = serde_json::to_value(plan_for(&verified)).expect("plan json");
        assert_eq!(plan["wardrobe"][0]["scope"], "hero");
        assert_eq!(plan["wardrobe"][1]["scope"], "none");
        assert_eq!(plan["wardrobe"][1]["category"], "couriers");
        assert_eq!(plan["wardrobe"][2]["category"], "mega-kill");
        assert_eq!(plan["wardrobe"][1]["heroes"], serde_json::json!([]));
        // the same build twice is the same bytes
        let again = build_from_verified_store(&root, &ids).expect("builds again");
        assert_eq!(verified.bytes(), again.bytes());
        assert_eq!(verified.plan_id(), again.plan_id());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_wardrobe_path_shared_with_a_tuning_package_is_a_conflict_too() {
        let bundle = |tuning: &[u8], skin: &[u8]| {
            let analysis_archive = crate::vpk::build(vec![crate::vpk::VpkInput {
                path: "models/heroes/keeper_of_the_light/a.vmdl_c",
                bytes: b"x",
            }])
            .expect("vpk");
            let report = crate::skin_archive::analyze(
                &analysis_archive,
                &crate::skin_archive::ScopeSpec {
                    kind: crate::skin_archive::ScopeKind::Hero,
                    valve_references: Vec::new(),
                    types: crate::skin_archive::TypePolicy::Resources,
                    heroes: vec!["keeper_of_the_light".to_string()],
                    shared_paths: Vec::new(),
                    shared_policy: crate::skin_archive::SharedPolicy::Strip,
                },
                &crate::skin_archive::Limits::default(),
            )
            .expect("analysis")
            .report;
            let item = WardrobePlanItem {
                package_id: "wardrobe.test-mix".to_string(),
                catalog_id: "heroes-test-mix".to_string(),
                category: "heroes".to_string(),
                scope: crate::skin_archive::ScopeKind::Hero,
                heroes: vec!["keeper_of_the_light".to_string()],
                source_urls: Vec::new(),
                archive_sha256: "0".repeat(64),
                catalog_snapshot: String::new(),
                permission: String::new(),
                verified_languages: Vec::new(),
                compatibility_note: None,
                shared_policy: SharedPolicy::Install,
                game_checked: false,
                report,
            };
            build_verified_bundle(
                vec![
                    BundlePackage {
                        package_id: PACKAGE_ID.to_string(),
                        resources: BTreeMap::from([
                            ("materials/water/river.vmat_c".to_string(), tuning.to_vec()),
                            ("sounds/ui/ping.vsnd_c".to_string(), b"ping".to_vec()),
                        ]),
                    },
                    BundlePackage {
                        package_id: item.package_id.clone(),
                        resources: BTreeMap::from([
                            ("materials/water/river.vmat_c".to_string(), skin.to_vec()),
                            ("sounds/ui/ping.vsnd_c".to_string(), b"ping".to_vec()),
                        ]),
                    },
                ],
                vec![item],
            )
            .expect("builds")
        };
        let differing = bundle(b"tuning", b"skin");
        assert_eq!(
            differing.wardrobe_conflicts,
            vec![WardrobeConflict {
                path: "materials/water/river.vmat_c".to_string(),
                winner_package_id: PACKAGE_ID.to_string(),
                shadowed_package_ids: vec!["wardrobe.test-mix".to_string()],
            }]
        );
        assert!(differing.require_acknowledged_conflicts(false).is_err());
        // Identical bytes are a duplicate, not a conflict.
        let identical = bundle(b"same", b"same");
        assert!(identical.wardrobe_conflicts.is_empty());
        assert_eq!(identical.bundle_plan().duplicates.len(), 2);
    }

    #[test]
    fn the_strip_mode_is_its_own_package_id_and_its_own_plan() {
        let root = wardrobe_root("strip-mode");
        let item = stored_item(&root, "build-strip-mode", b"hood");
        let strip_id = format!("{}.strip", item.id);
        // Either spelling of the item, in either mode, resolves to the item.
        assert_eq!(
            normalize_package_ids(&[format!("{}.strip", item.catalog_id)]).expect("resolves"),
            vec![strip_id.clone()]
        );
        // A skin is in a build once, whatever the mode.
        for both in [
            vec![item.id.clone(), strip_id.clone()],
            vec![strip_id.clone(), item.catalog_id.clone()],
        ] {
            assert_eq!(
                normalize_package_ids(&both).err().as_deref(),
                Some("pilot_package_invalid")
            );
        }
        // Not an item: a strip suffix on something else.
        assert!(normalize_package_ids(&["minify-tree-mod.strip".to_string()]).is_err());

        let install =
            build_from_verified_store(&root, std::slice::from_ref(&item.id)).expect("install");
        let strip =
            build_from_verified_store(&root, std::slice::from_ref(&strip_id)).expect("strip");
        assert_eq!(install.bundle_plan().resource_count, 4);
        assert_eq!(strip.bundle_plan().resource_count, 3);
        assert_eq!(strip.bundle_plan().package_ids, vec![strip_id.clone()]);
        assert_ne!(install.plan_id(), strip.plan_id());
        assert_ne!(install.sha256(), strip.sha256());
        let reopened = vpk::extract_embedded(strip.bytes()).expect("reopen");
        assert!(!reopened.contains_key("particles/basic_ambient/basic_ambient.vpcf_c"));
        let plan = serde_json::to_value(plan_for(&strip)).expect("plan json");
        assert_eq!(plan["wardrobe"][0]["sharedPolicy"], "strip");
        assert_eq!(plan["wardrobe"][0]["packageId"], strip_id.as_str());
        assert_eq!(
            plan["wardrobe"][0]["report"]["install"]["strippedShared"],
            1
        );
        // The plan the person reviewed is the one that stages: a different
        // mode is a different plan ID, so staging the other one is refused.
        let stale = stage_from_verified_store(&root, install.plan_id(), &[strip_id], true);
        assert!(stale.is_err());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_selection_that_cannot_fit_one_vpk_is_refused_before_any_download() {
        let root = wardrobe_root("budget");
        let big = |name: &str| {
            let (manifest, _) = crate::wardrobe::synthetic_item(
                name,
                &[
                    ("models/heroes/keeper_of_the_light/a.vmdl_c", b"a"),
                    ("particles/basic_ambient/basic_ambient.vpcf_c", b"b"),
                ],
                |value| {
                    value["archive"]["innerBytes"] = 121_000_000.into();
                    value["audited"]["installBytes"] = 120_000_000.into();
                    value["audited"]["stripBytes"] = 1_000.into();
                },
            );
            crate::wardrobe::register_for_test(manifest)
        };
        let items = (0..5)
            .map(|index| big(&format!("budget-skin-{index}")))
            .collect::<Vec<_>>();
        let ids = items.iter().map(|item| item.id.clone()).collect::<Vec<_>>();
        // 5 x 120 MB is more than the 512 MiB the merged VPK can hold.
        assert_eq!(
            begin_download(root.clone(), ids.clone(), None)
                .err()
                .as_deref(),
            Some("bundle_limit_exceeded")
        );
        // Four fit under the limit, which the check only reads from the pins.
        assert_eq!(
            wardrobe::install_footprint(&ids[..4]).1,
            480_000_000,
            "four skins stay under the limit"
        );
        // The opt-in mode counts the smaller pinned size.
        let strip_ids = ids
            .iter()
            .map(|id| format!("{id}.strip"))
            .collect::<Vec<_>>();
        assert_eq!(wardrobe::install_footprint(&strip_ids).1, 5_000);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_wardrobe_item_without_its_stored_archive_does_not_build() {
        let root = wardrobe_root("missing");
        let (manifest, _) = crate::wardrobe::synthetic_item(
            "build-missing",
            &[("models/heroes/keeper_of_the_light/a.vmdl_c", b"a")],
            |_| {},
        );
        let item = crate::wardrobe::register_for_test(manifest);
        assert!(build_from_verified_store(&root, std::slice::from_ref(&item.id)).is_err());
        // An unknown wardrobe-looking ID is not an item.
        assert_eq!(
            normalize_package_ids(&["wardrobe.unknown".to_string()])
                .err()
                .as_deref(),
            Some("pilot_package_unsupported")
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    /// Local-only: builds the real Scarlet Keeper archive (named by
    /// `BETTERFY_SKIN_SAMPLE`) through the whole store-to-VPK path and prints
    /// the numbers. The archive is never part of the repository.
    #[test]
    fn builds_the_real_sample_through_the_store_when_given() {
        let Ok(path) = std::env::var("BETTERFY_SKIN_SAMPLE") else {
            return;
        };
        let archive = std::fs::read(path).expect("sample");
        let item = wardrobe::find("heroes-scarlet-keeper-of-the-light")
            .expect("registry")
            .expect("item");
        let root = wardrobe_root("real");
        crate::content_store::store_pinned_resource(
            &root,
            item.archive.inner_bytes,
            &item.archive.inner_sha256,
            &archive,
        )
        .expect("store");
        let started = std::time::Instant::now();
        let verified = build_from_verified_store(&root, std::slice::from_ref(&item.catalog_id))
            .expect("builds from the store");
        let built = started.elapsed();
        let plan = serde_json::to_value(plan_for(&verified)).expect("plan json");
        println!(
            "real sample: {} resources, {} payload bytes, vpk {} bytes, sha256 {}, plan {}, built in {:?}",
            plan["resourceCount"],
            plan["resourceBytes"],
            plan["vpkBytes"],
            plan["vpkSha256"],
            plan["planId"],
            built
        );
        let report = &plan["wardrobe"][0]["report"];
        println!(
            "real sample classes: hero {} / author {} / shared {} / rejected {}, renamed {}",
            report["heroScoped"],
            report["authorOwned"],
            report["shared"],
            report["rejected"],
            report["renamed"]
        );
        assert_eq!(plan["resourceCount"], item.audited.install_files);
        assert_eq!(plan["resourceBytes"], item.audited.install_bytes);
        assert_eq!(
            plan["wardrobe"][0]["report"]["install"]["strippedShared"],
            0
        );
        assert_eq!(
            plan["wardrobe"][0]["report"]["shared"]["files"],
            item.audited.shared
        );
        assert_eq!(plan["wardrobeConflicts"], serde_json::json!([]));
        let again =
            build_from_verified_store(&root, std::slice::from_ref(&item.id)).expect("rebuild");
        assert_eq!(again.bytes(), verified.bytes(), "deterministic");
        assert_eq!(again.plan_id(), verified.plan_id());
        let reopened = vpk::inspect(verified.bytes()).expect("the VPK reopens");
        assert_eq!(reopened.entries, item.audited.install_files);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    // -- map terrains -------------------------------------------------------

    /// Registers a synthetic map and stores its package, as acquiring does.
    fn stored_map(
        root: &Path,
        slug: &str,
        label: &'static [u8],
    ) -> &'static crate::map_terrain::MapManifest {
        let (manifest, _zip, vpk) = crate::map_terrain::fixtures::synthetic_item(slug, label);
        crate::content_store::store_pinned_resource(
            root,
            manifest.archive.inner_bytes,
            &manifest.archive.inner_sha256,
            &vpk,
        )
        .expect("store map");
        crate::map_terrain::register_for_test(manifest)
    }

    fn fake_game(base: &Path) -> std::path::PathBuf {
        let dota = base.join("dota 2 beta");
        std::fs::create_dir_all(dota.join("game/dota")).expect("game");
        std::fs::write(dota.join("game/dota/pak01_dir.vpk"), b"official-marker").expect("marker");
        let executable = if cfg!(target_os = "windows") {
            dota.join("game/bin/win64/dota2.exe")
        } else if cfg!(target_os = "macos") {
            dota.join("game/bin/osx64/dota2")
        } else {
            dota.join("game/bin/linuxsteamrt64/dota2")
        };
        std::fs::create_dir_all(executable.parent().expect("parent")).expect("bin");
        std::fs::write(executable, b"executable-marker").expect("exe");
        dota
    }

    #[test]
    fn a_map_alone_builds_a_plan_with_no_pak_file() {
        let root = wardrobe_root("map-alone");
        let map = stored_map(&root, "plan-alone", b"alone");
        let ids = vec![map.catalog_id.clone()];
        assert_eq!(
            normalize_package_ids(&ids).expect("resolves"),
            vec![map.id.clone()]
        );
        // Everything is already in the store, so acquiring needs no network.
        let mut seen = 0;
        acquire_verified_resources_with_progress(&root, &ids, &AtomicBool::new(false), |count| {
            seen = count
        })
        .expect("stored map is reused");
        assert_eq!(seen, 1);
        let verified = build_from_verified_store(&root, &ids).expect("builds");
        assert!(!verified.has_pak());
        assert!(verified.bytes().is_empty());
        assert_eq!(verified.package_ids(), std::slice::from_ref(&map.id));
        assert_eq!(verified.primary_sha256(), map.archive.inner_sha256);
        assert!(verified.plan_id().starts_with("sha256:") && verified.plan_id().len() == 71);
        let payload = verified.payload();
        assert!(payload.pak.is_none());
        assert_eq!(
            payload.map.map(|file| file.sha256),
            Some(map.archive.inner_sha256.as_str())
        );
        let json = serde_json::to_value(plan_for(&verified)).expect("plan json");
        assert_eq!(json["hasPak"], false);
        assert_eq!(json["vpkBytes"], 0);
        assert_eq!(json["packageCount"], 1);
        assert_eq!(json["map"]["packageId"], map.id.as_str());
        assert_eq!(json["map"]["file"], "maps/dota.vpk");
        assert_eq!(json["map"]["sha256"], map.archive.inner_sha256.as_str());
        assert_eq!(json["map"]["valveOverlap"], 0);
        assert_eq!(json["map"]["report"]["mainMap"], true);
        assert_eq!(json["map"]["report"]["entries"], 4);
        assert_eq!(json["wardrobe"], serde_json::json!([]));
        let again = build_from_verified_store(&root, &ids).expect("rebuild");
        assert_eq!(again.plan_id(), verified.plan_id(), "deterministic");
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_map_beside_a_skin_binds_the_map_into_the_plan_and_leaves_the_pak_alone() {
        let root = wardrobe_root("map-and-skin");
        let skin = stored_item(&root, "plan-skin", b"hood");
        let map = stored_map(&root, "plan-with-skin", b"with skin");
        let other = stored_map(&root, "plan-other", b"other map");
        let skin_only =
            build_from_verified_store(&root, std::slice::from_ref(&skin.id)).expect("skin alone");
        let ids = vec![skin.id.clone(), map.id.clone()];
        let both = build_from_verified_store(&root, &ids).expect("skin and map");
        // The merged VPK is the very same file; only the identity changes.
        assert_eq!(both.bytes(), skin_only.bytes());
        assert_eq!(both.sha256(), skin_only.sha256());
        assert_eq!(both.bundle_plan().plan_id, skin_only.bundle_plan().plan_id);
        assert_ne!(both.plan_id(), skin_only.plan_id());
        assert_eq!(both.plan_id().len(), 71);
        assert_eq!(both.package_ids(), [skin.id.clone(), map.id.clone()]);
        assert_eq!(both.primary_sha256(), both.sha256());
        // The reviewed identity moves with the map and with the order.
        let other_ids = vec![skin.id.clone(), other.id.clone()];
        let with_other = build_from_verified_store(&root, &other_ids).expect("other map");
        assert_ne!(with_other.plan_id(), both.plan_id());
        let swapped = vec![map.id.clone(), skin.id.clone()];
        let reordered = build_from_verified_store(&root, &swapped).expect("map first");
        assert_ne!(reordered.plan_id(), both.plan_id());
        let json = serde_json::to_value(plan_for(&both)).expect("plan json");
        assert_eq!(json["hasPak"], true);
        assert_eq!(json["packageCount"], 2);
        assert_eq!(json["packageIds"], serde_json::json!([skin.id, map.id]));
        assert_eq!(json["map"]["packageId"], map.id.as_str());
        assert_eq!(json["wardrobe"][0]["packageId"], skin.id.as_str());
        // Two maps conflict, and the engine says so before downloading.
        let two = vec![map.id.clone(), other.id.clone()];
        assert_eq!(
            normalize_package_ids(&two).err().as_deref(),
            Some("map_terrain_conflict")
        );
        assert_eq!(
            build_from_verified_store(&root, &two).err().as_deref(),
            Some("map_terrain_conflict")
        );
        // A map has no shared-files mode.
        assert!(normalize_package_ids(&[format!("{}.strip", map.id)]).is_err());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_map_that_is_not_in_the_store_or_drifted_from_its_audit_is_refused() {
        let root = wardrobe_root("map-drift");
        let (manifest, _zip, _vpk) =
            crate::map_terrain::fixtures::synthetic_item("plan-missing", b"missing");
        let missing = crate::map_terrain::register_for_test(manifest);
        assert!(build_from_verified_store(&root, std::slice::from_ref(&missing.id)).is_err());
        // A manifest whose audit does not describe the stored bytes.
        let (mut drifted, _zip, vpk) =
            crate::map_terrain::fixtures::synthetic_item("plan-drifted", b"drifted");
        crate::content_store::store_pinned_resource(
            &root,
            drifted.archive.inner_bytes,
            &drifted.archive.inner_sha256,
            &vpk,
        )
        .expect("store");
        drifted.audited.report.entries += 0;
        drifted.audited.report.identity = "e".repeat(64);
        let drifted = crate::map_terrain::register_for_test(drifted);
        assert_eq!(
            build_from_verified_store(&root, std::slice::from_ref(&drifted.id))
                .err()
                .as_deref(),
            Some("map_audit_mismatch")
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_staged_map_build_reopens_and_installs_and_restores_end_to_end() {
        use crate::game_deployment as deployment;
        let root = wardrobe_root("map-e2e");
        let dota = fake_game(&root);
        let app = root.clone();
        let skin = stored_item(&root, "e2e-skin", b"hood");
        let map = stored_map(&root, "e2e-map", b"e2e");
        let ids = vec![skin.id.clone(), map.id.clone()];
        let verified = build_from_verified_store(&root, &ids).expect("builds");
        let staged = stage_from_verified_store(&root, verified.plan_id(), &ids, true)
            .expect("stages both files");
        assert_eq!(
            serde_json::to_value(&staged).expect("receipt json")["stagedFiles"],
            2
        );
        let reopened = crate::build_engine::verified_staged_files(
            &root,
            &staged.operation_id,
            verified.plan_id(),
        )
        .expect("reopens");
        assert_eq!(
            reopened.pak.as_ref().map(|file| file.sha256.as_str()),
            Some(verified.sha256())
        );
        assert_eq!(
            reopened.map.as_ref().map(|file| file.bytes.as_slice()),
            verified.map().map(|map| map.bytes.as_slice())
        );
        // The single-file contract does not accept a build with a map.
        assert_eq!(
            crate::build_engine::verified_staged_vpk(
                &root,
                &staged.operation_id,
                verified.plan_id()
            )
            .err()
            .as_deref(),
            Some("staged_vpk_not_ready")
        );
        // A different reviewed plan does not reopen it.
        assert!(crate::build_engine::verified_staged_files(
            &root,
            &staged.operation_id,
            &format!("sha256:{}", "0".repeat(64)),
        )
        .is_err());

        let receipt = deployment::deploy_verified_payload_for_language(
            &app,
            &dota,
            verified.payload(),
            crate::game_language::GameLanguage::Russian,
            deployment::DeploymentBundleIdentity {
                plan_id: Some(verified.plan_id().to_string()),
                package_ids: verified.package_ids().to_vec(),
            },
        )
        .expect("deploys both");
        assert!(verified.matches_receipt(&receipt));
        assert_eq!(receipt.package_ids, [skin.id.clone(), map.id.clone()]);
        let current = deployment::current_owned_deployment(&app, &dota)
            .expect("current")
            .expect("owned");
        assert!(verified.matches_receipt(&current));
        // The same build without the map is a different build.
        let skin_only =
            build_from_verified_store(&root, std::slice::from_ref(&skin.id)).expect("skin alone");
        assert!(!skin_only.matches_receipt(&current));
        // And so is a map-only build.
        let map_only =
            build_from_verified_store(&root, std::slice::from_ref(&map.id)).expect("map alone");
        assert!(!map_only.matches_receipt(&current));
        assert!(dota.join("game/dota_russian/maps/dota.vpk").is_file());
        deployment::rollback(&app, &dota, &receipt.operation_id).expect("restore");
        assert!(!dota.join("game/dota_russian").exists());
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}

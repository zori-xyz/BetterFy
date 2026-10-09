//! Download, verification and bundling for the pinned, data-only packages
//! described by `package_registry` manifests.

use crate::mod_bundle::{
    self, BundleContribution, BundleDuplicate, BundleOverride, BundlePackage, BundlePlan,
};
use crate::package_registry::{self, PackageManifest, Resource};
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
) -> Result<Option<BTreeMap<String, String>>, String> {
    let Some(styles) = &manifest.panorama_styles else {
        return Ok(None);
    };
    let bytes =
        crate::content_store::read_pinned_resource(app_data_root, styles.bytes, &styles.sha256)?;
    let text = String::from_utf8(bytes).map_err(|_| "panorama_styling_invalid".to_string())?;
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
        let Some(sections) = read_styling(app_data_root, manifest)? else {
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
        let Some(sections) = read_styling(app_data_root, manifest)? else {
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
            let id = resolve(id)?.engine_id().to_string();
            if !seen.insert(id.clone()) {
                return Err("pilot_package_invalid".to_string());
            }
            Ok(id)
        })
        .collect::<Result<Vec<_>, String>>()?;
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
    Wardrobe(&'static WardrobeManifest),
}

impl Contract {
    fn engine_id(&self) -> &str {
        match self {
            Self::Tuning(manifest) => &manifest.id,
            Self::Wardrobe(item) => &item.id,
        }
    }
}

fn resolve(id: &str) -> Result<Contract, String> {
    if let Some(item) = wardrobe::find(id)? {
        return Ok(Contract::Wardrobe(item));
    }
    package_registry::find(id).map(Contract::Tuning)
}

/// The tuning manifest behind an ID, or `None` for a wardrobe item, which has
/// no blacklist, style or layout edits.
fn tuning(id: &str) -> Result<Option<&'static PackageManifest>, String> {
    match resolve(id)? {
        Contract::Tuning(manifest) => Ok(Some(manifest)),
        Contract::Wardrobe(_) => Ok(None),
    }
}

/// A tuning package only; wardrobe items have no pinned per-file contract.
fn contract(id: &str) -> Result<&'static PackageManifest, String> {
    tuning(id)?.ok_or_else(|| "pilot_package_unsupported".to_string())
}

pub(crate) struct VerifiedTreeVpk {
    bytes: Vec<u8>,
    sha256: String,
    bundle_plan: BundlePlan,
    /// What the analyzer found for each selected wardrobe item, in selection
    /// order. Empty for a build of tuning packages only.
    wardrobe: Vec<WardrobePlanItem>,
    /// Paths a wardrobe item writes that another package writes differently.
    wardrobe_conflicts: Vec<WardrobeConflict>,
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
        &self.bundle_plan.plan_id
    }

    pub(crate) fn bundle_plan(&self) -> &BundlePlan {
        &self.bundle_plan
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
        wardrobe: Vec::new(),
        wardrobe_conflicts: Vec::new(),
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
            Contract::Tuning(manifest) => manifest,
            // One pinned archive: fetched, verified and stored as a unit.
            Contract::Wardrobe(item) => {
                if cancelled.load(Ordering::Relaxed) {
                    return Err("download_cancelled".to_string());
                }
                wardrobe::acquire(app_data_root, item, cancelled)?;
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
    let mut wardrobe_items = Vec::new();
    let packages = package_ids
        .iter()
        .map(|package_id| {
            let manifest = match resolve(package_id)? {
                Contract::Tuning(manifest) => manifest,
                // The analyzer re-checks the stored archive against its audit
                // and returns only the files that audit allows.
                Contract::Wardrobe(item) => {
                    let loaded = wardrobe::load(app_data_root, item)?;
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
    apply_panorama_styles(app_data_root, &package_ids, &mut packages)?;
    apply_panorama_layouts(app_data_root, &package_ids, &mut packages)?;
    build_verified_bundle(packages, wardrobe_items)
}

fn plan_for(verified: &VerifiedTreeVpk) -> TreePilotPlan {
    let hash = verified.sha256().to_string();
    let commits = verified
        .bundle_plan()
        .package_ids
        .iter()
        .filter_map(|id| source_commit(id).ok())
        .collect::<BTreeSet<_>>();
    TreePilotPlan {
        plan_id: verified.plan_id().to_string(),
        package_id: verified.bundle_plan().package_ids.join("+"),
        package_ids: verified.bundle_plan().package_ids.clone(),
        source_commit: commits.into_iter().collect::<Vec<_>>().join(","),
        target_file: TARGET_FILE,
        resource_count: verified.bundle_plan().resource_count,
        resource_bytes: verified.bundle_plan().payload_bytes,
        vpk_bytes: verified.bytes().len(),
        vpk_sha256: hash,
        bundle_plan_id: verified.bundle_plan().plan_id.clone(),
        package_count: verified.bundle_plan().package_ids.len(),
        duplicate_resources: verified.bundle_plan().duplicates.len(),
        overridden_resources: verified.bundle_plan().overrides.len(),
        duplicates: verified.bundle_plan().duplicates.clone(),
        overrides: verified.bundle_plan().overrides.clone(),
        contributions: verified.bundle_plan().contributions.clone(),
        wardrobe: verified.wardrobe.clone(),
        wardrobe_conflicts: verified.wardrobe_conflicts.clone(),
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
        wardrobe,
        wardrobe_conflicts,
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
        // hero file, hero particle, author file; the shared and source files are out.
        assert_eq!(verified.bundle_plan().resource_count, 3);
        let report = &verified.wardrobe[0].report;
        assert_eq!(report.install.files, 3);
        assert_eq!(report.install.stripped_shared, 1);
        assert_eq!(report.rejected.files, 1);
        assert!(verified.wardrobe_conflicts.is_empty());
        verified
            .require_acknowledged_conflicts(false)
            .expect("nothing to acknowledge");
        let reopened = vpk::extract_embedded(verified.bytes()).expect("reopen");
        assert!(reopened.contains_key("darkness/materials/body.vmat_c"));
        assert!(!reopened.contains_key("particles/basic_ambient/basic_ambient.vpcf_c"));

        let plan = plan_for(&verified);
        let json = serde_json::to_value(&plan).expect("plan json");
        assert_eq!(json["wardrobe"][0]["packageId"], item.id.as_str());
        assert_eq!(json["wardrobe"][0]["report"]["install"]["files"], 3);
        assert_eq!(
            json["wardrobe"][0]["report"]["install"]["strippedShared"],
            1
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
        assert_eq!(verified.bundle_plan().duplicates.len(), 2);
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
                    hero: "keeper_of_the_light".to_string(),
                    author_namespaces: Vec::new(),
                    shared_policy: crate::skin_archive::SharedPolicy::Strip,
                },
                &crate::skin_archive::Limits::default(),
            )
            .expect("analysis")
            .report;
            let item = WardrobePlanItem {
                package_id: "wardrobe.test-mix".to_string(),
                catalog_id: "heroes-test-mix".to_string(),
                hero: "keeper_of_the_light".to_string(),
                source_url: String::new(),
                archive_sha256: "0".repeat(64),
                catalog_snapshot: String::new(),
                permission: String::new(),
                verified_languages: Vec::new(),
                compatibility_note: None,
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
        assert_eq!(plan["resourceCount"], item.audited.install_files);
        assert_eq!(plan["resourceBytes"], item.audited.install_bytes);
        assert_eq!(
            plan["wardrobe"][0]["report"]["install"]["strippedShared"],
            item.audited.stripped_shared
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
}

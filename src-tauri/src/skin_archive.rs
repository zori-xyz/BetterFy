//! Third-party hero skin archives ("wardrobe" content).
//!
//! A skin arrives as a VPK whose directory holds every file. Nothing in it is
//! trusted. [`analyze`] reads the directory, classifies every path and turns
//! the archive plus a hero ID into the list of files that may be installed,
//! with a typed report of everything that is not.
//!
//! Rules (see `docs/SKIN_ARCHIVE_AUDIT.md`):
//!
//! * Types. Only compiled resource types in [`ALLOWED_TYPES`] are accepted.
//!   Compiled Panorama scripts, uncompiled sources, executables and anything
//!   unknown are rejected. A `vxml_c` is accepted only when the layout reader
//!   proves it is an image wrapper (`panorama_layout::image_wrapper_dependency`);
//!   if the reader cannot prove it, the file is rejected.
//! * Scope. Every accepted path is hero-scoped, author-owned or shared.
//!   Hero-scoped paths are the exact folders of the one hero the skin is for.
//!   Author-owned paths are a top-level folder that is not one of Valve's
//!   (`darkness/...`), or a file or folder under `particles/`, `materials/` or
//!   `models/` named after a namespace the audited entry declares
//!   (`particles/darkness_snaps/...`). Namespaces are declared by a human
//!   audit and never derived from the archive itself, because an archive that
//!   names its own top-level folder `units` must not be able to claim
//!   `particles/units/...`. Everything else is shared: it would change how
//!   other heroes or the interface look.
//! * Shared paths follow [`SharedPolicy`]: `Strip` drops them and lists every
//!   one in the report, `Refuse` refuses the install.
//!
//! What this does not prove: that the bytes of an accepted compiled file are
//! well formed. The type allowlist limits what the game is asked to load; it
//! is not a parser of every Source 2 resource. The archive is therefore
//! installable only when its hash is pinned (see `wardrobe.rs`).

use crate::vpk;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

/// Compiled resource types a skin may carry.
pub(crate) const ALLOWED_TYPES: &[&str] = &[
    "vpcf_c",
    "vsnap_c",
    "vtex_c",
    "vmat_c",
    "vmdl_c",
    "vmesh_c",
    "vanim_c",
    "vsndevts_c",
    "vsnd_c",
    "vmorf_c",
    "vagrp_c",
    "vphys_c",
];
const LAYOUT_TYPE: &str = "vxml_c";

/// Uncompiled sources the game never loads from a VPK. Their presence is a
/// leftover of the author's work tree, so they are dropped.
const SOURCE_TYPES: &[&str] = &[
    "vpcf", "vmdl", "vmat", "vtex", "vmesh", "vanim", "vsnd", "vsndevts", "vxml", "vcss", "vts",
    "vmap", "vsnap", "vmorf", "vagrp", "vphys", "fbx", "dmx", "tga", "png", "jpg", "jpeg", "psd",
    "vsvg", "svg",
];
const SCRIPT_TYPES: &[&str] = &["vjs_c", "vjs", "js", "jse", "lua", "py", "ts"];
const EXECUTABLE_TYPES: &[&str] = &[
    "exe", "dll", "bat", "cmd", "ps1", "com", "scr", "msi", "vbs", "vbe", "lnk", "sys", "drv",
    "ocx", "sh", "so", "dylib", "jar", "app",
];

/// Top-level folders Valve's own archives use (and a few more, on purpose:
/// listing a name here only means a folder of that name is never "author
/// owned"). A top-level folder outside this list cannot replace any game file.
const VALVE_ROOTS: &[&str] = &[
    "addons",
    "ambient",
    "bin",
    "cfg",
    "core",
    "custom_games",
    "dota",
    "expressions",
    "fonts",
    "game",
    "maps",
    "materials",
    "media",
    "models",
    "movies",
    "panorama",
    "particles",
    "resource",
    "scenes",
    "scripts",
    "shaders",
    "soundevents",
    "sounds",
    "soundstacks",
    "textures",
    "ui",
    "vscripts",
];
/// Roots under which a declared namespace may prefix a folder or file name.
const NAMESPACE_ROOTS: &[&str] = &["particles", "materials", "models"];
/// Folder names that Valve uses below the roots above. A declared namespace
/// must not be one of them, or it would claim a shared game folder.
const RESERVED_NAMESPACES: &[&str] = &[
    "basic", "cubemaps", "default", "econ", "generic", "heroes", "hero", "items", "particle",
    "status", "units", "world", "creeps", "ui", "props", "water", "weather",
];

const MAX_SEGMENT_BYTES: usize = 96;
const MAX_PATH_BYTES: usize = 240;
const MAX_REPORTED_PATHS: usize = 2_000;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SharedPolicy {
    /// Drop shared paths from the install and list each one.
    Strip,
    /// Refuse the whole install when the archive carries any shared path.
    Refuse,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ScopeClass {
    HeroScoped,
    AuthorOwned,
    Shared,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RejectReason {
    /// Not a plain lowercase relative path (traversal, uppercase, odd bytes).
    PathUnsafe,
    /// Two entries whose paths are equal when case is ignored.
    DuplicatePath,
    Oversized,
    EmptyResource,
    /// A compiled or plain script.
    Script,
    /// Uncompiled source such as `.vpcf`, `.vmdl`, `.vmat`.
    UncompiledSource,
    Executable,
    /// Any other type, including compiled types that are not allowed.
    TypeNotAllowed,
    /// A `vxml_c` the layout reader could not prove to be an image wrapper.
    PanoramaUnproven,
}

impl RejectReason {
    /// Reasons that mean the archive is not what an audit describes. They
    /// block the install instead of being dropped.
    pub(crate) fn is_hostile(self) -> bool {
        matches!(self, Self::PathUnsafe | Self::Script | Self::Executable)
    }
}

/// The audited facts an archive is checked against.
#[derive(Clone, Debug)]
pub(crate) struct ScopeSpec {
    pub hero: String,
    pub author_namespaces: Vec<String>,
    pub shared_policy: SharedPolicy,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Limits {
    pub max_entry_bytes: usize,
    pub max_install_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entry_bytes: 16 * 1024 * 1024,
            max_install_bytes: 128 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClassStat {
    pub files: usize,
    pub bytes: u64,
}

impl ClassStat {
    fn add(&mut self, bytes: usize) {
        self.files += 1;
        self.bytes += bytes as u64;
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RejectedPath {
    pub path: String,
    pub reason: RejectReason,
    pub bytes: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SharedPath {
    pub path: String,
    pub bytes: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstallSummary {
    pub files: usize,
    pub bytes: u64,
    /// SHA-256 over the sorted path and payload hash of the files to install.
    pub identity: String,
    /// Shared files dropped under [`SharedPolicy::Strip`].
    pub stripped_shared: usize,
    /// Stable code when the policy or the archive forbids installing.
    pub blocked: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkinReport {
    pub hero: String,
    pub shared_policy: SharedPolicy,
    pub entries: usize,
    pub total_bytes: u64,
    /// SHA-256 over the sorted path and payload hash of every entry.
    pub archive_identity: String,
    pub hero_scoped: ClassStat,
    pub author_owned: ClassStat,
    pub shared: ClassStat,
    pub rejected: ClassStat,
    /// `vxml_c` files proven to be image wrappers (counted in the classes).
    pub image_wrappers: usize,
    pub rejected_paths: Vec<RejectedPath>,
    pub shared_paths: Vec<SharedPath>,
    /// Top-level folders outside Valve's own: candidates for a human to
    /// review and declare as the entry's namespaces. Informational only.
    pub suggested_namespaces: Vec<String>,
    pub hostile: bool,
    pub install: InstallSummary,
}

struct Accepted<'a> {
    path: String,
    class: ScopeClass,
    payload: Cow<'a, [u8]>,
    digest: [u8; 32],
}

pub(crate) struct Analysis<'a> {
    pub report: SkinReport,
    accepted: Vec<Accepted<'a>>,
    max_install_bytes: usize,
}

/// The files to install and the report that explains them.
#[derive(Debug)]
pub(crate) struct Selection {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: SkinReport,
}

fn token(value: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&value.len())
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

impl ScopeSpec {
    pub(crate) fn validate(&self) -> Result<(), String> {
        let invalid = || "skin_scope_invalid".to_string();
        if !token(&self.hero, 2, 48) || self.author_namespaces.len() > 8 {
            return Err(invalid());
        }
        let mut seen = BTreeSet::new();
        for namespace in &self.author_namespaces {
            if !token(namespace, 4, 48)
                || VALVE_ROOTS.contains(&namespace.as_str())
                || RESERVED_NAMESPACES.contains(&namespace.as_str())
                || namespace.starts_with("hero_")
                || *namespace == self.hero
                || !seen.insert(namespace.as_str())
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

fn path_safe(path: &str) -> bool {
    path.len() <= MAX_PATH_BYTES
        && vpk::validate_path(path).is_ok()
        && !path.contains(' ')
        && path.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment.len() <= MAX_SEGMENT_BYTES
        })
}

fn extension(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .and_then(|name| name.rsplit_once('.'))
        .map_or("", |(_, extension)| extension)
}

enum Kind {
    Compiled,
    Layout,
}

fn classify_type(extension: &str) -> Result<Kind, RejectReason> {
    if ALLOWED_TYPES.contains(&extension) {
        Ok(Kind::Compiled)
    } else if extension == LAYOUT_TYPE {
        Ok(Kind::Layout)
    } else if SCRIPT_TYPES.contains(&extension) {
        Err(RejectReason::Script)
    } else if EXECUTABLE_TYPES.contains(&extension) {
        Err(RejectReason::Executable)
    } else if SOURCE_TYPES.contains(&extension) {
        Err(RejectReason::UncompiledSource)
    } else {
        Err(RejectReason::TypeNotAllowed)
    }
}

/// `name` is `<stem>.<ext>`; true when the stem continues `prefix` with `_`
/// or ends it with the extension dot.
fn named_after(name: &str, prefix: &str) -> bool {
    name.strip_prefix(prefix)
        .is_some_and(|rest| rest.starts_with('_') || rest.starts_with('.'))
}

fn scope_of(path: &str, spec: &ScopeSpec) -> ScopeClass {
    let segments = path.split('/').collect::<Vec<_>>();
    let hero = spec.hero.as_str();
    let name = segments[segments.len() - 1];
    let directories = &segments[..segments.len() - 1];
    let hero_prefix = format!("hero_{hero}");
    let hero_icon = format!("npc_dota_hero_{hero}");
    let hero_scoped = match directories {
        ["models", "heroes", folder, ..] => *folder == hero,
        ["particles", "units", "heroes", folder, ..] => *folder == hero_prefix,
        // A few Valve particles sit next to the hero folders, named after the
        // hero (`keeper_of_the_light_radiant_bind_groundfx`).
        ["particles", "units", "heroes"] => named_after(name, hero),
        ["particles", "econ", "items", folder, ..] => *folder == hero,
        ["materials", "models", "heroes" | "items", folder, ..] => *folder == hero,
        ["panorama", "images", "heroes"] => named_after(name, &hero_icon),
        ["panorama", "images", "spellicons"] => named_after(name, hero),
        _ => false,
    };
    if hero_scoped {
        return ScopeClass::HeroScoped;
    }
    // A top-level folder outside Valve's cannot replace any game file.
    if segments.len() >= 2 && !VALVE_ROOTS.contains(&segments[0]) {
        return ScopeClass::AuthorOwned;
    }
    // `segments[1]` is a folder or, for a file directly under the root
    // (`particles/darkness_snapshot1.vsnap_c`), the file name.
    if segments.len() >= 2
        && NAMESPACE_ROOTS.contains(&segments[0])
        && spec
            .author_namespaces
            .iter()
            .any(|namespace| segments[1] == namespace || named_after(segments[1], namespace))
    {
        return ScopeClass::AuthorOwned;
    }
    ScopeClass::Shared
}

fn identity(domain: &str, items: &mut [(String, [u8; 32])]) -> String {
    items.sort();
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    for (path, digest) in items.iter() {
        hasher.update((path.len() as u64).to_le_bytes());
        hasher.update(path.as_bytes());
        hasher.update(digest);
    }
    format!("{:x}", hasher.finalize())
}

fn sha256_of(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Reads the directory of an archive (VPK version 1 or 2, every entry
/// embedded) and classifies each entry. Failures of the container itself
/// (bad header, side archives, CRC mismatch) are errors; everything wrong
/// with a single entry is a reported rejection.
pub(crate) fn analyze<'a>(
    archive: &'a [u8],
    spec: &ScopeSpec,
    limits: &Limits,
) -> Result<Analysis<'a>, String> {
    spec.validate()?;
    let entries = vpk::read_embedded_directory(archive)?;
    let mut by_lowercase = BTreeMap::<String, usize>::new();
    for entry in &entries {
        *by_lowercase
            .entry(entry.path.to_ascii_lowercase())
            .or_default() += 1;
    }

    let mut all = Vec::with_capacity(entries.len());
    let mut accepted = Vec::new();
    let mut rejected_paths = Vec::new();
    let mut shared_paths = Vec::new();
    let mut hero_scoped = ClassStat::default();
    let mut author_owned = ClassStat::default();
    let mut shared = ClassStat::default();
    let mut rejected = ClassStat::default();
    let mut namespaces = BTreeSet::new();
    let mut image_wrappers = 0usize;
    let mut hostile = false;
    let mut total_bytes = 0u64;

    for entry in &entries {
        let payload = entry.payload();
        let digest = sha256_of(&payload);
        total_bytes += payload.len() as u64;
        all.push((entry.path.clone(), digest));
        let path = entry.path.as_str();

        let outcome = (|| -> Result<(ScopeClass, bool), RejectReason> {
            if by_lowercase[&path.to_ascii_lowercase()] > 1 {
                return Err(RejectReason::DuplicatePath);
            }
            if !path_safe(path) {
                return Err(RejectReason::PathUnsafe);
            }
            let kind = classify_type(extension(path))?;
            if payload.is_empty() {
                return Err(RejectReason::EmptyResource);
            }
            if payload.len() > limits.max_entry_bytes {
                return Err(RejectReason::Oversized);
            }
            let wrapper = matches!(kind, Kind::Layout);
            if wrapper && crate::panorama_layout::image_wrapper_dependency(path, &payload).is_err()
            {
                return Err(RejectReason::PanoramaUnproven);
            }
            Ok((scope_of(path, spec), wrapper))
        })();

        if path_safe(path) {
            let segments = path.split('/').collect::<Vec<_>>();
            if segments.len() >= 2 && !VALVE_ROOTS.contains(&segments[0]) {
                namespaces.insert(segments[0].to_string());
            }
        }
        match outcome {
            Err(reason) => {
                hostile |= reason.is_hostile();
                rejected.add(payload.len());
                if rejected_paths.len() < MAX_REPORTED_PATHS {
                    rejected_paths.push(RejectedPath {
                        path: entry.path.clone(),
                        reason,
                        bytes: payload.len() as u64,
                    });
                }
            }
            Ok((class, wrapper)) => {
                if wrapper {
                    image_wrappers += 1;
                }
                match class {
                    ScopeClass::HeroScoped => hero_scoped.add(payload.len()),
                    ScopeClass::AuthorOwned => author_owned.add(payload.len()),
                    ScopeClass::Shared => {
                        shared.add(payload.len());
                        if shared_paths.len() < MAX_REPORTED_PATHS {
                            shared_paths.push(SharedPath {
                                path: entry.path.clone(),
                                bytes: payload.len() as u64,
                            });
                        }
                    }
                }
                accepted.push(Accepted {
                    path: entry.path.clone(),
                    class,
                    payload,
                    digest,
                });
            }
        }
    }

    rejected_paths.sort_by(|a, b| a.path.cmp(&b.path));
    shared_paths.sort_by(|a, b| a.path.cmp(&b.path));
    let archive_identity = identity("betterfy-skin-archive-v1\n", &mut all);

    let strip = spec.shared_policy == SharedPolicy::Strip;
    let mut install_items = accepted
        .iter()
        .filter(|item| item.class != ScopeClass::Shared)
        .map(|item| (item.path.clone(), item.digest))
        .collect::<Vec<_>>();
    let install_bytes = hero_scoped.bytes + author_owned.bytes;
    let install_files = hero_scoped.files + author_owned.files;
    let blocked = if hostile {
        Some("skin_archive_hostile".to_string())
    } else if !strip && shared.files > 0 {
        Some("skin_shared_paths_refused".to_string())
    } else if install_files == 0 {
        Some("skin_nothing_to_install".to_string())
    } else if install_bytes > limits.max_install_bytes as u64 {
        Some("skin_install_too_large".to_string())
    } else {
        None
    };
    let install = InstallSummary {
        files: install_files,
        bytes: install_bytes,
        identity: identity("betterfy-skin-install-v1\n", &mut install_items),
        stripped_shared: if strip { shared.files } else { 0 },
        blocked,
    };
    Ok(Analysis {
        report: SkinReport {
            hero: spec.hero.clone(),
            shared_policy: spec.shared_policy,
            entries: entries.len(),
            total_bytes,
            archive_identity,
            hero_scoped,
            author_owned,
            shared,
            rejected,
            image_wrappers,
            rejected_paths,
            shared_paths,
            suggested_namespaces: namespaces.into_iter().collect(),
            hostile,
            install,
        },
        accepted,
        max_install_bytes: limits.max_install_bytes,
    })
}

impl Analysis<'_> {
    /// The files to install under the spec's [`SharedPolicy`], or the stable
    /// code of the rule that forbids installing this archive.
    pub(crate) fn into_selection(self) -> Result<Selection, String> {
        if let Some(code) = &self.report.install.blocked {
            return Err(code.clone());
        }
        let mut files = BTreeMap::new();
        let mut bytes = 0usize;
        for item in self.accepted {
            if item.class == ScopeClass::Shared {
                continue;
            }
            bytes += item.payload.len();
            if bytes > self.max_install_bytes {
                return Err("skin_install_too_large".to_string());
            }
            if files.insert(item.path, item.payload.into_owned()).is_some() {
                // Case-insensitive duplicates were rejected above, so an
                // exact repeat here would be a reader bug.
                return Err("skin_archive_invalid".to_string());
            }
        }
        Ok(Selection {
            files,
            report: self.report,
        })
    }
}

/// An archive plus a hero ID in, the files to install (and the report) out.
pub(crate) fn select_install_files(
    archive: &[u8],
    spec: &ScopeSpec,
    limits: &Limits,
) -> Result<Selection, String> {
    analyze(archive, spec, limits)?.into_selection()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vpk::{build, build_unchecked, VpkInput};

    fn spec(policy: SharedPolicy) -> ScopeSpec {
        ScopeSpec {
            hero: "keeper_of_the_light".to_string(),
            author_namespaces: vec!["darkness".to_string()],
            shared_policy: policy,
        }
    }

    fn wrapper_bytes(path: &str) -> Vec<u8> {
        crate::panorama_layout::test_image_wrapper(path, None)
    }

    /// The same shape as the audited sample: hero files, the author's own
    /// namespaces, shared overrides, and leftovers.
    fn skin_files() -> Vec<(String, Vec<u8>)> {
        let wrapper = "panorama/images/spellicons/keeper_of_the_light_recall.vxml_c";
        let bad_wrapper = "panorama/images/heroes/npc_dota_hero_keeper_of_the_light.vxml_c";
        let mut files = vec![
            // hero-scoped
            (
                "models/heroes/keeper_of_the_light/kotl_hood.vmdl_c",
                b"hood".to_vec(),
            ),
            (
                "particles/units/heroes/hero_keeper_of_the_light/attack.vpcf_c",
                b"attack".to_vec(),
            ),
            (
                "particles/units/heroes/keeper_of_the_light_radiant_bind_groundfx.vpcf_c",
                b"bind".to_vec(),
            ),
            (
                "particles/econ/items/keeper_of_the_light/ti7/body.vpcf_c",
                b"ti7".to_vec(),
            ),
            (
                "materials/models/heroes/keeper_of_the_light/keeper_body.vmat_c",
                b"body".to_vec(),
            ),
            (
                "materials/models/items/keeper_of_the_light/mount/mount.vmat_c",
                b"mount".to_vec(),
            ),
            (
                "panorama/images/heroes/npc_dota_hero_keeper_of_the_light_png.vtex_c",
                b"icon".to_vec(),
            ),
            (
                "panorama/images/spellicons/keeper_of_the_light_recall_png.vtex_c",
                b"spell".to_vec(),
            ),
            // author-owned: own top-level folders and declared prefixes
            ("darkness/materials/body.vmat_c", b"d1".to_vec()),
            ("kisilev_ind/darkness/particles/p.vpcf_c", b"k1".to_vec()),
            ("particles/darkness_snaps/snap1.vsnap_c", b"s1".to_vec()),
            ("particles/darkness/flame.vpcf_c", b"flame".to_vec()),
            (
                "particles/darkness_weaponka_snapshot1.vsnap_c",
                b"s2".to_vec(),
            ),
            // shared
            (
                "particles/basic_ambient/basic_ambient.vpcf_c",
                b"basic".to_vec(),
            ),
            (
                "particles/models/basic_trail/basic_trail.vpcf_c",
                b"trail".to_vec(),
            ),
            (
                "particles/status_fx/status_effect_keeper_spirit_form.vpcf_c",
                b"status".to_vec(),
            ),
            (
                "materials/default/default_color_tga_11bf3739.vtex_c",
                b"default".to_vec(),
            ),
            ("materials/particle/basic_glow.vtex_c", b"glow".to_vec()),
            (
                "materials/models/cubemaps/glossy_cube_tga_4cca90d9.vtex_c",
                b"cube".to_vec(),
            ),
            (
                "materials/models/particle/net_color.vmat_c",
                b"net".to_vec(),
            ),
            // another hero, and a namespace prefix without the separator
            ("models/heroes/axe/axe.vmdl_c", b"axe".to_vec()),
            (
                "particles/units/heroes/hero_axe/axe.vpcf_c",
                b"axefx".to_vec(),
            ),
            (
                "panorama/images/spellicons/axe_battle_hunger_png.vtex_c",
                b"axeicon".to_vec(),
            ),
            ("particles/darknessfoo/foo.vpcf_c", b"foo".to_vec()),
            // dropped
            ("particles/darkness/sparks.vpcf", b"source".to_vec()),
            ("materials/darkness/leftover.vmat", b"source".to_vec()),
        ]
        .into_iter()
        .map(|(path, bytes)| (path.to_string(), bytes))
        .collect::<Vec<_>>();
        files.push((wrapper.to_string(), wrapper_bytes(wrapper)));
        files.push((
            bad_wrapper.to_string(),
            crate::panorama_layout::test_image_wrapper(bad_wrapper, Some(("onactivate", "x()"))),
        ));
        files
    }

    fn archive(files: &[(String, Vec<u8>)]) -> Vec<u8> {
        build(
            files
                .iter()
                .map(|(path, bytes)| VpkInput { path, bytes })
                .collect(),
        )
        .expect("synthetic archive")
    }

    fn report_of(files: &[(String, Vec<u8>)]) -> SkinReport {
        let bytes = archive(files);
        analyze(&bytes, &spec(SharedPolicy::Strip), &Limits::default())
            .expect("analysis")
            .report
    }

    #[test]
    fn classifies_every_path_of_a_sample_shaped_archive() {
        let bytes = archive(&skin_files());
        let analysis =
            analyze(&bytes, &spec(SharedPolicy::Strip), &Limits::default()).expect("analysis");
        let report = &analysis.report;
        assert_eq!(report.entries, 28);
        // 8 hero files plus the proven image wrapper.
        assert_eq!(report.hero_scoped.files, 9);
        assert_eq!(report.image_wrappers, 1);
        assert_eq!(report.author_owned.files, 5);
        assert_eq!(report.shared.files, 11);
        assert_eq!(report.rejected.files, 3);
        assert_eq!(
            report.hero_scoped.files
                + report.author_owned.files
                + report.shared.files
                + report.rejected.files,
            report.entries
        );
        assert!(!report.hostile);
        assert_eq!(report.install.files, 14);
        assert_eq!(report.install.stripped_shared, 11);
        assert_eq!(report.install.blocked, None);

        let rejected = report
            .rejected_paths
            .iter()
            .map(|item| (item.path.as_str(), item.reason))
            .collect::<Vec<_>>();
        assert_eq!(
            rejected,
            vec![
                (
                    "materials/darkness/leftover.vmat",
                    RejectReason::UncompiledSource
                ),
                (
                    "panorama/images/heroes/npc_dota_hero_keeper_of_the_light.vxml_c",
                    RejectReason::PanoramaUnproven
                ),
                (
                    "particles/darkness/sparks.vpcf",
                    RejectReason::UncompiledSource
                ),
            ]
        );
        let shared = report
            .shared_paths
            .iter()
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>();
        for expected in [
            "particles/basic_ambient/basic_ambient.vpcf_c",
            "materials/default/default_color_tga_11bf3739.vtex_c",
            "materials/models/cubemaps/glossy_cube_tga_4cca90d9.vtex_c",
            "materials/models/particle/net_color.vmat_c",
            "particles/status_fx/status_effect_keeper_spirit_form.vpcf_c",
            "models/heroes/axe/axe.vmdl_c",
            "particles/units/heroes/hero_axe/axe.vpcf_c",
            "panorama/images/spellicons/axe_battle_hunger_png.vtex_c",
            "particles/darknessfoo/foo.vpcf_c",
        ] {
            assert!(shared.contains(&expected), "{expected} should be shared");
        }
        assert_eq!(report.suggested_namespaces, vec!["darkness", "kisilev_ind"]);

        let selection = analysis.into_selection().expect("selection");
        assert_eq!(selection.files.len(), 14);
        assert!(selection
            .files
            .contains_key("panorama/images/spellicons/keeper_of_the_light_recall.vxml_c"));
        assert!(selection
            .files
            .contains_key("particles/darkness_snaps/snap1.vsnap_c"));
        assert!(!selection
            .files
            .keys()
            .any(|path| path.contains("basic_") || path.contains("axe")));
        assert_eq!(
            selection.files["models/heroes/keeper_of_the_light/kotl_hood.vmdl_c"],
            b"hood"
        );
    }

    #[test]
    fn identities_are_deterministic_and_content_sensitive() {
        let mut files = skin_files();
        let first = report_of(&files);
        files.reverse();
        assert_eq!(first, report_of(&files));
        assert_eq!(first.archive_identity.len(), 64);
        assert_ne!(first.archive_identity, first.install.identity);

        // One changed byte in a shared file moves the archive identity but
        // not the install identity; in a hero file it moves both.
        let mut shared_edit = skin_files();
        shared_edit
            .iter_mut()
            .find(|(path, _)| path == "materials/particle/basic_glow.vtex_c")
            .expect("file")
            .1 = b"GLOW".to_vec();
        let shared_edit = report_of(&shared_edit);
        assert_ne!(shared_edit.archive_identity, first.archive_identity);
        assert_eq!(shared_edit.install.identity, first.install.identity);
        let mut hero_edit = skin_files();
        hero_edit
            .iter_mut()
            .find(|(path, _)| path == "models/heroes/keeper_of_the_light/kotl_hood.vmdl_c")
            .expect("file")
            .1 = b"HOOD".to_vec();
        let hero_edit = report_of(&hero_edit);
        assert_ne!(hero_edit.install.identity, first.install.identity);
    }

    #[test]
    fn refuse_policy_blocks_any_shared_path_but_still_reports() {
        let bytes = archive(&skin_files());
        let analysis =
            analyze(&bytes, &spec(SharedPolicy::Refuse), &Limits::default()).expect("analysis");
        assert_eq!(analysis.report.shared.files, 11);
        assert_eq!(analysis.report.shared_paths.len(), 11);
        assert_eq!(analysis.report.install.stripped_shared, 0);
        assert_eq!(
            analysis.report.install.blocked.as_deref(),
            Some("skin_shared_paths_refused")
        );
        assert_eq!(
            analysis.into_selection().err().as_deref(),
            Some("skin_shared_paths_refused")
        );

        // The same archive without shared files installs under either policy.
        let clean = skin_files()
            .into_iter()
            .filter(|(path, _)| {
                !(path.contains("basic_")
                    || path.contains("default")
                    || path.contains("cubemaps")
                    || path.contains("net_color")
                    || path.contains("status_fx")
                    || path.contains("heroes/axe")
                    || path.contains("hero_axe")
                    || path.contains("axe_")
                    || path.contains("darknessfoo"))
            })
            .collect::<Vec<_>>();
        let selection = select_install_files(
            &archive(&clean),
            &spec(SharedPolicy::Refuse),
            &Limits::default(),
        )
        .expect("clean archive installs under refuse");
        assert_eq!(selection.files.len(), 14);
    }

    #[test]
    fn hostile_entries_block_the_install_and_are_listed() {
        let bytes = build_unchecked(
            2,
            &[
                ("models/heroes/keeper_of_the_light/ok.vmdl_c", b"ok"),
                ("../escape.vtex_c", b"x"),
                ("panorama/scripts/evil.vjs_c", b"script"),
                ("darkness/tool.exe", b"MZ"),
                ("Particles/Upper.vpcf_c", b"upper"),
                ("darkness//double.vpcf_c", b"double"),
                ("darkness/a b.vpcf_c", b"space"),
            ],
        );
        let analysis = analyze(&bytes, &spec(SharedPolicy::Strip), &Limits::default())
            .expect("entries are reported, not fatal");
        assert!(analysis.report.hostile);
        let reasons = analysis
            .report
            .rejected_paths
            .iter()
            .map(|item| (item.path.as_str(), item.reason))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(reasons["../escape.vtex_c"], RejectReason::PathUnsafe);
        assert_eq!(reasons["panorama/scripts/evil.vjs_c"], RejectReason::Script);
        assert_eq!(reasons["darkness/tool.exe"], RejectReason::Executable);
        assert_eq!(reasons["Particles/Upper.vpcf_c"], RejectReason::PathUnsafe);
        assert_eq!(reasons["darkness//double.vpcf_c"], RejectReason::PathUnsafe);
        assert_eq!(reasons["darkness/a b.vpcf_c"], RejectReason::PathUnsafe);
        assert_eq!(analysis.report.hero_scoped.files, 1);
        assert_eq!(
            analysis.into_selection().err().as_deref(),
            Some("skin_archive_hostile")
        );
    }

    #[test]
    fn duplicate_paths_with_different_case_are_all_rejected() {
        let bytes = build_unchecked(
            1,
            &[
                ("models/heroes/keeper_of_the_light/a.vmdl_c", b"lower"),
                ("models/heroes/keeper_of_the_light/A.vmdl_c", b"UPPER"),
                ("models/heroes/keeper_of_the_light/same.vmdl_c", b"one"),
                ("models/heroes/keeper_of_the_light/same.vmdl_c", b"two"),
                ("models/heroes/keeper_of_the_light/fine.vmdl_c", b"fine"),
            ],
        );
        let analysis =
            analyze(&bytes, &spec(SharedPolicy::Strip), &Limits::default()).expect("analysis");
        assert_eq!(analysis.report.entries, 5);
        assert_eq!(analysis.report.hero_scoped.files, 1);
        assert_eq!(analysis.report.rejected.files, 4);
        assert!(analysis
            .report
            .rejected_paths
            .iter()
            .all(|item| item.reason == RejectReason::DuplicatePath));
        let selection = analysis.into_selection().expect("the unique file installs");
        assert_eq!(
            selection.files.keys().collect::<Vec<_>>(),
            vec!["models/heroes/keeper_of_the_light/fine.vmdl_c"]
        );
    }

    #[test]
    fn oversized_empty_and_unknown_entries_are_dropped() {
        let big = vec![7u8; 33];
        let bytes = build_unchecked(
            2,
            &[
                ("models/heroes/keeper_of_the_light/ok.vmdl_c", b"ok"),
                ("models/heroes/keeper_of_the_light/big.vmdl_c", &big),
                ("models/heroes/keeper_of_the_light/empty.vmdl_c", b""),
                ("darkness/notes.txt", b"readme"),
                ("darkness/compiled.vcss_c", b"css"),
            ],
        );
        let limits = Limits {
            max_entry_bytes: 32,
            max_install_bytes: 1024,
        };
        let analysis = analyze(&bytes, &spec(SharedPolicy::Strip), &limits).expect("analysis");
        let reasons = analysis
            .report
            .rejected_paths
            .iter()
            .map(|item| (item.path.as_str(), item.reason))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            reasons["models/heroes/keeper_of_the_light/big.vmdl_c"],
            RejectReason::Oversized
        );
        assert_eq!(
            reasons["models/heroes/keeper_of_the_light/empty.vmdl_c"],
            RejectReason::EmptyResource
        );
        assert_eq!(reasons["darkness/notes.txt"], RejectReason::TypeNotAllowed);
        assert_eq!(
            reasons["darkness/compiled.vcss_c"],
            RejectReason::TypeNotAllowed
        );
        assert!(!analysis.report.hostile);
        assert_eq!(analysis.report.install.files, 1);

        let small = Limits {
            max_entry_bytes: 32,
            max_install_bytes: 1,
        };
        assert_eq!(
            analyze(&bytes, &spec(SharedPolicy::Strip), &small)
                .expect("analysis")
                .into_selection()
                .err()
                .as_deref(),
            Some("skin_install_too_large")
        );
    }

    #[test]
    fn scope_is_exact_not_a_prefix_match() {
        let short = ScopeSpec {
            hero: "keeper".to_string(),
            author_namespaces: vec![],
            shared_policy: SharedPolicy::Strip,
        };
        for path in [
            "models/heroes/keeper_of_the_light/a.vmdl_c",
            "particles/units/heroes/hero_keeper_of_the_light/a.vpcf_c",
            "particles/econ/items/keeper_of_the_light/a/b.vpcf_c",
        ] {
            assert_eq!(scope_of(path, &short), ScopeClass::Shared, "{path}");
        }
        let full = spec(SharedPolicy::Strip);
        for (path, class) in [
            (
                "models/heroes/keeper_of_the_light/a.vmdl_c",
                ScopeClass::HeroScoped,
            ),
            (
                "models/heroes/keeper_of_the_light_x/a.vmdl_c",
                ScopeClass::Shared,
            ),
            (
                "models/items/keeper_of_the_light/a.vmdl_c",
                ScopeClass::Shared,
            ),
            (
                "particles/units/heroes/hero_keeper_of_the_light_x/a.vpcf_c",
                ScopeClass::Shared,
            ),
            (
                "particles/units/heroes/keeper_of_the_light.vpcf_c",
                ScopeClass::HeroScoped,
            ),
            (
                "particles/units/heroes/keeper_of_the_lightx.vpcf_c",
                ScopeClass::Shared,
            ),
            (
                "panorama/images/heroes/npc_dota_hero_keeper_of_the_light_png.vtex_c",
                ScopeClass::HeroScoped,
            ),
            (
                "panorama/images/heroes/npc_dota_hero_keeper_of_the_lighter_png.vtex_c",
                ScopeClass::Shared,
            ),
            (
                "panorama/images/heroes/deep/npc_dota_hero_keeper_of_the_light_png.vtex_c",
                ScopeClass::Shared,
            ),
            // a top-level folder that is not Valve's is the author's
            ("darkness/a.vpcf_c", ScopeClass::AuthorOwned),
            ("units/heroes/a.vpcf_c", ScopeClass::AuthorOwned),
            ("a.vpcf_c", ScopeClass::Shared),
            // declared prefixes count only below particles, materials, models
            ("particles/darkness_a/b.vpcf_c", ScopeClass::AuthorOwned),
            ("materials/darkness/b.vmat_c", ScopeClass::AuthorOwned),
            ("models/darkness_m/b.vmdl_c", ScopeClass::AuthorOwned),
            (
                "panorama/images/darkness_a/b_png.vtex_c",
                ScopeClass::Shared,
            ),
            ("particles/darknessx/b.vpcf_c", ScopeClass::Shared),
            (
                "particles/units/heroes/hero_axe/b.vpcf_c",
                ScopeClass::Shared,
            ),
        ] {
            assert_eq!(scope_of(path, &full), class, "{path}");
        }
    }

    #[test]
    fn declared_namespaces_cannot_claim_valve_folders_or_the_hero() {
        let make = |namespace: &str| ScopeSpec {
            hero: "keeper_of_the_light".to_string(),
            author_namespaces: vec![namespace.to_string()],
            shared_policy: SharedPolicy::Strip,
        };
        assert!(make("darkness").validate().is_ok());
        for bad in [
            "units",
            "econ",
            "materials",
            "particles",
            "heroes",
            "hero_keeper",
            "keeper_of_the_light",
            "abc",
            "Darkness",
            "dark-ness",
            "9lives",
            "",
        ] {
            assert_eq!(
                make(bad).validate().err().as_deref(),
                Some("skin_scope_invalid"),
                "{bad}"
            );
        }
        let duplicate = ScopeSpec {
            author_namespaces: vec!["darkness".to_string(), "darkness".to_string()],
            ..make("darkness")
        };
        assert!(duplicate.validate().is_err());
        let hero = ScopeSpec {
            hero: "Keeper".to_string(),
            ..make("darkness")
        };
        assert!(hero.validate().is_err());
    }

    #[test]
    fn container_failures_are_errors_not_reports() {
        let good = archive(&skin_files());
        assert!(analyze(&good, &spec(SharedPolicy::Strip), &Limits::default()).is_ok());
        assert_eq!(
            analyze(
                b"not a vpk at all",
                &spec(SharedPolicy::Strip),
                &Limits::default()
            )
            .err()
            .as_deref(),
            Some("vpk_invalid")
        );
        let mut corrupt = good.clone();
        let last = corrupt.len() - 1;
        corrupt[last] ^= 0xff;
        assert_eq!(
            analyze(&corrupt, &spec(SharedPolicy::Strip), &Limits::default())
                .err()
                .as_deref(),
            Some("vpk_crc_mismatch")
        );
        let mut external = good.clone();
        let index = external
            .windows(2)
            .position(|window| window == 0x7fffu16.to_le_bytes())
            .expect("archive index");
        external[index..index + 2].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            analyze(&external, &spec(SharedPolicy::Strip), &Limits::default())
                .err()
                .as_deref(),
            Some("vpk_archive_unsupported")
        );
        let mut version = good.clone();
        version[4..8].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            analyze(&version, &spec(SharedPolicy::Strip), &Limits::default())
                .err()
                .as_deref(),
            Some("vpk_version_unsupported")
        );
        let bad_spec = ScopeSpec {
            hero: "Bad Hero".to_string(),
            ..spec(SharedPolicy::Strip)
        };
        assert_eq!(
            analyze(&good, &bad_spec, &Limits::default())
                .err()
                .as_deref(),
            Some("skin_scope_invalid")
        );
    }

    #[test]
    fn reads_version_two_directories_too() {
        let bytes = build_unchecked(
            2,
            &[
                ("models/heroes/keeper_of_the_light/ok.vmdl_c", b"ok"),
                ("darkness/a.vpcf_c", b"fx"),
            ],
        );
        let selection =
            select_install_files(&bytes, &spec(SharedPolicy::Refuse), &Limits::default())
                .expect("version 2");
        assert_eq!(selection.files.len(), 2);
        assert_eq!(selection.report.entries, 2);
    }

    /// Local-only: analyzes the real sample archive named by the
    /// `BETTERFY_SKIN_SAMPLE` environment variable and prints the report.
    /// The archive is never part of the repository or of any fixture.
    #[test]
    fn analyzes_the_local_sample_archive_when_given() {
        let Ok(path) = std::env::var("BETTERFY_SKIN_SAMPLE") else {
            return;
        };
        let bytes = std::fs::read(&path).expect("sample archive");
        let spec = ScopeSpec {
            hero: "keeper_of_the_light".to_string(),
            author_namespaces: vec!["darkness".to_string()],
            shared_policy: SharedPolicy::Strip,
        };
        let analysis = analyze(&bytes, &spec, &Limits::default()).expect("analysis");
        println!(
            "{}",
            serde_json::to_string_pretty(&analysis.report).expect("report json")
        );
        assert!(!analysis.report.hostile);
        assert_eq!(analysis.report.install.blocked, None);
    }
}

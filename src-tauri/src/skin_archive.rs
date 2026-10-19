//! Third-party hero skin archives ("wardrobe" content).
//!
//! A skin arrives as a VPK whose directory holds every file. Nothing in it is
//! trusted. [`analyze`] reads the directory, classifies every path and turns
//! the archive plus an audited [`ScopeSpec`] into the list of files that may be
//! installed, with a typed report of everything that is not.
//!
//! Rules (see `docs/SKIN_ARCHIVE_AUDIT.md`):
//!
//! * Types. Only compiled resource types in [`ALLOWED_TYPES`] are accepted.
//!   Compiled Panorama scripts, executables and anything unknown are rejected.
//!   A `vxml_c` is accepted only when the layout reader proves it is an image
//!   wrapper (`panorama_layout::image_wrapper_dependency`); a layout with a
//!   script, an event handler or an include is rejected as scripted, and one
//!   the reader cannot prove is rejected as unproven.
//! * Empty files. A zero-length compiled resource is a deliberate blank
//!   override (VPK authors ship them to silence a model, a sound or a snapshot)
//!   and carries nothing to execute, so it is accepted like any other file of
//!   an allowed type. A zero-length layout proves nothing and is rejected.
//! * Ignored files. Uncompiled sources and tool or reading leftovers
//!   ([`RejectReason::UncompiledSource`], [`RejectReason::NotGameFile`]) are not
//!   something the game loads from a VPK, so leaving them out changes nothing
//!   in game. They are counted apart from the files that are really dropped.
//! * Paths. Upper-case letters are folded to lower case (the game looks files
//!   up case-insensitively and BetterFy's own VPK writer is lower-case only);
//!   every such rename is counted in the report, and two entries that are equal
//!   once folded are both rejected. Traversal, spaces, empty segments and any
//!   other odd byte are an unsafe path and block the install.
//! * Scope. Every accepted path is hero-scoped, shared or author-owned.
//!   Hero-scoped paths are the folders and files of the heroes the skin is for
//!   (a hero can have several internal names: `shadow_fiend` models and
//!   `nevermore` particles, so the audit names a set). Shared paths are the
//!   ones an audit found in Valve's own file list and that are not hero-scoped:
//!   the skin would replace a file every hero or the interface uses. The audit
//!   pins that list (`ScopeSpec::shared_paths`). Everything else is
//!   author-owned: the archive adds a file the game does not ship (its own
//!   folders, but also new particle or material files the skin's effects
//!   depend on), so it installs. Which class a path is in is decided by the
//!   pinned list, never by a name heuristic.
//! * Shared paths follow [`SharedPolicy`]. The default is `Install`: every
//!   accepted file installs, shared ones included, and the report counts and
//!   lists the shared files as information. That is what a person gets by
//!   dropping the skin's VPK into the game folder by hand, which is known to
//!   give the full look. `Strip` is an explicit opt-in that leaves the shared
//!   files out (and lists each), `Refuse` refuses an archive that carries any.
//!
//! What this does not prove: that the bytes of an accepted compiled file are
//! well formed. The type allowlist limits what the game is asked to load; it
//! is not a parser of every Source 2 resource. The archive is therefore
//! installable only when its hash is pinned (see `wardrobe.rs`).

use crate::panorama_layout::WrapperError;
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
    "vseq_c",
];
const LAYOUT_TYPE: &str = "vxml_c";

/// Uncompiled sources the game never loads from a VPK: Valve's own file list
/// (`pak01_dir.txt`, 2026-10-17, 388,116 entries) holds none of these
/// extensions, and the game loads only the compiled `_c` form. Their presence
/// is a leftover of the author's work tree.
const SOURCE_TYPES: &[&str] = &[
    "vpcf", "vmdl", "vmat", "vtex", "vmesh", "vanim", "vsnd", "vsndevts", "vxml", "vcss", "vts",
    "vmap", "vsnap", "vmorf", "vagrp", "vphys", "vseq", "fbx", "tga", "psd", "vsvg", "svg",
];
/// Raw images. Valve ships PNGs of its own under `resource/` (the legacy
/// interface, cursors), `materials/vgui/` and `panorama/` (a few spell icons),
/// so a PNG there may be a file the game loads; anywhere else (texture sources
/// under `materials/`, `models/`) it is a leftover of the author's work tree.
const RAW_IMAGE_TYPES: &[&str] = &["png", "jpg", "jpeg"];
const RAW_IMAGE_GAME_FOLDERS: &[&str] = &["resource/", "materials/vgui/", "panorama/"];
/// Tool, system and reading leftovers that no game loader reads (none of
/// these extensions is in Valve's file list). `txt` is not here: Valve ships
/// thousands of them (localisation, scripts), so only a readme counts, see
/// [`classify_type`].
const LEFTOVER_TYPES: &[&str] = &["db", "xml", "md", "log", "ini", "url", "bak", "tmp"];
const SCRIPT_TYPES: &[&str] = &["vjs_c", "vjs", "js", "jse", "lua", "py", "ts"];
const EXECUTABLE_TYPES: &[&str] = &[
    "exe", "dll", "bat", "cmd", "ps1", "com", "scr", "msi", "vbs", "vbe", "lnk", "sys", "drv",
    "ocx", "sh", "so", "dylib", "jar", "app",
];

const MAX_SEGMENT_BYTES: usize = 96;
const MAX_PATH_BYTES: usize = 240;
const MAX_REPORTED_PATHS: usize = 2_000;
/// Internal names a single skin may be scoped to.
pub(crate) const MAX_HEROES: usize = 8;
/// Upper bound on the audited shared list (a VPK holds at most 65,536 entries).
pub(crate) const MAX_SHARED_PATHS: usize = 65_536;

/// What a skin or pack is scoped to. `Hero` is a hero skin: its files are
/// split into the hero's own, shared and author-owned. `None` is every other
/// catalog category (a courier, a sound pack, a shader): there is no hero, every
/// accepted file installs, and the split into "overrides a Valve file" and
/// "new file" is information computed at audit time and pinned.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ScopeKind {
    Hero,
    None,
}

/// Which compiled file types a category may carry. All of them are subsets of
/// [`ALLOWED_TYPES`]; scripts, executables and unsafe paths stay rejected
/// whatever the policy.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TypePolicy {
    /// Models, particles, materials, textures, animations and sounds, and the
    /// image-wrapper layouts the layout reader proves (heroes and most others).
    Resources,
    /// Sound files and sound events under `sounds/` and `soundevents/`.
    Sounds,
    /// Panorama image textures (`vtex_c`) under `panorama/images/`.
    Textures,
    /// Materials and their textures (shaders).
    Materials,
}

impl TypePolicy {
    fn allows(self, extension: &str) -> bool {
        match self {
            Self::Resources => ALLOWED_TYPES.contains(&extension),
            Self::Sounds => matches!(extension, "vsnd_c" | "vsndevts_c"),
            Self::Textures => extension == "vtex_c",
            Self::Materials => matches!(extension, "vmat_c" | "vtex_c"),
        }
    }

    fn allows_layouts(self) -> bool {
        self == Self::Resources
    }

    /// Folders the category installs from; empty means anywhere.
    fn folders(self) -> &'static [&'static str] {
        match self {
            Self::Sounds => &["sounds/", "soundevents/"],
            Self::Textures => &["panorama/images/"],
            Self::Resources | Self::Materials => &[],
        }
    }
}

/// One catalog category the engine can install, and the rules for it.
pub(crate) struct CategoryRule {
    pub name: &'static str,
    pub scope: ScopeKind,
    pub types: TypePolicy,
}

const fn rule(name: &'static str, scope: ScopeKind, types: TypePolicy) -> CategoryRule {
    CategoryRule { name, scope, types }
}

/// Every category of the Dota2PornFx catalog that is installable. A category
/// that is not here is not enabled, whatever a manifest claims (see
/// `docs/CATALOG_CATEGORIES.md` for each category left out and why).
pub(crate) const CATEGORIES: &[CategoryRule] = &[
    rule("heroes", ScopeKind::Hero, TypePolicy::Resources),
    rule("hero-items", ScopeKind::Hero, TypePolicy::Resources),
    rule("ancient", ScopeKind::None, TypePolicy::Resources),
    rule("announcers", ScopeKind::None, TypePolicy::Sounds),
    rule("couriers", ScopeKind::None, TypePolicy::Resources),
    rule("creep-deny", ScopeKind::None, TypePolicy::Resources),
    rule("creeps", ScopeKind::None, TypePolicy::Resources),
    rule("emblems", ScopeKind::None, TypePolicy::Resources),
    rule("hero-sounds", ScopeKind::None, TypePolicy::Sounds),
    rule("herofx", ScopeKind::None, TypePolicy::Resources),
    rule("high-five", ScopeKind::None, TypePolicy::Resources),
    rule("item-effects", ScopeKind::None, TypePolicy::Resources),
    rule("item-icons", ScopeKind::None, TypePolicy::Textures),
    rule("mega-kill", ScopeKind::None, TypePolicy::Sounds),
    rule("music", ScopeKind::None, TypePolicy::Sounds),
    rule("optimization", ScopeKind::None, TypePolicy::Resources),
    rule("pings", ScopeKind::None, TypePolicy::Resources),
    rule("ranged-attack", ScopeKind::None, TypePolicy::Resources),
    rule("ranks", ScopeKind::None, TypePolicy::Textures),
    rule("river", ScopeKind::None, TypePolicy::Resources),
    rule("roshan", ScopeKind::None, TypePolicy::Resources),
    rule("shaders", ScopeKind::None, TypePolicy::Materials),
    rule("sounds", ScopeKind::None, TypePolicy::Sounds),
    rule("terrains", ScopeKind::None, TypePolicy::Resources),
    rule("ti-bp-effects", ScopeKind::None, TypePolicy::Resources),
    rule("towers", ScopeKind::None, TypePolicy::Resources),
    rule("trees", ScopeKind::None, TypePolicy::Resources),
    rule("wards", ScopeKind::None, TypePolicy::Resources),
];

pub(crate) fn category_rule(name: &str) -> Option<&'static CategoryRule> {
    CATEGORIES.iter().find(|rule| rule.name == name)
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SharedPolicy {
    /// Install every accepted file, shared ones included (the default).
    Install,
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
    /// Not a plain relative path (traversal, spaces, odd bytes).
    PathUnsafe,
    /// Two entries whose paths are equal when case is ignored.
    DuplicatePath,
    Oversized,
    /// A compiled or plain script.
    Script,
    /// Uncompiled source such as `.vpcf`, `.vmdl`, `.vmat`. Not a game file.
    UncompiledSource,
    /// A tool, system or reading leftover (a readme, `thumbs.db`, an
    /// `imageCompiler.xml`). Not a game file.
    NotGameFile,
    Executable,
    /// Any other type, including compiled types that are not allowed and
    /// files the game may load (videos, styles, `scripts/*.txt`). These are
    /// really dropped.
    TypeNotAllowed,
    /// A `vxml_c` that carries a script, an event handler or an include.
    PanoramaScripted,
    /// A `vxml_c` the layout reader could not prove to be an image wrapper.
    PanoramaUnproven,
    /// A file of an allowed type outside the folders its category installs
    /// from (a sound pack's `vtex_c`, a rank icon pack's `vmat_c` under
    /// `materials/`). Really dropped.
    OutsideCategory,
}

impl RejectReason {
    /// Reasons that mean the archive is not what an audit describes. They
    /// block the install instead of being dropped.
    pub(crate) fn is_hostile(self) -> bool {
        matches!(self, Self::PathUnsafe | Self::Script | Self::Executable)
    }

    /// Files that are not game files: leaving them out changes nothing in
    /// game, so they are counted apart from the files that are really dropped.
    pub(crate) fn is_ignored(self) -> bool {
        matches!(self, Self::UncompiledSource | Self::NotGameFile)
    }
}

/// The audited facts an archive is checked against.
#[derive(Clone, Debug)]
pub(crate) struct ScopeSpec {
    pub kind: ScopeKind,
    pub types: TypePolicy,
    /// Internal names of the heroes the skin is for (1 to [`MAX_HEROES`] for
    /// [`ScopeKind::Hero`], none for [`ScopeKind::None`]).
    pub heroes: Vec<String>,
    /// Sorted, unique, lower-case paths that exist in Valve's file list at
    /// audit time and are not hero-scoped. These are the shared files.
    pub shared_paths: Vec<String>,
    pub shared_policy: SharedPolicy,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Limits {
    pub max_entry_bytes: usize,
    pub max_install_bytes: usize,
}

/// Room kept for the directory of a merged VPK: 65,536 entries at no more than
/// 120 bytes each is under 8 MiB.
const VPK_DIRECTORY_RESERVE: usize = 8 * 1024 * 1024;

/// One skin may fill the merged VPK, less the room its directory needs. The
/// limit is the VPK's own (`vpk::MAX_VPK_BYTES`, the bound the merge, the
/// staged copy and the deployment all enforce), not a policy of its own: a
/// skin that fits can be installed alone, and a selection of several is
/// checked against the same bound when the build is prepared. The earlier
/// 16 MiB bound on one file dropped 4K textures and morph targets of 18 to
/// 22 MiB, and the 128 MiB bound on the install refused two skins whose VPKs
/// are 170 and 210 MiB.
const MAX_INSTALL_BYTES: usize = vpk::MAX_VPK_BYTES - VPK_DIRECTORY_RESERVE;

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entry_bytes: MAX_INSTALL_BYTES,
            max_install_bytes: MAX_INSTALL_BYTES,
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
    /// Shared files left out of the install: all of them under
    /// [`SharedPolicy::Strip`], none under `Install`.
    pub stripped_shared: usize,
    /// Stable code when the policy or the archive forbids installing.
    pub blocked: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkinReport {
    pub heroes: Vec<String>,
    pub shared_policy: SharedPolicy,
    pub entries: usize,
    pub total_bytes: u64,
    /// SHA-256 over the sorted path and payload hash of every entry, with the
    /// paths exactly as the archive spells them.
    pub archive_identity: String,
    pub hero_scoped: ClassStat,
    pub author_owned: ClassStat,
    pub shared: ClassStat,
    /// Files really left out: scripts, executables, layouts the reader could
    /// not prove, oversized and unsafe entries, and types the game may load
    /// that BetterFy does not install.
    pub rejected: ClassStat,
    /// Files that are not game files (uncompiled sources, tool and reading
    /// leftovers). Leaving them out changes nothing in game. Listed in
    /// `rejected_paths` with their reasons, counted apart from `rejected`.
    pub ignored: ClassStat,
    /// Accepted paths whose upper-case letters were folded to lower case.
    pub renamed: usize,
    /// `vxml_c` files proven to be image wrappers (counted in the classes).
    pub image_wrappers: usize,
    pub rejected_paths: Vec<RejectedPath>,
    /// The files that override a Valve file outside the heroes' own (the audit's
    /// shared paths found in the archive), whatever the policy does with them.
    pub shared_paths: Vec<SharedPath>,
    /// Author-owned files the player's own game ships although the audit did
    /// not find them in Valve's list (added since the audit). Informational;
    /// only filled in by [`Analysis::apply_game_check`].
    pub new_in_game: Vec<String>,
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
        let heroes_ok = match self.kind {
            ScopeKind::Hero => !self.heroes.is_empty() && self.heroes.len() <= MAX_HEROES,
            // Content with no hero installs whole: there is nothing to strip,
            // so no other policy exists for it.
            ScopeKind::None => {
                self.heroes.is_empty() && self.shared_policy == SharedPolicy::Install
            }
        };
        if !heroes_ok || self.shared_paths.len() > MAX_SHARED_PATHS {
            return Err(invalid());
        }
        let mut seen = BTreeSet::new();
        for hero in &self.heroes {
            if !token(hero, 2, 48) || !seen.insert(hero.as_str()) {
                return Err(invalid());
            }
        }
        // Strictly increasing: sorted and unique. A shared path is a safe
        // lower-case path and never one the heroes own, so that the audit
        // cannot strip the very files a skin exists to replace.
        for pair in self.shared_paths.windows(2) {
            if pair[0] >= pair[1] {
                return Err(invalid());
            }
        }
        for path in &self.shared_paths {
            if !path_safe(path) || hero_scoped(path, &self.heroes) {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

/// Whether a lower-case path is one the analyzer can ever accept: plain
/// segments, no traversal, no spaces, within the length bounds. Used by the
/// local generator.
#[cfg(test)]
pub(crate) fn path_is_safe(path: &str) -> bool {
    path_safe(path)
}

fn path_safe(path: &str) -> bool {
    path.len() <= MAX_PATH_BYTES
        && vpk::validate_path(path).is_ok()
        && path.split('/').all(|segment| {
            !segment.is_empty()
                && !segment.starts_with(' ')
                && !segment.ends_with(' ')
                && segment != "."
                && segment != ".."
                && segment.len() <= MAX_SEGMENT_BYTES
        })
}

/// A path that is merely oddly spelled: no traversal, no absolute or drive
/// path, no backslash, no control character, no empty segment, within the
/// length bounds. [`path_safe`] additionally limits the characters.
fn path_is_plain(path: &str) -> bool {
    path.len() <= MAX_PATH_BYTES
        && !path.starts_with('/')
        && !path.contains(['\\', ':'])
        && !path.chars().any(char::is_control)
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

/// Classifies a lower-case path by its type. Scripts and executables come
/// first, so no rule below can turn one of them into an ignored file.
fn classify_type(path: &str, types: TypePolicy) -> Result<Kind, RejectReason> {
    let extension = extension(path);
    let name = path.rsplit('/').next().unwrap_or(path);
    if types.allows(extension) {
        Ok(Kind::Compiled)
    } else if extension == LAYOUT_TYPE && types.allows_layouts() {
        Ok(Kind::Layout)
    } else if SCRIPT_TYPES.contains(&extension) {
        Err(RejectReason::Script)
    } else if EXECUTABLE_TYPES.contains(&extension) {
        Err(RejectReason::Executable)
    } else if SOURCE_TYPES.contains(&extension) {
        Err(RejectReason::UncompiledSource)
    } else if RAW_IMAGE_TYPES.contains(&extension) {
        if RAW_IMAGE_GAME_FOLDERS
            .iter()
            .any(|folder| path.starts_with(folder))
        {
            Err(RejectReason::TypeNotAllowed)
        } else {
            Err(RejectReason::UncompiledSource)
        }
    } else if LEFTOVER_TYPES.contains(&extension)
        || (extension == "txt" && (name.contains("readme") || leftover_text_folder(path, name)))
    {
        Err(RejectReason::NotGameFile)
    } else {
        Err(RejectReason::TypeNotAllowed)
    }
}

/// A `.txt` inside a folder of compiled resources. Valve's list (2026-10-17,
/// 388,116 entries) holds no text file under `particles/`, `materials/` or
/// `sounds/`, and under `models/` only five files named `portraits.txt`, so any
/// other text file there is a note the author left in the folder, not something
/// the game reads.
fn leftover_text_folder(path: &str, name: &str) -> bool {
    ["models/", "materials/", "particles/", "sounds/"]
        .iter()
        .any(|folder| path.starts_with(folder))
        && name != "portraits.txt"
}

/// `name` is `<stem>.<ext>`; true when the stem continues `prefix` with `_`
/// or ends it with the extension dot.
fn named_after(name: &str, prefix: &str) -> bool {
    name.strip_prefix(prefix)
        .is_some_and(|rest| rest.starts_with('_') || rest.starts_with('.'))
}

/// The hero name a folder-shaped Valve path belongs to, if it has one:
/// `models/heroes/<h>/..`, `models/items/<h>/..`,
/// `particles/units/heroes/hero_<h>/..`, `particles/econ/items/<h>/..` and
/// `materials/models/(heroes|items)/<h>/..`. `directories` excludes the file
/// name. The audit's hero inference and the scope rules use this one function,
/// so what is inferred is exactly what is later treated as hero-scoped.
pub(crate) fn hero_folder<'a>(directories: &[&'a str]) -> Option<&'a str> {
    match directories {
        ["models", "heroes", folder, ..]
        | ["models", "items", folder, ..]
        | ["particles", "econ", "items", folder, ..]
        | ["materials", "models", "heroes" | "items", folder, ..] => Some(*folder),
        ["particles", "units", "heroes", folder, ..] => {
            folder.strip_prefix("hero_").filter(|name| !name.is_empty())
        }
        _ => None,
    }
}

/// Folders Valve keeps per hero under sounds and interface images. They are
/// the hero's own files, but they are not used to infer which heroes a skin is
/// for (a voice pack would drown out every other signal).
fn hero_extra_folder<'a>(directories: &[&'a str]) -> Option<&'a str> {
    match directories {
        ["sounds", "vo", folder, ..]
        | ["sounds", "weapons", "hero", folder, ..]
        | ["sounds", "physics", "footsteps", "hero", folder, ..]
        | ["panorama", "images", "spellicons", folder, ..]
        | ["panorama", "images", "econ", "heroes", folder, ..] => Some(*folder),
        _ => None,
    }
}

/// Whether a lower-case path belongs to one of the heroes: their folders (see
/// [`hero_folder`] and the sound and image folders), the files Valve names
/// after the hero (particles next to the hero folders, the hero and spell
/// icons, the hero's sound events and status effects).
pub(crate) fn hero_scoped(path: &str, heroes: &[String]) -> bool {
    let segments = path.split('/').collect::<Vec<_>>();
    let name = segments[segments.len() - 1];
    let directories = &segments[..segments.len() - 1];
    if let Some(folder) = hero_folder(directories).or_else(|| hero_extra_folder(directories)) {
        return heroes.iter().any(|hero| hero == folder);
    }
    let named = |prefix: String| {
        heroes
            .iter()
            .any(|hero| named_after(name, &format!("{prefix}{hero}")))
    };
    match directories {
        // A few Valve particles sit next to the hero folders, named after the
        // hero (`keeper_of_the_light_radiant_bind_groundfx`).
        ["particles", "units", "heroes"] => named(String::new()),
        ["panorama", "images", "heroes"]
        | ["panorama", "images", "heroes", "selection" | "icons"] => {
            named("npc_dota_hero_".to_string())
        }
        ["panorama", "images", "spellicons"] => named(String::new()),
        ["soundevents", "voscripts"] => named("game_sounds_vo_".to_string()),
        ["soundevents", "game_sounds_heroes"] => named("game_sounds_".to_string()),
        ["particles", "status_fx"] => named("status_effect_".to_string()),
        _ => false,
    }
}

fn scope_of(path: &str, heroes: &[String], shared: &BTreeSet<&str>) -> ScopeClass {
    if hero_scoped(path, heroes) {
        ScopeClass::HeroScoped
    } else if shared.contains(path) {
        ScopeClass::Shared
    } else {
        ScopeClass::AuthorOwned
    }
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

struct InstallInputs {
    hero_scoped: ClassStat,
    author_owned: ClassStat,
    shared: ClassStat,
    hostile: bool,
    policy: SharedPolicy,
    max_install_bytes: usize,
}

/// Whether the policy leaves an accepted file of this class out of the install.
fn left_out(policy: SharedPolicy, class: ScopeClass) -> bool {
    class == ScopeClass::Shared && policy == SharedPolicy::Strip
}

/// What installing the accepted files amounts to under the policy, and whether
/// a rule forbids it.
fn install_summary(accepted: &[Accepted<'_>], inputs: &InstallInputs) -> InstallSummary {
    let strip = inputs.policy == SharedPolicy::Strip;
    let mut items = accepted
        .iter()
        .filter(|item| !left_out(inputs.policy, item.class))
        .map(|item| (item.path.clone(), item.digest))
        .collect::<Vec<_>>();
    let mut bytes = inputs.hero_scoped.bytes + inputs.author_owned.bytes;
    let mut files = inputs.hero_scoped.files + inputs.author_owned.files;
    if !strip {
        bytes += inputs.shared.bytes;
        files += inputs.shared.files;
    }
    let blocked = if inputs.hostile {
        Some("skin_archive_hostile".to_string())
    } else if inputs.policy == SharedPolicy::Refuse && inputs.shared.files > 0 {
        Some("skin_shared_paths_refused".to_string())
    } else if files == 0 {
        Some("skin_nothing_to_install".to_string())
    } else if bytes > inputs.max_install_bytes as u64 {
        Some("skin_install_too_large".to_string())
    } else {
        None
    };
    InstallSummary {
        files,
        bytes,
        identity: identity("betterfy-skin-install-v1\n", &mut items),
        stripped_shared: if strip { inputs.shared.files } else { 0 },
        blocked,
    }
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
    let shared_set = spec
        .shared_paths
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
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
    let mut hero_scoped_stat = ClassStat::default();
    let mut author_owned = ClassStat::default();
    let mut shared = ClassStat::default();
    let mut rejected = ClassStat::default();
    let mut ignored = ClassStat::default();
    let mut renamed = 0usize;
    let mut image_wrappers = 0usize;
    let mut hostile = false;
    let mut total_bytes = 0u64;

    for entry in &entries {
        let payload = entry.payload();
        let digest = sha256_of(&payload);
        total_bytes += payload.len() as u64;
        all.push((entry.path.clone(), digest));
        let lower = entry.path.to_ascii_lowercase();

        let outcome = (|| -> Result<(ScopeClass, bool), RejectReason> {
            if by_lowercase[&lower] > 1 {
                return Err(RejectReason::DuplicatePath);
            }
            if !path_safe(&lower) {
                // A name with characters the VPK writer does not take (an
                // `=`, brackets) is unsafe, because the file could not be
                // installed. When the file is a leftover that is never
                // installed anyway (a note, a source), the odd name alone is
                // not a reason to refuse the archive. Traversal, absolute and
                // control-character paths stay hostile whatever the type.
                if path_is_plain(&lower) {
                    if let Err(
                        reason @ (RejectReason::UncompiledSource | RejectReason::NotGameFile),
                    ) = classify_type(&lower, spec.types)
                    {
                        return Err(reason);
                    }
                }
                return Err(RejectReason::PathUnsafe);
            }
            let kind = classify_type(&lower, spec.types)?;
            let folders = spec.types.folders();
            if !folders.is_empty() && !folders.iter().any(|folder| lower.starts_with(folder)) {
                return Err(RejectReason::OutsideCategory);
            }
            if payload.len() > limits.max_entry_bytes {
                return Err(RejectReason::Oversized);
            }
            // A zero-length compiled resource is a deliberate blank override
            // and installs; a zero-length layout proves nothing and does not.
            let wrapper = matches!(kind, Kind::Layout);
            if wrapper {
                crate::panorama_layout::image_wrapper_dependency(&lower, &payload).map_err(
                    |error| match error {
                        WrapperError::Scripted => RejectReason::PanoramaScripted,
                        WrapperError::Unproven => RejectReason::PanoramaUnproven,
                    },
                )?;
            }
            Ok((scope_of(&lower, &spec.heroes, &shared_set), wrapper))
        })();

        match outcome {
            Err(reason) => {
                hostile |= reason.is_hostile();
                if reason.is_ignored() {
                    ignored.add(payload.len());
                } else {
                    rejected.add(payload.len());
                }
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
                if lower != entry.path {
                    renamed += 1;
                }
                match class {
                    ScopeClass::HeroScoped => hero_scoped_stat.add(payload.len()),
                    ScopeClass::AuthorOwned => author_owned.add(payload.len()),
                    ScopeClass::Shared => {
                        shared.add(payload.len());
                        if shared_paths.len() < MAX_REPORTED_PATHS {
                            shared_paths.push(SharedPath {
                                path: lower.clone(),
                                bytes: payload.len() as u64,
                            });
                        }
                    }
                }
                accepted.push(Accepted {
                    path: lower,
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

    let install = install_summary(
        &accepted,
        &InstallInputs {
            hero_scoped: hero_scoped_stat,
            author_owned,
            shared,
            hostile,
            policy: spec.shared_policy,
            max_install_bytes: limits.max_install_bytes,
        },
    );
    Ok(Analysis {
        report: SkinReport {
            heroes: spec.heroes.clone(),
            shared_policy: spec.shared_policy,
            entries: entries.len(),
            total_bytes,
            archive_identity,
            hero_scoped: hero_scoped_stat,
            author_owned,
            shared,
            rejected,
            ignored,
            renamed,
            image_wrappers,
            rejected_paths,
            shared_paths,
            new_in_game: Vec::new(),
            hostile,
            install,
        },
        accepted,
        max_install_bytes: limits.max_install_bytes,
    })
}

impl Analysis<'_> {
    /// The lower-case paths the analyzer accepted as author-owned: files the
    /// skin adds that the audit found nowhere in Valve's list. Used to check
    /// them against the game the person actually has.
    pub(crate) fn author_owned_paths(&self) -> Vec<String> {
        self.accepted
            .iter()
            .filter(|item| item.class == ScopeClass::AuthorOwned)
            .map(|item| item.path.clone())
            .collect()
    }

    /// Records which author-owned files the player's own game ships (`paths`,
    /// lower case): Valve added them since the audit, so they now replace a
    /// game file the audit never reviewed. Under `Install` this is information
    /// only (`report.new_in_game`). Under `Strip`, whose point is leaving the
    /// game's files untouched, they are also left out: they move to the shared
    /// class and the install summary is recomputed. Hero-scoped files are
    /// never touched: replacing the hero's own game files is what a skin is
    /// for.
    pub(crate) fn apply_game_check(&mut self, paths: &BTreeSet<String>) {
        let strip = self.report.shared_policy == SharedPolicy::Strip;
        let mut moved = false;
        for item in &mut self.accepted {
            if item.class != ScopeClass::AuthorOwned || !paths.contains(&item.path) {
                continue;
            }
            self.report.new_in_game.push(item.path.clone());
            if !strip {
                continue;
            }
            item.class = ScopeClass::Shared;
            let report = &mut self.report;
            report.author_owned.files -= 1;
            report.author_owned.bytes -= item.payload.len() as u64;
            report.shared.add(item.payload.len());
            if report.shared_paths.len() < MAX_REPORTED_PATHS {
                report.shared_paths.push(SharedPath {
                    path: item.path.clone(),
                    bytes: item.payload.len() as u64,
                });
            }
            moved = true;
        }
        self.report.new_in_game.sort();
        if !moved {
            return;
        }
        self.report.shared_paths.sort_by(|a, b| a.path.cmp(&b.path));
        self.report.install = install_summary(
            &self.accepted,
            &InstallInputs {
                hero_scoped: self.report.hero_scoped,
                author_owned: self.report.author_owned,
                shared: self.report.shared,
                hostile: self.report.hostile,
                policy: self.report.shared_policy,
                max_install_bytes: self.max_install_bytes,
            },
        );
    }

    /// The files to install under the spec's [`SharedPolicy`], or the stable
    /// code of the rule that forbids installing this archive.
    pub(crate) fn into_selection(self) -> Result<Selection, String> {
        if let Some(code) = &self.report.install.blocked {
            return Err(code.clone());
        }
        let mut files = BTreeMap::new();
        let mut bytes = 0usize;
        let policy = self.report.shared_policy;
        for item in self.accepted {
            if left_out(policy, item.class) {
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

/// An archive plus an audited scope in, the files to install (and the report)
/// out.
#[cfg(test)]
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
            kind: ScopeKind::Hero,
            types: TypePolicy::Resources,
            heroes: vec!["keeper_of_the_light".to_string()],
            shared_paths: sorted(&[
                "materials/default/default_color_tga_11bf3739.vtex_c",
                "materials/models/cubemaps/glossy_cube_tga_4cca90d9.vtex_c",
                "materials/models/particle/net_color.vmat_c",
                "materials/particle/basic_glow.vtex_c",
                "models/heroes/axe/axe.vmdl_c",
                "panorama/images/spellicons/axe_battle_hunger_png.vtex_c",
                "particles/basic_ambient/basic_ambient.vpcf_c",
                "particles/models/basic_trail/basic_trail.vpcf_c",
                "particles/status_fx/status_effect_keeper_spirit_form.vpcf_c",
                "particles/units/heroes/hero_axe/axe.vpcf_c",
            ]),
            shared_policy: policy,
        }
    }

    fn sorted(paths: &[&str]) -> Vec<String> {
        let mut paths = paths
            .iter()
            .map(|path| (*path).to_string())
            .collect::<Vec<_>>();
        paths.sort();
        paths
    }

    fn wrapper_bytes(path: &str) -> Vec<u8> {
        crate::panorama_layout::test_image_wrapper(path, None)
    }

    /// The same shape as the audited sample: hero files, the author's own
    /// folders, shared overrides that Valve ships, new files Valve does not
    /// ship, and leftovers.
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
                "models/items/keeper_of_the_light/mount/mount.vmdl_c",
                b"item".to_vec(),
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
            // author-owned: the author's own folders
            ("darkness/materials/body.vmat_c", b"d1".to_vec()),
            ("kisilev_ind/darkness/particles/p.vpcf_c", b"k1".to_vec()),
            ("particles/darkness_snaps/snap1.vsnap_c", b"s1".to_vec()),
            ("particles/darkness/flame.vpcf_c", b"flame".to_vec()),
            // author-owned: new files in Valve-looking folders that Valve does
            // not ship (the skin's effects depend on them)
            (
                "particles/basic_ember/basic_ember.vpcf_c",
                b"ember".to_vec(),
            ),
            (
                "materials/particle/basic_ember.vtex_c",
                b"embertex".to_vec(),
            ),
            (
                "models/heroes/keeper_of_the_light/anim.vseq_c",
                b"seq".to_vec(),
            ),
            // shared: Valve ships these and they are not the hero's
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
            // another hero, listed as shared by the audit
            ("models/heroes/axe/axe.vmdl_c", b"axe".to_vec()),
            (
                "particles/units/heroes/hero_axe/axe.vpcf_c",
                b"axefx".to_vec(),
            ),
            (
                "panorama/images/spellicons/axe_battle_hunger_png.vtex_c",
                b"axeicon".to_vec(),
            ),
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
        assert_eq!(report.entries, 30);
        // 9 hero folder and file paths (one of them an item model), the
        // sequence file and the proven image wrapper.
        assert_eq!(report.hero_scoped.files, 11);
        assert_eq!(report.image_wrappers, 1);
        // 4 files in the author's folders and the 2 new basic_ember files.
        assert_eq!(report.author_owned.files, 6);
        assert_eq!(report.shared.files, 10);
        // The unproven wrapper is really dropped; the two uncompiled sources
        // are not game files and are counted apart.
        assert_eq!(report.rejected.files, 1);
        assert_eq!(report.ignored.files, 2);
        assert_eq!(
            report.hero_scoped.files
                + report.author_owned.files
                + report.shared.files
                + report.rejected.files
                + report.ignored.files,
            report.entries
        );
        assert!(!report.hostile);
        assert_eq!(report.renamed, 0);
        assert_eq!(report.install.files, 17);
        assert_eq!(report.install.stripped_shared, 10);
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
                    RejectReason::PanoramaScripted
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
        assert_eq!(
            shared,
            spec(SharedPolicy::Strip)
                .shared_paths
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        );

        let selection = analysis.into_selection().expect("selection");
        assert_eq!(selection.files.len(), 17);
        assert!(selection
            .files
            .contains_key("panorama/images/spellicons/keeper_of_the_light_recall.vxml_c"));
        assert!(selection
            .files
            .contains_key("particles/darkness_snaps/snap1.vsnap_c"));
        assert!(selection
            .files
            .contains_key("models/items/keeper_of_the_light/mount/mount.vmdl_c"));
        assert!(selection
            .files
            .contains_key("models/heroes/keeper_of_the_light/anim.vseq_c"));
        assert!(!selection
            .files
            .keys()
            .any(|path| path.contains("basic_ambient") || path.contains("axe")));
        assert_eq!(
            selection.files["models/heroes/keeper_of_the_light/kotl_hood.vmdl_c"],
            b"hood"
        );
    }

    #[test]
    fn install_is_the_default_and_installs_every_accepted_file() {
        // The same archive under Install: classes are reported as before, but
        // nothing accepted is left out, shared files included (that is what
        // dropping the VPK into the game folder by hand does).
        let bytes = archive(&skin_files());
        let analysis =
            analyze(&bytes, &spec(SharedPolicy::Install), &Limits::default()).expect("analysis");
        let report = &analysis.report;
        assert_eq!(report.shared_policy, SharedPolicy::Install);
        assert_eq!(report.hero_scoped.files, 11);
        assert_eq!(report.author_owned.files, 6);
        assert_eq!(report.shared.files, 10);
        assert_eq!(report.rejected.files, 1);
        assert_eq!(report.ignored.files, 2);
        assert_eq!(report.shared_paths.len(), 10);
        assert_eq!(report.install.files, 27);
        assert_eq!(report.install.stripped_shared, 0);
        assert_eq!(report.install.blocked, None);
        assert_eq!(
            report.install.bytes,
            report.hero_scoped.bytes + report.author_owned.bytes + report.shared.bytes
        );
        let selection = analysis.into_selection().expect("selection");
        assert_eq!(selection.files.len(), 27);
        for shared in &spec(SharedPolicy::Install).shared_paths {
            assert!(selection.files.contains_key(shared), "{shared}");
        }
        // Still rejected, exactly as before: sources and unproven wrappers.
        assert!(!selection
            .files
            .contains_key("particles/darkness/sparks.vpcf"));
        assert!(!selection
            .files
            .contains_key("panorama/images/heroes/npc_dota_hero_keeper_of_the_light.vxml_c"));

        // Strip is the explicit opt-in; the two summaries differ and so do
        // their identities.
        let strip = analyze(&bytes, &spec(SharedPolicy::Strip), &Limits::default())
            .expect("analysis")
            .report;
        assert_eq!(strip.install.files, 17);
        assert_eq!(strip.install.stripped_shared, 10);
        assert_ne!(strip.install.identity, selection.report.install.identity);
        assert_eq!(strip.archive_identity, selection.report.archive_identity);
    }

    #[test]
    fn hostile_entries_still_block_an_install_policy_archive() {
        let bytes = build_unchecked(
            2,
            &[
                ("models/heroes/keeper_of_the_light/ok.vmdl_c", b"ok"),
                ("panorama/scripts/evil.vjs_c", b"script"),
                ("particles/basic_ambient/basic_ambient.vpcf_c", b"shared"),
            ],
        );
        let analysis =
            analyze(&bytes, &spec(SharedPolicy::Install), &Limits::default()).expect("analysis");
        assert!(analysis.report.hostile);
        assert_eq!(
            analysis.into_selection().err().as_deref(),
            Some("skin_archive_hostile")
        );
    }

    #[test]
    fn files_that_replace_nothing_install_even_in_shared_looking_folders() {
        // `particles/basic_ember` and `materials/particle/basic_ember` look
        // like Valve's `basic_*` folders but are not in the audited list, so
        // they are the skin's own and install. A path the audit listed in the
        // same folder is stripped.
        let selection = select_install_files(
            &archive(&skin_files()),
            &spec(SharedPolicy::Strip),
            &Limits::default(),
        )
        .expect("selection");
        assert_eq!(
            selection.files["particles/basic_ember/basic_ember.vpcf_c"],
            b"ember"
        );
        assert!(selection
            .files
            .contains_key("materials/particle/basic_ember.vtex_c"));
        assert!(!selection
            .files
            .contains_key("particles/basic_ambient/basic_ambient.vpcf_c"));
        assert!(!selection
            .files
            .contains_key("materials/particle/basic_glow.vtex_c"));
        // The class counts say the same.
        let report = selection.report;
        assert!(report
            .shared_paths
            .iter()
            .all(|item| !item.path.contains("basic_ember")));
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
    fn the_audited_list_decides_what_is_shared_not_the_file_name() {
        // The same archive audited with an empty list strips nothing: every
        // non-hero path is the author's.
        let none = ScopeSpec {
            shared_paths: Vec::new(),
            ..spec(SharedPolicy::Strip)
        };
        let report = analyze(&archive(&skin_files()), &none, &Limits::default())
            .expect("analysis")
            .report;
        assert_eq!(report.shared.files, 0);
        assert_eq!(report.install.stripped_shared, 0);
        assert_eq!(report.author_owned.files, 16);
        // A list entry that names a hero path is not a valid audit.
        let hero_entry = ScopeSpec {
            shared_paths: sorted(&["models/heroes/keeper_of_the_light/kotl_hood.vmdl_c"]),
            ..spec(SharedPolicy::Strip)
        };
        assert_eq!(
            analyze(&archive(&skin_files()), &hero_entry, &Limits::default())
                .err()
                .as_deref(),
            Some("skin_scope_invalid")
        );
    }

    #[test]
    fn refuse_policy_blocks_any_shared_path_but_still_reports() {
        let bytes = archive(&skin_files());
        let analysis =
            analyze(&bytes, &spec(SharedPolicy::Refuse), &Limits::default()).expect("analysis");
        assert_eq!(analysis.report.shared.files, 10);
        assert_eq!(analysis.report.shared_paths.len(), 10);
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
        let listed = spec(SharedPolicy::Refuse).shared_paths;
        let clean = skin_files()
            .into_iter()
            .filter(|(path, _)| !listed.contains(path))
            .collect::<Vec<_>>();
        let selection = select_install_files(
            &archive(&clean),
            &spec(SharedPolicy::Refuse),
            &Limits::default(),
        )
        .expect("clean archive installs under refuse");
        assert_eq!(selection.files.len(), 17);
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
                ("darkness//double.vpcf_c", b"double"),
                (" darkness/lead.vpcf_c", b"space"),
                ("darkness/caf\u{e9}.vpcf_c", b"nonascii"),
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
        assert_eq!(reasons["darkness//double.vpcf_c"], RejectReason::PathUnsafe);
        assert_eq!(reasons[" darkness/lead.vpcf_c"], RejectReason::PathUnsafe);
        assert_eq!(
            reasons["darkness/caf\u{e9}.vpcf_c"],
            RejectReason::PathUnsafe
        );
        assert_eq!(analysis.report.hero_scoped.files, 1);
        assert_eq!(
            analysis.into_selection().err().as_deref(),
            Some("skin_archive_hostile")
        );
    }

    #[test]
    fn mixed_case_paths_are_renamed_to_lower_case_and_counted() {
        let bytes = build_unchecked(
            2,
            &[
                ("models/heroes/keeper_of_the_light/ok.vmdl_c", b"ok"),
                ("Sounds/VO/Yoyoko/Hello.vsnd_c", b"voice"),
                ("Particles/Upper.vpcf_c", b"upper"),
                // Folding makes a Valve-listed path out of this one.
                ("Particles/Basic_Ambient/Basic_Ambient.vpcf_c", b"basic"),
            ],
        );
        let analysis = analyze(&bytes, &spec(SharedPolicy::Strip), &Limits::default())
            .expect("renames are not fatal");
        let report = &analysis.report;
        assert!(!report.hostile);
        assert_eq!(report.renamed, 3);
        assert_eq!(report.shared.files, 1);
        assert_eq!(
            report.shared_paths[0].path,
            "particles/basic_ambient/basic_ambient.vpcf_c"
        );
        assert_eq!(report.install.files, 3);
        // The archive identity covers the names exactly as the archive spells
        // them; the install identity covers the folded names.
        let lower = build_unchecked(
            2,
            &[
                ("models/heroes/keeper_of_the_light/ok.vmdl_c", b"ok"),
                ("sounds/vo/yoyoko/hello.vsnd_c", b"voice"),
                ("particles/upper.vpcf_c", b"upper"),
                ("particles/basic_ambient/basic_ambient.vpcf_c", b"basic"),
            ],
        );
        let lower = analyze(&lower, &spec(SharedPolicy::Strip), &Limits::default())
            .expect("analysis")
            .report;
        assert_ne!(lower.archive_identity, report.archive_identity);
        assert_eq!(lower.install.identity, report.install.identity);
        assert_eq!(lower.renamed, 0);
        let selection = analysis.into_selection().expect("selection");
        assert_eq!(
            selection.files.keys().cloned().collect::<Vec<_>>(),
            vec![
                "models/heroes/keeper_of_the_light/ok.vmdl_c",
                "particles/upper.vpcf_c",
                "sounds/vo/yoyoko/hello.vsnd_c",
            ]
        );
    }

    #[test]
    fn a_folded_name_that_collides_rejects_both_entries() {
        let bytes = build_unchecked(
            1,
            &[
                ("models/heroes/keeper_of_the_light/a.vmdl_c", b"lower"),
                ("models/heroes/keeper_of_the_light/A.vmdl_c", b"UPPER"),
                ("models/heroes/keeper_of_the_light/same.vmdl_c", b"one"),
                ("models/heroes/keeper_of_the_light/same.vmdl_c", b"two"),
                ("models/heroes/keeper_of_the_light/fine.vmdl_c", b"fine"),
                ("Darkness/Mixed.vpcf_c", b"upper-only"),
            ],
        );
        let analysis =
            analyze(&bytes, &spec(SharedPolicy::Strip), &Limits::default()).expect("analysis");
        assert_eq!(analysis.report.entries, 6);
        assert_eq!(analysis.report.hero_scoped.files, 1);
        assert_eq!(analysis.report.author_owned.files, 1);
        assert_eq!(analysis.report.renamed, 1);
        assert_eq!(analysis.report.rejected.files, 4);
        assert!(analysis
            .report
            .rejected_paths
            .iter()
            .all(|item| item.reason == RejectReason::DuplicatePath));
        let selection = analysis.into_selection().expect("the unique files install");
        assert_eq!(
            selection.files.keys().collect::<Vec<_>>(),
            vec![
                "darkness/mixed.vpcf_c",
                "models/heroes/keeper_of_the_light/fine.vmdl_c"
            ]
        );
    }

    #[test]
    fn oversized_entries_are_dropped_at_the_entry_bound() {
        let big = vec![7u8; 33];
        let bytes = build_unchecked(
            2,
            &[
                ("models/heroes/keeper_of_the_light/ok.vmdl_c", b"ok"),
                ("models/heroes/keeper_of_the_light/big.vmdl_c", &big),
            ],
        );
        let limits = Limits {
            max_entry_bytes: 32,
            max_install_bytes: 1024,
        };
        let analysis = analyze(&bytes, &spec(SharedPolicy::Strip), &limits).expect("analysis");
        assert_eq!(analysis.report.rejected_paths.len(), 1);
        assert_eq!(
            analysis.report.rejected_paths[0].reason,
            RejectReason::Oversized
        );
        assert_eq!(analysis.report.rejected.files, 1);
        assert_eq!(analysis.report.ignored.files, 0);
        assert!(!analysis.report.hostile);
        assert_eq!(analysis.report.install.files, 1);

        // The install bound still applies to the sum.
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
    fn the_default_entry_bound_is_the_install_bound() {
        // One 4K texture or morph target is 18 to 22 MiB; the old 16 MiB bound
        // dropped them. An entry may now be as large as the whole install.
        let limits = Limits::default();
        assert_eq!(limits.max_entry_bytes, limits.max_install_bytes);
        assert!(limits.max_entry_bytes >= 22 * 1024 * 1024);
        let texture = vec![3u8; 22 * 1024 * 1024];
        let bytes = build_unchecked(
            2,
            &[
                ("models/heroes/keeper_of_the_light/ok.vmdl_c", b"ok"),
                (
                    "materials/models/heroes/keeper_of_the_light/body_png_ab12.vtex_c",
                    &texture,
                ),
            ],
        );
        let selection = select_install_files(&bytes, &spec(SharedPolicy::Install), &limits)
            .expect("a 22 MiB texture installs");
        assert_eq!(selection.report.rejected.files, 0);
        assert_eq!(selection.files.len(), 2);
    }

    #[test]
    fn empty_compiled_resources_install_as_blank_overrides() {
        let bytes = build_unchecked(
            2,
            &[
                ("models/heroes/keeper_of_the_light/ok.vmdl_c", b"ok"),
                ("models/items/keeper_of_the_light/blank.vmdl_c", b""),
                (
                    "particles/models/items/keeper_of_the_light/hat/edge_fx.vsnap_c",
                    b"",
                ),
            ],
        );
        let analysis =
            analyze(&bytes, &spec(SharedPolicy::Install), &Limits::default()).expect("analysis");
        assert_eq!(analysis.report.rejected.files, 0);
        assert_eq!(analysis.report.hero_scoped.files, 2);
        assert_eq!(analysis.report.author_owned.files, 1);
        assert_eq!(analysis.report.install.files, 3);
        let selection = analysis.into_selection().expect("selection");
        assert_eq!(selection.files.len(), 3);
        assert!(selection.files["models/items/keeper_of_the_light/blank.vmdl_c"].is_empty());

        // The merged VPK keeps the zero-length entries (the game treats them
        // as blanks) and reads them back.
        let inputs = selection
            .files
            .iter()
            .map(|(path, bytes)| VpkInput { path, bytes })
            .collect::<Vec<_>>();
        let merged = build(inputs).expect("a zero-length entry is a valid VPK entry");
        let back = crate::vpk::extract_embedded(&merged).expect("reads back");
        assert_eq!(back, selection.files);
    }

    #[test]
    fn an_empty_file_is_not_a_way_around_the_type_rules() {
        let bytes = build_unchecked(
            2,
            &[
                ("models/heroes/keeper_of_the_light/ok.vmdl_c", b"ok"),
                // Empty script and executable stay hostile.
                ("panorama/scripts/empty.vjs_c", b""),
                ("darkness/empty.exe", b""),
                // An empty layout proves nothing.
                (
                    "panorama/images/spellicons/keeper_of_the_light_recall.vxml_c",
                    b"",
                ),
                // An empty file of a type that is not installed is still that type.
                ("darkness/blank.vcss_c", b""),
            ],
        );
        let analysis =
            analyze(&bytes, &spec(SharedPolicy::Install), &Limits::default()).expect("analysis");
        let reasons = analysis
            .report
            .rejected_paths
            .iter()
            .map(|item| (item.path.as_str(), item.reason))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            reasons["panorama/scripts/empty.vjs_c"],
            RejectReason::Script
        );
        assert_eq!(reasons["darkness/empty.exe"], RejectReason::Executable);
        assert_eq!(
            reasons["panorama/images/spellicons/keeper_of_the_light_recall.vxml_c"],
            RejectReason::PanoramaUnproven
        );
        assert_eq!(
            reasons["darkness/blank.vcss_c"],
            RejectReason::TypeNotAllowed
        );
        assert!(analysis.report.hostile);
    }

    #[test]
    fn files_the_game_does_not_load_are_ignored_and_counted_apart() {
        let bytes = build_unchecked(
            2,
            &[
                ("models/heroes/keeper_of_the_light/ok.vmdl_c", b"ok"),
                // Uncompiled sources, tool and reading leftovers.
                ("darkness/flame.vpcf", b"source"),
                ("models/heroes/keeper_of_the_light/hood.fbx", b"fbx"),
                ("darkness/README.txt", b"read me"),
                ("darkness/effects_readme.txt", b"read me"),
                ("README.txt", b""),
                ("panorama/images/spellicons/thumbs.db", b"db"),
                ("panorama/images/spellicons/imageCompiler.xml", b"<x/>"),
                // A PNG outside the folders where Valve ships its own is a
                // texture source.
                (
                    "materials/models/heroes/keeper_of_the_light/body.png",
                    b"png",
                ),
                // Types that are dropped for real, so they are not ignored:
                // media and styles the game may load, text the game reads,
                // PNGs in the folders Valve ships PNGs in.
                (
                    "panorama/videos/heroes/npc_dota_hero_keeper_of_the_light.webm",
                    b"v",
                ),
                ("panorama/styles/ui_econ_item.vcss_c", b"css"),
                ("scripts/npc/npc_abilities_custom.txt", b"kv"),
                ("resource/flash3/images/heroes/keeper.png", b"png"),
                ("panorama/images/spellicons/x.png", b"png"),
                (
                    "resource/flash3/videos/portraits/npc_dota_hero_keeper.usm",
                    b"usm",
                ),
            ],
        );
        let analysis =
            analyze(&bytes, &spec(SharedPolicy::Install), &Limits::default()).expect("analysis");
        let report = &analysis.report;
        let reason_of = |path: &str| {
            report
                .rejected_paths
                .iter()
                .find(|item| item.path == path)
                .unwrap_or_else(|| panic!("{path} is listed"))
                .reason
        };
        for path in [
            "darkness/flame.vpcf",
            "models/heroes/keeper_of_the_light/hood.fbx",
            "materials/models/heroes/keeper_of_the_light/body.png",
        ] {
            assert_eq!(reason_of(path), RejectReason::UncompiledSource, "{path}");
        }
        for path in [
            "darkness/README.txt",
            "darkness/effects_readme.txt",
            "README.txt",
            "panorama/images/spellicons/thumbs.db",
            "panorama/images/spellicons/imageCompiler.xml",
        ] {
            assert_eq!(reason_of(path), RejectReason::NotGameFile, "{path}");
        }
        for path in [
            "panorama/videos/heroes/npc_dota_hero_keeper_of_the_light.webm",
            "panorama/styles/ui_econ_item.vcss_c",
            "scripts/npc/npc_abilities_custom.txt",
            "resource/flash3/images/heroes/keeper.png",
            "panorama/images/spellicons/x.png",
            "resource/flash3/videos/portraits/npc_dota_hero_keeper.usm",
        ] {
            assert_eq!(reason_of(path), RejectReason::TypeNotAllowed, "{path}");
        }
        assert_eq!(report.ignored.files, 8);
        assert_eq!(report.rejected.files, 6);
        assert!(!report.hostile);
        assert_eq!(report.install.files, 1);
        assert!(RejectReason::UncompiledSource.is_ignored());
        assert!(RejectReason::NotGameFile.is_ignored());
        assert!(!RejectReason::TypeNotAllowed.is_ignored());
        assert!(!RejectReason::PanoramaScripted.is_ignored());
    }

    #[test]
    fn layouts_are_installed_when_proven_and_listed_with_the_exact_reason_when_not() {
        use crate::panorama_layout::{test_image_wrapper, test_text_wrapper};
        // The three container spellings seen in third-party archives, each
        // proven from its parsed structure.
        let png_named = "panorama/images/heroes/npc_dota_hero_keeper_of_the_light_png.vxml_c";
        let png_vtex = "panorama/images/heroes/npc_dota_hero_keeper_of_the_light_png.vtex";
        let xml = format!(
            "<root><Panel class=\"AddonLoadingRoot\"><Image id=\"gamemode\" class=\"SeqImg\" src=\"s2r://{png_vtex}\" /></Panel></root>"
        );
        let old_container =
            "panorama/images/heroes/selection/npc_dota_hero_keeper_of_the_light_png.vxml_c";
        let old_vtex =
            "panorama/images/heroes/selection/npc_dota_hero_keeper_of_the_light_png.vtex";
        let old_xml = xml.replace(png_vtex, old_vtex);
        let scripted_xml = xml.replace("<Image ", "<Image onactivate=\"x()\" ");
        let not_a_layout = "panorama/images/heroes/npc_dota_hero_keeper_of_the_light.vxml_c";
        let files = vec![
            (
                "models/heroes/keeper_of_the_light/ok.vmdl_c".to_string(),
                b"ok".to_vec(),
            ),
            (png_named.to_string(), test_image_wrapper(png_named, None)),
            (
                old_container.to_string(),
                test_text_wrapper(&old_xml, Some(&[old_vtex.to_string()])),
            ),
            (
                "panorama/images/spellicons/keeper_of_the_light_blink_png.vxml_c".to_string(),
                test_text_wrapper(&scripted_xml, Some(&[png_vtex.to_string()])),
            ),
            (not_a_layout.to_string(), b"not even a resource".to_vec()),
        ];
        let bytes = archive(&files);
        let analysis =
            analyze(&bytes, &spec(SharedPolicy::Install), &Limits::default()).expect("analysis");
        let report = &analysis.report;
        assert_eq!(report.image_wrappers, 2);
        assert_eq!(report.install.files, 3);
        assert!(!report.hostile);
        let reasons = report
            .rejected_paths
            .iter()
            .map(|item| (item.path.as_str(), item.reason))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            reasons["panorama/images/spellicons/keeper_of_the_light_blink_png.vxml_c"],
            RejectReason::PanoramaScripted
        );
        assert_eq!(reasons[not_a_layout], RejectReason::PanoramaUnproven);
        // A scripted layout is a drop, not an ignored file.
        assert_eq!(report.rejected.files, 2);
        assert_eq!(report.ignored.files, 0);
    }

    #[test]
    fn hero_scope_is_exact_not_a_prefix_match() {
        let heroes = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        let short = heroes(&["keeper"]);
        for path in [
            "models/heroes/keeper_of_the_light/a.vmdl_c",
            "models/items/keeper_of_the_light/a.vmdl_c",
            "particles/units/heroes/hero_keeper_of_the_light/a.vpcf_c",
            "particles/econ/items/keeper_of_the_light/a/b.vpcf_c",
        ] {
            assert!(!hero_scoped(path, &short), "{path}");
        }
        let full = heroes(&["keeper_of_the_light"]);
        for (path, scoped) in [
            ("models/heroes/keeper_of_the_light/a.vmdl_c", true),
            ("models/heroes/keeper_of_the_light_x/a.vmdl_c", false),
            // hero item models are the hero's too
            ("models/items/keeper_of_the_light/a/b.vmdl_c", true),
            ("models/items/keeper_of_the_light_x/a.vmdl_c", false),
            ("models/items/keeper_of_the_light.vmdl_c", false),
            (
                "particles/units/heroes/hero_keeper_of_the_light_x/a.vpcf_c",
                false,
            ),
            (
                "particles/units/heroes/hero_keeper_of_the_light/a.vpcf_c",
                true,
            ),
            ("particles/units/heroes/keeper_of_the_light.vpcf_c", true),
            ("particles/units/heroes/keeper_of_the_lightx.vpcf_c", false),
            (
                "panorama/images/heroes/npc_dota_hero_keeper_of_the_light_png.vtex_c",
                true,
            ),
            (
                "panorama/images/heroes/npc_dota_hero_keeper_of_the_lighter_png.vtex_c",
                false,
            ),
            (
                "panorama/images/heroes/deep/er/npc_dota_hero_keeper_of_the_light_png.vtex_c",
                false,
            ),
            // the hero's sounds, icons, sound events and status effects
            (
                "panorama/images/heroes/selection/npc_dota_hero_keeper_of_the_light_png.vtex_c",
                true,
            ),
            (
                "panorama/images/heroes/icons/npc_dota_hero_keeper_of_the_light_png.vtex_c",
                true,
            ),
            (
                "panorama/images/heroes/selection/npc_dota_hero_axe_png.vtex_c",
                false,
            ),
            (
                "panorama/images/spellicons/keeper_of_the_light/ti9/recall_png.vtex_c",
                true,
            ),
            (
                "panorama/images/spellicons/keeper/ti9/recall_png.vtex_c",
                false,
            ),
            (
                "panorama/images/econ/heroes/keeper_of_the_light/hood_png.vtex_c",
                true,
            ),
            ("sounds/vo/keeper_of_the_light/kotl_01.vsnd_c", true),
            ("sounds/vo/axe/axe_01.vsnd_c", false),
            ("sounds/weapons/hero/keeper_of_the_light/blind.vsnd_c", true),
            ("sounds/weapons/hero/shared/large_blade/hit.vsnd_c", false),
            (
                "sounds/physics/footsteps/hero/keeper_of_the_light/foley01.vsnd_c",
                true,
            ),
            (
                "soundevents/voscripts/game_sounds_vo_keeper_of_the_light.vsndevts_c",
                true,
            ),
            (
                "soundevents/voscripts/game_sounds_vo_keeper_of_the_lighter.vsndevts_c",
                false,
            ),
            (
                "soundevents/game_sounds_heroes/game_sounds_keeper_of_the_light.vsndevts_c",
                true,
            ),
            (
                "soundevents/game_sounds_heroes/game_sounds_axe.vsndevts_c",
                false,
            ),
            (
                "particles/status_fx/status_effect_keeper_of_the_light_x.vpcf_c",
                true,
            ),
            (
                "particles/status_fx/status_effect_keeper_spirit_form.vpcf_c",
                false,
            ),
            ("materials/models/heroes/keeper_of_the_light/a.vmat_c", true),
            (
                "materials/models/items/keeper_of_the_light/a/b.vmat_c",
                true,
            ),
            ("materials/models/items/other/a.vmat_c", false),
            ("darkness/a.vpcf_c", false),
            ("particles/units/heroes/hero_axe/b.vpcf_c", false),
            ("a.vpcf_c", false),
        ] {
            assert_eq!(hero_scoped(path, &full), scoped, "{path}");
        }
    }

    #[test]
    fn a_hero_with_several_internal_names_accepts_all_of_them() {
        // Shadow Fiend: models and items under `shadow_fiend`, particles and
        // spell icons under `nevermore`.
        let names = vec!["shadow_fiend".to_string(), "nevermore".to_string()];
        for path in [
            "models/heroes/shadow_fiend/head.vmdl_c",
            "models/items/shadow_fiend/arms/a.vmdl_c",
            "particles/units/heroes/hero_nevermore/shadowraze.vpcf_c",
            "particles/econ/items/nevermore/ti6/a.vpcf_c",
            "particles/units/heroes/nevermore_requiem.vpcf_c",
            "materials/models/heroes/nevermore/body.vmat_c",
            "panorama/images/spellicons/nevermore_necromastery_png.vtex_c",
            "panorama/images/heroes/npc_dota_hero_nevermore_png.vtex_c",
        ] {
            assert!(hero_scoped(path, &names), "{path}");
        }
        assert!(!hero_scoped("models/heroes/axe/a.vmdl_c", &names));
        assert!(!hero_scoped(
            "particles/units/heroes/hero_axe/a.vpcf_c",
            &names
        ));

        let spec = ScopeSpec {
            kind: ScopeKind::Hero,
            types: TypePolicy::Resources,
            heroes: names,
            shared_paths: sorted(&["models/heroes/axe/a.vmdl_c"]),
            shared_policy: SharedPolicy::Strip,
        };
        let bytes = build_unchecked(
            2,
            &[
                ("models/heroes/shadow_fiend/head.vmdl_c", b"head"),
                ("particles/units/heroes/hero_nevermore/raze.vpcf_c", b"raze"),
                ("models/heroes/axe/a.vmdl_c", b"axe"),
                ("effects/own.vpcf_c", b"own"),
            ],
        );
        let report = analyze(&bytes, &spec, &Limits::default())
            .expect("analysis")
            .report;
        assert_eq!(report.hero_scoped.files, 2);
        assert_eq!(report.shared.files, 1);
        assert_eq!(report.author_owned.files, 1);
        assert_eq!(report.heroes, vec!["shadow_fiend", "nevermore"]);
    }

    #[test]
    fn inference_and_scope_share_one_hero_rule() {
        assert_eq!(
            hero_folder(&["models", "heroes", "axe", "sub"]),
            Some("axe")
        );
        assert_eq!(hero_folder(&["models", "items", "axe"]), Some("axe"));
        assert_eq!(
            hero_folder(&["particles", "units", "heroes", "hero_axe"]),
            Some("axe")
        );
        assert_eq!(
            hero_folder(&["particles", "units", "heroes", "hero_"]),
            None
        );
        assert_eq!(hero_folder(&["particles", "units", "heroes", "axe"]), None);
        assert_eq!(
            hero_folder(&["particles", "econ", "items", "axe", "x"]),
            Some("axe")
        );
        assert_eq!(
            hero_folder(&["materials", "models", "items", "axe", "x"]),
            Some("axe")
        );
        assert_eq!(hero_folder(&["materials", "default"]), None);
        assert_eq!(hero_folder(&["models", "heroes"]), None);
    }

    #[test]
    fn scope_specs_must_be_well_formed() {
        let base = || spec(SharedPolicy::Strip);
        assert!(base().validate().is_ok());
        for bad in [
            ScopeSpec {
                kind: ScopeKind::Hero,
                types: TypePolicy::Resources,
                heroes: vec![],
                ..base()
            },
            ScopeSpec {
                kind: ScopeKind::Hero,
                types: TypePolicy::Resources,
                heroes: vec!["Keeper".to_string()],
                ..base()
            },
            ScopeSpec {
                kind: ScopeKind::Hero,
                types: TypePolicy::Resources,
                heroes: vec!["a".to_string()],
                ..base()
            },
            ScopeSpec {
                kind: ScopeKind::Hero,
                types: TypePolicy::Resources,
                heroes: vec!["axe".to_string(), "axe".to_string()],
                ..base()
            },
            ScopeSpec {
                kind: ScopeKind::Hero,
                types: TypePolicy::Resources,
                heroes: (0..=MAX_HEROES).map(|i| format!("hero{i}")).collect(),
                ..base()
            },
            // unsorted, duplicated, upper-case, unsafe or hero-owned entries
            ScopeSpec {
                shared_paths: vec!["b/b.vtex_c".to_string(), "a/a.vtex_c".to_string()],
                ..base()
            },
            ScopeSpec {
                shared_paths: vec!["a/a.vtex_c".to_string(), "a/a.vtex_c".to_string()],
                ..base()
            },
            ScopeSpec {
                shared_paths: vec!["A/a.vtex_c".to_string()],
                ..base()
            },
            ScopeSpec {
                shared_paths: vec!["../a.vtex_c".to_string()],
                ..base()
            },
            ScopeSpec {
                shared_paths: vec![" a/a.vtex_c".to_string()],
                ..base()
            },
            ScopeSpec {
                shared_paths: vec!["models/items/keeper_of_the_light/a.vmdl_c".to_string()],
                ..base()
            },
        ] {
            assert_eq!(
                bad.validate().err().as_deref(),
                Some("skin_scope_invalid"),
                "{bad:?}"
            );
        }
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
            kind: ScopeKind::Hero,
            types: TypePolicy::Resources,
            heroes: vec!["Bad Hero".to_string()],
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

    #[test]
    fn author_owned_paths_are_exposed_for_the_game_check() {
        let bytes = archive(&skin_files());
        let analysis =
            analyze(&bytes, &spec(SharedPolicy::Strip), &Limits::default()).expect("analysis");
        let owned = analysis.author_owned_paths();
        assert_eq!(owned.len(), analysis.report.author_owned.files);
        assert!(owned.contains(&"darkness/materials/body.vmat_c".to_string()));
        assert!(!owned.iter().any(|path| path.contains("kotl_hood")));
    }

    #[test]
    fn a_game_check_informs_under_install_and_leaves_files_out_under_strip() {
        let bytes = archive(&skin_files());
        // The game ships one of the skin's own files, one hero file and one
        // shared file.
        let game = BTreeSet::from([
            "particles/basic_ember/basic_ember.vpcf_c".to_string(),
            "models/heroes/keeper_of_the_light/kotl_hood.vmdl_c".to_string(),
            "particles/basic_ambient/basic_ambient.vpcf_c".to_string(),
        ]);

        let mut install =
            analyze(&bytes, &spec(SharedPolicy::Install), &Limits::default()).expect("analysis");
        let before = install.report.install.clone();
        install.apply_game_check(&game);
        assert_eq!(
            install.report.new_in_game,
            vec!["particles/basic_ember/basic_ember.vpcf_c"]
        );
        // Information only: nothing changes in the install.
        assert_eq!(install.report.install, before);
        assert_eq!(install.report.author_owned.files, 6);
        assert!(install
            .into_selection()
            .expect("selection")
            .files
            .contains_key("particles/basic_ember/basic_ember.vpcf_c"));

        let mut strip =
            analyze(&bytes, &spec(SharedPolicy::Strip), &Limits::default()).expect("analysis");
        let before = strip.report.install.clone();
        strip.apply_game_check(&game);
        assert_eq!(
            strip.report.new_in_game,
            vec!["particles/basic_ember/basic_ember.vpcf_c"]
        );
        assert_eq!(strip.report.install.files, before.files - 1);
        assert_eq!(strip.report.install.stripped_shared, 11);
        assert_eq!(strip.report.author_owned.files, 5);
        assert_eq!(strip.report.shared.files, 11);
        assert_ne!(strip.report.install.identity, before.identity);
        let selection = strip.into_selection().expect("selection");
        assert!(!selection
            .files
            .contains_key("particles/basic_ember/basic_ember.vpcf_c"));
        assert!(selection
            .files
            .contains_key("models/heroes/keeper_of_the_light/kotl_hood.vmdl_c"));
    }

    fn none_spec(types: TypePolicy, shared: &[&str]) -> ScopeSpec {
        ScopeSpec {
            kind: ScopeKind::None,
            types,
            heroes: Vec::new(),
            shared_paths: sorted(shared),
            shared_policy: SharedPolicy::Install,
        }
    }

    fn analyze_files(files: &[(&str, &[u8])], spec: &ScopeSpec) -> SkinReport {
        analyze(&build_unchecked(2, files), spec, &Limits::default())
            .expect("analysis")
            .report
    }

    fn reason_of(report: &SkinReport, path: &str) -> RejectReason {
        report
            .rejected_paths
            .iter()
            .find(|entry| entry.path == path)
            .unwrap_or_else(|| panic!("{path} was not rejected"))
            .reason
    }

    #[test]
    fn content_with_no_hero_installs_every_accepted_file_and_only_describes_the_overrides() {
        let spec = none_spec(
            TypePolicy::Resources,
            &[
                "models/props_gameplay/donkey.vmdl_c",
                "particles/ui/ping.vpcf_c",
            ],
        );
        let report = analyze_files(
            &[
                ("models/props_gameplay/donkey.vmdl_c", b"overrides"),
                ("particles/ui/ping.vpcf_c", b"overrides"),
                ("models/courier/new/body.vmdl_c", b"new"),
                // even a path that looks like a hero's folder: with no hero
                // it is just a file the archive carries
                ("models/heroes/axe/axe.vmdl_c", b"axe"),
                ("models/courier/new/note.txt", b"a note"),
            ],
            &spec,
        );
        assert_eq!(report.hero_scoped.files, 0);
        assert_eq!(report.shared.files, 2);
        assert_eq!(report.author_owned.files, 2);
        assert_eq!(report.install.files, 4);
        assert_eq!(report.install.stripped_shared, 0);
        assert!(report.install.blocked.is_none());
        assert_eq!(report.ignored.files, 1);
        assert_eq!(
            reason_of(&report, "models/courier/new/note.txt"),
            RejectReason::NotGameFile
        );
    }

    #[test]
    fn content_with_no_hero_has_no_hero_and_no_strip_mode() {
        assert!(none_spec(TypePolicy::Resources, &[]).validate().is_ok());
        for bad in [
            ScopeSpec {
                heroes: vec!["axe".to_string()],
                ..none_spec(TypePolicy::Resources, &[])
            },
            ScopeSpec {
                shared_policy: SharedPolicy::Strip,
                ..none_spec(TypePolicy::Resources, &[])
            },
            ScopeSpec {
                shared_policy: SharedPolicy::Refuse,
                ..none_spec(TypePolicy::Resources, &[])
            },
        ] {
            assert_eq!(bad.validate().err().as_deref(), Some("skin_scope_invalid"));
        }
        // a hero skin still needs its hero
        assert_eq!(
            ScopeSpec {
                kind: ScopeKind::Hero,
                heroes: Vec::new(),
                ..none_spec(TypePolicy::Resources, &[])
            }
            .validate()
            .err()
            .as_deref(),
            Some("skin_scope_invalid")
        );
    }

    #[test]
    fn each_category_installs_only_its_own_kind_of_file() {
        let files: &[(&str, &[u8])] = &[
            ("sounds/vo/announcer/kill.vsnd_c", b"sound"),
            (
                "soundevents/voscripts/game_sounds_vo_a.vsndevts_c",
                b"events",
            ),
            ("panorama/images/rank_tier_icons/rank1_psd.vtex_c", b"icon"),
            ("materials/dev/shader.vmat_c", b"material"),
            ("materials/dev/shader_color.vtex_c", b"texture"),
            ("models/courier/a.vmdl_c", b"model"),
            ("particles/ui/ping.vpcf_c", b"particle"),
        ];
        let installed = |types| {
            let report = analyze_files(files, &none_spec(types, &[]));
            let mut paths = report
                .rejected_paths
                .iter()
                .map(|entry| entry.path.clone())
                .collect::<Vec<_>>();
            paths.sort();
            (report.install.files, paths.len())
        };
        // resources: everything that is a compiled resource
        assert_eq!(installed(TypePolicy::Resources), (7, 0));
        // sounds: the two sound files under sounds/ and soundevents/
        assert_eq!(installed(TypePolicy::Sounds), (2, 5));
        // panorama textures: the one texture under panorama/images/
        assert_eq!(installed(TypePolicy::Textures), (1, 6));
        // shaders: materials and their textures, anywhere
        assert_eq!(installed(TypePolicy::Materials), (3, 4));

        let sounds = analyze_files(files, &none_spec(TypePolicy::Sounds, &[]));
        assert_eq!(
            reason_of(&sounds, "models/courier/a.vmdl_c"),
            RejectReason::TypeNotAllowed
        );
        // a texture of an allowed type in the wrong folder is not this category's
        let textures = analyze_files(
            &[
                ("panorama/images/ranks/a_png.vtex_c", b"ok"),
                ("materials/models/a.vtex_c", b"elsewhere"),
            ],
            &none_spec(TypePolicy::Textures, &[]),
        );
        assert_eq!(textures.install.files, 1);
        assert_eq!(
            reason_of(&textures, "materials/models/a.vtex_c"),
            RejectReason::OutsideCategory
        );
        assert!(!textures.hostile);
    }

    #[test]
    fn a_policy_never_lets_a_script_or_a_layout_through() {
        for types in [
            TypePolicy::Sounds,
            TypePolicy::Textures,
            TypePolicy::Materials,
            TypePolicy::Resources,
        ] {
            let report = analyze_files(
                &[
                    ("sounds/a.vsnd_c", b"ok"),
                    ("scripts/vscripts/a.lua", b"print()"),
                    ("panorama/scripts/a.vjs_c", b"js"),
                    ("sounds/helper.exe", b"MZ"),
                ],
                &none_spec(types, &[]),
            );
            assert!(report.hostile, "{types:?}");
            assert_eq!(
                report.install.blocked.as_deref(),
                Some("skin_archive_hostile"),
                "{types:?}"
            );
        }
        // layouts are read only for the policy that proves image wrappers
        let layout = ("panorama/images/items/bottle.vxml_c", &b"<root/>"[..]);
        let sounds = analyze_files(
            &[("sounds/a.vsnd_c", b"ok"), layout],
            &none_spec(TypePolicy::Sounds, &[]),
        );
        assert_eq!(
            reason_of(&sounds, "panorama/images/items/bottle.vxml_c"),
            RejectReason::TypeNotAllowed
        );
        let resources = analyze_files(
            &[("sounds/a.vsnd_c", b"ok"), layout],
            &none_spec(TypePolicy::Resources, &[]),
        );
        assert_eq!(
            reason_of(&resources, "panorama/images/items/bottle.vxml_c"),
            RejectReason::PanoramaUnproven
        );
    }

    #[test]
    fn a_note_in_a_resource_folder_is_ignored_and_an_odd_name_alone_is_not_hostile() {
        let report = analyze_files(
            &[
                ("models/courier/a.vmdl_c", b"model"),
                ("models/courier/not a game file = note.txt", b"note"),
                ("materials/models/courier/readme-1.txt", b"note"),
                ("particles/econ/courier/notes.txt", b"note"),
                // Valve ships this one under models/: a real game file
                ("models/items/pudge/portraits.txt", b"portraits"),
            ],
            &none_spec(TypePolicy::Resources, &[]),
        );
        assert_eq!(report.ignored.files, 3);
        assert_eq!(report.rejected.files, 1);
        assert_eq!(
            reason_of(&report, "models/items/pudge/portraits.txt"),
            RejectReason::TypeNotAllowed
        );
        assert_eq!(
            reason_of(&report, "models/courier/not a game file = note.txt"),
            RejectReason::NotGameFile
        );
        assert!(!report.hostile);
        assert!(report.install.blocked.is_none());

        // The same odd characters in a file the game would load stay unsafe,
        // as do traversal and a script or program with a plain name.
        for path in [
            "models/courier/odd (name).vmdl_c",
            "models/../escape.txt",
            "models/courier/ok.vmdl_c/../../x.txt",
            "scripts/odd = name.lua",
        ] {
            let report = analyze(
                &build_unchecked(2, &[(path, b"x")]),
                &none_spec(TypePolicy::Resources, &[]),
                &Limits::default(),
            )
            .map(|analysis| analysis.report);
            if let Ok(report) = report {
                assert!(report.hostile, "{path}");
            }
        }
    }

    #[test]
    fn the_category_table_fixes_scope_and_type_policy() {
        assert_eq!(
            category_rule("heroes").map(|r| r.scope),
            Some(ScopeKind::Hero)
        );
        assert_eq!(
            category_rule("hero-items").map(|r| r.scope),
            Some(ScopeKind::Hero)
        );
        assert_eq!(
            category_rule("mega-kill").map(|r| r.types),
            Some(TypePolicy::Sounds)
        );
        assert_eq!(
            category_rule("ranks").map(|r| r.types),
            Some(TypePolicy::Textures)
        );
        assert_eq!(
            category_rule("shaders").map(|r| r.types),
            Some(TypePolicy::Materials)
        );
        assert_eq!(
            category_rule("trees").map(|r| r.types),
            Some(TypePolicy::Resources)
        );
        assert!(category_rule("backgrounds").is_none());
        // every policy is a subset of the compiled types, plus the layouts of
        // the resources policy: nothing in a policy is outside the allowlist
        for extension in ["vsnd_c", "vsndevts_c", "vtex_c", "vmat_c"] {
            assert!(ALLOWED_TYPES.contains(&extension));
        }
    }
}

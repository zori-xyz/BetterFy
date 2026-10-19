//! Wardrobe items: third-party hero skins installed through the same
//! verified pipeline as the audited game-tuning packages.
//!
//! An item is installable only when it is in the embedded allowlist
//! (`src-tauri/wardrobe/manifests.json`, one generated JSON array compiled
//! into the binary; a signed catalog cannot extend it, like the audited
//! scripts in `package_registry.rs`). An entry pins:
//!
//! * the archive: the downloaded ZIP and the one VPK inside it, by size and
//!   SHA-256, and the HTTPS URLs they may come from (an ordered list of
//!   mirrors, each under a trusted prefix);
//! * the audit: the heroes the skin is for, the shared policy, the shared
//!   paths (the files the archive carries that Valve's own file list also
//!   has, at audit time), and what the analyzer found (entry count, archive
//!   identity, the install file count, size and identity). The analyzer is
//!   re-run on every build and must reproduce the audit exactly, so a change
//!   to its rules can never silently change which files an audited archive
//!   installs.
//!
//! The bytes are fetched once into the content store (the VPK, after the ZIP
//! is verified), read back and re-hashed on every build, and turned into
//! ordinary bundle resources by [`load`]. Backup, journal, deployment and
//! rollback are the existing transaction and are not touched here.
//!
//! `src-tauri/wardrobe/index.json` is the interface's copy of the same list
//! (names, authors, sizes, without the shared paths); a test requires it to be
//! exactly what the manifests derive.

use crate::content_store;
use crate::package_registry::LocalizedName;
use crate::skin_archive::{
    self, Limits, ScopeKind, ScopeSpec, SharedPolicy, SkinReport, TypePolicy,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::OnceLock;

/// One generated file of the allowlist: the manifests of some categories,
/// sorted by engine ID. The allowlist is split by kind of content so no single
/// file passes about a megabyte and a regeneration shows up as a readable diff;
/// the engine reads all of them as one registry.
pub(crate) struct Shard {
    pub file: &'static str,
    pub categories: &'static [&'static str],
    text: &'static str,
}

pub(crate) const SHARDS: &[Shard] = &[
    Shard {
        file: "manifests.json",
        categories: &["heroes", "hero-items"],
        text: include_str!("../wardrobe/manifests.json"),
    },
    Shard {
        file: "manifests-effects.json",
        categories: &[
            "item-effects",
            "ti-bp-effects",
            "ranged-attack",
            "herofx",
            "optimization",
            "emblems",
            "pings",
            "creep-deny",
            "high-five",
        ],
        text: include_str!("../wardrobe/manifests-effects.json"),
    },
    Shard {
        file: "manifests-world.json",
        categories: &[
            "couriers", "creeps", "roshan", "wards", "towers", "ancient", "river", "trees",
            "terrains", "shaders",
        ],
        text: include_str!("../wardrobe/manifests-world.json"),
    },
    Shard {
        file: "manifests-panorama.json",
        categories: &["backgrounds", "huds", "versus-screens"],
        text: include_str!("../wardrobe/manifests-panorama.json"),
    },
    Shard {
        file: "manifests-audio-ui.json",
        categories: &[
            "mega-kill",
            "music",
            "sounds",
            "hero-sounds",
            "announcers",
            "ranks",
            "item-icons",
        ],
        text: include_str!("../wardrobe/manifests-audio-ui.json"),
    },
];
#[cfg(test)]
const INDEX: &str = include_str!("../wardrobe/index.json");

/// Where archives are downloaded from, in the order a manifest lists them:
/// the Dota2PornFx project's GitHub mirror and its Hugging Face dataset, both
/// over HTTPS. The pinned hash decides whether the bytes are accepted; the
/// host only decides where to look.
pub(crate) const TRUSTED_ARCHIVE_PREFIXES: [&str; 2] = [
    "https://raw.githubusercontent.com/h6rd/Dota2PornFxWeb/main/assets/files/",
    "https://huggingface.co/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/",
];
/// Wardrobe IDs share the package ID shape (`namespace.name`) under their own
/// namespace, which `package_registry` reserves.
pub(crate) const NAMESPACE: &str = "wardrobe";
const SCHEMA_VERSION: u32 = 3;
const MAX_ARCHIVE_BYTES: u64 = content_store::MAX_ARTIFACT_BYTES;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WardrobeSource {
    /// Mirrors of the same ZIP, tried in order. Each must be under a trusted
    /// prefix and name the same file; a mirror that does not have the file or
    /// serves other bytes is skipped.
    pub urls: Vec<String>,
    pub catalog_snapshot: String,
    /// How permission to redistribute was established. Recorded, never
    /// inferred: `founder_reported_author_consent` means the project's founder
    /// reports the authors agreed; no licence file ships in the archive.
    pub permission: String,
}

/// What the downloaded file is. `Zip` is a ZIP holding exactly one
/// `pakNN_dir.vpk`; `Vpk` is a raw `pakNN_dir.vpk` served as the catalog item
/// itself (no ZIP at all), pinned by its own size and SHA-256.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Zip,
    Vpk,
}

impl SourceKind {
    /// The extension of the URL's last segment for this kind of source.
    fn extension(self) -> &'static str {
        match self {
            Self::Zip => ".zip",
            Self::Vpk => ".vpk",
        }
    }
}

fn is_zero(value: &u64) -> bool {
    *value == 0
}

fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}

/// The pinned bytes. For a ZIP: the ZIP (`zip_*`) and the one VPK inside it
/// (`inner_*`). For a raw VPK there is no ZIP: only `inner_*` is present and it
/// is also the download.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchivePin {
    pub kind: SourceKind,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub zip_bytes: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub zip_sha256: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub inner_path: String,
    pub inner_bytes: usize,
    pub inner_sha256: String,
}

impl ArchivePin {
    /// Size of the file that is downloaded.
    pub(crate) fn download_bytes(&self) -> u64 {
        match self.kind {
            SourceKind::Zip => self.zip_bytes,
            SourceKind::Vpk => self.inner_bytes as u64,
        }
    }

    /// SHA-256 of the file that is downloaded.
    pub(crate) fn download_sha256(&self) -> &str {
        match self.kind {
            SourceKind::Zip => &self.zip_sha256,
            SourceKind::Vpk => &self.inner_sha256,
        }
    }
}

/// What the analyzer found when a person audited this archive. Both ways of
/// installing it are pinned: `install_*` is everything the analyzer accepts
/// (the default, [`SharedPolicy::Install`]); `strip_*` is the same without the
/// `shared` files that override a Valve file outside the heroes' own (the
/// explicit opt-in, [`SharedPolicy::Strip`]). The analyzer is re-run on every
/// build for the mode asked for and must reproduce that mode's numbers.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditedResult {
    pub entries: usize,
    pub archive_identity: String,
    /// Files really left out (see `SkinReport::rejected`).
    pub rejected: usize,
    /// Files that are not game files and are left out (`SkinReport::ignored`).
    pub ignored: usize,
    /// Accepted files that override a Valve file outside the heroes' own.
    pub shared: usize,
    pub install_files: usize,
    pub install_bytes: u64,
    pub install_identity: String,
    /// The opt-in `strip` result. Pinned for hero skins only: content with no
    /// hero installs whole and has no such mode (these stay zero and empty).
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub strip_files: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub strip_bytes: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub strip_identity: String,
    /// Accepted paths whose upper-case letters were folded to lower case.
    pub renamed: usize,
    /// SHA-256 and entry count of Valve's file list the shared paths were
    /// computed against (SteamDatabase `game/dota/pak01_dir.txt`).
    pub valve_list_sha256: String,
    pub valve_list_entries: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WardrobeManifest {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub catalog_id: String,
    /// The catalog category (`heroes`, `couriers`, `mega-kill`, ...); it fixes
    /// the scope kind and the type policy (`skin_archive::CATEGORIES`).
    pub category: String,
    /// `hero` for a hero skin, `none` for every other category.
    pub scope: ScopeKind,
    pub name: LocalizedName,
    pub author: String,
    /// Internal names of the heroes the skin is for (none for scope `none`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heroes: Vec<String>,
    /// Sorted lower-case paths in the archive that Valve ships and that are
    /// not the heroes': the files the opt-in `strip` mode leaves out.
    pub shared_paths: Vec<String>,
    /// Sorted lower-case Valve paths the accepted Panorama layouts and styles
    /// point at (Panorama categories only; see `ScopeSpec::valve_references`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub valve_references: Vec<String>,
    pub distribution: String,
    pub source: WardrobeSource,
    pub archive: ArchivePin,
    pub audited: AuditedResult,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verified_languages: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_note: Option<String>,
}

impl WardrobeManifest {
    pub(crate) fn scope_spec(&self, policy: SharedPolicy) -> ScopeSpec {
        ScopeSpec {
            kind: self.scope,
            types: skin_archive::category_rule(&self.category)
                .map_or(TypePolicy::Resources, |rule| rule.types),
            heroes: self.heroes.clone(),
            shared_paths: self.shared_paths.clone(),
            valve_references: self.valve_references.clone(),
            shared_policy: policy,
        }
    }
}

/// The engine ID suffix that selects the opt-in `strip` mode of an item:
/// `wardrobe.<name>.strip`. It is a separate package ID on purpose, so the
/// mode travels everywhere the package list already travels (plan, journal,
/// ownership, rebuilds) and is part of the plan identity with no new field.
pub(crate) const STRIP_SUFFIX: &str = ".strip";

/// What an item is installed as: the default (every accepted file) or the
/// opt-in that leaves the shared game files untouched.
pub(crate) fn parse_variant(id: &str) -> (&str, SharedPolicy) {
    match id.strip_suffix(STRIP_SUFFIX) {
        Some(base) => (base, SharedPolicy::Strip),
        None => (id, SharedPolicy::Install),
    }
}

pub(crate) fn variant_id(manifest: &WardrobeManifest, policy: SharedPolicy) -> String {
    match policy {
        SharedPolicy::Strip => format!("{}{STRIP_SUFFIX}", manifest.id),
        _ => manifest.id.clone(),
    }
}

impl AuditedResult {
    /// Files, bytes and identity pinned for a mode. `Refuse` is not an
    /// installable mode of an item.
    pub(crate) fn pin(&self, policy: SharedPolicy) -> Option<(usize, u64, &str)> {
        match policy {
            SharedPolicy::Install => Some((
                self.install_files,
                self.install_bytes,
                &self.install_identity,
            )),
            // Content with no hero has no strip result pinned.
            SharedPolicy::Strip if self.strip_identity.is_empty() => None,
            SharedPolicy::Strip => Some((self.strip_files, self.strip_bytes, &self.strip_identity)),
            SharedPolicy::Refuse => None,
        }
    }
}

/// The interface's copy of one manifest (`index.json`): what a card, a build
/// summary and the size warning need, without the shared paths.
#[cfg(test)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IndexEntry {
    pub id: String,
    pub catalog_id: String,
    pub kind: String,
    pub category: String,
    pub scope: ScopeKind,
    pub name: LocalizedName,
    pub author: String,
    pub heroes: Vec<String>,
    /// Size of the file that is downloaded (the ZIP, or the raw VPK).
    pub download_bytes: u64,
    /// Everything the analyzer accepts (the default mode).
    pub install_files: usize,
    pub install_bytes: u64,
    /// The same without the files that override a shared Valve file (hero
    /// skins only; zero for content with no hero).
    pub strip_bytes: u64,
    /// How many accepted files override a shared Valve file.
    pub shared_files: usize,
}

#[cfg(test)]
pub(crate) fn index_entry(manifest: &WardrobeManifest) -> IndexEntry {
    IndexEntry {
        id: manifest.id.clone(),
        catalog_id: manifest.catalog_id.clone(),
        kind: manifest.kind.clone(),
        category: manifest.category.clone(),
        scope: manifest.scope,
        name: manifest.name.clone(),
        author: manifest.author.clone(),
        heroes: manifest.heroes.clone(),
        download_bytes: manifest.archive.download_bytes(),
        install_files: manifest.audited.install_files,
        install_bytes: manifest.audited.install_bytes,
        strip_bytes: manifest.audited.strip_bytes,
        shared_files: manifest.audited.shared,
    }
}

/// One wardrobe item in a reviewed build plan: what the analyzer found and
/// what the install will contain.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WardrobePlanItem {
    /// The engine ID of this selection: the item's, with `.strip` when the
    /// person asked to keep the game's shared files untouched.
    pub package_id: String,
    pub catalog_id: String,
    /// The catalog category; the review shows its help text.
    pub category: String,
    /// `hero` for a skin, `none` for every other category.
    pub scope: ScopeKind,
    pub heroes: Vec<String>,
    /// The mirror list in the order it is tried.
    pub source_urls: Vec<String>,
    pub archive_sha256: String,
    /// Where the catalog listed it, and how permission to use it was
    /// established. Shown as recorded, never upgraded.
    pub catalog_snapshot: String,
    pub permission: String,
    pub verified_languages: Vec<String>,
    pub compatibility_note: Option<String>,
    /// `install` (default) or `strip`: whether the shared game files the
    /// archive overrides are part of the build.
    pub shared_policy: SharedPolicy,
    /// Whether the skin's files were also compared with the player's own
    /// `pak01_dir.vpk` when the build was prepared.
    pub game_checked: bool,
    pub report: SkinReport,
}

/// A path a wardrobe item writes that another selected package also writes
/// with different bytes. The first package in the selection wins; the plan
/// shows every such path and the install needs the person's acknowledgement.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WardrobeConflict {
    pub path: String,
    pub winner_package_id: String,
    pub shadowed_package_ids: Vec<String>,
}

pub(crate) struct LoadedWardrobe {
    pub resources: BTreeMap<String, Vec<u8>>,
    pub item: WardrobePlanItem,
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

/// Catalog IDs are the upstream's: lower-case words joined by hyphens. One
/// upstream ID ends in a hyphen (`shaders-diretide-outline-`), so unlike an
/// engine name a catalog ID may.
fn catalog_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value.starts_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// `<category>/<file>` of a URL under one of the trusted prefixes whose file
/// ends in `extension` (`.zip` or `.vpk`): exactly two percent-encoded path
/// segments, no query, no fragment, no traversal.
pub(crate) fn archive_tail<'a>(url: &'a str, extension: &str) -> Option<&'a str> {
    let rest = TRUSTED_ARCHIVE_PREFIXES
        .iter()
        .find_map(|prefix| url.strip_prefix(prefix))?;
    let segments = rest.split('/').collect::<Vec<_>>();
    (rest.ends_with(extension)
        && rest.len() <= 160
        && segments.len() == 2
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && *segment != "."
                && *segment != ".."
                && segment.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'%' | b'.' | b'_' | b'-' | b'~')
                })
        }))
    .then_some(rest)
}

fn valid_archive_urls(urls: &[String], kind: SourceKind) -> bool {
    if urls.is_empty() || urls.len() > TRUSTED_ARCHIVE_PREFIXES.len() {
        return false;
    }
    let extension = kind.extension();
    let Some(first) = archive_tail(&urls[0], extension) else {
        return false;
    };
    // Every mirror names the same file, and no mirror is listed twice.
    let mut prefixes = BTreeSet::new();
    urls.iter().all(|url| {
        archive_tail(url, extension) == Some(first)
            && TRUSTED_ARCHIVE_PREFIXES
                .iter()
                .find(|prefix| url.starts_with(**prefix))
                .is_some_and(|prefix| prefixes.insert(*prefix))
    })
}

pub(crate) fn validate(manifest: &WardrobeManifest) -> Result<(), String> {
    let invalid = |reason: &str| {
        Err(format!(
            "wardrobe_manifest_invalid:{}:{reason}",
            manifest.id
        ))
    };
    if manifest.schema_version != SCHEMA_VERSION || manifest.kind != "wardrobe" {
        return invalid("schema");
    }
    if manifest.distribution != "internal_pilot" {
        return invalid("distribution");
    }
    // The category fixes the scope and the type policy; a manifest cannot
    // claim a looser one.
    let Some(rule) = skin_archive::category_rule(&manifest.category) else {
        return invalid("category");
    };
    if manifest.scope != rule.scope {
        return invalid("scope");
    }
    let name = manifest.id.strip_prefix("wardrobe.");
    if !name.is_some_and(slug)
        || !catalog_slug(&manifest.catalog_id)
        || !manifest.catalog_id.contains('-')
        || !manifest
            .catalog_id
            .strip_prefix(manifest.category.as_str())
            .is_some_and(|rest| rest.starts_with('-') && rest.len() > 1)
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
    if manifest
        .scope_spec(SharedPolicy::Install)
        .validate()
        .is_err()
    {
        return invalid("scope");
    }
    if !valid_archive_urls(&manifest.source.urls, manifest.archive.kind) {
        return invalid("source");
    }
    let pin = &manifest.archive;
    let download_ok = match pin.kind {
        SourceKind::Zip => {
            pin.zip_bytes > 0
                && pin.zip_bytes <= MAX_ARCHIVE_BYTES
                && valid_sha256(&pin.zip_sha256)
                && !pin.inner_path.is_empty()
                && pin.inner_path.len() <= 64
                && pin.inner_path.ends_with("_dir.vpk")
                && pin.inner_path.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'_' | b'.')
                })
        }
        // The raw VPK is the download: nothing else is pinned.
        SourceKind::Vpk => {
            pin.zip_bytes == 0 && pin.zip_sha256.is_empty() && pin.inner_path.is_empty()
        }
    };
    if !download_ok
        || pin.inner_bytes == 0
        || pin.inner_bytes as u64 > MAX_ARCHIVE_BYTES
        || !valid_sha256(&pin.inner_sha256)
    {
        return invalid("archive");
    }
    let audit = &manifest.audited;
    if audit.entries == 0
        || audit.install_files == 0
        || audit.install_files + audit.rejected + audit.ignored > audit.entries
        || audit.renamed > audit.entries
        || audit.install_bytes == 0
        || audit.install_bytes > pin.inner_bytes as u64
        || !valid_sha256(&audit.archive_identity)
        || !valid_sha256(&audit.install_identity)
        || !valid_sha256(&audit.valve_list_sha256)
        || audit.valve_list_entries == 0
        // Every pinned shared path is a file the install carries: none left
        // over, none missing.
        || audit.shared != manifest.shared_paths.len()
        || audit.shared > audit.install_files
    {
        return invalid("audit");
    }
    match manifest.scope {
        // The opt-in mode leaves the shared files out: the two pinned results
        // differ by exactly those files.
        ScopeKind::Hero => {
            if !valid_sha256(&audit.strip_identity)
                || audit.strip_files + audit.shared != audit.install_files
                || audit.strip_bytes > audit.install_bytes
                || (audit.shared == 0
                    && (audit.strip_bytes != audit.install_bytes
                        || audit.strip_identity != audit.install_identity))
                || (audit.shared > 0 && audit.strip_identity == audit.install_identity)
            {
                return invalid("audit");
            }
        }
        ScopeKind::None => {
            if audit.strip_files != 0 || audit.strip_bytes != 0 || !audit.strip_identity.is_empty()
            {
                return invalid("audit");
            }
        }
    }
    Ok(())
}

fn parse_all(source: &str) -> Result<Vec<WardrobeManifest>, String> {
    let manifests: Vec<WardrobeManifest> = serde_json::from_str(source)
        .map_err(|error| format!("wardrobe_manifest_invalid:parse:{error}"))?;
    let mut ids = BTreeSet::new();
    let mut catalog_ids = BTreeSet::new();
    for manifest in &manifests {
        validate(manifest)?;
        if !ids.insert(manifest.id.as_str()) || !catalog_ids.insert(manifest.catalog_id.as_str()) {
            return Err(format!(
                "wardrobe_manifest_invalid:{}:duplicate",
                manifest.id
            ));
        }
    }
    Ok(manifests)
}

struct Registry {
    manifests: Vec<WardrobeManifest>,
    by_id: BTreeMap<String, usize>,
}

/// Every shard parsed, validated and joined; a manifest must sit in the shard
/// of its category, and IDs are unique across all of them.
fn parse_shards() -> Result<Vec<WardrobeManifest>, String> {
    let mut all = Vec::new();
    for shard in SHARDS {
        let manifests = parse_all(shard.text)?;
        if manifests
            .iter()
            .any(|manifest| !shard.categories.contains(&manifest.category.as_str()))
        {
            return Err(format!("wardrobe_manifest_invalid:{}:shard", shard.file));
        }
        all.extend(manifests);
    }
    let mut ids = BTreeSet::new();
    let mut catalog_ids = BTreeSet::new();
    for manifest in &all {
        if !ids.insert(manifest.id.as_str()) || !catalog_ids.insert(manifest.catalog_id.as_str()) {
            return Err(format!(
                "wardrobe_manifest_invalid:{}:duplicate",
                manifest.id
            ));
        }
    }
    Ok(all)
}

fn embedded() -> &'static Result<Registry, String> {
    static EMBEDDED: OnceLock<Result<Registry, String>> = OnceLock::new();
    EMBEDDED.get_or_init(|| {
        let manifests = parse_shards()?;
        let mut by_id = BTreeMap::new();
        for (index, manifest) in manifests.iter().enumerate() {
            by_id.insert(manifest.id.clone(), index);
            by_id.insert(manifest.catalog_id.clone(), index);
        }
        Ok(Registry { manifests, by_id })
    })
}

#[cfg(test)]
pub(crate) fn manifests() -> Result<&'static [WardrobeManifest], String> {
    embedded()
        .as_ref()
        .map(|registry| registry.manifests.as_slice())
        .map_err(|_| "wardrobe_registry_invalid".to_string())
}

/// Resolves either the engine ID (`wardrobe.scarlet-keeper-of-the-light`) or
/// the catalog ID (`heroes-scarlet-keeper-of-the-light`). `Ok(None)` means the
/// ID is not a wardrobe item; an invalid registry is an error, not `None`.
pub(crate) fn find(id: &str) -> Result<Option<&'static WardrobeManifest>, String> {
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
        .map_err(|_| "wardrobe_registry_invalid".to_string())?;
    Ok(registry
        .by_id
        .get(id)
        .map(|index| &registry.manifests[*index]))
}

/// Resolves an ID that may carry the `.strip` suffix to the item and the mode
/// it is installed in.
pub(crate) fn find_variant(
    id: &str,
) -> Result<Option<(&'static WardrobeManifest, SharedPolicy)>, String> {
    let (base, policy) = parse_variant(id);
    Ok(find(base)?
        .filter(|item| policy == SharedPolicy::Install || item.scope == ScopeKind::Hero)
        .map(|item| (item, policy)))
}

/// Test-only registry of synthetic items, so build tests can exercise the
/// whole pipeline without the real archive. Never compiled into the app.
#[cfg(test)]
static TEST_ITEMS: std::sync::Mutex<Vec<&'static WardrobeManifest>> =
    std::sync::Mutex::new(Vec::new());

#[cfg(test)]
pub(crate) fn register_for_test(manifest: WardrobeManifest) -> &'static WardrobeManifest {
    validate(&manifest).expect("valid test item");
    let leaked: &'static WardrobeManifest = Box::leak(Box::new(manifest));
    TEST_ITEMS.lock().expect("test registry").push(leaked);
    leaked
}

/// Test-only: a manifest value for a small synthetic archive made of `files`
/// (paths for `vpk::build`), with the audit the analyzer produces for it.
/// `particles/basic_ambient/basic_ambient.vpcf_c` and
/// `materials/default/default_color_tga_1.vtex_c` stand in for files Valve
/// ships: when present they are the archive's audited shared paths.
#[cfg(test)]
pub(crate) fn synthetic_value(id: &str, files: &[(&str, &[u8])]) -> (serde_json::Value, Vec<u8>) {
    use crate::vpk::{build, VpkInput};
    let archive = build(
        files
            .iter()
            .map(|(path, bytes)| VpkInput { path, bytes })
            .collect(),
    )
    .expect("synthetic archive");
    let mut shared_paths = files
        .iter()
        .map(|(path, _)| (*path).to_string())
        .filter(|path| {
            [
                "particles/basic_ambient/basic_ambient.vpcf_c",
                "materials/default/default_color_tga_1.vtex_c",
            ]
            .contains(&path.as_str())
        })
        .collect::<Vec<_>>();
    shared_paths.sort();
    let audited = audit_both_modes(
        &archive,
        &["keeper_of_the_light".to_string()],
        &shared_paths,
        &"2".repeat(64),
        388116,
    )
    .expect("audit");
    let value = serde_json::json!({
        "schemaVersion": SCHEMA_VERSION,
        "kind": "wardrobe",
        "id": format!("wardrobe.{id}"),
        "catalogId": format!("heroes-{id}"),
        "category": "heroes",
        "scope": "hero",
        "name": { "ru": "Test Skin", "en": "Test Skin" },
        "author": "Test",
        "heroes": ["keeper_of_the_light"],
        "sharedPaths": shared_paths,
        "distribution": "internal_pilot",
        "source": {
            "urls": [
                "https://raw.githubusercontent.com/h6rd/Dota2PornFxWeb/main/assets/files/heroes/Test%20Skin.zip",
                "https://huggingface.co/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/heroes/Test%20Skin.zip"
            ],
            "catalogSnapshot": "test",
            "permission": "founder_reported_author_consent"
        },
        "archive": {
            "kind": "zip",
            "zipBytes": 1000,
            "zipSha256": "1".repeat(64),
            "innerPath": "pak59_dir.vpk",
            "innerBytes": archive.len(),
            "innerSha256": sha256_hex(&archive)
        },
        "audited": audited
    });
    (value, archive)
}

/// What an audit pins: the analyzer's result in both modes for a hero skin,
/// in the one mode there is for content with no hero. Used by the generator and
/// by tests; the app only ever checks a pin, never makes one.
#[cfg(test)]
pub(crate) fn audit_archive(
    archive: &[u8],
    base: &ScopeSpec,
    valve_list_sha256: &str,
    valve_list_entries: usize,
) -> Result<AuditedResult, String> {
    let run = |policy| {
        let spec = ScopeSpec {
            kind: base.kind,
            valve_references: base.valve_references.clone(),
            types: base.types,
            heroes: base.heroes.clone(),
            shared_paths: base.shared_paths.clone(),
            shared_policy: policy,
        };
        skin_archive::analyze(archive, &spec, &Limits::default()).map(|analysis| analysis.report)
    };
    let install = run(SharedPolicy::Install)?;
    let strip = match base.kind {
        ScopeKind::Hero => Some(run(SharedPolicy::Strip)?),
        ScopeKind::None => None,
    };
    Ok(AuditedResult {
        entries: install.entries,
        archive_identity: install.archive_identity.clone(),
        rejected: install.rejected.files,
        ignored: install.ignored.files,
        shared: install.shared.files,
        install_files: install.install.files,
        install_bytes: install.install.bytes,
        install_identity: install.install.identity.clone(),
        strip_files: strip.as_ref().map_or(0, |report| report.install.files),
        strip_bytes: strip.as_ref().map_or(0, |report| report.install.bytes),
        strip_identity: strip
            .as_ref()
            .map_or_else(String::new, |report| report.install.identity.clone()),
        renamed: install.renamed,
        valve_list_sha256: valve_list_sha256.to_string(),
        valve_list_entries,
    })
}

/// Test-only: the hero-skin audit of a synthetic archive.
#[cfg(test)]
pub(crate) fn audit_both_modes(
    archive: &[u8],
    heroes: &[String],
    shared_paths: &[String],
    valve_list_sha256: &str,
    valve_list_entries: usize,
) -> Result<AuditedResult, String> {
    audit_archive(
        archive,
        &ScopeSpec {
            kind: ScopeKind::Hero,
            valve_references: Vec::new(),
            types: TypePolicy::Resources,
            heroes: heroes.to_vec(),
            shared_paths: shared_paths.to_vec(),
            shared_policy: SharedPolicy::Install,
        },
        valve_list_sha256,
        valve_list_entries,
    )
}

#[cfg(test)]
pub(crate) fn synthetic_item(
    id: &str,
    files: &[(&str, &[u8])],
    edit: impl FnOnce(&mut serde_json::Value),
) -> (WardrobeManifest, Vec<u8>) {
    let (mut value, archive) = synthetic_value(id, files);
    edit(&mut value);
    let manifest: WardrobeManifest = serde_json::from_value(value).expect("manifest");
    validate(&manifest).expect("valid");
    (manifest, archive)
}

/// Test-only: a manifest value for content with no hero (a courier, a sound
/// pack ...) made of `files`, as a raw VPK or a ZIP, with the audit the
/// analyzer produces. The same two paths as in [`synthetic_value`] stand in for
/// files Valve ships.
#[cfg(test)]
pub(crate) fn synthetic_pack_value(
    category: &str,
    id: &str,
    kind: SourceKind,
    files: &[(&str, &[u8])],
) -> (serde_json::Value, Vec<u8>) {
    use crate::vpk::{build, VpkInput};
    let archive = build(
        files
            .iter()
            .map(|(path, bytes)| VpkInput { path, bytes })
            .collect(),
    )
    .expect("synthetic archive");
    let mut shared_paths = files
        .iter()
        .map(|(path, _)| (*path).to_string())
        .filter(|path| {
            [
                "particles/basic_ambient/basic_ambient.vpcf_c",
                "materials/default/default_color_tga_1.vtex_c",
                "sounds/vo/announcer/kill.vsnd_c",
            ]
            .contains(&path.as_str())
        })
        .collect::<Vec<_>>();
    shared_paths.sort();
    let rule = skin_archive::category_rule(category).expect("category");
    let audited = audit_archive(
        &archive,
        &ScopeSpec {
            kind: rule.scope,
            valve_references: Vec::new(),
            types: rule.types,
            heroes: Vec::new(),
            shared_paths: shared_paths.clone(),
            shared_policy: SharedPolicy::Install,
        },
        &"2".repeat(64),
        388116,
    )
    .expect("audit");
    let (extension, pin) = match kind {
        SourceKind::Zip => (
            "zip",
            serde_json::json!({
                "kind": "zip",
                "zipBytes": 1000,
                "zipSha256": "1".repeat(64),
                "innerPath": "pak10_dir.vpk",
                "innerBytes": archive.len(),
                "innerSha256": sha256_hex(&archive)
            }),
        ),
        SourceKind::Vpk => (
            "vpk",
            serde_json::json!({
                "kind": "vpk",
                "innerBytes": archive.len(),
                "innerSha256": sha256_hex(&archive)
            }),
        ),
    };
    let value = serde_json::json!({
        "schemaVersion": SCHEMA_VERSION,
        "kind": "wardrobe",
        "id": format!("wardrobe.{category}-{id}"),
        "catalogId": format!("{category}-{id}"),
        "category": category,
        "scope": "none",
        "name": { "ru": "Test Pack", "en": "Test Pack" },
        "author": "Test",
        "sharedPaths": shared_paths,
        "distribution": "internal_pilot",
        "source": {
            "urls": [
                format!("https://raw.githubusercontent.com/h6rd/Dota2PornFxWeb/main/assets/files/{category}/Test%20Pack.{extension}"),
                format!("https://huggingface.co/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/{category}/Test%20Pack.{extension}")
            ],
            "catalogSnapshot": "test",
            "permission": "founder_reported_author_consent"
        },
        "archive": pin,
        "audited": audited
    });
    (value, archive)
}

#[cfg(test)]
pub(crate) fn synthetic_pack(
    category: &str,
    id: &str,
    kind: SourceKind,
    files: &[(&str, &[u8])],
) -> (WardrobeManifest, Vec<u8>) {
    let (value, archive) = synthetic_pack_value(category, id, kind, files);
    let manifest: WardrobeManifest = serde_json::from_value(value).expect("manifest");
    validate(&manifest).expect("valid");
    (manifest, archive)
}

/// Whether the catalog ID belongs to the wardrobe allowlist; a signed package
/// catalog may not reuse it.
pub(crate) fn catalog_id_taken(catalog_id: &str) -> bool {
    embedded().as_ref().is_ok_and(|registry| {
        registry
            .by_id
            .get(catalog_id)
            .is_some_and(|index| registry.manifests[*index].catalog_id == catalog_id)
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Reads exactly one named entry out of the downloaded ZIP and checks it
/// against the pin. The ZIP is inspected first (no traversal, links, encrypted
/// or executable entries, bounded sizes) and must hold exactly that one file;
/// the entry is read through a hard length bound, not the size it declares.
pub(crate) fn extract_pinned_entry(
    zip_bytes: &[u8],
    inner_path: &str,
    inner_bytes: usize,
    inner_sha256: &str,
) -> Result<Vec<u8>, String> {
    let report = crate::archive_inspector::inspect_zip_bytes(zip_bytes)?;
    if report.entries != 1 || report.files != 1 {
        return Err("wardrobe_archive_unexpected".to_string());
    }
    let mut archive = zip::read::ZipArchive::new(Cursor::new(zip_bytes))
        .map_err(|_| "archive_format_invalid".to_string())?;
    let mut entry = archive
        .by_name(inner_path)
        .map_err(|_| "wardrobe_archive_unexpected".to_string())?;
    if !entry.is_file() || entry.size() != inner_bytes as u64 {
        return Err("wardrobe_archive_unexpected".to_string());
    }
    let mut bytes = Vec::with_capacity(inner_bytes);
    entry
        .by_ref()
        .take(inner_bytes as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "wardrobe_archive_unreadable".to_string())?;
    if bytes.len() != inner_bytes || sha256_hex(&bytes) != inner_sha256 {
        return Err("wardrobe_archive_hash_mismatch".to_string());
    }
    Ok(bytes)
}

/// Makes sure the item's VPK is in the content store, downloading and
/// verifying the pinned file (a ZIP, or the raw VPK) when it is not. Only the
/// verified VPK is stored.
pub(crate) fn acquire(
    app_data_root: &Path,
    manifest: &WardrobeManifest,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    let pin = &manifest.archive;
    if content_store::read_pinned_resource(app_data_root, pin.inner_bytes, &pin.inner_sha256)
        .is_ok()
    {
        return Ok(());
    }
    let downloaded = crate::remote_intake::fetch_pinned_wardrobe_archive(manifest, cancelled)?;
    let inner = match pin.kind {
        SourceKind::Zip => extract_pinned_entry(
            &downloaded,
            &pin.inner_path,
            pin.inner_bytes,
            &pin.inner_sha256,
        )?,
        // The download is the VPK: size and SHA-256 were checked by the fetch.
        SourceKind::Vpk => downloaded,
    };
    content_store::store_pinned_resource(app_data_root, pin.inner_bytes, &pin.inner_sha256, &inner)
}

/// The analyzer must reproduce the audit, field for field, for the mode that
/// was asked for.
fn matches_audit(manifest: &WardrobeManifest, report: &SkinReport) -> bool {
    let audit = &manifest.audited;
    let Some((files, bytes, identity)) = audit.pin(report.shared_policy) else {
        return false;
    };
    report.entries == audit.entries
        && report.archive_identity == audit.archive_identity
        && report.install.files == files
        && report.install.bytes == bytes
        && report.install.identity == identity
        && report.rejected.files == audit.rejected
        && report.ignored.files == audit.ignored
        && report.renamed == audit.renamed
        // Every audited shared path was found in the archive.
        && report.shared.files == manifest.shared_paths.len()
}

/// What a build knows about the player's own game: which of the skin's
/// author-owned files the game ships now although the audit did not find them
/// in Valve's list (Valve added a file since the audit). Kept per item and
/// read back by every later rebuild, like the blacklist expansion, so a Dota
/// update does not change what BetterFy considers installed.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GameCheckRecord {
    package_id: String,
    inner_sha256: String,
    install_identity: String,
    /// False when no Dota install was available or its file list could not be
    /// read; the build then says so and nothing is stripped.
    checked: bool,
    new_in_game: Vec<String>,
}

fn game_check_path(app_data_root: &Path, package_id: &str) -> PathBuf {
    app_data_root
        .join("engine-v1")
        .join("wardrobe")
        .join(format!("{package_id}.json"))
}

fn read_game_check(app_data_root: &Path, manifest: &WardrobeManifest) -> Option<GameCheckRecord> {
    let bytes = std::fs::read(game_check_path(app_data_root, &manifest.id)).ok()?;
    if bytes.len() > 4 * 1024 * 1024 {
        return None;
    }
    let record: GameCheckRecord = serde_json::from_slice(&bytes).ok()?;
    // A record belongs to exactly this audited archive.
    (record.package_id == manifest.id
        && record.inner_sha256 == manifest.archive.inner_sha256
        && record.install_identity == manifest.audited.install_identity
        && record.checked)
        .then_some(record)
}

fn write_game_check(
    app_data_root: &Path,
    manifest: &WardrobeManifest,
    record: &GameCheckRecord,
) -> Result<(), String> {
    let target = game_check_path(app_data_root, &manifest.id);
    let parent = target
        .parent()
        .ok_or_else(|| "wardrobe_game_check_failed".to_string())?;
    std::fs::create_dir_all(parent).map_err(|_| "wardrobe_game_check_failed".to_string())?;
    let bytes = serde_json::to_vec(record).map_err(|_| "wardrobe_game_check_failed".to_string())?;
    let temporary = target.with_extension("json.tmp");
    std::fs::write(&temporary, bytes).map_err(|_| "wardrobe_game_check_failed".to_string())?;
    std::fs::rename(&temporary, &target).map_err(|_| "wardrobe_game_check_failed".to_string())
}

/// Compares the skin's author-owned files with the file list of the player's
/// own game (`game_paths`, lower-case paths of `game/dota/pak01_dir.vpk`) and
/// stores the result for this item. With `None` (no game folder, or its file
/// list could not be read) the stored record says "not checked". Never fails
/// the plan because of the game: only a broken content store or an unwritable
/// app-data folder is an error.
pub(crate) fn record_game_check(
    app_data_root: &Path,
    manifest: &WardrobeManifest,
    game_paths: Option<&BTreeSet<String>>,
) -> Result<(), String> {
    let mut record = GameCheckRecord {
        package_id: manifest.id.clone(),
        inner_sha256: manifest.archive.inner_sha256.clone(),
        install_identity: manifest.audited.install_identity.clone(),
        checked: false,
        new_in_game: Vec::new(),
    };
    if let Some(game_paths) = game_paths {
        let pin = &manifest.archive;
        let archive =
            content_store::read_pinned_resource(app_data_root, pin.inner_bytes, &pin.inner_sha256)?;
        let spec = manifest.scope_spec(SharedPolicy::Install);
        let analysis = skin_archive::analyze(&archive, &spec, &Limits::default())?;
        record.new_in_game = analysis
            .author_owned_paths()
            .into_iter()
            .filter(|path| game_paths.contains(path))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        record.checked = true;
    }
    write_game_check(app_data_root, manifest, &record)
}

/// Reads the item's VPK back from the content store (size and SHA-256 are
/// checked again), analyzes it in the requested mode and returns the files to
/// install.
pub(crate) fn load(
    app_data_root: &Path,
    manifest: &WardrobeManifest,
    policy: SharedPolicy,
) -> Result<LoadedWardrobe, String> {
    if !matches!(policy, SharedPolicy::Install | SharedPolicy::Strip) {
        return Err("wardrobe_policy_unsupported".to_string());
    }
    let pin = &manifest.archive;
    let archive =
        content_store::read_pinned_resource(app_data_root, pin.inner_bytes, &pin.inner_sha256)?;
    let mut analysis =
        skin_archive::analyze(&archive, &manifest.scope_spec(policy), &Limits::default())?;
    // The audit is checked before anything about the player's game is applied.
    if !matches_audit(manifest, &analysis.report) {
        return Err("wardrobe_audit_mismatch".to_string());
    }
    let game_check = read_game_check(app_data_root, manifest);
    if let Some(record) = &game_check {
        analysis.apply_game_check(&record.new_in_game.iter().cloned().collect());
    }
    let selection = analysis.into_selection()?;
    Ok(LoadedWardrobe {
        resources: selection.files,
        item: WardrobePlanItem {
            package_id: variant_id(manifest, policy),
            catalog_id: manifest.catalog_id.clone(),
            category: manifest.category.clone(),
            scope: manifest.scope,
            heroes: manifest.heroes.clone(),
            source_urls: manifest.source.urls.clone(),
            archive_sha256: pin.download_sha256().to_string(),
            catalog_snapshot: manifest.source.catalog_snapshot.clone(),
            permission: manifest.source.permission.clone(),
            verified_languages: manifest.verified_languages.clone(),
            compatibility_note: manifest.compatibility_note.clone(),
            shared_policy: policy,
            game_checked: game_check.is_some(),
            report: selection.report,
        },
    })
}

/// Files and bytes a set of wardrobe items adds to the merged VPK in the mode
/// each is selected in, for the size budget. Unknown IDs count for nothing
/// (they fail elsewhere).
pub(crate) fn install_footprint(ids: &[String]) -> (usize, u64) {
    ids.iter()
        .filter_map(|id| find_variant(id).ok().flatten())
        .filter_map(|(item, policy)| item.audited.pin(policy))
        .fold(
            (0usize, 0u64),
            |(files, bytes), (item_files, item_bytes, _)| (files + item_files, bytes + item_bytes),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn scarlet() -> &'static WardrobeManifest {
        find("heroes-scarlet-keeper-of-the-light")
            .expect("registry")
            .expect("scarlet keeper")
    }

    /// A valid manifest (over a synthetic archive) as JSON, for rejection cases.
    fn base_value() -> serde_json::Value {
        synthetic_value(
            "base-skin",
            &[("models/heroes/keeper_of_the_light/a.vmdl_c", b"a")],
        )
        .0
    }

    fn array_with(edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut value = base_value();
        edit(&mut value);
        serde_json::Value::Array(vec![value]).to_string()
    }

    fn temp_root(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "betterfy-wardrobe-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        root
    }

    #[test]
    fn the_embedded_allowlist_pins_the_audited_scarlet_keeper_archive() {
        let all = manifests().expect("registry parses");
        assert!(!all.is_empty());
        let item = scarlet();
        assert_eq!(item.id, "wardrobe.scarlet-keeper-of-the-light");
        assert_eq!(item.heroes, vec!["keeper_of_the_light"]);
        assert_eq!(
            item.archive.zip_sha256,
            "f9f0c2f63a9989ea6fd2779cfea1676a2e07bd94eabab9e0c2127a68e7e268a7"
        );
        assert_eq!(item.archive.zip_bytes, 24_071_988);
        assert_eq!(item.archive.inner_bytes, 58_096_981);
        // The default pins everything the analyzer accepts: the 1,010 entries
        // minus the 14 uncompiled `.vpcf` leftovers. Valve's list has 27 of
        // them outside the hero; the opt-in mode leaves exactly those out.
        let audit = &item.audited;
        assert_eq!(audit.entries, 1010);
        assert_eq!(audit.rejected, 0);
        assert_eq!(audit.ignored, 14);
        assert_eq!(audit.install_files, 996);
        assert_eq!(audit.shared, 27);
        assert_eq!(audit.strip_files, 969);
        assert_eq!(item.shared_paths.len(), 27);
        assert_eq!(
            item.archive.inner_sha256,
            "12137a743ff91c8dc1341e32718580109f86c70df735e24b022929e38cc9a190"
        );
        assert!(item.source.urls.contains(
            &"https://huggingface.co/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/heroes/Scarlet%20Keeper.zip"
                .to_string()
        ));
        assert!(find("wardrobe.scarlet-keeper-of-the-light")
            .expect("registry")
            .is_some());
        assert!(find("minify-tree-mod").expect("registry").is_none());
        assert!(catalog_id_taken("heroes-scarlet-keeper-of-the-light"));
        assert!(!catalog_id_taken("wardrobe.scarlet-keeper-of-the-light"));
        assert!(!catalog_id_taken("minify-tree-mod"));
    }

    #[test]
    fn every_embedded_manifest_revalidates_and_the_files_are_canonical() {
        let mut everything = Vec::new();
        for shard in SHARDS {
            let all = parse_all(shard.text).expect("every embedded manifest is valid");
            let mut previous = "";
            for item in &all {
                // sorted by engine id, so the generated file is deterministic
                assert!(previous < item.id.as_str(), "{} is out of order", item.id);
                previous = &item.id;
                assert!(
                    shard.categories.contains(&item.category.as_str()),
                    "{} is in the wrong shard",
                    item.id
                );
                assert!(
                    valid_archive_urls(&item.source.urls, item.archive.kind),
                    "{}",
                    item.id
                );
                assert_eq!(item.source.permission, "founder_reported_author_consent");
                // mirrors are listed in the order they are tried, GitHub or HF
                assert!(item.source.urls.len() <= TRUSTED_ARCHIVE_PREFIXES.len());
                assert!(item.shared_paths.windows(2).all(|pair| pair[0] < pair[1]));
                assert!(item.audited.install_files > 0);
                assert!(item.audited.install_bytes <= item.archive.inner_bytes as u64);
            }
            // Compact and re-serializable: what is embedded is what the
            // generator would write for these manifests.
            assert_eq!(
                serde_json::to_string(&all).expect("serializes"),
                shard.text.trim_end(),
                "{} is not the canonical compact form; regenerate it",
                shard.file
            );
            everything.extend(all);
        }
        assert!(!everything.is_empty());
        // Every enabled category has a shard, and no shard names another.
        for rule in skin_archive::CATEGORIES {
            assert_eq!(
                SHARDS
                    .iter()
                    .filter(|shard| shard.categories.contains(&rule.name))
                    .count(),
                1,
                "{}",
                rule.name
            );
        }
        everything.sort_by(|a, b| a.id.cmp(&b.id));
        let index = everything.iter().map(index_entry).collect::<Vec<_>>();
        assert_eq!(
            serde_json::to_string(&index).expect("serializes"),
            INDEX.trim_end(),
            "index.json is not derived from the manifests; regenerate it"
        );
        // Only the generated files live in the folder.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wardrobe");
        let mut names = std::fs::read_dir(dir)
            .expect("wardrobe dir")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        names.sort();
        let mut expected = SHARDS
            .iter()
            .map(|shard| shard.file.to_string())
            .chain(["index.json".to_string()])
            .collect::<Vec<_>>();
        expected.sort();
        assert_eq!(names, expected);
    }

    #[test]
    fn hostile_or_malformed_entries_are_rejected() {
        assert!(parse_all(&array_with(|_| {})).is_ok());
        let github = |tail: &str| {
            format!(
                "https://raw.githubusercontent.com/h6rd/Dota2PornFxWeb/main/assets/files/{tail}"
            )
        };
        let hf = |tail: &str| {
            format!(
                "https://huggingface.co/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/{tail}"
            )
        };
        let cases = [
            array_with(|v| v["kind"] = "package".into()),
            array_with(|v| v["schemaVersion"] = 2.into()),
            array_with(|v| v["schemaVersion"] = 4.into()),
            array_with(|v| v["distribution"] = "public".into()),
            array_with(|v| v["id"] = "minify.scarlet".into()),
            array_with(|v| v["id"] = "wardrobe.Scarlet".into()),
            array_with(|v| v["catalogId"] = "scarlet".into()),
            array_with(|v| v["heroes"] = serde_json::json!([])),
            array_with(|v| v["heroes"] = serde_json::json!(["Keeper"])),
            array_with(|v| v["heroes"] = serde_json::json!(["axe", "axe"])),
            array_with(|v| v["sharedPaths"] = serde_json::json!(["b/b.vtex_c", "a/a.vtex_c"])),
            array_with(|v| v["sharedPaths"] = serde_json::json!(["A/a.vtex_c"])),
            array_with(|v| v["sharedPaths"] = serde_json::json!(["../a.vtex_c"])),
            // the audit says no file is stripped but a path is pinned
            array_with(|v| v["sharedPaths"] = serde_json::json!(["a/a.vtex_c"])),
            // the audit counts a shared file that no pinned path accounts for
            array_with(|v| v["audited"]["shared"] = 1.into()),
            array_with(|v| v["audited"]["stripFiles"] = 0.into()),
            array_with(|v| v["audited"]["stripIdentity"] = "9".repeat(64).into()),
            array_with(|v| v["audited"]["stripBytes"] = 1_000_000.into()),
            array_with(|v| v["audited"]["strippedShared"] = 1.into()),
            array_with(|v| v["sharedPolicy"] = "strip".into()),
            array_with(|v| v["source"]["urls"] = serde_json::json!([])),
            array_with(|v| {
                v["source"]["urls"] =
                    serde_json::json!([github("heroes/Test%20Skin.zip").replace("https", "http")])
            }),
            array_with(|v| {
                v["source"]["urls"] = serde_json::json!([
                    "https://raw.githubusercontent.com.evil.example/h6rd/Dota2PornFxWeb/main/assets/files/heroes/a.zip"
                ])
            }),
            array_with(|v| {
                v["source"]["urls"] = serde_json::json!([
                    "https://huggingface.co.evil.example/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/heroes/a.zip"
                ])
            }),
            array_with(|v| {
                v["source"]["urls"] = serde_json::json!([
                    "https://huggingface.co/datasets/other/Repo/resolve/main/assets/files/heroes/a.zip"
                ])
            }),
            array_with(|v| {
                v["source"]["urls"] = serde_json::json!([
                    "https://raw.githubusercontent.com/other/Repo/main/assets/files/heroes/a.zip"
                ])
            }),
            array_with(|v| v["source"]["urls"] = serde_json::json!([hf("../../x.zip")])),
            array_with(|v| {
                v["source"]["urls"] = serde_json::json!([hf("heroes/a.zip?download=1")])
            }),
            array_with(|v| v["source"]["urls"] = serde_json::json!([hf("heroes/a.exe")])),
            array_with(|v| v["source"]["urls"] = serde_json::json!([hf("a.zip")])),
            array_with(|v| v["source"]["urls"] = serde_json::json!([hf("heroes/x/a.zip")])),
            // mirrors must name the same file, and each host appears once
            array_with(|v| {
                v["source"]["urls"] =
                    serde_json::json!([hf("heroes/a.zip"), github("heroes/b.zip")])
            }),
            array_with(|v| {
                v["source"]["urls"] = serde_json::json!([hf("heroes/a.zip"), hf("heroes/a.zip")])
            }),
            array_with(|v| {
                v["source"]["urls"] = serde_json::json!([
                    hf("heroes/a.zip"),
                    github("heroes/a.zip"),
                    github("heroes/a.zip")
                ])
            }),
            array_with(|v| v["source"]["permission"] = "".into()),
            array_with(|v| v["archive"]["zipSha256"] = "00".into()),
            array_with(|v| v["archive"]["innerSha256"] = "A".repeat(64).into()),
            array_with(|v| v["archive"]["innerPath"] = "../pak01_dir.vpk".into()),
            array_with(|v| v["archive"]["innerPath"] = "payload.exe".into()),
            array_with(|v| v["archive"]["innerBytes"] = 300_000_000.into()),
            array_with(|v| v["archive"]["zipBytes"] = 0.into()),
            array_with(|v| v["archive"]["zipBytes"] = 300_000_000.into()),
            array_with(|v| v["audited"]["installFiles"] = 0.into()),
            array_with(|v| v["audited"]["installFiles"] = 2000.into()),
            array_with(|v| v["audited"]["installIdentity"] = "abc".into()),
            array_with(|v| v["audited"]["valveListSha256"] = "abc".into()),
            array_with(|v| v["audited"]["renamed"] = 99.into()),
            array_with(|v| v["script"] = "rm -rf".into()),
            array_with(|v| {
                v["source"]["extra"] = 1.into();
            }),
        ];
        for case in &cases {
            assert!(parse_all(case).is_err(), "accepted: {case}");
        }
        // Both trusted mirrors, in either order, are fine.
        for urls in [
            serde_json::json!([github("heroes/a.zip")]),
            serde_json::json!([hf("hero-items/a%20b.zip")]),
            serde_json::json!([github("heroes/a.zip"), hf("heroes/a.zip")]),
            serde_json::json!([hf("heroes/a.zip"), github("heroes/a.zip")]),
        ] {
            let accepted = array_with(|v| v["source"]["urls"] = urls.clone());
            assert!(parse_all(&accepted).is_ok(), "rejected: {urls}");
        }
        // A duplicate ID or catalog ID is refused.
        let one = base_value();
        let twice = serde_json::Value::Array(vec![one.clone(), one]).to_string();
        assert!(parse_all(&twice).is_err(), "duplicates");
        let (mut other, _) = synthetic_value(
            "other-skin",
            &[("models/heroes/keeper_of_the_light/a.vmdl_c", b"a")],
        );
        other["catalogId"] = base_value()["catalogId"].clone();
        let clash = serde_json::Value::Array(vec![base_value(), other]).to_string();
        assert!(parse_all(&clash).is_err(), "catalog id clash");
    }

    #[test]
    fn the_package_registry_reserves_the_wardrobe_namespace() {
        for manifest in crate::package_registry::packages().expect("registry") {
            assert!(!manifest.id.starts_with("wardrobe."), "{}", manifest.id);
            assert!(!catalog_id_taken(&manifest.catalog_id), "{}", manifest.id);
        }
    }

    fn zip_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .expect("start");
            writer.write_all(bytes).expect("write");
        }
        writer.finish().expect("finish").into_inner()
    }

    #[test]
    fn extracts_exactly_the_pinned_entry_and_nothing_else() {
        let payload = b"inner-vpk-bytes".to_vec();
        let sha = sha256_hex(&payload);
        let good = zip_with(&[("pak01_dir.vpk", &payload)]);
        assert_eq!(
            extract_pinned_entry(&good, "pak01_dir.vpk", payload.len(), &sha).expect("extracts"),
            payload
        );
        for (zip, path, size, hash, code) in [
            // wrong hash, wrong declared size, wrong name
            (
                &good,
                "pak01_dir.vpk",
                payload.len(),
                "0".repeat(64),
                "wardrobe_archive_hash_mismatch",
            ),
            (
                &good,
                "pak01_dir.vpk",
                payload.len() + 1,
                sha.clone(),
                "wardrobe_archive_unexpected",
            ),
            (
                &good,
                "other_dir.vpk",
                payload.len(),
                sha.clone(),
                "wardrobe_archive_unexpected",
            ),
        ] {
            assert_eq!(
                extract_pinned_entry(zip, path, size, &hash)
                    .err()
                    .as_deref(),
                Some(code)
            );
        }
        // A second file, a traversal name and an executable are all refused.
        for hostile in [
            zip_with(&[("pak01_dir.vpk", &payload), ("extra.txt", b"x")]),
            zip_with(&[("../pak01_dir.vpk", &payload)]),
            zip_with(&[("pak01_dir.vpk.exe", &payload)]),
        ] {
            assert!(extract_pinned_entry(&hostile, "pak01_dir.vpk", payload.len(), &sha).is_err());
        }
        assert!(extract_pinned_entry(b"not a zip", "pak01_dir.vpk", 1, &sha).is_err());
    }

    /// A small archive with the sample's shape, and a manifest that pins it.
    fn synthetic(edit: impl FnOnce(&mut serde_json::Value)) -> (WardrobeManifest, Vec<u8>) {
        synthetic_item(
            "test-skin",
            &[
                (
                    "models/heroes/keeper_of_the_light/kotl_hood.vmdl_c",
                    b"hood",
                ),
                (
                    "particles/units/heroes/hero_keeper_of_the_light/attack.vpcf_c",
                    b"attack",
                ),
                ("darkness/materials/body.vmat_c", b"body"),
                ("particles/darkness_snaps/snap1.vsnap_c", b"snap"),
                // not in Valve's list: the skin's own, so it installs
                ("particles/basic_ember/basic_ember.vpcf_c", b"ember"),
                ("particles/basic_ambient/basic_ambient.vpcf_c", b"shared"),
                ("materials/default/default_color_tga_1.vtex_c", b"shared2"),
                ("particles/darkness/flame.vpcf", b"source"),
            ],
            edit,
        )
    }

    fn store(root: &Path, manifest: &WardrobeManifest, archive: &[u8]) {
        content_store::store_pinned_resource(
            root,
            manifest.archive.inner_bytes,
            &manifest.archive.inner_sha256,
            archive,
        )
        .expect("store");
    }

    #[test]
    fn loads_only_what_the_audit_describes() {
        let (manifest, archive) = synthetic(|_| {});
        let root = temp_root("load");
        // Nothing stored yet: the build cannot start from thin air.
        assert!(load(&root, &manifest, SharedPolicy::Install).is_err());
        store(&root, &manifest, &archive);

        // The default installs every accepted file, shared ones included.
        let loaded = load(&root, &manifest, SharedPolicy::Install).expect("loads");
        assert_eq!(loaded.resources.len(), 7);
        assert_eq!(loaded.item.shared_policy, SharedPolicy::Install);
        assert_eq!(loaded.item.package_id, manifest.id);
        assert_eq!(loaded.item.report.install.stripped_shared, 0);
        assert_eq!(loaded.item.report.shared.files, 2);
        assert_eq!(loaded.item.report.rejected.files, 0);
        assert_eq!(loaded.item.report.ignored.files, 1);
        assert_eq!(loaded.item.heroes, vec!["keeper_of_the_light"]);
        assert!(!loaded.item.game_checked);
        for path in [
            "particles/basic_ember/basic_ember.vpcf_c",
            "particles/basic_ambient/basic_ambient.vpcf_c",
            "materials/default/default_color_tga_1.vtex_c",
        ] {
            assert!(loaded.resources.contains_key(path), "{path}");
        }
        assert_eq!(
            loaded.resources["models/heroes/keeper_of_the_light/kotl_hood.vmdl_c"],
            b"hood"
        );

        // The opt-in leaves the two shared files out and is a different
        // package ID, so it is a different plan.
        let strip = load(&root, &manifest, SharedPolicy::Strip).expect("loads");
        assert_eq!(strip.resources.len(), 5);
        assert_eq!(strip.item.shared_policy, SharedPolicy::Strip);
        assert_eq!(strip.item.package_id, format!("{}.strip", manifest.id));
        assert_eq!(strip.item.report.install.stripped_shared, 2);
        assert!(!strip
            .resources
            .contains_key("particles/basic_ambient/basic_ambient.vpcf_c"));
        assert!(strip
            .resources
            .contains_key("particles/basic_ember/basic_ember.vpcf_c"));
        assert_eq!(
            load(&root, &manifest, SharedPolicy::Refuse)
                .err()
                .as_deref(),
            Some("wardrobe_policy_unsupported")
        );

        // Any drift between the audit and the analyzer refuses the build in
        // the mode it concerns.
        type Edit = fn(&mut serde_json::Value);
        let drifts: [(Edit, SharedPolicy); 8] = [
            (
                |v| v["audited"]["installIdentity"] = "1".repeat(64).into(),
                SharedPolicy::Install,
            ),
            (
                |v| {
                    v["audited"]["installFiles"] = 6.into();
                    v["audited"]["stripFiles"] = 4.into();
                },
                SharedPolicy::Install,
            ),
            (
                |v| v["audited"]["archiveIdentity"] = "2".repeat(64).into(),
                SharedPolicy::Install,
            ),
            (
                |v| v["audited"]["renamed"] = 1.into(),
                SharedPolicy::Install,
            ),
            (
                |v| v["audited"]["stripIdentity"] = "3".repeat(64).into(),
                SharedPolicy::Strip,
            ),
            (
                |v| v["audited"]["stripBytes"] = 1.into(),
                SharedPolicy::Strip,
            ),
            // An audited shared path the archive does not carry.
            (
                |v| {
                    v["sharedPaths"] = serde_json::json!([
                        "materials/default/default_color_tga_1.vtex_c",
                        "materials/default/default_color_tga_2.vtex_c",
                        "particles/basic_ambient/basic_ambient.vpcf_c"
                    ]);
                    v["audited"]["shared"] = 3.into();
                    v["audited"]["stripFiles"] = 4.into();
                },
                SharedPolicy::Install,
            ),
            // A shared path dropped from the list turns a file into the
            // author's; the opt-in then leaves out one file fewer.
            (
                |v| {
                    v["sharedPaths"] =
                        serde_json::json!(["materials/default/default_color_tga_1.vtex_c"]);
                    v["audited"]["shared"] = 1.into();
                    v["audited"]["stripFiles"] = 6.into();
                },
                SharedPolicy::Strip,
            ),
        ];
        for (edit, policy) in drifts {
            let (drifted, _) = synthetic(edit);
            let root = temp_root("drift");
            store(&root, &drifted, &archive);
            assert_eq!(
                load(&root, &drifted, policy).err().as_deref(),
                Some("wardrobe_audit_mismatch")
            );
            std::fs::remove_dir_all(root).expect("cleanup");
        }
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn strip_variant_ids_resolve_to_the_same_item() {
        let (manifest, _) = synthetic_item(
            "variant-skin",
            &[("models/heroes/keeper_of_the_light/a.vmdl_c", b"a")],
            |_| {},
        );
        let item = register_for_test(manifest);
        assert_eq!(
            parse_variant("wardrobe.x"),
            ("wardrobe.x", SharedPolicy::Install)
        );
        assert_eq!(
            parse_variant("wardrobe.x.strip"),
            ("wardrobe.x", SharedPolicy::Strip)
        );
        for (id, policy) in [
            (item.id.clone(), SharedPolicy::Install),
            (item.catalog_id.clone(), SharedPolicy::Install),
            (format!("{}.strip", item.id), SharedPolicy::Strip),
            (format!("{}.strip", item.catalog_id), SharedPolicy::Strip),
        ] {
            let (found, found_policy) = find_variant(&id).expect("registry").expect(&id);
            assert_eq!(found.id, item.id);
            assert_eq!(found_policy, policy);
        }
        assert!(find_variant("wardrobe.unknown.strip")
            .expect("registry")
            .is_none());
        assert!(find_variant(&format!("{}.strip.strip", item.id))
            .expect("registry")
            .is_none());
        assert_eq!(variant_id(item, SharedPolicy::Install), item.id);
        assert_eq!(
            variant_id(item, SharedPolicy::Strip),
            format!("{}.strip", item.id)
        );
    }

    #[test]
    fn a_tampered_stored_archive_is_never_analyzed() {
        let (manifest, archive) = synthetic(|_| {});
        let root = temp_root("tamper");
        store(&root, &manifest, &archive);
        let object = root
            .join("content-v1")
            .join("objects")
            .join("sha256")
            .join(&manifest.archive.inner_sha256);
        let mut bytes = std::fs::read(&object).expect("object");
        bytes[0] ^= 0xff;
        std::fs::write(&object, bytes).expect("tamper");
        assert_eq!(
            load(&root, &manifest, SharedPolicy::Install)
                .err()
                .as_deref(),
            Some("content_store_corrupt")
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn the_game_check_is_information_by_default_and_strips_only_under_the_opt_in() {
        let (manifest, archive) = synthetic(|_| {});
        let root = temp_root("game-check");
        store(&root, &manifest, &archive);
        let baseline = load(&root, &manifest, SharedPolicy::Install).expect("loads");
        assert!(!baseline.item.game_checked);
        assert!(baseline.item.report.new_in_game.is_empty());

        // A game that ships one of the skin's own files (added since the
        // audit), the skin's hero file (which the skin replaces on purpose)
        // and a file the audit already knew as shared.
        let game = BTreeSet::from([
            "particles/basic_ember/basic_ember.vpcf_c".to_string(),
            "models/heroes/keeper_of_the_light/kotl_hood.vmdl_c".to_string(),
            "materials/default/default_color_tga_1.vtex_c".to_string(),
            "unrelated/file.vmat_c".to_string(),
        ]);
        record_game_check(&root, &manifest, Some(&game)).expect("records");

        // Default: informational, the install is exactly the audited one.
        let checked = load(&root, &manifest, SharedPolicy::Install).expect("loads");
        assert!(checked.item.game_checked);
        assert_eq!(checked.resources.len(), 7);
        assert_eq!(
            checked.item.report.new_in_game,
            vec!["particles/basic_ember/basic_ember.vpcf_c"]
        );
        assert_eq!(checked.item.report.install, baseline.item.report.install);

        // Opt-in: the new file is left out as well.
        let strip = load(&root, &manifest, SharedPolicy::Strip).expect("loads");
        assert_eq!(strip.resources.len(), 4);
        assert!(!strip
            .resources
            .contains_key("particles/basic_ember/basic_ember.vpcf_c"));
        // hero files stay: replacing them is the point of the skin
        assert!(strip
            .resources
            .contains_key("models/heroes/keeper_of_the_light/kotl_hood.vmdl_c"));
        assert_eq!(strip.item.report.install.stripped_shared, 3);

        // The result is stored: a rebuild without the game gives the same files.
        let again = load(&root, &manifest, SharedPolicy::Strip).expect("rebuild");
        assert_eq!(
            again.resources.keys().collect::<Vec<_>>(),
            strip.resources.keys().collect::<Vec<_>>()
        );

        // An unreadable or missing game listing is "not checked", never an
        // error, and it replaces the earlier record.
        record_game_check(&root, &manifest, None).expect("records");
        let unchecked = load(&root, &manifest, SharedPolicy::Strip).expect("loads");
        assert!(!unchecked.item.game_checked);
        assert_eq!(unchecked.resources.len(), 5);

        // A record for another archive is ignored.
        let other = GameCheckRecord {
            package_id: manifest.id.clone(),
            inner_sha256: "3".repeat(64),
            install_identity: manifest.audited.install_identity.clone(),
            checked: true,
            new_in_game: vec!["darkness/materials/body.vmat_c".to_string()],
        };
        write_game_check(&root, &manifest, &other).expect("writes");
        let ignored = load(&root, &manifest, SharedPolicy::Strip).expect("loads");
        assert_eq!(ignored.resources.len(), 5);
        // A garbled record is ignored too.
        std::fs::write(game_check_path(&root, &manifest.id), b"{not json").expect("garble");
        assert_eq!(
            load(&root, &manifest, SharedPolicy::Strip)
                .expect("loads")
                .resources
                .len(),
            5
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_game_that_ships_everything_still_leaves_the_hero_files() {
        let (manifest, archive) = synthetic(|_| {});
        let root = temp_root("game-everything");
        store(&root, &manifest, &archive);
        // Hero files are never stripped, so something always remains.
        let game = BTreeSet::from([
            "darkness/materials/body.vmat_c".to_string(),
            "particles/darkness_snaps/snap1.vsnap_c".to_string(),
            "particles/basic_ember/basic_ember.vpcf_c".to_string(),
        ]);
        record_game_check(&root, &manifest, Some(&game)).expect("records");
        let loaded = load(&root, &manifest, SharedPolicy::Strip).expect("loads");
        assert_eq!(loaded.resources.len(), 2);
        assert_eq!(
            load(&root, &manifest, SharedPolicy::Install)
                .expect("loads")
                .resources
                .len(),
            7
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn the_footprint_counts_the_pinned_size_of_the_mode_selected() {
        let (manifest, _) = synthetic(|_| {});
        let item = register_for_test(manifest);
        let (files, bytes) = install_footprint(&[item.id.clone(), "minify.tree-mod".to_string()]);
        assert_eq!(files, item.audited.install_files);
        assert_eq!(bytes, item.audited.install_bytes);
        let (strip_files, strip_bytes) = install_footprint(&[format!("{}.strip", item.id)]);
        assert_eq!(strip_files, item.audited.strip_files);
        assert_eq!(strip_bytes, item.audited.strip_bytes);
        assert!(strip_bytes < bytes);
        assert_eq!(install_footprint(&[]), (0, 0));
    }

    /// Local-only: checks the embedded pins against the real sample archive
    /// (`BETTERFY_SKIN_SAMPLE`, the inner VPK) and, when `BETTERFY_SKIN_ZIP`
    /// is also set, extracts it from the real ZIP. Neither file is ever part
    /// of the repository.
    #[test]
    fn the_real_sample_matches_its_pins_when_given() {
        let item = scarlet();
        if let Ok(path) = std::env::var("BETTERFY_SKIN_ZIP") {
            let zip = std::fs::read(path).expect("zip");
            assert_eq!(sha256_hex(&zip), item.archive.zip_sha256);
            assert_eq!(zip.len() as u64, item.archive.zip_bytes);
            let inner = extract_pinned_entry(
                &zip,
                &item.archive.inner_path,
                item.archive.inner_bytes,
                &item.archive.inner_sha256,
            )
            .expect("real zip extracts to the pinned entry");
            assert_eq!(inner.len(), item.archive.inner_bytes);
        }
        let Ok(path) = std::env::var("BETTERFY_SKIN_SAMPLE") else {
            return;
        };
        let archive = std::fs::read(path).expect("sample");
        assert_eq!(sha256_hex(&archive), item.archive.inner_sha256);
        let root = temp_root("real");
        content_store::store_pinned_resource(
            &root,
            item.archive.inner_bytes,
            &item.archive.inner_sha256,
            &archive,
        )
        .expect("store");
        let loaded =
            load(&root, item, SharedPolicy::Install).expect("the analyzer reproduces the audit");
        assert_eq!(loaded.resources.len(), item.audited.install_files);
        let strip = load(&root, item, SharedPolicy::Strip)
            .expect("the analyzer reproduces the opt-in audit");
        assert_eq!(strip.resources.len(), item.audited.strip_files);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    fn pack_files() -> Vec<(&'static str, &'static [u8])> {
        vec![
            ("models/courier/test_courier/body.vmdl_c", &b"body"[..]),
            ("materials/models/courier/test_courier/body.vmat_c", b"mat"),
            // Valve ships these two: they override game files and still install
            ("particles/basic_ambient/basic_ambient.vpcf_c", b"shared"),
            ("materials/default/default_color_tga_1.vtex_c", b"shared2"),
            // a note the author left behind: not a game file
            ("models/courier/test_courier/readme.txt", b"note"),
        ]
    }

    fn pack_array_with(edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let (mut value, _) =
            synthetic_pack_value("couriers", "base", SourceKind::Vpk, &pack_files());
        edit(&mut value);
        serde_json::Value::Array(vec![value]).to_string()
    }

    #[test]
    fn a_raw_vpk_pack_with_no_hero_loads_whole_and_has_no_strip_mode() {
        let (manifest, archive) =
            synthetic_pack("couriers", "raw-pack", SourceKind::Vpk, &pack_files());
        assert_eq!(manifest.scope, ScopeKind::None);
        assert!(manifest.heroes.is_empty());
        // The download is the VPK: one pin, no ZIP fields anywhere.
        assert_eq!(manifest.archive.download_bytes(), archive.len() as u64);
        assert_eq!(manifest.archive.download_sha256(), sha256_hex(&archive));
        let json = serde_json::to_value(&manifest).expect("json");
        assert_eq!(json["archive"]["kind"], "vpk");
        for absent in ["zipBytes", "zipSha256", "innerPath"] {
            assert!(json["archive"].get(absent).is_none(), "{absent}");
        }
        assert!(json.get("heroes").is_none());
        for absent in ["stripFiles", "stripBytes", "stripIdentity"] {
            assert!(json["audited"].get(absent).is_none(), "{absent}");
        }
        assert_eq!(manifest.audited.shared, 2);
        assert_eq!(manifest.audited.ignored, 1);
        assert_eq!(manifest.audited.install_files, 4);

        let root = temp_root("raw-pack");
        store(&root, &manifest, &archive);
        let loaded = load(&root, &manifest, SharedPolicy::Install).expect("loads");
        assert_eq!(loaded.resources.len(), 4);
        assert!(loaded
            .resources
            .contains_key("particles/basic_ambient/basic_ambient.vpcf_c"));
        assert_eq!(loaded.item.category, "couriers");
        assert_eq!(loaded.item.scope, ScopeKind::None);
        assert_eq!(loaded.item.archive_sha256, sha256_hex(&archive));
        assert_eq!(loaded.item.report.hero_scoped.files, 0);
        assert_eq!(loaded.item.report.author_owned.files, 2);
        assert_eq!(loaded.item.report.shared.files, 2);
        assert_eq!(loaded.item.report.ignored.files, 1);
        // Nothing to strip for content with no hero: the mode does not exist.
        assert_eq!(
            load(&root, &manifest, SharedPolicy::Strip).err().as_deref(),
            Some("skin_scope_invalid")
        );
        assert!(manifest.audited.pin(SharedPolicy::Strip).is_none());
        let registered = register_for_test(manifest);
        assert!(find_variant(&registered.id).expect("registry").is_some());
        assert!(find_variant(&format!("{}.strip", registered.id))
            .expect("registry")
            .is_none());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_zip_pack_pins_the_zip_and_the_vpk_inside_it() {
        let (manifest, _) = synthetic_pack(
            "creep-deny",
            "zipped",
            SourceKind::Zip,
            &[("particles/msg_fx/msg_deny.vpcf_c", b"deny")],
        );
        assert_eq!(manifest.archive.kind, SourceKind::Zip);
        assert_eq!(manifest.archive.download_bytes(), 1000);
        assert_eq!(manifest.archive.download_sha256(), "1".repeat(64));
        assert_eq!(manifest.archive.inner_path, "pak10_dir.vpk");
    }

    #[test]
    fn malformed_pack_manifests_are_rejected() {
        assert!(parse_all(&pack_array_with(|_| {})).is_ok());
        let cases = [
            (
                "zip fields on a raw vpk",
                pack_array_with(|v| v["archive"]["zipBytes"] = 5.into()),
            ),
            (
                "zip hash on a raw vpk",
                pack_array_with(|v| v["archive"]["zipSha256"] = "1".repeat(64).into()),
            ),
            (
                "inner path on a raw vpk",
                pack_array_with(|v| v["archive"]["innerPath"] = "pak10_dir.vpk".into()),
            ),
            (
                "a zip url for a raw vpk",
                pack_array_with(|v| {
                    v["source"]["urls"] = serde_json::json!([
                        "https://raw.githubusercontent.com/h6rd/Dota2PornFxWeb/main/assets/files/couriers/Test%20Pack.zip"
                    ])
                }),
            ),
            (
                "a path outside the two segments",
                pack_array_with(|v| {
                    v["source"]["urls"] = serde_json::json!([
                        "https://raw.githubusercontent.com/h6rd/Dota2PornFxWeb/main/assets/files/couriers/a/Test.vpk"
                    ])
                }),
            ),
            (
                "heroes on a pack",
                pack_array_with(|v| v["heroes"] = serde_json::json!(["axe"])),
            ),
            (
                "hero scope on a courier",
                pack_array_with(|v| v["scope"] = "hero".into()),
            ),
            (
                "a category that is not enabled",
                pack_array_with(|v| v["category"] = "backgrounds".into()),
            ),
            (
                "an unknown category",
                pack_array_with(|v| v["category"] = "mystery".into()),
            ),
            (
                "a catalog id of another category",
                pack_array_with(|v| v["catalogId"] = "heroes-base".into()),
            ),
            (
                "a strip result on a pack",
                pack_array_with(|v| {
                    v["audited"]["stripFiles"] = 1.into();
                    v["audited"]["stripBytes"] = 1.into();
                    v["audited"]["stripIdentity"] = "3".repeat(64).into();
                }),
            ),
            (
                "a shared path that was not found",
                pack_array_with(|v| v["sharedPaths"] = serde_json::json!(["a/a.vtex_c"])),
            ),
            (
                "the old schema",
                pack_array_with(|v| v["schemaVersion"] = 2.into()),
            ),
        ];
        for (label, case) in cases {
            assert!(parse_all(&case).is_err(), "{label}");
        }
    }

    #[test]
    fn every_enabled_category_has_a_rule_and_a_catalog_prefix() {
        let mut seen = BTreeSet::new();
        for rule in skin_archive::CATEGORIES {
            assert!(seen.insert(rule.name), "{} twice", rule.name);
            assert_eq!(
                rule.scope == ScopeKind::Hero,
                matches!(rule.name, "heroes" | "hero-items"),
                "{}",
                rule.name
            );
        }
        // Only audited ones are enabled; these are deliberately not.
        for name in [
            "cursors",
            "fonts",
            "tools",
            "other",
            "pedestal",
            "tormentor",
            "packs",
        ] {
            assert!(skin_archive::category_rule(name).is_none(), "{name}");
        }
    }
}

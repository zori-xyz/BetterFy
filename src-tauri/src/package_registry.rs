//! Declarative package manifests (`PackageManifest` schema version 1).
//!
//! Every installable package is a JSON file in `src-tauri/packages/`, embedded
//! into the binary at build time and validated once on first use. The engine
//! looks packages up here instead of branching on package IDs, so adding a
//! data-only package is a manifest change, not a Rust change. The same files
//! are read by the interface, which keeps one source of truth for what can be
//! installed.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::OnceLock;

const MANIFESTS: &[&str] = &[
    include_str!("../packages/minify-tree-mod.json"),
    include_str!("../packages/minify-show-networth.json"),
    include_str!("../packages/minify-repopulate-unit-query-hud.json"),
    include_str!("../packages/minify-remove-river.json"),
];

/// Only this repository is trusted as a source, and only through
/// raw.githubusercontent.com at a pinned commit.
const TRUSTED_REPOSITORY: &str = "Egezenn/dota2-minify";
const MAX_RESOURCES_PER_PACKAGE: usize = 256;
const MAX_RESOURCE_BYTES: usize = 8 * 1024 * 1024;
const MAX_PACKAGE_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalizedName {
    pub ru: String,
    pub en: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageSource {
    pub repository: String,
    pub commit: String,
    pub directory: String,
    pub license: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Resource {
    /// Where the file goes inside the built VPK.
    pub path: String,
    pub bytes: usize,
    pub sha256: String,
    /// Repository path to fetch the bytes from, when it is not
    /// `<directory>/<path>`. This is how a Minify `blacklist.txt` line is
    /// expressed: the listed game path receives the matching
    /// `Minify/bin/blank-files/blank.<ext>` placeholder.
    #[serde(default)]
    pub from: Option<String>,
}

/// SHA-256 of zero bytes. Empty resources are valid (Minify silences sounds
/// with zero-length files) and never need a download.
pub const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

impl Resource {
    pub fn is_empty(&self) -> bool {
        self.bytes == 0
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageManifest {
    pub schema_version: u32,
    pub id: String,
    pub catalog_id: String,
    pub name: LocalizedName,
    pub author: String,
    pub source: PackageSource,
    pub distribution: String,
    #[serde(default)]
    pub verified_languages: Vec<String>,
    #[serde(default)]
    pub compatibility_note: Option<String>,
    pub resources: Vec<Resource>,
}

/// What the interface needs to list and label installable packages.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageSummary {
    pub id: String,
    pub catalog_id: String,
    pub name: LocalizedName,
    pub author: String,
    pub license: String,
    pub resource_count: usize,
    pub verified_languages: Vec<String>,
    pub compatibility_note: Option<String>,
}

impl PackageManifest {
    /// `https://raw.githubusercontent.com/<repository>/<commit>/<directory>/<path>`,
    /// with spaces in the directory percent-encoded exactly as before.
    pub fn resource_url(&self, resource: &Resource) -> String {
        let location = match &resource.from {
            Some(from) => from.replace(' ', "%20"),
            None => format!(
                "{}/{}",
                self.source.directory.replace(' ', "%20"),
                resource.path
            ),
        };
        format!(
            "https://raw.githubusercontent.com/{}/{}/{location}",
            self.source.repository, self.source.commit
        )
    }

    pub fn summary(&self) -> PackageSummary {
        PackageSummary {
            id: self.id.clone(),
            catalog_id: self.catalog_id.clone(),
            name: self.name.clone(),
            author: self.author.clone(),
            license: self.source.license.clone(),
            resource_count: self.resources.len(),
            verified_languages: self.verified_languages.clone(),
            compatibility_note: self.compatibility_note.clone(),
        }
    }
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

/// Engine IDs are `namespace.name` (`minify.tree-mod`).
fn valid_package_id(value: &str) -> bool {
    matches!(value.split_once('.'), Some((namespace, name)) if slug(namespace) && slug(name))
}

/// Catalog IDs are the interface's slugs (`minify-tree-mod`).
fn valid_catalog_id(value: &str) -> bool {
    slug(value) && value.contains('-')
}

fn valid_resource_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 240
        && !path.starts_with('/')
        && path.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'_' | b'-' | b'.')
                })
        })
}

fn valid_repository_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 240
        && path.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b' ' | b'_' | b'-' | b'.')
                })
        })
}

fn extension(path: &str) -> Option<&str> {
    path.rsplit('/')
        .next()?
        .rsplit_once('.')
        .map(|(_, extension)| extension)
}

fn validate(manifest: &PackageManifest) -> Result<(), String> {
    let invalid = |reason: &str| Err(format!("package_manifest_invalid:{}:{reason}", manifest.id));
    if manifest.schema_version != 1 {
        return invalid("schema_version");
    }
    if !valid_package_id(&manifest.id) || !valid_catalog_id(&manifest.catalog_id) {
        return invalid("id");
    }
    if manifest.name.ru.trim().is_empty() || manifest.name.en.trim().is_empty() {
        return invalid("name");
    }
    let source = &manifest.source;
    if source.repository != TRUSTED_REPOSITORY
        || source.commit.len() != 40
        || !source.commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !valid_repository_path(&source.directory)
    {
        return invalid("source");
    }
    if manifest.distribution != "internal_pilot" {
        return invalid("distribution");
    }
    if manifest.resources.is_empty() || manifest.resources.len() > MAX_RESOURCES_PER_PACKAGE {
        return invalid("resources");
    }
    let mut paths = BTreeSet::new();
    let mut total = 0usize;
    for resource in &manifest.resources {
        if !valid_resource_path(&resource.path) || !paths.insert(resource.path.as_str()) {
            return invalid("resource_path");
        }
        if resource.bytes > MAX_RESOURCE_BYTES
            || (resource.is_empty() && resource.sha256 != EMPTY_SHA256)
        {
            return invalid("resource_size");
        }
        if let Some(from) = &resource.from {
            // A placeholder must be the same kind of compiled file as the
            // game path it stands in for.
            if !valid_repository_path(from)
                || resource.is_empty()
                || extension(from) != extension(&resource.path)
            {
                return invalid("resource_from");
            }
        }
        if resource.sha256.len() != 64
            || !resource
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return invalid("resource_hash");
        }
        total = total.saturating_add(resource.bytes);
    }
    if total > MAX_PACKAGE_BYTES {
        return invalid("package_size");
    }
    Ok(())
}

fn parse_all(sources: &[&str]) -> Result<Vec<PackageManifest>, String> {
    let mut ids = BTreeSet::new();
    let mut catalog_ids = BTreeSet::new();
    sources
        .iter()
        .map(|source| {
            let manifest: PackageManifest = serde_json::from_str(source)
                .map_err(|error| format!("package_manifest_invalid:parse:{error}"))?;
            validate(&manifest)?;
            if !ids.insert(manifest.id.clone()) || !catalog_ids.insert(manifest.catalog_id.clone())
            {
                return Err(format!(
                    "package_manifest_invalid:{}:duplicate",
                    manifest.id
                ));
            }
            Ok(manifest)
        })
        .collect()
}

fn registry() -> &'static Result<Vec<PackageManifest>, String> {
    static REGISTRY: OnceLock<Result<Vec<PackageManifest>, String>> = OnceLock::new();
    REGISTRY.get_or_init(|| parse_all(MANIFESTS))
}

pub fn packages() -> Result<&'static [PackageManifest], String> {
    registry()
        .as_ref()
        .map(Vec::as_slice)
        .map_err(|_| "package_registry_invalid".to_string())
}

/// Resolves either the engine ID (`minify.tree-mod`) or the catalog ID
/// (`minify-tree-mod`) used by the interface.
pub fn find(id: &str) -> Result<&'static PackageManifest, String> {
    packages()?
        .iter()
        .find(|manifest| manifest.id == id || manifest.catalog_id == id)
        .ok_or_else(|| "pilot_package_unsupported".to_string())
}

pub fn summaries() -> Result<Vec<PackageSummary>, String> {
    Ok(packages()?.iter().map(PackageManifest::summary).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    /// SHA-256 over `id|directory|path|bytes|sha256` lines of the three pilot
    /// contracts as they were hard-coded in tree_pilot.rs before manifests.
    /// If this changes, installed builds would no longer match their plans.
    const PILOT_CONTRACT_FINGERPRINT: &str =
        "3aea21fe81c136c851775f980b0e35c71c5facf2504ce9be1166b503f4c8731b";

    #[test]
    fn embedded_manifests_reproduce_the_original_pilot_contracts() {
        let mut lines = String::new();
        let originals = [
            "minify.tree-mod",
            "minify.show-networth",
            "minify.repopulate-unit-query-hud",
        ];
        for manifest in packages()
            .expect("registry")
            .iter()
            .filter(|manifest| originals.contains(&manifest.id.as_str()))
        {
            let folder = manifest
                .source
                .directory
                .strip_prefix("Minify/mods/")
                .and_then(|value| value.strip_suffix("/files"))
                .expect("minify layout")
                .replace(' ', "%20");
            for resource in &manifest.resources {
                lines.push_str(&format!(
                    "{}|{}|{}|{}|{}\n",
                    manifest.id, folder, resource.path, resource.bytes, resource.sha256
                ));
            }
        }
        assert_eq!(
            format!("{:x}", Sha256::digest(lines.as_bytes())),
            PILOT_CONTRACT_FINGERPRINT
        );
    }

    #[test]
    fn urls_match_the_previous_pinned_layout() {
        let tree = find("minify-tree-mod").expect("tree");
        assert_eq!(
            tree.resource_url(&tree.resources[0]),
            format!(
                "https://raw.githubusercontent.com/Egezenn/dota2-minify/3a85572029f2c264e2a17cee1c9b54ce93e4fd93/Minify/mods/Tree%20Mod/files/{}",
                tree.resources[0].path
            )
        );
        assert_eq!(
            find("minify.show-networth").expect("id").catalog_id,
            "minify-show-networth"
        );
        assert_eq!(
            find("unknown").err().as_deref(),
            Some("pilot_package_unsupported")
        );
    }

    fn manifest_with(edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut value: serde_json::Value = serde_json::from_str(MANIFESTS[1]).expect("json");
        edit(&mut value);
        value.to_string()
    }

    #[test]
    fn blacklist_lines_become_placeholder_resources() {
        let river = find("minify-remove-river").expect("remove river");
        let empty = river.resources.iter().filter(|r| r.is_empty()).count();
        let placeholders = river
            .resources
            .iter()
            .filter(|r| r.from.is_some())
            .collect::<Vec<_>>();
        assert_eq!(river.resources.len(), 23);
        assert_eq!(empty, 5);
        assert_eq!(placeholders.len(), 9);
        assert!(river
            .resource_url(placeholders[0])
            .ends_with("/Minify/bin/blank-files/blank.vmat_c"));
        assert!(river
            .resource_url(&river.resources[0])
            .contains("/Minify/mods/Remove%20River/files/materials/water/"));
    }

    #[test]
    fn hostile_or_malformed_manifests_are_rejected() {
        let cases = [
            manifest_with(|v| v["source"]["repository"] = "attacker/mods".into()),
            manifest_with(|v| v["source"]["commit"] = "main".into()),
            manifest_with(|v| v["source"]["directory"] = "Minify/../../etc".into()),
            manifest_with(|v| v["resources"][0]["path"] = "../escape.vpk".into()),
            manifest_with(|v| v["resources"][0]["path"] = "Panorama/Upper.vxml_c".into()),
            manifest_with(|v| v["resources"][0]["sha256"] = "00".into()),
            manifest_with(|v| v["resources"][0]["bytes"] = 0.into()),
            manifest_with(|v| v["resources"][0]["from"] = "Minify/../secrets.vxml_c".into()),
            manifest_with(|v| {
                v["resources"][0]["from"] = "Minify/bin/blank-files/blank.vmat_c".into()
            }),
            manifest_with(|v| v["resources"] = serde_json::json!([])),
            manifest_with(|v| v["id"] = "no-namespace".into()),
            manifest_with(|v| v["distribution"] = "public".into()),
            manifest_with(|v| v["script"] = "rm -rf".into()),
        ];
        for case in &cases {
            assert!(parse_all(&[case.as_str()]).is_err(), "accepted: {case}");
        }
        assert!(
            parse_all(&[MANIFESTS[1], MANIFESTS[1]]).is_err(),
            "duplicate IDs"
        );
        assert!(parse_all(&[MANIFESTS[1]]).is_ok());
    }
}

//! Wardrobe items: third-party hero skins installed through the same
//! verified pipeline as the audited game-tuning packages.
//!
//! An item is installable only when it is in the embedded allowlist below
//! (`src-tauri/wardrobe/*.json`, compiled into the binary; a signed catalog
//! cannot extend it, like the audited scripts in `package_registry.rs`). An
//! entry pins:
//!
//! * the archive: the downloaded ZIP and the one VPK inside it, by size and
//!   SHA-256, and the HTTPS URL they come from;
//! * the audit: the hero, the author namespaces a human declared, the shared
//!   policy, and what the analyzer found (entry count, archive identity, the
//!   install file count, size and identity). The analyzer is re-run on every
//!   build and must reproduce the audit exactly, so a change to its rules can
//!   never silently change which files an audited archive installs.
//!
//! The bytes are fetched once into the content store (the VPK, after the ZIP
//! is verified), read back and re-hashed on every build, and turned into
//! ordinary bundle resources by [`load`]. Backup, journal, deployment and
//! rollback are the existing transaction and are not touched here.

use crate::content_store;
use crate::package_registry::LocalizedName;
use crate::skin_archive::{self, Limits, ScopeSpec, SharedPolicy, SkinReport};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::OnceLock;

const MANIFESTS: &[&str] = &[include_str!(
    "../wardrobe/heroes-scarlet-keeper-of-the-light.json"
)];

/// The only place archives are downloaded from: the Dota2PornFx project's
/// Hugging Face dataset, over HTTPS. The pinned hash decides whether the bytes
/// are accepted; the host only decides where to look.
const TRUSTED_ARCHIVE_PREFIX: &str =
    "https://huggingface.co/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/";
/// Wardrobe IDs share the package ID shape (`namespace.name`) under their own
/// namespace, which `package_registry` reserves.
pub(crate) const NAMESPACE: &str = "wardrobe";
const MAX_ARCHIVE_BYTES: u64 = content_store::MAX_ARTIFACT_BYTES;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WardrobeSource {
    pub url: String,
    pub catalog_snapshot: String,
    /// How permission to redistribute was established. Recorded, never
    /// inferred: `founder_reported_author_consent` means the project's founder
    /// reports the authors agreed; no licence file ships in the archive.
    pub permission: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchivePin {
    pub zip_bytes: u64,
    pub zip_sha256: String,
    pub inner_path: String,
    pub inner_bytes: usize,
    pub inner_sha256: String,
}

/// What the analyzer found when a person audited this archive.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditedResult {
    pub entries: usize,
    pub archive_identity: String,
    pub install_files: usize,
    pub install_bytes: u64,
    pub install_identity: String,
    pub stripped_shared: usize,
    pub rejected: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WardrobeManifest {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub catalog_id: String,
    pub name: LocalizedName,
    pub author: String,
    pub hero: String,
    pub author_namespaces: Vec<String>,
    pub shared_policy: SharedPolicy,
    pub distribution: String,
    pub source: WardrobeSource,
    pub archive: ArchivePin,
    pub audited: AuditedResult,
    #[serde(default)]
    pub verified_languages: Vec<String>,
    #[serde(default)]
    pub compatibility_note: Option<String>,
}

impl WardrobeManifest {
    pub(crate) fn scope_spec(&self) -> ScopeSpec {
        ScopeSpec {
            hero: self.hero.clone(),
            author_namespaces: self.author_namespaces.clone(),
            shared_policy: self.shared_policy,
        }
    }
}

/// One wardrobe item in a reviewed build plan: what the analyzer found and
/// what the install will contain.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WardrobePlanItem {
    pub package_id: String,
    pub catalog_id: String,
    pub hero: String,
    pub source_url: String,
    pub archive_sha256: String,
    /// Where the catalog listed it, and how permission to use it was
    /// established. Shown as recorded, never upgraded.
    pub catalog_snapshot: String,
    pub permission: String,
    pub verified_languages: Vec<String>,
    pub compatibility_note: Option<String>,
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

fn valid_archive_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix(TRUSTED_ARCHIVE_PREFIX) else {
        return false;
    };
    rest.ends_with(".zip")
        && rest.len() <= 160
        && rest.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'%' | b'.' | b'_' | b'-')
                })
        })
}

fn validate(manifest: &WardrobeManifest) -> Result<(), String> {
    let invalid = |reason: &str| {
        Err(format!(
            "wardrobe_manifest_invalid:{}:{reason}",
            manifest.id
        ))
    };
    if manifest.schema_version != 1 || manifest.kind != "wardrobe" {
        return invalid("schema");
    }
    if manifest.distribution != "internal_pilot" {
        return invalid("distribution");
    }
    let name = manifest.id.strip_prefix("wardrobe.");
    if !name.is_some_and(slug) || !slug(&manifest.catalog_id) || !manifest.catalog_id.contains('-')
    {
        return invalid("id");
    }
    if manifest.name.ru.trim().is_empty()
        || manifest.name.en.trim().is_empty()
        || manifest.author.trim().is_empty()
        || manifest.source.permission.trim().is_empty()
    {
        return invalid("name");
    }
    if manifest.scope_spec().validate().is_err() {
        return invalid("scope");
    }
    if !valid_archive_url(&manifest.source.url) {
        return invalid("source");
    }
    let pin = &manifest.archive;
    if pin.zip_bytes == 0
        || pin.zip_bytes > MAX_ARCHIVE_BYTES
        || pin.inner_bytes == 0
        || pin.inner_bytes as u64 > MAX_ARCHIVE_BYTES
        || !valid_sha256(&pin.zip_sha256)
        || !valid_sha256(&pin.inner_sha256)
        || pin.inner_path.is_empty()
        || pin.inner_path.len() > 64
        || !pin.inner_path.ends_with("_dir.vpk")
        || !pin.inner_path.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'.')
        })
    {
        return invalid("archive");
    }
    let audit = &manifest.audited;
    if audit.entries == 0
        || audit.install_files == 0
        || audit.install_files + audit.stripped_shared + audit.rejected > audit.entries
        || audit.install_bytes == 0
        || audit.install_bytes > pin.inner_bytes as u64
        || !valid_sha256(&audit.archive_identity)
        || !valid_sha256(&audit.install_identity)
    {
        return invalid("audit");
    }
    Ok(())
}

fn parse_all(sources: &[&str]) -> Result<Vec<WardrobeManifest>, String> {
    let mut ids = std::collections::BTreeSet::new();
    let mut catalog_ids = std::collections::BTreeSet::new();
    sources
        .iter()
        .map(|source| {
            let manifest: WardrobeManifest = serde_json::from_str(source)
                .map_err(|error| format!("wardrobe_manifest_invalid:parse:{error}"))?;
            validate(&manifest)?;
            if !ids.insert(manifest.id.clone()) || !catalog_ids.insert(manifest.catalog_id.clone())
            {
                return Err(format!(
                    "wardrobe_manifest_invalid:{}:duplicate",
                    manifest.id
                ));
            }
            Ok(manifest)
        })
        .collect()
}

fn embedded() -> &'static Result<Vec<WardrobeManifest>, String> {
    static EMBEDDED: OnceLock<Result<Vec<WardrobeManifest>, String>> = OnceLock::new();
    EMBEDDED.get_or_init(|| parse_all(MANIFESTS))
}

pub(crate) fn manifests() -> Result<&'static [WardrobeManifest], String> {
    embedded()
        .as_ref()
        .map(Vec::as_slice)
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
    Ok(manifests()?
        .iter()
        .find(|manifest| manifest.id == id || manifest.catalog_id == id))
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

/// Test-only: a manifest that pins a small synthetic archive made of `files`
/// (paths for `vpk::build`), with the audit the analyzer produces for it.
#[cfg(test)]
pub(crate) fn synthetic_item(
    id: &str,
    files: &[(&str, &[u8])],
    edit: impl FnOnce(&mut serde_json::Value),
) -> (WardrobeManifest, Vec<u8>) {
    use crate::vpk::{build, VpkInput};
    let archive = build(
        files
            .iter()
            .map(|(path, bytes)| VpkInput { path, bytes })
            .collect(),
    )
    .expect("synthetic archive");
    let spec = ScopeSpec {
        hero: "keeper_of_the_light".to_string(),
        author_namespaces: vec!["darkness".to_string()],
        shared_policy: SharedPolicy::Strip,
    };
    let report = skin_archive::analyze(&archive, &spec, &Limits::default())
        .expect("analysis")
        .report;
    let mut value: serde_json::Value = serde_json::from_str(MANIFESTS[0]).expect("json");
    value["id"] = format!("wardrobe.{id}").into();
    value["catalogId"] = format!("heroes-{id}").into();
    value["archive"]["innerBytes"] = archive.len().into();
    value["archive"]["innerSha256"] = sha256_hex(&archive).into();
    value["audited"] = serde_json::json!({
        "entries": report.entries,
        "archiveIdentity": report.archive_identity,
        "installFiles": report.install.files,
        "installBytes": report.install.bytes,
        "installIdentity": report.install.identity,
        "strippedShared": report.install.stripped_shared,
        "rejected": report.rejected.files,
    });
    edit(&mut value);
    let manifest: WardrobeManifest = serde_json::from_value(value).expect("manifest");
    validate(&manifest).expect("valid");
    (manifest, archive)
}

/// Whether the catalog ID belongs to the wardrobe allowlist; a signed package
/// catalog may not reuse it.
pub(crate) fn catalog_id_taken(catalog_id: &str) -> bool {
    manifests().is_ok_and(|all| all.iter().any(|manifest| manifest.catalog_id == catalog_id))
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
/// verifying the pinned ZIP when it is not. Only the verified inner VPK is
/// stored.
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
    let zip = crate::remote_intake::fetch_pinned_wardrobe_archive(manifest, cancelled)?;
    let inner = extract_pinned_entry(&zip, &pin.inner_path, pin.inner_bytes, &pin.inner_sha256)?;
    content_store::store_pinned_resource(app_data_root, pin.inner_bytes, &pin.inner_sha256, &inner)
}

/// The analyzer must reproduce the audit, field for field.
fn matches_audit(manifest: &WardrobeManifest, report: &SkinReport) -> bool {
    let audit = &manifest.audited;
    report.entries == audit.entries
        && report.archive_identity == audit.archive_identity
        && report.install.files == audit.install_files
        && report.install.bytes == audit.install_bytes
        && report.install.identity == audit.install_identity
        && report.install.stripped_shared == audit.stripped_shared
        && report.rejected.files == audit.rejected
}

/// Reads the item's VPK back from the content store (size and SHA-256 are
/// checked again), analyzes it and returns the files to install.
pub(crate) fn load(
    app_data_root: &Path,
    manifest: &WardrobeManifest,
) -> Result<LoadedWardrobe, String> {
    let pin = &manifest.archive;
    let archive =
        content_store::read_pinned_resource(app_data_root, pin.inner_bytes, &pin.inner_sha256)?;
    let selection =
        skin_archive::select_install_files(&archive, &manifest.scope_spec(), &Limits::default())?;
    if !matches_audit(manifest, &selection.report) {
        return Err("wardrobe_audit_mismatch".to_string());
    }
    Ok(LoadedWardrobe {
        resources: selection.files,
        item: WardrobePlanItem {
            package_id: manifest.id.clone(),
            catalog_id: manifest.catalog_id.clone(),
            hero: manifest.hero.clone(),
            source_url: manifest.source.url.clone(),
            archive_sha256: pin.zip_sha256.clone(),
            catalog_snapshot: manifest.source.catalog_snapshot.clone(),
            permission: manifest.source.permission.clone(),
            verified_languages: manifest.verified_languages.clone(),
            compatibility_note: manifest.compatibility_note.clone(),
            report: selection.report,
        },
    })
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

    fn manifest_with(edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut value: serde_json::Value = serde_json::from_str(MANIFESTS[0]).expect("json");
        edit(&mut value);
        value.to_string()
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
        assert_eq!(all.len(), 1);
        let item = scarlet();
        assert_eq!(item.id, "wardrobe.scarlet-keeper-of-the-light");
        assert_eq!(item.hero, "keeper_of_the_light");
        assert_eq!(
            item.archive.zip_sha256,
            "f9f0c2f63a9989ea6fd2779cfea1676a2e07bd94eabab9e0c2127a68e7e268a7"
        );
        assert_eq!(item.archive.zip_bytes, 24_071_988);
        assert_eq!(item.archive.inner_bytes, 58_096_981);
        assert_eq!(item.shared_policy, SharedPolicy::Strip);
        assert_eq!(
            item.source.url,
            "https://huggingface.co/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/heroes/Scarlet%20Keeper.zip"
        );
        assert!(find("wardrobe.scarlet-keeper-of-the-light")
            .expect("registry")
            .is_some());
        assert!(find("minify-tree-mod").expect("registry").is_none());
        assert!(catalog_id_taken("heroes-scarlet-keeper-of-the-light"));
        assert!(!catalog_id_taken("minify-tree-mod"));
    }

    #[test]
    fn every_wardrobe_file_is_embedded() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wardrobe");
        let files = std::fs::read_dir(dir)
            .expect("wardrobe dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
            .count();
        assert_eq!(files, MANIFESTS.len(), "add the new manifest to MANIFESTS");
    }

    #[test]
    fn hostile_or_malformed_entries_are_rejected() {
        assert!(parse_all(&[MANIFESTS[0]]).is_ok());
        let cases = [
            manifest_with(|v| v["kind"] = "package".into()),
            manifest_with(|v| v["schemaVersion"] = 2.into()),
            manifest_with(|v| v["distribution"] = "public".into()),
            manifest_with(|v| v["id"] = "minify.scarlet".into()),
            manifest_with(|v| v["id"] = "wardrobe.Scarlet".into()),
            manifest_with(|v| v["catalogId"] = "scarlet".into()),
            manifest_with(|v| v["hero"] = "Keeper".into()),
            manifest_with(|v| v["authorNamespaces"] = serde_json::json!(["units"])),
            manifest_with(|v| v["authorNamespaces"] = serde_json::json!(["particles"])),
            manifest_with(|v| {
                v["source"]["url"] = "http://huggingface.co/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/heroes/a.zip".into()
            }),
            manifest_with(|v| {
                v["source"]["url"] = "https://huggingface.co.evil.example/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/heroes/a.zip".into()
            }),
            manifest_with(|v| {
                v["source"]["url"] = "https://huggingface.co/datasets/other/Repo/resolve/main/assets/files/heroes/a.zip".into()
            }),
            manifest_with(|v| {
                v["source"]["url"] = "https://huggingface.co/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/../../x.zip".into()
            }),
            manifest_with(|v| {
                v["source"]["url"] = "https://huggingface.co/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/heroes/a.zip?download=1".into()
            }),
            manifest_with(|v| {
                v["source"]["url"] = "https://huggingface.co/datasets/hrdq/Dota2PornFx/resolve/main/assets/files/heroes/a.exe".into()
            }),
            manifest_with(|v| v["archive"]["zipSha256"] = "00".into()),
            manifest_with(|v| v["archive"]["innerSha256"] = "A".repeat(64).into()),
            manifest_with(|v| v["archive"]["innerPath"] = "../pak01_dir.vpk".into()),
            manifest_with(|v| v["archive"]["innerPath"] = "payload.exe".into()),
            manifest_with(|v| v["archive"]["innerBytes"] = 70_000_000.into()),
            manifest_with(|v| v["archive"]["zipBytes"] = 0.into()),
            manifest_with(|v| v["audited"]["installFiles"] = 0.into()),
            manifest_with(|v| v["audited"]["installFiles"] = 2000.into()),
            manifest_with(|v| v["audited"]["installIdentity"] = "abc".into()),
            manifest_with(|v| v["script"] = "rm -rf".into()),
        ];
        for case in &cases {
            assert!(parse_all(&[case.as_str()]).is_err(), "accepted: {case}");
        }
        assert!(
            parse_all(&[MANIFESTS[0], MANIFESTS[0]]).is_err(),
            "duplicates"
        );
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
                ("particles/basic_ambient/basic_ambient.vpcf_c", b"shared"),
                ("materials/default/default_color_tga_1.vtex_c", b"shared2"),
                ("particles/darkness/flame.vpcf", b"source"),
            ],
            edit,
        )
    }

    #[test]
    fn loads_only_what_the_audit_describes() {
        let (manifest, archive) = synthetic(|_| {});
        let root = temp_root("load");
        // Nothing stored yet: the build cannot start from thin air.
        assert!(load(&root, &manifest).is_err());
        content_store::store_pinned_resource(
            &root,
            manifest.archive.inner_bytes,
            &manifest.archive.inner_sha256,
            &archive,
        )
        .expect("store");
        let loaded = load(&root, &manifest).expect("loads");
        assert_eq!(loaded.resources.len(), 4);
        assert_eq!(loaded.item.report.install.stripped_shared, 2);
        assert_eq!(loaded.item.report.rejected.files, 1);
        assert_eq!(loaded.item.package_id, manifest.id);
        assert!(!loaded.resources.keys().any(|path| path.contains("basic_")));
        assert_eq!(
            loaded.resources["models/heroes/keeper_of_the_light/kotl_hood.vmdl_c"],
            b"hood"
        );

        // Any drift between the audit and the analyzer refuses the build.
        for edit in [
            |v: &mut serde_json::Value| v["audited"]["installIdentity"] = "1".repeat(64).into(),
            |v: &mut serde_json::Value| v["audited"]["installFiles"] = 3.into(),
            |v: &mut serde_json::Value| v["audited"]["archiveIdentity"] = "2".repeat(64).into(),
            |v: &mut serde_json::Value| v["audited"]["strippedShared"] = 1.into(),
            // A different declared hero re-classifies the same archive.
            |v: &mut serde_json::Value| v["hero"] = "axe".into(),
        ] {
            let (drifted, _) = synthetic(edit);
            let root = temp_root("drift");
            content_store::store_pinned_resource(
                &root,
                drifted.archive.inner_bytes,
                &drifted.archive.inner_sha256,
                &archive,
            )
            .expect("store");
            assert_eq!(
                load(&root, &drifted).err().as_deref(),
                Some("wardrobe_audit_mismatch")
            );
            std::fs::remove_dir_all(root).expect("cleanup");
        }
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_tampered_stored_archive_is_never_analyzed() {
        let (manifest, archive) = synthetic(|_| {});
        let root = temp_root("tamper");
        content_store::store_pinned_resource(
            &root,
            manifest.archive.inner_bytes,
            &manifest.archive.inner_sha256,
            &archive,
        )
        .expect("store");
        let object = root
            .join("content-v1")
            .join("objects")
            .join("sha256")
            .join(&manifest.archive.inner_sha256);
        let mut bytes = std::fs::read(&object).expect("object");
        bytes[0] ^= 0xff;
        std::fs::write(&object, bytes).expect("tamper");
        assert_eq!(
            load(&root, &manifest).err().as_deref(),
            Some("content_store_corrupt")
        );
        std::fs::remove_dir_all(root).expect("cleanup");
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
        let loaded = load(&root, item).expect("the analyzer reproduces the audit");
        assert_eq!(loaded.resources.len(), item.audited.install_files);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}

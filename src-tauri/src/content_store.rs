use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const CONTENT_SCHEMA_VERSION: u32 = 1;
const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RELATIONS: usize = 64;
static PUBLISH_COUNTER: AtomicU64 = AtomicU64::new(0);
static CONTENT_STORE_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LocalizedText {
    ru: String,
    en: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ArtifactManifest {
    format: String,
    file_name: String,
    media_type: String,
    size: u64,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PackageManifest {
    schema_version: u32,
    id: String,
    version: String,
    name: LocalizedText,
    description: LocalizedText,
    category: String,
    resource_type: String,
    author: String,
    source: String,
    license_status: String,
    trust_rationale: String,
    artifact: ArtifactManifest,
    dependencies: Vec<String>,
    incompatibilities: Vec<String>,
    recipe_version: u32,
    dota_compatibility: String,
    last_verification: String,
    signature_status: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentIntakeRequest {
    pub package_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentReceipt {
    package_id: String,
    version: String,
    content_identity: String,
    size: u64,
    already_present: bool,
    signature_status: String,
    compatibility: String,
}

#[derive(Clone, Debug)]
pub struct ContentDiagnosticCounts {
    pub verified_packages: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FixturePackageContract {
    pub version: String,
    pub file_name: String,
    pub size: u64,
    pub sha256: String,
    pub recipe_version: u32,
}

fn fixture_manifest_source(id: &str) -> Option<&'static str> {
    match id {
        "fixture.ambient-violet" => Some(include_str!(
            "../fixtures/packages/ambient-violet.package.json"
        )),
        "fixture.ambient-clean" => Some(include_str!(
            "../fixtures/packages/ambient-clean.package.json"
        )),
        _ => None,
    }
}

fn fixture_payload(id: &str) -> Option<&'static [u8]> {
    match id {
        "fixture.ambient-violet" => Some(include_bytes!("../fixtures/payloads/ambient-violet.css")),
        "fixture.ambient-clean" => Some(include_bytes!("../fixtures/payloads/ambient-clean.css")),
        _ => None,
    }
}

fn fixture_ids() -> [&'static str; 2] {
    ["fixture.ambient-clean", "fixture.ambient-violet"]
}

pub(crate) fn fixture_package_contract(package_id: &str) -> Result<FixturePackageContract, String> {
    let source =
        fixture_manifest_source(package_id).ok_or_else(|| "content_package_unknown".to_string())?;
    let manifest = validate_manifest(source)?;
    if manifest.id != package_id {
        return Err("content_manifest_invalid".to_string());
    }
    Ok(FixturePackageContract {
        version: manifest.version,
        file_name: manifest.artifact.file_name,
        size: manifest.artifact.size,
        sha256: manifest.artifact.sha256,
        recipe_version: manifest.recipe_version,
    })
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_identifier(value: &str) -> bool {
    (3..=96).contains(&value.len())
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'.' || byte == b'-'
        })
        && !value.contains("..")
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && value
            .bytes()
            .last()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}

fn valid_version(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 || !value.is_ascii() {
        return false;
    }
    let core = value.split_once('-').map_or(value, |(core, _)| core);
    let components = core.split('.').collect::<Vec<_>>();
    components.len() == 3
        && components
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
}

fn valid_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

fn valid_leaf_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && !value.contains(['/', '\\', ':'])
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn validate_relations(values: &mut [String], own_id: &str) -> Result<(), String> {
    if values.len() > MAX_RELATIONS {
        return Err("content_manifest_invalid".to_string());
    }
    if values
        .iter()
        .any(|value| !valid_identifier(value) || value == own_id)
    {
        return Err("content_manifest_invalid".to_string());
    }
    values.sort();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("content_manifest_invalid".to_string());
    }
    Ok(())
}

fn validate_manifest(contents: &str) -> Result<PackageManifest, String> {
    if contents.len() > MAX_MANIFEST_BYTES {
        return Err("content_manifest_invalid".to_string());
    }
    let mut manifest: PackageManifest =
        serde_json::from_str(contents).map_err(|_| "content_manifest_invalid".to_string())?;
    if manifest.schema_version != CONTENT_SCHEMA_VERSION
        || !valid_identifier(&manifest.id)
        || !valid_version(&manifest.version)
        || !valid_text(&manifest.name.ru, 120)
        || !valid_text(&manifest.name.en, 120)
        || !valid_text(&manifest.description.ru, 600)
        || !valid_text(&manifest.description.en, 600)
        || !matches!(
            manifest.category.as_str(),
            "optimization"
                | "map_world"
                | "hud_menu"
                | "audio"
                | "customization"
                | "utilities"
                | "wardrobe"
        )
        || !matches!(
            manifest.resource_type.as_str(),
            "panorama_style" | "audio" | "texture" | "model" | "vpk_fragment"
        )
        || !valid_text(&manifest.author, 160)
        || manifest.source.len() > 2048
        || !manifest.source.starts_with("https://")
        || manifest.source.chars().any(char::is_whitespace)
        || !matches!(
            manifest.license_status.as_str(),
            "unknown" | "permission_verified" | "upstream_license" | "repository_fixture"
        )
        || !valid_text(&manifest.trust_rationale, 300)
        || manifest.artifact.format != "raw"
        || !valid_leaf_name(&manifest.artifact.file_name)
        || manifest.artifact.media_type != "text/css"
        || manifest.artifact.size == 0
        || manifest.artifact.size > MAX_ARTIFACT_BYTES
        || manifest.artifact.sha256.len() != 64
        || !manifest
            .artifact
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || manifest.recipe_version != 1
        || !matches!(
            manifest.dota_compatibility.as_str(),
            "unknown" | "verified" | "unsupported"
        )
        || !matches!(
            manifest.last_verification.as_str(),
            "unknown" | "repository_fixture"
        )
        || !matches!(
            manifest.signature_status.as_str(),
            "not_provided" | "unverified" | "verified"
        )
    {
        return Err("content_manifest_invalid".to_string());
    }
    validate_relations(&mut manifest.dependencies, &manifest.id)?;
    validate_relations(&mut manifest.incompatibilities, &manifest.id)?;
    Ok(manifest)
}

fn reject_symlink(path: &Path) -> Result<(), String> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err("content_store_invalid".to_string());
        }
    }
    Ok(())
}

fn content_paths(app_data_root: &Path) -> (PathBuf, PathBuf) {
    let root = app_data_root.join("content-v1");
    (root.join("objects").join("sha256"), root.join("manifests"))
}

fn prepare_roots(app_data_root: &Path) -> Result<(PathBuf, PathBuf), String> {
    reject_symlink(app_data_root)?;
    fs::create_dir_all(app_data_root).map_err(|_| "content_store_unavailable".to_string())?;
    reject_symlink(app_data_root)?;
    let content_root = app_data_root.join("content-v1");
    reject_symlink(&content_root)?;
    fs::create_dir(&content_root)
        .or_else(|error| {
            (error.kind() == ErrorKind::AlreadyExists)
                .then_some(())
                .ok_or(error)
        })
        .map_err(|_| "content_store_unavailable".to_string())?;
    reject_symlink(&content_root)?;
    let (objects, manifests) = content_paths(app_data_root);
    for path in [&objects, &manifests] {
        fs::create_dir_all(path).map_err(|_| "content_store_unavailable".to_string())?;
        reject_symlink(path)?;
    }
    Ok((objects, manifests))
}

fn temporary_path(parent: &Path, final_name: &str) -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "content_publish_failed".to_string())?
        .as_nanos();
    let counter = PUBLISH_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(parent.join(format!(
        ".{final_name}.{}.{}.tmp",
        std::process::id(),
        timestamp + u128::from(counter)
    )))
}

fn read_limited(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    reject_symlink(path)?;
    let metadata = fs::metadata(path).map_err(|_| "content_store_invalid".to_string())?;
    if !metadata.is_file() || metadata.len() > max {
        return Err("content_store_invalid".to_string());
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(|_| "content_store_invalid".to_string())?;
    Ok(bytes)
}

fn publish_noclobber(parent: &Path, final_name: &str, bytes: &[u8]) -> Result<bool, String> {
    publish_noclobber_with_failure(parent, final_name, bytes, false)
}

fn publish_noclobber_with_failure(
    parent: &Path,
    final_name: &str,
    bytes: &[u8],
    fail_after_temporary_verification: bool,
) -> Result<bool, String> {
    reject_symlink(parent)?;
    let destination = parent.join(final_name);
    reject_symlink(&destination)?;
    if destination.exists() {
        return (read_limited(
            &destination,
            MAX_ARTIFACT_BYTES.max(MAX_MANIFEST_BYTES as u64),
        )? == bytes)
            .then_some(true)
            .ok_or_else(|| "content_store_corrupt".to_string());
    }

    let temporary = temporary_path(parent, final_name)?;
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| "content_publish_failed".to_string())?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "content_publish_failed".to_string())?;
        if read_limited(
            &temporary,
            MAX_ARTIFACT_BYTES.max(MAX_MANIFEST_BYTES as u64),
        )? != bytes
        {
            return Err("content_verification_failed".to_string());
        }
        if fail_after_temporary_verification {
            return Err("injected_failure".to_string());
        }
        match fs::hard_link(&temporary, &destination) {
            Ok(()) => Ok(false),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                if read_limited(
                    &destination,
                    MAX_ARTIFACT_BYTES.max(MAX_MANIFEST_BYTES as u64),
                )? == bytes
                {
                    Ok(true)
                } else {
                    Err("content_store_corrupt".to_string())
                }
            }
            Err(_) => Err("content_publish_failed".to_string()),
        }
    })();
    let _ = fs::remove_file(&temporary);
    result
}

fn store_manifest_name(manifest: &PackageManifest) -> String {
    format!("{}@{}.json", manifest.id, manifest.version)
}

fn store_one(
    objects: &Path,
    manifests: &Path,
    manifest: &PackageManifest,
    payload: &[u8],
) -> Result<ContentReceipt, String> {
    store_one_with_failure(objects, manifests, manifest, payload, false)
}

fn store_one_with_failure(
    objects: &Path,
    manifests: &Path,
    manifest: &PackageManifest,
    payload: &[u8],
    fail_after_object_publication: bool,
) -> Result<ContentReceipt, String> {
    if payload.len() as u64 != manifest.artifact.size || sha256(payload) != manifest.artifact.sha256
    {
        return Err("content_hash_mismatch".to_string());
    }
    let object_reused = publish_noclobber(objects, &manifest.artifact.sha256, payload)?;
    if fail_after_object_publication {
        return Err("injected_failure".to_string());
    }
    let normalized =
        serde_json::to_vec_pretty(manifest).map_err(|_| "content_manifest_invalid".to_string())?;
    publish_noclobber(manifests, &store_manifest_name(manifest), &normalized)?;
    verify_stored_package(objects, manifests, manifest)?;
    Ok(ContentReceipt {
        package_id: manifest.id.clone(),
        version: manifest.version.clone(),
        content_identity: format!("sha256:{}", manifest.artifact.sha256),
        size: manifest.artifact.size,
        already_present: object_reused,
        signature_status: manifest.signature_status.clone(),
        compatibility: manifest.dota_compatibility.clone(),
    })
}

fn verify_stored_package(
    objects: &Path,
    manifests: &Path,
    expected: &PackageManifest,
) -> Result<(), String> {
    let record = read_limited(
        &manifests.join(store_manifest_name(expected)),
        MAX_MANIFEST_BYTES as u64,
    )?;
    let record_text =
        std::str::from_utf8(&record).map_err(|_| "content_store_invalid".to_string())?;
    let stored = validate_manifest(record_text)?;
    if stored.id != expected.id
        || stored.version != expected.version
        || stored.artifact.sha256 != expected.artifact.sha256
        || stored.artifact.size != expected.artifact.size
    {
        return Err("content_store_corrupt".to_string());
    }
    let payload = read_limited(&objects.join(&stored.artifact.sha256), MAX_ARTIFACT_BYTES)?;
    if payload.len() as u64 != stored.artifact.size || sha256(&payload) != stored.artifact.sha256 {
        return Err("content_store_corrupt".to_string());
    }
    Ok(())
}

pub fn intake_fixture_content(
    app_data_root: &Path,
    request: ContentIntakeRequest,
) -> Result<Vec<ContentReceipt>, String> {
    let requested = request
        .package_ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect::<BTreeSet<_>>();
    if requested.is_empty() || requested.len() > MAX_RELATIONS {
        return Err("content_request_invalid".to_string());
    }
    let mutex = CONTENT_STORE_MUTEX.get_or_init(|| Mutex::new(()));
    let _guard = mutex
        .lock()
        .map_err(|_| "content_store_unavailable".to_string())?;
    let (objects, manifests) = prepare_roots(app_data_root)?;
    let mut receipts = Vec::with_capacity(requested.len());
    for id in requested {
        let source =
            fixture_manifest_source(&id).ok_or_else(|| "content_package_unknown".to_string())?;
        let manifest = validate_manifest(source)?;
        if manifest.id != id {
            return Err("content_manifest_invalid".to_string());
        }
        let payload = fixture_payload(&id).ok_or_else(|| "content_package_unknown".to_string())?;
        receipts.push(store_one(&objects, &manifests, &manifest, payload)?);
    }
    Ok(receipts)
}

pub fn read_verified_fixture_artifact(
    app_data_root: &Path,
    package_id: &str,
) -> Result<Vec<u8>, String> {
    let source =
        fixture_manifest_source(package_id).ok_or_else(|| "content_package_unknown".to_string())?;
    let manifest = validate_manifest(source)?;
    let (objects, manifests) = prepare_roots(app_data_root)?;
    verify_stored_package(&objects, &manifests, &manifest)?;
    read_limited(&objects.join(manifest.artifact.sha256), MAX_ARTIFACT_BYTES)
}

pub fn content_diagnostic_counts(app_data_root: &Path) -> Result<ContentDiagnosticCounts, String> {
    let (objects, manifests) = prepare_roots(app_data_root)?;
    let mut verified_packages = 0;
    for id in fixture_ids() {
        let source =
            fixture_manifest_source(id).ok_or_else(|| "content_store_invalid".to_string())?;
        let manifest = validate_manifest(source)?;
        let record = manifests.join(store_manifest_name(&manifest));
        if !record.exists() {
            continue;
        }
        verify_stored_package(&objects, &manifests, &manifest)?;
        verified_packages += 1;
    }
    Ok(ContentDiagnosticCounts { verified_packages })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "betterfy-content-{name}-{}-{}",
            std::process::id(),
            PUBLISH_COUNTER.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn one_package() -> ContentIntakeRequest {
        ContentIntakeRequest {
            package_ids: vec!["fixture.ambient-violet".to_string()],
        }
    }

    #[test]
    fn validates_and_normalizes_the_versioned_manifest() {
        let manifest = validate_manifest(
            fixture_manifest_source("fixture.ambient-violet").expect("fixture manifest"),
        )
        .expect("valid manifest");
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.artifact.sha256.len(), 64);
        assert_eq!(manifest.signature_status, "not_provided");
        assert_eq!(manifest.dota_compatibility, "unknown");
    }

    #[test]
    fn exposes_a_validated_recipe_contract() {
        let contract = fixture_package_contract("fixture.ambient-violet").expect("contract");
        assert_eq!(contract.version, "1.0.0");
        assert_eq!(contract.file_name, "ambient-violet.css");
        assert_eq!(contract.size, 61);
        assert_eq!(contract.sha256.len(), 64);
        assert_eq!(contract.recipe_version, 1);
    }

    #[test]
    fn rejects_unknown_fields_bad_hashes_and_unsafe_sources() {
        let source = fixture_manifest_source("fixture.ambient-violet").expect("fixture manifest");
        let with_unknown = source.replacen("{", "{\n  \"unexpected\": true,", 1);
        assert_eq!(
            validate_manifest(&with_unknown).err().as_deref(),
            Some("content_manifest_invalid")
        );
        let bad_hash = source.replace(
            "e2c06353f3a5c99162512e40c6a2e318778d1c73bc0ba79126df5a4beff13c64",
            "E2C06353F3A5C99162512E40C6A2E318778D1C73BC0BA79126DF5A4BEFF13C64",
        );
        assert_eq!(
            validate_manifest(&bad_hash).err().as_deref(),
            Some("content_manifest_invalid")
        );
        let unsafe_source = source.replace("https://github.com", "file:///tmp");
        assert_eq!(
            validate_manifest(&unsafe_source).err().as_deref(),
            Some("content_manifest_invalid")
        );
        let unsafe_name = source.replace("ambient-violet.css", "../ambient-violet.css");
        assert_eq!(
            validate_manifest(&unsafe_name).err().as_deref(),
            Some("content_manifest_invalid")
        );
    }

    #[test]
    fn valid_but_substituted_hash_never_reaches_the_store() {
        let root = temp_root("substituted-hash");
        let source = fixture_manifest_source("fixture.ambient-violet").expect("fixture manifest");
        let substituted = source.replace(
            "e2c06353f3a5c99162512e40c6a2e318778d1c73bc0ba79126df5a4beff13c64",
            "02c06353f3a5c99162512e40c6a2e318778d1c73bc0ba79126df5a4beff13c64",
        );
        let manifest = validate_manifest(&substituted).expect("structurally valid manifest");
        let (objects, manifests) = prepare_roots(&root).expect("roots");
        assert_eq!(
            store_one(
                &objects,
                &manifests,
                &manifest,
                fixture_payload("fixture.ambient-violet").expect("fixture payload"),
            )
            .err()
            .as_deref(),
            Some("content_hash_mismatch")
        );
        assert!(fs::read_dir(objects).expect("objects").next().is_none());
        assert!(fs::read_dir(manifests).expect("manifests").next().is_none());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn publishes_once_and_reuses_the_verified_identity() {
        let root = temp_root("idempotent");
        let first = intake_fixture_content(&root, one_package()).expect("first intake");
        let second = intake_fixture_content(&root, one_package()).expect("second intake");
        assert!(!first[0].already_present);
        assert!(second[0].already_present);
        assert_eq!(first[0].content_identity, second[0].content_identity);
        assert_eq!(
            read_verified_fixture_artifact(&root, "fixture.ambient-violet")
                .expect("stored artifact"),
            fixture_payload("fixture.ambient-violet").expect("fixture payload")
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn failure_before_publication_leaves_no_final_object() {
        let root = temp_root("pre-publish-failure");
        let (objects, _) = prepare_roots(&root).expect("roots");
        let hash = "e2c06353f3a5c99162512e40c6a2e318778d1c73bc0ba79126df5a4beff13c64";
        assert_eq!(
            publish_noclobber_with_failure(
                &objects,
                hash,
                fixture_payload("fixture.ambient-violet").expect("fixture payload"),
                true,
            )
            .err()
            .as_deref(),
            Some("injected_failure")
        );
        assert!(!objects.join(hash).exists());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn retry_completes_a_manifest_after_object_publication() {
        let root = temp_root("post-object-failure");
        let (objects, manifests) = prepare_roots(&root).expect("roots");
        let manifest = validate_manifest(
            fixture_manifest_source("fixture.ambient-violet").expect("fixture manifest"),
        )
        .expect("valid manifest");
        let payload = fixture_payload("fixture.ambient-violet").expect("fixture payload");
        assert_eq!(
            store_one_with_failure(&objects, &manifests, &manifest, payload, true)
                .err()
                .as_deref(),
            Some("injected_failure")
        );
        assert!(objects.join(&manifest.artifact.sha256).exists());
        assert!(!manifests.join(store_manifest_name(&manifest)).exists());
        let receipt = intake_fixture_content(&root, one_package()).expect("retry");
        assert!(receipt[0].already_present);
        verify_stored_package(&objects, &manifests, &manifest).expect("verified after retry");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn rejects_unknown_packages_without_publishing() {
        let root = temp_root("unknown");
        assert_eq!(
            intake_fixture_content(
                &root,
                ContentIntakeRequest {
                    package_ids: vec!["fixture.not-allowed".to_string()],
                },
            )
            .err()
            .as_deref(),
            Some("content_package_unknown")
        );
        let counts = content_diagnostic_counts(&root).expect("diagnostics");
        assert_eq!(counts.verified_packages, 0);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn detects_tampering_instead_of_replacing_the_object() {
        let root = temp_root("tamper");
        intake_fixture_content(&root, one_package()).expect("intake");
        let manifest = validate_manifest(
            fixture_manifest_source("fixture.ambient-violet").expect("fixture manifest"),
        )
        .expect("valid manifest");
        let (objects, _) = content_paths(&root);
        fs::write(objects.join(&manifest.artifact.sha256), b"tampered").expect("tamper object");
        assert_eq!(
            intake_fixture_content(&root, one_package())
                .err()
                .as_deref(),
            Some("content_store_corrupt")
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_content_roots() {
        use std::os::unix::fs::symlink;

        let root = temp_root("symlink");
        fs::create_dir_all(root.join("content-v1").join("objects")).expect("parents");
        symlink(
            std::env::temp_dir(),
            root.join("content-v1").join("objects").join("sha256"),
        )
        .expect("symlink");
        assert_eq!(
            intake_fixture_content(&root, one_package())
                .err()
                .as_deref(),
            Some("content_store_invalid")
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlinked_content_root_component() {
        use std::os::unix::fs::symlink;

        let root = temp_root("symlink-content-root");
        let outside = temp_root("symlink-content-outside");
        fs::create_dir_all(&root).expect("root");
        fs::create_dir_all(&outside).expect("outside");
        symlink(&outside, root.join("content-v1")).expect("symlink");
        assert_eq!(
            intake_fixture_content(&root, one_package())
                .err()
                .as_deref(),
            Some("content_store_invalid")
        );
        fs::remove_file(root.join("content-v1")).expect("remove symlink");
        fs::remove_dir_all(root).expect("cleanup root");
        fs::remove_dir_all(outside).expect("cleanup outside");
    }
}

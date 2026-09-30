//! Download, verification and bundling for the pinned, data-only packages
//! described by `package_registry` manifests.

use crate::mod_bundle::{
    self, BundleContribution, BundleDuplicate, BundleOverride, BundlePackage, BundlePlan,
};
use crate::package_registry::{self, PackageManifest, Resource};
use crate::vpk;
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

fn download_slot() -> &'static Mutex<Option<Arc<TreeDownload>>> {
    DOWNLOAD.get_or_init(|| Mutex::new(None))
}

/// The first pilot package. Older journals written before bundles recorded
/// their package list are read as this single package.
pub(crate) const PACKAGE_ID: &str = "minify.tree-mod";
pub(crate) const TARGET_FILE: &str = "pak66_dir.vpk";
/// Upper bound on packages merged into one VPK.
const MAX_BUNDLE_PACKAGES: usize = 16;

fn normalize_package_ids(ids: &[String]) -> Result<Vec<String>, String> {
    if ids.is_empty() || ids.len() > MAX_BUNDLE_PACKAGES {
        return Err("pilot_package_unsupported".to_string());
    }
    let mut seen = BTreeSet::new();
    ids.iter()
        .map(|id| {
            let id = package_registry::find(id)?.id.clone();
            if !seen.insert(id.clone()) {
                return Err("pilot_package_invalid".to_string());
            }
            Ok(id)
        })
        .collect()
}

fn contract(id: &str) -> Result<&'static PackageManifest, String> {
    package_registry::find(id)
}

pub(crate) struct VerifiedTreeVpk {
    bytes: Vec<u8>,
    sha256: String,
    bundle_plan: BundlePlan,
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
    }
}

/// The only URL a pilot resource may be downloaded from: the manifest's
/// pinned commit, for a resource that is exactly in that manifest.
pub(crate) fn pinned_url(package_id: &str, resource: &Resource) -> Result<String, String> {
    let manifest = contract(package_id)?;
    if !manifest.resources.iter().any(|expected| {
        expected.path == resource.path
            && expected.bytes == resource.bytes
            && expected.sha256 == resource.sha256
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
    let manifests = package_ids
        .iter()
        .map(|id| contract(id))
        .collect::<Result<Vec<_>, _>>()?;
    let mut index = 0usize;
    for (package_id, manifest) in package_ids.iter().zip(manifests) {
        for resource in &manifest.resources {
            if cancelled.load(Ordering::Relaxed) {
                return Err("download_cancelled".to_string());
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
) -> Result<TreeDownloadStatus, String> {
    let package_ids = normalize_package_ids(&requested_package_ids)?;
    let total_resources = package_ids.iter().try_fold(0usize, |total, id| {
        contract(id).map(|manifest| total + manifest.resources.len())
    })?;
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
    let packages = package_ids
        .iter()
        .map(|package_id| {
            let manifest = contract(package_id)?;
            let resources = manifest
                .resources
                .iter()
                .map(|resource| {
                    crate::content_store::read_pinned_resource(
                        app_data_root,
                        resource.bytes,
                        &resource.sha256,
                    )
                    .map(|bytes| (resource.path.clone(), bytes))
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?;
            verify_resources(&manifest.resources, &resources)?;
            Ok(BundlePackage {
                package_id: package_id.clone(),
                resources,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    build_verified_bundle(packages)
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
    build_verified_bundle(vec![BundlePackage {
        package_id: PACKAGE_ID.to_string(),
        resources: resources.clone(),
    }])
}

fn build_verified_bundle(packages: Vec<BundlePackage>) -> Result<VerifiedTreeVpk, String> {
    let bundle = mod_bundle::build(packages)?;
    let report = vpk::inspect(bundle.bytes())?;
    if report.entries != bundle.plan().resource_count {
        return Err("tree_vpk_invalid".to_string());
    }
    Ok(VerifiedTreeVpk {
        bytes: bundle.bytes().to_vec(),
        sha256: bundle.sha256().to_string(),
        bundle_plan: bundle.plan().clone(),
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
        begin_download(root.clone(), vec![PACKAGE_ID.to_string()]).expect("start pinned download");
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
}

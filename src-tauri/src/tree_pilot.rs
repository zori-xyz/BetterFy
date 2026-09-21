//! Pinned, data-only contract for the first engine pilot.

use crate::vpk::{self, VpkInput};
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
}

fn download_slot() -> &'static Mutex<Option<Arc<TreeDownload>>> {
    DOWNLOAD.get_or_init(|| Mutex::new(None))
}

pub(crate) const PACKAGE_ID: &str = "minify.tree-mod";
pub(crate) const SOURCE_COMMIT: &str = "3a85572029f2c264e2a17cee1c9b54ce93e4fd93";
pub(crate) const TARGET_FILE: &str = "pak66_dir.vpk";

#[derive(Clone, Copy)]
pub struct Resource {
    pub path: &'static str,
    pub bytes: usize,
    pub sha256: &'static str,
}

pub const RESOURCES: [Resource; 21] = [
    Resource {
        path: "materials/default/default_color_tga_41192599.vtex_c",
        bytes: 2184,
        sha256: "31b8213992d35927c009f82a6dc25104e90179f1285317ccad4f81a78d90f247",
    },
    Resource {
        path: "materials/default/default_refl_tga_250508db.vtex_c",
        bytes: 2280,
        sha256: "f0456410fba5bf070fcb760ee2e3c8dde67748b229910e329f5627ca8f61162d",
    },
    Resource {
        path: "materials/tree_topiary.vmat_c",
        bytes: 3541,
        sha256: "4984a99ad0b98966c09fb30f3387d6e229c0011f0e17a2d2b721aef8ec6be3a2",
    },
    Resource {
        path: "materials/tree_topiary_block.vmat_c",
        bytes: 2756,
        sha256: "14fb25a924bfaaf9a422067348e3c83321530ab4433eaf8d56540731f2832e6c",
    },
    Resource {
        path: "materials/tree_topiary_normals_png_b25ef11b.vtex_c",
        bytes: 176820,
        sha256: "ed25236d789cb2c67b25055a0e1475dcb10d67e909e5bbc66720664180fa09e5",
    },
    Resource {
        path: "materials/tree_topiary_texture_png_6834bd45.vtex_c",
        bytes: 176836,
        sha256: "f53063c0d5d45a0f3d95650abbe9a0598704c708cc963358a56920ad784333c7",
    },
    Resource {
        path: "models/props_tree/dire_tree004.vmdl_c",
        bytes: 16692,
        sha256: "e38d311f65638ca693398102f0a6cf6b882c5c63d83757309949d57338180354",
    },
    Resource {
        path: "models/props_tree/dire_tree004b.vmdl_c",
        bytes: 16709,
        sha256: "4deef81ce074c7bab7065fbc093dcb6d0246750912faf1119af07c41a20bb59f",
    },
    Resource {
        path: "models/props_tree/dire_tree007.vmdl_c",
        bytes: 16708,
        sha256: "8a0a066202a1a47187958c10d473a22c52b87421c4650c3b466414e8c8b23c0f",
    },
    Resource {
        path: "models/props_tree/dire_tree008.vmdl_c",
        bytes: 16692,
        sha256: "65c3d58ca27c8e3446b3c2b8c9dd5da273eca0d835f870e7ed52e867a4bffd27",
    },
    Resource {
        path: "models/props_tree/tree_bamboo_01.vmdl_c",
        bytes: 16584,
        sha256: "74e6787b8eb7a4ce84c0b47accb34cbd62019b83965f5e66b00631305faeead1",
    },
    Resource {
        path: "models/props_tree/tree_bamboo_02.vmdl_c",
        bytes: 16712,
        sha256: "6621e86081c76356b580155ec6160a37fb9e567e7021030f78a03423c3caab73",
    },
    Resource {
        path: "models/props_tree/tree_cine_00_low.vmdl_c",
        bytes: 16714,
        sha256: "b603bb945fee943ddc9735fd1165f747ae9cebd95e18041325c7d5d9a533dd00",
    },
    Resource {
        path: "models/props_tree/tree_cine_02_low.vmdl_c",
        bytes: 16682,
        sha256: "2637581f1bf9830893540668e36722f6e7604c2ac6c8180d8cb61d9ba414d904",
    },
    Resource {
        path: "models/props_tree/tree_oak_01.vmdl_c",
        bytes: 16581,
        sha256: "17dedac072ef3f5b2ef68f5c9629046d3bd81cf41a50228f972b71fcb8a53a58",
    },
    Resource {
        path: "models/props_tree/tree_oak_01b.vmdl_c",
        bytes: 16582,
        sha256: "cdc510a3511b5011f9687ad1dd8588e9c2d74fd91b6b9fbc877b5728bf1a0351",
    },
    Resource {
        path: "models/props_tree/tree_oak_02.vmdl_c",
        bytes: 16581,
        sha256: "51afa0d18c83ef9581599efeaefadbc4b03cfee29849adee8aa7460c398ad7d1",
    },
    Resource {
        path: "models/props_tree/tree_pine_01.vmdl_c",
        bytes: 16710,
        sha256: "9c9a60aaf2ee340d7568f1dcd86b45f60b452d105f8a1f4164caac19c9e6b953",
    },
    Resource {
        path: "models/props_tree/tree_pine_02.vmdl_c",
        bytes: 16710,
        sha256: "5882bcc219dcce99549bcfaf398f6704eb58f682efe1b017855cafeed42498e7",
    },
    Resource {
        path: "models/props_tree/tree_pine_03b.vmdl_c",
        bytes: 16711,
        sha256: "5ff016bd90a915389d4d6742a88d62691b10d9d47dc974b1ed4b3f11d47cc35c",
    },
    Resource {
        path: "models/props_tree/tree_pine_03b_sfm.vmdl_c",
        bytes: 16711,
        sha256: "5ff016bd90a915389d4d6742a88d62691b10d9d47dc974b1ed4b3f11d47cc35c",
    },
];

pub(crate) struct VerifiedTreeVpk(Vec<u8>);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TreePilotPlan {
    plan_id: String,
    package_id: &'static str,
    source_commit: &'static str,
    target_file: &'static str,
    resource_count: usize,
    resource_bytes: usize,
    vpk_bytes: usize,
    vpk_sha256: String,
    compatibility: &'static str,
    distribution: &'static str,
    deploy_enabled: bool,
}

impl VerifiedTreeVpk {
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.0
    }
}

pub(crate) fn pinned_url(resource: &Resource) -> String {
    format!(
        "https://raw.githubusercontent.com/Egezenn/dota2-minify/{SOURCE_COMMIT}/Minify/mods/Tree%20Mod/files/{}",
        resource.path
    )
}

#[cfg(test)]
pub(crate) fn acquire_verified_resources(
    app_data_root: &Path,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    acquire_verified_resources_with_progress(app_data_root, cancelled, |_| {})
}

fn acquire_verified_resources_with_progress(
    app_data_root: &Path,
    cancelled: &AtomicBool,
    mut progress: impl FnMut(usize),
) -> Result<(), String> {
    for (index, resource) in RESOURCES.iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            return Err("download_cancelled".to_string());
        }
        if crate::content_store::read_pinned_resource(
            app_data_root,
            resource.bytes,
            resource.sha256,
        )
        .is_ok()
        {
            progress(index + 1);
            continue;
        }
        let bytes = crate::remote_intake::fetch_pinned_tree_resource(resource, cancelled)?;
        if cancelled.load(Ordering::Relaxed) {
            return Err("download_cancelled".to_string());
        }
        crate::content_store::store_pinned_resource(
            app_data_root,
            resource.bytes,
            resource.sha256,
            &bytes,
        )?;
        progress(index + 1);
    }
    Ok(())
}

pub(crate) fn begin_download(
    app_data_root: std::path::PathBuf,
) -> Result<TreeDownloadStatus, String> {
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
            || (status.phase == "ready" && plan_from_verified_store(&app_data_root).is_ok())
        {
            return Ok(status);
        }
    }
    let operation = Arc::new(TreeDownload {
        status: Mutex::new(TreeDownloadStatus {
            phase: "downloading",
            verified_resources: 0,
            total_resources: RESOURCES.len(),
            error_code: None,
            plan: None,
        }),
        cancelled: AtomicBool::new(false),
        app_data_root: app_data_root.clone(),
    });
    *slot = Some(operation.clone());
    std::thread::spawn(move || {
        let result = acquire_verified_resources_with_progress(
            &app_data_root,
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
            plan_from_verified_store(&app_data_root)
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
        total_resources: RESOURCES.len(),
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
    if status.phase == "ready" && plan_from_verified_store(&operation.app_data_root).is_err() {
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

pub(crate) fn build_from_verified_store(app_data_root: &Path) -> Result<VerifiedTreeVpk, String> {
    let resources = RESOURCES
        .iter()
        .map(|resource| {
            crate::content_store::read_pinned_resource(
                app_data_root,
                resource.bytes,
                resource.sha256,
            )
            .map(|bytes| (resource.path.to_string(), bytes))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    build_verified_vpk(&resources)
}

fn plan_for(verified: &VerifiedTreeVpk) -> TreePilotPlan {
    let hash = format!("{:x}", Sha256::digest(verified.bytes()));
    TreePilotPlan {
        plan_id: format!("sha256:{hash}"),
        package_id: PACKAGE_ID,
        source_commit: SOURCE_COMMIT,
        target_file: TARGET_FILE,
        resource_count: RESOURCES.len(),
        resource_bytes: RESOURCES.iter().map(|resource| resource.bytes).sum(),
        vpk_bytes: verified.bytes().len(),
        vpk_sha256: hash,
        compatibility: "unknown",
        distribution: "internal_pilot",
        deploy_enabled: cfg!(target_os = "windows"),
    }
}

pub(crate) fn plan_from_verified_store(app_data_root: &Path) -> Result<TreePilotPlan, String> {
    build_from_verified_store(app_data_root).map(|verified| plan_for(&verified))
}

pub(crate) fn stage_from_verified_store(
    app_data_root: &Path,
    expected_plan_id: &str,
    confirmed: bool,
) -> Result<crate::build_engine::BuildReceipt, String> {
    if !confirmed {
        return Err("build_confirmation_required".to_string());
    }
    let verified = build_from_verified_store(app_data_root)?;
    crate::build_engine::stage_verified_vpk(app_data_root, &verified, expected_plan_id, confirmed)
}

fn verify_resources<'a>(
    contract: &[Resource],
    resources: &'a BTreeMap<String, Vec<u8>>,
) -> Result<Vec<VpkInput<'a>>, String> {
    if resources.len() != contract.len() {
        return Err("tree_resource_set_invalid".to_string());
    }
    let mut seen = BTreeSet::new();
    let mut inputs = Vec::with_capacity(contract.len());
    for expected in contract {
        if !seen.insert(expected.path.to_ascii_lowercase()) {
            return Err("tree_contract_invalid".to_string());
        }
        let bytes = resources
            .get(expected.path)
            .ok_or_else(|| "tree_resource_set_invalid".to_string())?;
        if bytes.len() != expected.bytes
            || format!("{:x}", Sha256::digest(bytes)) != expected.sha256
        {
            return Err("tree_resource_unverified".to_string());
        }
        inputs.push(VpkInput {
            path: expected.path,
            bytes,
        });
    }
    Ok(inputs)
}

pub(crate) fn build_verified_vpk(
    resources: &BTreeMap<String, Vec<u8>>,
) -> Result<VerifiedTreeVpk, String> {
    let inputs = verify_resources(&RESOURCES, resources)?;
    let vpk = vpk::build(inputs)?;
    let report = vpk::inspect(&vpk)?;
    if report.entries != RESOURCES.len() {
        return Err("tree_vpk_invalid".to_string());
    }
    Ok(VerifiedTreeVpk(vpk))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_is_exact_and_bounded() {
        assert_eq!(PACKAGE_ID, "minify.tree-mod");
        assert_eq!(SOURCE_COMMIT.len(), 40);
        assert_eq!(TARGET_FILE, "pak66_dir.vpk");
        assert_eq!(RESOURCES.len(), 21);
        assert!(RESOURCES.iter().map(|item| item.bytes).sum::<usize>() < 1024 * 1024);
        assert!(RESOURCES.iter().all(|item| item.sha256.len() == 64));
        let ledger = include_str!("../../docs/TREE_MOD_PILOT.md");
        for item in RESOURCES {
            assert!(ledger.contains(&format!(
                "| `{}` | {} | `{}` |",
                item.path, item.bytes, item.sha256
            )));
        }
    }

    #[test]
    fn only_complete_untampered_resources_can_enter_vpk_writer() {
        assert_eq!(
            build_verified_vpk(&BTreeMap::new()).err().as_deref(),
            Some("tree_resource_set_invalid")
        );
        let contract = [Resource {
            path: "models/props_tree/tree_oak_01.vmdl_c",
            bytes: 4,
            sha256: "dc9c5edb8b2d479e697b4b0b8ab874f32b325138598ce9e7b759eb8292110622",
        }];
        let mut resources = BTreeMap::from([(contract[0].path.to_string(), b"tree".to_vec())]);
        assert!(verify_resources(&contract, &resources).is_ok());
        assert!(vpk::inspect(
            &vpk::build(verify_resources(&contract, &resources).unwrap()).unwrap()
        )
        .is_ok());
        resources.get_mut(contract[0].path).unwrap()[0] ^= 1;
        assert_eq!(
            verify_resources(&contract, &resources).err().as_deref(),
            Some("tree_resource_unverified")
        );
        resources.remove(contract[0].path);
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
        let resources = RESOURCES
            .iter()
            .map(|resource| {
                let bytes = std::fs::read(std::path::Path::new(&root).join(resource.path))
                    .expect("read pinned resource");
                (resource.path.to_string(), bytes)
            })
            .collect::<BTreeMap<_, _>>();
        let first = build_verified_vpk(&resources).expect("build exact pinned resources");
        let second = build_verified_vpk(&resources).expect("build again");
        assert_eq!(first.bytes(), second.bytes());
        let report = vpk::inspect(first.bytes()).expect("reopen VPK");
        assert_eq!(report.entries, RESOURCES.len());
        assert_eq!(
            report.payload_bytes,
            RESOURCES
                .iter()
                .map(|entry| entry.bytes as u64)
                .sum::<u64>()
        );
        let app_data = std::path::Path::new(&root).join("betterfy-test-app-data");
        for resource in RESOURCES {
            crate::content_store::store_pinned_resource(
                &app_data,
                resource.bytes,
                resource.sha256,
                resources.get(resource.path).unwrap(),
            )
            .expect("publish verified pinned resource");
        }
        acquire_verified_resources(&app_data, &AtomicBool::new(false))
            .expect("reuse verified cache without network");
        let from_store = build_from_verified_store(&app_data).expect("build from immutable store");
        assert_eq!(from_store.bytes(), first.bytes());
        let plan = plan_from_verified_store(&app_data).expect("reviewed plan");
        assert_eq!(plan.resource_count, 21);
        assert_eq!(plan.deploy_enabled, cfg!(target_os = "windows"));
        let plan_id = plan.plan_id;
        assert_eq!(
            stage_from_verified_store(&app_data, &plan_id, false)
                .err()
                .as_deref(),
            Some("build_confirmation_required")
        );
        assert_eq!(
            stage_from_verified_store(&app_data, "sha256:stale", true)
                .err()
                .as_deref(),
            Some("build_plan_stale")
        );
        let receipt =
            stage_from_verified_store(&app_data, &plan_id, true).expect("stage verified VPK");
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
        begin_download(root.clone()).expect("start pinned download");
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
        assert_eq!(status.verified_resources, RESOURCES.len());
        assert!(status.plan.is_some());
        let first = build_from_verified_store(&root).expect("build from cache");
        acquire_verified_resources(&root, &cancelled).expect("idempotent cache read");
        let second = build_from_verified_store(&root).expect("rebuild from cache");
        assert_eq!(first.bytes(), second.bytes());
        std::fs::remove_dir_all(root).expect("clean generated test cache");
    }
}

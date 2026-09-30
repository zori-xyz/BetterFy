use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

use crate::vpk::{self, VpkInput};

const MAX_PACKAGES: usize = 64;
const MAX_RESOURCES: usize = 4096;
const MAX_PAYLOAD_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Debug)]
pub(crate) struct BundlePackage {
    pub package_id: String,
    pub resources: BTreeMap<String, Vec<u8>>,
}

impl BundlePackage {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "activated by the next pinned VPK package contract"
        )
    )]
    pub(crate) fn from_embedded_vpk(package_id: String, bytes: &[u8]) -> Result<Self, String> {
        validate_package_id(&package_id)?;
        let resources = vpk::extract_embedded(bytes)?;
        Ok(Self {
            package_id,
            resources,
        })
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BundleDuplicate {
    pub path: String,
    pub kept_package_id: String,
    pub duplicate_package_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BundleOverride {
    pub path: String,
    pub winner_package_id: String,
    pub shadowed_package_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BundleContribution {
    pub package_id: String,
    pub input_resources: usize,
    pub effective_resources: usize,
    pub duplicate_resources: usize,
    pub shadowed_resources: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BundlePlan {
    pub plan_id: String,
    pub package_ids: Vec<String>,
    pub resource_count: usize,
    pub payload_bytes: usize,
    pub duplicates: Vec<BundleDuplicate>,
    pub overrides: Vec<BundleOverride>,
    pub contributions: Vec<BundleContribution>,
}

#[derive(Clone, Debug)]
pub(crate) struct VerifiedBundle {
    bytes: Vec<u8>,
    sha256: String,
    plan: BundlePlan,
}

#[derive(Clone, Debug)]
struct ResolvedResource {
    package_id: String,
    bytes: Vec<u8>,
}

impl VerifiedBundle {
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn sha256(&self) -> &str {
        &self.sha256
    }

    pub(crate) fn plan(&self) -> &BundlePlan {
        &self.plan
    }
}

fn validate_package_id(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || value.starts_with('.')
        || value.ends_with('.')
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
    {
        return Err("bundle_package_invalid".to_string());
    }
    Ok(())
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn feed_field(hasher: &mut Sha256, bytes: &[u8]) -> Result<(), String> {
    let length = u64::try_from(bytes.len()).map_err(|_| "bundle_limit_exceeded".to_string())?;
    hasher.update(length.to_le_bytes());
    hasher.update(bytes);
    Ok(())
}

pub(crate) fn build(packages: Vec<BundlePackage>) -> Result<VerifiedBundle, String> {
    if packages.is_empty() || packages.len() > MAX_PACKAGES {
        return Err("bundle_package_invalid".to_string());
    }

    let mut package_ids = Vec::with_capacity(packages.len());
    let mut input_resources = BTreeMap::<String, usize>::new();
    let mut seen_packages = BTreeSet::new();
    let mut resolved = BTreeMap::<String, ResolvedResource>::new();
    let mut duplicate_map = BTreeMap::<String, BundleDuplicate>::new();
    let mut override_map = BTreeMap::<String, BundleOverride>::new();
    let mut input_count = 0usize;
    let mut input_bytes = 0usize;
    let mut plan_hasher = Sha256::new();
    plan_hasher.update(b"betterfy-bundle-plan-v1");

    for package in packages {
        validate_package_id(&package.package_id)?;
        if package.resources.is_empty() || !seen_packages.insert(package.package_id.clone()) {
            return Err("bundle_package_invalid".to_string());
        }
        package_ids.push(package.package_id.clone());
        input_resources.insert(package.package_id.clone(), package.resources.len());
        plan_hasher.update(b"package");
        feed_field(&mut plan_hasher, package.package_id.as_bytes())?;
        plan_hasher.update(
            u64::try_from(package.resources.len())
                .map_err(|_| "bundle_limit_exceeded".to_string())?
                .to_le_bytes(),
        );

        for (path, bytes) in package.resources {
            vpk::validate_path(&path)?;
            input_count = input_count
                .checked_add(1)
                .ok_or_else(|| "bundle_limit_exceeded".to_string())?;
            input_bytes = input_bytes
                .checked_add(bytes.len())
                .ok_or_else(|| "bundle_limit_exceeded".to_string())?;
            if input_count > MAX_RESOURCES || input_bytes > MAX_PAYLOAD_BYTES {
                return Err("bundle_limit_exceeded".to_string());
            }

            plan_hasher.update(b"resource");
            feed_field(&mut plan_hasher, path.as_bytes())?;
            feed_field(&mut plan_hasher, &bytes)?;

            match resolved.get(&path) {
                None => {
                    resolved.insert(
                        path,
                        ResolvedResource {
                            package_id: package.package_id.clone(),
                            bytes,
                        },
                    );
                }
                Some(winner) if winner.bytes == bytes => {
                    duplicate_map
                        .entry(path.clone())
                        .or_insert_with(|| BundleDuplicate {
                            path,
                            kept_package_id: winner.package_id.clone(),
                            duplicate_package_ids: Vec::new(),
                        })
                        .duplicate_package_ids
                        .push(package.package_id.clone());
                }
                Some(winner) => {
                    override_map
                        .entry(path.clone())
                        .or_insert_with(|| BundleOverride {
                            path,
                            winner_package_id: winner.package_id.clone(),
                            shadowed_package_ids: Vec::new(),
                        })
                        .shadowed_package_ids
                        .push(package.package_id.clone());
                }
            }
        }
    }

    let payload_bytes = resolved.values().try_fold(0usize, |total, resource| {
        total
            .checked_add(resource.bytes.len())
            .ok_or_else(|| "bundle_limit_exceeded".to_string())
    })?;
    if resolved.is_empty() || resolved.len() > MAX_RESOURCES || payload_bytes > MAX_PAYLOAD_BYTES {
        return Err("bundle_limit_exceeded".to_string());
    }

    let inputs = resolved
        .iter()
        .map(|(path, resource)| VpkInput {
            path,
            bytes: &resource.bytes,
        })
        .collect::<Vec<_>>();
    let bytes = vpk::build(inputs)?;
    let report = vpk::inspect(&bytes)?;
    if report.entries != resolved.len() || report.payload_bytes != payload_bytes as u64 {
        return Err("bundle_verification_failed".to_string());
    }
    let sha256 = hash_bytes(&bytes);
    let duplicates = duplicate_map.into_values().collect::<Vec<_>>();
    let overrides = override_map.into_values().collect::<Vec<_>>();
    let contributions = package_ids
        .iter()
        .map(|package_id| BundleContribution {
            package_id: package_id.clone(),
            input_resources: input_resources.get(package_id).copied().unwrap_or_default(),
            effective_resources: resolved
                .values()
                .filter(|resource| resource.package_id == *package_id)
                .count(),
            duplicate_resources: duplicates
                .iter()
                .filter(|item| item.duplicate_package_ids.contains(package_id))
                .count(),
            shadowed_resources: overrides
                .iter()
                .filter(|item| item.shadowed_package_ids.contains(package_id))
                .count(),
        })
        .collect();
    let plan = BundlePlan {
        plan_id: format!("sha256:{:x}", plan_hasher.finalize()),
        package_ids,
        resource_count: resolved.len(),
        payload_bytes,
        duplicates,
        overrides,
        contributions,
    };
    Ok(VerifiedBundle {
        bytes,
        sha256,
        plan,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(id: &str, resources: &[(&str, &[u8])]) -> BundlePackage {
        BundlePackage {
            package_id: id.to_string(),
            resources: resources
                .iter()
                .map(|(path, bytes)| ((*path).to_string(), (*bytes).to_vec()))
                .collect(),
        }
    }

    #[test]
    fn merges_packages_into_one_deterministic_vpk() {
        let selected = vec![
            package("betterfy.first", &[("models/first.vmdl_c", b"first")]),
            package("betterfy.second", &[("materials/second.vmat_c", b"second")]),
        ];
        let first = build(selected.clone()).expect("first build");
        let second = build(selected).expect("second build");
        assert_eq!(first.bytes(), second.bytes());
        assert_eq!(first.plan(), second.plan());
        assert_eq!(first.plan().resource_count, 2);
        assert_eq!(
            first.plan().package_ids,
            ["betterfy.first", "betterfy.second"]
        );
        assert!(first.plan().duplicates.is_empty());
        assert!(first.plan().overrides.is_empty());
        assert_eq!(first.plan().contributions[0].effective_resources, 1);
    }

    #[test]
    fn merges_verified_embedded_vpks_without_trusting_their_archive_filename() {
        let first_vpk = vpk::build(vec![VpkInput {
            path: "models/first.vmdl_c",
            bytes: b"first",
        }])
        .expect("first VPK");
        let second_vpk = vpk::build(vec![VpkInput {
            path: "materials/second.vmat_c",
            bytes: b"second",
        }])
        .expect("second VPK");
        let result = build(vec![
            BundlePackage::from_embedded_vpk("betterfy.first".to_string(), &first_vpk)
                .expect("first package"),
            BundlePackage::from_embedded_vpk("betterfy.second".to_string(), &second_vpk)
                .expect("second package"),
        ])
        .expect("bundle");
        let resources = vpk::extract_embedded(result.bytes()).expect("combined resources");
        assert_eq!(resources["models/first.vmdl_c"], b"first");
        assert_eq!(resources["materials/second.vmat_c"], b"second");
        assert_eq!(result.plan().package_ids.len(), 2);
    }

    #[test]
    fn identical_paths_and_bytes_are_deduplicated() {
        let result = build(vec![
            package("betterfy.first", &[("models/shared.vmdl_c", b"same")]),
            package("betterfy.second", &[("models/shared.vmdl_c", b"same")]),
        ])
        .expect("deduplicated build");
        assert_eq!(result.plan().resource_count, 1);
        assert_eq!(result.plan().duplicates.len(), 1);
        assert_eq!(
            result.plan().duplicates[0].kept_package_id,
            "betterfy.first"
        );
        assert_eq!(
            result.plan().duplicates[0].duplicate_package_ids,
            ["betterfy.second"]
        );
        assert!(result.plan().overrides.is_empty());
        assert_eq!(result.plan().contributions[0].effective_resources, 1);
        assert_eq!(result.plan().contributions[1].duplicate_resources, 1);
        assert_eq!(result.plan().contributions[1].effective_resources, 0);
    }

    #[test]
    fn first_selected_package_wins_a_real_resource_collision() {
        let first_priority = build(vec![
            package("betterfy.first", &[("models/shared.vmdl_c", b"winner")]),
            package("betterfy.second", &[("models/shared.vmdl_c", b"shadowed")]),
        ])
        .expect("first priority");
        let reversed = build(vec![
            package("betterfy.second", &[("models/shared.vmdl_c", b"shadowed")]),
            package("betterfy.first", &[("models/shared.vmdl_c", b"winner")]),
        ])
        .expect("reversed priority");

        assert_eq!(first_priority.plan().overrides.len(), 1);
        assert_eq!(
            first_priority.plan().overrides[0].winner_package_id,
            "betterfy.first"
        );
        assert_eq!(
            first_priority.plan().overrides[0].shadowed_package_ids,
            ["betterfy.second"]
        );
        assert_eq!(
            first_priority.plan().contributions,
            [
                BundleContribution {
                    package_id: "betterfy.first".to_string(),
                    input_resources: 1,
                    effective_resources: 1,
                    duplicate_resources: 0,
                    shadowed_resources: 0,
                },
                BundleContribution {
                    package_id: "betterfy.second".to_string(),
                    input_resources: 1,
                    effective_resources: 0,
                    duplicate_resources: 0,
                    shadowed_resources: 1,
                },
            ]
        );
        assert_ne!(first_priority.bytes(), reversed.bytes());
        assert_ne!(first_priority.plan().plan_id, reversed.plan().plan_id);
    }

    #[test]
    fn shadowed_inputs_remain_bound_to_the_reviewed_plan() {
        let first = build(vec![
            package("betterfy.first", &[("models/shared.vmdl_c", b"winner")]),
            package(
                "betterfy.second",
                &[("models/shared.vmdl_c", b"shadowed-a")],
            ),
        ])
        .expect("first");
        let changed = build(vec![
            package("betterfy.first", &[("models/shared.vmdl_c", b"winner")]),
            package(
                "betterfy.second",
                &[("models/shared.vmdl_c", b"shadowed-b")],
            ),
        ])
        .expect("changed");
        assert_eq!(first.bytes(), changed.bytes());
        assert_ne!(first.plan().plan_id, changed.plan().plan_id);
    }

    #[test]
    fn rejects_duplicate_packages_and_unsafe_resources() {
        assert_eq!(
            build(vec![
                package("betterfy.same", &[("models/one.vmdl_c", b"one")]),
                package("betterfy.same", &[("models/two.vmdl_c", b"two")]),
            ])
            .err()
            .as_deref(),
            Some("bundle_package_invalid")
        );
        assert_eq!(
            build(vec![package(
                "betterfy.escape",
                &[("../outside.vmdl_c", b"outside")],
            )])
            .err()
            .as_deref(),
            Some("vpk_path_invalid")
        );
        assert_eq!(
            build(vec![package(
                "BetterFy.Invalid",
                &[("models/a.vmdl_c", b"a")]
            )])
            .err()
            .as_deref(),
            Some("bundle_package_invalid")
        );
    }
}

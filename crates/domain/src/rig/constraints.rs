use super::{SPINE_CAPABILITY_ID, SPINE_PATCH, bone_tree::BoneTree};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConstraintKind {
    Transform,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RigConstraint {
    pub constraint_id: String,
    pub kind: ConstraintKind,
    pub constrained_bone_id: String,
    pub target_bone_id: String,
    pub mix_ppm: u32,
    pub order: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintCapability {
    pub capability_id: String,
    pub spine_patch: String,
    pub transform_constraints: bool,
    pub manifest_sha256: String,
    pub source_hashes_verified: bool,
}

impl ConstraintCapability {
    pub fn from_verified_manifest(
        manifest_bytes: &[u8],
        source_files: &[(&str, &[u8])],
    ) -> Result<Self, String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Manifest {
            capability_id: String,
            target: Target,
            allowed_features: Vec<String>,
            source_hashes: BTreeMap<String, String>,
            static_contract_status: String,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Target {
            exact_version: String,
        }
        let manifest: Manifest =
            serde_json::from_slice(manifest_bytes).map_err(|e| e.to_string())?;
        if manifest.capability_id != SPINE_CAPABILITY_ID
            || manifest.target.exact_version != SPINE_PATCH
            || manifest.static_contract_status != "VERIFIED"
            || !manifest
                .allowed_features
                .iter()
                .any(|v| v == "transform-constraints")
            || source_files.len() != manifest.source_hashes.len()
        {
            return Err("Spine capability manifest contract mismatch".into());
        }
        let mut verified_names = BTreeSet::new();
        for (name, bytes) in source_files {
            if !verified_names.insert(*name) {
                return Err(format!("duplicate Spine capability source: {name}"));
            }
            let actual = format!("{:x}", Sha256::digest(bytes));
            if manifest.source_hashes.get(*name) != Some(&actual) {
                return Err(format!("Spine capability source hash mismatch: {name}"));
            }
        }
        let capability = Self {
            capability_id: manifest.capability_id,
            spine_patch: manifest.target.exact_version,
            transform_constraints: true,
            manifest_sha256: format!("{:x}", Sha256::digest(manifest_bytes)),
            source_hashes_verified: true,
        };
        capability.validate_verified()?;
        Ok(capability)
    }

    pub fn validate_verified(&self) -> Result<(), String> {
        if self.capability_id != SPINE_CAPABILITY_ID
            || self.spine_patch != SPINE_PATCH
            || !self.transform_constraints
            || !self.source_hashes_verified
            || !is_lower_sha256(&self.manifest_sha256)
        {
            Err("Spine 4.2.43 capability manifest mismatch".into())
        } else {
            Ok(())
        }
    }
}

pub fn validate_constraints(
    items: &[RigConstraint],
    bones: &BoneTree,
    capability: &ConstraintCapability,
) -> Result<Vec<String>, String> {
    capability.validate_verified()?;
    let bone_ids: BTreeSet<_> = bones.bones.iter().map(|b| b.bone_id.as_str()).collect();
    let mut ids = BTreeSet::new();
    let mut orders = BTreeSet::new();
    let mut edges: BTreeMap<&str, &str> = BTreeMap::new();
    for item in items {
        if !ids.insert(item.constraint_id.as_str()) || !orders.insert(item.order) {
            return Err("duplicate constraint id or order".into());
        }
        if item.mix_ppm > 1_000_000 {
            return Err("constraint mix outside range".into());
        }
        if !bone_ids.contains(item.constrained_bone_id.as_str())
            || !bone_ids.contains(item.target_bone_id.as_str())
        {
            return Err("constraint references unknown bone".into());
        }
        edges.insert(&item.constrained_bone_id, &item.target_bone_id);
    }
    for start in edges.keys() {
        let mut seen = BTreeSet::new();
        let mut cursor = Some(*start);
        while let Some(id) = cursor {
            if !seen.insert(id) {
                return Err("constraint dependency cycle".into());
            }
            cursor = edges.get(id).copied();
        }
    }
    let mut ordered: Vec<_> = items.iter().collect();
    ordered.sort_by_key(|v| (v.order, &v.constraint_id));
    Ok(ordered
        .into_iter()
        .map(|v| v.constraint_id.clone())
        .collect())
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

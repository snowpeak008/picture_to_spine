use f2s_application::export::publish_snapshot::{
    ActionApprovalBinding, AttachmentSnapshot, PublishSnapshot,
};
use f2s_domain::{
    ACTION_KEYS, TimeBase,
    animation::{clip::AnimationClip, markers::GameplayMarker},
    rig::{
        constraints::{ConstraintCapability, RigConstraint},
        mesh::Mesh,
        pivots_sockets::{Pivot, Socket},
        weights::WeightSet,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RigIrBone {
    pub id: String,
    pub parent: Option<String>,
    pub x_milli_px: i64,
    pub y_milli_px: i64,
    pub rotation_milli_degrees: i32,
    pub scale_x_ppm: i32,
    pub scale_y_ppm: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RigIrSlot {
    pub id: String,
    pub layer_id: String,
    pub bone_id: String,
    pub draw_key: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RigIrApprovalClosure {
    pub layer_set_sha256: String,
    pub rig_sha256: String,
    pub actions: Vec<ActionApprovalBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RigIrDocument {
    pub schema_version: String,
    pub export_id: String,
    pub rig_id: String,
    pub project_revision: u64,
    pub capability_id: String,
    pub spine_patch: String,
    pub time_base: TimeBase,
    pub approval_closure: RigIrApprovalClosure,
    pub bones: Vec<RigIrBone>,
    pub slots: Vec<RigIrSlot>,
    pub pivots: Vec<Pivot>,
    pub sockets: Vec<Socket>,
    pub meshes: Vec<Mesh>,
    pub weights: Vec<WeightSet>,
    pub constraints: Vec<RigConstraint>,
    pub constraint_capability: ConstraintCapability,
    pub attachments: Vec<AttachmentSnapshot>,
    pub animations: BTreeMap<String, AnimationClip>,
    pub gameplay_markers: Vec<GameplayMarker>,
}

impl RigIrDocument {
    pub fn from_snapshot(snapshot: &PublishSnapshot) -> Self {
        let animations = snapshot
            .clips
            .iter()
            .map(|clip| (clip.action_key.clone(), clip.clone()))
            .collect();
        Self {
            schema_version: "1.1.0".into(),
            export_id: snapshot.export_id.clone(),
            rig_id: snapshot.rig_id.clone(),
            project_revision: snapshot.project_revision,
            capability_id: snapshot.capability_id.clone(),
            spine_patch: snapshot.spine_patch.clone(),
            time_base: snapshot.time_base,
            approval_closure: RigIrApprovalClosure {
                layer_set_sha256: snapshot.approved_layer_set_hash.clone(),
                rig_sha256: snapshot.approved_rig_hash.clone(),
                actions: snapshot.action_approvals.clone(),
            },
            bones: snapshot
                .bones
                .bones
                .iter()
                .map(|bone| RigIrBone {
                    id: bone.bone_id.clone(),
                    parent: bone.parent_id.clone(),
                    x_milli_px: bone.rest.x_milli_px,
                    y_milli_px: bone.rest.y_milli_px,
                    rotation_milli_degrees: bone.rest.rotation_milli_deg,
                    scale_x_ppm: bone.rest.scale_x_ppm,
                    scale_y_ppm: bone.rest.scale_y_ppm,
                })
                .collect(),
            slots: snapshot
                .slots
                .stable_draw_order()
                .into_iter()
                .map(|slot| RigIrSlot {
                    id: slot.slot_id.clone(),
                    layer_id: slot.layer_id.clone(),
                    bone_id: slot.bone_id.clone(),
                    draw_key: slot.draw_key,
                })
                .collect(),
            pivots: snapshot.pivots.clone(),
            sockets: snapshot.sockets.clone(),
            meshes: snapshot.meshes.clone(),
            weights: snapshot.weights.clone(),
            constraints: snapshot.constraints.clone(),
            constraint_capability: snapshot.constraint_capability.clone(),
            attachments: snapshot.attachments.clone(),
            animations,
            gameplay_markers: snapshot.markers.clone(),
        }
    }

    pub fn validate_reopened(&self) -> Result<(), String> {
        if self.schema_version != "1.1.0" || self.spine_patch != "4.2.43" {
            return Err("Rig IR schema or Spine patch mismatch".into());
        }
        let actions = self
            .animations
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        if actions != ACTION_KEYS.into_iter().collect() {
            return Err("Rig IR must reopen with the exact ten actions".into());
        }
        Ok(())
    }
}

pub fn rig_ir_bytes(snapshot: &PublishSnapshot) -> Result<Vec<u8>, String> {
    let document = RigIrDocument::from_snapshot(snapshot);
    document.validate_reopened()?;
    let bytes = f2s_domain::canonical::canonical_bytes(&document).map_err(|e| e.to_string())?;
    let reopened: RigIrDocument = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    reopened.validate_reopened()?;
    if reopened != document {
        return Err("Rig IR semantic reopen mismatch".into());
    }
    Ok(bytes)
}

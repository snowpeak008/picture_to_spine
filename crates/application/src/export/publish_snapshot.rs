use f2s_domain::{
    TimeBase,
    animation::{clip::AnimationClip, markers::GameplayMarker},
    rig::{
        SPINE_CAPABILITY_ID, SPINE_PATCH,
        bone_tree::BoneTree,
        constraints::{ConstraintCapability, RigConstraint},
        mesh::Mesh,
        pivots_sockets::{Pivot, Socket},
        slots::SlotSet,
        weights::WeightSet,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttachmentSnapshot {
    pub attachment_id: String,
    pub slot_id: String,
    pub logical_png_path: String,
    pub source_sha256: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionApprovalBinding {
    pub action_key: String,
    pub clip_sha256: String,
    pub pose_approval_sha256: String,
    pub hit_approval_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishSnapshot {
    pub export_id: String,
    pub rig_id: String,
    pub project_revision: u64,
    pub approved_layer_set_hash: String,
    pub approved_rig_hash: String,
    pub action_approvals: Vec<ActionApprovalBinding>,
    pub capability_id: String,
    pub spine_patch: String,
    pub primary_weapon: String,
    pub time_base: TimeBase,
    pub bones: BoneTree,
    pub slots: SlotSet,
    pub pivots: Vec<Pivot>,
    pub sockets: Vec<Socket>,
    pub meshes: Vec<Mesh>,
    pub weights: Vec<WeightSet>,
    pub constraints: Vec<RigConstraint>,
    pub constraint_capability: ConstraintCapability,
    pub attachments: Vec<AttachmentSnapshot>,
    pub clips: Vec<AnimationClip>,
    pub markers: Vec<GameplayMarker>,
}

impl PublishSnapshot {
    pub fn pinned_capability(&self) -> bool {
        self.capability_id == SPINE_CAPABILITY_ID && self.spine_patch == SPINE_PATCH
    }
}

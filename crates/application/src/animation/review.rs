use crate::approvals::VerifiedHumanActor;
use f2s_domain::{
    animation::{
        clip::AnimationClip,
        markers::{GameplayMarker, validate_markers},
    },
    canonical::canonical_sha256,
    motion::registry::requires_hit_frame,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoseApprovalRecord {
    pub action_key: String,
    pub clip_hash: String,
    pub pose_keys: Vec<String>,
    pub human_actor: String,
    pub valid: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HitApprovalRecord {
    pub action_key: String,
    pub clip_hash: String,
    pub marker_hash: String,
    pub human_actor: String,
    pub valid: bool,
}

pub fn approve_key_poses(
    clip: &AnimationClip,
    required: &[String],
    confirmed: &[String],
    actor: VerifiedHumanActor,
) -> Result<PoseApprovalRecord, String> {
    clip.validate()?;
    actor.require_binding(
        "approve-key-poses",
        &canonical_sha256(clip).map_err(|e| e.to_string())?,
    )?;
    if required.is_empty() || required != confirmed {
        return Err("every required key pose must be confirmed in canonical order".into());
    }
    Ok(PoseApprovalRecord {
        action_key: clip.action_key.clone(),
        clip_hash: canonical_sha256(clip).map_err(|e| e.to_string())?,
        pose_keys: confirmed.to_vec(),
        human_actor: actor.actor_id().into(),
        valid: true,
    })
}
pub fn approve_hit_frame(
    clip: &AnimationClip,
    markers: &[GameplayMarker],
    actor: VerifiedHumanActor,
) -> Result<HitApprovalRecord, String> {
    if !requires_hit_frame(&clip.action_key) {
        return Err("hit approval exists only for attack clips".into());
    }
    validate_markers(&clip.action_key, clip.duration_ticks, markers)?;
    actor.require_binding(
        "approve-hit-frame",
        &canonical_sha256(&markers).map_err(|e| e.to_string())?,
    )?;
    Ok(HitApprovalRecord {
        action_key: clip.action_key.clone(),
        clip_hash: canonical_sha256(clip).map_err(|e| e.to_string())?,
        marker_hash: canonical_sha256(&markers).map_err(|e| e.to_string())?,
        human_actor: actor.actor_id().into(),
        valid: true,
    })
}

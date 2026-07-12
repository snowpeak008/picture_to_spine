use super::temporary_rig::TemporaryRigSnapshot;
use crate::approvals::VerifiedHumanActor;
use f2s_domain::canonical::canonical_sha256;
use f2s_domain::governance::Approval;
use f2s_domain::layers::LayerSet;
use f2s_domain::rig::{RigApprovalState, RigCandidate, RigRevisionRefs, SPINE_CAPABILITY_ID};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RigApprovalRequest {
    pub rig_id: String,
    pub revisions: RigRevisionRefs,
    pub approved_layer_set_hash: String,
    pub expected_revision: u64,
    pub capability_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovedRigRef {
    pub rig_id: String,
    pub revision_hash: String,
    pub approved_layer_set_hash: String,
    pub actor_id: String,
    pub valid: bool,
}

pub fn approve_rig(
    request: RigApprovalRequest,
    current_revision: u64,
    diagnostics: &TemporaryRigSnapshot,
    actor: VerifiedHumanActor,
) -> Result<ApprovedRigRef, String> {
    actor.require_binding(
        "approve-rig",
        &canonical_sha256(&request.revisions).map_err(|e| e.to_string())?,
    )?;
    if request.expected_revision != current_revision
        || diagnostics.source_revisions != request.revisions
    {
        return Err("stale Rig revision".into());
    }
    if request.capability_id != SPINE_CAPABILITY_ID {
        return Err("capability manifest mismatch".into());
    }
    if diagnostics.has_blocking_issues() {
        return Err("blocking Rig diagnostics remain".into());
    }
    let revision_hash = canonical_sha256(&request.revisions).map_err(|e| e.to_string())?;
    Ok(ApprovedRigRef {
        rig_id: request.rig_id,
        revision_hash,
        approved_layer_set_hash: request.approved_layer_set_hash,
        actor_id: actor.actor_id().into(),
        valid: true,
    })
}

pub fn invalidate_if_changed(
    approval: &mut ApprovedRigRef,
    current_hash: &str,
    current_layer_hash: &str,
) -> bool {
    if approval.revision_hash != current_hash
        || approval.approved_layer_set_hash != current_layer_hash
    {
        approval.valid = false;
        true
    } else {
        false
    }
}

/// Hashes the complete Rig aggregate after deterministic ordering and approval-state
/// normalization. This intentionally does not hash only revision counters.
pub fn rig_approval_payload(candidate: &RigCandidate) -> Result<String, String> {
    f2s_domain::rig::rig_approval_payload_sha256(candidate)
}

pub fn approve_rig_candidate(
    candidate: &mut RigCandidate,
    layer_set: &LayerSet,
    diagnostics: &TemporaryRigSnapshot,
    actor: VerifiedHumanActor,
    approved_at_utc: &str,
) -> Result<Approval, String> {
    if approved_at_utc.trim().is_empty() {
        return Err("approval timestamp required".into());
    }
    if candidate.approval_state != RigApprovalState::Pending {
        return Err("Rig candidate is not pending review".into());
    }
    candidate.validate(layer_set)?;
    if !diagnostics.is_current_for(candidate) {
        return Err("Rig diagnostics are incomplete, unverified, or stale".into());
    }
    if diagnostics.has_blocking_issues() {
        return Err("blocking Rig diagnostics remain".into());
    }
    let payload = rig_approval_payload(candidate)?;
    actor.require_binding("approve-rig", &payload)?;
    let approval = Approval {
        approval_id: Uuid::new_v4().to_string(),
        gate_id: "rig".into(),
        target_id: candidate.rig_id.clone(),
        target_revision: candidate.revision,
        target_sha256: payload,
        actor_id: actor.actor_id().into(),
        approved_at_utc: approved_at_utc.into(),
        invalidated: false,
    };
    candidate.approval_state = RigApprovalState::Approved;
    Ok(approval)
}

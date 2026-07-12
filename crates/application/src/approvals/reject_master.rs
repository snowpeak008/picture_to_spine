use super::VerifiedHumanActor;
use f2s_domain::{
    canonical::canonical_sha256,
    governance::{ReviewOutcome, ReviewRecord},
    master::MasterCandidate,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RejectionPayload<'a> {
    master: &'a MasterCandidate,
    reason: &'a str,
}

pub fn master_rejection_payload(master: &MasterCandidate, reason: &str) -> Result<String, String> {
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > 1_000 {
        return Err("rejection reason must contain 1..1000 characters".into());
    }
    canonical_sha256(&RejectionPayload { master, reason }).map_err(|error| error.to_string())
}

pub fn reject_master(
    master: &mut MasterCandidate,
    reason: &str,
    actor: VerifiedHumanActor,
    at: &str,
) -> Result<ReviewRecord, String> {
    if master.approval_state != "PENDING" {
        return Err("only a pending master can be rejected".into());
    }
    if at.trim().is_empty() {
        return Err("review timestamp required".into());
    }
    let reason = reason.trim();
    let payload = master_rejection_payload(master, reason)?;
    actor.require_binding("reject-master", &payload)?;
    master.approval_state = "REJECTED".into();
    Ok(ReviewRecord {
        review_id: Uuid::new_v4().to_string(),
        gate_id: "master".into(),
        target_id: master.master_id.clone(),
        target_revision: master.candidate_revision,
        target_sha256: master.source_sha256.clone(),
        actor_id: actor.actor_id().into(),
        outcome: ReviewOutcome::Rejected,
        reason: reason.into(),
        reviewed_at_utc: at.into(),
    })
}

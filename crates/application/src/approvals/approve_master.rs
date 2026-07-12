use super::VerifiedHumanActor;
use f2s_domain::{governance::Approval, master::MasterCandidate};
use uuid::Uuid;

pub fn approve(
    master: &mut MasterCandidate,
    actor: VerifiedHumanActor,
    at: &str,
) -> Result<Approval, String> {
    if at.trim().is_empty() {
        return Err("approval timestamp required".into());
    }
    if master.approval_state != "PENDING" {
        return Err("only a pending master can be approved".into());
    }
    let payload = master.approval_payload_sha256()?;
    actor.require_binding("approve-master", &payload)?;
    master.approval_state = "APPROVED".into();
    Ok(Approval {
        approval_id: Uuid::new_v4().to_string(),
        gate_id: "master".into(),
        target_id: master.master_id.clone(),
        target_revision: master.candidate_revision,
        target_sha256: payload,
        actor_id: actor.actor_id().into(),
        approved_at_utc: at.into(),
        invalidated: false,
    })
}

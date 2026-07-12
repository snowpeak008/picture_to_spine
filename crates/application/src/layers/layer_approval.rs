use crate::approvals::VerifiedHumanActor;
use f2s_domain::{
    canonical::canonical_sha256,
    governance::Approval,
    layers::{LayerSet, PixelProvenance, RecompositionMetrics},
};
use uuid::Uuid;

pub fn layer_approval_payload(set: &LayerSet) -> Result<String, String> {
    let mut content = set.clone();
    content.approval_state = "PENDING".into();
    for layer in &mut content.layers {
        layer.approved = false
    }
    canonical_sha256(&content).map_err(|e| e.to_string())
}

pub fn recomposition_is_approvable(
    set: &LayerSet,
    metrics: RecompositionMetrics,
    provenance: &[PixelProvenance],
) -> bool {
    let structural_qa_passed = metrics.missing_pixels == 0
        && metrics.overlap_pixels == 0
        && metrics.alpha_error_pixels == 0
        && metrics.empty_layer_masks == 0;
    let changed_pixels_are_human_supplied = metrics.changed_visible_pixels == 0
        || set.layers.iter().all(|layer| {
            provenance.iter().any(|pixel_source| {
                pixel_source.artifact_sha256 == layer.attachment_sha256
                    && pixel_source.can_enter_approved_layer()
            })
        });
    structural_qa_passed && changed_pixels_are_human_supplied
}
pub fn approve_layers(
    set: &mut LayerSet,
    metrics: RecompositionMetrics,
    provenance: &[PixelProvenance],
    actor: VerifiedHumanActor,
    at: &str,
) -> Result<Approval, String> {
    set.validate()?;
    set.validate_required_v1_roles()?;
    let payload = layer_approval_payload(set)?;
    actor.require_binding("approve-layers", &payload)?;
    if at.trim().is_empty() {
        return Err("approval timestamp required".into());
    }
    if !recomposition_is_approvable(set, metrics, provenance) {
        return Err("recomposition QA failed".into());
    }
    if provenance.iter().any(|v| !v.can_enter_approved_layer())
        || set.layers.iter().any(|layer| {
            !provenance.iter().any(|pixel_source| {
                pixel_source.artifact_sha256 == layer.attachment_sha256
                    && pixel_source.can_enter_approved_layer()
            })
        })
    {
        return Err("unaccepted or missing pixel provenance present".into());
    }
    for layer in &mut set.layers {
        layer.approved = true
    }
    set.approval_state = "APPROVED".into();
    Ok(Approval {
        approval_id: Uuid::new_v4().to_string(),
        gate_id: "layers".into(),
        target_id: set.layer_set_id.clone(),
        target_revision: set.revision,
        target_sha256: payload,
        actor_id: actor.actor_id().into(),
        approved_at_utc: at.into(),
        invalidated: false,
    })
}

use f2s_application::layers::{
    approve_layers, layer_approval_payload, register_completion, replace_layer_attachment,
};
use f2s_domain::layers::{
    Layer, LayerRole, LayerSet, PixelOrigin, PixelProvenance, RecompositionMetrics,
};
mod common;
fn set() -> LayerSet {
    LayerSet {
        layer_set_id: "ls".into(),
        master_id: "m".into(),
        revision: 0,
        layers: LayerRole::REQUIRED_V1
            .iter()
            .enumerate()
            .map(|(index, role)| Layer {
                layer_id: format!("layer-{index}"),
                name: format!("{role:?}"),
                role: *role,
                attachment_sha256: format!("{index:064x}"),
                mask_sha256: format!("{:064x}", index + 100),
                visible: true,
                approved: false,
            })
            .collect(),
        approval_state: "PENDING".into(),
    }
}
fn source_provenance(set: &LayerSet) -> Vec<PixelProvenance> {
    set.layers
        .iter()
        .map(|layer| PixelProvenance {
            artifact_sha256: layer.attachment_sha256.clone(),
            origin: PixelOrigin::Source,
            source_sha256: "f".repeat(64),
            prompt_pack_id: None,
            receipt_ref: None,
            accepted_by: None,
            acceptance_attestation_sha256: None,
        })
        .collect()
}
#[test]
fn ai_pixels_cannot_bypass_acceptance() {
    let mut layers = set();
    let mut provenance = source_provenance(&layers);
    let ai = PixelProvenance {
        artifact_sha256: layers.layers[0].attachment_sha256.clone(),
        origin: PixelOrigin::LocalAi,
        source_sha256: "b".repeat(64),
        prompt_pack_id: Some("p".into()),
        receipt_ref: None,
        accepted_by: None,
        acceptance_attestation_sha256: None,
    };
    assert!(register_completion(ai.clone()).is_ok());
    provenance[0] = ai;
    let actor = common::human(
        "approve-layers",
        &layer_approval_payload(&layers).unwrap(),
        "artist",
    );
    assert!(
        approve_layers(
            &mut layers,
            RecompositionMetrics {
                missing_pixels: 0,
                overlap_pixels: 0,
                changed_visible_pixels: 0,
                alpha_error_pixels: 0,
                empty_layer_masks: 0,
            },
            &provenance,
            actor,
            "2026-07-11T00:00:00Z",
        )
        .is_err()
    );
}
#[test]
fn recomposition_error_blocks_gate() {
    let mut layers = set();
    let provenance = source_provenance(&layers);
    let actor = common::human(
        "approve-layers",
        &layer_approval_payload(&layers).unwrap(),
        "artist",
    );
    assert!(
        approve_layers(
            &mut layers,
            RecompositionMetrics {
                missing_pixels: 1,
                overlap_pixels: 0,
                changed_visible_pixels: 0,
                alpha_error_pixels: 0,
                empty_layer_masks: 0,
            },
            &provenance,
            actor,
            "2026-07-11T00:00:00Z",
        )
        .is_err()
    );
}

#[test]
fn overlap_and_empty_ai_acceptance_are_hard_failures() {
    let mut layers = set();
    let provenance = source_provenance(&layers);
    let actor = common::human(
        "approve-layers",
        &layer_approval_payload(&layers).unwrap(),
        "artist",
    );
    let empty_acceptance = PixelProvenance {
        artifact_sha256: "a".repeat(64),
        origin: PixelOrigin::LocalAi,
        source_sha256: "b".repeat(64),
        prompt_pack_id: Some("p".into()),
        receipt_ref: None,
        accepted_by: Some("".into()),
        acceptance_attestation_sha256: Some("c".repeat(64)),
    };
    assert!(!empty_acceptance.can_enter_approved_layer());
    assert!(
        approve_layers(
            &mut layers,
            RecompositionMetrics {
                missing_pixels: 0,
                overlap_pixels: 1,
                changed_visible_pixels: 0,
                alpha_error_pixels: 0,
                empty_layer_masks: 0,
            },
            &provenance,
            actor,
            "2026-07-11T00:00:00Z",
        )
        .is_err()
    );
}

#[test]
fn complete_recomposition_with_bound_human_and_provenance_approves() {
    let mut layers = set();
    let provenance = source_provenance(&layers);
    let actor = common::human(
        "approve-layers",
        &layer_approval_payload(&layers).unwrap(),
        "artist",
    );
    let approval = approve_layers(
        &mut layers,
        RecompositionMetrics {
            missing_pixels: 0,
            overlap_pixels: 0,
            changed_visible_pixels: 0,
            alpha_error_pixels: 0,
            empty_layer_masks: 0,
        },
        &provenance,
        actor,
        "2026-07-11T00:00:00Z",
    )
    .unwrap();
    assert_eq!(layers.approval_state, "APPROVED");
    assert!(layers.layers.iter().all(|layer| layer.approved));
    assert_eq!(approval.gate_id, "layers");
}

#[test]
fn human_supplied_replacement_is_revisioned_and_changed_pixels_require_provenance() {
    let mut layers = set();
    let original_revision = layers.revision;
    replace_layer_attachment(&mut layers, "layer-0", &"a".repeat(64), &"b".repeat(64)).unwrap();
    assert_eq!(layers.revision, original_revision + 1);
    assert_eq!(layers.layers[0].attachment_sha256, "a".repeat(64));

    let metrics = RecompositionMetrics {
        missing_pixels: 0,
        overlap_pixels: 0,
        changed_visible_pixels: 12,
        alpha_error_pixels: 0,
        empty_layer_masks: 0,
    };
    let incomplete = source_provenance(&set());
    let actor = common::human(
        "approve-layers",
        &layer_approval_payload(&layers).unwrap(),
        "artist",
    );
    assert!(
        approve_layers(
            &mut layers.clone(),
            metrics,
            &incomplete,
            actor,
            "2026-07-11T00:00:00Z"
        )
        .is_err()
    );

    let mut provenance = source_provenance(&set());
    provenance[0] = PixelProvenance {
        artifact_sha256: "a".repeat(64),
        origin: PixelOrigin::Manual,
        source_sha256: "f".repeat(64),
        prompt_pack_id: None,
        receipt_ref: None,
        accepted_by: None,
        acceptance_attestation_sha256: None,
    };
    let actor = common::human(
        "approve-layers",
        &layer_approval_payload(&layers).unwrap(),
        "artist",
    );
    approve_layers(
        &mut layers,
        metrics,
        &provenance,
        actor,
        "2026-07-11T00:00:00Z",
    )
    .unwrap();
}

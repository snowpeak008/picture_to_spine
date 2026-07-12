use f2s_application::{
    approvals::{approve, master_rejection_payload, reject_master, revalidate},
    master::{create_master, revise},
};
use f2s_domain::{
    import::SourceArtifact,
    master::{GripMode, PrimaryWeaponSpec, StyleSpec, WeaponHand, WeaponSizeClass},
};
mod common;
fn spec() -> StyleSpec {
    StyleSpec {
        revision: 0,
        viewpoint: "side-view".into(),
        rendering_style: "anime-clean".into(),
        outline: "dark".into(),
        palette_notes: "blue".into(),
        identity_notes: "original test character".into(),
        primary_weapon: Some(PrimaryWeaponSpec {
            weapon_type: "test-sword".into(),
            grip_mode: GripMode::OneHand,
            weapon_hand: WeaponHand::FarHand,
            socket_semantic: "weapon-grip".into(),
            size_class: WeaponSizeClass::Medium,
            silhouette_constraints: "weapon remains readable beside body".into(),
        }),
    }
}

#[test]
fn rejection_requires_reason_and_exact_human_binding() {
    let source = SourceArtifact {
        artifact_id: "source".into(),
        sha256: "a".repeat(64),
        media_type: "image/png".into(),
        width: 512,
        height: 512,
        byte_length: 100,
        bit_depth: 8,
        provenance: "user".into(),
        approval_state: "UNAPPROVED".into(),
    };
    let mut master = create_master(&source, spec()).unwrap();
    assert!(master_rejection_payload(&master, " ").is_err());
    let payload = master_rejection_payload(&master, "轮廓需修正").unwrap();
    let wrong_actor = common::human("reject-master", &"b".repeat(64), "artist");
    assert!(
        reject_master(
            &mut master.clone(),
            "轮廓需修正",
            wrong_actor,
            "2026-01-01T00:00:00Z"
        )
        .is_err()
    );
    let actor = common::human("reject-master", &payload, "artist");
    let record = reject_master(&mut master, "轮廓需修正", actor, "2026-01-01T00:00:00Z").unwrap();
    assert_eq!(master.approval_state, "REJECTED");
    assert_eq!(record.reason, "轮廓需修正");
}
#[test]
fn approval_is_bound_to_exact_master_revision_and_hash() {
    let source = SourceArtifact {
        artifact_id: "source".into(),
        sha256: "a".repeat(64),
        media_type: "image/png".into(),
        width: 512,
        height: 512,
        byte_length: 100,
        bit_depth: 8,
        provenance: "user".into(),
        approval_state: "UNAPPROVED".into(),
    };
    let mut master = create_master(&source, spec()).unwrap();
    let payload = master.approval_payload_sha256().unwrap();
    let source_only_actor = common::human("approve-master", &source.sha256, "artist");
    assert!(
        approve(
            &mut master.clone(),
            source_only_actor,
            "2026-01-01T00:00:00Z"
        )
        .is_err(),
        "source bytes alone must never authorize the complete StyleSpec candidate"
    );
    let actor = common::human("approve-master", &payload, "artist");
    let approval = approve(&mut master, actor, "2026-01-01T00:00:00Z").unwrap();
    assert!(revalidate(&approval, &master.master_id, 0, &payload).is_ok());
    let revised = revise(
        &master,
        StyleSpec {
            revision: 1,
            ..spec()
        },
    )
    .unwrap();
    assert_ne!(payload, revised.approval_payload_sha256().unwrap());
    assert!(
        revalidate(
            &approval,
            &revised.master_id,
            1,
            &revised.approval_payload_sha256().unwrap()
        )
        .is_err()
    );
}

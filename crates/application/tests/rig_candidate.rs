use f2s_application::rig::{
    ReparentBoneCommand, SetBoneTransformCommand, SetPivotCommand, SetSlotCommand,
    SetSocketCommand, approve_rig_candidate, diagnose_rig_candidate, reparent_bone,
    rig_approval_payload, set_bone_transform, set_layer_pivot, set_slot, set_socket,
    temporary_rig::{RigIssue, RigIssueSeverity, TemporaryRigSnapshot},
};
use f2s_domain::{
    layers::{Layer, LayerRole, LayerSet},
    master::{GripMode, PrimaryWeaponSpec, WeaponHand, WeaponSizeClass},
    rig::{
        RigApprovalState, RigCanvas, build_default_side_view_humanoid_rig,
        constraints::ConstraintCapability,
        layer_set_approval_payload_sha256,
        pivots_sockets::{LocalPoint, Socket, SocketKind},
    },
};
mod common;

fn capability() -> ConstraintCapability {
    ConstraintCapability::from_verified_manifest(
        include_bytes!("../../../fixtures/m00/spine42-probe/capability-manifest.json"),
        &[
            (
                "rig-ir.json",
                include_bytes!("../../../fixtures/m00/spine42-probe/rig-ir.json"),
            ),
            (
                "skeleton.json",
                include_bytes!("../../../fixtures/m00/spine42-probe/skeleton.json"),
            ),
        ],
    )
    .unwrap()
}

fn weapon() -> PrimaryWeaponSpec {
    PrimaryWeaponSpec {
        weapon_type: "katana".into(),
        grip_mode: GripMode::OneHand,
        weapon_hand: WeaponHand::NearHand,
        socket_semantic: "primary-grip".into(),
        size_class: WeaponSizeClass::Medium,
        silhouette_constraints: "single straight blade, readable side silhouette".into(),
    }
}

fn approved_layers() -> LayerSet {
    let layers = LayerRole::REQUIRED_V1
        .iter()
        .enumerate()
        .map(|(index, role)| Layer {
            layer_id: format!("layer-{index:02}"),
            name: format!("{role:?}"),
            role: *role,
            attachment_sha256: format!("{index:064x}"),
            mask_sha256: format!("{:064x}", index + 32),
            visible: true,
            approved: true,
        })
        .collect();
    LayerSet {
        layer_set_id: "layers-v1".into(),
        master_id: "master-v1".into(),
        revision: 7,
        layers,
        approval_state: "APPROVED".into(),
    }
}

fn candidate() -> (LayerSet, f2s_domain::rig::RigCandidate) {
    let layers = approved_layers();
    let hash = layer_set_approval_payload_sha256(&layers).unwrap();
    let candidate = build_default_side_view_humanoid_rig(
        "rig-v1",
        &layers,
        hash,
        weapon(),
        RigCanvas {
            width_px: 1024,
            height_px: 1024,
        },
        capability(),
    )
    .unwrap();
    (layers, candidate)
}

#[test]
fn default_builder_is_deterministic_complete_and_serializable() {
    let (layers, first) = candidate();
    let hash = layer_set_approval_payload_sha256(&layers).unwrap();
    let second = build_default_side_view_humanoid_rig(
        "rig-v1",
        &layers,
        hash,
        weapon(),
        first.canvas,
        capability(),
    )
    .unwrap();
    assert_eq!(first, second);
    first.validate(&layers).unwrap();
    assert_eq!(first.slot_set.slots.len(), layers.layers.len());
    assert_eq!(first.pivots.len(), layers.layers.len());
    assert_eq!(first.meshes.len(), layers.layers.len());
    assert_eq!(first.weights.len(), layers.layers.len());
    assert_eq!(
        first
            .sockets
            .iter()
            .filter(|socket| socket.kind == SocketKind::PrimaryWeapon)
            .count(),
        1
    );
    let json = serde_json::to_vec(&first).unwrap();
    let reopened = serde_json::from_slice(&json).unwrap();
    assert_eq!(first, reopened);
}

#[test]
fn validation_fails_closed_for_every_cross_component_binding() {
    let (layers, value) = candidate();

    let mut broken = value.clone();
    broken.layer_set_approval_sha256 = "0".repeat(64);
    assert!(broken.validate(&layers).is_err());

    let mut broken = value.clone();
    broken.slot_set.slots.pop();
    assert!(broken.validate(&layers).is_err());

    let mut broken = value.clone();
    broken.pivots.pop();
    assert!(broken.validate(&layers).is_err());

    let mut broken = value.clone();
    broken.sockets.push(Socket {
        socket_id: "second-weapon".into(),
        bone_id: "hand-back".into(),
        kind: SocketKind::PrimaryWeapon,
        point: LocalPoint {
            x_milli_px: 0,
            y_milli_px: 0,
        },
        semantic: "primary-grip".into(),
    });
    assert!(broken.validate(&layers).is_err());

    let mut broken = value.clone();
    broken.weights.pop();
    assert!(broken.validate(&layers).is_err());

    let mut broken = value.clone();
    broken.constraint_capability.source_hashes_verified = false;
    assert!(broken.validate(&layers).is_err());

    let mut broken = value.clone();
    broken.constraint_capability.manifest_sha256 = "G".repeat(64);
    assert!(broken.validate(&layers).is_err());

    let mut changed_layers = layers.clone();
    changed_layers.revision += 1;
    assert!(value.validate(&changed_layers).is_err());
}

#[test]
fn editor_commands_advance_only_the_aggregate_and_affected_subrevision() {
    let (_layers, mut value) = candidate();
    value.approval_state = RigApprovalState::Approved;
    let original_refs = value.revision_refs();
    let mut rest = value
        .bone_tree
        .bones
        .iter()
        .find(|bone| bone.bone_id == "head")
        .unwrap()
        .rest;
    rest.x_milli_px += 1_000;
    set_bone_transform(
        &mut value,
        SetBoneTransformCommand {
            expected_revision: 1,
            bone_id: "head".into(),
            rest,
        },
    )
    .unwrap();
    assert_eq!(value.revision, 2);
    assert_eq!(
        value.bone_tree.revision,
        original_refs.bone_tree_revision + 1
    );
    assert_eq!(
        value.pivot_socket_revision,
        original_refs.pivot_socket_revision
    );
    assert_eq!(value.approval_state, RigApprovalState::Pending);

    let after_transform = value.clone();
    assert!(
        set_layer_pivot(
            &mut value,
            SetPivotCommand {
                expected_revision: 1,
                layer_id: "layer-00".into(),
                point: LocalPoint {
                    x_milli_px: 10,
                    y_milli_px: 20,
                },
            },
        )
        .is_err()
    );
    assert_eq!(value, after_transform, "stale command must be atomic");

    set_layer_pivot(
        &mut value,
        SetPivotCommand {
            expected_revision: 2,
            layer_id: "layer-00".into(),
            point: LocalPoint {
                x_milli_px: 10,
                y_milli_px: 20,
            },
        },
    )
    .unwrap();
    assert_eq!(value.revision, 3);
    assert_eq!(
        value.pivot_socket_revision,
        original_refs.pivot_socket_revision + 1
    );

    set_socket(
        &mut value,
        SetSocketCommand {
            expected_revision: 3,
            socket_id: "primary-weapon".into(),
            bone_id: "hand-front".into(),
            point: LocalPoint {
                x_milli_px: 250,
                y_milli_px: -100,
            },
            semantic: "primary-grip".into(),
        },
    )
    .unwrap();
    assert_eq!(value.revision, 4);
    assert_eq!(
        value.pivot_socket_revision,
        original_refs.pivot_socket_revision + 2
    );

    reparent_bone(
        &mut value,
        ReparentBoneCommand {
            expected_revision: 4,
            bone_id: "head".into(),
            parent_id: "root".into(),
        },
    )
    .unwrap();
    assert_eq!(value.revision, 5);
    assert_eq!(
        value.bone_tree.revision,
        original_refs.bone_tree_revision + 2
    );
}

#[test]
fn invalid_edits_and_revision_overflow_roll_back_completely() {
    let (_layers, mut value) = candidate();
    let before_cycle = value.clone();
    assert!(
        reparent_bone(
            &mut value,
            ReparentBoneCommand {
                expected_revision: 1,
                bone_id: "root".into(),
                parent_id: "head".into(),
            },
        )
        .is_err()
    );
    assert_eq!(value, before_cycle);

    let before_socket = value.clone();
    assert!(
        set_socket(
            &mut value,
            SetSocketCommand {
                expected_revision: 1,
                socket_id: "primary-weapon".into(),
                bone_id: "missing".into(),
                point: LocalPoint {
                    x_milli_px: 0,
                    y_milli_px: 0,
                },
                semantic: "primary-grip".into(),
            },
        )
        .is_err()
    );
    assert_eq!(value, before_socket);

    value.revision = u64::MAX;
    let before_overflow = value.clone();
    let mut rest = value.bone_tree.bones[1].rest;
    let bone_id = value.bone_tree.bones[1].bone_id.clone();
    rest.x_milli_px += 1;
    assert!(
        set_bone_transform(
            &mut value,
            SetBoneTransformCommand {
                expected_revision: u64::MAX,
                bone_id,
                rest,
            },
        )
        .is_err()
    );
    assert_eq!(value, before_overflow);
}

#[test]
fn slot_binding_and_draw_order_are_revisioned_and_atomic() {
    let (_layers, mut value) = candidate();
    let slot = value.slot_set.slots[0].clone();
    let previous_slot_revision = value.slot_set.revision;
    set_slot(
        &mut value,
        SetSlotCommand {
            expected_revision: 1,
            slot_id: slot.slot_id.clone(),
            bone_id: "torso".into(),
            draw_key: 10_000,
        },
    )
    .unwrap();
    assert_eq!(value.revision, 2);
    assert_eq!(value.slot_set.revision, previous_slot_revision + 1);
    let updated = value
        .slot_set
        .slots
        .iter()
        .find(|candidate| candidate.slot_id == slot.slot_id)
        .unwrap();
    assert_eq!(updated.bone_id, "torso");
    assert_eq!(updated.draw_key, 10_000);

    let before = value.clone();
    let duplicate_draw_key = value.slot_set.slots[1].draw_key;
    assert!(
        set_slot(
            &mut value,
            SetSlotCommand {
                expected_revision: 2,
                slot_id: slot.slot_id.clone(),
                bone_id: "torso".into(),
                draw_key: duplicate_draw_key,
            },
        )
        .is_err()
    );
    assert_eq!(value, before);
    assert!(
        set_slot(
            &mut value,
            SetSlotCommand {
                expected_revision: 1,
                slot_id: slot.slot_id,
                bone_id: "root".into(),
                draw_key: 20_000,
            },
        )
        .is_err()
    );
    assert_eq!(value, before);
}

#[test]
fn approval_hashes_the_normalized_full_aggregate_and_blocks_p0_p1() {
    let (layers, mut value) = candidate();
    let payload = rig_approval_payload(&value).unwrap();
    let mut reordered = value.clone();
    reordered.bone_tree.bones.reverse();
    reordered.slot_set.slots.reverse();
    reordered.pivots.reverse();
    reordered.meshes.reverse();
    reordered.weights.reverse();
    assert_eq!(payload, rig_approval_payload(&reordered).unwrap());

    for severity in [RigIssueSeverity::P0, RigIssueSeverity::P1] {
        let diagnostics = TemporaryRigSnapshot::new(
            value.revision_refs(),
            vec![RigIssue {
                code: "BLOCKING".into(),
                target: "rig".into(),
                severity,
                fix_target: "rig-editor".into(),
            }],
        );
        let actor = common::human("approve-rig", &payload, "rig-artist");
        assert!(
            approve_rig_candidate(
                &mut value,
                &layers,
                &diagnostics,
                actor,
                "2026-07-11T12:00:00Z",
            )
            .is_err()
        );
        assert_eq!(value.approval_state, RigApprovalState::Pending);
    }

    let unverified = TemporaryRigSnapshot::new(value.revision_refs(), vec![]);
    let actor = common::human("approve-rig", &payload, "rig-artist");
    assert!(
        approve_rig_candidate(
            &mut value,
            &layers,
            &unverified,
            actor,
            "2026-07-11T12:00:00Z",
        )
        .is_err(),
        "a caller-supplied empty issue list must not pass the production gate"
    );

    let diagnostics = diagnose_rig_candidate(&value, &layers);
    assert!(diagnostics.completed);
    assert!(diagnostics.is_current_for(&value));
    assert!(!diagnostics.has_blocking_issues());
    let actor = common::human("approve-rig", &payload, "rig-artist");
    let approval = approve_rig_candidate(
        &mut value,
        &layers,
        &diagnostics,
        actor,
        "2026-07-11T12:00:00Z",
    )
    .unwrap();
    assert_eq!(approval.gate_id, "rig");
    assert_eq!(approval.target_sha256, payload);
    assert!(approval.is_valid_for("rig-v1", 1, &payload));
    assert_eq!(value.approval_state, RigApprovalState::Approved);
}

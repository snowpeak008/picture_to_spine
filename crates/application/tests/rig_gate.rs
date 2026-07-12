use f2s_application::rig::{
    rig_approval::{RigApprovalRequest, approve_rig, invalidate_if_changed},
    temporary_rig::{RigIssue, RigIssueSeverity, TemporaryRigSnapshot},
};
use f2s_domain::canonical::canonical_sha256;
use f2s_domain::rig::{
    RigRevisionRefs, SPINE_CAPABILITY_ID,
    bone_tree::{BoneNode, BoneTree, RestTransform},
    constraints::{ConstraintCapability, ConstraintKind, RigConstraint, validate_constraints},
    mesh::{Mesh, Triangle, Vertex},
    pivots_sockets::{LocalPoint, Socket, SocketKind, validate_pivots_and_sockets},
    slots::{Slot, SlotSet},
    weights::WeightSet,
};
use std::collections::BTreeMap;
mod common;

fn bones() -> BoneTree {
    BoneTree {
        revision: 1,
        bones: vec![
            BoneNode {
                bone_id: "root".into(),
                name: "Root".into(),
                parent_id: None,
                rest: RestTransform::default(),
            },
            BoneNode {
                bone_id: "hand".into(),
                name: "Hand".into(),
                parent_id: Some("root".into()),
                rest: RestTransform::default(),
            },
        ],
    }
}
fn revisions() -> RigRevisionRefs {
    RigRevisionRefs {
        layer_set_revision: 1,
        bone_tree_revision: 1,
        slot_revision: 1,
        pivot_socket_revision: 1,
        mesh_revision: 1,
        weight_revision: 1,
        constraint_revision: 1,
    }
}
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

#[test]
fn rig_components_validate_as_one_candidate() {
    let bones = bones();
    bones.validate().unwrap();
    SlotSet {
        revision: 1,
        slots: vec![Slot {
            slot_id: "body-slot".into(),
            layer_id: "body".into(),
            bone_id: "root".into(),
            draw_key: 0,
        }],
    }
    .validate(&["body".into()], &bones)
    .unwrap();
    validate_pivots_and_sockets(
        &[],
        &[Socket {
            socket_id: "weapon".into(),
            bone_id: "hand".into(),
            kind: SocketKind::PrimaryWeapon,
            point: LocalPoint {
                x_milli_px: 0,
                y_milli_px: 0,
            },
            semantic: "single-sword".into(),
        }],
        &bones,
        Some("single-sword"),
    )
    .unwrap();
    let mesh = Mesh {
        mesh_id: "body-mesh".into(),
        layer_id: "body".into(),
        topology_revision: 1,
        vertices: vec![
            Vertex {
                vertex_id: 1,
                x_milli_px: 0,
                y_milli_px: 0,
                u_ppm: 0,
                v_ppm: 0,
            },
            Vertex {
                vertex_id: 2,
                x_milli_px: 1000,
                y_milli_px: 0,
                u_ppm: 1_000_000,
                v_ppm: 0,
            },
            Vertex {
                vertex_id: 3,
                x_milli_px: 0,
                y_milli_px: 1000,
                u_ppm: 0,
                v_ppm: 1_000_000,
            },
        ],
        triangles: vec![Triangle(1, 2, 3)],
    };
    mesh.validate().unwrap();
    let weights = WeightSet {
        mesh_id: "body-mesh".into(),
        topology_revision: 1,
        by_vertex: BTreeMap::from([
            (
                1,
                vec![f2s_domain::rig::weights::BoneWeight {
                    bone_id: "root".into(),
                    weight_ppm: 1_000_000,
                }],
            ),
            (
                2,
                vec![f2s_domain::rig::weights::BoneWeight {
                    bone_id: "root".into(),
                    weight_ppm: 1_000_000,
                }],
            ),
            (
                3,
                vec![f2s_domain::rig::weights::BoneWeight {
                    bone_id: "root".into(),
                    weight_ppm: 1_000_000,
                }],
            ),
        ]),
    };
    weights.validate(&mesh, &bones).unwrap();
    validate_constraints(
        &[RigConstraint {
            constraint_id: "follow-hand".into(),
            kind: ConstraintKind::Transform,
            constrained_bone_id: "hand".into(),
            target_bone_id: "root".into(),
            mix_ppm: 500_000,
            order: 0,
        }],
        &bones,
        &capability(),
    )
    .unwrap();
}

#[test]
fn cycle_stale_and_non_human_approvals_fail_closed() {
    let mut tree = bones();
    assert!(tree.reparent("root", "hand").is_err());
    let diagnostics = TemporaryRigSnapshot::new(revisions(), vec![]);
    let request = RigApprovalRequest {
        rig_id: "rig-1".into(),
        revisions: revisions(),
        approved_layer_set_hash: "layer-hash".into(),
        expected_revision: 1,
        capability_id: SPINE_CAPABILITY_ID.into(),
    };
    let actor = common::human(
        "approve-rig",
        &canonical_sha256(&request.revisions).unwrap(),
        "artist-01",
    );
    assert!(approve_rig(request.clone(), 2, &diagnostics, actor).is_err());
    let actor = common::human(
        "approve-rig",
        &canonical_sha256(&request.revisions).unwrap(),
        "artist-01",
    );
    let mut approval = approve_rig(request, 1, &diagnostics, actor).unwrap();
    assert!(approval.valid);
    assert!(invalidate_if_changed(
        &mut approval,
        "different",
        "layer-hash"
    ));
    assert!(!approval.valid);
    let blocked = TemporaryRigSnapshot::new(
        revisions(),
        vec![RigIssue {
            code: "WEIGHT_GAP".into(),
            target: "vertex:2".into(),
            severity: RigIssueSeverity::P1,
            fix_target: "weight-editor".into(),
        }],
    );
    assert!(blocked.has_blocking_issues());
}

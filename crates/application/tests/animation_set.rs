use f2s_application::{
    animation::{
        approve_action_hit, approve_action_poses, initialize_animation_set, put_track,
        set_hit_frame_marker, set_review_pose_tick,
    },
    motion::initialize_motion_content,
    rig::rig_approval_payload,
};
use f2s_domain::{
    animation::clip::{Curve, Keyframe},
    layers::{Layer, LayerRole, LayerSet},
    master::{GripMode, PrimaryWeaponSpec, StyleSpec, WeaponHand, WeaponSizeClass},
    rig::{
        RigCanvas, build_default_side_view_humanoid_rig, constraints::ConstraintCapability,
        layer_set_approval_payload_sha256,
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

fn style() -> StyleSpec {
    StyleSpec {
        revision: 1,
        viewpoint: "side-view".into(),
        rendering_style: "anime-clean".into(),
        outline: "dark-clean".into(),
        palette_notes: "blue".into(),
        identity_notes: "stable character".into(),
        primary_weapon: Some(PrimaryWeaponSpec {
            weapon_type: "spear".into(),
            grip_mode: GripMode::TwoHand,
            weapon_hand: WeaponHand::BothHands,
            socket_semantic: "primary-grip".into(),
            size_class: WeaponSizeClass::Large,
            silhouette_constraints: "single long weapon remains readable".into(),
        }),
    }
}

fn approved_layers() -> LayerSet {
    LayerSet {
        layer_set_id: "layers-v1".into(),
        master_id: "master-v1".into(),
        revision: 1,
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
                approved: true,
            })
            .collect(),
        approval_state: "APPROVED".into(),
    }
}

fn fixture() -> (
    f2s_domain::motion::content::MotionContent,
    f2s_domain::rig::RigCandidate,
    f2s_domain::animation::set::AnimationSet,
) {
    let style = style();
    let motion = initialize_motion_content(&style).unwrap();
    let layers = approved_layers();
    let rig = build_default_side_view_humanoid_rig(
        "rig-v1",
        &layers,
        layer_set_approval_payload_sha256(&layers).unwrap(),
        style.primary_weapon.unwrap(),
        RigCanvas {
            width_px: 512,
            height_px: 512,
        },
        capability(),
    )
    .unwrap();
    let rig_hash = rig_approval_payload(&rig).unwrap();
    let animation = initialize_animation_set(&motion, &rig, &rig_hash).unwrap();
    (motion, rig, animation)
}

#[test]
fn initialization_binds_every_required_pose_to_a_real_keyframe_and_three_hits() {
    let (motion, rig, animation) = fixture();
    animation
        .validate(
            &motion,
            &rig.bone_tree,
            &rig.slot_set,
            &rig.sockets
                .iter()
                .map(|socket| socket.socket_id.clone())
                .collect::<Vec<_>>(),
        )
        .unwrap();
    assert_eq!(animation.clips.len(), 10);
    assert_eq!(animation.review_pose_markers.len(), motion.assets.len());
    assert_eq!(animation.gameplay_markers.len(), 3);
}

#[test]
fn pose_and_hit_approvals_bind_rig_clip_real_pose_ticks_and_socket() {
    let (motion, _rig, animation) = fixture();
    let pose_payload = animation.pose_payload(&motion, "attack_01").unwrap();
    let pose_actor = common::human("approve-key-poses", &pose_payload, "animator");
    let pose = approve_action_poses(
        &animation,
        &motion,
        "attack_01",
        pose_actor,
        "2026-07-11T00:00:00Z",
    )
    .unwrap();
    let hit_payload = animation.hit_payload("attack_01").unwrap();
    let hit_actor = common::human("approve-hit-frame", &hit_payload, "animator");
    let hit = approve_action_hit(
        &animation,
        &motion,
        "attack_01",
        &pose,
        hit_actor,
        "2026-07-11T00:00:01Z",
    )
    .unwrap();
    assert_eq!(pose.gate_id, "poses");
    assert_eq!(hit.gate_id, "hits");
    assert!(animation.hit_payload("idle").is_err());
}

#[test]
fn edits_are_atomic_and_make_old_pose_approval_stale() {
    let (motion, rig, mut animation) = fixture();
    let payload = animation.pose_payload(&motion, "idle").unwrap();
    let actor = common::human("approve-key-poses", &payload, "animator");
    let approval =
        approve_action_poses(&animation, &motion, "idle", actor, "2026-07-11T00:00:00Z").unwrap();
    let before = animation.clone();
    assert!(set_review_pose_tick(&mut animation, &motion, &rig, "idle", "inhale", 1_234).is_err());
    assert_eq!(animation, before);

    let mut track = animation.clips[0].tracks[0].clone();
    track.keyframes.push(Keyframe {
        keyframe_id: "manual-idle-key".into(),
        tick: 1_234,
        values_milli: vec![0, 0],
        curve: Curve::Linear,
        bezier_milli: None,
    });
    put_track(&mut animation, &motion, &rig, "idle", track).unwrap();
    let new_payload = animation.pose_payload(&motion, "idle").unwrap();
    assert_ne!(payload, new_payload);
    assert!(!approval.is_valid_for("idle", animation.clips[0].revision, &new_payload));
}

#[test]
fn hit_frame_is_human_editable_inside_contact_phase_and_revision_checked() {
    let (motion, rig, mut animation) = fixture();
    let socket_id = rig
        .sockets
        .iter()
        .find(|socket| socket.kind == f2s_domain::rig::pivots_sockets::SocketKind::PrimaryWeapon)
        .unwrap()
        .socket_id
        .clone();
    let before = animation.clone();
    set_hit_frame_marker(
        &mut animation,
        &motion,
        &rig,
        before.revision,
        "attack_01",
        9_000,
        &socket_id,
    )
    .unwrap();
    assert_eq!(animation.revision, before.revision + 1);
    assert_eq!(animation.clip("attack_01").unwrap().revision, 0);
    let marker = animation
        .gameplay_markers
        .iter()
        .find(|marker| marker.action_key == "attack_01")
        .unwrap();
    assert_eq!((marker.start_tick, marker.end_tick), (9_000, 9_000));

    let edited = animation.clone();
    assert!(
        set_hit_frame_marker(
            &mut animation,
            &motion,
            &rig,
            before.revision,
            "attack_01",
            10_000,
            &socket_id,
        )
        .is_err()
    );
    assert_eq!(animation, edited);
    assert!(
        set_hit_frame_marker(
            &mut animation,
            &motion,
            &rig,
            edited.revision,
            "attack_01",
            1_000,
            &socket_id,
        )
        .is_err()
    );
    assert_eq!(animation, edited);
    assert!(
        set_hit_frame_marker(
            &mut animation,
            &motion,
            &rig,
            edited.revision,
            "idle",
            1_000,
            &socket_id,
        )
        .is_err()
    );
    assert_eq!(animation, edited);
}

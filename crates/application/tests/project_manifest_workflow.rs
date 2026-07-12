use f2s_application::{
    animation::{
        approve_action_hit, approve_action_poses, initialize_animation_set, put_track,
        set_hit_frame_marker, set_review_pose_tick,
    },
    approvals::approve as approve_master,
    layers::{approve_layers, layer_approval_payload},
    master::create_master,
    motion::{
        approve_key_pose_asset, bind_key_pose_image, initialize_motion_content, replace_motion_spec,
    },
    rig::{
        SetBoneTransformCommand, approve_rig_candidate, diagnose_rig_candidate,
        rig_approval_payload, set_bone_transform,
    },
};
use f2s_domain::{
    ACTION_KEYS,
    animation::clip::{Curve, Keyframe},
    governance::Approval,
    import::SourceArtifact,
    layers::{Layer, LayerRole, LayerSet, PixelOrigin, PixelProvenance, RecompositionMetrics},
    master::{GripMode, PrimaryWeaponSpec, StyleSpec, WeaponHand, WeaponSizeClass},
    project::{ExportRecord, ProjectIdentity, ProjectManifest},
    rig::{RigCanvas, build_default_side_view_humanoid_rig, constraints::ConstraintCapability},
};

mod common;

const REVIEWED_AT: &str = "2026-07-11T00:00:00Z";

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
    .expect("verified Spine 4.2 capability fixture")
}

fn source() -> SourceArtifact {
    SourceArtifact {
        artifact_id: "source-main".into(),
        sha256: "a".repeat(64),
        media_type: "image/png".into(),
        width: 512,
        height: 512,
        byte_length: 16_384,
        bit_depth: 8,
        provenance: "user-local".into(),
        approval_state: "UNAPPROVED".into(),
    }
}

fn style() -> StyleSpec {
    StyleSpec {
        revision: 1,
        viewpoint: "side-view".into(),
        rendering_style: "anime-clean".into(),
        outline: "dark-clean".into(),
        palette_notes: "blue and silver".into(),
        identity_notes: "stable original character identity".into(),
        primary_weapon: Some(PrimaryWeaponSpec {
            weapon_type: "single-sword".into(),
            grip_mode: GripMode::OneHand,
            weapon_hand: WeaponHand::NearHand,
            socket_semantic: "primary-grip".into(),
            size_class: WeaponSizeClass::Medium,
            silhouette_constraints: "single readable side-view blade".into(),
        }),
    }
}

fn pending_layer_set(master_id: &str) -> LayerSet {
    LayerSet {
        layer_set_id: "layers-main".into(),
        master_id: master_id.into(),
        revision: 1,
        layers: LayerRole::REQUIRED_V1
            .iter()
            .enumerate()
            .map(|(index, role)| Layer {
                layer_id: format!("layer-{index:02}"),
                name: format!("{role:?}"),
                role: *role,
                attachment_sha256: format!("{:064x}", index + 1),
                mask_sha256: format!("{:064x}", index + 101),
                visible: true,
                approved: false,
            })
            .collect(),
        approval_state: "PENDING".into(),
    }
}

fn provenance(set: &LayerSet, source_sha256: &str) -> Vec<PixelProvenance> {
    set.layers
        .iter()
        .map(|layer| PixelProvenance {
            artifact_sha256: layer.attachment_sha256.clone(),
            origin: PixelOrigin::Source,
            source_sha256: source_sha256.into(),
            prompt_pack_id: None,
            receipt_ref: None,
            accepted_by: None,
            acceptance_attestation_sha256: None,
        })
        .collect()
}

fn passing_recomposition() -> RecompositionMetrics {
    RecompositionMetrics {
        missing_pixels: 0,
        overlap_pixels: 0,
        changed_visible_pixels: 0,
        alpha_error_pixels: 0,
        empty_layer_masks: 0,
    }
}

/// Builds the complete production workflow up to valid MotionContent. Every
/// approval is produced by the same application service used by the host, including a verified
/// human actor binding; the fixture never pushes an approval directly into the manifest.
fn manifest_at_motion() -> ProjectManifest {
    let mut project = ProjectManifest::new(ProjectIdentity::new("workflow-e2e").unwrap());
    let source = source();
    project.add_source(source.clone());

    let master = create_master(&source, style()).expect("create master");
    project.set_master(master);
    let master_payload = project
        .active_master
        .as_ref()
        .expect("active master")
        .approval_payload_sha256()
        .expect("master approval payload");
    let actor = common::human("approve-master", &master_payload, "master-reviewer");
    let master_approval = approve_master(
        project.active_master.as_mut().expect("active master"),
        actor,
        REVIEWED_AT,
    )
    .expect("approve master");
    project
        .record_master_approval(master_approval)
        .expect("record master approval");

    let layer_set = pending_layer_set(
        &project
            .active_master
            .as_ref()
            .expect("active master")
            .master_id,
    );
    let pixel_provenance = provenance(&layer_set, &source.sha256);
    project
        .set_layer_set(layer_set, pixel_provenance)
        .expect("set layer candidate");
    let layer_payload =
        layer_approval_payload(project.active_layer_set.as_ref().expect("active layer set"))
            .expect("layer approval payload");
    let actor = common::human("approve-layers", &layer_payload, "layer-reviewer");
    let layer_provenance = project.layer_provenance.clone();
    let layer_approval = approve_layers(
        project.active_layer_set.as_mut().expect("active layer set"),
        passing_recomposition(),
        &layer_provenance,
        actor,
        REVIEWED_AT,
    )
    .expect("approve layers");
    project
        .record_layer_approval(layer_approval)
        .expect("record layer approval");

    let layer_set = project
        .active_layer_set
        .as_ref()
        .expect("active layer set")
        .clone();
    let layer_approval_sha256 = project
        .current_layer_approval()
        .expect("current layer approval")
        .target_sha256
        .clone();
    let weapon = project
        .active_master
        .as_ref()
        .and_then(|master| master.style_spec.primary_weapon.clone())
        .expect("approved primary weapon");
    let rig = build_default_side_view_humanoid_rig(
        "rig-main",
        &layer_set,
        layer_approval_sha256,
        weapon,
        RigCanvas {
            width_px: source.width,
            height_px: source.height,
        },
        capability(),
    )
    .expect("build default rig");
    project.set_rig(rig).expect("set rig candidate");
    let rig_payload = rig_approval_payload(project.active_rig.as_ref().expect("active rig"))
        .expect("rig approval payload");
    let actor = common::human("approve-rig", &rig_payload, "rig-reviewer");
    let diagnostics =
        diagnose_rig_candidate(project.active_rig.as_ref().expect("active rig"), &layer_set);
    let rig_approval = approve_rig_candidate(
        project.active_rig.as_mut().expect("active rig"),
        &layer_set,
        &diagnostics,
        actor,
        REVIEWED_AT,
    )
    .expect("approve rig");
    project
        .record_rig_approval(rig_approval)
        .expect("record rig approval");

    let motion = initialize_motion_content(
        &project
            .active_master
            .as_ref()
            .expect("active master")
            .style_spec,
    )
    .expect("initialize motion content");
    project
        .set_motion_content(motion)
        .expect("set motion content");
    project
        .validate_cross_aggregate()
        .expect("valid aggregate chain through MotionContent");
    project
}

fn manifest_at_animation() -> ProjectManifest {
    let mut project = manifest_at_motion();
    let approved_rig_sha256 = project
        .current_rig_approval()
        .expect("current rig approval")
        .target_sha256
        .clone();
    let animation = initialize_animation_set(
        project.motion_content.as_ref().expect("motion content"),
        project.active_rig.as_ref().expect("active rig"),
        &approved_rig_sha256,
    )
    .expect("initialize animation set");
    project
        .set_animation_set(animation)
        .expect("set animation set");
    project
        .validate_cross_aggregate()
        .expect("valid complete aggregate chain");
    project
}

fn approve_pose(project: &mut ProjectManifest, action_key: &str) -> Approval {
    let animation = project.animation_set.as_ref().expect("animation set");
    let motion = project.motion_content.as_ref().expect("motion content");
    let payload = animation
        .pose_payload(motion, action_key)
        .expect("pose payload");
    let actor = common::human("approve-key-poses", &payload, "animation-reviewer");
    let approval = approve_action_poses(animation, motion, action_key, actor, REVIEWED_AT)
        .expect("approve action poses");
    project
        .record_pose_approval(approval.clone())
        .expect("record pose approval");
    approval
}

fn approve_hit(project: &mut ProjectManifest, action_key: &str) -> Approval {
    let pose = project
        .current_pose_approval(action_key)
        .expect("current pose approval")
        .clone();
    let animation = project.animation_set.as_ref().expect("animation set");
    let motion = project.motion_content.as_ref().expect("motion content");
    let payload = animation.hit_payload(action_key).expect("hit payload");
    let actor = common::human("approve-hit-frame", &payload, "gameplay-reviewer");
    let approval = approve_action_hit(animation, motion, action_key, &pose, actor, REVIEWED_AT)
        .expect("approve attack hit");
    project
        .record_hit_approval(approval.clone())
        .expect("record hit approval");
    approval
}

fn manifest_with_approval_closure() -> ProjectManifest {
    let mut project = manifest_at_animation();
    for action in ACTION_KEYS {
        approve_pose(&mut project, action);
    }
    for action in ["attack_01", "attack_02", "attack_03"] {
        approve_hit(&mut project, action);
    }
    assert!(project.approval_closure_complete());
    project
}

#[test]
fn layer_rig_motion_animation_chain_is_valid_and_export_needs_ten_poses_and_three_hits() {
    let mut project = manifest_at_animation();
    assert_eq!(project.workflow_stage, "animation");
    assert!(project.current_master_approval().is_some());
    assert!(project.current_layer_approval().is_some());
    assert!(project.current_rig_approval().is_some());
    assert!(!project.approval_closure_complete());

    for (index, action) in ACTION_KEYS.iter().enumerate() {
        approve_pose(&mut project, action);
        assert_eq!(
            project
                .approval_log
                .iter()
                .filter(|approval| approval.gate_id == "poses" && !approval.invalidated)
                .count(),
            index + 1
        );
        assert!(
            !project.approval_closure_complete(),
            "pose approvals alone must never unlock export"
        );
    }

    for (index, action) in ["attack_01", "attack_02", "attack_03"].iter().enumerate() {
        approve_hit(&mut project, action);
        assert_eq!(
            project
                .approval_log
                .iter()
                .filter(|approval| approval.gate_id == "hits" && !approval.invalidated)
                .count(),
            index + 1
        );
        assert_eq!(project.approval_closure_complete(), index == 2);
    }

    assert_eq!(project.workflow_stage, "export");
    project.validate_cross_aggregate().unwrap();
}

#[test]
fn first_animation_set_preserves_a_current_human_key_pose_asset_approval() {
    let mut project = manifest_at_motion();
    let mut motion = project.motion_content.clone().expect("motion content");
    let asset_spec_id = motion.assets[0].asset_spec_id.clone();
    let mut key_pose_source = source();
    key_pose_source.artifact_id = "key-pose-source".into();
    key_pose_source.sha256 = "b".repeat(64);
    let binding = bind_key_pose_image(&mut motion, &key_pose_source, &asset_spec_id)
        .expect("bind validated local key-pose image");
    project
        .set_motion_content(motion)
        .expect("store imported key-pose binding");

    let motion = project.motion_content.as_ref().expect("motion content");
    let payload = motion
        .binding_approval_payload(&binding)
        .expect("key-pose approval payload");
    let actor = common::human("approve-key-pose-asset", &payload, "key-pose-reviewer");
    let asset_approval =
        approve_key_pose_asset(motion, &binding.binding_id, actor, "2026-07-11T00:00:01Z")
            .expect("approve key-pose asset");
    project
        .record_key_pose_asset_approval(asset_approval.clone())
        .expect("record key-pose asset approval");

    let approved_rig_sha256 = project
        .current_rig_approval()
        .expect("current rig approval")
        .target_sha256
        .clone();
    let animation = initialize_animation_set(
        project.motion_content.as_ref().expect("motion content"),
        project.active_rig.as_ref().expect("active rig"),
        &approved_rig_sha256,
    )
    .expect("initialize first AnimationSet");
    project
        .set_animation_set(animation)
        .expect("store first AnimationSet");

    let current = project
        .approval_log
        .iter()
        .find(|approval| approval.approval_id == asset_approval.approval_id)
        .expect("recorded key-pose approval");
    assert!(!current.invalidated);
    assert!(current.is_valid_for(&binding.binding_id, binding.revision, &payload));
    project
        .validate_cross_aggregate()
        .expect("approved key-pose asset remains valid after AnimationSet initialization");
}

#[test]
fn rig_or_motion_edits_invalidate_all_pose_and_hit_approvals() {
    let mut rig_edited = manifest_with_approval_closure();
    let mut revised_rig = rig_edited.active_rig.clone().expect("active rig");
    let mut rest = revised_rig
        .bone_tree
        .bones
        .iter()
        .find(|bone| bone.bone_id == "head")
        .expect("head bone")
        .rest;
    rest.x_milli_px += 1_000;
    let expected_revision = revised_rig.revision;
    set_bone_transform(
        &mut revised_rig,
        SetBoneTransformCommand {
            expected_revision,
            bone_id: "head".into(),
            rest,
        },
    )
    .expect("valid Rig edit");
    rig_edited.set_rig(revised_rig).expect("replace Rig");
    assert!(rig_edited.animation_set.is_none());
    assert!(rig_edited.current_rig_approval().is_none());
    assert!(
        ACTION_KEYS
            .iter()
            .all(|action| rig_edited.current_pose_approval(action).is_none())
    );
    assert!(
        ["attack_01", "attack_02", "attack_03"]
            .iter()
            .all(|action| rig_edited.current_hit_approval(action).is_none())
    );
    assert!(!rig_edited.approval_closure_complete());

    let mut motion_edited = manifest_with_approval_closure();
    let mut revised_motion = motion_edited
        .motion_content
        .clone()
        .expect("motion content");
    let mut revised_idle = revised_motion.specs[0].clone();
    revised_idle.silhouette_goal.push_str("，并强化呼吸轮廓");
    let approved_style = &motion_edited
        .active_master
        .as_ref()
        .expect("active master")
        .style_spec;
    replace_motion_spec(&mut revised_motion, approved_style, revised_idle)
        .expect("valid MotionSpec edit");
    motion_edited
        .set_motion_content(revised_motion)
        .expect("replace MotionContent");
    assert!(motion_edited.animation_set.is_none());
    assert!(motion_edited.current_rig_approval().is_some());
    assert!(
        ACTION_KEYS
            .iter()
            .all(|action| motion_edited.current_pose_approval(action).is_none())
    );
    assert!(
        ["attack_01", "attack_02", "attack_03"]
            .iter()
            .all(|action| motion_edited.current_hit_approval(action).is_none())
    );
    assert!(!motion_edited.approval_closure_complete());
}

#[test]
fn clip_and_review_marker_edits_invalidate_only_the_affected_action() {
    let mut project = manifest_with_approval_closure();
    let motion = project.motion_content.clone().expect("motion content");
    let rig = project.active_rig.clone().expect("active rig");
    let mut revised_animation = project.animation_set.clone().expect("animation set");
    let mut track = revised_animation
        .clip("attack_01")
        .expect("attack clip")
        .tracks[0]
        .clone();
    track.keyframes.push(Keyframe {
        keyframe_id: "key:attack_01:manual-adjustment".into(),
        tick: 1_000,
        values_milli: vec![250, 0],
        curve: Curve::Linear,
        bezier_milli: None,
    });
    put_track(&mut revised_animation, &motion, &rig, "attack_01", track).expect("valid clip edit");
    project
        .set_animation_set(revised_animation)
        .expect("replace AnimationSet");
    assert!(project.current_pose_approval("attack_01").is_none());
    assert!(project.current_hit_approval("attack_01").is_none());
    assert!(project.current_pose_approval("attack_02").is_some());
    assert!(project.current_hit_approval("attack_02").is_some());

    let mut project = manifest_with_approval_closure();
    let motion = project.motion_content.clone().expect("motion content");
    let rig = project.active_rig.clone().expect("active rig");
    let mut revised_animation = project.animation_set.clone().expect("animation set");
    let replacement_tick = revised_animation
        .review_pose_markers
        .iter()
        .find(|marker| marker.action_key == "attack_02" && marker.pose_key == "contact")
        .expect("contact pose marker")
        .tick;
    set_review_pose_tick(
        &mut revised_animation,
        &motion,
        &rig,
        "attack_02",
        "anticipation",
        replacement_tick,
    )
    .expect("valid pose marker edit");
    project
        .set_animation_set(revised_animation)
        .expect("replace AnimationSet");
    assert!(project.current_pose_approval("attack_02").is_none());
    assert!(project.current_hit_approval("attack_02").is_none());
    assert!(project.current_pose_approval("attack_03").is_some());
    assert!(project.current_hit_approval("attack_03").is_some());
    assert!(!project.approval_closure_complete());
}

#[test]
fn hit_marker_edit_invalidates_only_that_hit_gate_and_keeps_pose_current() {
    let mut project = manifest_with_approval_closure();
    let motion = project.motion_content.clone().expect("motion content");
    let rig = project.active_rig.clone().expect("active rig");
    let mut animation = project.animation_set.clone().expect("animation set");
    let socket_id = rig
        .sockets
        .iter()
        .find(|socket| socket.kind == f2s_domain::rig::pivots_sockets::SocketKind::PrimaryWeapon)
        .unwrap()
        .socket_id
        .clone();
    let expected_revision = animation.revision;
    set_hit_frame_marker(
        &mut animation,
        &motion,
        &rig,
        expected_revision,
        "attack_01",
        9_000,
        &socket_id,
    )
    .expect("edit hit frame inside contact phase");
    project
        .set_animation_set(animation)
        .expect("replace AnimationSet");
    assert!(project.current_pose_approval("attack_01").is_some());
    assert!(project.current_hit_approval("attack_01").is_none());
    assert!(project.current_hit_approval("attack_02").is_some());
    assert!(project.current_hit_approval("attack_03").is_some());
    assert!(!project.approval_closure_complete());
}

#[test]
fn key_pose_binding_change_invalidates_only_the_affected_pose_and_hit_chain() {
    let mut project = manifest_with_approval_closure();
    let mut motion = project.motion_content.clone().expect("motion content");
    let asset_spec_id = motion
        .assets
        .iter()
        .find(|asset| asset.action_key == "attack_01" && asset.pose_key == "contact")
        .unwrap()
        .asset_spec_id
        .clone();
    let mut key_pose_source = source();
    key_pose_source.artifact_id = "attack-01-contact-source".into();
    key_pose_source.sha256 = "c".repeat(64);
    bind_key_pose_image(&mut motion, &key_pose_source, &asset_spec_id).unwrap();
    project.set_motion_content(motion).unwrap();
    assert!(project.animation_set.is_some());
    assert!(project.current_pose_approval("attack_01").is_none());
    assert!(project.current_hit_approval("attack_01").is_none());
    assert!(project.current_pose_approval("attack_02").is_some());
    assert!(project.current_hit_approval("attack_02").is_some());
}

#[test]
fn non_attack_hit_and_stale_or_mismatched_approvals_fail_closed_atomically() {
    let mut project = manifest_with_approval_closure();
    let idle_pose = project
        .current_pose_approval("idle")
        .expect("idle pose approval")
        .clone();
    let animation = project.animation_set.as_ref().expect("animation set");
    let motion = project.motion_content.as_ref().expect("motion content");
    let actor = common::human("approve-hit-frame", &"f".repeat(64), "gameplay-reviewer");
    assert!(
        approve_action_hit(animation, motion, "idle", &idle_pose, actor, REVIEWED_AT,).is_err(),
        "a non-attack action must not have a hit approval"
    );

    let revision_before_forgery = project.revision;
    let approval_count_before_forgery = project.approval_log.len();
    let idle_revision = project
        .animation_set
        .as_ref()
        .unwrap()
        .clip("idle")
        .unwrap()
        .revision;
    let forged_hit = Approval {
        approval_id: "forged-idle-hit".into(),
        gate_id: "hits".into(),
        target_id: "idle".into(),
        target_revision: idle_revision,
        target_sha256: "f".repeat(64),
        actor_id: "forged-actor".into(),
        approved_at_utc: REVIEWED_AT.into(),
        invalidated: false,
    };
    assert!(project.record_hit_approval(forged_hit).is_err());
    assert_eq!(project.revision, revision_before_forgery);
    assert_eq!(project.approval_log.len(), approval_count_before_forgery);

    let stale_pose = project
        .current_pose_approval("attack_01")
        .expect("current pose approval")
        .clone();
    let motion = project.motion_content.clone().unwrap();
    let rig = project.active_rig.clone().unwrap();
    let mut revised_animation = project.animation_set.clone().unwrap();
    let mut track = revised_animation.clip("attack_01").unwrap().tracks[0].clone();
    track.keyframes.push(Keyframe {
        keyframe_id: "key:attack_01:stale-boundary".into(),
        tick: 1_001,
        values_milli: vec![251, 0],
        curve: Curve::Linear,
        bezier_milli: None,
    });
    put_track(&mut revised_animation, &motion, &rig, "attack_01", track).unwrap();
    project.set_animation_set(revised_animation).unwrap();
    let revision_before_stale = project.revision;
    let count_before_stale = project.approval_log.len();
    assert!(project.record_pose_approval(stale_pose).is_err());
    assert_eq!(project.revision, revision_before_stale);
    assert_eq!(project.approval_log.len(), count_before_stale);

    let mut invalidated_exact = project
        .current_pose_approval("idle")
        .expect("unaffected idle approval")
        .clone();
    invalidated_exact.approval_id = "forged-invalidated-pose".into();
    invalidated_exact.invalidated = true;
    let revision_before_invalidated = project.revision;
    assert!(project.record_pose_approval(invalidated_exact).is_err());
    assert_eq!(project.revision, revision_before_invalidated);

    let current_idle = project.current_pose_approval("idle").unwrap().clone();
    let mut wrong_hash = current_idle;
    wrong_hash.approval_id = "forged-wrong-hash".into();
    wrong_hash.target_sha256 = "e".repeat(64);
    assert!(project.record_pose_approval(wrong_hash).is_err());
}

#[test]
fn immutable_export_record_requires_current_approval_closure_and_exact_status_contract() {
    let mut project = manifest_with_approval_closure();
    let source_revision = project.revision;
    project
        .append_export_record(ExportRecord {
            export_id: "f2s-export-record-001".into(),
            snapshot_sha256: "a".repeat(64),
            source_project_revision: source_revision,
            status: "EXPORTED_UNVERIFIED".into(),
            checksums: std::collections::BTreeMap::from([("rig-ir.json".into(), "b".repeat(64))]),
            created_at_utc: REVIEWED_AT.into(),
            external_status: "EXPORTED_UNVERIFIED".into(),
        })
        .expect("current approval closure accepts an immutable candidate export record");
    assert_eq!(project.export_records.len(), 1);
    assert_eq!(project.revision, source_revision + 1);
    assert!(project.validate_cross_aggregate().is_ok());
    let mut tampered = project.clone();
    tampered.export_records[0].snapshot_sha256 = "ABC".into();
    assert!(tampered.validate_cross_aggregate().is_err());

    let mut invalid = manifest_with_approval_closure();
    let revision_before = invalid.revision;
    let result = invalid.append_export_record(ExportRecord {
        export_id: "f2s-export-record-002".into(),
        snapshot_sha256: "a".repeat(64),
        source_project_revision: revision_before,
        status: "VERIFIED".into(),
        checksums: std::collections::BTreeMap::from([("rig-ir.json".into(), "b".repeat(64))]),
        created_at_utc: REVIEWED_AT.into(),
        external_status: "VERIFIED".into(),
    });
    assert!(result.is_err());
    assert_eq!(invalid.revision, revision_before);
    assert!(invalid.export_records.is_empty());
}

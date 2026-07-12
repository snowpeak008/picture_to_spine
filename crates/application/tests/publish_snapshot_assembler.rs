use f2s_application::{
    animation::initialize_animation_set,
    export::{assemble_publish_snapshot, preflight::preflight},
    motion::initialize_motion_content,
};
use f2s_domain::{
    ACTION_KEYS, TimeBase,
    canonical::canonical_sha256,
    governance::Approval,
    import::SourceArtifact,
    layers::{Layer, LayerRole, LayerSet, PixelOrigin, PixelProvenance},
    master::{
        GripMode, MasterCandidate, PrimaryWeaponSpec, StyleSpec, WeaponHand, WeaponSizeClass,
    },
    motion::{assets::AssetState, content::KeyPoseBinding},
    project::{ProjectIdentity, ProjectManifest},
    rig::{
        RigApprovalState, RigCanvas, build_default_side_view_humanoid_rig,
        constraints::ConstraintCapability, layer_set_approval_payload_sha256,
        rig_approval_payload_sha256, weights::BoneWeight,
    },
};

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

fn primary_weapon() -> PrimaryWeaponSpec {
    PrimaryWeaponSpec {
        weapon_type: "single-sword".into(),
        grip_mode: GripMode::OneHand,
        weapon_hand: WeaponHand::NearHand,
        socket_semantic: "primary-grip".into(),
        size_class: WeaponSizeClass::Medium,
        silhouette_constraints: "one readable sword beside the character".into(),
    }
}

fn style() -> StyleSpec {
    StyleSpec {
        revision: 1,
        viewpoint: "side-view".into(),
        rendering_style: "anime-clean".into(),
        outline: "dark-clean".into(),
        palette_notes: "blue and silver".into(),
        identity_notes: "stable original character".into(),
        primary_weapon: Some(primary_weapon()),
    }
}

fn source() -> SourceArtifact {
    SourceArtifact {
        artifact_id: "source-main".into(),
        sha256: "a".repeat(64),
        media_type: "image/png".into(),
        width: 512,
        height: 512,
        byte_length: 4_096,
        bit_depth: 8,
        provenance: "user-local".into(),
        approval_state: "UNAPPROVED".into(),
    }
}

fn pending_layers() -> LayerSet {
    LayerSet {
        layer_set_id: "layers-main".into(),
        master_id: "master-main".into(),
        revision: 1,
        layers: LayerRole::REQUIRED_V1
            .iter()
            .enumerate()
            .map(|(index, role)| Layer {
                layer_id: format!("layer-{index}"),
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

fn provenance(layers: &LayerSet) -> Vec<PixelProvenance> {
    layers
        .layers
        .iter()
        .map(|layer| PixelProvenance {
            artifact_sha256: layer.attachment_sha256.clone(),
            origin: PixelOrigin::Source,
            source_sha256: "a".repeat(64),
            prompt_pack_id: None,
            receipt_ref: None,
            accepted_by: None,
            acceptance_attestation_sha256: None,
        })
        .collect()
}

fn approval(
    sequence: usize,
    gate_id: &str,
    target_id: &str,
    target_revision: u64,
    target_sha256: &str,
) -> Approval {
    Approval {
        approval_id: format!("approval-{sequence:03}"),
        gate_id: gate_id.into(),
        target_id: target_id.into(),
        target_revision,
        target_sha256: target_sha256.into(),
        actor_id: "human-reviewer".into(),
        approved_at_utc: format!("2026-07-11T00:{sequence:02}:00Z"),
        invalidated: false,
    }
}

fn fully_approved_project() -> ProjectManifest {
    let mut project =
        ProjectManifest::new(ProjectIdentity::new("publish assembler fixture").unwrap());
    let source = source();
    project.add_source(source.clone());

    let master = MasterCandidate {
        master_id: "master-main".into(),
        source_artifact_id: source.artifact_id,
        candidate_revision: 1,
        source_sha256: source.sha256,
        style_spec: style(),
        approval_state: "APPROVED".into(),
        supersedes: None,
    };
    project.set_master(master.clone());
    project
        .record_master_approval(approval(
            1,
            "master",
            &master.master_id,
            master.candidate_revision,
            &master.approval_payload_sha256().unwrap(),
        ))
        .unwrap();

    let layers = pending_layers();
    project
        .set_layer_set(layers.clone(), provenance(&layers))
        .unwrap();
    let layer_payload = layer_set_approval_payload_sha256(
        project.active_layer_set.as_ref().expect("active layers"),
    )
    .unwrap();
    let active_layers = project.active_layer_set.as_mut().unwrap();
    active_layers.approval_state = "APPROVED".into();
    for layer in &mut active_layers.layers {
        layer.approved = true;
    }
    let layer_approval = approval(
        2,
        "layers",
        &active_layers.layer_set_id,
        active_layers.revision,
        &layer_payload,
    );
    project.record_layer_approval(layer_approval).unwrap();

    let layers = project.active_layer_set.as_ref().unwrap().clone();
    let rig = build_default_side_view_humanoid_rig(
        "rig-main",
        &layers,
        layer_payload,
        primary_weapon(),
        RigCanvas {
            width_px: 512,
            height_px: 512,
        },
        capability(),
    )
    .unwrap();
    project.set_rig(rig).unwrap();
    let active_rig = project.active_rig.as_mut().unwrap();
    let rig_payload = rig_approval_payload_sha256(active_rig).unwrap();
    active_rig.approval_state = RigApprovalState::Approved;
    let rig_approval = approval(
        3,
        "rig",
        &active_rig.rig_id,
        active_rig.revision,
        &rig_payload,
    );
    project.record_rig_approval(rig_approval).unwrap();

    let motion = initialize_motion_content(&style()).unwrap();
    project.set_motion_content(motion.clone()).unwrap();
    let mut motion = motion;
    let mut sequence = 4;
    for index in 0..motion.assets.len() {
        let asset = motion.assets[index].clone();
        let binding = KeyPoseBinding {
            binding_id: format!("binding-{}", asset.asset_spec_id),
            revision: 1,
            asset_spec_id: asset.asset_spec_id.clone(),
            action_key: asset.action_key,
            pose_key: asset.pose_key,
            source_sha256: format!("{:064x}", index + 500),
            media_type: "image/png".into(),
            width: 512,
            height: 512,
            prompt_pack_id: motion.prompt_pack.pack_id.clone(),
            ground_y_milli_px: 0,
            scale_ppm: 1_000_000,
        };
        motion.key_pose_bindings.push(binding);
        motion.assets[index].state = AssetState::Approved;
    }
    for binding in &motion.key_pose_bindings {
        let payload = motion.binding_approval_payload(binding).unwrap();
        project.approval_log.push(approval(
            sequence,
            "key-pose-asset",
            &binding.binding_id,
            binding.revision,
            &payload,
        ));
        sequence += 1;
    }
    project.set_motion_content(motion.clone()).unwrap();
    let animation =
        initialize_animation_set(&motion, project.active_rig.as_ref().unwrap(), &rig_payload)
            .unwrap();
    project.set_animation_set(animation).unwrap();

    for action_key in ACTION_KEYS {
        let animation = project.animation_set.as_ref().unwrap();
        let motion = project.motion_content.as_ref().unwrap();
        let clip_revision = animation.clip(action_key).unwrap().revision;
        let pose_payload = animation.pose_payload(motion, action_key).unwrap();
        project
            .record_pose_approval(approval(
                sequence,
                "poses",
                action_key,
                clip_revision,
                &pose_payload,
            ))
            .unwrap();
        sequence += 1;
        if action_key.starts_with("attack_") {
            let animation = project.animation_set.as_ref().unwrap();
            let hit_payload = animation.hit_payload(action_key).unwrap();
            project
                .record_hit_approval(approval(
                    sequence,
                    "hits",
                    action_key,
                    clip_revision,
                    &hit_payload,
                ))
                .unwrap();
            sequence += 1;
        }
    }
    assert!(project.approval_closure_complete());
    project.validate_cross_aggregate().unwrap();
    project
}

fn rebind_current_rig_and_action_approvals(project: &mut ProjectManifest) {
    let rig_payload = rig_approval_payload_sha256(project.active_rig.as_ref().unwrap()).unwrap();
    project
        .approval_log
        .iter_mut()
        .rev()
        .find(|entry| entry.gate_id == "rig" && !entry.invalidated)
        .unwrap()
        .target_sha256 = rig_payload.clone();
    project.animation_set.as_mut().unwrap().approved_rig_sha256 = rig_payload;

    for action_key in ACTION_KEYS {
        let animation = project.animation_set.as_ref().unwrap();
        let motion = project.motion_content.as_ref().unwrap();
        let pose_payload = animation.pose_payload(motion, action_key).unwrap();
        project
            .approval_log
            .iter_mut()
            .rev()
            .find(|entry| {
                entry.gate_id == "poses" && entry.target_id == action_key && !entry.invalidated
            })
            .unwrap()
            .target_sha256 = pose_payload;
        if action_key.starts_with("attack_") {
            let hit_payload = project
                .animation_set
                .as_ref()
                .unwrap()
                .hit_payload(action_key)
                .unwrap();
            project
                .approval_log
                .iter_mut()
                .rev()
                .find(|entry| {
                    entry.gate_id == "hits" && entry.target_id == action_key && !entry.invalidated
                })
                .unwrap()
                .target_sha256 = hit_payload;
        }
    }
}

#[test]
fn assembler_recomputes_complete_current_approval_closure_and_export_view() {
    let project = fully_approved_project();
    let snapshot = assemble_publish_snapshot(&project, "publish-001").unwrap();

    assert_eq!(snapshot.project_revision, project.revision);
    assert_eq!(snapshot.spine_patch, "4.2.43");
    assert_eq!(snapshot.primary_weapon, "single-sword");
    assert_eq!(snapshot.clips.len(), 10);
    assert_eq!(snapshot.attachments.len(), LayerRole::REQUIRED_V1.len());
    assert_eq!(
        snapshot
            .action_approvals
            .iter()
            .map(|binding| binding.action_key.as_str())
            .collect::<Vec<_>>(),
        ACTION_KEYS
    );
    assert_eq!(
        snapshot
            .action_approvals
            .iter()
            .filter(|binding| binding.hit_approval_sha256.is_some())
            .count(),
        3
    );
    for binding in &snapshot.action_approvals {
        let pose = project.current_pose_approval(&binding.action_key).unwrap();
        assert_eq!(
            binding.pose_approval_sha256,
            canonical_sha256(pose).unwrap()
        );
        if binding.action_key.starts_with("attack_") {
            let hit = project.current_hit_approval(&binding.action_key).unwrap();
            assert_eq!(
                binding.hit_approval_sha256.as_deref(),
                Some(canonical_sha256(hit).unwrap().as_str())
            );
        }
    }
    let report = preflight(&snapshot);
    assert!(report.passed, "{:?}", report.errors);
}

#[test]
fn assembler_rejects_missing_or_malformed_current_human_approval() {
    let mut missing = fully_approved_project();
    missing
        .approval_log
        .iter_mut()
        .rev()
        .find(|entry| entry.gate_id == "poses" && entry.target_id == "idle")
        .unwrap()
        .invalidated = true;
    let error = assemble_publish_snapshot(&missing, "publish-002").unwrap_err();
    assert!(error.starts_with("POSE_APPROVAL_MISSING:"), "{error}");

    let mut malformed = fully_approved_project();
    let mut current = malformed.current_pose_approval("idle").unwrap().clone();
    current.approval_id = "latest-but-not-auditable".into();
    current.actor_id.clear();
    malformed.approval_log.push(current);
    let error = assemble_publish_snapshot(&malformed, "publish-003").unwrap_err();
    assert!(error.starts_with("APPROVAL_RECORD_INVALID:"), "{error}");
}

#[test]
fn assembler_rejects_project_timebase_and_rig_binding_drift() {
    let mut timebase_drift = fully_approved_project();
    timebase_drift.time_base = TimeBase {
        numerator: 1,
        denominator: 60,
    };
    let error = assemble_publish_snapshot(&timebase_drift, "publish-004").unwrap_err();
    assert!(error.starts_with("TIMEBASE_MISMATCH:"), "{error}");

    let mut rig_drift = fully_approved_project();
    rig_drift
        .animation_set
        .as_mut()
        .unwrap()
        .approved_rig_sha256 = "f".repeat(64);
    let error = assemble_publish_snapshot(&rig_drift, "publish-005").unwrap_err();
    assert!(error.starts_with("RIG_APPROVAL_BINDING_STALE:"), "{error}");
}

#[test]
fn assembler_blocks_multi_bone_vertices_even_when_all_approvals_are_rebound() {
    let mut project = fully_approved_project();
    let (original_bone, second_bone) = {
        let rig = project.active_rig.as_ref().unwrap();
        let original_bone = rig.weights[0].by_vertex.values().next().unwrap()[0]
            .bone_id
            .clone();
        let second_bone = rig
            .bone_tree
            .bones
            .iter()
            .find(|bone| bone.bone_id != original_bone)
            .unwrap()
            .bone_id
            .clone();
        (original_bone, second_bone)
    };
    let influences = project.active_rig.as_mut().unwrap().weights[0]
        .by_vertex
        .values_mut()
        .next()
        .unwrap();
    *influences = vec![
        BoneWeight {
            bone_id: original_bone,
            weight_ppm: 500_000,
        },
        BoneWeight {
            bone_id: second_bone,
            weight_ppm: 500_000,
        },
    ];
    rebind_current_rig_and_action_approvals(&mut project);
    project.validate_cross_aggregate().unwrap();

    let error = assemble_publish_snapshot(&project, "publish-006").unwrap_err();
    assert!(
        error.starts_with("MULTI_BONE_WEIGHTS_UNSUPPORTED:"),
        "{error}"
    );
}

use f2s_domain::{
    canonical::canonical_sha256,
    governance::{Approval, ReviewOutcome, ReviewRecord},
    import::SourceArtifact,
    layers::{Layer, LayerRole, LayerSet, PixelOrigin, PixelProvenance},
    master::{
        GripMode, MasterCandidate, PrimaryWeaponSpec, StyleSpec, WeaponHand, WeaponSizeClass,
    },
    project::{ProjectIdentity, ProjectManifest},
};

fn source(id: &str, hash_byte: char) -> SourceArtifact {
    SourceArtifact {
        artifact_id: id.into(),
        sha256: hash_byte.to_string().repeat(64),
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
        revision: 0,
        viewpoint: "side-view".into(),
        rendering_style: "anime-clean".into(),
        outline: "dark".into(),
        palette_notes: "blue and silver".into(),
        identity_notes: "stable original character identity".into(),
        primary_weapon: Some(PrimaryWeaponSpec {
            weapon_type: "single-sword".into(),
            grip_mode: GripMode::OneHand,
            weapon_hand: WeaponHand::FarHand,
            socket_semantic: "weapon-grip".into(),
            size_class: WeaponSizeClass::Medium,
            silhouette_constraints: "weapon remains readable beside the body".into(),
        }),
    }
}

fn master(source: &SourceArtifact, id: &str, state: &str) -> MasterCandidate {
    MasterCandidate {
        master_id: id.into(),
        source_artifact_id: source.artifact_id.clone(),
        candidate_revision: 0,
        source_sha256: source.sha256.clone(),
        style_spec: style(),
        approval_state: state.into(),
        supersedes: None,
    }
}

fn approval(gate: &str, id: &str, revision: u64, hash: &str) -> Approval {
    Approval {
        approval_id: format!("approval-{gate}-{revision}"),
        gate_id: gate.into(),
        target_id: id.into(),
        target_revision: revision,
        target_sha256: hash.into(),
        actor_id: "artist-1".into(),
        approved_at_utc: "2026-07-11T00:00:00Z".into(),
        invalidated: false,
    }
}

fn rejection(master: &MasterCandidate, hash: &str) -> ReviewRecord {
    ReviewRecord {
        review_id: "review-master-0".into(),
        gate_id: "master".into(),
        target_id: master.master_id.clone(),
        target_revision: master.candidate_revision,
        target_sha256: hash.into(),
        actor_id: "artist-1".into(),
        outcome: ReviewOutcome::Rejected,
        reason: "silhouette requires correction".into(),
        reviewed_at_utc: "2026-07-11T00:00:00Z".into(),
    }
}

fn pending_layer_set(master_id: &str, revision: u64) -> LayerSet {
    let layers = LayerRole::REQUIRED_V1
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
        .collect();
    LayerSet {
        layer_set_id: "layers-main".into(),
        master_id: master_id.into(),
        revision,
        layers,
        approval_state: "PENDING".into(),
    }
}

fn provenance(set: &LayerSet, source_hash: &str) -> Vec<PixelProvenance> {
    set.layers
        .iter()
        .map(|layer| PixelProvenance {
            artifact_sha256: layer.attachment_sha256.clone(),
            origin: PixelOrigin::Source,
            source_sha256: source_hash.into(),
            prompt_pack_id: None,
            receipt_ref: None,
            accepted_by: None,
            acceptance_attestation_sha256: None,
        })
        .collect()
}

fn approve_layer_set(set: &mut LayerSet) -> Approval {
    let mut normalized = set.clone();
    normalized.approval_state = "PENDING".into();
    for layer in &mut normalized.layers {
        layer.approved = false;
    }
    let payload = canonical_sha256(&normalized).expect("layer approval payload");
    set.approval_state = "APPROVED".into();
    for layer in &mut set.layers {
        layer.approved = true;
    }
    approval("layers", &set.layer_set_id, set.revision, &payload)
}

fn manifest() -> ProjectManifest {
    ProjectManifest::new(ProjectIdentity::new("manifest workflow test").unwrap())
}

fn approved_through_layers() -> ProjectManifest {
    let mut manifest = manifest();
    let source = source("source-main", 'a');
    manifest.add_source(source.clone());

    let candidate = master(&source, "master-main", "APPROVED");
    manifest.set_master(candidate.clone());
    manifest
        .record_master_approval(approval(
            "master",
            &candidate.master_id,
            candidate.candidate_revision,
            &candidate.approval_payload_sha256().unwrap(),
        ))
        .unwrap();

    let layers = pending_layer_set(&candidate.master_id, 0);
    manifest
        .set_layer_set(layers.clone(), provenance(&layers, &source.sha256))
        .unwrap();
    let layer_approval = approve_layer_set(manifest.active_layer_set.as_mut().unwrap());
    manifest.record_layer_approval(layer_approval).unwrap();
    manifest
}

#[test]
fn approved_source_master_and_layers_advance_exactly_one_revision_per_transition() {
    let mut manifest = manifest();
    assert_eq!(manifest.schema_version, "1.4.0");
    assert_eq!(manifest.revision, 0);
    assert_eq!(manifest.workflow_stage, "draft");

    let source = source("source-main", 'a');
    manifest.add_source(source.clone());
    assert_eq!(
        (manifest.revision, manifest.workflow_stage.as_str()),
        (1, "master")
    );

    let candidate = master(&source, "master-main", "APPROVED");
    manifest.set_master(candidate.clone());
    assert_eq!(
        (manifest.revision, manifest.workflow_stage.as_str()),
        (2, "master")
    );

    let master_approval = approval(
        "master",
        &candidate.master_id,
        candidate.candidate_revision,
        &candidate.approval_payload_sha256().unwrap(),
    );
    manifest.record_master_approval(master_approval).unwrap();
    assert_eq!(
        (manifest.revision, manifest.workflow_stage.as_str()),
        (3, "layers")
    );

    let layers = pending_layer_set(&candidate.master_id, 0);
    manifest
        .set_layer_set(layers.clone(), provenance(&layers, &source.sha256))
        .unwrap();
    assert_eq!(
        (manifest.revision, manifest.workflow_stage.as_str()),
        (4, "layers")
    );

    let layer_approval = approve_layer_set(manifest.active_layer_set.as_mut().unwrap());
    manifest.record_layer_approval(layer_approval).unwrap();
    assert_eq!(
        (manifest.revision, manifest.workflow_stage.as_str()),
        (5, "rig")
    );
    assert_eq!(manifest.approval_log.len(), 2);
    assert!(manifest.approval_log.iter().all(|entry| !entry.invalidated));
}

#[test]
fn master_approval_rejects_wrong_gate_hash_revision_and_unapproved_candidate_atomically() {
    let source = source("source-main", 'a');
    let cases = [
        (
            "layers",
            "master-main",
            0,
            source.sha256.clone(),
            "APPROVED",
        ),
        ("master", "master-main", 0, "b".repeat(64), "APPROVED"),
        (
            "master",
            "master-main",
            1,
            source.sha256.clone(),
            "APPROVED",
        ),
        ("master", "master-main", 0, source.sha256.clone(), "PENDING"),
    ];

    for (gate, id, target_revision, hash, master_state) in cases {
        let mut manifest = manifest();
        manifest.add_source(source.clone());
        manifest.set_master(master(&source, "master-main", master_state));
        let before_revision = manifest.revision;
        let before_stage = manifest.workflow_stage.clone();

        let result = manifest.record_master_approval(approval(gate, id, target_revision, &hash));
        assert!(
            result.is_err(),
            "invalid case unexpectedly accepted: {gate}"
        );
        assert_eq!(manifest.revision, before_revision);
        assert_eq!(manifest.workflow_stage, before_stage);
        assert!(manifest.approval_log.is_empty());
    }
}

#[test]
fn master_rejection_requires_rejected_state_and_exact_review_binding_without_partial_mutation() {
    let source = source("source-main", 'a');
    let mut manifest = manifest();
    manifest.add_source(source.clone());
    manifest.set_master(master(&source, "master-main", "PENDING"));
    let pending = manifest.active_master.as_ref().unwrap().clone();

    let before_revision = manifest.revision;
    assert!(
        manifest
            .record_master_rejection(rejection(&pending, &pending.source_sha256))
            .is_err()
    );
    assert_eq!(manifest.revision, before_revision);
    assert!(manifest.review_log.is_empty());

    manifest.active_master.as_mut().unwrap().approval_state = "REJECTED".into();
    let rejected = manifest.active_master.as_ref().unwrap().clone();
    assert!(
        manifest
            .record_master_rejection(rejection(&rejected, &"b".repeat(64)))
            .is_err()
    );
    assert_eq!(manifest.revision, before_revision);
    assert!(manifest.review_log.is_empty());

    manifest
        .record_master_rejection(rejection(&rejected, &rejected.source_sha256))
        .unwrap();
    assert_eq!(manifest.revision, before_revision + 1);
    assert_eq!(manifest.workflow_stage, "master");
    assert_eq!(manifest.review_log.len(), 1);
}

#[test]
fn layer_set_requires_approval_bound_to_the_active_master() {
    let source = source("source-main", 'a');
    let mut manifest = manifest();
    manifest.add_source(source.clone());
    let candidate = master(&source, "master-main", "APPROVED");
    manifest.set_master(candidate.clone());

    manifest.approval_log.push(approval(
        "master",
        "different-master",
        candidate.candidate_revision,
        &candidate.source_sha256,
    ));
    let layers = pending_layer_set(&candidate.master_id, 0);
    let before_revision = manifest.revision;
    assert!(
        manifest
            .set_layer_set(layers.clone(), provenance(&layers, &source.sha256))
            .is_err()
    );
    assert_eq!(manifest.revision, before_revision);
    assert!(manifest.active_layer_set.is_none());
}

#[test]
fn layer_approval_is_bound_to_normalized_payload_and_current_revision() {
    let mut manifest = approved_through_layers();
    let first_layer_approval = manifest
        .approval_log
        .iter()
        .find(|entry| entry.gate_id == "layers")
        .unwrap()
        .clone();

    let master_id = manifest.active_master.as_ref().unwrap().master_id.clone();
    let source_hash = manifest
        .active_master
        .as_ref()
        .unwrap()
        .source_sha256
        .clone();
    let revised = pending_layer_set(&master_id, 1);
    manifest
        .set_layer_set(revised.clone(), provenance(&revised, &source_hash))
        .unwrap();
    assert!(
        manifest
            .approval_log
            .iter()
            .find(|entry| entry.approval_id == first_layer_approval.approval_id)
            .unwrap()
            .invalidated
    );
    assert!(
        !manifest.approval_log[0].invalidated,
        "master gate remains valid"
    );

    let before_revision = manifest.revision;
    assert!(
        manifest
            .record_layer_approval(first_layer_approval)
            .is_err()
    );
    assert_eq!(manifest.revision, before_revision);
    assert_eq!(manifest.workflow_stage, "layers");

    let payload = {
        let set = manifest.active_layer_set.as_ref().unwrap();
        canonical_sha256(set).unwrap()
    };
    let active = manifest.active_layer_set.as_ref().unwrap();
    let forged = approval("layers", &active.layer_set_id, active.revision, &payload);
    assert!(manifest.record_layer_approval(forged).is_err());
    assert_eq!(manifest.revision, before_revision);
}

#[test]
fn adding_replacement_source_clears_derived_state_and_invalidates_all_downstream_gates() {
    let mut manifest = approved_through_layers();
    let before_revision = manifest.revision;
    assert_eq!(manifest.workflow_stage, "rig");

    manifest.add_source(source("source-main", 'b'));

    assert_eq!(manifest.revision, before_revision + 1);
    assert_eq!(manifest.workflow_stage, "master");
    assert_eq!(manifest.source_artifacts.len(), 1);
    assert_eq!(manifest.source_artifacts[0].sha256, "b".repeat(64));
    assert!(manifest.active_master.is_none());
    assert!(manifest.active_layer_set.is_none());
    assert!(manifest.layer_provenance.is_empty());
    assert!(manifest.approval_log.iter().all(|entry| entry.invalidated));
}

#[test]
fn exact_but_invalidated_gate_approvals_are_never_current_or_replayable() {
    let mut manifest = approved_through_layers();
    let layer_approval_index = manifest
        .approval_log
        .iter()
        .position(|approval| approval.gate_id == "layers")
        .expect("layer approval");
    let mut replay = manifest.approval_log[layer_approval_index].clone();
    manifest.approval_log[layer_approval_index].invalidated = true;
    replay.invalidated = true;

    assert!(manifest.current_layer_approval().is_none());
    assert!(manifest.validate_cross_aggregate().is_err());
    let revision_before_replay = manifest.revision;
    let log_len_before_replay = manifest.approval_log.len();
    assert!(manifest.record_layer_approval(replay).is_err());
    assert_eq!(manifest.revision, revision_before_replay);
    assert_eq!(manifest.approval_log.len(), log_len_before_replay);

    let master_approval = manifest
        .approval_log
        .iter_mut()
        .find(|approval| approval.gate_id == "master")
        .expect("master approval");
    master_approval.invalidated = true;
    assert!(manifest.current_master_approval().is_none());
    assert!(manifest.validate_cross_aggregate().is_err());
}

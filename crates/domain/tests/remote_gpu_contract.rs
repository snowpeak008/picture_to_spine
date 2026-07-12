use f2s_domain::remote_gpu::{
    EndpointOwnership, RemoteApprovalBinding, RemoteDeletionReceipt, RemoteEvidenceScope,
    RemoteGpuMethod, RemoteGpuProfile, RemoteInputPurpose, RemoteJob, RemoteJobState,
    RemoteMediaType, RemoteModelBinding, RemoteOutputDescriptor, RemoteOutputPurpose,
    RemoteRequestReceipt, RemoteResponseReceipt, RemoteRetentionPolicy, RemoteTransferItem,
    RemoteTransferPlan,
};

fn profile() -> RemoteGpuProfile {
    RemoteGpuProfile {
        schema_version: "1.0.0".into(),
        enabled: true,
        profile_id: "studio-gpu-01".into(),
        ownership: EndpointOwnership::UserControlledPrivate,
        origin: "https://gpu.internal.example".into(),
        allowed_ports: vec![443, 8443],
        certificate_spki_sha256: "b".repeat(64),
        organization_identity_sha256: "c".repeat(64),
        credential_manager_target: "FlashToSpine/RemoteGpu/studio-gpu-01".into(),
        allowed_methods: vec![
            RemoteGpuMethod::LayerSegmentationCandidate,
            RemoteGpuMethod::RigProposalCandidate,
            RemoteGpuMethod::MotionCurveCandidate,
        ],
        allowed_input_media_types: vec![
            RemoteMediaType::ImagePng,
            RemoteMediaType::ImageJpeg,
            RemoteMediaType::ImageWebp,
            RemoteMediaType::ApplicationJson,
            RemoteMediaType::ApplicationRigIrJson,
        ],
        allowed_model_manifest_sha256: vec!["d".repeat(64)],
        max_upload_bytes: 4096,
        max_response_bytes: 4096,
        request_timeout_seconds: 120,
    }
}

fn layer_plan(revision: u64, byte_length: u64, retention: u32) -> RemoteTransferPlan {
    RemoteTransferPlan::build(
        "operation-001",
        "project-001",
        revision,
        &profile(),
        RemoteGpuMethod::LayerSegmentationCandidate,
        "e".repeat(64),
        RemoteModelBinding {
            model_id: "layer-helper".into(),
            exact_version: "1.2.3".into(),
            manifest_sha256: "d".repeat(64),
        },
        vec![RemoteTransferItem {
            artifact_id: "approved-master".into(),
            sha256: "a".repeat(64),
            byte_length,
            media_type: RemoteMediaType::ImagePng,
            purpose: RemoteInputPurpose::ApprovedMasterImage,
        }],
        RemoteRetentionPolicy {
            delete_after_seconds: retention,
            require_deletion_receipt: true,
        },
    )
    .unwrap()
}

fn request(plan: &RemoteTransferPlan) -> RemoteRequestReceipt {
    let profile = profile();
    RemoteRequestReceipt {
        api_version: "f2s-rgpu/v1".into(),
        provider_job_id: "01JEXTERNAL001".into(),
        idempotency_key: plan.idempotency_key.clone(),
        request_sha256: plan.plan_sha256.clone(),
        accepted_manifest_sha256: plan.plan_sha256.clone(),
        server_capability_sha256: "f".repeat(64),
        retention_deadline_utc: "2030-01-01T00:00:00Z".into(),
        event_sequence_start: 0,
        observed_origin: profile.origin,
        observed_spki_sha256: profile.certificate_spki_sha256,
        observed_organization_identity_sha256: profile.organization_identity_sha256,
        evidence_scope: RemoteEvidenceScope::DeterministicContractMock,
    }
}

fn approval(plan: &RemoteTransferPlan) -> RemoteApprovalBinding {
    RemoteApprovalBinding {
        actor_id: "artist-01".into(),
        attestation_id: "attestation-01".into(),
        verification_proof_sha256: "9".repeat(64),
        approved_plan_sha256: plan.plan_sha256.clone(),
    }
}

fn response(
    plan: &RemoteTransferPlan,
    request: &RemoteRequestReceipt,
    sequence: u64,
) -> RemoteResponseReceipt {
    let bytes = br#"{"candidateOnly":true}"#;
    RemoteResponseReceipt::build(
        request,
        sequence,
        vec![RemoteOutputDescriptor {
            artifact_id: "layer-candidate-manifest".into(),
            sha256: f2s_domain::canonical::sha256_bytes(bytes),
            byte_length: bytes.len() as u64,
            media_type: RemoteMediaType::ApplicationJson,
            purpose: RemoteOutputPurpose::LayerCandidateManifest,
        }],
    )
    .map(|receipt| {
        receipt.validate_for(plan, request, &profile()).unwrap();
        receipt
    })
    .unwrap()
}

#[test]
fn profile_rejects_public_default_and_non_origin_endpoints() {
    let valid = profile();
    valid.validate_configuration().unwrap();

    let mut bad = valid.clone();
    bad.profile_id = "default".into();
    bad.credential_manager_target = "FlashToSpine/RemoteGpu/default".into();
    assert!(bad.validate_configuration().is_err());

    let mut bad = valid.clone();
    bad.origin = "https://api.openai.com".into();
    assert!(bad.validate_configuration().is_err());

    let mut bad = valid.clone();
    bad.origin = "https://gpu.internal.example/v1".into();
    assert!(bad.validate_configuration().is_err());

    let mut bad = valid.clone();
    bad.origin = "https://gpu.internal.example:9443".into();
    assert!(bad.validate_configuration().is_err());

    let mut bad = valid.clone();
    bad.credential_manager_target = "token=plaintext".into();
    assert!(bad.validate_configuration().is_err());

    let mut disabled = valid;
    disabled.enabled = false;
    disabled.validate_configuration().unwrap();
    assert!(disabled.require_enabled().is_err());
}

#[test]
fn canonical_plan_hash_binds_revision_items_method_and_retention() {
    let base = layer_plan(7, 128, 3600);
    assert_ne!(base.plan_sha256, layer_plan(8, 128, 3600).plan_sha256);
    assert_ne!(base.plan_sha256, layer_plan(7, 129, 3600).plan_sha256);
    assert_ne!(base.plan_sha256, layer_plan(7, 128, 7200).plan_sha256);

    let motion = RemoteTransferPlan::build(
        "operation-001",
        "project-001",
        7,
        &profile(),
        RemoteGpuMethod::MotionCurveCandidate,
        "e".repeat(64),
        RemoteModelBinding {
            model_id: "layer-helper".into(),
            exact_version: "1.2.3".into(),
            manifest_sha256: "d".repeat(64),
        },
        vec![RemoteTransferItem {
            artifact_id: "approved-rig-ir".into(),
            sha256: "a".repeat(64),
            byte_length: 128,
            media_type: RemoteMediaType::ApplicationRigIrJson,
            purpose: RemoteInputPurpose::ApprovedRigIr,
        }],
        RemoteRetentionPolicy {
            delete_after_seconds: 3600,
            require_deletion_receipt: true,
        },
    )
    .unwrap();
    assert_ne!(base.plan_sha256, motion.plan_sha256);

    let mut tampered = base.clone();
    tampered.items[0].purpose = RemoteInputPurpose::SelectionMaskImage;
    assert!(tampered.validate_against_profile(&profile()).is_err());
}

#[test]
fn unknown_submission_is_terminal_and_never_reposted() {
    let plan = layer_plan(7, 128, 3600);
    let mut job = RemoteJob::new("remote-job-001", plan.clone(), approval(&plan)).unwrap();
    job.begin_submission().unwrap();
    job.mark_submission_failed(true).unwrap();
    assert_eq!(job.state, RemoteJobState::Interrupted);
    assert!(job.begin_submission().is_err());
    assert!(!job.candidate_contract_validated());
    job.validate_persisted().unwrap();
}

#[test]
fn receipts_are_idempotent_candidate_only_and_deletion_stays_external() {
    let plan = layer_plan(7, 128, 3600);
    let mut job = RemoteJob::new("remote-job-001", plan.clone(), approval(&plan)).unwrap();
    job.begin_submission().unwrap();
    let accepted = request(&plan);
    assert!(
        job.record_request_receipt(accepted.clone(), &profile())
            .unwrap()
    );
    assert!(
        !job.record_request_receipt(accepted.clone(), &profile())
            .unwrap()
    );
    job.observe_running(1).unwrap();
    let success = response(&plan, &accepted, 2);
    job.record_success(success.clone(), &profile()).unwrap();
    assert!(job.candidate_contract_validated());
    assert!(!job.candidate_eligible_for_project_registration());

    let deletion = RemoteDeletionReceipt::build(
        &accepted,
        success
            .outputs
            .iter()
            .map(|output| output.sha256.clone())
            .collect(),
        "2030-01-01T00:00:00Z",
        "2030-01-01T00:00:01Z",
        "8".repeat(64),
    )
    .unwrap();
    assert!(!deletion.proves_local_deletion());
    job.record_deletion_receipt(deletion).unwrap();
    assert!(!job.has_external_deletion_receipt());
    job.validate_persisted().unwrap();
}

#[test]
fn late_success_after_cancel_remains_quarantined() {
    let plan = layer_plan(7, 128, 3600);
    let mut job = RemoteJob::new("remote-job-001", plan.clone(), approval(&plan)).unwrap();
    job.begin_submission().unwrap();
    let accepted = request(&plan);
    job.record_request_receipt(accepted.clone(), &profile())
        .unwrap();
    job.request_cancel().unwrap();
    job.record_success(response(&plan, &accepted, 1), &profile())
        .unwrap();
    assert_eq!(job.state, RemoteJobState::CancelRequested);
    assert!(job.late_success_quarantined);
    assert!(!job.candidate_contract_validated());
    job.record_cancelled(2).unwrap();
    assert_eq!(job.state, RemoteJobState::Cancelled);
}

#[test]
fn persisted_transition_chain_detects_tampering() {
    let plan = layer_plan(7, 128, 3600);
    let mut job = RemoteJob::new("remote-job-001", plan.clone(), approval(&plan)).unwrap();
    job.begin_submission().unwrap();
    job.transitions[1].reason_code = "TAMPERED".into();
    assert!(job.validate_persisted().is_err());
}

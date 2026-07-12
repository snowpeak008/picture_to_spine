use f2s_adapters::safety::private_remote::{
    DeterministicPrivateRemoteMock, ExternalPrivateRemoteNotRun, FsRemoteCandidateQuarantine,
    FsRemoteGpuProfileStore, FsRemoteJobStore,
};
use f2s_application::{
    approvals::{HumanCredentialVerifier, VerifiedHumanActor},
    ports::{RemoteGpuProfileStore, RemoteJobStore},
    remote_gpu::RemoteGpuService,
};
use f2s_domain::{
    governance::CredentialAttestation,
    remote_gpu::{
        EndpointOwnership, REMOTE_TRANSFER_APPROVAL_PURPOSE, RemoteGpuMethod, RemoteGpuProfile,
        RemoteInputPurpose, RemoteJobState, RemoteMediaType, RemoteModelBinding,
        RemoteRetentionPolicy, RemoteTransferItem, RemoteTransferPlan,
    },
};
use std::{fs, path::PathBuf};
use uuid::Uuid;

struct TestVerifier;
impl HumanCredentialVerifier for TestVerifier {
    fn verify_and_consume(&self, attestation: &CredentialAttestation) -> Result<(), String> {
        if attestation.credential_ref == "test-credential://artist-01" {
            Ok(())
        } else {
            Err("test credential rejected".into())
        }
    }
}

struct TestRoot(PathBuf);
impl TestRoot {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("f2s-rgpu-test-{}", Uuid::new_v4())))
    }
}
impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn profile() -> RemoteGpuProfile {
    RemoteGpuProfile {
        schema_version: "1.0.0".into(),
        enabled: true,
        profile_id: "studio-gpu-01".into(),
        ownership: EndpointOwnership::UserControlledPrivate,
        origin: "https://gpu.internal.example".into(),
        allowed_ports: vec![443],
        certificate_spki_sha256: "b".repeat(64),
        organization_identity_sha256: "c".repeat(64),
        credential_manager_target: "FlashToSpine/RemoteGpu/studio-gpu-01".into(),
        allowed_methods: vec![RemoteGpuMethod::LayerSegmentationCandidate],
        allowed_input_media_types: vec![RemoteMediaType::ImagePng],
        allowed_model_manifest_sha256: vec!["d".repeat(64)],
        max_upload_bytes: 4096,
        max_response_bytes: 4096,
        request_timeout_seconds: 60,
    }
}

fn plan(profile: &RemoteGpuProfile) -> RemoteTransferPlan {
    RemoteTransferPlan::build(
        "operation-001",
        "project-001",
        12,
        profile,
        RemoteGpuMethod::LayerSegmentationCandidate,
        "e".repeat(64),
        RemoteModelBinding {
            model_id: "layer-helper".into(),
            exact_version: "1.0.0".into(),
            manifest_sha256: "d".repeat(64),
        },
        vec![RemoteTransferItem {
            artifact_id: "approved-master".into(),
            sha256: "a".repeat(64),
            byte_length: 128,
            media_type: RemoteMediaType::ImagePng,
            purpose: RemoteInputPurpose::ApprovedMasterImage,
        }],
        RemoteRetentionPolicy {
            delete_after_seconds: 3600,
            require_deletion_receipt: true,
        },
    )
    .unwrap()
}

fn actor(purpose: &str, payload_sha256: &str) -> VerifiedHumanActor {
    VerifiedHumanActor::verify(
        CredentialAttestation {
            attestation_id: format!("attestation-{}", Uuid::new_v4()),
            actor_id: "artist-01".into(),
            actor_kind: "HUMAN".into(),
            credential_ref: "test-credential://artist-01".into(),
            purpose: purpose.into(),
            issued_at_utc: "2026-07-11T00:00:00Z".into(),
            expires_at_utc: "2026-07-11T00:05:00Z".into(),
            payload_sha256: payload_sha256.into(),
            verification_proof_sha256: "9".repeat(64),
        },
        purpose,
        payload_sha256,
        &TestVerifier,
    )
    .unwrap()
}

#[test]
fn deterministic_contract_persists_candidate_and_receipts_without_real_claims() {
    let root = TestRoot::new();
    let profiles = FsRemoteGpuProfileStore::new(&root.0);
    let jobs = FsRemoteJobStore::new(&root.0);
    let quarantine = FsRemoteCandidateQuarantine::new(&root.0);
    let transport = DeterministicPrivateRemoteMock::default();
    let profile = profile();
    profiles.save_profile(&profile).unwrap();
    let plan = plan(&profile);
    let service = RemoteGpuService::new(&profiles, &jobs, &quarantine, &transport);

    let created = service
        .approve_and_create_job(
            "remote-job-001",
            plan.clone(),
            12,
            actor(REMOTE_TRANSFER_APPROVAL_PURPOSE, &plan.plan_sha256),
        )
        .unwrap();
    assert_eq!(created.state, RemoteJobState::Approved);
    assert_eq!(
        service.submit("remote-job-001").unwrap().state,
        RemoteJobState::Submitted
    );
    transport.complete(&plan).unwrap();
    assert_eq!(
        service.poll("remote-job-001").unwrap().state,
        RemoteJobState::Running
    );
    let succeeded = service.poll("remote-job-001").unwrap();
    assert_eq!(succeeded.state, RemoteJobState::Succeeded);
    assert!(succeeded.candidate_contract_validated());
    assert!(!succeeded.candidate_eligible_for_project_registration());

    let deleted = service.delete_remote_artifacts("remote-job-001").unwrap();
    assert!(deleted.deletion_receipt.is_some());
    assert!(!deleted.has_external_deletion_receipt());
    assert!(
        !deleted
            .deletion_receipt
            .as_ref()
            .unwrap()
            .proves_local_deletion()
    );
    jobs.load_job("remote-job-001")
        .unwrap()
        .unwrap()
        .validate_persisted()
        .unwrap();

    let profile_text =
        fs::read_to_string(root.0.join("profiles").join("studio-gpu-01.json")).unwrap();
    assert!(!profile_text.to_ascii_lowercase().contains("\"token\""));
    assert!(!profile_text.to_ascii_lowercase().contains("\"password\""));
    assert!(
        root.0
            .join("quarantine")
            .join("remote-job-001")
            .join("layer-candidate-manifest.candidate")
            .exists()
    );
}

#[test]
fn human_binding_revision_and_not_run_transport_fail_closed() {
    let root = TestRoot::new();
    let profiles = FsRemoteGpuProfileStore::new(&root.0);
    let jobs = FsRemoteJobStore::new(&root.0);
    let quarantine = FsRemoteCandidateQuarantine::new(&root.0);
    let transport = ExternalPrivateRemoteNotRun;
    let profile = profile();
    profiles.save_profile(&profile).unwrap();
    let plan = plan(&profile);
    let service = RemoteGpuService::new(&profiles, &jobs, &quarantine, &transport);

    assert!(
        service
            .approve_and_create_job(
                "remote-job-stale",
                plan.clone(),
                13,
                actor(REMOTE_TRANSFER_APPROVAL_PURPOSE, &plan.plan_sha256),
            )
            .is_err()
    );
    assert!(
        service
            .approve_and_create_job(
                "remote-job-wrong-approval",
                plan.clone(),
                12,
                actor("another.purpose", &plan.plan_sha256),
            )
            .is_err()
    );

    service
        .approve_and_create_job(
            "remote-job-not-run",
            plan.clone(),
            12,
            actor(REMOTE_TRANSFER_APPROVAL_PURPOSE, &plan.plan_sha256),
        )
        .unwrap();
    assert!(service.submit("remote-job-not-run").is_err());
    let failed = jobs.load_job("remote-job-not-run").unwrap().unwrap();
    assert_eq!(failed.state, RemoteJobState::Failed);
    assert!(failed.request_receipt.is_none());
    assert!(!failed.candidate_contract_validated());
}

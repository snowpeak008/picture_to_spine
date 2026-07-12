use crate::storage::ntfs_atomic::write_atomic;
use f2s_application::ports::{
    DownloadedRemoteCandidate, ExternalCapabilityState, PrivateRemoteGpuTransport,
    RemoteCandidateQuarantine, RemoteCapabilityReport, RemoteGpuProfileStore, RemoteJobStore,
    RemoteStatusObservation, RemoteTransportError, RemoteTransportFailureDisposition,
};
use f2s_domain::{
    canonical::sha256_bytes,
    remote_gpu::{
        RemoteDeletionReceipt, RemoteEvidenceScope, RemoteGpuMethod, RemoteGpuProfile, RemoteJob,
        RemoteMediaType, RemoteOutputDescriptor, RemoteOutputPurpose, RemoteRequestReceipt,
        RemoteResponseReceipt, RemoteTransferPlan,
    },
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

/// Local-machine profile storage. The serializable profile type contains only a
/// Credential Manager target reference; it has no token/password/secret field.
pub struct FsRemoteGpuProfileStore {
    root: PathBuf,
    writer: Mutex<()>,
}

impl FsRemoteGpuProfileStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            writer: Mutex::new(()),
        }
    }

    fn path(&self, profile_id: &str) -> Result<PathBuf, String> {
        require_local_identifier(profile_id)?;
        Ok(self
            .root
            .join("profiles")
            .join(format!("{profile_id}.json")))
    }
}

impl RemoteGpuProfileStore for FsRemoteGpuProfileStore {
    fn save_profile(&self, profile: &RemoteGpuProfile) -> Result<(), String> {
        profile.validate_configuration()?;
        let bytes = serde_json::to_vec_pretty(profile).map_err(|error| error.to_string())?;
        let _guard = self
            .writer
            .lock()
            .map_err(|_| "remote profile store lock is poisoned".to_owned())?;
        write_atomic(&self.path(&profile.profile_id)?, &bytes)
    }

    fn load_profile(&self, profile_id: &str) -> Result<Option<RemoteGpuProfile>, String> {
        let path = self.path(profile_id)?;
        if !path.exists() {
            return Ok(None);
        }
        let profile: RemoteGpuProfile =
            serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
                .map_err(|error| error.to_string())?;
        profile.validate_configuration()?;
        if profile.profile_id != profile_id {
            return Err("remote profile file identity mismatch".into());
        }
        Ok(Some(profile))
    }
}

/// A recoverable, atomic local authority for the remote-job state machine.
/// compare-and-swap prevents two poll/cancel paths from overwriting transitions.
pub struct FsRemoteJobStore {
    root: PathBuf,
    writer: Mutex<()>,
}

impl FsRemoteJobStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            writer: Mutex::new(()),
        }
    }

    fn path(&self, job_id: &str) -> Result<PathBuf, String> {
        require_local_identifier(job_id)?;
        Ok(self.root.join("jobs").join(format!("{job_id}.json")))
    }

    fn read(&self, job_id: &str) -> Result<Option<RemoteJob>, String> {
        let path = self.path(job_id)?;
        if !path.exists() {
            return Ok(None);
        }
        let job: RemoteJob =
            serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
                .map_err(|error| error.to_string())?;
        job.validate_persisted()?;
        if job.job_id != job_id {
            return Err("remote job file identity mismatch".into());
        }
        Ok(Some(job))
    }
}

impl RemoteJobStore for FsRemoteJobStore {
    fn create_job(&self, job: &RemoteJob) -> Result<(), String> {
        job.validate_persisted()?;
        let _guard = self
            .writer
            .lock()
            .map_err(|_| "remote job store lock is poisoned".to_owned())?;
        let path = self.path(&job.job_id)?;
        if path.exists() {
            return Err("remote job already exists".into());
        }
        write_atomic(
            &path,
            &serde_json::to_vec_pretty(job).map_err(|error| error.to_string())?,
        )
    }

    fn load_job(&self, job_id: &str) -> Result<Option<RemoteJob>, String> {
        self.read(job_id)
    }

    fn compare_and_swap_job(
        &self,
        job: &RemoteJob,
        expected_persistence_sequence: u64,
    ) -> Result<(), String> {
        job.validate_persisted()?;
        let _guard = self
            .writer
            .lock()
            .map_err(|_| "remote job store lock is poisoned".to_owned())?;
        let current = self.read(&job.job_id)?.ok_or("remote job does not exist")?;
        if current.persistence_sequence() != expected_persistence_sequence {
            return Err("remote job compare-and-swap conflict".into());
        }
        if job.persistence_sequence() <= expected_persistence_sequence {
            return Err("remote job update did not append a transition".into());
        }
        write_atomic(
            &self.path(&job.job_id)?,
            &serde_json::to_vec_pretty(job).map_err(|error| error.to_string())?,
        )
    }
}

pub struct FsRemoteCandidateQuarantine {
    root: PathBuf,
    writer: Mutex<()>,
}

impl FsRemoteCandidateQuarantine {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            writer: Mutex::new(()),
        }
    }
}

impl RemoteCandidateQuarantine for FsRemoteCandidateQuarantine {
    fn put_quarantined(
        &self,
        job_id: &str,
        descriptor: &RemoteOutputDescriptor,
        bytes: &[u8],
    ) -> Result<(), String> {
        require_local_identifier(job_id)?;
        require_local_identifier(&descriptor.artifact_id)?;
        if bytes.len() as u64 != descriptor.byte_length || sha256_bytes(bytes) != descriptor.sha256
        {
            return Err("quarantine candidate bytes do not match their descriptor".into());
        }
        let _guard = self
            .writer
            .lock()
            .map_err(|_| "remote quarantine lock is poisoned".to_owned())?;
        let directory = self.root.join("quarantine").join(job_id);
        let binary = directory.join(format!("{}.candidate", descriptor.artifact_id));
        let manifest = directory.join(format!("{}.json", descriptor.artifact_id));
        if binary.exists() || manifest.exists() {
            if !binary.exists() {
                return Err("quarantine manifest exists without candidate bytes".into());
            }
            let existing = fs::read(&binary).map_err(|error| error.to_string())?;
            if existing == bytes {
                if !manifest.exists() {
                    write_atomic(
                        &manifest,
                        &serde_json::to_vec_pretty(descriptor)
                            .map_err(|error| error.to_string())?,
                    )?;
                }
                return Ok(());
            }
            return Err("quarantine artifact identity conflict".into());
        }
        write_atomic(&binary, bytes)?;
        write_atomic(
            &manifest,
            &serde_json::to_vec_pretty(descriptor).map_err(|error| error.to_string())?,
        )
    }
}

/// Production fail-closed placeholder. It performs no network or credential
/// access and reports the real endpoint capability as NOT_RUN/EXTERNAL.
#[derive(Default)]
pub struct ExternalPrivateRemoteNotRun;

impl PrivateRemoteGpuTransport for ExternalPrivateRemoteNotRun {
    fn capability_report(&self) -> RemoteCapabilityReport {
        RemoteCapabilityReport {
            state: ExternalCapabilityState::NotRunExternal,
            network_attempted: false,
            reason: "WinHTTP TLS/SPKI transport is not integrated; this transport never reads Credential Manager secrets and the real endpoint remains NOT_RUN/EXTERNAL".into(),
        }
    }

    fn submit(
        &self,
        _profile: &RemoteGpuProfile,
        _plan: &RemoteTransferPlan,
    ) -> Result<RemoteRequestReceipt, RemoteTransportError> {
        Err(not_run_error())
    }

    fn query(
        &self,
        _profile: &RemoteGpuProfile,
        _request: &RemoteRequestReceipt,
    ) -> Result<RemoteStatusObservation, RemoteTransportError> {
        Err(not_run_error())
    }

    fn cancel(
        &self,
        _profile: &RemoteGpuProfile,
        _request: &RemoteRequestReceipt,
    ) -> Result<RemoteStatusObservation, RemoteTransportError> {
        Err(not_run_error())
    }

    fn delete(
        &self,
        _profile: &RemoteGpuProfile,
        _request: &RemoteRequestReceipt,
        _response: Option<&RemoteResponseReceipt>,
    ) -> Result<RemoteDeletionReceipt, RemoteTransportError> {
        Err(not_run_error())
    }
}

fn not_run_error() -> RemoteTransportError {
    RemoteTransportError::new(
        "F2S-RGPU-REAL-ENDPOINT-NOT-RUN",
        RemoteTransportFailureDisposition::DefinitelyNotSent,
    )
}

#[derive(Debug, Clone)]
struct MockRemoteJob {
    plan_sha256: String,
    request: RemoteRequestReceipt,
    polls: u64,
    cancelled: bool,
    result: Option<(RemoteResponseReceipt, Vec<DownloadedRemoteCandidate>)>,
}

/// Deterministic protocol contract double. Its receipts are permanently marked
/// MOCK and therefore can never make a candidate eligible for project registration.
#[derive(Default)]
pub struct DeterministicPrivateRemoteMock {
    jobs: Mutex<BTreeMap<String, MockRemoteJob>>,
}

impl DeterministicPrivateRemoteMock {
    fn candidate(
        plan: &RemoteTransferPlan,
        request: &RemoteRequestReceipt,
        event_sequence: u64,
    ) -> Result<(RemoteResponseReceipt, Vec<DownloadedRemoteCandidate>), RemoteTransportError> {
        let (artifact_id, purpose) = match plan.method {
            RemoteGpuMethod::LayerSegmentationCandidate => (
                "layer-candidate-manifest",
                RemoteOutputPurpose::LayerCandidateManifest,
            ),
            RemoteGpuMethod::RigProposalCandidate => (
                "rig-candidate-manifest",
                RemoteOutputPurpose::RigCandidateManifest,
            ),
            RemoteGpuMethod::MotionCurveCandidate => (
                "animation-candidate-manifest",
                RemoteOutputPurpose::AnimationCandidateManifest,
            ),
        };
        let bytes = serde_json::to_vec(&json!({
            "schemaVersion": "f2s-rgpu-candidate/v1",
            "candidateOnly": true,
            "method": plan.method,
            "requestSha256": plan.plan_sha256,
            "source": "DETERMINISTIC_CONTRACT_MOCK"
        }))
        .map_err(|_| contract_error("F2S-RGPU-MOCK-SERIALIZE"))?;
        let descriptor = RemoteOutputDescriptor {
            artifact_id: artifact_id.into(),
            sha256: sha256_bytes(&bytes),
            byte_length: bytes.len() as u64,
            media_type: RemoteMediaType::ApplicationJson,
            purpose,
        };
        let receipt =
            RemoteResponseReceipt::build(request, event_sequence, vec![descriptor.clone()])
                .map_err(|_| contract_error("F2S-RGPU-MOCK-RESPONSE"))?;
        Ok((
            receipt,
            vec![DownloadedRemoteCandidate { descriptor, bytes }],
        ))
    }
}

impl PrivateRemoteGpuTransport for DeterministicPrivateRemoteMock {
    fn capability_report(&self) -> RemoteCapabilityReport {
        RemoteCapabilityReport {
            state: ExternalCapabilityState::ContractMockOnly,
            network_attempted: false,
            reason: "deterministic provider contract only; no real network, TLS, credential, GPU, or deletion proof".into(),
        }
    }

    fn submit(
        &self,
        profile: &RemoteGpuProfile,
        plan: &RemoteTransferPlan,
    ) -> Result<RemoteRequestReceipt, RemoteTransportError> {
        plan.validate_against_profile(profile)
            .map_err(|_| contract_error("F2S-RGPU-MOCK-PLAN"))?;
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| contract_error("F2S-RGPU-MOCK-LOCK"))?;
        if let Some(existing) = jobs.get(&plan.idempotency_key) {
            return if existing.plan_sha256 == plan.plan_sha256 {
                Ok(existing.request.clone())
            } else {
                Err(contract_error("F2S-RGPU-IDEMPOTENCY-CONFLICT"))
            };
        }
        let request = RemoteRequestReceipt {
            api_version: "f2s-rgpu/v1".into(),
            provider_job_id: format!("mock-{}", &plan.plan_sha256[..24]),
            idempotency_key: plan.idempotency_key.clone(),
            request_sha256: plan.plan_sha256.clone(),
            accepted_manifest_sha256: plan.plan_sha256.clone(),
            server_capability_sha256: sha256_bytes(b"f2s-deterministic-private-remote-mock-v1"),
            retention_deadline_utc: "2099-01-01T00:00:00Z".into(),
            event_sequence_start: 0,
            observed_origin: profile.origin.clone(),
            observed_spki_sha256: profile.certificate_spki_sha256.clone(),
            observed_organization_identity_sha256: profile.organization_identity_sha256.clone(),
            evidence_scope: RemoteEvidenceScope::DeterministicContractMock,
        };
        request
            .validate_for(plan, profile)
            .map_err(|_| contract_error("F2S-RGPU-MOCK-REQUEST"))?;
        jobs.insert(
            plan.idempotency_key.clone(),
            MockRemoteJob {
                plan_sha256: plan.plan_sha256.clone(),
                request: request.clone(),
                polls: 0,
                cancelled: false,
                result: None,
            },
        );
        Ok(request)
    }

    fn query(
        &self,
        profile: &RemoteGpuProfile,
        request: &RemoteRequestReceipt,
    ) -> Result<RemoteStatusObservation, RemoteTransportError> {
        profile
            .require_enabled()
            .map_err(|_| contract_error("F2S-RGPU-MOCK-PROFILE"))?;
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| contract_error("F2S-RGPU-MOCK-LOCK"))?;
        let job = jobs
            .get_mut(&request.idempotency_key)
            .ok_or_else(|| contract_error("F2S-RGPU-MOCK-NOT-FOUND"))?;
        if &job.request != request {
            return Err(contract_error("F2S-RGPU-MOCK-REQUEST-MISMATCH"));
        }
        job.polls = job.polls.saturating_add(1);
        if job.cancelled {
            return Ok(RemoteStatusObservation::Cancelled {
                remote_sequence: job.polls,
            });
        }
        if job.polls == 1 {
            return Ok(RemoteStatusObservation::Running { remote_sequence: 1 });
        }
        match job.result.clone() {
            Some((receipt, candidates)) => Ok(RemoteStatusObservation::Succeeded {
                receipt,
                candidates,
            }),
            None => Ok(RemoteStatusObservation::Running {
                remote_sequence: job.polls,
            }),
        }
    }

    fn cancel(
        &self,
        profile: &RemoteGpuProfile,
        request: &RemoteRequestReceipt,
    ) -> Result<RemoteStatusObservation, RemoteTransportError> {
        profile
            .require_enabled()
            .map_err(|_| contract_error("F2S-RGPU-MOCK-PROFILE"))?;
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| contract_error("F2S-RGPU-MOCK-LOCK"))?;
        let job = jobs
            .get_mut(&request.idempotency_key)
            .ok_or_else(|| contract_error("F2S-RGPU-MOCK-NOT-FOUND"))?;
        if &job.request != request {
            return Err(contract_error("F2S-RGPU-MOCK-REQUEST-MISMATCH"));
        }
        job.polls = job.polls.saturating_add(1);
        job.cancelled = true;
        Ok(RemoteStatusObservation::Cancelled {
            remote_sequence: job.polls.max(1),
        })
    }

    fn delete(
        &self,
        profile: &RemoteGpuProfile,
        request: &RemoteRequestReceipt,
        response: Option<&RemoteResponseReceipt>,
    ) -> Result<RemoteDeletionReceipt, RemoteTransportError> {
        profile
            .require_enabled()
            .map_err(|_| contract_error("F2S-RGPU-MOCK-PROFILE"))?;
        let jobs = self
            .jobs
            .lock()
            .map_err(|_| contract_error("F2S-RGPU-MOCK-LOCK"))?;
        let job = jobs
            .get(&request.idempotency_key)
            .ok_or_else(|| contract_error("F2S-RGPU-MOCK-NOT-FOUND"))?;
        if &job.request != request {
            return Err(contract_error("F2S-RGPU-MOCK-REQUEST-MISMATCH"));
        }
        let hashes = response
            .map(|receipt| {
                receipt
                    .outputs
                    .iter()
                    .map(|output| output.sha256.clone())
                    .collect()
            })
            .unwrap_or_default();
        RemoteDeletionReceipt::build(
            request,
            hashes,
            "2030-01-01T00:00:00Z",
            "2030-01-01T00:00:01Z",
            sha256_bytes(b"mock-server-identity-signature"),
        )
        .map_err(|_| contract_error("F2S-RGPU-MOCK-DELETION"))
    }
}

impl DeterministicPrivateRemoteMock {
    /// Binds a deterministic candidate result to an already submitted plan.
    /// Kept separate from `submit` to make the mocked execution boundary explicit.
    pub fn complete(&self, plan: &RemoteTransferPlan) -> Result<(), String> {
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| "deterministic remote mock lock is poisoned".to_owned())?;
        let job = jobs
            .get_mut(&plan.idempotency_key)
            .ok_or("deterministic remote mock job was not submitted")?;
        if job.plan_sha256 != plan.plan_sha256 {
            return Err("deterministic remote mock plan identity conflict".into());
        }
        let result = Self::candidate(plan, &job.request, job.polls.saturating_add(2))
            .map_err(|error| error.code)?;
        job.result = Some(result);
        Ok(())
    }
}

fn contract_error(code: &str) -> RemoteTransportError {
    RemoteTransportError::new(code, RemoteTransportFailureDisposition::ContractViolation)
}

fn require_local_identifier(value: &str) -> Result<(), String> {
    if (3..=96).contains(&value.len())
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
    {
        Ok(())
    } else {
        Err("invalid private remote local identifier".into())
    }
}

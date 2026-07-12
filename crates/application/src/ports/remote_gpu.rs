use f2s_domain::remote_gpu::{
    RemoteDeletionReceipt, RemoteGpuProfile, RemoteJob, RemoteOutputDescriptor,
    RemoteRequestReceipt, RemoteResponseReceipt, RemoteTransferPlan,
};

pub trait RemoteGpuProfileStore: Send + Sync {
    fn save_profile(&self, profile: &RemoteGpuProfile) -> Result<(), String>;
    fn load_profile(&self, profile_id: &str) -> Result<Option<RemoteGpuProfile>, String>;
}

pub trait RemoteJobStore: Send + Sync {
    fn create_job(&self, job: &RemoteJob) -> Result<(), String>;
    fn load_job(&self, job_id: &str) -> Result<Option<RemoteJob>, String>;
    fn compare_and_swap_job(
        &self,
        job: &RemoteJob,
        expected_persistence_sequence: u64,
    ) -> Result<(), String>;
}

pub trait RemoteCandidateQuarantine: Send + Sync {
    fn put_quarantined(
        &self,
        job_id: &str,
        descriptor: &RemoteOutputDescriptor,
        bytes: &[u8],
    ) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedRemoteCandidate {
    pub descriptor: RemoteOutputDescriptor,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteStatusObservation {
    Running {
        remote_sequence: u64,
    },
    Succeeded {
        receipt: RemoteResponseReceipt,
        candidates: Vec<DownloadedRemoteCandidate>,
    },
    Failed {
        remote_sequence: u64,
        failure_code: String,
    },
    Cancelled {
        remote_sequence: u64,
    },
    UnknownResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteTransportFailureDisposition {
    DefinitelyNotSent,
    RetryableRead,
    UnknownMutationResult,
    ContractViolation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTransportError {
    pub code: String,
    pub disposition: RemoteTransportFailureDisposition,
}

impl RemoteTransportError {
    pub fn new(code: impl Into<String>, disposition: RemoteTransportFailureDisposition) -> Self {
        Self {
            code: code.into(),
            disposition,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalCapabilityState {
    NotRunExternal,
    ContractMockOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteCapabilityReport {
    pub state: ExternalCapabilityState,
    pub network_attempted: bool,
    pub reason: String,
}

/// The production implementation owns TLS, redirect refusal, SPKI pinning and
/// Credential Manager lookup. A token is intentionally absent from every method
/// signature so it cannot enter an application DTO, project, log, or command line.
pub trait PrivateRemoteGpuTransport: Send + Sync {
    fn capability_report(&self) -> RemoteCapabilityReport;

    fn submit(
        &self,
        profile: &RemoteGpuProfile,
        plan: &RemoteTransferPlan,
    ) -> Result<RemoteRequestReceipt, RemoteTransportError>;

    fn query(
        &self,
        profile: &RemoteGpuProfile,
        request: &RemoteRequestReceipt,
    ) -> Result<RemoteStatusObservation, RemoteTransportError>;

    fn cancel(
        &self,
        profile: &RemoteGpuProfile,
        request: &RemoteRequestReceipt,
    ) -> Result<RemoteStatusObservation, RemoteTransportError>;

    fn delete(
        &self,
        profile: &RemoteGpuProfile,
        request: &RemoteRequestReceipt,
        response: Option<&RemoteResponseReceipt>,
    ) -> Result<RemoteDeletionReceipt, RemoteTransportError>;
}

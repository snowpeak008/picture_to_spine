use crate::{
    approvals::VerifiedHumanActor,
    ports::{
        PrivateRemoteGpuTransport, RemoteCandidateQuarantine, RemoteGpuProfileStore,
        RemoteJobStore, RemoteStatusObservation, RemoteTransportFailureDisposition,
    },
};
use f2s_domain::{
    canonical::sha256_bytes,
    remote_gpu::{
        REMOTE_TRANSFER_APPROVAL_PURPOSE, RemoteApprovalBinding, RemoteGpuProfile, RemoteJob,
        RemoteJobState, RemoteTransferPlan,
    },
};

pub struct RemoteGpuService<'a> {
    profiles: &'a dyn RemoteGpuProfileStore,
    jobs: &'a dyn RemoteJobStore,
    quarantine: &'a dyn RemoteCandidateQuarantine,
    transport: &'a dyn PrivateRemoteGpuTransport,
}

impl<'a> RemoteGpuService<'a> {
    pub fn new(
        profiles: &'a dyn RemoteGpuProfileStore,
        jobs: &'a dyn RemoteJobStore,
        quarantine: &'a dyn RemoteCandidateQuarantine,
        transport: &'a dyn PrivateRemoteGpuTransport,
    ) -> Self {
        Self {
            profiles,
            jobs,
            quarantine,
            transport,
        }
    }

    pub fn approve_and_create_job(
        &self,
        job_id: impl Into<String>,
        plan: RemoteTransferPlan,
        current_project_revision: u64,
        actor: VerifiedHumanActor,
    ) -> Result<RemoteJob, String> {
        let profile = self.load_enabled_profile(&plan.profile_id)?;
        plan.validate_against_profile(&profile)?;
        if plan.project_revision != current_project_revision {
            return Err("remote transfer approval targets a stale project revision".into());
        }
        actor.require_binding(REMOTE_TRANSFER_APPROVAL_PURPOSE, &plan.plan_sha256)?;
        let approval = RemoteApprovalBinding {
            actor_id: actor.actor_id().to_owned(),
            attestation_id: actor.attestation_id().to_owned(),
            verification_proof_sha256: actor.proof_sha256().to_owned(),
            approved_plan_sha256: plan.plan_sha256.clone(),
        };
        let job = RemoteJob::new(job_id, plan, approval)?;
        self.jobs.create_job(&job)?;
        Ok(job)
    }

    pub fn submit(&self, job_id: &str) -> Result<RemoteJob, String> {
        let mut job = self.load_job(job_id)?;
        let profile = self.load_enabled_profile(&job.plan.profile_id)?;
        job.plan.validate_against_profile(&profile)?;
        let expected = job.persistence_sequence();
        job.begin_submission()?;
        self.jobs.compare_and_swap_job(&job, expected)?;

        let submitting_sequence = job.persistence_sequence();
        match self.transport.submit(&profile, &job.plan) {
            Ok(receipt) => {
                if let Err(error) = job.record_request_receipt(receipt, &profile) {
                    job.mark_submission_failed(true)?;
                    self.jobs.compare_and_swap_job(&job, submitting_sequence)?;
                    return Err(format!("remote request receipt rejected: {error}"));
                }
            }
            Err(error) => {
                let unknown = !matches!(
                    error.disposition,
                    RemoteTransportFailureDisposition::DefinitelyNotSent
                );
                job.mark_submission_failed(unknown)?;
                self.jobs.compare_and_swap_job(&job, submitting_sequence)?;
                return Err(format!("remote submit failed [{}]", error.code));
            }
        }
        self.jobs.compare_and_swap_job(&job, submitting_sequence)?;
        Ok(job)
    }

    pub fn poll(&self, job_id: &str) -> Result<RemoteJob, String> {
        let mut job = self.load_job(job_id)?;
        if !matches!(
            job.state,
            RemoteJobState::Submitted | RemoteJobState::Running | RemoteJobState::CancelRequested
        ) {
            return Err("remote job is not pollable from the current state".into());
        }
        let profile = self.load_enabled_profile(&job.plan.profile_id)?;
        let request = job
            .request_receipt
            .as_ref()
            .cloned()
            .ok_or("remote job is missing its accepted request receipt")?;
        let expected = job.persistence_sequence();
        let observation = match self.transport.query(&profile, &request) {
            Ok(observation) => observation,
            Err(error) if error.disposition == RemoteTransportFailureDisposition::RetryableRead => {
                return Err(format!("remote status read is retryable [{}]", error.code));
            }
            Err(error) => {
                job.record_unknown_result()?;
                self.jobs.compare_and_swap_job(&job, expected)?;
                return Err(format!("remote status became unknown [{}]", error.code));
            }
        };
        if let Err(error) = self.apply_observation(&mut job, &profile, observation) {
            if job.persistence_sequence() > expected {
                self.jobs.compare_and_swap_job(&job, expected)?;
            }
            return Err(error);
        }
        self.jobs.compare_and_swap_job(&job, expected)?;
        Ok(job)
    }

    pub fn cancel(&self, job_id: &str) -> Result<RemoteJob, String> {
        let mut job = self.load_job(job_id)?;
        let profile = self.load_enabled_profile(&job.plan.profile_id)?;
        let expected = job.persistence_sequence();
        job.request_cancel()?;
        self.jobs.compare_and_swap_job(&job, expected)?;
        if job.state == RemoteJobState::Cancelled {
            return Ok(job);
        }
        let cancel_sequence = job.persistence_sequence();
        let request = job
            .request_receipt
            .as_ref()
            .cloned()
            .ok_or("remote cancel is missing an accepted request")?;
        match self.transport.cancel(&profile, &request) {
            Ok(observation) => {
                if let Err(error) = self.apply_observation(&mut job, &profile, observation) {
                    if job.persistence_sequence() > cancel_sequence {
                        self.jobs.compare_and_swap_job(&job, cancel_sequence)?;
                    }
                    return Err(error);
                }
            }
            Err(error) => {
                job.record_unknown_result()?;
                self.jobs.compare_and_swap_job(&job, cancel_sequence)?;
                return Err(format!("remote cancel result is unknown [{}]", error.code));
            }
        }
        self.jobs.compare_and_swap_job(&job, cancel_sequence)?;
        Ok(job)
    }

    pub fn delete_remote_artifacts(&self, job_id: &str) -> Result<RemoteJob, String> {
        let mut job = self.load_job(job_id)?;
        if !job.state.is_terminal() || job.deletion_receipt.is_some() {
            return Err("remote job is not awaiting a deletion receipt".into());
        }
        let profile = self.load_enabled_profile(&job.plan.profile_id)?;
        let request = job
            .request_receipt
            .as_ref()
            .cloned()
            .ok_or("remote deletion is missing an accepted request")?;
        let expected = job.persistence_sequence();
        let receipt = self
            .transport
            .delete(&profile, &request, job.response_receipt.as_ref())
            .map_err(|error| {
                format!(
                    "remote deletion receipt remains external/unverified [{}]",
                    error.code
                )
            })?;
        job.record_deletion_receipt(receipt)?;
        self.jobs.compare_and_swap_job(&job, expected)?;
        Ok(job)
    }

    pub fn load_job(&self, job_id: &str) -> Result<RemoteJob, String> {
        let job = self
            .jobs
            .load_job(job_id)?
            .ok_or_else(|| "remote job was not found".to_owned())?;
        job.validate_persisted()?;
        Ok(job)
    }

    fn apply_observation(
        &self,
        job: &mut RemoteJob,
        profile: &RemoteGpuProfile,
        observation: RemoteStatusObservation,
    ) -> Result<(), String> {
        match observation {
            RemoteStatusObservation::Running { remote_sequence } => {
                job.observe_running(remote_sequence)
            }
            RemoteStatusObservation::Succeeded {
                receipt,
                mut candidates,
            } => {
                let request = job
                    .request_receipt
                    .as_ref()
                    .ok_or("remote response has no accepted request")?;
                if let Err(error) = receipt.validate_for(&job.plan, request, profile) {
                    job.record_unknown_result()?;
                    return Err(format!("remote response receipt rejected: {error}"));
                }
                if candidates.len() != receipt.outputs.len() {
                    job.record_unknown_result()?;
                    return Err("remote response payload count does not match its manifest".into());
                }
                candidates.sort_by(|left, right| {
                    left.descriptor
                        .artifact_id
                        .cmp(&right.descriptor.artifact_id)
                });
                for (candidate, descriptor) in candidates.iter().zip(&receipt.outputs) {
                    if &candidate.descriptor != descriptor
                        || candidate.bytes.len() as u64 != descriptor.byte_length
                        || sha256_bytes(&candidate.bytes) != descriptor.sha256
                    {
                        job.record_unknown_result()?;
                        return Err(
                            "remote candidate bytes failed hash/size/manifest validation".into(),
                        );
                    }
                    descriptor.validate_for(job.plan.method, profile.max_response_bytes)?;
                }
                for candidate in candidates {
                    self.quarantine.put_quarantined(
                        &job.job_id,
                        &candidate.descriptor,
                        &candidate.bytes,
                    )?;
                }
                job.record_success(receipt, profile)
            }
            RemoteStatusObservation::Failed {
                remote_sequence,
                failure_code,
            } => {
                if failure_code.trim().is_empty() || failure_code.len() > 96 {
                    job.record_unknown_result()?;
                    Err("remote failure response has an invalid code".into())
                } else {
                    job.record_failed(remote_sequence)
                }
            }
            RemoteStatusObservation::Cancelled { remote_sequence } => {
                job.record_cancelled(remote_sequence)
            }
            RemoteStatusObservation::UnknownResult => job.record_unknown_result(),
        }
    }

    fn load_enabled_profile(&self, profile_id: &str) -> Result<RemoteGpuProfile, String> {
        let profile = self
            .profiles
            .load_profile(profile_id)?
            .ok_or_else(|| "private remote profile was not found".to_owned())?;
        profile.require_enabled()?;
        Ok(profile)
    }
}

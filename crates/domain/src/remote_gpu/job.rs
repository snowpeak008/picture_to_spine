use super::{
    RemoteGpuProfile, RemoteOutputDescriptor, RemoteTransferPlan, is_lower_hex_sha256,
    is_safe_identifier,
};
use crate::canonical::canonical_sha256;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RemoteEvidenceScope {
    DeterministicContractMock,
    ExternalPrivateEndpoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteApprovalBinding {
    pub actor_id: String,
    pub attestation_id: String,
    pub verification_proof_sha256: String,
    pub approved_plan_sha256: String,
}

impl RemoteApprovalBinding {
    pub fn validate_for(&self, plan: &RemoteTransferPlan) -> Result<(), String> {
        if self.actor_id.trim().is_empty()
            || self.actor_id.len() > 128
            || self.attestation_id.trim().is_empty()
            || self.attestation_id.len() > 128
            || !is_lower_hex_sha256(&self.verification_proof_sha256)
            || self.approved_plan_sha256 != plan.plan_sha256
        {
            return Err("remote approval is not bound to the exact transfer plan".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteRequestReceipt {
    pub api_version: String,
    pub provider_job_id: String,
    pub idempotency_key: String,
    pub request_sha256: String,
    pub accepted_manifest_sha256: String,
    pub server_capability_sha256: String,
    pub retention_deadline_utc: String,
    pub event_sequence_start: u64,
    pub observed_origin: String,
    pub observed_spki_sha256: String,
    pub observed_organization_identity_sha256: String,
    pub evidence_scope: RemoteEvidenceScope,
}

impl RemoteRequestReceipt {
    pub fn validate_for(
        &self,
        plan: &RemoteTransferPlan,
        profile: &RemoteGpuProfile,
    ) -> Result<(), String> {
        plan.validate_against_profile(profile)?;
        if self.api_version != "f2s-rgpu/v1"
            || !is_provider_identifier(&self.provider_job_id)
            || self.idempotency_key != plan.idempotency_key
            || self.request_sha256 != plan.plan_sha256
            || self.accepted_manifest_sha256 != plan.plan_sha256
            || !is_lower_hex_sha256(&self.server_capability_sha256)
            || self.retention_deadline_utc.trim().is_empty()
            || self.observed_origin != profile.origin
            || self.observed_spki_sha256 != profile.certificate_spki_sha256
            || self.observed_organization_identity_sha256 != profile.organization_identity_sha256
        {
            return Err(
                "remote request receipt does not bind the approved endpoint and request".into(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteResponseReceipt {
    pub api_version: String,
    pub provider_job_id: String,
    pub request_sha256: String,
    pub response_manifest_sha256: String,
    pub event_sequence: u64,
    pub outputs: Vec<RemoteOutputDescriptor>,
    pub evidence_scope: RemoteEvidenceScope,
}

impl RemoteResponseReceipt {
    pub fn build(
        request: &RemoteRequestReceipt,
        event_sequence: u64,
        mut outputs: Vec<RemoteOutputDescriptor>,
    ) -> Result<Self, String> {
        outputs.sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));
        if outputs.is_empty()
            || outputs
                .windows(2)
                .any(|pair| pair[0].artifact_id == pair[1].artifact_id)
        {
            return Err("remote response output manifest must be non-empty and unique".into());
        }
        let response_manifest_sha256 =
            canonical_sha256(&outputs).map_err(|error| error.to_string())?;
        Ok(Self {
            api_version: "f2s-rgpu/v1".into(),
            provider_job_id: request.provider_job_id.clone(),
            request_sha256: request.request_sha256.clone(),
            response_manifest_sha256,
            event_sequence,
            outputs,
            evidence_scope: request.evidence_scope,
        })
    }

    pub fn validate_for(
        &self,
        plan: &RemoteTransferPlan,
        request: &RemoteRequestReceipt,
        profile: &RemoteGpuProfile,
    ) -> Result<(), String> {
        request.validate_for(plan, profile)?;
        if self.api_version != "f2s-rgpu/v1"
            || self.provider_job_id != request.provider_job_id
            || self.request_sha256 != plan.plan_sha256
            || self.evidence_scope != request.evidence_scope
            || self.event_sequence < request.event_sequence_start
            || self.outputs.is_empty()
            || self
                .outputs
                .windows(2)
                .any(|pair| pair[0].artifact_id >= pair[1].artifact_id)
            || canonical_sha256(&self.outputs).map_err(|error| error.to_string())?
                != self.response_manifest_sha256
        {
            return Err("remote response manifest does not bind the accepted request".into());
        }
        let mut total = 0u64;
        for output in &self.outputs {
            output.validate_for(plan.method, profile.max_response_bytes)?;
            total = total
                .checked_add(output.byte_length)
                .ok_or("remote response byte size overflow")?;
        }
        if total > profile.max_response_bytes {
            return Err("remote response exceeds the profile byte budget".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteDeletionReceipt {
    pub api_version: String,
    pub provider_job_id: String,
    pub request_sha256: String,
    pub artifact_sha256: Vec<String>,
    pub requested_at_utc: String,
    pub deleted_at_utc: String,
    pub status: String,
    pub server_identity_signature_sha256: String,
    pub receipt_sha256: String,
    pub evidence_scope: RemoteEvidenceScope,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeletionHashPayload<'a> {
    api_version: &'a str,
    provider_job_id: &'a str,
    request_sha256: &'a str,
    artifact_sha256: &'a [String],
    requested_at_utc: &'a str,
    deleted_at_utc: &'a str,
    status: &'a str,
    server_identity_signature_sha256: &'a str,
    evidence_scope: RemoteEvidenceScope,
}

impl RemoteDeletionReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        request: &RemoteRequestReceipt,
        mut artifact_sha256: Vec<String>,
        requested_at_utc: impl Into<String>,
        deleted_at_utc: impl Into<String>,
        server_identity_signature_sha256: impl Into<String>,
    ) -> Result<Self, String> {
        artifact_sha256.sort();
        artifact_sha256.dedup();
        let mut receipt = Self {
            api_version: "f2s-rgpu/v1".into(),
            provider_job_id: request.provider_job_id.clone(),
            request_sha256: request.request_sha256.clone(),
            artifact_sha256,
            requested_at_utc: requested_at_utc.into(),
            deleted_at_utc: deleted_at_utc.into(),
            status: "DELETED".into(),
            server_identity_signature_sha256: server_identity_signature_sha256.into(),
            receipt_sha256: String::new(),
            evidence_scope: request.evidence_scope,
        };
        receipt.receipt_sha256 = receipt.recompute_sha256()?;
        Ok(receipt)
    }

    pub fn recompute_sha256(&self) -> Result<String, String> {
        canonical_sha256(&DeletionHashPayload {
            api_version: &self.api_version,
            provider_job_id: &self.provider_job_id,
            request_sha256: &self.request_sha256,
            artifact_sha256: &self.artifact_sha256,
            requested_at_utc: &self.requested_at_utc,
            deleted_at_utc: &self.deleted_at_utc,
            status: &self.status,
            server_identity_signature_sha256: &self.server_identity_signature_sha256,
            evidence_scope: self.evidence_scope,
        })
        .map_err(|error| error.to_string())
    }

    pub fn validate_for(
        &self,
        request: &RemoteRequestReceipt,
        response: Option<&RemoteResponseReceipt>,
    ) -> Result<(), String> {
        let mut expected_artifacts: Vec<_> = response
            .map(|value| {
                value
                    .outputs
                    .iter()
                    .map(|output| output.sha256.clone())
                    .collect()
            })
            .unwrap_or_default();
        expected_artifacts.sort();
        if self.api_version != "f2s-rgpu/v1"
            || self.provider_job_id != request.provider_job_id
            || self.request_sha256 != request.request_sha256
            || self.status != "DELETED"
            || self.requested_at_utc.trim().is_empty()
            || self.deleted_at_utc.trim().is_empty()
            || self.evidence_scope != request.evidence_scope
            || self.artifact_sha256 != expected_artifacts
            || self
                .artifact_sha256
                .iter()
                .any(|hash| !is_lower_hex_sha256(hash))
            || !is_lower_hex_sha256(&self.server_identity_signature_sha256)
            || self.recompute_sha256()? != self.receipt_sha256
        {
            return Err("remote deletion receipt does not bind the accepted job artifacts".into());
        }
        Ok(())
    }

    pub fn proves_local_deletion(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RemoteJobState {
    Approved,
    Submitting,
    Submitted,
    Running,
    CancelRequested,
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
}

impl RemoteJobState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Interrupted
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteJobTransition {
    pub sequence: u64,
    pub from: Option<RemoteJobState>,
    pub to: RemoteJobState,
    pub reason_code: String,
    pub previous_transition_sha256: Option<String>,
    pub transition_sha256: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TransitionHashPayload<'a> {
    sequence: u64,
    from: Option<RemoteJobState>,
    to: RemoteJobState,
    reason_code: &'a str,
    previous_transition_sha256: &'a Option<String>,
}

impl RemoteJobTransition {
    fn new(
        sequence: u64,
        from: Option<RemoteJobState>,
        to: RemoteJobState,
        reason_code: impl Into<String>,
        previous_transition_sha256: Option<String>,
    ) -> Result<Self, String> {
        let reason_code = reason_code.into();
        if !is_reason_code(&reason_code) {
            return Err("remote job transition reason code is invalid".into());
        }
        let transition_sha256 = canonical_sha256(&TransitionHashPayload {
            sequence,
            from,
            to,
            reason_code: &reason_code,
            previous_transition_sha256: &previous_transition_sha256,
        })
        .map_err(|error| error.to_string())?;
        Ok(Self {
            sequence,
            from,
            to,
            reason_code,
            previous_transition_sha256,
            transition_sha256,
        })
    }

    fn validate(&self) -> Result<(), String> {
        let expected = Self::new(
            self.sequence,
            self.from,
            self.to,
            self.reason_code.clone(),
            self.previous_transition_sha256.clone(),
        )?;
        if expected.transition_sha256 != self.transition_sha256 {
            return Err("remote job transition hash chain is invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteJob {
    pub schema_version: String,
    pub job_id: String,
    pub state: RemoteJobState,
    pub plan: RemoteTransferPlan,
    pub approval: RemoteApprovalBinding,
    pub request_receipt: Option<RemoteRequestReceipt>,
    pub response_receipt: Option<RemoteResponseReceipt>,
    pub deletion_receipt: Option<RemoteDeletionReceipt>,
    pub last_remote_sequence: Option<u64>,
    pub late_success_quarantined: bool,
    pub transitions: Vec<RemoteJobTransition>,
}

impl RemoteJob {
    pub fn new(
        job_id: impl Into<String>,
        plan: RemoteTransferPlan,
        approval: RemoteApprovalBinding,
    ) -> Result<Self, String> {
        let job_id = job_id.into();
        if !is_safe_identifier(&job_id)
            || plan.recompute_sha256()? != plan.plan_sha256
            || plan.idempotency_key != format!("sha256:{}", plan.plan_sha256)
        {
            return Err("remote job or transfer plan identity is invalid".into());
        }
        approval.validate_for(&plan)?;
        let initial = RemoteJobTransition::new(
            0,
            None,
            RemoteJobState::Approved,
            "APPROVAL_PERSISTED",
            None,
        )?;
        Ok(Self {
            schema_version: "1.0.0".into(),
            job_id,
            state: RemoteJobState::Approved,
            plan,
            approval,
            request_receipt: None,
            response_receipt: None,
            deletion_receipt: None,
            last_remote_sequence: None,
            late_success_quarantined: false,
            transitions: vec![initial],
        })
    }

    pub fn persistence_sequence(&self) -> u64 {
        self.transitions
            .last()
            .map(|transition| transition.sequence)
            .unwrap_or_default()
    }

    pub fn validate_persisted(&self) -> Result<(), String> {
        if self.schema_version != "1.0.0"
            || !is_safe_identifier(&self.job_id)
            || self.plan.recompute_sha256()? != self.plan.plan_sha256
            || self.plan.idempotency_key != format!("sha256:{}", self.plan.plan_sha256)
            || self.transitions.is_empty()
            || self.transitions.last().map(|item| item.to) != Some(self.state)
        {
            return Err("persisted remote job identity or state is invalid".into());
        }
        self.approval.validate_for(&self.plan)?;
        for (index, transition) in self.transitions.iter().enumerate() {
            transition.validate()?;
            if transition.sequence != index as u64
                || (index == 0
                    && (transition.from.is_some()
                        || transition.to != RemoteJobState::Approved
                        || transition.previous_transition_sha256.is_some()))
                || (index > 0
                    && (transition.from != Some(self.transitions[index - 1].to)
                        || transition.previous_transition_sha256
                            != Some(self.transitions[index - 1].transition_sha256.clone())))
            {
                return Err("persisted remote job transition order is invalid".into());
            }
        }
        if self.request_receipt.is_none()
            && matches!(
                self.state,
                RemoteJobState::Submitted
                    | RemoteJobState::Running
                    | RemoteJobState::CancelRequested
                    | RemoteJobState::Succeeded
            )
        {
            return Err("persisted remote job state is missing its request receipt".into());
        }
        if self.state == RemoteJobState::Succeeded && self.response_receipt.is_none() {
            return Err("persisted successful remote job is missing its response receipt".into());
        }
        if let Some(request) = &self.request_receipt {
            if request.api_version != "f2s-rgpu/v1"
                || !is_provider_identifier(&request.provider_job_id)
                || request.idempotency_key != self.plan.idempotency_key
                || request.request_sha256 != self.plan.plan_sha256
                || request.accepted_manifest_sha256 != self.plan.plan_sha256
                || !is_lower_hex_sha256(&request.server_capability_sha256)
                || request.retention_deadline_utc.trim().is_empty()
                || request.observed_origin != self.plan.endpoint_origin
                || request.observed_spki_sha256 != self.plan.endpoint_spki_sha256
                || request.observed_organization_identity_sha256
                    != self.plan.organization_identity_sha256
            {
                return Err("persisted remote request receipt is invalid".into());
            }
        }
        if let Some(response) = &self.response_receipt {
            let request = self
                .request_receipt
                .as_ref()
                .ok_or("persisted response has no request receipt")?;
            if response.api_version != "f2s-rgpu/v1"
                || response.provider_job_id != request.provider_job_id
                || response.request_sha256 != self.plan.plan_sha256
                || response.evidence_scope != request.evidence_scope
                || response.event_sequence < request.event_sequence_start
                || response.outputs.is_empty()
                || response
                    .outputs
                    .windows(2)
                    .any(|pair| pair[0].artifact_id >= pair[1].artifact_id)
                || canonical_sha256(&response.outputs).map_err(|error| error.to_string())?
                    != response.response_manifest_sha256
            {
                return Err("persisted remote response receipt is invalid".into());
            }
            for output in &response.outputs {
                output.validate_for(self.plan.method, u64::MAX)?;
            }
        }
        if let Some(deletion) = &self.deletion_receipt {
            deletion.validate_for(
                self.request_receipt
                    .as_ref()
                    .ok_or("persisted deletion has no request receipt")?,
                self.response_receipt.as_ref(),
            )?;
        }
        Ok(())
    }

    pub fn begin_submission(&mut self) -> Result<(), String> {
        if self.state != RemoteJobState::Approved {
            return Err("remote submission is not retryable from the current state".into());
        }
        self.transition(RemoteJobState::Submitting, "SUBMISSION_STARTED")
    }

    pub fn record_request_receipt(
        &mut self,
        receipt: RemoteRequestReceipt,
        profile: &RemoteGpuProfile,
    ) -> Result<bool, String> {
        receipt.validate_for(&self.plan, profile)?;
        if let Some(existing) = &self.request_receipt {
            return if existing == &receipt {
                Ok(false)
            } else {
                Err("idempotency key returned a conflicting remote request".into())
            };
        }
        if self.state != RemoteJobState::Submitting {
            return Err("remote request receipt arrived outside submission".into());
        }
        self.last_remote_sequence = Some(receipt.event_sequence_start);
        self.request_receipt = Some(receipt);
        self.transition(RemoteJobState::Submitted, "REQUEST_ACCEPTED")?;
        Ok(true)
    }

    pub fn mark_submission_failed(&mut self, outcome_unknown: bool) -> Result<(), String> {
        if self.state != RemoteJobState::Submitting {
            return Err("submission failure arrived outside submission".into());
        }
        if outcome_unknown {
            self.transition(RemoteJobState::Interrupted, "SUBMISSION_RESULT_UNKNOWN")
        } else {
            self.transition(RemoteJobState::Failed, "SUBMISSION_NOT_SENT")
        }
    }

    pub fn observe_running(&mut self, remote_sequence: u64) -> Result<(), String> {
        self.require_new_remote_sequence(remote_sequence)?;
        match self.state {
            RemoteJobState::Submitted => {
                self.last_remote_sequence = Some(remote_sequence);
                self.transition(RemoteJobState::Running, "REMOTE_RUNNING")
            }
            RemoteJobState::Running => {
                self.last_remote_sequence = Some(remote_sequence);
                self.transition(RemoteJobState::Running, "REMOTE_PROGRESS")
            }
            _ => Err("running observation is invalid for the remote job state".into()),
        }
    }

    pub fn request_cancel(&mut self) -> Result<(), String> {
        match self.state {
            RemoteJobState::Approved => {
                self.transition(RemoteJobState::Cancelled, "CANCELLED_LOCALLY")
            }
            RemoteJobState::Submitted | RemoteJobState::Running => {
                self.transition(RemoteJobState::CancelRequested, "CANCEL_PERSISTED")
            }
            _ => Err("remote job cannot be cancelled from the current state".into()),
        }
    }

    pub fn record_success(
        &mut self,
        receipt: RemoteResponseReceipt,
        profile: &RemoteGpuProfile,
    ) -> Result<(), String> {
        let request = self
            .request_receipt
            .as_ref()
            .ok_or("remote success has no accepted request")?;
        receipt.validate_for(&self.plan, request, profile)?;
        self.require_new_remote_sequence(receipt.event_sequence)?;
        self.last_remote_sequence = Some(receipt.event_sequence);
        if self.state == RemoteJobState::CancelRequested {
            self.response_receipt = Some(receipt);
            self.late_success_quarantined = true;
            self.transition(RemoteJobState::CancelRequested, "LATE_SUCCESS_QUARANTINED")
        } else if matches!(
            self.state,
            RemoteJobState::Submitted | RemoteJobState::Running
        ) {
            self.response_receipt = Some(receipt);
            self.transition(RemoteJobState::Succeeded, "CANDIDATE_RESPONSE_VERIFIED")
        } else {
            Err("remote success arrived outside an active job".into())
        }
    }

    pub fn record_failed(&mut self, remote_sequence: u64) -> Result<(), String> {
        self.require_new_remote_sequence(remote_sequence)?;
        if !matches!(
            self.state,
            RemoteJobState::Submitted | RemoteJobState::Running | RemoteJobState::CancelRequested
        ) {
            return Err("remote failure arrived outside an active job".into());
        }
        self.last_remote_sequence = Some(remote_sequence);
        self.transition(RemoteJobState::Failed, "REMOTE_FAILED")
    }

    pub fn record_cancelled(&mut self, remote_sequence: u64) -> Result<(), String> {
        self.require_new_remote_sequence(remote_sequence)?;
        if self.state != RemoteJobState::CancelRequested {
            return Err("remote cancellation arrived without a persisted cancel request".into());
        }
        self.last_remote_sequence = Some(remote_sequence);
        self.transition(RemoteJobState::Cancelled, "REMOTE_CANCELLED")
    }

    pub fn record_unknown_result(&mut self) -> Result<(), String> {
        if !matches!(
            self.state,
            RemoteJobState::Submitting
                | RemoteJobState::Submitted
                | RemoteJobState::Running
                | RemoteJobState::CancelRequested
        ) {
            return Err("unknown remote result arrived outside an active job".into());
        }
        self.transition(RemoteJobState::Interrupted, "REMOTE_RESULT_UNKNOWN")
    }

    pub fn record_deletion_receipt(
        &mut self,
        receipt: RemoteDeletionReceipt,
    ) -> Result<(), String> {
        if !self.state.is_terminal() || self.deletion_receipt.is_some() {
            return Err("remote deletion receipt is not expected for this job".into());
        }
        let request = self
            .request_receipt
            .as_ref()
            .ok_or("remote deletion receipt has no accepted request")?;
        receipt.validate_for(request, self.response_receipt.as_ref())?;
        self.deletion_receipt = Some(receipt);
        self.transition(self.state, "DELETION_RECEIPT_RECORDED")
    }

    pub fn candidate_contract_validated(&self) -> bool {
        self.state == RemoteJobState::Succeeded
            && self.response_receipt.is_some()
            && !self.late_success_quarantined
    }

    pub fn candidate_eligible_for_project_registration(&self) -> bool {
        self.candidate_contract_validated()
            && self.request_receipt.as_ref().is_some_and(|receipt| {
                receipt.evidence_scope == RemoteEvidenceScope::ExternalPrivateEndpoint
            })
    }

    pub fn has_external_deletion_receipt(&self) -> bool {
        self.deletion_receipt.as_ref().is_some_and(|receipt| {
            receipt.evidence_scope == RemoteEvidenceScope::ExternalPrivateEndpoint
        })
    }

    fn require_new_remote_sequence(&self, remote_sequence: u64) -> Result<(), String> {
        if self
            .last_remote_sequence
            .is_some_and(|last| remote_sequence <= last)
        {
            return Err("remote event sequence is duplicate or out of order".into());
        }
        Ok(())
    }

    fn transition(
        &mut self,
        next: RemoteJobState,
        reason_code: impl Into<String>,
    ) -> Result<(), String> {
        if self.state.is_terminal() && next != self.state {
            return Err("remote terminal job state is immutable".into());
        }
        let previous_hash = self
            .transitions
            .last()
            .map(|transition| transition.transition_sha256.clone());
        let sequence = self
            .persistence_sequence()
            .checked_add(1)
            .ok_or("remote transition sequence overflow")?;
        let transition =
            RemoteJobTransition::new(sequence, Some(self.state), next, reason_code, previous_hash)?;
        self.state = next;
        self.transitions.push(transition);
        Ok(())
    }
}

fn is_provider_identifier(value: &str) -> bool {
    (3..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        && !value.contains("..")
}

fn is_reason_code(value: &str) -> bool {
    (3..=64).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

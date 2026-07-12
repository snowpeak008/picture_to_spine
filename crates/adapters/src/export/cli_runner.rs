use super::cli_policy::{
    REQUIRED_SPINE_PATCH, SpineCliPolicy, ValidatedSpineCli, reject_reparse_components,
    sha256_bytes, sha256_file, validate_local_absolute_path_shape,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const RECEIPT_SCHEMA: &str = "f2s-spine-cli-operation-provenance/1.0";
const MAX_CONSENT_WINDOW_MS: u64 = 5 * 60 * 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExternalEvidenceClass {
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CliExecutionState {
    NotRun,
    Failed,
    Succeeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpineCliOperationKind {
    ImportProject,
    ExportBinary,
    PackAtlas,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpineCliOperation {
    /// Imports an open Spine JSON file into a new `.spine` project.
    ImportProject {
        source_json: PathBuf,
        output_project: PathBuf,
    },
    /// Exports one `.spine` project to binary data in a new, empty directory.
    ExportBinary {
        source_project: PathBuf,
        output_directory: PathBuf,
    },
    /// Packs PNG inputs into an atlas in a new, empty directory.
    PackAtlas {
        input_directory: PathBuf,
        output_directory: PathBuf,
        pack_settings_json: PathBuf,
    },
}

impl SpineCliOperation {
    pub fn kind(&self) -> SpineCliOperationKind {
        match self {
            Self::ImportProject { .. } => SpineCliOperationKind::ImportProject,
            Self::ExportBinary { .. } => SpineCliOperationKind::ExportBinary,
            Self::PackAtlas { .. } => SpineCliOperationKind::PackAtlas,
        }
    }

    /// Contains only fixed switches and operand roles, never local paths or credentials.
    pub fn safe_command_summary(&self) -> CliCommandSummary {
        let argv_shape = match self {
            Self::ImportProject { .. } => vec![
                "--input",
                "<SOURCE_JSON>",
                "--output",
                "<NEW_PROJECT.SPINE>",
                "--import",
            ],
            Self::ExportBinary { .. } => vec![
                "--input",
                "<SOURCE_PROJECT.SPINE>",
                "--output",
                "<NEW_EMPTY_DIRECTORY>",
                "--export",
                "binary",
            ],
            Self::PackAtlas { .. } => vec![
                "--input",
                "<PNG_DIRECTORY>",
                "--output",
                "<NEW_EMPTY_DIRECTORY>",
                "--pack",
                "<PACK_SETTINGS_JSON>",
            ],
        };
        CliCommandSummary {
            operation_kind: self.kind(),
            executable_name: "Spine.com".into(),
            argv_shape: argv_shape.into_iter().map(str::to_owned).collect(),
            shell_used: false,
            update_or_activation_switch_used: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliCommandSummary {
    pub operation_kind: SpineCliOperationKind,
    pub executable_name: String,
    pub argv_shape: Vec<String>,
    pub shell_used: bool,
    pub update_or_activation_switch_used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliConsentBinding {
    binding_sha256: String,
    executable_sha256: String,
    operation_id: String,
    operation_kind: SpineCliOperationKind,
    command_summary: CliCommandSummary,
}

impl CliConsentBinding {
    pub fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub fn executable_sha256(&self) -> &str {
        &self.executable_sha256
    }

    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    pub fn operation_kind(&self) -> SpineCliOperationKind {
        self.operation_kind
    }

    pub fn command_summary(&self) -> &CliCommandSummary {
        &self.command_summary
    }
}

/// Evidence produced by the native human-confirmation boundary. The adapter validates that the
/// attested payload is the exact prepared operation. It never manufactures this evidence itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedHumanCliConfirmation {
    pub operation_id: String,
    pub confirmation_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub attested_payload_sha256: String,
    pub native_attestation_sha256: String,
    pub issued_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
}

/// Deliberately not `Clone` or serializable: one value authorizes one runner call in this process.
#[derive(Debug)]
pub struct SpineCliOperationConsent {
    operation_id: String,
    confirmation_id: String,
    actor_id: String,
    binding_sha256: String,
    executable_sha256: String,
    native_attestation_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
}

impl SpineCliOperationConsent {
    pub fn from_verified_human_confirmation(
        binding: &CliConsentBinding,
        evidence: VerifiedHumanCliConfirmation,
    ) -> Result<Self, String> {
        validate_identifier("operation id", &evidence.operation_id)?;
        validate_identifier("confirmation id", &evidence.confirmation_id)?;
        validate_identifier("actor id", &evidence.actor_id)?;
        if evidence.actor_kind != "HUMAN" {
            return Err("Spine CLI operation requires a verified HUMAN actor".into());
        }
        validate_sha256("attested payload sha256", &evidence.attested_payload_sha256)?;
        validate_sha256(
            "native attestation sha256",
            &evidence.native_attestation_sha256,
        )?;
        if evidence.attested_payload_sha256 != binding.binding_sha256 {
            return Err("human confirmation is bound to a different CLI operation".into());
        }
        if evidence.operation_id != binding.operation_id {
            return Err(
                "human confirmation operation id does not match its attested payload".into(),
            );
        }
        if evidence.expires_at_unix_ms <= evidence.issued_at_unix_ms
            || evidence.expires_at_unix_ms - evidence.issued_at_unix_ms > MAX_CONSENT_WINDOW_MS
        {
            return Err(
                "human confirmation lifetime exceeds the five-minute operation window".into(),
            );
        }
        let now = unix_ms()?;
        if evidence.issued_at_unix_ms > now.saturating_add(5_000) {
            return Err("human confirmation timestamp is in the future".into());
        }
        if now > evidence.expires_at_unix_ms {
            return Err("human confirmation has expired".into());
        }
        Ok(Self {
            operation_id: evidence.operation_id,
            confirmation_id: evidence.confirmation_id,
            actor_id: evidence.actor_id,
            binding_sha256: binding.binding_sha256.clone(),
            executable_sha256: binding.executable_sha256.clone(),
            native_attestation_sha256: evidence.native_attestation_sha256,
            issued_at_unix_ms: evidence.issued_at_unix_ms,
            expires_at_unix_ms: evidence.expires_at_unix_ms,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SpineCliRunnerLimits {
    pub probe_timeout: Duration,
    pub operation_timeout: Duration,
    pub stdout_limit_bytes: usize,
    pub stderr_limit_bytes: usize,
    pub snapshot_file_limit: usize,
    pub snapshot_byte_limit: u64,
}

impl Default for SpineCliRunnerLimits {
    fn default() -> Self {
        Self {
            probe_timeout: Duration::from_secs(20),
            operation_timeout: Duration::from_secs(10 * 60),
            stdout_limit_bytes: 256 * 1024,
            stderr_limit_bytes: 256 * 1024,
            snapshot_file_limit: 16_384,
            snapshot_byte_limit: 16 * 1024 * 1024 * 1024,
        }
    }
}

impl SpineCliRunnerLimits {
    pub fn validate(&self) -> Result<(), String> {
        if !(Duration::from_millis(100)..=Duration::from_secs(60)).contains(&self.probe_timeout) {
            return Err("probe timeout must be between 100ms and 60s".into());
        }
        if !(Duration::from_secs(1)..=Duration::from_secs(60 * 60))
            .contains(&self.operation_timeout)
        {
            return Err("operation timeout must be between 1s and 60m".into());
        }
        if !(4 * 1024..=4 * 1024 * 1024).contains(&self.stdout_limit_bytes)
            || !(4 * 1024..=4 * 1024 * 1024).contains(&self.stderr_limit_bytes)
        {
            return Err("stdout/stderr limits must be between 4KiB and 4MiB".into());
        }
        if !(1..=100_000).contains(&self.snapshot_file_limit) || self.snapshot_byte_limit == 0 {
            return Err("invalid operation snapshot limit".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SpineCliRunner {
    limits: SpineCliRunnerLimits,
    consumed_authorities: Arc<Mutex<BTreeMap<String, u64>>>,
}

impl SpineCliRunner {
    pub fn new(limits: SpineCliRunnerLimits) -> Result<Self, String> {
        limits.validate()?;
        Ok(Self {
            limits,
            consumed_authorities: Arc::new(Mutex::new(BTreeMap::new())),
        })
    }

    /// Read-only assessment. It never launches Spine. Therefore even a valid selection remains
    /// EXTERNAL/NOT_RUN until a human-authorized operation performs the version probe.
    pub fn assess_selection(&self, policy: &SpineCliPolicy) -> CliSelectionAssessment {
        match policy.validate_selected_executable() {
            Ok(executable) => CliSelectionAssessment {
                evidence_class: ExternalEvidenceClass::External,
                state: CliExecutionState::NotRun,
                executable_path_token: Some(executable.path_token()),
                executable_sha256: Some(executable.executable_sha256().to_owned()),
                reason_code: "VERSION_PROBE_REQUIRES_CURRENT_HUMAN_CONSENT".into(),
            },
            Err(_) => CliSelectionAssessment {
                evidence_class: ExternalEvidenceClass::External,
                state: CliExecutionState::NotRun,
                executable_path_token: None,
                executable_sha256: None,
                reason_code: "EXTERNAL_CLI_UNAVAILABLE_OR_REJECTED".into(),
            },
        }
    }

    /// Hashes all bound inputs without launching Spine. Callers should present the returned safe
    /// summary to the user, then bind the native human attestation to `binding_sha256`.
    pub fn prepare_consent_binding(
        &self,
        policy: &SpineCliPolicy,
        operation: &SpineCliOperation,
        operation_id: &str,
    ) -> Result<CliConsentBinding, String> {
        validate_identifier("operation id", operation_id)?;
        let executable = policy.validate_selected_executable()?;
        let prepared = prepare_operation(operation, &self.limits)?;
        Ok(build_consent_binding(
            &executable,
            operation,
            &prepared,
            operation_id,
        ))
    }

    pub fn run(
        &self,
        policy: &SpineCliPolicy,
        operation: &SpineCliOperation,
        consent: SpineCliOperationConsent,
    ) -> SpineCliOperationReport {
        let started_at_unix_ms = unix_ms().unwrap_or_default();
        let executable = match policy.validate_selected_executable() {
            Ok(value) => value,
            Err(_) => {
                return SpineCliOperationReport::not_run("EXTERNAL_CLI_UNAVAILABLE_OR_REJECTED");
            }
        };
        let prepared = match prepare_operation(operation, &self.limits) {
            Ok(value) => value,
            Err(_) => return SpineCliOperationReport::not_run("OPERATION_PATH_POLICY_REJECTED"),
        };
        let binding =
            build_consent_binding(&executable, operation, &prepared, &consent.operation_id);
        let now = unix_ms().unwrap_or(u64::MAX);
        if consent.binding_sha256 != binding.binding_sha256
            || consent.executable_sha256 != executable.executable_sha256()
            || now > consent.expires_at_unix_ms
            || consent.issued_at_unix_ms > now.saturating_add(5_000)
        {
            return SpineCliOperationReport::not_run("CURRENT_OPERATION_CONSENT_INVALID");
        }
        let accepted_once = self
            .consumed_authorities
            .lock()
            .ok()
            .map(|mut authorities| {
                authorities.retain(|_, expires_at| *expires_at >= now);
                let confirmation_key = format!("confirmation:{}", consent.confirmation_id);
                let operation_key = format!("operation:{}", consent.operation_id);
                if authorities.len() > 8_190
                    || authorities.contains_key(&confirmation_key)
                    || authorities.contains_key(&operation_key)
                {
                    return false;
                }
                authorities.insert(confirmation_key, consent.expires_at_unix_ms);
                authorities.insert(operation_key, consent.expires_at_unix_ms);
                true
            })
            .unwrap_or(false);
        if !accepted_once {
            return SpineCliOperationReport::not_run("CURRENT_OPERATION_CONSENT_ALREADY_USED");
        }
        // Keep a no-delete/no-write sharing handle open for the entire probe and operation. This
        // closes the normal Windows replace-after-hash race while CreateProcess opens Spine.com.
        let _executable_guard = match lock_executable(executable.canonical_executable()) {
            Ok(value) => value,
            Err(_) => return SpineCliOperationReport::not_run("EXECUTABLE_LOCK_FAILED"),
        };

        let pre_probe = run_version_probe(&executable, &self.limits);
        if pre_probe.state != CliExecutionState::Succeeded {
            return SpineCliOperationReport {
                evidence_class: ExternalEvidenceClass::External,
                state: CliExecutionState::Failed,
                failure_code: pre_probe.failure_code.clone(),
                probe: Some(pre_probe),
                provenance: None,
            };
        }
        let executable_hash_before = match sha256_file(executable.canonical_executable()) {
            Ok(value) if value == consent.executable_sha256 => value,
            _ => {
                return SpineCliOperationReport {
                    evidence_class: ExternalEvidenceClass::External,
                    state: CliExecutionState::Failed,
                    failure_code: Some("EXECUTABLE_CHANGED_AFTER_CONSENT".into()),
                    probe: Some(pre_probe),
                    provenance: None,
                };
            }
        };
        let process = run_process(
            executable.canonical_executable(),
            &prepared.argv,
            self.limits.operation_timeout,
            self.limits.stdout_limit_bytes,
            self.limits.stderr_limit_bytes,
        );
        let finished_at_unix_ms = unix_ms().unwrap_or(started_at_unix_ms);
        let post_inputs = snapshot_inputs(&prepared.inputs, &self.limits);
        let post_output = snapshot_path(&prepared.output.path, &self.limits, true);
        let executable_hash_after = sha256_file(executable.canonical_executable());

        let mut failure_code = process_failure_code(&process);
        if failure_code.is_none()
            && executable_hash_after.as_ref().ok().map(String::as_str)
                != Some(executable_hash_before.as_str())
        {
            failure_code = Some("EXECUTABLE_CHANGED_DURING_OPERATION".into());
        }
        if failure_code.is_none()
            && post_inputs
                .as_ref()
                .map(|items| !input_snapshots_equal(&prepared.inputs, items))
                .unwrap_or(true)
        {
            failure_code = Some("INPUT_CHANGED_DURING_OPERATION".into());
        }

        let post_output = match post_output {
            Ok(value) => Some(value),
            Err(_) => {
                if failure_code.is_none() {
                    failure_code = Some("OUTPUT_SNAPSHOT_REJECTED".into());
                }
                None
            }
        };
        let mut changed_outputs = Vec::new();
        let mut proprietary_outputs = Vec::new();
        if let Some(after) = &post_output {
            changed_outputs = changed_file_transitions(&prepared.output.before, after);
            proprietary_outputs = proprietary_origins(
                &changed_outputs,
                &consent.operation_id,
                operation.kind(),
                false,
            );
            if failure_code.is_none()
                && !has_required_proprietary_output(operation.kind(), &proprietary_outputs)
            {
                failure_code = Some("REQUIRED_PROPRIETARY_OUTPUT_NOT_CREATED".into());
            }
            if failure_code.is_none()
                && proprietary_outputs
                    .iter()
                    .any(|output| output.extension != required_extension(operation.kind()))
            {
                failure_code = Some("UNEXPECTED_PROPRIETARY_OUTPUT_CREATED".into());
            }
        }

        let post_probe = if failure_code.is_none() {
            Some(run_version_probe(&executable, &self.limits))
        } else {
            None
        };
        if post_probe
            .as_ref()
            .is_some_and(|probe| probe.state != CliExecutionState::Succeeded)
        {
            failure_code = Some("POST_OPERATION_PATCH_NOT_CONFIRMED".into());
        }
        let succeeded = failure_code.is_none();
        for origin in &mut proprietary_outputs {
            origin.accepted_for_release = succeeded;
        }
        let input_transitions = post_inputs
            .unwrap_or_default()
            .into_iter()
            .zip(&prepared.inputs)
            .map(|(after, before)| ArtifactTransition {
                role: before.role.clone(),
                path_token: before.path_token.clone(),
                existed_before: before.before.exists,
                existed_after: after.exists,
                sha256_before: before.before.root_sha256.clone(),
                sha256_after: after.root_sha256,
                byte_length_before: before.before.byte_length,
                byte_length_after: after.byte_length,
            })
            .collect();
        let output_transition = post_output.map(|after| ArtifactTransition {
            role: prepared.output.role.clone(),
            path_token: prepared.output.path_token.clone(),
            existed_before: prepared.output.before.exists,
            existed_after: after.exists,
            sha256_before: prepared.output.before.root_sha256.clone(),
            sha256_after: after.root_sha256,
            byte_length_before: prepared.output.before.byte_length,
            byte_length_after: after.byte_length,
        });
        let provenance = CliOperationProvenance {
            schema_version: RECEIPT_SCHEMA.into(),
            evidence_class: ExternalEvidenceClass::External,
            execution_state: if succeeded {
                CliExecutionState::Succeeded
            } else {
                CliExecutionState::Failed
            },
            operation_id: consent.operation_id,
            confirmation_id: consent.confirmation_id,
            operation_kind: operation.kind(),
            command_summary: operation.safe_command_summary(),
            human_actor_id: consent.actor_id,
            consent_binding_sha256: consent.binding_sha256,
            native_attestation_sha256: consent.native_attestation_sha256,
            executable_path_token: executable.path_token(),
            executable_sha256_before: executable_hash_before,
            executable_sha256_after: executable_hash_after.ok(),
            observed_patch_before: pre_probe.observed_patch.clone(),
            observed_patch_after: post_probe
                .as_ref()
                .and_then(|probe| probe.observed_patch.clone()),
            pre_operation_probe: pre_probe.clone(),
            post_operation_probe: post_probe.clone(),
            started_at_unix_ms,
            finished_at_unix_ms,
            exit_code: process.exit_code,
            timed_out: process.timed_out,
            stdout: process.stdout.digest(),
            stderr: process.stderr.digest(),
            input_artifacts: input_transitions,
            output_artifact: output_transition,
            changed_outputs,
            proprietary_outputs,
            failure_code: failure_code.clone(),
        };
        SpineCliOperationReport {
            evidence_class: ExternalEvidenceClass::External,
            state: provenance.execution_state,
            failure_code,
            probe: Some(pre_probe),
            provenance: Some(provenance),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliSelectionAssessment {
    pub evidence_class: ExternalEvidenceClass,
    pub state: CliExecutionState,
    pub executable_path_token: Option<String>,
    pub executable_sha256: Option<String>,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpineCliProbeReport {
    pub evidence_class: ExternalEvidenceClass,
    pub state: CliExecutionState,
    pub observed_patch: Option<String>,
    pub executable_sha256: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub stdout: CaptureDigest,
    pub stderr: CaptureDigest,
    pub failure_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpineCliOperationReport {
    pub evidence_class: ExternalEvidenceClass,
    pub state: CliExecutionState,
    pub failure_code: Option<String>,
    pub probe: Option<SpineCliProbeReport>,
    pub provenance: Option<CliOperationProvenance>,
}

impl SpineCliOperationReport {
    fn not_run(reason: &str) -> Self {
        Self {
            evidence_class: ExternalEvidenceClass::External,
            state: CliExecutionState::NotRun,
            failure_code: Some(reason.into()),
            probe: None,
            provenance: None,
        }
    }

    /// This is the release-authority check for `.atlas`, `.spine`, and `.skel` artifacts. Callers
    /// must provide the relative output path and the hash of the bytes they are about to publish.
    pub fn authorizes_proprietary_output(&self, relative_path: &str, sha256: &str) -> bool {
        self.state == CliExecutionState::Succeeded
            && validate_sha256("output sha256", sha256).is_ok()
            && self.provenance.as_ref().is_some_and(|provenance| {
                provenance.execution_state == CliExecutionState::Succeeded
                    && provenance.pre_operation_probe.state == CliExecutionState::Succeeded
                    && provenance
                        .post_operation_probe
                        .as_ref()
                        .is_some_and(|probe| probe.state == CliExecutionState::Succeeded)
                    && provenance.observed_patch_before.as_deref() == Some(REQUIRED_SPINE_PATCH)
                    && provenance.observed_patch_after.as_deref() == Some(REQUIRED_SPINE_PATCH)
                    && provenance.proprietary_outputs.iter().any(|origin| {
                        origin.accepted_for_release
                            && origin.relative_path == relative_path
                            && origin.sha256_after == sha256
                            && origin.observed_patch == REQUIRED_SPINE_PATCH
                            && origin.producing_operation_id == provenance.operation_id
                    })
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureDigest {
    pub sha256: String,
    pub captured_byte_length: u64,
    pub observed_byte_length: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactTransition {
    pub role: String,
    pub path_token: String,
    pub existed_before: bool,
    pub existed_after: bool,
    pub sha256_before: Option<String>,
    pub sha256_after: Option<String>,
    pub byte_length_before: u64,
    pub byte_length_after: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedOutputArtifact {
    /// Relative to the operation's dedicated output root; no absolute user path is recorded.
    pub relative_path: String,
    pub extension: String,
    pub sha256_before: Option<String>,
    pub sha256_after: Option<String>,
    pub byte_length_before: u64,
    pub byte_length_after: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProprietaryOutputOrigin {
    pub relative_path: String,
    pub extension: String,
    pub producing_operation_id: String,
    pub operation_kind: SpineCliOperationKind,
    pub observed_patch: String,
    pub sha256_before: Option<String>,
    pub sha256_after: String,
    pub accepted_for_release: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliOperationProvenance {
    pub schema_version: String,
    pub evidence_class: ExternalEvidenceClass,
    pub execution_state: CliExecutionState,
    pub operation_id: String,
    pub confirmation_id: String,
    pub operation_kind: SpineCliOperationKind,
    pub command_summary: CliCommandSummary,
    pub human_actor_id: String,
    pub consent_binding_sha256: String,
    pub native_attestation_sha256: String,
    pub executable_path_token: String,
    pub executable_sha256_before: String,
    pub executable_sha256_after: Option<String>,
    pub observed_patch_before: Option<String>,
    pub observed_patch_after: Option<String>,
    pub pre_operation_probe: SpineCliProbeReport,
    pub post_operation_probe: Option<SpineCliProbeReport>,
    pub started_at_unix_ms: u64,
    pub finished_at_unix_ms: u64,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub stdout: CaptureDigest,
    pub stderr: CaptureDigest,
    pub input_artifacts: Vec<ArtifactTransition>,
    pub output_artifact: Option<ArtifactTransition>,
    pub changed_outputs: Vec<ChangedOutputArtifact>,
    pub proprietary_outputs: Vec<ProprietaryOutputOrigin>,
    pub failure_code: Option<String>,
}

#[derive(Debug)]
struct PreparedOperation {
    argv: Vec<String>,
    inputs: Vec<PreparedArtifact>,
    output: PreparedArtifact,
}

#[derive(Debug)]
struct PreparedArtifact {
    role: String,
    path: PathBuf,
    path_token: String,
    before: PathSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathSnapshot {
    exists: bool,
    root_sha256: Option<String>,
    byte_length: u64,
    files: BTreeMap<String, FileSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileSnapshot {
    sha256: String,
    byte_length: u64,
}

fn prepare_operation(
    operation: &SpineCliOperation,
    limits: &SpineCliRunnerLimits,
) -> Result<PreparedOperation, String> {
    match operation {
        SpineCliOperation::ImportProject {
            source_json,
            output_project,
        } => {
            let input = canonical_existing_file(source_json, Some("json"))?;
            let output = canonical_new_file(output_project, "spine")?;
            reject_overlapping_paths(&input, &output)?;
            let argv = vec![
                "--input".into(),
                input.to_string_lossy().into_owned(),
                "--output".into(),
                output.to_string_lossy().into_owned(),
                "--import".into(),
            ];
            prepared(
                operation,
                argv,
                vec![("sourceJson", input)],
                ("outputProject", output),
                limits,
            )
        }
        SpineCliOperation::ExportBinary {
            source_project,
            output_directory,
        } => {
            let input = canonical_existing_file(source_project, Some("spine"))?;
            let output = canonical_empty_directory(output_directory)?;
            reject_overlapping_paths(&input, &output)?;
            let argv = vec![
                "--input".into(),
                input.to_string_lossy().into_owned(),
                "--output".into(),
                output.to_string_lossy().into_owned(),
                "--export".into(),
                "binary".into(),
            ];
            prepared(
                operation,
                argv,
                vec![("sourceProject", input)],
                ("outputDirectory", output),
                limits,
            )
        }
        SpineCliOperation::PackAtlas {
            input_directory,
            output_directory,
            pack_settings_json,
        } => {
            let input = canonical_existing_directory(input_directory)?;
            let settings = canonical_existing_file(pack_settings_json, Some("json"))?;
            let output = canonical_empty_directory(output_directory)?;
            reject_overlapping_paths(&input, &output)?;
            reject_overlapping_paths(&settings, &output)?;
            let argv = vec![
                "--input".into(),
                input.to_string_lossy().into_owned(),
                "--output".into(),
                output.to_string_lossy().into_owned(),
                "--pack".into(),
                settings.to_string_lossy().into_owned(),
            ];
            prepared(
                operation,
                argv,
                vec![("pngDirectory", input), ("packSettingsJson", settings)],
                ("outputDirectory", output),
                limits,
            )
        }
    }
}

fn prepared(
    _operation: &SpineCliOperation,
    argv: Vec<String>,
    inputs: Vec<(&str, PathBuf)>,
    output: (&str, PathBuf),
    limits: &SpineCliRunnerLimits,
) -> Result<PreparedOperation, String> {
    let inputs = inputs
        .into_iter()
        .map(|(role, path)| {
            let before = snapshot_path(&path, limits, false)?;
            Ok(PreparedArtifact {
                role: role.into(),
                path_token: path_token(&path),
                path,
                before,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let output_before = snapshot_path(&output.1, limits, true)?;
    Ok(PreparedOperation {
        argv,
        inputs,
        output: PreparedArtifact {
            role: output.0.into(),
            path_token: path_token(&output.1),
            path: output.1,
            before: output_before,
        },
    })
}

fn build_consent_binding(
    executable: &ValidatedSpineCli,
    operation: &SpineCliOperation,
    prepared: &PreparedOperation,
    operation_id: &str,
) -> CliConsentBinding {
    let mut canonical = String::from("F2S-SPINE-CLI-CONSENT-V1\n");
    canonical.push_str(REQUIRED_SPINE_PATCH);
    canonical.push('\n');
    canonical.push_str(executable.executable_sha256());
    canonical.push('\n');
    canonical.push_str(&executable.path_token());
    canonical.push('\n');
    canonical.push_str(&format!("{:?}\n", operation.kind()));
    canonical.push_str(operation_id);
    canonical.push('\n');
    for input in &prepared.inputs {
        canonical.push_str(&input.role);
        canonical.push(':');
        canonical.push_str(&input.path_token);
        canonical.push(':');
        canonical.push_str(input.before.root_sha256.as_deref().unwrap_or("MISSING"));
        canonical.push('\n');
    }
    canonical.push_str(&prepared.output.role);
    canonical.push(':');
    canonical.push_str(&prepared.output.path_token);
    canonical.push(':');
    canonical.push_str(
        prepared
            .output
            .before
            .root_sha256
            .as_deref()
            .unwrap_or("MISSING"),
    );
    canonical.push('\n');
    CliConsentBinding {
        binding_sha256: sha256_bytes(canonical.as_bytes()),
        executable_sha256: executable.executable_sha256().into(),
        operation_id: operation_id.into(),
        operation_kind: operation.kind(),
        command_summary: operation.safe_command_summary(),
    }
}

fn canonical_existing_file(path: &Path, extension: Option<&str>) -> Result<PathBuf, String> {
    let canonical = canonical_existing(path)?;
    if !fs::metadata(&canonical)
        .map_err(|error| format!("cannot inspect input file: {error}"))?
        .is_file()
    {
        return Err("operation input must be a regular file".into());
    }
    if let Some(expected) = extension {
        require_extension(&canonical, expected)?;
    }
    Ok(canonical)
}

fn canonical_existing_directory(path: &Path) -> Result<PathBuf, String> {
    let canonical = canonical_existing(path)?;
    if !fs::metadata(&canonical)
        .map_err(|error| format!("cannot inspect input directory: {error}"))?
        .is_dir()
    {
        return Err("operation input must be a directory".into());
    }
    Ok(canonical)
}

fn canonical_existing(path: &Path) -> Result<PathBuf, String> {
    validate_local_absolute_path_shape(path)?;
    reject_reparse_components(path)?;
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("operation path is unavailable: {error}"))?;
    reject_reparse_components(&canonical)?;
    if path_token(path) != path_token(&canonical) {
        return Err("operation path is not canonical".into());
    }
    Ok(canonical)
}

fn canonical_empty_directory(path: &Path) -> Result<PathBuf, String> {
    let canonical = canonical_existing_directory(path)?;
    if fs::read_dir(&canonical)
        .map_err(|error| format!("cannot inspect output directory: {error}"))?
        .next()
        .is_some()
    {
        return Err("Spine CLI output directory must be newly created and empty".into());
    }
    Ok(canonical)
}

fn canonical_new_file(path: &Path, expected_extension: &str) -> Result<PathBuf, String> {
    validate_local_absolute_path_shape(path)?;
    require_extension(path, expected_extension)?;
    if path.exists() {
        return Err("Spine CLI output file must not already exist".into());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "output file has no parent directory".to_owned())?;
    let canonical_parent = canonical_existing_directory(parent)?;
    let file_name = path
        .file_name()
        .ok_or_else(|| "output file name is missing".to_owned())?;
    Ok(canonical_parent.join(file_name))
}

fn reject_overlapping_paths(input: &Path, output: &Path) -> Result<(), String> {
    if input == output || input.starts_with(output) || output.starts_with(input) {
        return Err("Spine CLI input and output paths must be disjoint".into());
    }
    Ok(())
}

fn require_extension(path: &Path, expected: &str) -> Result<(), String> {
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case(expected))
    {
        return Err(format!("operation path must have .{expected} extension"));
    }
    Ok(())
}

fn snapshot_inputs(
    inputs: &[PreparedArtifact],
    limits: &SpineCliRunnerLimits,
) -> Result<Vec<PathSnapshot>, String> {
    inputs
        .iter()
        .map(|input| snapshot_path(&input.path, limits, false))
        .collect()
}

fn input_snapshots_equal(before: &[PreparedArtifact], after: &[PathSnapshot]) -> bool {
    before.len() == after.len()
        && before
            .iter()
            .zip(after)
            .all(|(left, right)| left.before == *right)
}

fn snapshot_path(
    path: &Path,
    limits: &SpineCliRunnerLimits,
    allow_missing: bool,
) -> Result<PathSnapshot, String> {
    if !path.exists() {
        return if allow_missing {
            Ok(PathSnapshot {
                exists: false,
                root_sha256: None,
                byte_length: 0,
                files: BTreeMap::new(),
            })
        } else {
            Err("required operation artifact disappeared".into())
        };
    }
    reject_reparse_components(path)?;
    let metadata =
        fs::metadata(path).map_err(|error| format!("cannot inspect artifact: {error}"))?;
    let mut files = BTreeMap::new();
    if metadata.is_file() {
        let digest = sha256_file(path)?;
        files.insert(
            path.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("artifact")
                .to_owned(),
            FileSnapshot {
                sha256: digest,
                byte_length: metadata.len(),
            },
        );
    } else if metadata.is_dir() {
        collect_files(path, path, &mut files, limits)?;
    } else {
        return Err("operation artifact is not a regular file or directory".into());
    }
    let byte_length = files.values().try_fold(0_u64, |total, file| {
        total
            .checked_add(file.byte_length)
            .ok_or_else(|| "artifact byte length overflow".to_owned())
    })?;
    if byte_length > limits.snapshot_byte_limit {
        return Err("operation artifact exceeds snapshot byte limit".into());
    }
    let mut hasher = Sha256::new();
    hasher.update(if metadata.is_file() {
        b"F2S-FILE-SNAPSHOT-V1\n".as_slice()
    } else {
        b"F2S-DIRECTORY-SNAPSHOT-V1\n".as_slice()
    });
    for (relative, file) in &files {
        hasher.update(relative.as_bytes());
        hasher.update(b"\0");
        hasher.update(file.sha256.as_bytes());
        hasher.update(b"\0");
        hasher.update(file.byte_length.to_string().as_bytes());
        hasher.update(b"\n");
    }
    Ok(PathSnapshot {
        exists: true,
        root_sha256: Some(format!("{:x}", hasher.finalize())),
        byte_length,
        files,
    })
}

fn collect_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeMap<String, FileSnapshot>,
    limits: &SpineCliRunnerLimits,
) -> Result<(), String> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| format!("cannot enumerate operation artifact: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot enumerate operation artifact: {error}"))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        reject_reparse_components(&path)?;
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect operation artifact entry: {error}"))?;
        if metadata.is_dir() {
            collect_files(root, &path, files, limits)?;
        } else if metadata.is_file() {
            if files.len() >= limits.snapshot_file_limit {
                return Err("operation artifact exceeds snapshot file limit".into());
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "cannot make operation artifact path relative".to_owned())?
                .to_string_lossy()
                .replace('\\', "/");
            files.insert(
                relative,
                FileSnapshot {
                    sha256: sha256_file(&path)?,
                    byte_length: metadata.len(),
                },
            );
        } else {
            return Err("non-file entry is not accepted in a CLI operation tree".into());
        }
    }
    Ok(())
}

fn changed_file_transitions(
    before: &PathSnapshot,
    after: &PathSnapshot,
) -> Vec<ChangedOutputArtifact> {
    let paths = before
        .files
        .keys()
        .chain(after.files.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    paths
        .into_iter()
        .filter_map(|relative_path| {
            let old = before.files.get(&relative_path);
            let new = after.files.get(&relative_path);
            (old != new).then(|| ChangedOutputArtifact {
                extension: Path::new(&relative_path)
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase(),
                relative_path,
                sha256_before: old.map(|file| file.sha256.clone()),
                sha256_after: new.map(|file| file.sha256.clone()),
                byte_length_before: old.map(|file| file.byte_length).unwrap_or_default(),
                byte_length_after: new.map(|file| file.byte_length).unwrap_or_default(),
            })
        })
        .collect()
}

fn proprietary_origins(
    outputs: &[ChangedOutputArtifact],
    operation_id: &str,
    operation_kind: SpineCliOperationKind,
    accepted_for_release: bool,
) -> Vec<ProprietaryOutputOrigin> {
    outputs
        .iter()
        .filter(|output| ["atlas", "spine", "skel"].contains(&output.extension.as_str()))
        .filter_map(|output| {
            output
                .sha256_after
                .as_ref()
                .map(|sha256_after| ProprietaryOutputOrigin {
                    relative_path: output.relative_path.clone(),
                    extension: output.extension.clone(),
                    producing_operation_id: operation_id.into(),
                    operation_kind,
                    observed_patch: REQUIRED_SPINE_PATCH.into(),
                    sha256_before: output.sha256_before.clone(),
                    sha256_after: sha256_after.clone(),
                    accepted_for_release,
                })
        })
        .collect()
}

fn has_required_proprietary_output(
    kind: SpineCliOperationKind,
    outputs: &[ProprietaryOutputOrigin],
) -> bool {
    let extension = required_extension(kind);
    outputs.iter().any(|output| output.extension == extension)
}

fn required_extension(kind: SpineCliOperationKind) -> &'static str {
    match kind {
        SpineCliOperationKind::ImportProject => "spine",
        SpineCliOperationKind::ExportBinary => "skel",
        SpineCliOperationKind::PackAtlas => "atlas",
    }
}

fn run_version_probe(
    executable: &ValidatedSpineCli,
    limits: &SpineCliRunnerLimits,
) -> SpineCliProbeReport {
    let process = run_process(
        executable.canonical_executable(),
        &["--version".into()],
        limits.probe_timeout,
        limits.stdout_limit_bytes,
        limits.stderr_limit_bytes,
    );
    let mut failure_code = process_failure_code(&process);
    let versions = observed_editor_versions(&process.stdout.bytes, &process.stderr.bytes);
    let observed_patch = if versions.len() == 1 {
        versions.iter().next().cloned()
    } else {
        None
    };
    if failure_code.is_none() {
        if versions.is_empty() {
            failure_code = Some("EDITOR_PATCH_NOT_OBSERVED".into());
        } else if versions.len() != 1 || observed_patch.as_deref() != Some(REQUIRED_SPINE_PATCH) {
            failure_code = Some("EDITOR_PATCH_MISMATCH".into());
        }
    }
    SpineCliProbeReport {
        evidence_class: ExternalEvidenceClass::External,
        state: if failure_code.is_none() {
            CliExecutionState::Succeeded
        } else {
            CliExecutionState::Failed
        },
        observed_patch,
        executable_sha256: executable.executable_sha256().into(),
        exit_code: process.exit_code,
        timed_out: process.timed_out,
        stdout: process.stdout.digest(),
        stderr: process.stderr.digest(),
        failure_code,
    }
}

fn observed_editor_versions(stdout: &[u8], stderr: &[u8]) -> BTreeSet<String> {
    let mut values = BTreeSet::new();
    for text in [stdout, stderr].map(String::from_utf8_lossy) {
        for line in text.lines().map(str::trim) {
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("spine launcher") {
                continue;
            }
            let tail = if lower.starts_with("spine editor ") {
                line.get("spine editor ".len()..)
            } else if lower.starts_with("spine: ") {
                line.get("spine: ".len()..)
            } else if lower.starts_with("spine ") {
                line.get("spine ".len()..)
            } else {
                None
            };
            let Some(tail) = tail else { continue };
            if let Some(token) = tail.split_whitespace().next() {
                let candidate =
                    token.trim_matches(|value: char| !value.is_ascii_digit() && value != '.');
                if is_patch(candidate) {
                    values.insert(candidate.into());
                }
            }
        }
    }
    values
}

fn is_patch(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|value| value.is_ascii_digit()))
}

#[derive(Debug)]
struct ProcessObservation {
    exit_code: Option<i32>,
    timed_out: bool,
    spawn_failed: bool,
    stdout: BoundedCapture,
    stderr: BoundedCapture,
}

#[derive(Debug)]
struct BoundedCapture {
    bytes: Vec<u8>,
    observed_byte_length: u64,
    truncated: bool,
    read_failed: bool,
}

impl BoundedCapture {
    fn digest(&self) -> CaptureDigest {
        CaptureDigest {
            sha256: sha256_bytes(&self.bytes),
            captured_byte_length: self.bytes.len() as u64,
            observed_byte_length: self.observed_byte_length,
            truncated: self.truncated,
        }
    }
}

fn run_process(
    executable: &Path,
    argv: &[String],
    timeout: Duration,
    stdout_limit: usize,
    stderr_limit: usize,
) -> ProcessObservation {
    let mut command = Command::new(executable);
    command
        .args(argv)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(parent) = executable.parent() {
        command.current_dir(parent);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = match command.spawn() {
        Ok(value) => value,
        Err(_) => {
            return ProcessObservation {
                exit_code: None,
                timed_out: false,
                spawn_failed: true,
                stdout: empty_capture(),
                stderr: empty_capture(),
            };
        }
    };
    let overflow = Arc::new(AtomicBool::new(false));
    let stdout = child
        .stdout
        .take()
        .map(|pipe| spawn_capture(pipe, stdout_limit, Arc::clone(&overflow)));
    let stderr = child
        .stderr
        .take()
        .map(|pipe| spawn_capture(pipe, stderr_limit, Arc::clone(&overflow)));
    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if overflow.load(Ordering::Relaxed) {
            terminate_process_tree(&mut child);
            break child.wait().ok();
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            terminate_process_tree(&mut child);
            break child.wait().ok();
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(_) => {
                terminate_process_tree(&mut child);
                break child.wait().ok();
            }
        }
    };
    ProcessObservation {
        exit_code: status.and_then(exit_code),
        timed_out,
        spawn_failed: false,
        stdout: join_capture(stdout),
        stderr: join_capture(stderr),
    }
}

fn spawn_capture<R: Read + Send + 'static>(
    mut reader: R,
    limit: usize,
    overflow: Arc<AtomicBool>,
) -> Receiver<BoundedCapture> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::with_capacity(limit.min(64 * 1024));
        let mut observed = 0_u64;
        let mut buffer = [0_u8; 8 * 1024];
        let mut read_failed = false;
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    observed = observed.saturating_add(count as u64);
                    let remaining = limit.saturating_sub(bytes.len());
                    bytes.extend_from_slice(&buffer[..count.min(remaining)]);
                    if count > remaining {
                        overflow.store(true, Ordering::Relaxed);
                    }
                }
                Err(_) => {
                    read_failed = true;
                    break;
                }
            }
        }
        let _ = sender.send(BoundedCapture {
            truncated: observed > bytes.len() as u64,
            bytes,
            observed_byte_length: observed,
            read_failed,
        });
    });
    receiver
}

fn join_capture(receiver: Option<Receiver<BoundedCapture>>) -> BoundedCapture {
    receiver
        .and_then(|value| value.recv_timeout(Duration::from_secs(2)).ok())
        .unwrap_or_else(|| BoundedCapture {
            bytes: Vec::new(),
            observed_byte_length: 0,
            truncated: true,
            read_failed: true,
        })
}

fn empty_capture() -> BoundedCapture {
    BoundedCapture {
        bytes: Vec::new(),
        observed_byte_length: 0,
        truncated: false,
        read_failed: false,
    }
}

fn exit_code(status: ExitStatus) -> Option<i32> {
    status.code()
}

fn lock_executable(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 0x0000_0001;
        options.share_mode(FILE_SHARE_READ);
    }
    options
        .open(path)
        .map_err(|error| format!("cannot lock selected Spine.com: {error}"))
}

fn terminate_process_tree(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let system_root = std::env::var_os("SystemRoot")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        let taskkill = system_root.join("System32").join("taskkill.exe");
        if let Ok(taskkill) = canonical_existing_file(&taskkill, Some("exe")) {
            let pid = child.id().to_string();
            if let Ok(mut terminator) = Command::new(taskkill)
                .args(["/PID", &pid, "/T", "/F"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
            {
                let deadline = Instant::now() + Duration::from_secs(2);
                loop {
                    match terminator.try_wait() {
                        Ok(Some(_)) | Err(_) => break,
                        Ok(None) if Instant::now() < deadline => {
                            thread::sleep(Duration::from_millis(10));
                        }
                        Ok(None) => {
                            let _ = terminator.kill();
                            let _ = terminator.wait();
                            break;
                        }
                    }
                }
            }
        }
    }
    let _ = child.kill();
}

fn process_failure_code(process: &ProcessObservation) -> Option<String> {
    if process.spawn_failed {
        Some("PROCESS_SPAWN_FAILED".into())
    } else if process.timed_out {
        Some("PROCESS_TIMEOUT".into())
    } else if process.stdout.truncated || process.stderr.truncated {
        Some("PROCESS_OUTPUT_LIMIT_EXCEEDED".into())
    } else if process.stdout.read_failed || process.stderr.read_failed {
        Some("PROCESS_OUTPUT_READ_FAILED".into())
    } else if process.exit_code != Some(0) {
        Some("PROCESS_NON_ZERO_EXIT".into())
    } else {
        None
    }
}

fn path_token(path: &Path) -> String {
    let value = if cfg!(windows) {
        let path = path.to_string_lossy();
        path.strip_prefix(r"\\?\")
            .unwrap_or(&path)
            .replace('/', "\\")
            .to_ascii_lowercase()
    } else {
        path.to_string_lossy().into_owned()
    };
    sha256_bytes(value.as_bytes())
}

fn validate_identifier(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
    {
        return Err(format!("invalid {label}"));
    }
    Ok(())
}

fn validate_sha256(label: &str, value: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(format!("invalid {label}"));
    }
    Ok(())
}

fn unix_ms() -> Result<u64, String> {
    let value = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before Unix epoch".to_owned())?
        .as_millis();
    u64::try_from(value).map_err(|_| "system time is out of range".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parser_rejects_launcher_and_ambient_mentions() {
        let values =
            observed_editor_versions(b"Spine Launcher 4.2.43\nwarning: use Spine 4.2.43\n", b"");
        assert!(values.is_empty());
        let values = observed_editor_versions(b"Spine 4.2.43 Professional\n", b"");
        assert_eq!(values.into_iter().collect::<Vec<_>>(), ["4.2.43"]);
    }

    #[test]
    fn summaries_never_contain_local_operands() {
        let operation = SpineCliOperation::ImportProject {
            source_json: PathBuf::from(r"C:\private\hero.json"),
            output_project: PathBuf::from(r"C:\private\hero.spine"),
        };
        let text = serde_json::to_string(&operation.safe_command_summary()).unwrap();
        assert!(!text.contains("private"));
        assert!(!text.contains("hero"));
        assert!(text.contains("SOURCE_JSON"));
        assert!(text.contains("\"shellUsed\":false"));
    }
}

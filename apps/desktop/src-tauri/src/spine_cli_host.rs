use f2s_adapters::{
    export::{
        cli_policy::{REQUIRED_SPINE_PATCH, SpineCliPolicy},
        cli_runner::{
            CliExecutionState, SpineCliOperation, SpineCliOperationConsent, SpineCliRunner,
            SpineCliRunnerLimits, VerifiedHumanCliConfirmation,
        },
    },
    storage::ntfs_atomic::write_atomic,
};
use f2s_application::approvals::{HumanCredentialVerifier, VerifiedHumanActor};
use f2s_domain::governance::CredentialAttestation;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    ffi::c_void,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;
use windows::{
    Win32::{
        Foundation::{ERROR_CANCELLED, HWND},
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
            CoTaskMemFree, CoUninitialize,
        },
        UI::{
            Controls::Dialogs::{
                CommDlgExtendedError, GetOpenFileNameW, OFN_DONTADDTORECENT, OFN_EXPLORER,
                OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
            },
            Shell::{
                FOS_FORCEFILESYSTEM, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS, FileOpenDialog,
                IFileOpenDialog, SIGDN_FILESYSPATH,
            },
            WindowsAndMessaging::{IDYES, MB_ICONQUESTION, MB_YESNO, MessageBoxW},
        },
    },
    core::{HRESULT, PCWSTR, PWSTR},
};

const CONFIG_SCHEMA: &str = "f2s-spine-cli-config/1.0";
const JOB_SCHEMA: &str = "f2s-spine-cli-job/1.0";
const CONFIG_MAX_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone)]
pub struct OpenExportGrant {
    pub export_id: String,
    pub project_id: String,
    pub project_revision: u64,
    pub snapshot_sha256: String,
    pub directory: PathBuf,
    pub checksums: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SpineCliConfigPayload {
    schema_version: String,
    canonical_executable: PathBuf,
    executable_sha256: String,
    executable_path_token: String,
    expected_patch: String,
    professional_license_confirmed: bool,
    confirmed_actor_id: String,
    confirmed_at_unix_ms: u64,
    confirmation_proof_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SignedSpineCliConfig {
    payload: SpineCliConfigPayload,
    integrity_key_id: String,
    integrity_hmac_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CliJobOutput {
    relative_path: String,
    extension: String,
    sha256: String,
    authorized: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CliJobProjection {
    schema_version: String,
    job_id: String,
    operation_id: String,
    operation_kind: String,
    export_id: String,
    evidence_class: String,
    state: String,
    failure_code: Option<String>,
    output_path_token: Option<String>,
    outputs: Vec<CliJobOutput>,
    provenance_sha256: Option<String>,
    created_at_unix_ms: u64,
    finished_at_unix_ms: Option<u64>,
}

impl CliJobProjection {
    fn pending(job_id: String, operation_id: String, kind: String, export_id: String) -> Self {
        Self {
            schema_version: JOB_SCHEMA.into(),
            job_id,
            operation_id,
            operation_kind: kind,
            export_id,
            evidence_class: "EXTERNAL".into(),
            state: "QUEUED".into(),
            failure_code: None,
            output_path_token: None,
            outputs: Vec::new(),
            provenance_sha256: None,
            created_at_unix_ms: unix_ms().unwrap_or_default(),
            finished_at_unix_ms: None,
        }
    }
}

#[derive(Clone)]
pub struct SpineCliHost {
    app_data_root: PathBuf,
    runner: Arc<SpineCliRunner>,
    jobs: Arc<Mutex<HashMap<String, CliJobProjection>>>,
    open_exports: Arc<Mutex<HashMap<String, OpenExportGrant>>>,
    consumed_attestations: Arc<Mutex<HashSet<String>>>,
    session_nonce: Arc<String>,
}

impl Default for SpineCliHost {
    fn default() -> Self {
        let app_data_root = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("FlashToSpine");
        Self {
            app_data_root,
            runner: Arc::new(
                SpineCliRunner::new(SpineCliRunnerLimits::default())
                    .expect("default Spine CLI runner limits must be valid"),
            ),
            jobs: Arc::new(Mutex::new(HashMap::new())),
            open_exports: Arc::new(Mutex::new(HashMap::new())),
            consumed_attestations: Arc::new(Mutex::new(HashSet::new())),
            session_nonce: Arc::new(Uuid::new_v4().simple().to_string()),
        }
    }
}

impl SpineCliHost {
    pub fn register_open_export(&self, grant: OpenExportGrant) -> Result<(), String> {
        validate_identifier("export id", &grant.export_id)?;
        validate_sha256("snapshot sha256", &grant.snapshot_sha256)?;
        let canonical = grant
            .directory
            .canonicalize()
            .map_err(|_| "open export directory is unavailable".to_owned())?;
        if !canonical.is_dir() {
            return Err("open export directory is not a directory".into());
        }
        let mut grants = self
            .open_exports
            .lock()
            .map_err(|_| "Spine CLI export grant registry is unavailable")?;
        if grants.len() >= 32 {
            grants.clear();
        }
        grants.insert(
            grant.export_id.clone(),
            OpenExportGrant {
                directory: canonical,
                ..grant
            },
        );
        Ok(())
    }

    pub fn status(&self) -> Value {
        let (configured, assessment) = match self.load_config_and_policy() {
            Ok(Some((config, policy))) => {
                let assessment = self.runner.assess_selection(&policy);
                (
                    true,
                    json!({
                        "evidenceClass": assessment.evidence_class,
                        "state": assessment.state,
                        "reasonCode": assessment.reason_code,
                        "pathToken": config.executable_path_token,
                        "executableSha256": config.executable_sha256,
                        "expectedPatch": config.expected_patch,
                        "professionalLicenseConfirmed": config.professional_license_confirmed,
                        "confirmedAtUnixMs": config.confirmed_at_unix_ms,
                        "observedPatch": Value::Null,
                        "realCliTested": false
                    }),
                )
            }
            Ok(None) => (
                false,
                json!({
                    "evidenceClass":"EXTERNAL",
                    "state":"NOT_RUN",
                    "reasonCode":"EXTERNAL_CLI_NOT_SELECTED",
                    "pathToken":Value::Null,
                    "executableSha256":Value::Null,
                    "expectedPatch":REQUIRED_SPINE_PATCH,
                    "professionalLicenseConfirmed":false,
                    "confirmedAtUnixMs":Value::Null,
                    "observedPatch":Value::Null,
                    "realCliTested":false
                }),
            ),
            Err(_) => (
                false,
                json!({
                    "evidenceClass":"EXTERNAL",
                    "state":"NOT_RUN",
                    "reasonCode":"LOCAL_CLI_CONFIG_INVALID",
                    "pathToken":Value::Null,
                    "executableSha256":Value::Null,
                    "expectedPatch":REQUIRED_SPINE_PATCH,
                    "professionalLicenseConfirmed":false,
                    "confirmedAtUnixMs":Value::Null,
                    "observedPatch":Value::Null,
                    "realCliTested":false
                }),
            ),
        };
        let open_exports = self
            .open_exports
            .lock()
            .map(|values| {
                values
                    .values()
                    .map(|grant| {
                        json!({
                            "exportId":grant.export_id,
                            "projectId":grant.project_id,
                            "projectRevision":grant.project_revision,
                            "snapshotSha256":grant.snapshot_sha256
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        json!({
            "schemaVersion":"1.0.0",
            "configured":configured,
            "assessment":assessment,
            "openExports":open_exports,
            "policy":{
                "bundled":false,
                "downloadedByApp":false,
                "activationDataRead":false,
                "networkGranted":false,
                "absolutePathReturnedToWebView":false
            }
        })
    }

    pub fn select_and_assess(&self, hwnd: HWND) -> Result<Value, String> {
        let Some(path) = choose_file(
            hwnd,
            "Spine command line (Spine.com)\0Spine.com\0\0",
            "Select your locally installed Spine.com",
        )?
        else {
            return Ok(json!({"cancelled":true,"status":self.status()}));
        };
        let policy = SpineCliPolicy {
            executable: path,
            user_confirmed_professional_license: true,
            network_granted_for_operation: false,
            expected_patch: REQUIRED_SPINE_PATCH.into(),
        };
        let validated = policy.validate_selected_executable()?;
        let path_token = validated.path_token();
        let payload_sha256 = sha256_text(&format!(
            "F2S-SPINE-PROFESSIONAL-LICENSE-V1:{REQUIRED_SPINE_PATCH}:{}:{path_token}",
            validated.executable_sha256()
        ));
        let actor = self.verified_human(
            hwnd,
            "confirm-spine-professional-license",
            &payload_sha256,
            "I confirm that this locally installed Spine.com is covered by my legal Spine Professional license. FlashToSpine will not inspect activation data or download Spine.",
        )?;
        let payload = SpineCliConfigPayload {
            schema_version: CONFIG_SCHEMA.into(),
            canonical_executable: validated.canonical_executable().to_path_buf(),
            executable_sha256: validated.executable_sha256().into(),
            executable_path_token: path_token,
            expected_patch: REQUIRED_SPINE_PATCH.into(),
            professional_license_confirmed: true,
            confirmed_actor_id: actor.actor_id().into(),
            confirmed_at_unix_ms: unix_ms()?,
            confirmation_proof_sha256: actor.proof_sha256().into(),
        };
        self.save_config(&payload)?;
        Ok(json!({"cancelled":false,"status":self.status()}))
    }

    pub fn clear_config(&self) -> Result<Value, String> {
        let path = self.config_path();
        if path.exists() {
            reject_reparse_ancestors(&path)?;
            fs::remove_file(path).map_err(|_| "cannot clear local Spine CLI config")?;
        }
        Ok(self.status())
    }

    pub fn start_job(
        &self,
        export_id: &str,
        operation_kind: &str,
        current_project_id: &str,
        current_project_revision: u64,
        hwnd: HWND,
    ) -> Result<Value, String> {
        validate_identifier("export id", export_id)?;
        let kind = match operation_kind {
            "IMPORT_PROJECT" | "PACK_ATLAS" | "EXPORT_BINARY" => operation_kind,
            _ => return Err("unsupported Spine CLI operation kind".into()),
        };
        let grant = self
            .open_exports
            .lock()
            .map_err(|_| "Spine CLI export grant registry is unavailable")?
            .get(export_id)
            .cloned()
            .ok_or("open export is not available in this application session")?;
        if grant.project_id != current_project_id
            || grant.project_revision != current_project_revision
        {
            return Err("open export belongs to another project revision".into());
        }
        let job_id = format!("cli-job-{}", Uuid::new_v4().simple());
        let operation_id = format!("cli-op-{}", Uuid::new_v4().simple());
        let projection =
            CliJobProjection::pending(job_id.clone(), operation_id, kind.into(), export_id.into());
        {
            let mut jobs = self
                .jobs
                .lock()
                .map_err(|_| "Spine CLI job registry is unavailable")?;
            if jobs.len() >= 64 {
                jobs.retain(|_, job| !is_terminal(&job.state));
            }
            if jobs.values().any(|job| !is_terminal(&job.state)) {
                return Err("another Spine CLI job is awaiting input or running".into());
            }
            if jobs.len() >= 64 {
                return Err("too many active Spine CLI jobs".into());
            }
            jobs.insert(job_id.clone(), projection.clone());
        }
        let host = self.clone();
        let hwnd_value = hwnd.0 as isize;
        thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                host.execute_job(&job_id, grant, hwnd_value)
            }));
            match result {
                Ok(Ok(())) => {}
                Ok(Err(code)) => host.finish_job(&job_id, "FAILED", Some(&code)),
                Err(_) => host.finish_job(&job_id, "FAILED", Some("CLI_JOB_PANIC")),
            }
        });
        serde_json::to_value(projection).map_err(|error| error.to_string())
    }

    pub fn job_status(&self, job_id: &str) -> Result<Value, String> {
        validate_identifier("job id", job_id)?;
        let job = self
            .jobs
            .lock()
            .map_err(|_| "Spine CLI job registry is unavailable")?
            .get(job_id)
            .cloned()
            .ok_or("Spine CLI job not found")?;
        serde_json::to_value(job).map_err(|error| error.to_string())
    }

    pub fn path_token(path: &Path) -> String {
        path_token(path)
    }

    fn execute_job(
        &self,
        job_id: &str,
        grant: OpenExportGrant,
        hwnd_value: isize,
    ) -> Result<(), String> {
        let _com = ComApartment::initialize()?;
        let hwnd = HWND(hwnd_value as *mut c_void);
        self.set_job_state(job_id, "AWAITING_NATIVE_INPUT", None);
        let Some((config, policy)) = self.load_config_and_policy()? else {
            self.finish_job(job_id, "NOT_RUN", Some("EXTERNAL_CLI_NOT_SELECTED"));
            return Ok(());
        };
        if !config.professional_license_confirmed {
            self.finish_job(
                job_id,
                "NOT_RUN",
                Some("PROFESSIONAL_LICENSE_NOT_CONFIRMED"),
            );
            return Ok(());
        }
        let (operation_id, kind) = {
            let jobs = self
                .jobs
                .lock()
                .map_err(|_| "CLI_JOB_REGISTRY_UNAVAILABLE".to_owned())?;
            let job = jobs.get(job_id).ok_or("CLI_JOB_NOT_FOUND")?;
            (job.operation_id.clone(), job.operation_kind.clone())
        };
        let operation = match self.choose_operation(hwnd, &grant, &operation_id, &kind)? {
            Some(value) => value,
            None => {
                self.finish_job(job_id, "NOT_RUN", Some("NATIVE_SELECTION_CANCELLED"));
                return Ok(());
            }
        };
        self.set_job_state(job_id, "PREPARING_CONSENT", None);
        let binding = match self
            .runner
            .prepare_consent_binding(&policy, &operation, &operation_id)
        {
            Ok(value) => value,
            Err(_) => {
                self.finish_job(job_id, "NOT_RUN", Some("CLI_PREPARE_REJECTED"));
                return Ok(());
            }
        };
        match kind.as_str() {
            "IMPORT_PROJECT" => self.verify_open_export_file(&grant, "character.spine.json")?,
            "PACK_ATLAS" => self.verify_open_export_images(&grant)?,
            "EXPORT_BINARY" => {}
            _ => return Err("CLI_OPERATION_KIND_REJECTED".into()),
        }
        self.set_job_state(job_id, "AWAITING_HUMAN_CONFIRMATION", None);
        let summary = format!(
            "Operation: {:?}\nCommand: {}\nBinding: {}…\n\nOnly the selected local Spine.com may run. Inputs are hashed; output is a separate new directory.",
            binding.operation_kind(),
            binding.command_summary().argv_shape.join(" "),
            &binding.binding_sha256()[..12]
        );
        let actor = match self.verified_human(
            hwnd,
            "run-spine-professional-cli",
            binding.binding_sha256(),
            &summary,
        ) {
            Ok(value) => value,
            Err(_) => {
                self.finish_job(job_id, "NOT_RUN", Some("HUMAN_CONFIRMATION_CANCELLED"));
                return Ok(());
            }
        };
        let now = unix_ms()?;
        let consent = SpineCliOperationConsent::from_verified_human_confirmation(
            &binding,
            VerifiedHumanCliConfirmation {
                operation_id,
                confirmation_id: actor.attestation_id().into(),
                actor_id: actor.actor_id().into(),
                actor_kind: "HUMAN".into(),
                attested_payload_sha256: binding.binding_sha256().into(),
                native_attestation_sha256: actor.proof_sha256().into(),
                issued_at_unix_ms: now,
                expires_at_unix_ms: now + 4 * 60 * 1_000,
            },
        )
        .map_err(|_| "CLI_CONSENT_REJECTED".to_owned())?;
        self.set_job_state(job_id, "RUNNING", None);
        let report = self.runner.run(&policy, &operation, consent);
        let report_bytes = serde_json::to_vec_pretty(&json!({
            "schemaVersion":"f2s-spine-cli-provenance-envelope/1.0",
            "report":report
        }))
        .map_err(|_| "CLI_PROVENANCE_SERIALIZATION_FAILED".to_owned())?;
        let provenance_sha256 = sha256_bytes(&report_bytes);
        let provenance_root = self.app_data_root.join("spine-cli").join("provenance");
        reject_reparse_ancestors(&provenance_root)
            .map_err(|_| "CLI_PROVENANCE_STORAGE_FAILED".to_owned())?;
        fs::create_dir_all(&provenance_root)
            .map_err(|_| "CLI_PROVENANCE_STORAGE_FAILED".to_owned())?;
        reject_reparse_ancestors(&provenance_root)
            .map_err(|_| "CLI_PROVENANCE_STORAGE_FAILED".to_owned())?;
        let provenance_path = provenance_root.join(format!("{job_id}.json"));
        write_atomic(&provenance_path, &report_bytes)
            .map_err(|_| "CLI_PROVENANCE_STORAGE_FAILED".to_owned())?;

        let mut outputs = Vec::new();
        let mut output_path_token = None;
        if let Some(provenance) = &report.provenance {
            output_path_token = provenance
                .output_artifact
                .as_ref()
                .map(|output| output.path_token.clone());
            for origin in &provenance.proprietary_outputs {
                let authorized = report
                    .authorizes_proprietary_output(&origin.relative_path, &origin.sha256_after);
                outputs.push(CliJobOutput {
                    relative_path: origin.relative_path.clone(),
                    extension: origin.extension.clone(),
                    sha256: origin.sha256_after.clone(),
                    authorized,
                });
            }
        }
        let succeeded = report.state == CliExecutionState::Succeeded
            && !outputs.is_empty()
            && outputs.iter().all(|output| output.authorized);
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| "CLI_JOB_REGISTRY_UNAVAILABLE".to_owned())?;
        let job = jobs.get_mut(job_id).ok_or("CLI_JOB_NOT_FOUND")?;
        job.state = if succeeded { "SUCCEEDED" } else { "FAILED" }.into();
        job.failure_code = if succeeded {
            None
        } else {
            report
                .failure_code
                .clone()
                .or_else(|| Some("PROPRIETARY_OUTPUT_NOT_AUTHORIZED".into()))
        };
        job.output_path_token = output_path_token;
        job.outputs = outputs;
        job.provenance_sha256 = Some(provenance_sha256);
        job.finished_at_unix_ms = Some(unix_ms().unwrap_or_default());
        Ok(())
    }

    fn choose_operation(
        &self,
        hwnd: HWND,
        grant: &OpenExportGrant,
        operation_id: &str,
        kind: &str,
    ) -> Result<Option<SpineCliOperation>, String> {
        let (selected_input, pack_settings) = match kind {
            "IMPORT_PROJECT" => {
                self.verify_open_export_file(grant, "character.spine.json")?;
                (Some(grant.directory.join("character.spine.json")), None)
            }
            "PACK_ATLAS" => {
                self.verify_open_export_images(grant)?;
                let Some(settings) = choose_file(
                    hwnd,
                    "Spine pack settings (*.json)\0*.json\0\0",
                    "Select Spine texture pack settings JSON",
                )?
                else {
                    return Ok(None);
                };
                (Some(grant.directory.join("images")), Some(settings))
            }
            "EXPORT_BINARY" => {
                let Some(project) = choose_file(
                    hwnd,
                    "Spine project (*.spine)\0*.spine\0\0",
                    "Select a Spine project to export as .skel",
                )?
                else {
                    return Ok(None);
                };
                (Some(project), None)
            }
            _ => return Err("CLI_OPERATION_KIND_REJECTED".into()),
        };
        let Some(base) = choose_folder(hwnd, "Select a separate folder for Spine CLI output")?
        else {
            return Ok(None);
        };
        let output_directory =
            self.create_output_directory(&base, &grant.directory, operation_id)?;
        let operation = match kind {
            "IMPORT_PROJECT" => SpineCliOperation::ImportProject {
                source_json: selected_input.ok_or("CLI_INPUT_MISSING")?,
                output_project: output_directory.join("character.spine"),
            },
            "PACK_ATLAS" => SpineCliOperation::PackAtlas {
                input_directory: selected_input.ok_or("CLI_INPUT_MISSING")?,
                output_directory,
                pack_settings_json: pack_settings.ok_or("CLI_PACK_SETTINGS_MISSING")?,
            },
            "EXPORT_BINARY" => SpineCliOperation::ExportBinary {
                source_project: selected_input.ok_or("CLI_INPUT_MISSING")?,
                output_directory,
            },
            _ => return Err("CLI_OPERATION_KIND_REJECTED".into()),
        };
        Ok(Some(operation))
    }

    fn create_output_directory(
        &self,
        selected_base: &Path,
        open_package: &Path,
        operation_id: &str,
    ) -> Result<PathBuf, String> {
        let base = selected_base
            .canonicalize()
            .map_err(|_| "CLI_OUTPUT_BASE_UNAVAILABLE".to_owned())?;
        let package = open_package
            .canonicalize()
            .map_err(|_| "OPEN_EXPORT_UNAVAILABLE".to_owned())?;
        let private = self
            .app_data_root
            .canonicalize()
            .map_err(|_| "PRIVATE_STORAGE_UNAVAILABLE".to_owned())?;
        if base.starts_with(&private) || base.starts_with(&package) {
            return Err("CLI_OUTPUT_BOUNDARY_REJECTED".into());
        }
        let output = base.join(format!("flash-to-spine-{operation_id}"));
        if output.starts_with(&package) || output.exists() {
            return Err("CLI_OUTPUT_BOUNDARY_REJECTED".into());
        }
        fs::create_dir(&output).map_err(|_| "CLI_OUTPUT_DIRECTORY_CREATE_FAILED".to_owned())?;
        output
            .canonicalize()
            .map_err(|_| "CLI_OUTPUT_DIRECTORY_CREATE_FAILED".to_owned())
    }

    fn verify_open_export_file(
        &self,
        grant: &OpenExportGrant,
        relative: &str,
    ) -> Result<(), String> {
        let expected = grant
            .checksums
            .get(relative)
            .ok_or("OPEN_EXPORT_CHECKSUM_MISSING")?;
        let bytes = fs::read(grant.directory.join(relative))
            .map_err(|_| "OPEN_EXPORT_ARTIFACT_UNAVAILABLE".to_owned())?;
        if &sha256_bytes(&bytes) != expected {
            return Err("OPEN_EXPORT_ARTIFACT_CHANGED".into());
        }
        Ok(())
    }

    fn verify_open_export_images(&self, grant: &OpenExportGrant) -> Result<(), String> {
        let expected = grant
            .checksums
            .keys()
            .filter(|path| path.starts_with("images/") && path.ends_with(".png"))
            .cloned()
            .collect::<BTreeSet<_>>();
        if expected.is_empty() {
            return Err("OPEN_EXPORT_IMAGES_MISSING".into());
        }
        let mut actual = BTreeSet::new();
        collect_relative_files(
            &grant.directory,
            &grant.directory.join("images"),
            &mut actual,
        )?;
        if actual != expected {
            return Err("OPEN_EXPORT_IMAGE_INVENTORY_CHANGED".into());
        }
        for path in expected {
            self.verify_open_export_file(grant, &path)?;
        }
        Ok(())
    }

    fn set_job_state(&self, job_id: &str, state: &str, failure_code: Option<&str>) {
        if let Ok(mut jobs) = self.jobs.lock()
            && let Some(job) = jobs.get_mut(job_id)
        {
            job.state = state.into();
            job.failure_code = failure_code.map(str::to_owned);
        }
    }

    fn finish_job(&self, job_id: &str, state: &str, failure_code: Option<&str>) {
        if let Ok(mut jobs) = self.jobs.lock()
            && let Some(job) = jobs.get_mut(job_id)
        {
            if is_terminal(&job.state) {
                return;
            }
            job.state = state.into();
            job.failure_code = failure_code.map(str::to_owned);
            job.finished_at_unix_ms = Some(unix_ms().unwrap_or_default());
        }
    }

    fn config_path(&self) -> PathBuf {
        self.app_data_root
            .join("spine-cli")
            .join("spine-cli-config.json")
    }

    fn save_config(&self, payload: &SpineCliConfigPayload) -> Result<(), String> {
        let bytes = serde_json::to_vec(payload).map_err(|error| error.to_string())?;
        let (key_id, key) =
            crate::local_security::load_or_create_project_integrity_key(&self.app_data_root)?;
        let config = SignedSpineCliConfig {
            payload: payload.clone(),
            integrity_key_id: key_id,
            integrity_hmac_sha256: hmac_sha256_hex(&key, &bytes),
        };
        let path = self.config_path();
        let parent = path.parent().ok_or("Spine CLI config path has no parent")?;
        reject_reparse_ancestors(parent)?;
        fs::create_dir_all(parent)
            .map_err(|_| "cannot create private Spine CLI config directory")?;
        reject_reparse_ancestors(parent)?;
        let bytes = serde_json::to_vec_pretty(&config).map_err(|error| error.to_string())?;
        write_atomic(&path, &bytes)
    }

    fn load_config_and_policy(
        &self,
    ) -> Result<Option<(SpineCliConfigPayload, SpineCliPolicy)>, String> {
        let path = self.config_path();
        if !path.exists() {
            return Ok(None);
        }
        reject_reparse_ancestors(&path)?;
        let metadata = fs::symlink_metadata(&path).map_err(|_| "invalid Spine CLI config")?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > CONFIG_MAX_BYTES
        {
            return Err("invalid Spine CLI config".into());
        }
        let bytes = fs::read(&path).map_err(|_| "invalid Spine CLI config")?;
        let config: SignedSpineCliConfig =
            serde_json::from_slice(&bytes).map_err(|_| "invalid Spine CLI config")?;
        let payload_bytes =
            serde_json::to_vec(&config.payload).map_err(|_| "invalid Spine CLI config")?;
        let (key_id, key) =
            crate::local_security::load_or_create_project_integrity_key(&self.app_data_root)?;
        let expected = hmac_sha256_hex(&key, &payload_bytes);
        if config.integrity_key_id != key_id
            || !constant_time_eq(config.integrity_hmac_sha256.as_bytes(), expected.as_bytes())
            || config.payload.schema_version != CONFIG_SCHEMA
            || config.payload.expected_patch != REQUIRED_SPINE_PATCH
            || !config.payload.professional_license_confirmed
        {
            return Err("invalid Spine CLI config".into());
        }
        let policy = SpineCliPolicy {
            executable: config.payload.canonical_executable.clone(),
            user_confirmed_professional_license: true,
            network_granted_for_operation: false,
            expected_patch: REQUIRED_SPINE_PATCH.into(),
        };
        let validated = policy.validate_selected_executable()?;
        if validated.executable_sha256() != config.payload.executable_sha256
            || validated.path_token() != config.payload.executable_path_token
        {
            return Err("selected Spine.com changed after configuration".into());
        }
        Ok(Some((config.payload, policy)))
    }

    fn verified_human(
        &self,
        hwnd: HWND,
        purpose: &str,
        payload_sha256: &str,
        summary: &str,
    ) -> Result<VerifiedHumanActor, String> {
        validate_sha256("native confirmation payload", payload_sha256)?;
        let text = format!(
            "FlashToSpine requires an explicit local confirmation.\n\n{summary}\n\nBound payload: {}…",
            &payload_sha256[..12]
        );
        let mut wide = text.encode_utf16().collect::<Vec<_>>();
        wide.push(0);
        let accepted = unsafe {
            MessageBoxW(
                Some(hwnd),
                PCWSTR(wide.as_ptr()),
                windows::core::w!("FlashToSpine — Spine Professional authorization"),
                MB_YESNO | MB_ICONQUESTION,
            )
        };
        if accepted != IDYES {
            return Err("native human confirmation was cancelled".into());
        }
        let attestation_id = Uuid::new_v4().simple().to_string();
        let proof = sha256_text(&format!(
            "{}:{attestation_id}:{purpose}:{payload_sha256}",
            self.session_nonce
        ));
        let now = unix_ms()?;
        let attestation = CredentialAttestation {
            attestation_id,
            actor_id: "local-interactive-user".into(),
            actor_kind: "HUMAN".into(),
            credential_ref: "windows-session/native-confirmation".into(),
            purpose: purpose.into(),
            issued_at_utc: format!("{now:020}"),
            expires_at_utc: format!("{:020}", now + 4 * 60 * 1_000),
            payload_sha256: payload_sha256.into(),
            verification_proof_sha256: proof,
        };
        VerifiedHumanActor::verify(
            attestation,
            purpose,
            payload_sha256,
            &CliNativeVerifier {
                consumed: &self.consumed_attestations,
                session_nonce: &self.session_nonce,
            },
        )
    }
}

struct CliNativeVerifier<'a> {
    consumed: &'a Mutex<HashSet<String>>,
    session_nonce: &'a str,
}

impl HumanCredentialVerifier for CliNativeVerifier<'_> {
    fn verify_and_consume(&self, attestation: &CredentialAttestation) -> Result<(), String> {
        if attestation.credential_ref != "windows-session/native-confirmation" {
            return Err("untrusted human credential source".into());
        }
        let expected = sha256_text(&format!(
            "{}:{}:{}:{}",
            self.session_nonce,
            attestation.attestation_id,
            attestation.purpose,
            attestation.payload_sha256
        ));
        if !constant_time_eq(
            attestation.verification_proof_sha256.as_bytes(),
            expected.as_bytes(),
        ) || attestation.issued_at_utc >= attestation.expires_at_utc
        {
            return Err("native confirmation proof or lifetime is invalid".into());
        }
        if !self
            .consumed
            .lock()
            .map_err(|_| "native confirmation registry is unavailable")?
            .insert(attestation.attestation_id.clone())
        {
            return Err("human approval attestation replayed".into());
        }
        Ok(())
    }
}

struct ComApartment(bool);

impl ComApartment {
    fn initialize() -> Result<Self, String> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
            .ok()
            .map_err(|_| "CLI_NATIVE_DIALOG_THREAD_INIT_FAILED".to_owned())?;
        Ok(Self(true))
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

fn choose_file(hwnd: HWND, filter: &str, title: &str) -> Result<Option<PathBuf>, String> {
    let mut file_buffer = vec![0_u16; 32_768];
    let filter = filter.encode_utf16().collect::<Vec<_>>();
    let mut title = title.encode_utf16().collect::<Vec<_>>();
    title.push(0);
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(file_buffer.as_mut_ptr()),
        nMaxFile: file_buffer.len() as u32,
        lpstrTitle: PCWSTR(title.as_ptr()),
        Flags: OFN_EXPLORER
            | OFN_FILEMUSTEXIST
            | OFN_PATHMUSTEXIST
            | OFN_NOCHANGEDIR
            | OFN_DONTADDTORECENT,
        ..Default::default()
    };
    if !unsafe { GetOpenFileNameW(&mut dialog).as_bool() } {
        let error = unsafe { CommDlgExtendedError() };
        return if error.0 == 0 {
            Ok(None)
        } else {
            Err("native file dialog failed".into())
        };
    }
    let length = file_buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(file_buffer.len());
    String::from_utf16(&file_buffer[..length])
        .map(PathBuf::from)
        .map(Some)
        .map_err(|_| "native file path is not valid UTF-16".into())
}

fn choose_folder(hwnd: HWND, title: &str) -> Result<Option<PathBuf>, String> {
    let dialog: IFileOpenDialog = unsafe {
        CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
            .map_err(|_| "native output folder dialog is unavailable")?
    };
    let mut title = title.encode_utf16().collect::<Vec<_>>();
    title.push(0);
    unsafe {
        let options = dialog
            .GetOptions()
            .map_err(|_| "native output folder dialog is unavailable")?;
        dialog
            .SetOptions(options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST)
            .map_err(|_| "native output folder dialog cannot be configured")?;
        dialog
            .SetTitle(PCWSTR(title.as_ptr()))
            .map_err(|_| "native output folder dialog cannot be configured")?;
        if let Err(error) = dialog.Show(Some(hwnd)) {
            if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
                return Ok(None);
            }
            return Err("native output folder dialog failed".into());
        }
        let item = dialog
            .GetResult()
            .map_err(|_| "native output folder dialog returned no result")?;
        let raw = item
            .GetDisplayName(SIGDN_FILESYSPATH)
            .map_err(|_| "cannot resolve selected output folder")?;
        let value = raw.to_string().map(PathBuf::from);
        CoTaskMemFree(Some(raw.0.cast()));
        value
            .map(Some)
            .map_err(|_| "selected output folder is not valid UTF-16".into())
    }
}

fn is_terminal(state: &str) -> bool {
    matches!(state, "SUCCEEDED" | "FAILED" | "NOT_RUN")
}

fn collect_relative_files(
    root: &Path,
    directory: &Path,
    output: &mut BTreeSet<String>,
) -> Result<(), String> {
    use std::os::windows::fs::MetadataExt;
    for entry in fs::read_dir(directory).map_err(|_| "OPEN_EXPORT_IMAGES_UNAVAILABLE")? {
        let entry = entry.map_err(|_| "OPEN_EXPORT_IMAGES_UNAVAILABLE")?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|_| "OPEN_EXPORT_IMAGES_UNAVAILABLE")?;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("OPEN_EXPORT_IMAGE_REPARSE_REJECTED".into());
        }
        if metadata.is_dir() {
            collect_relative_files(root, &path, output)?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "OPEN_EXPORT_IMAGE_PATH_REJECTED")?
                .to_string_lossy()
                .replace('\\', "/");
            if !output.insert(relative) {
                return Err("OPEN_EXPORT_IMAGE_PATH_DUPLICATE".into());
            }
        } else {
            return Err("OPEN_EXPORT_IMAGE_ENTRY_REJECTED".into());
        }
    }
    Ok(())
}

fn reject_reparse_ancestors(path: &Path) -> Result<(), String> {
    use std::os::windows::fs::MetadataExt;
    for current in path.ancestors() {
        if current.as_os_str().is_empty() || !current.exists() {
            continue;
        }
        let metadata =
            fs::symlink_metadata(current).map_err(|_| "private path inspection failed")?;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("reparse point rejected in private Spine CLI storage".into());
        }
    }
    Ok(())
}

fn path_token(path: &Path) -> String {
    let text = path.to_string_lossy();
    let normalized = text
        .strip_prefix(r"\\?\")
        .unwrap_or(&text)
        .replace('/', "\\")
        .to_ascii_lowercase();
    sha256_text(&normalized)
}

fn sha256_text(value: &str) -> String {
    sha256_bytes(value.as_bytes())
}

fn sha256_bytes(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    const BLOCK_SIZE: usize = 64;
    let mut normalized = [0_u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        normalized[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        normalized[..key.len()].copy_from_slice(key);
    }
    let mut inner_pad = [0x36_u8; BLOCK_SIZE];
    let mut outer_pad = [0x5c_u8; BLOCK_SIZE];
    for index in 0..BLOCK_SIZE {
        inner_pad[index] ^= normalized[index];
        outer_pad[index] ^= normalized[index];
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(message);
    let inner = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner);
    format!("{:x}", outer.finalize())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn validate_identifier(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
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
        .map_err(|_| "system clock is before Unix epoch")?
        .as_millis();
    u64::try_from(value).map_err(|_| "system time is out of range".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_tokens_and_hmacs_do_not_expose_paths_or_accept_mutation() {
        let path = Path::new(r"C:\Users\alice\Spine\Spine.com");
        let token = path_token(path);
        assert_eq!(token.len(), 64);
        assert!(!token.contains("alice"));
        let first = hmac_sha256_hex(b"key", b"payload");
        let second = hmac_sha256_hex(b"key", b"payload-2");
        assert_ne!(first, second);
        assert!(constant_time_eq(first.as_bytes(), first.as_bytes()));
        assert!(!constant_time_eq(first.as_bytes(), second.as_bytes()));
    }

    #[test]
    fn job_terminal_states_are_explicit() {
        assert!(is_terminal("SUCCEEDED"));
        assert!(is_terminal("FAILED"));
        assert!(is_terminal("NOT_RUN"));
        assert!(!is_terminal("RUNNING"));
    }

    #[test]
    fn missing_real_cli_job_returns_immediately_then_stays_external_not_run() {
        let root =
            std::env::temp_dir().join(format!("f2s-cli-host-not-run-{}", Uuid::new_v4().simple()));
        let open_export = root.join("open-export");
        fs::create_dir_all(&open_export).unwrap();
        let host = SpineCliHost {
            app_data_root: root.join("private"),
            runner: Arc::new(SpineCliRunner::new(SpineCliRunnerLimits::default()).unwrap()),
            jobs: Arc::new(Mutex::new(HashMap::new())),
            open_exports: Arc::new(Mutex::new(HashMap::new())),
            consumed_attestations: Arc::new(Mutex::new(HashSet::new())),
            session_nonce: Arc::new("test-session".into()),
        };
        host.register_open_export(OpenExportGrant {
            export_id: "export-1".into(),
            project_id: "project-1".into(),
            project_revision: 7,
            snapshot_sha256: "a".repeat(64),
            directory: open_export,
            checksums: BTreeMap::new(),
        })
        .unwrap();
        let initial = host
            .start_job(
                "export-1",
                "IMPORT_PROJECT",
                "project-1",
                7,
                HWND(std::ptr::null_mut()),
            )
            .unwrap();
        assert_eq!(initial["state"], "QUEUED");
        let job_id = initial["jobId"].as_str().unwrap();
        let mut terminal = Value::Null;
        for _ in 0..100 {
            terminal = host.job_status(job_id).unwrap();
            if terminal["state"] == "NOT_RUN" {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(terminal["evidenceClass"], "EXTERNAL");
        assert_eq!(terminal["state"], "NOT_RUN");
        assert_eq!(terminal["failureCode"], "EXTERNAL_CLI_NOT_SELECTED");
        assert!(terminal["provenanceSha256"].is_null());
        let _ = fs::remove_dir_all(root);
    }
}

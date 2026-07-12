#![cfg(windows)]

use f2s_adapters::export::{
    cli_policy::SpineCliPolicy,
    cli_runner::{
        CliExecutionState, SpineCliOperation, SpineCliOperationConsent, SpineCliRunner,
        SpineCliRunnerLimits, VerifiedHumanCliConfirmation,
    },
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

struct SyntheticFixture {
    root: PathBuf,
    executable: PathBuf,
    source_json: PathBuf,
    output_project: PathBuf,
}

impl SyntheticFixture {
    fn compile(mode: &str) -> Self {
        let nonce = format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(format!("f2s-spine-cli-negative-{nonce}"));
        fs::create_dir_all(&root).unwrap();
        let root = fs::canonicalize(root).unwrap();
        let executable = root.join("Spine.com");
        let source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/synthetic_spine.rs");
        let status = Command::new("rustc")
            .args(["--edition=2024", "-o"])
            .arg(&executable)
            .arg(source)
            .status()
            .expect("rustc must compile the synthetic negative fixture");
        assert!(status.success());
        fs::write(root.join("synthetic-mode.txt"), mode).unwrap();
        let source_json = root.join("source.json");
        fs::write(&source_json, br#"{"skeleton":{"spine":"4.2.43"}}"#).unwrap();
        Self {
            output_project: root.join("result.spine"),
            root,
            executable,
            source_json,
        }
    }

    fn policy(&self) -> SpineCliPolicy {
        SpineCliPolicy {
            executable: self.executable.clone(),
            user_confirmed_professional_license: true,
            network_granted_for_operation: false,
            expected_patch: "4.2.43".into(),
        }
    }

    fn operation(&self) -> SpineCliOperation {
        SpineCliOperation::ImportProject {
            source_json: self.source_json.clone(),
            output_project: self.output_project.clone(),
        }
    }
}

impl Drop for SyntheticFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn runner(probe_timeout: Duration, output_limit: usize) -> SpineCliRunner {
    SpineCliRunner::new(SpineCliRunnerLimits {
        probe_timeout,
        operation_timeout: Duration::from_secs(1),
        stdout_limit_bytes: output_limit,
        stderr_limit_bytes: output_limit,
        snapshot_file_limit: 128,
        snapshot_byte_limit: 16 * 1024 * 1024,
    })
    .unwrap()
}

fn consent(runner: &SpineCliRunner, fixture: &SyntheticFixture) -> SpineCliOperationConsent {
    let operation = fixture.operation();
    let binding = runner
        .prepare_consent_binding(&fixture.policy(), &operation, "negative-op-1")
        .unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    SpineCliOperationConsent::from_verified_human_confirmation(
        &binding,
        VerifiedHumanCliConfirmation {
            operation_id: "negative-op-1".into(),
            confirmation_id: "native-confirmation-1".into(),
            actor_id: "test-human".into(),
            actor_kind: "HUMAN".into(),
            attested_payload_sha256: binding.binding_sha256().into(),
            native_attestation_sha256: "a".repeat(64),
            issued_at_unix_ms: now,
            expires_at_unix_ms: now + 60_000,
        },
    )
    .unwrap()
}

#[test]
fn missing_external_cli_is_explicitly_external_not_run() {
    let runner = runner(Duration::from_secs(1), 8 * 1024);
    let policy = SpineCliPolicy {
        executable: PathBuf::from(r"C:\definitely-missing\Spine.com"),
        user_confirmed_professional_license: true,
        network_granted_for_operation: false,
        expected_patch: "4.2.43".into(),
    };
    let assessment = runner.assess_selection(&policy);
    assert_eq!(assessment.state, CliExecutionState::NotRun);
    assert_eq!(
        assessment.reason_code,
        "EXTERNAL_CLI_UNAVAILABLE_OR_REJECTED"
    );
    assert_eq!(format!("{:?}", assessment.evidence_class), "External");
}

#[test]
fn synthetic_wrong_version_never_reaches_the_operation() {
    let fixture = SyntheticFixture::compile("wrong-version");
    let runner = runner(Duration::from_secs(2), 8 * 1024);
    let consent = consent(&runner, &fixture);
    let report = runner.run(&fixture.policy(), &fixture.operation(), consent);
    assert_eq!(report.state, CliExecutionState::Failed);
    assert_eq!(
        report.failure_code.as_deref(),
        Some("EDITOR_PATCH_MISMATCH")
    );
    assert!(report.provenance.is_none());
    assert!(!fixture.output_project.exists());
}

#[test]
fn one_native_confirmation_cannot_be_replayed_on_the_same_runner() {
    let fixture = SyntheticFixture::compile("wrong-version");
    let runner = runner(Duration::from_secs(2), 8 * 1024);
    let first = consent(&runner, &fixture);
    let second = consent(&runner, &fixture);
    let first_report = runner.run(&fixture.policy(), &fixture.operation(), first);
    assert_eq!(first_report.state, CliExecutionState::Failed);
    let second_report = runner.run(&fixture.policy(), &fixture.operation(), second);
    assert_eq!(second_report.state, CliExecutionState::NotRun);
    assert_eq!(
        second_report.failure_code.as_deref(),
        Some("CURRENT_OPERATION_CONSENT_ALREADY_USED")
    );
}

#[test]
fn synthetic_ambient_or_ambiguous_version_is_not_accepted() {
    for mode in ["ambient-version", "ambiguous-version"] {
        let fixture = SyntheticFixture::compile(mode);
        let runner = runner(Duration::from_secs(2), 8 * 1024);
        let consent = consent(&runner, &fixture);
        let report = runner.run(&fixture.policy(), &fixture.operation(), consent);
        assert_eq!(report.state, CliExecutionState::Failed, "mode={mode}");
        assert!(report.provenance.is_none());
        assert!(!fixture.output_project.exists());
    }
}

#[test]
fn synthetic_probe_timeout_is_bounded_and_operation_is_not_run() {
    let fixture = SyntheticFixture::compile("hang");
    let runner = runner(Duration::from_millis(150), 8 * 1024);
    let consent = consent(&runner, &fixture);
    let started = std::time::Instant::now();
    let report = runner.run(&fixture.policy(), &fixture.operation(), consent);
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(report.failure_code.as_deref(), Some("PROCESS_TIMEOUT"));
    assert!(!fixture.output_project.exists());
}

#[test]
fn synthetic_probe_output_is_capped_and_rejected() {
    let fixture = SyntheticFixture::compile("overflow");
    let runner = runner(Duration::from_secs(2), 4 * 1024);
    let consent = consent(&runner, &fixture);
    let report = runner.run(&fixture.policy(), &fixture.operation(), consent);
    assert_eq!(
        report.failure_code.as_deref(),
        Some("PROCESS_OUTPUT_LIMIT_EXCEEDED")
    );
    let probe = report.probe.unwrap();
    assert!(probe.stdout.truncated);
    assert!(probe.stdout.captured_byte_length <= 4 * 1024);
    assert!(!fixture.output_project.exists());
}

#[test]
fn consent_is_rejected_when_native_attestation_binds_another_payload() {
    let fixture = SyntheticFixture::compile("wrong-version");
    let runner = runner(Duration::from_secs(2), 8 * 1024);
    let operation = fixture.operation();
    let binding = runner
        .prepare_consent_binding(&fixture.policy(), &operation, "negative-op-2")
        .unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let result = SpineCliOperationConsent::from_verified_human_confirmation(
        &binding,
        VerifiedHumanCliConfirmation {
            operation_id: "negative-op-2".into(),
            confirmation_id: "native-confirmation-2".into(),
            actor_id: "test-human".into(),
            actor_kind: "HUMAN".into(),
            attested_payload_sha256: "b".repeat(64),
            native_attestation_sha256: "a".repeat(64),
            issued_at_unix_ms: now,
            expires_at_unix_ms: now + 60_000,
        },
    );
    assert!(result.is_err());
}

#[test]
fn executable_policy_rejects_wrong_name_and_network_enabled_mode() {
    let fixture = SyntheticFixture::compile("wrong-version");
    let wrong_name = fixture.root.join("renamed.exe");
    fs::copy(&fixture.executable, &wrong_name).unwrap();
    let mut policy = fixture.policy();
    policy.executable = wrong_name;
    assert!(policy.validate().is_err());
    let mut policy = fixture.policy();
    policy.network_granted_for_operation = true;
    assert!(policy.validate().is_err());

    let mut policy = fixture.policy();
    policy.executable = PathBuf::from(format!(r"{}\.\Spine.com", fixture.root.display()));
    assert!(policy.validate().is_err());

    let reparse = fixture.root.join("linked").join("Spine.com");
    fs::create_dir(fixture.root.join("linked")).unwrap();
    if std::os::windows::fs::symlink_file(&fixture.executable, &reparse).is_ok() {
        let mut policy = fixture.policy();
        policy.executable = reparse;
        assert!(policy.validate().is_err());
    }
}

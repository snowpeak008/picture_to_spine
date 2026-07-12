use f2s_adapters::safety::{
    diagnostics::{diagnostic_bundle_is_safe, redact},
    worker_eligibility::{
        ProbeState, REQUIRED_CONTROLS, SandboxControl, evaluate_worker_eligibility,
    },
};
use f2s_domain::remote_gpu::{
    EndpointOwnership, RemoteGpuMethod, RemoteGpuProfile, RemoteMediaType,
};
use serde_json::json;
#[test]
fn worker_stays_excluded_without_five_real_native_controls() {
    let not_run = REQUIRED_CONTROLS
        .iter()
        .map(|id| SandboxControl {
            id: (*id).into(),
            state: ProbeState::NotRun,
            evidence_sha256: None,
        })
        .collect();
    assert!(!evaluate_worker_eligibility("windows-appcontainer-v1", not_run).worker_pack_eligible);
    let pass = REQUIRED_CONTROLS
        .iter()
        .map(|id| SandboxControl {
            id: (*id).into(),
            state: ProbeState::Pass,
            evidence_sha256: Some("a".repeat(64)),
        })
        .collect();
    assert!(evaluate_worker_eligibility("windows-appcontainer-v1", pass).worker_pack_eligible);
}
#[test]
fn private_remote_requires_explicit_profile_pin_and_credential_reference() {
    let mut config = RemoteGpuProfile {
        schema_version: "1.0.0".into(),
        enabled: true,
        profile_id: "private-01".into(),
        ownership: EndpointOwnership::UserControlledPrivate,
        origin: "https://gpu.internal.example".into(),
        allowed_ports: vec![443],
        certificate_spki_sha256: "b".repeat(64),
        organization_identity_sha256: "c".repeat(64),
        credential_manager_target: "FlashToSpine/RemoteGpu/private-01".into(),
        allowed_methods: vec![RemoteGpuMethod::LayerSegmentationCandidate],
        allowed_input_media_types: vec![RemoteMediaType::ImagePng],
        allowed_model_manifest_sha256: vec!["d".repeat(64)],
        max_upload_bytes: 1024,
        max_response_bytes: 1024,
        request_timeout_seconds: 60,
    };
    config.validate_configuration().unwrap();
    config.origin = "https://api.openai.com".into();
    assert!(config.validate_configuration().is_err());
}
#[test]
fn diagnostics_remove_secret_and_absolute_path_canaries() {
    let input = json!({"token":"canary","nested":{"sourcePath":"C:\\Users\\alice\\private.png","safe":"revision-7"},"authorization":"Bearer private"});
    let output = redact(&input);
    assert!(diagnostic_bundle_is_safe(&output));
    let text = output.to_string();
    assert!(!text.contains("alice"));
    assert!(!text.contains("canary"));
    assert!(text.contains("revision-7"));
}

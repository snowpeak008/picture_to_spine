use f2s_adapters::storage::ntfs_atomic::write_atomic;
use f2s_domain::project::ProjectManifest;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use windows::{
    Win32::{Foundation::HWND, UI::Controls::Dialogs::*},
    core::{PCWSTR, PWSTR},
};

const REPORT_SCHEMA: &str = "f2s-redacted-diagnostics/1.0.0";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CapabilitySummary<'a> {
    ipc: &'a str,
    image_decode: &'a str,
    worker: &'a str,
    private_remote_gpu: &'a str,
    spine_editor: &'a str,
    project_integrity: &'a str,
    network_call_count: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RedactedProjectSummary {
    project_id_token: String,
    revision: u64,
    workflow_stage: String,
    source_count: usize,
    approval_record_count: usize,
    review_record_count: usize,
    export_record_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RedactedDiagnostics<'a> {
    schema_version: &'static str,
    product: &'static str,
    product_version: &'static str,
    generated_at_utc: &'a str,
    release_authorized: bool,
    privacy_class: &'static str,
    capabilities: CapabilitySummary<'a>,
    current_project: Option<RedactedProjectSummary>,
    excluded_content: [&'static str; 7],
}

fn field<'a>(status: &'a Value, key: &str, fallback: &'a str) -> &'a str {
    status.get(key).and_then(Value::as_str).unwrap_or(fallback)
}

pub fn redacted_report_bytes(
    status: &Value,
    project: Option<&ProjectManifest>,
    generated_at_utc: &str,
) -> Result<Vec<u8>, String> {
    if generated_at_utc.trim().is_empty() {
        return Err("diagnostics timestamp required".into());
    }
    let project = project.map(|project| RedactedProjectSummary {
        project_id_token: format!(
            "{:x}",
            Sha256::digest(project.identity.project_id.to_string().as_bytes())
        ),
        revision: project.revision,
        workflow_stage: project.workflow_stage.clone(),
        source_count: project.source_artifacts.len(),
        approval_record_count: project.approval_log.len(),
        review_record_count: project.review_log.len(),
        export_record_count: project.export_records.len(),
    });
    let report = RedactedDiagnostics {
        schema_version: REPORT_SCHEMA,
        product: "FlashToSpine",
        product_version: env!("CARGO_PKG_VERSION"),
        generated_at_utc,
        release_authorized: false,
        privacy_class: "REDACTED_LOCAL_SUPPORT",
        capabilities: CapabilitySummary {
            ipc: field(status, "ipc", "UNVERIFIED"),
            image_decode: field(status, "imageDecode", "UNVERIFIED"),
            worker: field(status, "worker", "UNVERIFIED_EXCLUDED"),
            private_remote_gpu: field(status, "privateRemoteGpu", "NOT_RUN_EXTERNAL"),
            spine_editor: field(status, "spineEditor", "EXTERNAL"),
            project_integrity: field(status, "projectIntegrity", "UNVERIFIED"),
            network_call_count: status
                .get("networkCallCount")
                .and_then(Value::as_u64)
                .unwrap_or_default(),
        },
        current_project: project,
        excluded_content: [
            "image-bytes",
            "prompt-text",
            "credentials",
            "absolute-paths",
            "windows-user-name",
            "private-endpoint-origin",
            "spine-activation-information",
        ],
    };
    let mut bytes = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn choose_and_write_report(
    hwnd: HWND,
    bytes: &[u8],
    app_data_root: &Path,
) -> Result<Option<PathBuf>, String> {
    let mut file_buffer = vec![0u16; 32_768];
    let filter = "JSON report (*.json)\0*.json\0\0"
        .encode_utf16()
        .collect::<Vec<_>>();
    let title = "保存脱敏诊断报告\0".encode_utf16().collect::<Vec<_>>();
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(file_buffer.as_mut_ptr()),
        nMaxFile: file_buffer.len() as u32,
        lpstrTitle: PCWSTR(title.as_ptr()),
        lpstrDefExt: windows::core::w!("json"),
        Flags: OFN_EXPLORER | OFN_NOCHANGEDIR | OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST,
        ..Default::default()
    };
    let selected = unsafe { GetSaveFileNameW(&mut dialog).as_bool() };
    if !selected {
        let error = unsafe { CommDlgExtendedError() };
        if error.0 == 0 {
            return Ok(None);
        }
        return Err(format!(
            "native diagnostics save dialog failed: {}",
            error.0
        ));
    }
    let length = file_buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(file_buffer.len());
    let path = PathBuf::from(
        String::from_utf16(&file_buffer[..length])
            .map_err(|_| "diagnostics path is not valid UTF-16")?,
    );
    if path.extension().and_then(|value| value.to_str()) != Some("json") {
        return Err("diagnostics report must use the .json extension".into());
    }
    ensure_outside_private_storage(&path, app_data_root)?;
    write_atomic(&path, bytes)?;
    Ok(Some(path))
}

pub fn ensure_outside_private_storage(path: &Path, app_data_root: &Path) -> Result<(), String> {
    let parent = path.parent().ok_or("diagnostics path has no parent")?;
    let parent = parent.canonicalize().map_err(|error| error.to_string())?;
    let private = app_data_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if parent.starts_with(private) {
        return Err(
            "diagnostics report cannot be written inside private application storage".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use f2s_domain::project::ProjectIdentity;

    #[test]
    fn report_excludes_names_paths_prompts_and_endpoint_details() {
        let project = ProjectManifest::new(ProjectIdentity::new("secret-project-name").unwrap());
        let status = serde_json::json!({
            "ipc":"WIRED",
            "imageDecode":"BOUNDED",
            "worker":"UNVERIFIED_EXCLUDED",
            "privateRemoteGpu":"CONFIGURED_DISABLED_NOT_RUN_EXTERNAL",
            "spineEditor":"EXTERNAL",
            "projectIntegrity":"DPAPI_CURRENT_USER_HMAC_CHAIN",
            "networkCallCount":0,
            "origin":"https://gpu.private.example",
            "absolutePath":"C:\\Users\\secret\\image.png",
            "prompt":"secret prompt"
        });
        let bytes = redacted_report_bytes(&status, Some(&project), "2026-07-11T00:00:00Z").unwrap();
        let text = String::from_utf8(bytes).unwrap();
        for forbidden in [
            "secret-project-name",
            "gpu.private.example",
            "C:\\\\Users",
            "secret prompt",
        ] {
            assert!(!text.contains(forbidden), "leaked {forbidden}");
        }
        assert!(text.contains("DPAPI_CURRENT_USER_HMAC_CHAIN"));
        assert!(text.contains("REDACTED_LOCAL_SUPPORT"));
    }
}

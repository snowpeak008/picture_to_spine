use f2s_adapters::ipc::decode_webview_request;
use f2s_adapters::{
    ipc::{MAX_FRAME_BYTES, decode_frame, encode_frame},
    storage::{FsCas, FsProjectStore},
};
use f2s_application::{
    ports::{CasStore, ProjectStore},
    storage::commit_project,
};
use std::{fs, path::PathBuf};
use uuid::Uuid;
fn temp() -> PathBuf {
    std::env::temp_dir().join(format!("f2s-test-{}", Uuid::new_v4()))
}
#[test]
fn frame_rejects_truncation_and_oversize() {
    let frame = encode_frame(b"ok").unwrap();
    assert_eq!(decode_frame(&frame).unwrap(), b"ok");
    assert!(decode_frame(&frame[..5]).is_err());
    assert!(encode_frame(&vec![0; MAX_FRAME_BYTES + 1]).is_err());
}
#[test]
fn project_and_cas_commit_is_atomic_and_immutable() {
    let root = temp();
    let cas = FsCas::new(root.join("cas"));
    let projects = FsProjectStore::new(root.join("projects"));
    let manifest = serde_json::json!({"z":1,"a":2});
    let head = commit_project(&projects, &cas, "project-1", 0, None, &manifest).unwrap();
    assert_eq!(projects.load_head("project-1").unwrap(), Some(head.clone()));
    let reference = f2s_domain::storage::CasRef {
        sha256: head.manifest_sha256,
        byte_length: 13,
        media_type: "application/json".into(),
    };
    assert!(!cas.get(&reference).unwrap().is_empty());
    assert!(commit_project(&projects, &cas, "project-1", 0, None, &manifest).is_err());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn project_id_cannot_escape_root() {
    let root = temp();
    let projects = FsProjectStore::new(&root);
    assert!(projects.load_head("../escape").is_err());
    assert!(!root.join("escape").exists());
}

#[test]
fn webview_ipc_is_versioned_allowlisted_and_bounded() {
    let valid = r#"{"schemaVersion":"1.0.0","requestId":"req-1","method":"bootstrap.status","expectedRevision":null,"payload":{}}"#;
    assert!(decode_webview_request(valid).is_ok());
    assert!(decode_webview_request(&valid.replace("bootstrap.status", "shell.execute")).is_err());
    for method in [
        "spineCli.status",
        "spineCli.selectAndAssess",
        "spineCli.clear",
        "spineCli.job.start",
        "spineCli.job.status",
    ] {
        assert!(
            decode_webview_request(&valid.replace("bootstrap.status", method)).is_ok(),
            "typed Spine CLI method must be allowlisted: {method}"
        );
    }
    assert!(
        decode_webview_request(&valid.replace("\"payload\":{}", "\"payload\":{},\"extra\":true"))
            .is_err()
    );
    assert!(decode_webview_request(&"x".repeat(1024 * 1024 + 1)).is_err());
}

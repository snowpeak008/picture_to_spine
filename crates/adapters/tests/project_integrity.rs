use f2s_adapters::storage::{FsCas, FsProjectStore};
use f2s_application::{
    ports::ProjectStore,
    project::{create_project, open_project},
    storage::commit_project,
};
use f2s_domain::{canonical::canonical_bytes, storage::ProjectHead};
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
use uuid::Uuid;

const CROSS_PROCESS_KEY_ID: &str = "core-storage-cross-process-test-key-v1";
const CROSS_PROCESS_KEY: [u8; 32] = [0xc7; 32];
const CROSS_PROCESS_WRONG_KEY: [u8; 32] = [0x3d; 32];
const CROSS_PROCESS_PROJECT_NAME: &str = "CORE_STORAGE_CROSS_PROCESS project";
const CROSS_PROCESS_SENTINEL_FILE: &str = "CORE_STORAGE_CROSS_PROCESS.child-root";
const CROSS_PROCESS_SENTINEL_CONTENT: &str = "CORE_STORAGE_CROSS_PROCESS authorized child root";
const CROSS_PROCESS_PROJECT_ID_FILE: &str = "CORE_STORAGE_CROSS_PROCESS.project-id";
const CROSS_PROCESS_CHILD_CREATE_TEST: &str = "CORE_STORAGE_CROSS_PROCESS_CHILD_CREATE";
const CROSS_PROCESS_CHILD_OPEN_TEST: &str = "CORE_STORAGE_CROSS_PROCESS_CHILD_OPEN";
const CROSS_PROCESS_CHILD_CREATE_PASS: &str = "CORE_STORAGE_CROSS_PROCESS CHILD_CREATE PASS";
const CROSS_PROCESS_CHILD_OPEN_PASS: &str = "CORE_STORAGE_CROSS_PROCESS CHILD_OPEN PASS";

fn temp() -> PathBuf {
    std::env::temp_dir().join(format!("f2s-integrity-test-{}", Uuid::new_v4()))
}

fn secure_store(root: &PathBuf) -> FsProjectStore {
    FsProjectStore::new_with_integrity_key(root, "dpapi-current-user-v1", [0x5a; 32])
        .expect("secure project store")
}

fn cross_process_store(root: &PathBuf) -> FsProjectStore {
    FsProjectStore::new_with_integrity_key(root, CROSS_PROCESS_KEY_ID, CROSS_PROCESS_KEY)
        .expect("CORE_STORAGE_CROSS_PROCESS secure project store")
}

struct CrossProcessRoot(PathBuf);

impl Drop for CrossProcessRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn cross_process_child_root() -> Option<PathBuf> {
    let root = std::env::current_dir().expect("CORE_STORAGE_CROSS_PROCESS child current directory");
    let sentinel = fs::read_to_string(root.join(CROSS_PROCESS_SENTINEL_FILE)).ok();
    if sentinel.as_deref() != Some(CROSS_PROCESS_SENTINEL_CONTENT) {
        println!("CORE_STORAGE_CROSS_PROCESS CHILD SKIP unauthorized child root");
        return None;
    }
    Some(root)
}

fn run_cross_process_child(root: &PathBuf, test_name: &str) -> Output {
    Command::new(
        std::env::current_exe().expect("CORE_STORAGE_CROSS_PROCESS current test executable"),
    )
    .current_dir(root)
    .args([
        test_name,
        "--exact",
        "--ignored",
        "--no-capture",
        "--test-threads",
        "1",
    ])
    .output()
    .expect("CORE_STORAGE_CROSS_PROCESS spawn child test process")
}

fn assert_cross_process_child_passed(output: &Output, pass_marker: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "CORE_STORAGE_CROSS_PROCESS child failed with {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        stdout,
        stderr
    );
    assert!(
        stdout.contains(pass_marker),
        "CORE_STORAGE_CROSS_PROCESS child did not emit {pass_marker:?}\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

fn head_path(root: &std::path::Path, project_id: &str) -> PathBuf {
    root.join(project_id).join("head.json")
}

fn anchor_path(root: &std::path::Path, project_id: &str) -> PathBuf {
    root.join("security")
        .join("anchors")
        .join(format!("{project_id}.json"))
}

fn revision_path(root: &std::path::Path, project_id: &str, revision: u64) -> PathBuf {
    root.join(project_id)
        .join("revisions")
        .join(format!("{revision}.json"))
}

// Production obtains this same 32-byte store key contract by unwrapping a
// DPAPI CurrentUser-protected key in the host. These child modes deliberately
// inject fixed test-only material in separate processes: they prove that the
// persisted project/CAS data and HMAC chain reopen across a process boundary,
// while leaving DPAPI provisioning itself outside this adapter-level test.
#[test]
#[ignore = "CORE_STORAGE_CROSS_PROCESS child mode; launched by the parent test"]
#[allow(non_snake_case)]
fn CORE_STORAGE_CROSS_PROCESS_CHILD_CREATE() {
    let Some(root) = cross_process_child_root() else {
        return;
    };
    let store_root = root.join("projects");
    let cas = FsCas::new(root.join("cas"));
    let store = cross_process_store(&store_root);

    let project = create_project(&store, &cas, CROSS_PROCESS_PROJECT_NAME)
        .expect("CORE_STORAGE_CROSS_PROCESS child create project");
    let project_id = project.identity.project_id.to_string();
    let head = store
        .load_head(&project_id)
        .expect("CORE_STORAGE_CROSS_PROCESS child load created head")
        .expect("CORE_STORAGE_CROSS_PROCESS child created head missing");
    assert_eq!(head.key_id.as_deref(), Some(CROSS_PROCESS_KEY_ID));
    assert_eq!(head.head_mac.as_deref().map(str::len), Some(64));
    fs::write(root.join(CROSS_PROCESS_PROJECT_ID_FILE), &project_id)
        .expect("CORE_STORAGE_CROSS_PROCESS child persist project id");

    println!(
        "{CROSS_PROCESS_CHILD_CREATE_PASS} pid={} project_id={project_id}",
        std::process::id()
    );
}

#[test]
#[ignore = "CORE_STORAGE_CROSS_PROCESS child mode; launched by the parent test"]
#[allow(non_snake_case)]
fn CORE_STORAGE_CROSS_PROCESS_CHILD_OPEN() {
    let Some(root) = cross_process_child_root() else {
        return;
    };
    let project_id = fs::read_to_string(root.join(CROSS_PROCESS_PROJECT_ID_FILE))
        .expect("CORE_STORAGE_CROSS_PROCESS child read project id");
    let project_id = project_id.trim();
    Uuid::parse_str(project_id).expect("CORE_STORAGE_CROSS_PROCESS child project id is a UUID");
    let store_root = root.join("projects");
    let cas = FsCas::new(root.join("cas"));

    let wrong_key_store = FsProjectStore::new_with_integrity_key(
        &store_root,
        CROSS_PROCESS_KEY_ID,
        CROSS_PROCESS_WRONG_KEY,
    )
    .expect("CORE_STORAGE_CROSS_PROCESS wrong-key store construction");
    let wrong_key_error = wrong_key_store
        .load_head(project_id)
        .expect_err("CORE_STORAGE_CROSS_PROCESS wrong HMAC key must fail closed");
    assert!(
        wrong_key_error.contains("MAC"),
        "CORE_STORAGE_CROSS_PROCESS wrong-key failure was not a MAC failure: {wrong_key_error}"
    );
    drop(wrong_key_store);

    let store = cross_process_store(&store_root);
    let reopened = open_project(&store, &cas, project_id)
        .expect("CORE_STORAGE_CROSS_PROCESS child reopen project")
        .expect("CORE_STORAGE_CROSS_PROCESS child project missing");
    assert_eq!(reopened.identity.project_id.to_string(), project_id);
    assert_eq!(reopened.identity.display_name, CROSS_PROCESS_PROJECT_NAME);
    assert_eq!(reopened.revision, 0);
    let head = store
        .load_head(project_id)
        .expect("CORE_STORAGE_CROSS_PROCESS child reload verified head")
        .expect("CORE_STORAGE_CROSS_PROCESS child verified head missing");
    assert_eq!(head.key_id.as_deref(), Some(CROSS_PROCESS_KEY_ID));
    assert_eq!(head.head_mac.as_deref().map(str::len), Some(64));

    println!(
        "{CROSS_PROCESS_CHILD_OPEN_PASS} pid={} project_id={project_id} HMAC_PERSISTED",
        std::process::id()
    );
}

#[test]
#[allow(non_snake_case)]
fn CORE_STORAGE_CROSS_PROCESS_create_then_open_with_fixed_test_key() {
    let root = temp();
    fs::create_dir_all(&root).expect("CORE_STORAGE_CROSS_PROCESS create isolated root");
    let _cleanup = CrossProcessRoot(root.clone());
    fs::write(
        root.join(CROSS_PROCESS_SENTINEL_FILE),
        CROSS_PROCESS_SENTINEL_CONTENT,
    )
    .expect("CORE_STORAGE_CROSS_PROCESS authorize isolated child root");

    let create_output = run_cross_process_child(&root, CROSS_PROCESS_CHILD_CREATE_TEST);
    assert_cross_process_child_passed(&create_output, CROSS_PROCESS_CHILD_CREATE_PASS);

    // `output` waits for the creator to exit. This second Command invocation
    // therefore reopens the files in a fresh process with separately supplied
    // copies of the same fixed test key.
    let open_output = run_cross_process_child(&root, CROSS_PROCESS_CHILD_OPEN_TEST);
    assert_cross_process_child_passed(&open_output, CROSS_PROCESS_CHILD_OPEN_PASS);

    println!("CORE_STORAGE_CROSS_PROCESS PASS create_process_to_open_process HMAC_PERSISTED");
}

#[test]
fn old_unsigned_project_head_shape_remains_deserializable() {
    let old = r#"{
        "schemaVersion":"1.0.0",
        "projectId":"legacy-project",
        "headRevision":0,
        "manifestSha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "previousHeadSha256":null
    }"#;
    let head: ProjectHead = serde_json::from_str(old).unwrap();
    assert_eq!(head.key_id, None);
    assert_eq!(head.previous_head_mac, None);
    assert_eq!(head.head_mac, None);
}

#[test]
fn secure_store_seals_heads_chains_revisions_and_opens_project() {
    let root = temp();
    let store_root = root.join("projects");
    let cas = FsCas::new(root.join("cas"));
    let store = secure_store(&store_root);

    let mut project = create_project(&store, &cas, "Integrity project").unwrap();
    let first = store
        .load_head(&project.identity.project_id.to_string())
        .unwrap()
        .unwrap();
    assert_eq!(first.key_id.as_deref(), Some("dpapi-current-user-v1"));
    assert!(first.previous_head_mac.is_none());
    assert_eq!(first.head_mac.as_deref().map(str::len), Some(64));

    project.revision = 1;
    let second = commit_project(
        &store,
        &cas,
        &project.identity.project_id.to_string(),
        1,
        Some(first.manifest_sha256.clone()),
        &project,
    )
    .unwrap();
    assert_eq!(second.previous_head_mac, first.head_mac);
    assert_eq!(
        store.load_head(&second.project_id).unwrap(),
        Some(second.clone())
    );
    let opened = open_project(&store, &cas, &second.project_id)
        .unwrap()
        .expect("opened integrity project");
    assert_eq!(opened.identity.project_id, project.identity.project_id);
    assert_eq!(opened.revision, project.revision);

    let revisions = store_root.join(&second.project_id).join("revisions");
    assert!(revisions.join("0.head.json").is_file());
    assert!(revisions.join("1.head.json").is_file());
    assert!(
        store_root
            .join("security")
            .join("anchors")
            .join(format!("{}.json", second.project_id))
            .is_file()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn secure_store_fails_closed_on_unsigned_legacy_state() {
    let root = temp();
    let store_root = root.join("projects");
    let cas = FsCas::new(root.join("cas"));
    let legacy = FsProjectStore::new(&store_root);
    let manifest = serde_json::json!({"legacy": true});
    commit_project(&legacy, &cas, "legacy-project", 0, None, &manifest).unwrap();

    let secure = secure_store(&store_root);
    let error = secure.load_head("legacy-project").unwrap_err();
    assert!(
        error.contains("unsigned")
            || error.contains("unanchored")
            || error.contains("integrity key id mismatch")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn revision_only_fixture_is_uncommitted_reusable_and_different_bytes_are_isolated() {
    let root = temp();
    let store_root = root.join("projects");
    let cas = FsCas::new(root.join("cas"));
    let store = secure_store(&store_root);

    let same_manifest = serde_json::json!({"fixture": "revision-only-same"});
    let same_bytes = canonical_bytes(&same_manifest).unwrap();
    let same_revision = revision_path(&store_root, "revision-only-same", 0);
    fs::create_dir_all(same_revision.parent().unwrap()).unwrap();
    fs::write(&same_revision, &same_bytes).unwrap();
    assert_eq!(store.load_head("revision-only-same").unwrap(), None);
    let committed =
        commit_project(&store, &cas, "revision-only-same", 0, None, &same_manifest).unwrap();
    assert_eq!(committed.head_revision, 0);

    let orphan = canonical_bytes(&serde_json::json!({"fixture": "unsigned-orphan"})).unwrap();
    let different_revision = revision_path(&store_root, "revision-only-different", 0);
    fs::create_dir_all(different_revision.parent().unwrap()).unwrap();
    fs::write(&different_revision, &orphan).unwrap();
    let desired = serde_json::json!({"fixture": "committed"});
    commit_project(&store, &cas, "revision-only-different", 0, None, &desired).unwrap();
    let quarantine = store_root
        .join("revision-only-different")
        .join("recovery")
        .join("unsigned-orphan-revisions");
    let isolated = fs::read_dir(quarantine)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(isolated.len(), 1);
    assert_eq!(fs::read(&isolated[0]).unwrap(), orphan);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn sidecar_only_genesis_fixture_rolls_both_projections_forward() {
    let root = temp();
    let store_root = root.join("projects");
    let cas = FsCas::new(root.join("cas"));
    let store = secure_store(&store_root);
    let committed = commit_project(
        &store,
        &cas,
        "sidecar-only",
        0,
        None,
        &serde_json::json!({"revision": 0}),
    )
    .unwrap();

    fs::remove_file(head_path(&store_root, "sidecar-only")).unwrap();
    fs::remove_file(anchor_path(&store_root, "sidecar-only")).unwrap();
    let recovered = store
        .load_head("sidecar-only")
        .unwrap()
        .expect("valid genesis sidecar is recoverable");
    assert_eq!(recovered, committed);
    assert!(head_path(&store_root, "sidecar-only").is_file());
    assert!(anchor_path(&store_root, "sidecar-only").is_file());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn signed_successor_fixture_advances_two_old_projections() {
    let root = temp();
    let store_root = root.join("projects");
    let cas = FsCas::new(root.join("cas"));
    let store = secure_store(&store_root);
    let first = commit_project(
        &store,
        &cas,
        "successor-project",
        0,
        None,
        &serde_json::json!({"revision": 0}),
    )
    .unwrap();
    let old_head = fs::read(head_path(&store_root, "successor-project")).unwrap();
    let old_anchor = fs::read(anchor_path(&store_root, "successor-project")).unwrap();
    let second = commit_project(
        &store,
        &cas,
        "successor-project",
        1,
        Some(first.manifest_sha256),
        &serde_json::json!({"revision": 1}),
    )
    .unwrap();

    fs::write(head_path(&store_root, "successor-project"), old_head).unwrap();
    fs::write(anchor_path(&store_root, "successor-project"), old_anchor).unwrap();
    let recovered = store
        .load_head("successor-project")
        .unwrap()
        .expect("signed successor rolls both projections forward");
    assert_eq!(recovered, second);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn tampered_successor_fixture_never_advances_head_or_anchor() {
    let root = temp();
    let store_root = root.join("projects");
    let cas = FsCas::new(root.join("cas"));
    let store = secure_store(&store_root);
    let first = commit_project(
        &store,
        &cas,
        "tampered-successor",
        0,
        None,
        &serde_json::json!({"revision": 0}),
    )
    .unwrap();
    let old_head = fs::read(head_path(&store_root, "tampered-successor")).unwrap();
    let old_anchor = fs::read(anchor_path(&store_root, "tampered-successor")).unwrap();
    commit_project(
        &store,
        &cas,
        "tampered-successor",
        1,
        Some(first.manifest_sha256),
        &serde_json::json!({"revision": 1}),
    )
    .unwrap();
    fs::write(head_path(&store_root, "tampered-successor"), &old_head).unwrap();
    fs::write(anchor_path(&store_root, "tampered-successor"), &old_anchor).unwrap();
    let sidecar = store_root
        .join("tampered-successor")
        .join("revisions")
        .join("1.head.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&sidecar).unwrap()).unwrap();
    value["manifestSha256"] = Value::String("e".repeat(64));
    fs::write(sidecar, serde_json::to_vec(&value).unwrap()).unwrap();

    let error = store.load_head("tampered-successor").unwrap_err();
    assert!(error.contains("MAC") || error.contains("signed revision"));
    assert_eq!(
        fs::read(head_path(&store_root, "tampered-successor")).unwrap(),
        old_head
    );
    assert_eq!(
        fs::read(anchor_path(&store_root, "tampered-successor")).unwrap(),
        old_anchor
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn head_current_anchor_old_fixture_rolls_anchor_forward() {
    let root = temp();
    let store_root = root.join("projects");
    let cas = FsCas::new(root.join("cas"));
    let store = secure_store(&store_root);
    let first = commit_project(
        &store,
        &cas,
        "head-current",
        0,
        None,
        &serde_json::json!({"revision": 0}),
    )
    .unwrap();
    let old_anchor = fs::read(anchor_path(&store_root, "head-current")).unwrap();
    let second = commit_project(
        &store,
        &cas,
        "head-current",
        1,
        Some(first.manifest_sha256),
        &serde_json::json!({"revision": 1}),
    )
    .unwrap();

    fs::write(anchor_path(&store_root, "head-current"), old_anchor).unwrap();
    let recovered = store
        .load_head("head-current")
        .unwrap()
        .expect("current signed head rolls anchor forward");
    assert_eq!(recovered, second);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn secure_store_rolls_head_forward_to_the_authenticated_high_water_anchor() {
    let root = temp();
    let store_root = root.join("projects");
    let cas = FsCas::new(root.join("cas"));
    let store = secure_store(&store_root);
    let first_manifest = serde_json::json!({"revision": 0});
    let first = commit_project(&store, &cas, "rollback-project", 0, None, &first_manifest).unwrap();
    let old_head = fs::read(
        store_root
            .join("rollback-project")
            .join("revisions")
            .join("0.head.json"),
    )
    .unwrap();
    let second_manifest = serde_json::json!({"revision": 1});
    let second = commit_project(
        &store,
        &cas,
        "rollback-project",
        1,
        Some(first.manifest_sha256),
        &second_manifest,
    )
    .unwrap();

    fs::write(head_path(&store_root, "rollback-project"), old_head).unwrap();
    let recovered = store
        .load_head("rollback-project")
        .unwrap()
        .expect("anchor rolls the stale head forward");
    assert_eq!(recovered, second);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn secure_store_detects_sidecar_tampering_and_rejects_cas_forks() {
    let root = temp();
    let store_root = root.join("projects");
    let cas = FsCas::new(root.join("cas"));
    let store = secure_store(&store_root);
    let first_manifest = serde_json::json!({"revision": 0});
    let first = commit_project(&store, &cas, "fork-project", 0, None, &first_manifest).unwrap();

    let skipped = serde_json::json!({"revision": 2});
    assert!(
        commit_project(
            &store,
            &cas,
            "fork-project",
            2,
            Some(first.manifest_sha256.clone()),
            &skipped,
        )
        .unwrap_err()
        .contains("compare-and-swap")
    );
    let wrong_previous = serde_json::json!({"revision": 1});
    assert!(
        commit_project(
            &store,
            &cas,
            "fork-project",
            1,
            Some("f".repeat(64)),
            &wrong_previous,
        )
        .unwrap_err()
        .contains("compare-and-swap")
    );

    let sidecar = store_root
        .join("fork-project")
        .join("revisions")
        .join("0.head.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&sidecar).unwrap()).unwrap();
    value["manifestSha256"] = Value::String("e".repeat(64));
    fs::write(&sidecar, serde_json::to_vec(&value).unwrap()).unwrap();
    let error = store.load_head("fork-project").unwrap_err();
    assert!(error.contains("signed revision") || error.contains("MAC"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn secure_store_rejects_wrong_key_and_tampered_anchor() {
    let root = temp();
    let store_root = root.join("projects");
    let cas = FsCas::new(root.join("cas"));
    let store = secure_store(&store_root);
    commit_project(
        &store,
        &cas,
        "anchor-project",
        0,
        None,
        &serde_json::json!({"revision": 0}),
    )
    .unwrap();

    let wrong_key =
        FsProjectStore::new_with_integrity_key(&store_root, "dpapi-current-user-v1", [0x33; 32])
            .unwrap();
    assert!(wrong_key.load_head("anchor-project").is_err());

    let anchor_path = store_root
        .join("security")
        .join("anchors")
        .join("anchor-project.json");
    let mut anchor: Value = serde_json::from_slice(&fs::read(&anchor_path).unwrap()).unwrap();
    anchor["highestRevision"] = Value::from(99_u64);
    fs::write(&anchor_path, serde_json::to_vec(&anchor).unwrap()).unwrap();
    assert!(
        store
            .load_head("anchor-project")
            .unwrap_err()
            .contains("anchor")
    );
    fs::remove_dir_all(root).unwrap();
}

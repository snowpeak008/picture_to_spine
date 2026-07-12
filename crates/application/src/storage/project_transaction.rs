use crate::ports::{CasStore, ProjectStore};
use f2s_domain::{canonical::canonical_bytes, storage::ProjectHead};
use serde::Serialize;
pub fn commit_project<P: ProjectStore, C: CasStore, T: Serialize>(
    projects: &P,
    cas: &C,
    project_id: &str,
    revision: u64,
    previous: Option<String>,
    manifest: &T,
) -> Result<ProjectHead, String> {
    let bytes = canonical_bytes(manifest).map_err(|e| e.to_string())?;
    let reference = cas.put("application/json", &bytes)?;
    let head = ProjectHead {
        schema_version: "1.0.0".into(),
        project_id: project_id.into(),
        head_revision: revision,
        manifest_sha256: reference.sha256,
        previous_head_sha256: previous,
        key_id: None,
        previous_head_mac: None,
        head_mac: None,
    };
    projects.commit_head(&head, &bytes)
}

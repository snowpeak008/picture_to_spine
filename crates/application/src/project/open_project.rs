use crate::ports::{CasStore, ProjectStore};
use f2s_domain::{project::ProjectManifest, storage::CasRef};
pub fn open_project<P: ProjectStore, C: CasStore>(
    projects: &P,
    cas: &C,
    id: &str,
) -> Result<Option<ProjectManifest>, String> {
    let Some(head) = projects.load_head(id)? else {
        return Ok(None);
    };
    let bytes = cas.get(&CasRef {
        sha256: head.manifest_sha256,
        byte_length: 0,
        media_type: "application/json".into(),
    })?;
    let mut project: ProjectManifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    match project.schema_version.as_str() {
        "1.1.0" | "1.2.0" | "1.3.0" => project.schema_version = "1.4.0".into(),
        "1.4.0" => {}
        version => return Err(format!("unsupported project schema version: {version}")),
    }
    project.validate_cross_aggregate()?;
    Ok(Some(project))
}

use f2s_domain::{
    import::SourceArtifact,
    master::{MasterCandidate, StyleSpec},
};
use uuid::Uuid;
pub fn create_master(
    source: &SourceArtifact,
    style_spec: StyleSpec,
) -> Result<MasterCandidate, String> {
    style_spec.validate()?;
    Ok(MasterCandidate {
        master_id: Uuid::new_v4().to_string(),
        source_artifact_id: source.artifact_id.clone(),
        candidate_revision: 0,
        source_sha256: source.sha256.clone(),
        style_spec,
        approval_state: "PENDING".into(),
        supersedes: None,
    })
}

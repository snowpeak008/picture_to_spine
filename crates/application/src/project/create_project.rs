use crate::{
    ports::{CasStore, ProjectStore},
    storage::commit_project,
};
use f2s_domain::project::{ProjectIdentity, ProjectManifest};
pub fn create_project<P: ProjectStore, C: CasStore>(
    projects: &P,
    cas: &C,
    name: &str,
) -> Result<ProjectManifest, String> {
    let manifest = ProjectManifest::new(ProjectIdentity::new(name)?);
    commit_project(
        projects,
        cas,
        &manifest.identity.project_id.to_string(),
        0,
        None,
        &manifest,
    )?;
    Ok(manifest)
}

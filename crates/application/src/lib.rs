#![forbid(unsafe_code)]

use f2s_domain::{DomainError, ProjectSummary, TimeBase};
use thiserror::Error;
use uuid::Uuid;

pub mod animation;
pub mod approvals;
pub mod commands;
pub mod export;
pub mod governance;
pub mod import;
pub mod jobs;
pub mod layers;
pub mod master;
pub mod motion;
pub mod observability;
pub mod ports;
pub mod project;
pub mod remote_gpu;
pub mod rig;
pub mod storage;

pub trait ProjectRepository: Send + Sync {
    fn create(&self, project: &ProjectSummary) -> Result<(), String>;
}

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("项目存储失败：{0}")]
    Repository(String),
}

pub struct CreateProject<'a, R: ProjectRepository> {
    repository: &'a R,
}
impl<'a, R: ProjectRepository> CreateProject<'a, R> {
    pub fn new(repository: &'a R) -> Self {
        Self { repository }
    }
    pub fn execute(&self, name: &str) -> Result<ProjectSummary, ApplicationError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(DomainError::EmptyProjectName.into());
        }
        let project = ProjectSummary {
            project_id: Uuid::new_v4(),
            name: name.to_owned(),
            revision: 0,
            time_base: TimeBase::default(),
        };
        self.repository
            .create(&project)
            .map_err(ApplicationError::Repository)?;
        Ok(project)
    }
}

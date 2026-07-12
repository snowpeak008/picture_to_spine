// Unsafe code remains denied throughout the crate. The only exception is the
// narrowly scoped Windows Credential Manager FFI module declared in `safety`.
#![deny(unsafe_code)]

use f2s_application::ProjectRepository;
use f2s_domain::ProjectSummary;
use std::sync::Mutex;

pub mod export;
pub mod image;
pub mod ipc;
pub mod observability;
pub mod safety;
pub mod storage;

#[derive(Default)]
pub struct InMemoryProjectRepository {
    projects: Mutex<Vec<ProjectSummary>>,
}
impl ProjectRepository for InMemoryProjectRepository {
    fn create(&self, project: &ProjectSummary) -> Result<(), String> {
        self.projects
            .lock()
            .map_err(|_| "存储锁已损坏".to_owned())?
            .push(project.clone());
        Ok(())
    }
}
impl InMemoryProjectRepository {
    pub fn count(&self) -> usize {
        self.projects
            .lock()
            .map(|items| items.len())
            .unwrap_or_default()
    }
}

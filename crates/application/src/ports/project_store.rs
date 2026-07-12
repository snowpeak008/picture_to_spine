use f2s_domain::storage::ProjectHead;
pub trait ProjectStore {
    fn load_head(&self, project_id: &str) -> Result<Option<ProjectHead>, String>;
    /// Compare-and-swap the current project head and return the head actually
    /// persisted by the store. Production stores may add an integrity seal.
    fn commit_head(&self, head: &ProjectHead, manifest: &[u8]) -> Result<ProjectHead, String>;
}

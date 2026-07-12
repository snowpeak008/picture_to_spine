use f2s_domain::storage::ClockCheckpoint;
pub trait ClockStore {
    fn current(&self) -> Result<Option<ClockCheckpoint>, String>;
    fn append(&self, checkpoint: &ClockCheckpoint) -> Result<(), String>;
}

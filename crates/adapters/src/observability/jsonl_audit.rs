use f2s_application::ports::AuditSink;
use f2s_domain::observability::AuditEvent;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
pub struct JsonlAuditSink {
    path: PathBuf,
}
impl JsonlAuditSink {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}
impl AuditSink for JsonlAuditSink {
    fn append(&self, event: &AuditEvent) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec(event).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.write_all(b"\n").map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())
    }
}

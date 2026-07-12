use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRecord {
    pub sequence: u64,
    pub operation: String,
    pub target: String,
    pub sha256: String,
    pub committed: bool,
}
pub fn append(path: &Path, record: &JournalRecord) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec(record).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}
pub fn read_all(path: &Path) -> Result<Vec<JournalRecord>, String> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    text.lines()
        .filter(|v| !v.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(|e| e.to_string()))
        .collect()
}

use super::ntfs_atomic::write_atomic;
use std::{collections::BTreeMap, fs, path::Path};
pub fn write_index(path: &Path, index: &BTreeMap<String, String>) -> Result<(), String> {
    write_atomic(path, &serde_json::to_vec(index).map_err(|e| e.to_string())?)
}
pub fn read_index(path: &Path) -> Result<BTreeMap<String, String>, String> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

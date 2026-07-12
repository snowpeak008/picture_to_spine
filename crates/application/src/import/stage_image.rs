use std::{fs, path::Path};
pub fn read_stable(path: &Path, max_bytes: u64) -> Result<Vec<u8>, String> {
    let before = fs::metadata(path).map_err(|e| e.to_string())?;
    if before.len() > max_bytes {
        return Err("source exceeds absolute byte limit".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let after = fs::metadata(path).map_err(|e| e.to_string())?;
    if before.len() != after.len() || bytes.len() as u64 != after.len() {
        return Err("source changed during staging".into());
    }
    Ok(bytes)
}

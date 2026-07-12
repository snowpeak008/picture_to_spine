use super::ntfs_atomic::write_atomic;
use std::{fs, path::Path};
pub fn migrate_copy(
    source: &Path,
    target: &Path,
    transform: impl FnOnce(Vec<u8>) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    if target.exists() {
        return Err("migration target exists".into());
    }
    let input = fs::read(source).map_err(|e| e.to_string())?;
    let output = transform(input)?;
    write_atomic(target, &output)
}

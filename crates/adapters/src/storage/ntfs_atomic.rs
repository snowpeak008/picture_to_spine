#[cfg(not(windows))]
use std::fs::File;
use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

fn backup_path(path: &Path) -> Result<PathBuf, String> {
    let file_name = path.file_name().ok_or("target has no file name")?;
    let mut backup_name = OsString::from(".");
    backup_name.push(file_name);
    backup_name.push(".f2s-backup");
    Ok(path.with_file_name(backup_name))
}

/// Repairs the only ambiguous window in the portable replace algorithm: the
/// old target was moved aside, but the fully synced temporary file was not yet
/// installed. The deterministic backup name makes this recovery possible on
/// the next read or write instead of leaving an undiscoverable random `.bak`.
pub(crate) fn recover_atomic_target(path: &Path) -> Result<(), String> {
    let backup = backup_path(path)?;
    let target_exists = path.try_exists().map_err(|error| error.to_string())?;
    let backup_exists = backup.try_exists().map_err(|error| error.to_string())?;
    match (target_exists, backup_exists) {
        (false, true) => fs::rename(&backup, path).map_err(|error| {
            format!(
                "cannot restore atomic backup {} -> {}: {error}",
                backup.display(),
                path.display()
            )
        }),
        (true, true) => fs::remove_file(&backup).map_err(|error| {
            format!(
                "cannot retire completed atomic backup {}: {error}",
                backup.display()
            )
        }),
        _ => Ok(()),
    }
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("target has no parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    recover_atomic_target(path)?;
    let temp = parent.join(format!(".f2s-{}.tmp", Uuid::new_v4()));
    let backup = backup_path(path)?;
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        if path.try_exists().map_err(|e| e.to_string())? {
            fs::rename(path, &backup).map_err(|e| e.to_string())?;
        }
        if let Err(error) = fs::rename(&temp, path) {
            if backup.try_exists().unwrap_or(false) {
                let _ = fs::rename(&backup, path);
            }
            return Err(error.to_string());
        }
        if backup.try_exists().map_err(|e| e.to_string())? {
            fs::remove_file(&backup).map_err(|e| e.to_string())?;
        }
        #[cfg(not(windows))]
        File::open(parent)
            .and_then(|v| v.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() && temp.try_exists().unwrap_or(false) {
        let _ = fs::remove_file(temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{backup_path, recover_atomic_target, write_atomic};
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn missing_target_is_restored_from_deterministic_backup() {
        let root = std::env::temp_dir().join(format!("f2s-atomic-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let target = root.join("head.json");
        let backup = backup_path(&target).unwrap();
        fs::write(&backup, b"old").unwrap();

        recover_atomic_target(&target).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"old");
        assert!(!backup.exists());

        write_atomic(&target, b"new").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"new");
        assert!(!backup.exists());
        fs::remove_dir_all(root).unwrap();
    }
}

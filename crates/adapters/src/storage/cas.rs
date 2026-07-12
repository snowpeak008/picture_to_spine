use super::ntfs_atomic::write_atomic;
use f2s_application::ports::CasStore;
use f2s_domain::storage::CasRef;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
pub struct FsCas {
    root: PathBuf,
}
impl FsCas {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }
    fn path(&self, sha: &str) -> Result<PathBuf, String> {
        if sha.len() != 64
            || !sha
                .bytes()
                .all(|value| value.is_ascii_hexdigit() && !value.is_ascii_uppercase())
        {
            return Err("invalid CAS sha256".into());
        }
        Ok(self.root.join(&sha[0..2]).join(sha))
    }
}
impl CasStore for FsCas {
    fn put(&self, media_type: &str, bytes: &[u8]) -> Result<CasRef, String> {
        let hash: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let path = self.path(&hash)?;
        if !path.exists() {
            write_atomic(&path, bytes)?
        }
        Ok(CasRef {
            sha256: hash,
            byte_length: bytes.len() as u64,
            media_type: media_type.into(),
        })
    }
    fn get(&self, reference: &CasRef) -> Result<Vec<u8>, String> {
        let bytes = fs::read(self.path(&reference.sha256)?).map_err(|e| e.to_string())?;
        let actual: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if actual != reference.sha256 {
            return Err("CAS hash mismatch".into());
        }
        Ok(bytes)
    }
}

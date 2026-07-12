use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

pub fn materialize_approved_png(
    bytes: &[u8],
    expected_sha256: &str,
    destination: &Path,
) -> Result<String, String> {
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return Err("approved attachment is not PNG".into());
    }
    let hash = format!("{:x}", Sha256::digest(bytes));
    if hash != expected_sha256.to_ascii_lowercase() {
        return Err("approved PNG hash mismatch".into());
    }
    if destination
        .extension()
        .and_then(|v| v.to_str())
        .map(|v| v.eq_ignore_ascii_case("png"))
        != Some(true)
    {
        return Err("built-in attachment output must be .png".into());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?
    }
    let temp = destination.with_extension("png.f2s-tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    if destination.exists() {
        let _ = fs::remove_file(&temp);
        return Err("immutable PNG destination already exists".into());
    }
    fs::rename(&temp, destination).map_err(|e| e.to_string())?;
    Ok(hash)
}

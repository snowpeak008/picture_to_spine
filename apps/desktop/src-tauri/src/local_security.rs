use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use windows::Win32::{
    Foundation::{HLOCAL, LocalFree},
    Security::Cryptography::{
        BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom, CRYPT_INTEGER_BLOB,
        CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    },
};

const INTEGRITY_KEY_FILE: &str = "project-integrity-key.dpapi";
const INTEGRITY_KEY_BYTES: usize = 32;
const MAX_DPAPI_BLOB_BYTES: usize = 16 * 1024;

/// Loads the project integrity key under the current Windows account, or
/// creates it once using the system CSPRNG. Only the DPAPI ciphertext is ever
/// persisted; production project stores receive the 256-bit plaintext in
/// memory and fail closed if it cannot be recovered.
pub fn load_or_create_project_integrity_key(
    app_data_root: &Path,
) -> Result<(String, [u8; INTEGRITY_KEY_BYTES]), String> {
    let security_root = app_data_root.join("security");
    fs::create_dir_all(&security_root)
        .map_err(|error| format!("cannot create local security directory: {error}"))?;
    reject_symlink(&security_root)?;
    let path = security_root.join(INTEGRITY_KEY_FILE);

    let key = if path.exists() {
        load_key(&path)?
    } else {
        create_key_once(&path)?
    };
    let key_fingerprint = Sha256::digest(key);
    let fingerprint_hex = key_fingerprint[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let key_id = format!("dpapi-current-user-v1-{fingerprint_hex}");
    Ok((key_id, key))
}

fn reject_symlink(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect local security path: {error}"))?;
    if metadata.file_type().is_symlink() {
        return Err("local security path must not be a symbolic link".into());
    }
    Ok(())
}

fn load_key(path: &Path) -> Result<[u8; INTEGRITY_KEY_BYTES], String> {
    reject_symlink(path)?;
    let metadata =
        fs::metadata(path).map_err(|error| format!("cannot inspect DPAPI key file: {error}"))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_DPAPI_BLOB_BYTES as u64 {
        return Err("DPAPI key file has an invalid shape".into());
    }
    let ciphertext =
        fs::read(path).map_err(|error| format!("cannot read DPAPI key file: {error}"))?;
    let mut plaintext = dpapi_unprotect(&ciphertext)?;
    if plaintext.len() != INTEGRITY_KEY_BYTES {
        plaintext.fill(0);
        return Err("DPAPI project integrity key has an invalid length".into());
    }
    let mut key = [0u8; INTEGRITY_KEY_BYTES];
    key.copy_from_slice(&plaintext);
    plaintext.fill(0);
    Ok(key)
}

fn create_key_once(path: &Path) -> Result<[u8; INTEGRITY_KEY_BYTES], String> {
    let mut key = [0u8; INTEGRITY_KEY_BYTES];
    let status = unsafe { BCryptGenRandom(None, &mut key, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
    if status.0 < 0 {
        return Err(format!(
            "Windows system random generator failed with NTSTATUS 0x{:08x}",
            status.0 as u32
        ));
    }
    let ciphertext = match dpapi_protect(&key) {
        Ok(value) => value,
        Err(error) => {
            key.fill(0);
            return Err(error);
        }
    };

    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            if let Err(error) = file.write_all(&ciphertext).and_then(|_| file.sync_all()) {
                key.fill(0);
                let _ = fs::remove_file(path);
                return Err(format!("cannot persist DPAPI key file: {error}"));
            }
            Ok(key)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            key.fill(0);
            load_key(path)
        }
        Err(error) => {
            key.fill(0);
            Err(format!("cannot create DPAPI key file: {error}"))
        }
    }
}

fn dpapi_protect(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: plaintext
            .len()
            .try_into()
            .map_err(|_| "DPAPI input too large")?,
        pbData: plaintext.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptProtectData(
            &input,
            windows::core::w!("FlashToSpine Project Integrity Key"),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|error| format!("DPAPI protection failed: {error}"))?;
    }
    copy_and_free_blob(output, false)
}

fn dpapi_unprotect(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: ciphertext
            .len()
            .try_into()
            .map_err(|_| "DPAPI input too large")?,
        pbData: ciphertext.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|error| format!("DPAPI recovery failed for the current Windows user: {error}"))?;
    }
    copy_and_free_blob(output, true)
}

fn copy_and_free_blob(
    output: CRYPT_INTEGER_BLOB,
    clear_before_free: bool,
) -> Result<Vec<u8>, String> {
    let length = output.cbData as usize;
    if output.pbData.is_null() || length == 0 || length > MAX_DPAPI_BLOB_BYTES {
        if !output.pbData.is_null() {
            unsafe {
                let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
            }
        }
        return Err("DPAPI returned an invalid output blob".into());
    }
    let value = unsafe { std::slice::from_raw_parts(output.pbData, length).to_vec() };
    unsafe {
        if clear_before_free {
            std::ptr::write_bytes(output.pbData, 0, length);
        }
        let result = LocalFree(Some(HLOCAL(output.pbData.cast())));
        if !result.is_invalid() {
            return Err("DPAPI output memory could not be released".into());
        }
    }
    Ok(value)
}

#[allow(dead_code)]
fn key_path(app_data_root: &Path) -> PathBuf {
    app_data_root.join("security").join(INTEGRITY_KEY_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpapi_key_is_stable_and_never_persisted_as_plaintext() {
        let root =
            std::env::temp_dir().join(format!("f2s-dpapi-test-{}", uuid::Uuid::new_v4().simple()));
        let (first_id, first_key) = load_or_create_project_integrity_key(&root).unwrap();
        let ciphertext = fs::read(key_path(&root)).unwrap();
        assert_ne!(ciphertext, first_key);

        let (second_id, second_key) = load_or_create_project_integrity_key(&root).unwrap();
        assert_eq!(first_id, second_id);
        assert_eq!(first_key, second_key);

        let _ = fs::remove_dir_all(root);
    }
}

//! Current-user Windows Credential Manager storage for private remote GPU secrets.
//!
//! This module deliberately exposes no enumeration API and accepts only the
//! product's exact private-remote target namespace. `CRED_PERSIST_LOCAL_MACHINE`
//! means that the generic credential persists across this user's logon sessions
//! on this machine; it does not make the credential readable by other users.

use std::{
    error::Error,
    ffi::c_void,
    fmt, ptr,
    sync::atomic::{Ordering, compiler_fence},
};
use windows::{
    Win32::{
        Foundation::ERROR_NOT_FOUND,
        Security::Credentials::{
            CRED_MAX_CREDENTIAL_BLOB_SIZE, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
            CREDENTIALW, CredDeleteW, CredFree, CredReadW, CredWriteW,
        },
    },
    core::{HRESULT, PCWSTR, PWSTR},
};

pub const REMOTE_GPU_CREDENTIAL_TARGET_PREFIX: &str = "FlashToSpine/RemoteGpu/";
pub const MAX_REMOTE_GPU_PROFILE_ID_BYTES: usize = 96;

/// An owned secret copied out of Credential Manager.
///
/// The type intentionally implements neither `Debug` nor serialization traits.
/// Callers must explicitly borrow the bytes and should keep that borrow short.
pub struct SecretBytes {
    bytes: Vec<u8>,
}

impl SecretBytes {
    pub fn expose_secret(&self) -> &[u8] {
        &self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl Drop for SecretBytes {
    fn drop(&mut self) {
        zeroize_slice(&mut self.bytes);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialManagerErrorKind {
    InvalidTarget,
    InvalidSecret,
    WindowsApi,
    CorruptStoredCredential,
}

/// A redacted error. It carries only a stable category and an optional HRESULT;
/// target names and credential bytes are never included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialManagerError {
    kind: CredentialManagerErrorKind,
    hresult: Option<i32>,
}

impl CredentialManagerError {
    fn policy(kind: CredentialManagerErrorKind) -> Self {
        Self {
            kind,
            hresult: None,
        }
    }

    fn windows(error: windows::core::Error) -> Self {
        Self {
            kind: CredentialManagerErrorKind::WindowsApi,
            hresult: Some(error.code().0),
        }
    }

    pub fn kind(&self) -> CredentialManagerErrorKind {
        self.kind
    }

    pub fn hresult(&self) -> Option<i32> {
        self.hresult
    }
}

impl fmt::Display for CredentialManagerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.hresult {
            Some(code) => write!(
                formatter,
                "Windows Credential Manager operation failed (HRESULT 0x{:08x})",
                code as u32
            ),
            None => write!(
                formatter,
                "Windows Credential Manager policy rejected the input"
            ),
        }
    }
}

impl Error for CredentialManagerError {}

/// Minimal generic-credential facade. It never enumerates credentials and never
/// persists a secret anywhere other than the current user's Windows vault.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsCredentialManager;

impl WindowsCredentialManager {
    pub fn new() -> Self {
        Self
    }

    /// Stores or replaces a generic credential for an exact private-remote target.
    pub fn write(&self, target: &str, secret: &[u8]) -> Result<(), CredentialManagerError> {
        validate_remote_gpu_credential_target(target)?;
        validate_secret(secret)?;

        let mut target_wide = nul_terminated_wide(target);
        let mut username_wide = nul_terminated_wide("FlashToSpine");
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(target_wide.as_mut_ptr()),
            CredentialBlobSize: secret.len() as u32,
            // CredWriteW treats this as input even though the Win32 structure uses
            // a mutable pointer for historical API compatibility.
            CredentialBlob: secret.as_ptr().cast_mut(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            UserName: PWSTR(username_wide.as_mut_ptr()),
            ..Default::default()
        };

        // SAFETY: every pointer in `credential` refers to a live, correctly sized
        // buffer for the duration of the call. Both strings are NUL terminated,
        // and the blob size was checked against the Win32 maximum above.
        unsafe { CredWriteW(&credential, 0) }.map_err(CredentialManagerError::windows)
    }

    /// Reads a private-remote credential. A missing target is not an error.
    pub fn read(&self, target: &str) -> Result<Option<SecretBytes>, CredentialManagerError> {
        validate_remote_gpu_credential_target(target)?;
        let target_wide = nul_terminated_wide(target);
        let mut raw = ptr::null_mut::<CREDENTIALW>();

        // SAFETY: `target_wide` is NUL terminated and `raw` is a valid out pointer.
        match unsafe {
            CredReadW(
                PCWSTR(target_wide.as_ptr()),
                CRED_TYPE_GENERIC,
                None,
                &mut raw,
            )
        } {
            Ok(()) => {}
            Err(error) if is_not_found(&error) => return Ok(None),
            Err(error) => return Err(CredentialManagerError::windows(error)),
        }

        if raw.is_null() {
            return Err(CredentialManagerError::policy(
                CredentialManagerErrorKind::CorruptStoredCredential,
            ));
        }
        let allocation = CredentialAllocation(raw);

        // SAFETY: a successful CredReadW returns a CREDENTIALW allocation valid
        // until CredFree. `allocation` owns that lifetime for this scope.
        let credential = unsafe { &*allocation.0 };
        let size = credential.CredentialBlobSize as usize;
        if credential.Type != CRED_TYPE_GENERIC
            || size == 0
            || size > CRED_MAX_CREDENTIAL_BLOB_SIZE as usize
            || credential.CredentialBlob.is_null()
        {
            return Err(CredentialManagerError::policy(
                CredentialManagerErrorKind::CorruptStoredCredential,
            ));
        }

        // SAFETY: CredReadW guarantees that CredentialBlob points to exactly
        // CredentialBlobSize readable bytes inside its returned allocation.
        let secret = SecretBytes {
            bytes: unsafe { std::slice::from_raw_parts(credential.CredentialBlob, size).to_vec() },
        };
        validate_secret(secret.expose_secret()).map_err(|_| {
            CredentialManagerError::policy(CredentialManagerErrorKind::CorruptStoredCredential)
        })?;
        Ok(Some(secret))
    }

    /// Deletes a private-remote credential. Returns `false` when it did not exist.
    pub fn delete(&self, target: &str) -> Result<bool, CredentialManagerError> {
        validate_remote_gpu_credential_target(target)?;
        let target_wide = nul_terminated_wide(target);

        // SAFETY: `target_wide` is a live NUL-terminated string for this call.
        match unsafe { CredDeleteW(PCWSTR(target_wide.as_ptr()), CRED_TYPE_GENERIC, None) } {
            Ok(()) => Ok(true),
            Err(error) if is_not_found(&error) => Ok(false),
            Err(error) => Err(CredentialManagerError::windows(error)),
        }
    }
}

/// Validates the complete target reference used by remote GPU profiles.
pub fn validate_remote_gpu_credential_target(target: &str) -> Result<(), CredentialManagerError> {
    let profile_id = target
        .strip_prefix(REMOTE_GPU_CREDENTIAL_TARGET_PREFIX)
        .ok_or_else(|| CredentialManagerError::policy(CredentialManagerErrorKind::InvalidTarget))?;
    if !(3..=MAX_REMOTE_GPU_PROFILE_ID_BYTES).contains(&profile_id.len())
        || !profile_id.is_ascii()
        || profile_id.starts_with('.')
        || profile_id.ends_with('.')
        || profile_id.contains("..")
        || !profile_id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
    {
        return Err(CredentialManagerError::policy(
            CredentialManagerErrorKind::InvalidTarget,
        ));
    }
    Ok(())
}

fn validate_secret(secret: &[u8]) -> Result<(), CredentialManagerError> {
    if secret.is_empty()
        || secret.len() > CRED_MAX_CREDENTIAL_BLOB_SIZE as usize
        || secret.contains(&0)
    {
        Err(CredentialManagerError::policy(
            CredentialManagerErrorKind::InvalidSecret,
        ))
    } else {
        Ok(())
    }
}

fn nul_terminated_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn is_not_found(error: &windows::core::Error) -> bool {
    error.code() == HRESULT::from_win32(ERROR_NOT_FOUND.0)
}

struct CredentialAllocation(*mut CREDENTIALW);

impl Drop for CredentialAllocation {
    fn drop(&mut self) {
        if self.0.is_null() {
            return;
        }
        // SAFETY: this pointer came from a successful CredReadW call and is owned
        // by this guard. Wiping the documented blob range happens before CredFree.
        unsafe {
            let credential = &mut *self.0;
            let blob_size = credential.CredentialBlobSize as usize;
            if !credential.CredentialBlob.is_null()
                && blob_size <= CRED_MAX_CREDENTIAL_BLOB_SIZE as usize
            {
                zeroize_raw(credential.CredentialBlob, blob_size);
            }
            CredFree(self.0.cast::<c_void>());
        }
    }
}

fn zeroize_slice(bytes: &mut [u8]) {
    // SAFETY: the pointer and length originate from this exclusive mutable slice.
    unsafe { zeroize_raw(bytes.as_mut_ptr(), bytes.len()) }
}

unsafe fn zeroize_raw(pointer: *mut u8, length: usize) {
    for offset in 0..length {
        // SAFETY: the caller guarantees a writable range of `length` bytes.
        unsafe { pointer.add(offset).write_volatile(0) };
    }
    compiler_fence(Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_policy_is_exact() {
        assert!(
            validate_remote_gpu_credential_target("FlashToSpine/RemoteGpu/studio-gpu-01").is_ok()
        );
        for invalid in [
            "",
            "FlashToSpine/RemoteGpu/",
            "FlashToSpine/RemoteGpu/UPPER",
            "FlashToSpine/RemoteGpu/..",
            "FlashToSpine/RemoteGpu/.hidden",
            "FlashToSpine/Other/studio-gpu-01",
            "FlashToSpine/RemoteGpu/a\0b",
        ] {
            assert!(validate_remote_gpu_credential_target(invalid).is_err());
        }
        let too_long = format!(
            "{REMOTE_GPU_CREDENTIAL_TARGET_PREFIX}{}",
            "a".repeat(MAX_REMOTE_GPU_PROFILE_ID_BYTES + 1)
        );
        assert!(validate_remote_gpu_credential_target(&too_long).is_err());
    }

    #[test]
    fn secret_policy_rejects_empty_nul_and_over_limit() {
        assert!(validate_secret(b"token").is_ok());
        assert!(validate_secret(b"").is_err());
        assert!(validate_secret(b"a\0b").is_err());
        assert!(validate_secret(&vec![b'x'; CRED_MAX_CREDENTIAL_BLOB_SIZE as usize + 1]).is_err());
    }
}

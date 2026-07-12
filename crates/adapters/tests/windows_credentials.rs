#![cfg(windows)]

use f2s_adapters::safety::windows_credentials::{
    CredentialManagerErrorKind, REMOTE_GPU_CREDENTIAL_TARGET_PREFIX, WindowsCredentialManager,
};
use uuid::Uuid;

struct CredentialCleanup {
    manager: WindowsCredentialManager,
    target: String,
}

impl Drop for CredentialCleanup {
    fn drop(&mut self) {
        let _ = self.manager.delete(&self.target);
    }
}

fn unique_target() -> String {
    format!(
        "{REMOTE_GPU_CREDENTIAL_TARGET_PREFIX}test-{}",
        Uuid::new_v4().simple()
    )
}

#[test]
fn generic_credential_round_trip_and_delete_are_current_user_local_machine() {
    let manager = WindowsCredentialManager::new();
    let target = unique_target();
    let _cleanup = CredentialCleanup {
        manager,
        target: target.clone(),
    };
    let _ = manager.delete(&target).unwrap();
    assert!(manager.read(&target).unwrap().is_none());

    manager.write(&target, b"temporary-test-token").unwrap();
    let secret = manager.read(&target).unwrap().unwrap();
    assert_eq!(secret.expose_secret(), b"temporary-test-token");
    assert_eq!(secret.len(), 20);
    assert!(!secret.is_empty());
    drop(secret);

    assert!(manager.delete(&target).unwrap());
    assert!(manager.read(&target).unwrap().is_none());
    assert!(!manager.delete(&target).unwrap());
}

#[test]
fn invalid_target_and_secret_are_rejected_before_win32_access() {
    let manager = WindowsCredentialManager::new();
    for target in [
        "",
        "FlashToSpine/RemoteGpu/UPPERCASE",
        "FlashToSpine/RemoteGpu/../escape",
        "AnotherProduct/RemoteGpu/test-profile",
    ] {
        let error = manager.read(target).err().expect("target must be rejected");
        assert_eq!(error.kind(), CredentialManagerErrorKind::InvalidTarget);
    }

    let target = unique_target();
    for secret in [b"".as_slice(), b"contains\0nul".as_slice()] {
        let error = manager.write(&target, secret).unwrap_err();
        assert_eq!(error.kind(), CredentialManagerErrorKind::InvalidSecret);
    }
    let over_limit = vec![b'x'; 2561];
    let error = manager.write(&target, &over_limit).unwrap_err();
    assert_eq!(error.kind(), CredentialManagerErrorKind::InvalidSecret);
}

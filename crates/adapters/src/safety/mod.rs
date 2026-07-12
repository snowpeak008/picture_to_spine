pub mod diagnostics;
pub mod private_remote;
pub mod worker_eligibility;

// CredReadW/CredWriteW/CredDeleteW and CredFree are raw Win32 APIs. Keep their
// pointer handling isolated here while unsafe code remains denied elsewhere.
#[cfg(windows)]
#[allow(unsafe_code)]
pub mod windows_credentials;

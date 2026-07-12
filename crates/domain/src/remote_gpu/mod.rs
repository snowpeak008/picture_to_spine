mod job;
mod profile;
mod transfer;

pub use job::*;
pub use profile::*;
pub use transfer::*;

pub(crate) fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(crate) fn is_safe_identifier(value: &str) -> bool {
    (3..=96).contains(&value.len())
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
}

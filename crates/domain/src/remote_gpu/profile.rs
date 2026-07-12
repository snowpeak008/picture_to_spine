use super::{is_lower_hex_sha256, is_safe_identifier};
use crate::canonical::canonical_sha256;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};

const PUBLIC_PROVIDER_SUFFIXES: [&str; 9] = [
    "openai.com",
    "anthropic.com",
    "replicate.com",
    "fal.ai",
    "stability.ai",
    "runwayml.com",
    "huggingface.co",
    "generativelanguage.googleapis.com",
    "ai.google.dev",
];

const RESERVED_PROFILE_IDS: [&str; 8] = [
    "default",
    "public",
    "auto",
    "automatic",
    "fallback",
    "cloud",
    "saas",
    "provider",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RemoteGpuMethod {
    LayerSegmentationCandidate,
    RigProposalCandidate,
    MotionCurveCandidate,
}

impl RemoteGpuMethod {
    pub const METHOD_SCHEMA: u32 = 1;

    pub fn is_candidate_only(self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RemoteMediaType {
    ImagePng,
    ImageJpeg,
    ImageWebp,
    ApplicationJson,
    ApplicationRigIrJson,
}

impl RemoteMediaType {
    pub fn as_mime(self) -> &'static str {
        match self {
            Self::ImagePng => "image/png",
            Self::ImageJpeg => "image/jpeg",
            Self::ImageWebp => "image/webp",
            Self::ApplicationJson => "application/json",
            Self::ApplicationRigIrJson => "application/vnd.flash-to-spine.rig-ir+json",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EndpointOwnership {
    UserControlledPrivate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteGpuProfile {
    pub schema_version: String,
    pub enabled: bool,
    pub profile_id: String,
    pub ownership: EndpointOwnership,
    pub origin: String,
    pub allowed_ports: Vec<u16>,
    pub certificate_spki_sha256: String,
    pub organization_identity_sha256: String,
    pub credential_manager_target: String,
    pub allowed_methods: Vec<RemoteGpuMethod>,
    pub allowed_input_media_types: Vec<RemoteMediaType>,
    pub allowed_model_manifest_sha256: Vec<String>,
    pub max_upload_bytes: u64,
    pub max_response_bytes: u64,
    pub request_timeout_seconds: u32,
}

impl RemoteGpuProfile {
    pub fn validate_configuration(&self) -> Result<(), String> {
        if self.schema_version != "1.0.0" {
            return Err("unsupported private remote profile schema".into());
        }
        if !is_safe_identifier(&self.profile_id)
            || RESERVED_PROFILE_IDS.contains(&self.profile_id.as_str())
        {
            return Err(
                "remote profile id must be explicit and cannot be a default/fallback".into(),
            );
        }
        let parsed = CanonicalHttpsOrigin::parse(&self.origin)?;
        if parsed.canonical != self.origin {
            return Err("private endpoint origin is not canonical".into());
        }
        if self.allowed_ports.is_empty()
            || !is_strictly_sorted_unique(&self.allowed_ports)
            || !self.allowed_ports.contains(&parsed.port)
        {
            return Err("private endpoint port is not in the canonical port allowlist".into());
        }
        if !is_lower_hex_sha256(&self.certificate_spki_sha256)
            || !is_lower_hex_sha256(&self.organization_identity_sha256)
        {
            return Err("SPKI and organization identity pins must be lowercase SHA-256".into());
        }
        let expected_target = format!("FlashToSpine/RemoteGpu/{}", self.profile_id);
        if self.credential_manager_target != expected_target
            || self.credential_manager_target.len() > 160
        {
            return Err(
                "credential must be an exact Windows Credential Manager target reference".into(),
            );
        }
        if self.allowed_methods.is_empty()
            || !is_strictly_sorted_unique(&self.allowed_methods)
            || self
                .allowed_methods
                .iter()
                .any(|method| !method.is_candidate_only())
        {
            return Err("remote profile must allow only fixed candidate methods".into());
        }
        if self.allowed_input_media_types.is_empty()
            || !is_strictly_sorted_unique(&self.allowed_input_media_types)
        {
            return Err("remote input media allowlist must be sorted and non-empty".into());
        }
        if self.allowed_model_manifest_sha256.is_empty()
            || !is_strictly_sorted_unique(&self.allowed_model_manifest_sha256)
            || self
                .allowed_model_manifest_sha256
                .iter()
                .any(|hash| !is_lower_hex_sha256(hash))
        {
            return Err("remote model manifest allowlist is invalid".into());
        }
        if self.max_upload_bytes == 0
            || self.max_upload_bytes > 512 * 1024 * 1024
            || self.max_response_bytes == 0
            || self.max_response_bytes > 512 * 1024 * 1024
        {
            return Err("remote byte budget is outside the fixed Core policy".into());
        }
        if !(5..=900).contains(&self.request_timeout_seconds) {
            return Err("remote request timeout is outside policy".into());
        }
        Ok(())
    }

    pub fn require_enabled(&self) -> Result<(), String> {
        self.validate_configuration()?;
        if !self.enabled {
            return Err("private remote GPU is disabled by default".into());
        }
        Ok(())
    }

    pub fn canonical_sha256(&self) -> Result<String, String> {
        self.validate_configuration()?;
        canonical_sha256(self).map_err(|error| error.to_string())
    }

    pub fn effective_port(&self) -> Result<u16, String> {
        Ok(CanonicalHttpsOrigin::parse(&self.origin)?.port)
    }
}

fn is_strictly_sorted_unique<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

struct CanonicalHttpsOrigin {
    canonical: String,
    port: u16,
}

impl CanonicalHttpsOrigin {
    fn parse(value: &str) -> Result<Self, String> {
        if value.len() > 512
            || value.trim() != value
            || !value.is_ascii()
            || !value.starts_with("https://")
        {
            return Err("private endpoint must be an ASCII HTTPS origin".into());
        }
        let authority = &value[8..];
        if authority.is_empty()
            || authority.bytes().any(|byte| {
                byte.is_ascii_whitespace()
                    || matches!(byte, b'/' | b'\\' | b'?' | b'#' | b'@' | b'%')
            })
        {
            return Err(
                "private endpoint cannot contain path, query, fragment, userinfo, or escapes"
                    .into(),
            );
        }
        let (host, explicit_port, rendered_host) = if let Some(rest) = authority.strip_prefix('[') {
            let close = rest.find(']').ok_or("invalid bracketed IPv6 endpoint")?;
            let host = &rest[..close];
            let suffix = &rest[close + 1..];
            let port = parse_port_suffix(suffix)?;
            let ip: Ipv6Addr = host.parse().map_err(|_| "invalid IPv6 endpoint")?;
            if !(ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local()) {
                return Err("public IP endpoints are not private remote profiles".into());
            }
            let canonical_host = ip.to_string();
            if host != canonical_host {
                return Err("IPv6 endpoint is not canonical".into());
            }
            (host.to_owned(), port, format!("[{host}]"))
        } else {
            if authority.matches(':').count() > 1 {
                return Err("IPv6 endpoints must use canonical brackets".into());
            }
            let (host, port) = match authority.rsplit_once(':') {
                Some((host, raw_port)) => (host, Some(parse_explicit_port(raw_port)?)),
                None => (authority, None),
            };
            validate_host(host)?;
            (host.to_owned(), port, host.to_owned())
        };
        let port = explicit_port.unwrap_or(443);
        let canonical = if explicit_port.is_some() {
            format!("https://{rendered_host}:{port}")
        } else {
            format!("https://{rendered_host}")
        };
        let _ = host;
        Ok(Self { canonical, port })
    }
}

fn parse_port_suffix(value: &str) -> Result<Option<u16>, String> {
    if value.is_empty() {
        Ok(None)
    } else if let Some(port) = value.strip_prefix(':') {
        Ok(Some(parse_explicit_port(port)?))
    } else {
        Err("invalid endpoint authority suffix".into())
    }
}

fn parse_explicit_port(value: &str) -> Result<u16, String> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err("endpoint port is not canonical".into());
    }
    value
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| "endpoint port is invalid".into())
}

fn validate_host(host: &str) -> Result<(), String> {
    if host.is_empty() || host.len() > 253 || host != host.to_ascii_lowercase() {
        return Err("endpoint host must be canonical lowercase ASCII".into());
    }
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        if !(ip.is_private() || ip.is_loopback() || ip.is_link_local()) {
            return Err("public IP endpoints are not private remote profiles".into());
        }
        if host != ip.to_string() {
            return Err("IPv4 endpoint is not canonical".into());
        }
        return Ok(());
    }
    if host.ends_with('.')
        || host.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
    {
        return Err("endpoint DNS host is invalid".into());
    }
    if PUBLIC_PROVIDER_SUFFIXES
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
    {
        return Err("public AI providers are forbidden".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_is_exact_and_private() {
        assert!(CanonicalHttpsOrigin::parse("https://gpu.internal.example").is_ok());
        assert!(CanonicalHttpsOrigin::parse("https://10.4.0.8:8443").is_ok());
        assert!(CanonicalHttpsOrigin::parse("http://gpu.internal.example").is_err());
        assert!(CanonicalHttpsOrigin::parse("https://user@gpu.internal.example").is_err());
        assert!(CanonicalHttpsOrigin::parse("https://gpu.internal.example/").is_err());
        assert!(CanonicalHttpsOrigin::parse("https://api.openai.com").is_err());
        assert!(CanonicalHttpsOrigin::parse("https://8.8.8.8").is_err());
        assert!(CanonicalHttpsOrigin::parse("https://GPU.internal.example").is_err());
    }
}

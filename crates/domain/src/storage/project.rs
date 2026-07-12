use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectHead {
    pub schema_version: String,
    pub project_id: String,
    pub head_revision: u64,
    pub manifest_sha256: String,
    /// Hash of the previous revision's manifest. The historic field name is
    /// retained so existing unsigned projects remain readable.
    #[serde(default)]
    pub previous_head_sha256: Option<String>,
    /// Identifier of the local integrity key used to seal this head.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
    /// MAC of the previous signed ProjectHead. Together with
    /// `previous_head_sha256`, this makes the immutable revision sidecars a
    /// verifiable chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_head_mac: Option<String>,
    /// HMAC-SHA256 over every security-relevant field above (excluding this
    /// field itself).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_mac: Option<String>,
}

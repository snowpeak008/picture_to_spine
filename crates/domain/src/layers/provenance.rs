use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PixelOrigin {
    Source,
    Manual,
    LocalAi,
    PrivateRemoteAi,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PixelProvenance {
    pub artifact_sha256: String,
    pub origin: PixelOrigin,
    pub source_sha256: String,
    pub prompt_pack_id: Option<String>,
    pub receipt_ref: Option<String>,
    pub accepted_by: Option<String>,
    pub acceptance_attestation_sha256: Option<String>,
}
impl PixelProvenance {
    pub fn can_enter_approved_layer(&self) -> bool {
        matches!(self.origin, PixelOrigin::Source | PixelOrigin::Manual)
            || (self
                .accepted_by
                .as_deref()
                .map(str::trim)
                .is_some_and(|v| !v.is_empty())
                && self
                    .acceptance_attestation_sha256
                    .as_deref()
                    .is_some_and(|value| {
                        value.len() == 64
                            && value
                                .bytes()
                                .all(|v| v.is_ascii_hexdigit() && !v.is_ascii_uppercase())
                    }))
    }
}

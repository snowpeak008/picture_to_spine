use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceArtifact {
    pub artifact_id: String,
    pub sha256: String,
    pub media_type: String,
    pub width: u32,
    pub height: u32,
    pub byte_length: u64,
    pub bit_depth: u8,
    pub provenance: String,
    pub approval_state: String,
}

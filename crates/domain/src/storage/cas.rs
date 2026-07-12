use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CasRef {
    pub sha256: String,
    pub byte_length: u64,
    pub media_type: String,
}

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceEvent {
    pub sequence: u64,
    pub stream_id: String,
    pub event_type: String,
    pub target_revision: u64,
    pub previous_event_sha256: Option<String>,
    pub payload_sha256: String,
}

use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub event_id: String,
    pub sequence: u64,
    pub event_type: String,
    pub actor_id: String,
    pub occurred_at_utc: String,
    pub previous_sha256: Option<String>,
    pub payload: Value,
}

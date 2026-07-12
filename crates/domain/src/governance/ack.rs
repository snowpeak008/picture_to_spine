use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WarningAck {
    pub warning_id: String,
    pub actor_id: String,
    pub target_revision: u64,
    pub acknowledged_at_utc: String,
}

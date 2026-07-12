use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Invalidation {
    pub source_id: String,
    pub source_revision: u64,
    pub affected_ids: Vec<String>,
    pub reason: String,
    pub created_at_utc: String,
}

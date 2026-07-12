use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewOutcome {
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewRecord {
    pub review_id: String,
    pub gate_id: String,
    pub target_id: String,
    pub target_revision: u64,
    pub target_sha256: String,
    pub actor_id: String,
    pub outcome: ReviewOutcome,
    pub reason: String,
    pub reviewed_at_utc: String,
}

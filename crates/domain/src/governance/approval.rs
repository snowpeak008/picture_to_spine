use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Approval {
    pub approval_id: String,
    pub gate_id: String,
    pub target_id: String,
    pub target_revision: u64,
    pub target_sha256: String,
    pub actor_id: String,
    pub approved_at_utc: String,
    pub invalidated: bool,
}
impl Approval {
    pub fn is_valid_for(&self, id: &str, revision: u64, sha: &str) -> bool {
        !self.invalidated
            && self.target_id == id
            && self.target_revision == revision
            && self.target_sha256 == sha
    }
}

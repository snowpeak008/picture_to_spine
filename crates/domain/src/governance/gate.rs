use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GateState {
    Pending,
    Approved,
    Rejected,
    Invalidated,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gate {
    pub gate_id: String,
    pub target_id: String,
    pub target_revision: u64,
    pub state: GateState,
    pub dependency_gate_ids: Vec<String>,
}
impl Gate {
    pub fn is_currently_approved(&self, revision: u64) -> bool {
        self.state == GateState::Approved && self.target_revision == revision
    }
}

use super::GateState;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub decision_id: String,
    pub gate_id: String,
    pub state: GateState,
    pub actor_id: String,
    pub target_revision: u64,
    pub decided_at_utc: String,
    pub reason: String,
}

use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub command_id: String,
    pub project_id: String,
    pub expected_revision: u64,
    pub kind: String,
    pub payload: Value,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandReceipt {
    pub command_id: String,
    pub before_revision: u64,
    pub after_revision: u64,
    pub effect_refs: Vec<String>,
}

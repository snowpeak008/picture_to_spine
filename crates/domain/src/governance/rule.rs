use super::Priority;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub rule_id: String,
    pub priority: Priority,
    pub hard: bool,
    pub message: String,
}
impl Rule {
    pub fn cannot_be_waived(&self) -> bool {
        self.hard || self.priority >= Priority::Safety
    }
}

use super::Priority;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Waiver {
    pub waiver_id: String,
    pub rule_id: String,
    pub rule_priority: Priority,
    pub target_revision: u64,
    pub expires_at_utc: String,
    pub limitation_acknowledged: bool,
}
impl Waiver {
    pub fn validate(&self) -> Result<(), String> {
        if self.rule_priority >= Priority::Safety {
            return Err("safety/license rules cannot be waived".into());
        }
        if !self.limitation_acknowledged {
            return Err("limitation acknowledgement required".into());
        }
        Ok(())
    }
}

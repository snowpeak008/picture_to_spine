use crate::validate_action_key;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionRule {
    pub from_action: String,
    pub to_action: String,
    pub mix_ticks: i64,
    pub interruptible_after_tick: i64,
}

impl TransitionRule {
    pub fn validate(&self) -> Result<(), String> {
        validate_action_key(&self.from_action).map_err(|e| e.to_string())?;
        validate_action_key(&self.to_action).map_err(|e| e.to_string())?;
        if self.from_action == self.to_action {
            return Err("self transition is implicit".into());
        }
        if self.mix_ticks < 0 || self.mix_ticks > 30_000 || self.interruptible_after_tick < 0 {
            return Err("transition timing outside V1 limits".into());
        }
        Ok(())
    }
}

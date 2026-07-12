use crate::ACTION_KEYS;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionCategory {
    Locomotion,
    Combat,
    Reaction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionDefinition {
    pub key: String,
    pub category: ActionCategory,
    pub loops: bool,
    pub requires_hit_frame: bool,
}

pub fn requires_hit_frame(action_key: &str) -> bool {
    matches!(action_key, "attack_01" | "attack_02" | "attack_03")
}

pub fn canonical_action_registry() -> Vec<ActionDefinition> {
    ACTION_KEYS
        .iter()
        .map(|key| ActionDefinition {
            key: (*key).into(),
            category: if requires_hit_frame(key) {
                ActionCategory::Combat
            } else if ["hit", "death"].contains(key) {
                ActionCategory::Reaction
            } else {
                ActionCategory::Locomotion
            },
            loops: ["idle", "run"].contains(key),
            requires_hit_frame: requires_hit_frame(key),
        })
        .collect()
}

pub fn validate_exact_registry(items: &[ActionDefinition]) -> Result<(), String> {
    let keys: Vec<_> = items.iter().map(|v| v.key.as_str()).collect();
    if keys != ACTION_KEYS {
        return Err("action registry must match the canonical ten keys and order".into());
    }
    if keys.iter().collect::<BTreeSet<_>>().len() != 10 {
        return Err("duplicate action key".into());
    }
    Ok(())
}

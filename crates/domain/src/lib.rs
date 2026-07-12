#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub mod animation;
pub mod canonical;
pub mod commands;
pub mod errors;
pub mod governance;
pub mod import;
pub mod jobs;
pub mod layers;
pub mod master;
pub mod motion;
pub mod observability;
pub mod project;
pub mod remote_gpu;
pub mod rig;
pub mod storage;

pub const ACTION_KEYS: [&str; 10] = [
    "idle",
    "run",
    "jump",
    "fall",
    "dash",
    "attack_01",
    "attack_02",
    "attack_03",
    "hit",
    "death",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeBase {
    pub numerator: i64,
    pub denominator: i64,
}

impl Default for TimeBase {
    fn default() -> Self {
        Self {
            numerator: 1,
            denominator: 30_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub project_id: Uuid,
    pub name: String,
    pub revision: u64,
    pub time_base: TimeBase,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("项目名称不能为空")]
    EmptyProjectName,
    #[error("未知动作键：{0}")]
    UnknownActionKey(String),
}

pub fn validate_action_key(value: &str) -> Result<(), DomainError> {
    if ACTION_KEYS.contains(&value) {
        Ok(())
    } else {
        Err(DomainError::UnknownActionKey(value.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_exact() {
        assert_eq!(ACTION_KEYS.len(), 10);
        assert!(validate_action_key("attack_01").is_ok());
        assert!(validate_action_key("attack_1").is_err());
    }
}

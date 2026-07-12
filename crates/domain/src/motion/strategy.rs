use super::spec::MotionSpec;
use crate::rig::SPINE_CAPABILITY_ID;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RepresentationStrategy {
    Bone,
    Mesh,
    Sequence,
    Replacement,
    Hybrid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyDecision {
    pub action_key: String,
    pub part: String,
    pub strategy: RepresentationStrategy,
    pub rule_id: String,
    pub capability_id: String,
    pub explanation: String,
}

pub fn choose_strategy(spec: &MotionSpec, part: &str) -> StrategyDecision {
    let strategy = if part.contains("hair") || part.contains("cloth") {
        RepresentationStrategy::Mesh
    } else if spec.action_key == "death" && part == "face" {
        RepresentationStrategy::Replacement
    } else if spec.action_key.starts_with("attack") && part == "weapon-effect" {
        RepresentationStrategy::Sequence
    } else {
        RepresentationStrategy::Bone
    };
    StrategyDecision {
        action_key: spec.action_key.clone(),
        part: part.into(),
        strategy,
        rule_id: "F2S-STRATEGY-V1".into(),
        capability_id: SPINE_CAPABILITY_ID.into(),
        explanation: "确定性 V1 规则；人工可覆盖但必须留下原因".into(),
    }
}

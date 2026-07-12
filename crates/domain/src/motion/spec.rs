use super::registry::{ActionDefinition, validate_exact_registry};
use crate::{TimeBase, validate_action_key};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LoopPolicy {
    Loop,
    OneShot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RootMotionPolicy {
    InPlace,
    PreviewTranslation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MotionPhase {
    pub key: String,
    pub start_tick: i64,
    pub end_tick: i64,
    pub intent: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MotionSpec {
    pub action_key: String,
    pub revision: u64,
    pub duration_ticks: i64,
    pub time_base: TimeBase,
    pub loop_policy: LoopPolicy,
    pub root_motion: RootMotionPolicy,
    pub silhouette_goal: String,
    pub weapon_intent: Option<String>,
    pub phases: Vec<MotionPhase>,
    pub contact_ticks: Vec<i64>,
}

impl MotionSpec {
    pub fn validate(
        &self,
        action: &ActionDefinition,
        primary_weapon: Option<&str>,
    ) -> Result<(), String> {
        validate_action_key(&self.action_key).map_err(|e| e.to_string())?;
        if self.action_key != action.key {
            return Err("MotionSpec/action registry mismatch".into());
        }
        if self.duration_ticks <= 0
            || self.time_base.numerator <= 0
            || self.time_base.denominator <= 0
        {
            return Err("invalid rational timebase or duration".into());
        }
        if self.silhouette_goal.trim().is_empty() || self.phases.is_empty() {
            return Err("motion intent and phases are required".into());
        }
        let mut cursor = 0;
        for phase in &self.phases {
            if phase.key.trim().is_empty()
                || phase.intent.trim().is_empty()
                || phase.start_tick != cursor
                || phase.end_tick <= phase.start_tick
            {
                return Err("motion phases must be contiguous and non-empty".into());
            }
            cursor = phase.end_tick;
        }
        if cursor != self.duration_ticks {
            return Err("motion phases must cover the full duration".into());
        }
        if action.loops != matches!(self.loop_policy, LoopPolicy::Loop) {
            return Err("loop policy disagrees with registry".into());
        }
        if action.requires_hit_frame && self.contact_ticks.len() != 1 {
            return Err("each attack requires exactly one contact tick".into());
        }
        if self
            .contact_ticks
            .iter()
            .any(|v| *v < 0 || *v >= self.duration_ticks)
        {
            return Err("contact tick outside clip".into());
        }
        if (action.requires_hit_frame || self.action_key == "dash")
            && primary_weapon
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .is_none()
        {
            return Err("primary weapon unresolved".into());
        }
        Ok(())
    }
}

pub fn validate_motion_set(
    registry: &[ActionDefinition],
    specs: &[MotionSpec],
    weapon: Option<&str>,
) -> Result<(), String> {
    validate_exact_registry(registry)?;
    if specs.len() != registry.len() {
        return Err("one MotionSpec required for each canonical action".into());
    }
    for (action, spec) in registry.iter().zip(specs) {
        spec.validate(action, weapon)?
    }
    Ok(())
}

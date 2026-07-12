use super::{
    assets::{AssetSpec, AssetState},
    prompt::PromptPack,
    registry::canonical_action_registry,
    spec::{MotionSpec, validate_motion_set},
    strategy::StrategyDecision,
};
use crate::{
    ACTION_KEYS, canonical::canonical_sha256, governance::Approval, master::StyleSpec,
    rig::SPINE_CAPABILITY_ID,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const KEY_POSE_GROUND_Y_MIN_MILLI_PX: i64 = -100_000_000;
pub const KEY_POSE_GROUND_Y_MAX_MILLI_PX: i64 = 100_000_000;
pub const KEY_POSE_SCALE_MIN_PPM: u32 = 10_000;
pub const KEY_POSE_SCALE_MAX_PPM: u32 = 100_000_000;

pub fn validate_key_pose_alignment(ground_y_milli_px: i64, scale_ppm: u32) -> Result<(), String> {
    if !(KEY_POSE_GROUND_Y_MIN_MILLI_PX..=KEY_POSE_GROUND_Y_MAX_MILLI_PX)
        .contains(&ground_y_milli_px)
    {
        return Err("key-pose ground alignment lies outside the supported canvas".into());
    }
    if !(KEY_POSE_SCALE_MIN_PPM..=KEY_POSE_SCALE_MAX_PPM).contains(&scale_ppm) {
        return Err("key-pose scale must be between 0.01x and 100x".into());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeyPoseBinding {
    pub binding_id: String,
    pub revision: u64,
    pub asset_spec_id: String,
    pub action_key: String,
    pub pose_key: String,
    pub source_sha256: String,
    pub media_type: String,
    pub width: u32,
    pub height: u32,
    pub prompt_pack_id: String,
    pub ground_y_milli_px: i64,
    pub scale_ppm: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MotionContent {
    pub revision: u64,
    pub specs: Vec<MotionSpec>,
    pub strategies: Vec<StrategyDecision>,
    pub assets: Vec<AssetSpec>,
    pub prompt_pack: PromptPack,
    pub key_pose_bindings: Vec<KeyPoseBinding>,
}

impl MotionContent {
    pub fn animation_input_sha256(&self) -> Result<String, String> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct AnimationInput<'a> {
            specs: &'a [MotionSpec],
            strategies: &'a [StrategyDecision],
            assets: Vec<AssetSpec>,
            prompt_pack: &'a PromptPack,
        }
        let assets = self
            .assets
            .iter()
            .cloned()
            .map(|mut asset| {
                asset.state = AssetState::Missing;
                asset
            })
            .collect();
        canonical_sha256(&AnimationInput {
            specs: &self.specs,
            strategies: &self.strategies,
            assets,
            prompt_pack: &self.prompt_pack,
        })
        .map_err(|error| error.to_string())
    }

    pub fn validate(&self, style: &StyleSpec) -> Result<(), String> {
        style.validate()?;
        let weapon = style
            .primary_weapon
            .as_ref()
            .map(|weapon| weapon.prompt_description());
        validate_motion_set(&canonical_action_registry(), &self.specs, weapon.as_deref())?;
        if self
            .specs
            .iter()
            .map(|spec| spec.action_key.as_str())
            .collect::<Vec<_>>()
            != ACTION_KEYS
        {
            return Err("MotionContent specs must use canonical action order".into());
        }
        let mut asset_ids = BTreeSet::new();
        for asset in &self.assets {
            asset.validate()?;
            if !asset_ids.insert(asset.asset_spec_id.as_str()) {
                return Err("duplicate AssetSpec id".into());
            }
        }
        for action in ACTION_KEYS {
            if !self
                .assets
                .iter()
                .any(|asset| asset.action_key == action && asset.required)
            {
                return Err(format!("required key-pose BOM missing for {action}"));
            }
        }
        self.prompt_pack.validate()?;
        let expected_motion_hash = canonical_sha256(&self.specs).map_err(|e| e.to_string())?;
        let expected_style_hash = canonical_sha256(style).map_err(|e| e.to_string())?;
        if self.prompt_pack.motion_revision_hash != expected_motion_hash
            || self.prompt_pack.style_revision != style.revision
            || self.prompt_pack.style_sha256 != expected_style_hash
        {
            return Err("PromptPack is stale for StyleSpec or MotionSpec".into());
        }
        let mut strategy_keys = BTreeSet::new();
        for strategy in &self.strategies {
            if !ACTION_KEYS.contains(&strategy.action_key.as_str())
                || strategy.part.trim().is_empty()
                || strategy.capability_id != SPINE_CAPABILITY_ID
                || strategy.explanation.trim().is_empty()
                || !strategy_keys.insert((strategy.action_key.as_str(), strategy.part.as_str()))
            {
                return Err("invalid or duplicate representation strategy".into());
            }
        }
        if ACTION_KEYS.iter().any(|action| {
            !self
                .strategies
                .iter()
                .any(|strategy| strategy.action_key == *action)
        }) {
            return Err("representation strategy missing for canonical action".into());
        }
        let mut binding_ids = BTreeSet::new();
        let mut bound_assets = BTreeSet::new();
        for binding in &self.key_pose_bindings {
            if !binding_ids.insert(binding.binding_id.as_str())
                || !bound_assets.insert(binding.asset_spec_id.as_str())
                || binding.source_sha256.len() != 64
                || !binding
                    .source_sha256
                    .bytes()
                    .all(|value| value.is_ascii_hexdigit() && !value.is_ascii_uppercase())
                || binding.width == 0
                || binding.height == 0
                || u64::from(binding.width) * u64::from(binding.height) > 16_777_216
                || !matches!(
                    binding.media_type.as_str(),
                    "image/png" | "image/jpeg" | "image/webp"
                )
            {
                return Err("invalid key-pose binding".into());
            }
            validate_key_pose_alignment(binding.ground_y_milli_px, binding.scale_ppm)?;
            let asset = self
                .assets
                .iter()
                .find(|asset| asset.asset_spec_id == binding.asset_spec_id)
                .ok_or("key-pose binding references unknown AssetSpec")?;
            if asset.action_key != binding.action_key
                || asset.pose_key != binding.pose_key
                || binding.prompt_pack_id != self.prompt_pack.pack_id
            {
                return Err("key-pose binding metadata or approval is inconsistent".into());
            }
        }
        if self.assets.iter().any(|asset| {
            asset.state == AssetState::Approved
                && !self
                    .key_pose_bindings
                    .iter()
                    .any(|binding| binding.asset_spec_id == asset.asset_spec_id)
        }) {
            return Err("approved AssetSpec has no key-pose binding".into());
        }
        Ok(())
    }

    pub fn required_pose_keys(&self, action_key: &str) -> Vec<String> {
        self.assets
            .iter()
            .filter(|asset| asset.action_key == action_key && asset.required)
            .map(|asset| asset.pose_key.clone())
            .collect()
    }

    pub fn approved_asset_count(&self, action_key: &str) -> usize {
        self.assets
            .iter()
            .filter(|asset| asset.action_key == action_key && asset.state == AssetState::Approved)
            .count()
    }

    pub fn action_required_assets_approved(&self, action_key: &str) -> bool {
        let required = self
            .assets
            .iter()
            .filter(|asset| asset.action_key == action_key && asset.required)
            .collect::<Vec<_>>();
        !required.is_empty()
            && required
                .iter()
                .all(|asset| asset.state == AssetState::Approved)
    }

    pub fn all_required_assets_approved(&self) -> bool {
        ACTION_KEYS
            .iter()
            .all(|action| self.action_required_assets_approved(action))
    }

    pub fn binding_approval_payload(&self, binding: &KeyPoseBinding) -> Result<String, String> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload<'a> {
            binding: &'a KeyPoseBinding,
            prompt_pack_sha256: String,
            motion_specs_sha256: String,
        }
        canonical_sha256(&Payload {
            binding,
            prompt_pack_sha256: canonical_sha256(&self.prompt_pack)
                .map_err(|error| error.to_string())?,
            motion_specs_sha256: canonical_sha256(&self.specs)
                .map_err(|error| error.to_string())?,
        })
        .map_err(|error| error.to_string())
    }

    pub fn apply_asset_approval(&mut self, approval: &Approval) -> Result<(), String> {
        if approval.gate_id != "key-pose-asset" {
            return Err("wrong gate for key-pose asset approval".into());
        }
        let binding = self
            .key_pose_bindings
            .iter()
            .find(|binding| binding.binding_id == approval.target_id)
            .ok_or("asset approval binding missing")?;
        let payload = self.binding_approval_payload(binding)?;
        if !approval.is_valid_for(&binding.binding_id, binding.revision, &payload) {
            return Err("asset approval is stale or bound to another key pose".into());
        }
        let asset = self
            .assets
            .iter_mut()
            .find(|asset| asset.asset_spec_id == binding.asset_spec_id)
            .ok_or("approved key-pose AssetSpec missing")?;
        asset.state = AssetState::Approved;
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or("MotionContent revision overflow")?;
        Ok(())
    }
}

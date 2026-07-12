use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayerRole {
    HairBack,
    Body,
    Head,
    HairFront,
    UpperArmBack,
    ForearmBack,
    UpperArmFront,
    ForearmFront,
    HandBack,
    HandFront,
    ThighBack,
    ShinBack,
    FootBack,
    ThighFront,
    ShinFront,
    FootFront,
    Weapon,
    Accessory,
}
impl LayerRole {
    pub const REQUIRED_V1: [Self; 17] = [
        Self::HairBack,
        Self::Body,
        Self::Head,
        Self::HairFront,
        Self::UpperArmBack,
        Self::ForearmBack,
        Self::HandBack,
        Self::UpperArmFront,
        Self::ForearmFront,
        Self::HandFront,
        Self::ThighBack,
        Self::ShinBack,
        Self::FootBack,
        Self::ThighFront,
        Self::ShinFront,
        Self::FootFront,
        Self::Weapon,
    ];
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub layer_id: String,
    pub name: String,
    pub role: LayerRole,
    pub attachment_sha256: String,
    pub mask_sha256: String,
    pub visible: bool,
    pub approved: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerSet {
    pub layer_set_id: String,
    pub master_id: String,
    pub revision: u64,
    pub layers: Vec<Layer>,
    pub approval_state: String,
}
impl LayerSet {
    pub fn validate(&self) -> Result<(), String> {
        let mut ids = BTreeSet::new();
        for layer in &self.layers {
            if !ids.insert(&layer.layer_id) {
                return Err("duplicate layer id".into());
            }
            if layer.name.trim().is_empty() {
                return Err("empty layer name".into());
            }
        }
        if !self.layers.iter().any(|v| v.role == LayerRole::Body) {
            return Err("body layer required".into());
        }
        Ok(())
    }

    pub fn validate_required_v1_roles(&self) -> Result<(), String> {
        let missing = LayerRole::REQUIRED_V1
            .iter()
            .filter(|role| !self.layers.iter().any(|layer| layer.role == **role))
            .map(|role| format!("{role:?}"))
            .collect::<Vec<_>>();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "required V1 layer roles missing: {}",
                missing.join(", ")
            ))
        }
    }
}

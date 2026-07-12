use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptEntry {
    pub asset_spec_id: String,
    pub action_key: String,
    pub pose_key: String,
    pub positive: String,
    pub negative: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptPack {
    pub pack_id: String,
    pub revision: u64,
    pub style_revision: u64,
    pub style_sha256: String,
    pub motion_revision_hash: String,
    pub provider_profile: String,
    pub entries: Vec<PromptEntry>,
    pub network_calls_made: u32,
}

impl PromptPack {
    pub fn validate(&self) -> Result<(), String> {
        if self.network_calls_made != 0 {
            return Err("PromptPack must be generated offline".into());
        }
        if self.style_sha256.len() != 64
            || !self
                .style_sha256
                .bytes()
                .all(|value| value.is_ascii_hexdigit() && !value.is_ascii_uppercase())
        {
            return Err("PromptPack StyleSpec hash is invalid".into());
        }
        if self.entries.is_empty() {
            return Err("PromptPack requires entries".into());
        }
        for e in &self.entries {
            if e.positive.contains("{{") || e.negative.contains("{{") {
                return Err("unresolved prompt placeholder".into());
            }
        }
        Ok(())
    }
}

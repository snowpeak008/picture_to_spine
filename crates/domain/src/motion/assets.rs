use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssetState {
    Missing,
    Requested,
    Imported,
    Reviewed,
    Approved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetSpec {
    pub asset_spec_id: String,
    pub action_key: String,
    pub pose_key: String,
    pub required: bool,
    pub purpose: String,
    pub state: AssetState,
}

impl AssetSpec {
    pub fn validate(&self) -> Result<(), String> {
        crate::validate_action_key(&self.action_key).map_err(|e| e.to_string())?;
        if self.asset_spec_id.trim().is_empty()
            || self.pose_key.trim().is_empty()
            || self.purpose.trim().is_empty()
        {
            return Err("asset spec fields required".into());
        }
        Ok(())
    }
}

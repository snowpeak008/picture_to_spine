use f2s_domain::motion::assets::AssetSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPoseCandidate {
    pub asset_spec_id: String,
    pub action_key: String,
    pub pose_key: String,
    pub source_artifact_sha256: String,
    pub prompt_pack_id: String,
    pub ground_y_milli_px: i64,
    pub scale_ppm: u32,
    pub approved: bool,
}

pub fn bind_key_pose(
    asset: &AssetSpec,
    source_hash: &str,
    prompt_pack_id: &str,
) -> Result<KeyPoseCandidate, String> {
    asset.validate()?;
    if source_hash.len() != 64 || !source_hash.bytes().all(|v| v.is_ascii_hexdigit()) {
        return Err("invalid source artifact hash".into());
    }
    if prompt_pack_id.trim().is_empty() {
        return Err("PromptPack reference required".into());
    }
    Ok(KeyPoseCandidate {
        asset_spec_id: asset.asset_spec_id.clone(),
        action_key: asset.action_key.clone(),
        pose_key: asset.pose_key.clone(),
        source_artifact_sha256: source_hash.to_ascii_lowercase(),
        prompt_pack_id: prompt_pack_id.into(),
        ground_y_milli_px: 0,
        scale_ppm: 1_000_000,
        approved: false,
    })
}

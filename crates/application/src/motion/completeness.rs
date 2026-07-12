use f2s_domain::{
    ACTION_KEYS,
    motion::{
        assets::{AssetSpec, AssetState},
        prompt::PromptPack,
        spec::MotionSpec,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionContentRow {
    pub action_key: String,
    pub spec_ready: bool,
    pub prompt_ready: bool,
    pub required_assets: u32,
    pub approved_assets: u32,
    pub ready: bool,
    pub reason: String,
}

pub fn content_matrix(
    specs: &[MotionSpec],
    assets: &[AssetSpec],
    pack: &PromptPack,
) -> Vec<ActionContentRow> {
    ACTION_KEYS
        .iter()
        .map(|key| {
            let spec_ready = specs.iter().any(|v| v.action_key == *key);
            let needed: Vec<_> = assets
                .iter()
                .filter(|v| v.action_key == *key && v.required)
                .collect();
            let approved = needed
                .iter()
                .filter(|v| v.state == AssetState::Approved)
                .count();
            let prompt_ready = pack.entries.iter().any(|v| v.action_key == *key);
            let ready =
                spec_ready && prompt_ready && !needed.is_empty() && approved == needed.len();
            ActionContentRow {
                action_key: (*key).into(),
                spec_ready,
                prompt_ready,
                required_assets: needed.len() as u32,
                approved_assets: approved as u32,
                ready,
                reason: if ready {
                    "ready".into()
                } else {
                    "spec/prompt/import/review chain incomplete".into()
                },
            }
        })
        .collect()
}

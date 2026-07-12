use f2s_domain::animation::clip::{AnimationClip, TrackChannel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationIssue {
    pub code: String,
    pub target: String,
    pub severity: String,
    pub tick: Option<i64>,
    pub explanation: String,
}

pub fn diagnose_clip(clip: &AnimationClip) -> Vec<AnimationIssue> {
    let mut issues = Vec::new();
    if clip.validate().is_err() {
        issues.push(AnimationIssue {
            code: "CLIP_INVALID".into(),
            target: clip.clip_id.clone(),
            severity: "P0".into(),
            tick: None,
            explanation: "时间轴不变量失败".into(),
        });
        return issues;
    }
    for track in &clip.tracks {
        for pair in track.keyframes.windows(2) {
            if pair[1].tick - pair[0].tick < 100
                && matches!(track.channel, TrackChannel::BoneRotate)
            {
                issues.push(AnimationIssue {
                    code: "ROTATION_SPIKE".into(),
                    target: track.target_id.clone(),
                    severity: "P1".into(),
                    tick: Some(pair[1].tick),
                    explanation: "相邻旋转关键帧过密，需要人工复核".into(),
                })
            }
        }
    }
    issues
}

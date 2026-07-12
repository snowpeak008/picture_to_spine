use f2s_domain::{
    canonical::canonical_sha256,
    master::StyleSpec,
    motion::{
        assets::AssetSpec,
        prompt::{PromptEntry, PromptPack},
        spec::MotionSpec,
    },
};

pub fn compose_prompt_pack(
    style: &StyleSpec,
    motions: &[MotionSpec],
    assets: &[AssetSpec],
) -> Result<PromptPack, String> {
    style.validate()?;
    let primary_weapon = style
        .primary_weapon
        .as_ref()
        .ok_or("primary weapon unresolved")?
        .prompt_description();
    let motion_hash = canonical_sha256(&motions).map_err(|e| e.to_string())?;
    let style_hash = canonical_sha256(style).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    for asset in assets.iter().filter(|v| v.required) {
        asset.validate()?;
        let motion = motions
            .iter()
            .find(|v| v.action_key == asset.action_key)
            .ok_or("asset has no MotionSpec")?;
        let phases = motion
            .phases
            .iter()
            .map(|v| format!("{}:{}", v.key, v.intent))
            .collect::<Vec<_>>()
            .join("；");
        entries.push(PromptEntry{asset_spec_id:asset.asset_spec_id.clone(),action_key:asset.action_key.clone(),pose_key:asset.pose_key.clone(),positive:format!("二次元类人角色，严格横版侧视，单一主武器：{primary_weapon}。保持身份：{}。画面风格：{}；线条：{}；色彩：{}。动作 {}，关键姿势 {}，轮廓目标：{}，阶段：{}。完整身体，纯色背景，正交视角，供2D骨骼动画拆分参考。",style.identity_notes,style.rendering_style,style.outline,style.palette_notes,asset.action_key,asset.pose_key,motion.silhouette_goal,phases),negative:"正面或三分之四视角，多武器，身份变化，服装变化，裁切肢体，透视畸变，运动模糊，文字，水印，复杂背景，额外手指，肢体粘连，光照方向漂移".into()});
    }
    entries.sort_by(|a, b| {
        (&a.action_key, &a.pose_key, &a.asset_spec_id).cmp(&(
            &b.action_key,
            &b.pose_key,
            &b.asset_spec_id,
        ))
    });
    let pack = PromptPack {
        pack_id: format!("prompt-{:.8}-{:.8}", style_hash, motion_hash),
        revision: 1,
        style_revision: style.revision,
        style_sha256: style_hash,
        motion_revision_hash: motion_hash,
        provider_profile: "provider-neutral".into(),
        entries,
        network_calls_made: 0,
    };
    pack.validate()?;
    Ok(pack)
}

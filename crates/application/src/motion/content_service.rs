use crate::approvals::VerifiedHumanActor;
use f2s_domain::{
    ACTION_KEYS, TimeBase,
    governance::Approval,
    import::SourceArtifact,
    master::StyleSpec,
    motion::{
        assets::{AssetSpec, AssetState},
        content::{KeyPoseBinding, MotionContent, validate_key_pose_alignment},
        registry::{canonical_action_registry, requires_hit_frame},
        spec::{LoopPolicy, MotionPhase, MotionSpec, RootMotionPolicy},
        strategy::{StrategyDecision, choose_strategy},
    },
};
use uuid::Uuid;

use super::prompt_composer::compose_prompt_pack;

fn template(action: &str) -> Result<(i64, Vec<(&'static str, &'static str)>), String> {
    Ok(match action {
        "idle" => (
            30_000,
            vec![
                ("inhale", "轻微吸气并保持身份轮廓"),
                ("exhale", "呼气回到稳定重心"),
            ],
        ),
        "run" => (
            20_000,
            vec![
                ("contact", "前脚接触地面"),
                ("down", "重心下沉"),
                ("passing", "双腿交错通过"),
                ("up", "腾空并准备下一次接触"),
            ],
        ),
        "jump" => (
            24_000,
            vec![
                ("anticipation", "屈膝蓄力"),
                ("takeoff", "蹬地离地"),
                ("apex", "空中最高点保持可读轮廓"),
            ],
        ),
        "fall" => (
            18_000,
            vec![
                ("descent", "稳定下落姿势"),
                ("landing-prep", "屈膝准备落地"),
            ],
        ),
        "dash" => (
            12_000,
            vec![
                ("anticipation", "压低重心"),
                ("burst", "沿侧视轴爆发位移"),
                ("recovery", "回收到可衔接姿势"),
            ],
        ),
        "attack_01" => (
            24_000,
            vec![
                ("anticipation", "单武器快速攻击预备"),
                ("contact", "唯一命中姿势"),
                ("recovery", "攻击收势"),
            ],
        ),
        "attack_02" => (
            28_000,
            vec![
                ("anticipation", "反向蓄势"),
                ("contact", "唯一命中姿势"),
                ("recovery", "回正武器与重心"),
            ],
        ),
        "attack_03" => (
            36_000,
            vec![
                ("anticipation", "重击长蓄力"),
                ("contact", "唯一重击命中姿势"),
                ("recovery", "明显硬直收势"),
            ],
        ),
        "hit" => (
            15_000,
            vec![
                ("impact", "受击瞬间"),
                ("recoil", "后仰位移"),
                ("recovery", "恢复站姿"),
            ],
        ),
        "death" => (
            36_000,
            vec![
                ("impact", "失衡起点"),
                ("fall", "倒地过程"),
                ("settled", "最终静止姿势"),
            ],
        ),
        _ => return Err("unknown canonical action".into()),
    })
}

fn default_spec(action: &str, style: &StyleSpec) -> Result<MotionSpec, String> {
    let (duration, phases) = template(action)?;
    let phase_count = phases.len() as i64;
    let mut motion_phases = Vec::with_capacity(phases.len());
    for (index, (key, intent)) in phases.into_iter().enumerate() {
        let start_tick = duration * index as i64 / phase_count;
        let end_tick = if index + 1 == phase_count as usize {
            duration
        } else {
            duration * (index as i64 + 1) / phase_count
        };
        motion_phases.push(MotionPhase {
            key: key.into(),
            start_tick,
            end_tick,
            intent: intent.into(),
        });
    }
    let contact_ticks = if requires_hit_frame(action) {
        let contact = motion_phases
            .iter()
            .find(|phase| phase.key == "contact")
            .ok_or("attack contact phase missing")?;
        vec![(contact.start_tick + contact.end_tick) / 2]
    } else {
        vec![]
    };
    let weapon = style
        .primary_weapon
        .as_ref()
        .ok_or("primary weapon unresolved")?;
    Ok(MotionSpec {
        action_key: action.into(),
        revision: 0,
        duration_ticks: duration,
        time_base: TimeBase::default(),
        loop_policy: if ["idle", "run"].contains(&action) {
            LoopPolicy::Loop
        } else {
            LoopPolicy::OneShot
        },
        root_motion: if action == "dash" {
            RootMotionPolicy::PreviewTranslation
        } else {
            RootMotionPolicy::InPlace
        },
        silhouette_goal: format!("{action} 在游戏尺寸下保持二次元侧视轮廓清晰"),
        weapon_intent: if requires_hit_frame(action) || action == "dash" {
            Some(format!(
                "{}；{}",
                weapon.weapon_type, weapon.silhouette_constraints
            ))
        } else {
            None
        },
        phases: motion_phases,
        contact_ticks,
    })
}

fn assets_for(specs: &[MotionSpec]) -> Vec<AssetSpec> {
    specs
        .iter()
        .flat_map(|spec| {
            spec.phases.iter().map(|phase| AssetSpec {
                asset_spec_id: format!("{}--{}", spec.action_key, phase.key),
                action_key: spec.action_key.clone(),
                pose_key: phase.key.clone(),
                required: true,
                purpose: format!("{} 的 {} 关键姿势参考图", spec.action_key, phase.key),
                state: AssetState::Missing,
            })
        })
        .collect()
}

fn strategies_for(specs: &[MotionSpec]) -> Vec<StrategyDecision> {
    let mut values = specs
        .iter()
        .flat_map(|spec| {
            ["body", "hair-front", "hair-back", "weapon", "weapon-effect"]
                .into_iter()
                .map(|part| choose_strategy(spec, part))
        })
        .collect::<Vec<_>>();
    values.sort_by(|a, b| (&a.action_key, &a.part).cmp(&(&b.action_key, &b.part)));
    values
}

pub fn initialize_motion_content(style: &StyleSpec) -> Result<MotionContent, String> {
    style.validate()?;
    let specs = ACTION_KEYS
        .iter()
        .map(|action| default_spec(action, style))
        .collect::<Result<Vec<_>, _>>()?;
    let assets = assets_for(&specs);
    let prompt_pack = compose_prompt_pack(style, &specs, &assets)?;
    let content = MotionContent {
        revision: 0,
        strategies: strategies_for(&specs),
        specs,
        assets,
        prompt_pack,
        key_pose_bindings: vec![],
    };
    content.validate(style)?;
    Ok(content)
}

pub fn replace_motion_spec(
    content: &mut MotionContent,
    style: &StyleSpec,
    mut replacement: MotionSpec,
) -> Result<(), String> {
    let registry = canonical_action_registry();
    let action = registry
        .iter()
        .find(|action| action.key == replacement.action_key)
        .ok_or("replacement action is not canonical")?;
    let weapon = style
        .primary_weapon
        .as_ref()
        .map(|weapon| weapon.prompt_description());
    replacement.validate(action, weapon.as_deref())?;
    let index = content
        .specs
        .iter()
        .position(|spec| spec.action_key == replacement.action_key)
        .ok_or("current MotionSpec missing")?;
    replacement.revision = content.specs[index]
        .revision
        .checked_add(1)
        .ok_or("MotionSpec revision overflow")?;
    let mut candidate = content.clone();
    candidate.specs[index] = replacement;
    candidate.assets = assets_for(&candidate.specs);
    candidate.strategies = strategies_for(&candidate.specs);
    candidate.prompt_pack = compose_prompt_pack(style, &candidate.specs, &candidate.assets)?;
    candidate.key_pose_bindings.clear();
    candidate.revision = candidate
        .revision
        .checked_add(1)
        .ok_or("MotionContent revision overflow")?;
    candidate.validate(style)?;
    *content = candidate;
    Ok(())
}

pub fn bind_key_pose_image(
    content: &mut MotionContent,
    source: &SourceArtifact,
    asset_spec_id: &str,
) -> Result<KeyPoseBinding, String> {
    if source.bit_depth != 8
        || !matches!(
            source.media_type.as_str(),
            "image/png" | "image/jpeg" | "image/webp"
        )
    {
        return Err("key-pose image must be a validated 8-bit image".into());
    }
    let asset = content
        .assets
        .iter_mut()
        .find(|asset| asset.asset_spec_id == asset_spec_id)
        .ok_or("AssetSpec not found")?;
    let binding = KeyPoseBinding {
        binding_id: Uuid::new_v4().to_string(),
        revision: 0,
        asset_spec_id: asset.asset_spec_id.clone(),
        action_key: asset.action_key.clone(),
        pose_key: asset.pose_key.clone(),
        source_sha256: source.sha256.clone(),
        media_type: source.media_type.clone(),
        width: source.width,
        height: source.height,
        prompt_pack_id: content.prompt_pack.pack_id.clone(),
        ground_y_milli_px: 0,
        scale_ppm: 1_000_000,
    };
    content
        .key_pose_bindings
        .retain(|existing| existing.asset_spec_id != asset_spec_id);
    content.key_pose_bindings.push(binding.clone());
    content
        .key_pose_bindings
        .sort_by(|a, b| a.asset_spec_id.cmp(&b.asset_spec_id));
    asset.state = AssetState::Imported;
    content.revision = content
        .revision
        .checked_add(1)
        .ok_or("MotionContent revision overflow")?;
    Ok(binding)
}

pub fn set_key_pose_alignment(
    content: &mut MotionContent,
    binding_id: &str,
    expected_revision: u64,
    ground_y_milli_px: i64,
    scale_ppm: u32,
) -> Result<KeyPoseBinding, String> {
    validate_key_pose_alignment(ground_y_milli_px, scale_ppm)?;
    let mut next = content.clone();
    let binding = next
        .key_pose_bindings
        .iter_mut()
        .find(|binding| binding.binding_id == binding_id)
        .ok_or("key-pose binding missing")?;
    if binding.revision != expected_revision {
        return Err("stale key-pose binding revision".into());
    }
    if binding.ground_y_milli_px == ground_y_milli_px && binding.scale_ppm == scale_ppm {
        return Err("key-pose alignment is unchanged".into());
    }
    binding.ground_y_milli_px = ground_y_milli_px;
    binding.scale_ppm = scale_ppm;
    binding.revision = binding
        .revision
        .checked_add(1)
        .ok_or("key-pose binding revision overflow")?;
    let updated = binding.clone();
    let asset_spec_id = binding.asset_spec_id.clone();
    let asset = next
        .assets
        .iter_mut()
        .find(|asset| asset.asset_spec_id == asset_spec_id)
        .ok_or("key-pose AssetSpec missing")?;
    asset.state = AssetState::Imported;
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or("MotionContent revision overflow")?;
    *content = next;
    Ok(updated)
}

pub fn approve_key_pose_asset(
    content: &MotionContent,
    binding_id: &str,
    actor: VerifiedHumanActor,
    at: &str,
) -> Result<Approval, String> {
    if at.trim().is_empty() {
        return Err("key-pose approval timestamp required".into());
    }
    let binding = content
        .key_pose_bindings
        .iter()
        .find(|binding| binding.binding_id == binding_id)
        .ok_or("key-pose binding missing")?;
    let payload = content.binding_approval_payload(binding)?;
    actor.require_binding("approve-key-pose-asset", &payload)?;
    Ok(Approval {
        approval_id: Uuid::new_v4().to_string(),
        gate_id: "key-pose-asset".into(),
        target_id: binding.binding_id.clone(),
        target_revision: binding.revision,
        target_sha256: payload,
        actor_id: actor.actor_id().into(),
        approved_at_utc: at.into(),
        invalidated: false,
    })
}

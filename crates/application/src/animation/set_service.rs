use crate::approvals::VerifiedHumanActor;
use f2s_domain::{
    ACTION_KEYS,
    animation::{
        clip::{AnimationClip, Curve, Keyframe, Track, TrackChannel},
        markers::{GameplayMarker, GameplayMarkerKind},
        set::{AnimationSet, ReviewPoseMarker},
    },
    governance::Approval,
    motion::{content::MotionContent, registry::requires_hit_frame},
    rig::RigCandidate,
};
use uuid::Uuid;

use super::diagnostics::{AnimationIssue, diagnose_clip};

fn sample_values(action: &str, index: usize) -> [i64; 2] {
    match action {
        "run" => [
            index as i64 * 1_000,
            if index % 2 == 0 { 0 } else { -2_000 },
        ],
        "jump" => [index as i64 * 300, -(index as i64) * 8_000],
        "fall" => [0, index as i64 * 5_000],
        "dash" => [index as i64 * 12_000, 0],
        "attack_01" | "attack_02" | "attack_03" => [index as i64 * 1_500, 0],
        "hit" => [-(index as i64) * 2_000, 0],
        "death" => [index as i64 * 1_000, index as i64 * 4_000],
        _ => [0, if index % 2 == 0 { 0 } else { -500 }],
    }
}

fn socket_ids(rig: &RigCandidate) -> Vec<String> {
    rig.sockets
        .iter()
        .map(|socket| socket.socket_id.clone())
        .collect()
}

pub fn initialize_animation_set(
    motion: &MotionContent,
    rig: &RigCandidate,
    approved_rig_sha256: &str,
) -> Result<AnimationSet, String> {
    let weapon_socket = rig
        .sockets
        .iter()
        .find(|socket| socket.kind == f2s_domain::rig::pivots_sockets::SocketKind::PrimaryWeapon)
        .ok_or("primary weapon socket missing")?;
    let mut clips = Vec::with_capacity(ACTION_KEYS.len());
    let mut review_pose_markers = Vec::new();
    let mut gameplay_markers = Vec::new();
    for spec in &motion.specs {
        let mut ticks = vec![0, spec.duration_ticks];
        for phase in &spec.phases {
            let tick = (phase.start_tick + phase.end_tick) / 2;
            ticks.push(tick);
            review_pose_markers.push(ReviewPoseMarker {
                marker_id: format!("pose:{}:{}", spec.action_key, phase.key),
                action_key: spec.action_key.clone(),
                pose_key: phase.key.clone(),
                tick,
            });
        }
        ticks.sort_unstable();
        ticks.dedup();
        let keyframes = ticks
            .iter()
            .enumerate()
            .map(|(index, tick)| Keyframe {
                keyframe_id: format!("key:{}:root:{tick}", spec.action_key),
                tick: *tick,
                values_milli: sample_values(&spec.action_key, index).to_vec(),
                curve: Curve::Linear,
                bezier_milli: None,
            })
            .collect();
        clips.push(AnimationClip {
            clip_id: format!("clip:{}", spec.action_key),
            action_key: spec.action_key.clone(),
            revision: 0,
            duration_ticks: spec.duration_ticks,
            time_base: spec.time_base,
            tracks: vec![Track {
                track_id: format!("track:{}:root-translate", spec.action_key),
                target_id: "root".into(),
                channel: TrackChannel::BoneTranslate,
                keyframes,
            }],
        });
        if requires_hit_frame(&spec.action_key) {
            let tick = *spec
                .contact_ticks
                .first()
                .ok_or("attack MotionSpec contact tick missing")?;
            gameplay_markers.push(GameplayMarker {
                marker_id: format!("hit:{}", spec.action_key),
                action_key: spec.action_key.clone(),
                kind: GameplayMarkerKind::HitFrame,
                start_tick: tick,
                end_tick: tick,
                socket_id: Some(weapon_socket.socket_id.clone()),
            });
        }
    }
    let set = AnimationSet {
        revision: 0,
        approved_rig_sha256: approved_rig_sha256.into(),
        motion_content_sha256: motion.animation_input_sha256()?,
        clips,
        review_pose_markers,
        gameplay_markers,
    };
    set.validate(motion, &rig.bone_tree, &rig.slot_set, &socket_ids(rig))?;
    Ok(set)
}

fn validate_track_arity(track: &Track) -> Result<(), String> {
    for keyframe in &track.keyframes {
        let valid = match track.channel {
            TrackChannel::BoneTranslate | TrackChannel::BoneScale => {
                keyframe.values_milli.len() == 2
            }
            TrackChannel::BoneRotate | TrackChannel::DrawOrder => keyframe.values_milli.len() == 1,
            TrackChannel::SlotColor => {
                keyframe.values_milli.len() == 4
                    && keyframe
                        .values_milli
                        .iter()
                        .all(|value| (0..=1_000).contains(value))
            }
            TrackChannel::Deform => {
                !keyframe.values_milli.is_empty() && keyframe.values_milli.len() % 2 == 0
            }
            TrackChannel::Event => keyframe.values_milli.is_empty(),
        };
        if !valid {
            return Err("keyframe values do not match track channel arity".into());
        }
    }
    Ok(())
}

pub fn put_track(
    set: &mut AnimationSet,
    motion: &MotionContent,
    rig: &RigCandidate,
    action_key: &str,
    mut track: Track,
) -> Result<(), String> {
    validate_track_arity(&track)?;
    track
        .keyframes
        .sort_by_key(|keyframe| (keyframe.tick, keyframe.keyframe_id.clone()));
    let mut next = set.clone();
    let clip = next
        .clips
        .iter_mut()
        .find(|clip| clip.action_key == action_key)
        .ok_or("animation clip missing")?;
    if let Some(existing) = clip
        .tracks
        .iter_mut()
        .find(|existing| existing.track_id == track.track_id)
    {
        *existing = track;
    } else {
        clip.tracks.push(track);
    }
    clip.stable_sort();
    clip.revision = clip
        .revision
        .checked_add(1)
        .ok_or("clip revision overflow")?;
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or("AnimationSet revision overflow")?;
    next.validate(motion, &rig.bone_tree, &rig.slot_set, &socket_ids(rig))?;
    *set = next;
    Ok(())
}

pub fn set_review_pose_tick(
    set: &mut AnimationSet,
    motion: &MotionContent,
    rig: &RigCandidate,
    action_key: &str,
    pose_key: &str,
    tick: i64,
) -> Result<(), String> {
    let mut next = set.clone();
    let marker = next
        .review_pose_markers
        .iter_mut()
        .find(|marker| marker.action_key == action_key && marker.pose_key == pose_key)
        .ok_or("review pose marker missing")?;
    marker.tick = tick;
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or("AnimationSet revision overflow")?;
    next.validate(motion, &rig.bone_tree, &rig.slot_set, &socket_ids(rig))?;
    *set = next;
    Ok(())
}

pub fn set_hit_frame_marker(
    set: &mut AnimationSet,
    motion: &MotionContent,
    rig: &RigCandidate,
    expected_revision: u64,
    action_key: &str,
    tick: i64,
    socket_id: &str,
) -> Result<(), String> {
    if set.revision != expected_revision {
        return Err("stale AnimationSet revision".into());
    }
    if !requires_hit_frame(action_key) {
        return Err("hit-frame markers exist only for canonical attack actions".into());
    }
    let clip = set.clip(action_key)?;
    if tick < 0 || tick > clip.duration_ticks {
        return Err("hit-frame tick lies outside the clip".into());
    }
    let primary_socket = rig
        .sockets
        .iter()
        .find(|socket| socket.kind == f2s_domain::rig::pivots_sockets::SocketKind::PrimaryWeapon)
        .ok_or("primary weapon socket missing")?;
    if primary_socket.socket_id != socket_id {
        return Err("hit frame must reference the current primary weapon socket".into());
    }
    let mut next = set.clone();
    let marker = next
        .gameplay_markers
        .iter_mut()
        .find(|marker| {
            marker.action_key == action_key && marker.kind == GameplayMarkerKind::HitFrame
        })
        .ok_or("attack hit-frame marker missing")?;
    if marker.start_tick == tick
        && marker.end_tick == tick
        && marker.socket_id.as_deref() == Some(socket_id)
    {
        return Err("hit-frame marker is unchanged".into());
    }
    marker.start_tick = tick;
    marker.end_tick = tick;
    marker.socket_id = Some(socket_id.into());
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or("AnimationSet revision overflow")?;
    next.validate(motion, &rig.bone_tree, &rig.slot_set, &socket_ids(rig))?;
    *set = next;
    Ok(())
}

pub fn diagnose_animation_set(set: &AnimationSet) -> Vec<AnimationIssue> {
    set.clips.iter().flat_map(diagnose_clip).collect()
}

pub fn approve_action_poses(
    set: &AnimationSet,
    motion: &MotionContent,
    action_key: &str,
    actor: VerifiedHumanActor,
    approved_at_utc: &str,
) -> Result<Approval, String> {
    if approved_at_utc.trim().is_empty() {
        return Err("pose approval timestamp required".into());
    }
    let clip = set.clip(action_key)?;
    if diagnose_clip(clip)
        .iter()
        .any(|issue| matches!(issue.severity.as_str(), "P0" | "P1"))
    {
        return Err("blocking animation diagnostics remain".into());
    }
    let payload = set.pose_payload(motion, action_key)?;
    actor.require_binding("approve-key-poses", &payload)?;
    Ok(Approval {
        approval_id: Uuid::new_v4().to_string(),
        gate_id: "poses".into(),
        target_id: action_key.into(),
        target_revision: clip.revision,
        target_sha256: payload,
        actor_id: actor.actor_id().into(),
        approved_at_utc: approved_at_utc.into(),
        invalidated: false,
    })
}

pub fn approve_action_hit(
    set: &AnimationSet,
    motion: &MotionContent,
    action_key: &str,
    current_pose_approval: &Approval,
    actor: VerifiedHumanActor,
    approved_at_utc: &str,
) -> Result<Approval, String> {
    if approved_at_utc.trim().is_empty() {
        return Err("hit approval timestamp required".into());
    }
    let clip = set.clip(action_key)?;
    let pose_payload = set.pose_payload(motion, action_key)?;
    if current_pose_approval.gate_id != "poses"
        || !current_pose_approval.is_valid_for(action_key, clip.revision, &pose_payload)
    {
        return Err("current pose approval required before hit approval".into());
    }
    let payload = set.hit_payload(action_key)?;
    actor.require_binding("approve-hit-frame", &payload)?;
    Ok(Approval {
        approval_id: Uuid::new_v4().to_string(),
        gate_id: "hits".into(),
        target_id: action_key.into(),
        target_revision: clip.revision,
        target_sha256: payload,
        actor_id: actor.actor_id().into(),
        approved_at_utc: approved_at_utc.into(),
        invalidated: false,
    })
}

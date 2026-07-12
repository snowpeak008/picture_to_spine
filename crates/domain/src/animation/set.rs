use super::{
    clip::{AnimationClip, TrackChannel},
    markers::{GameplayMarker, GameplayMarkerKind, validate_markers},
};
use crate::{
    ACTION_KEYS,
    canonical::canonical_sha256,
    motion::{content::MotionContent, registry::requires_hit_frame},
    rig::{bone_tree::BoneTree, slots::SlotSet},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewPoseMarker {
    pub marker_id: String,
    pub action_key: String,
    pub pose_key: String,
    pub tick: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnimationSet {
    pub revision: u64,
    pub approved_rig_sha256: String,
    pub motion_content_sha256: String,
    pub clips: Vec<AnimationClip>,
    pub review_pose_markers: Vec<ReviewPoseMarker>,
    pub gameplay_markers: Vec<GameplayMarker>,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

impl AnimationSet {
    pub fn validate(
        &self,
        motion: &MotionContent,
        bones: &BoneTree,
        slots: &SlotSet,
        socket_ids: &[String],
    ) -> Result<(), String> {
        if !valid_sha256(&self.approved_rig_sha256)
            || !valid_sha256(&self.motion_content_sha256)
            || self.motion_content_sha256 != motion.animation_input_sha256()?
        {
            return Err("AnimationSet upstream hash binding is invalid".into());
        }
        if self
            .clips
            .iter()
            .map(|clip| clip.action_key.as_str())
            .collect::<Vec<_>>()
            != ACTION_KEYS
        {
            return Err("AnimationSet requires the canonical ten clips in order".into());
        }
        let bone_ids = bones
            .bones
            .iter()
            .map(|bone| bone.bone_id.as_str())
            .collect::<BTreeSet<_>>();
        let slot_ids = slots
            .slots
            .iter()
            .map(|slot| slot.slot_id.as_str())
            .collect::<BTreeSet<_>>();
        for (clip, spec) in self.clips.iter().zip(&motion.specs) {
            clip.validate()?;
            if clip.action_key != spec.action_key
                || clip.duration_ticks != spec.duration_ticks
                || clip.time_base != spec.time_base
                || clip.tracks.is_empty()
            {
                return Err(format!("clip is stale or empty for {}", spec.action_key));
            }
            for track in &clip.tracks {
                match track.channel {
                    TrackChannel::BoneTranslate
                    | TrackChannel::BoneRotate
                    | TrackChannel::BoneScale => {
                        if !bone_ids.contains(track.target_id.as_str()) {
                            return Err("animation track references unknown bone".into());
                        }
                    }
                    TrackChannel::SlotColor | TrackChannel::DrawOrder => {
                        if !slot_ids.contains(track.target_id.as_str()) {
                            return Err("animation track references unknown slot".into());
                        }
                    }
                    TrackChannel::Deform => {
                        let slot = track
                            .target_id
                            .split_once('/')
                            .map(|value| value.0)
                            .ok_or("deform target must be slot/mesh")?;
                        if !slot_ids.contains(slot) {
                            return Err("deform track references unknown slot".into());
                        }
                    }
                    TrackChannel::Event => {
                        if track.target_id.trim().is_empty() {
                            return Err("event track name required".into());
                        }
                    }
                }
            }
            let required = motion.required_pose_keys(&clip.action_key);
            let markers = self
                .review_pose_markers
                .iter()
                .filter(|marker| marker.action_key == clip.action_key)
                .collect::<Vec<_>>();
            if markers.len() != required.len()
                || markers
                    .iter()
                    .map(|marker| marker.pose_key.as_str())
                    .collect::<Vec<_>>()
                    != required.iter().map(String::as_str).collect::<Vec<_>>()
            {
                return Err(format!(
                    "review pose markers do not exactly cover {}",
                    clip.action_key
                ));
            }
            let mut marker_ids = BTreeSet::new();
            for marker in markers {
                if marker.marker_id.trim().is_empty()
                    || !marker_ids.insert(marker.marker_id.as_str())
                    || marker.tick < 0
                    || marker.tick > clip.duration_ticks
                    || !clip
                        .tracks
                        .iter()
                        .flat_map(|track| &track.keyframes)
                        .any(|keyframe| keyframe.tick == marker.tick)
                {
                    return Err("review pose marker is not bound to a real keyframe tick".into());
                }
            }
            let gameplay = self
                .gameplay_markers
                .iter()
                .filter(|marker| marker.action_key == clip.action_key)
                .cloned()
                .collect::<Vec<_>>();
            validate_markers(&clip.action_key, clip.duration_ticks, &gameplay)?;
            for marker in &gameplay {
                if let Some(socket) = marker.socket_id.as_ref()
                    && !socket_ids.contains(socket)
                {
                    return Err("gameplay marker references unknown reviewed socket".into());
                }
            }
            if requires_hit_frame(&clip.action_key) {
                let hit = gameplay
                    .iter()
                    .find(|marker| marker.kind == GameplayMarkerKind::HitFrame)
                    .ok_or("attack hit-frame marker missing")?;
                let contact_phase = spec
                    .phases
                    .iter()
                    .find(|phase| phase.key == "contact")
                    .ok_or("attack MotionSpec contact phase missing")?;
                if hit.start_tick != hit.end_tick
                    || hit.start_tick < contact_phase.start_tick
                    || hit.start_tick > contact_phase.end_tick
                {
                    return Err(
                        "attack hit frame must stay inside the MotionSpec contact phase".into(),
                    );
                }
            }
        }
        Ok(())
    }

    pub fn clip(&self, action_key: &str) -> Result<&AnimationClip, String> {
        self.clips
            .iter()
            .find(|clip| clip.action_key == action_key)
            .ok_or("animation clip missing".into())
    }

    pub fn pose_payload(&self, motion: &MotionContent, action_key: &str) -> Result<String, String> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload<'a> {
            approved_rig_sha256: &'a str,
            action_key: &'a str,
            clip_sha256: String,
            review_pose_markers_sha256: String,
            required_pose_keys: Vec<String>,
        }
        let clip = self.clip(action_key)?;
        let markers = self
            .review_pose_markers
            .iter()
            .filter(|marker| marker.action_key == action_key)
            .collect::<Vec<_>>();
        canonical_sha256(&Payload {
            approved_rig_sha256: &self.approved_rig_sha256,
            action_key,
            clip_sha256: canonical_sha256(clip).map_err(|error| error.to_string())?,
            review_pose_markers_sha256: canonical_sha256(&markers)
                .map_err(|error| error.to_string())?,
            required_pose_keys: motion.required_pose_keys(action_key),
        })
        .map_err(|error| error.to_string())
    }

    pub fn hit_payload(&self, action_key: &str) -> Result<String, String> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload<'a> {
            approved_rig_sha256: &'a str,
            action_key: &'a str,
            clip_sha256: String,
            marker_set_sha256: String,
            reviewed_weapon_socket_id: &'a str,
        }
        if !requires_hit_frame(action_key) {
            return Err("hit payload exists only for attack actions".into());
        }
        let clip = self.clip(action_key)?;
        let markers = self
            .gameplay_markers
            .iter()
            .filter(|marker| marker.action_key == action_key)
            .collect::<Vec<_>>();
        let socket = markers
            .iter()
            .find(|marker| marker.kind == GameplayMarkerKind::HitFrame)
            .and_then(|marker| marker.socket_id.as_deref())
            .ok_or("reviewed hit-frame socket missing")?;
        canonical_sha256(&Payload {
            approved_rig_sha256: &self.approved_rig_sha256,
            action_key,
            clip_sha256: canonical_sha256(clip).map_err(|error| error.to_string())?,
            marker_set_sha256: canonical_sha256(&markers).map_err(|error| error.to_string())?,
            reviewed_weapon_socket_id: socket,
        })
        .map_err(|error| error.to_string())
    }
}

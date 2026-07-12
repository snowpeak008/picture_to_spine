use crate::{TimeBase, validate_action_key};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrackChannel {
    BoneTranslate,
    BoneRotate,
    BoneScale,
    SlotColor,
    DrawOrder,
    Deform,
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Curve {
    Stepped,
    Linear,
    Bezier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Keyframe {
    pub keyframe_id: String,
    pub tick: i64,
    pub values_milli: Vec<i64>,
    pub curve: Curve,
    pub bezier_milli: Option<[i32; 4]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub track_id: String,
    pub target_id: String,
    pub channel: TrackChannel,
    pub keyframes: Vec<Keyframe>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationClip {
    pub clip_id: String,
    pub action_key: String,
    pub revision: u64,
    pub duration_ticks: i64,
    pub time_base: TimeBase,
    pub tracks: Vec<Track>,
}

impl AnimationClip {
    pub fn validate(&self) -> Result<(), String> {
        validate_action_key(&self.action_key).map_err(|e| e.to_string())?;
        if self.clip_id.trim().is_empty() || self.duration_ticks <= 0 {
            return Err("clip id and positive duration required".into());
        }
        let mut track_ids = BTreeSet::new();
        let mut track_bindings = BTreeSet::new();
        for track in &self.tracks {
            if !track_ids.insert(&track.track_id)
                || !track_bindings.insert((&track.target_id, &track.channel))
            {
                return Err("duplicate track id or target/channel binding".into());
            }
            if track.target_id.trim().is_empty() || track.keyframes.is_empty() {
                return Err("track target and at least one keyframe are required".into());
            }
            let mut ids = BTreeSet::new();
            let mut ticks = BTreeSet::new();
            for key in &track.keyframes {
                if !ids.insert(&key.keyframe_id) || !ticks.insert(key.tick) {
                    return Err("duplicate keyframe id or tick in track".into());
                }
                if key.tick < 0 || key.tick > self.duration_ticks {
                    return Err("keyframe outside clip".into());
                }
                if matches!(key.curve, Curve::Bezier) && key.bezier_milli.is_none() {
                    return Err("Bezier curve requires controls".into());
                }
            }
            if track
                .keyframes
                .windows(2)
                .any(|pair| pair[0].tick >= pair[1].tick)
            {
                return Err("keyframes must be stored in strictly increasing tick order".into());
            }
        }
        Ok(())
    }
    pub fn stable_sort(&mut self) {
        self.tracks.sort_by(|a, b| {
            (&a.target_id, &a.channel, &a.track_id).cmp(&(&b.target_id, &b.channel, &b.track_id))
        });
        for track in &mut self.tracks {
            track
                .keyframes
                .sort_by_key(|v| (v.tick, v.keyframe_id.clone()))
        }
    }
    pub fn move_keyframe(
        &mut self,
        track_id: &str,
        keyframe_id: &str,
        to_tick: i64,
    ) -> Result<i64, String> {
        if to_tick < 0 || to_tick > self.duration_ticks {
            return Err("destination tick outside clip".into());
        }
        let before = self.clone();
        let track = self
            .tracks
            .iter_mut()
            .find(|v| v.track_id == track_id)
            .ok_or("unknown track")?;
        if track
            .keyframes
            .iter()
            .any(|v| v.tick == to_tick && v.keyframe_id != keyframe_id)
        {
            return Err("destination tick occupied".into());
        }
        let key = track
            .keyframes
            .iter_mut()
            .find(|v| v.keyframe_id == keyframe_id)
            .ok_or("unknown keyframe")?;
        let old = key.tick;
        key.tick = to_tick;
        self.stable_sort();
        if let Err(e) = self.validate() {
            *self = before;
            return Err(e);
        }
        self.revision += 1;
        Ok(old)
    }
}

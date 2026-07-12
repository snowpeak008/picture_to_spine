use crate::{motion::registry::requires_hit_frame, validate_action_key};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GameplayMarkerKind {
    HitFrame,
    HitWindow,
    Footstep,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayMarker {
    pub marker_id: String,
    pub action_key: String,
    pub kind: GameplayMarkerKind,
    pub start_tick: i64,
    pub end_tick: i64,
    pub socket_id: Option<String>,
}

pub fn validate_markers(
    action_key: &str,
    duration: i64,
    markers: &[GameplayMarker],
) -> Result<(), String> {
    validate_action_key(action_key).map_err(|e| e.to_string())?;
    let mut ids = BTreeSet::new();
    for marker in markers {
        if marker.action_key != action_key || !ids.insert(&marker.marker_id) {
            return Err("marker action or id invalid".into());
        }
        if marker.start_tick < 0
            || marker.end_tick < marker.start_tick
            || marker.end_tick > duration
        {
            return Err("marker range outside clip".into());
        }
        if matches!(
            marker.kind,
            GameplayMarkerKind::HitFrame | GameplayMarkerKind::HitWindow
        ) && marker
            .socket_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .is_none()
        {
            return Err("hit marker requires a reviewed socket".into());
        }
    }
    let hits = markers
        .iter()
        .filter(|v| matches!(v.kind, GameplayMarkerKind::HitFrame))
        .count();
    if requires_hit_frame(action_key) {
        if hits != 1 {
            return Err("each attack requires exactly one hit-frame marker".into());
        }
    } else if markers.iter().any(|marker| {
        matches!(
            marker.kind,
            GameplayMarkerKind::HitFrame | GameplayMarkerKind::HitWindow
        )
    }) {
        return Err("non-attack actions cannot contain hit markers".into());
    }
    Ok(())
}

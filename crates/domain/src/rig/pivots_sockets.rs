use super::bone_tree::BoneTree;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPoint {
    pub x_milli_px: i64,
    pub y_milli_px: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SocketKind {
    PrimaryWeapon,
    GameplayOrigin,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pivot {
    pub layer_id: String,
    pub point: LocalPoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Socket {
    pub socket_id: String,
    pub bone_id: String,
    pub kind: SocketKind,
    pub point: LocalPoint,
    pub semantic: String,
}

pub fn validate_pivots_and_sockets(
    pivots: &[Pivot],
    sockets: &[Socket],
    bones: &BoneTree,
    primary_weapon: Option<&str>,
) -> Result<(), String> {
    let bone_ids: BTreeSet<_> = bones.bones.iter().map(|v| v.bone_id.as_str()).collect();
    let weapon: Vec<_> = sockets
        .iter()
        .filter(|v| v.kind == SocketKind::PrimaryWeapon)
        .collect();
    if primary_weapon
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .is_none()
    {
        return Err("primary weapon semantic unresolved".into());
    }
    if weapon.len() != 1 {
        return Err("exactly one primary weapon socket required".into());
    }
    let mut ids = BTreeSet::new();
    for socket in sockets {
        if !ids.insert(&socket.socket_id) {
            return Err("duplicate socket id".into());
        }
        if !bone_ids.contains(socket.bone_id.as_str()) {
            return Err("socket references unknown bone".into());
        }
        if socket.semantic.trim().is_empty() {
            return Err("socket semantic required".into());
        }
    }
    let layers: BTreeSet<_> = pivots.iter().map(|p| &p.layer_id).collect();
    if layers.len() != pivots.len() {
        return Err("duplicate layer pivot".into());
    }
    Ok(())
}

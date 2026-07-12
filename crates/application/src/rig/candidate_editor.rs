use f2s_domain::rig::{
    RigApprovalState, RigCandidate,
    bone_tree::RestTransform,
    pivots_sockets::{LocalPoint, validate_pivots_and_sockets},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetBoneTransformCommand {
    pub expected_revision: u64,
    pub bone_id: String,
    pub rest: RestTransform,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReparentBoneCommand {
    pub expected_revision: u64,
    pub bone_id: String,
    pub parent_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetPivotCommand {
    pub expected_revision: u64,
    pub layer_id: String,
    pub point: LocalPoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetSocketCommand {
    pub expected_revision: u64,
    pub socket_id: String,
    pub bone_id: String,
    pub point: LocalPoint,
    pub semantic: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetSlotCommand {
    pub expected_revision: u64,
    pub slot_id: String,
    pub bone_id: String,
    pub draw_key: i32,
}

pub fn set_bone_transform(
    candidate: &mut RigCandidate,
    command: SetBoneTransformCommand,
) -> Result<(), String> {
    require_current(candidate, command.expected_revision)?;
    let mut next = candidate.clone();
    next.bone_tree
        .set_rest_transform(&command.bone_id, command.rest)?;
    next.mark_edited()?;
    *candidate = next;
    Ok(())
}

pub fn reparent_bone(
    candidate: &mut RigCandidate,
    command: ReparentBoneCommand,
) -> Result<(), String> {
    require_current(candidate, command.expected_revision)?;
    let mut next = candidate.clone();
    let previous = next
        .bone_tree
        .bones
        .iter()
        .find(|bone| bone.bone_id == command.bone_id)
        .ok_or("unknown bone")?
        .parent_id
        .as_deref();
    if previous == Some(command.parent_id.as_str()) {
        return Err("bone parent is unchanged".into());
    }
    next.bone_tree
        .reparent(&command.bone_id, &command.parent_id)?;
    next.mark_edited()?;
    *candidate = next;
    Ok(())
}

pub fn set_layer_pivot(
    candidate: &mut RigCandidate,
    command: SetPivotCommand,
) -> Result<(), String> {
    require_current(candidate, command.expected_revision)?;
    if !candidate.canvas.contains_local_point(command.point) {
        return Err("pivot lies outside the supported Rig canvas".into());
    }
    let mut next = candidate.clone();
    let pivot = next
        .pivots
        .iter_mut()
        .find(|pivot| pivot.layer_id == command.layer_id)
        .ok_or("unknown layer pivot")?;
    if pivot.point == command.point {
        return Err("layer pivot is unchanged".into());
    }
    pivot.point = command.point;
    next.pivot_socket_revision = next
        .pivot_socket_revision
        .checked_add(1)
        .ok_or("pivot/socket revision overflow")?;
    next.mark_edited()?;
    *candidate = next;
    Ok(())
}

pub fn set_socket(candidate: &mut RigCandidate, command: SetSocketCommand) -> Result<(), String> {
    require_current(candidate, command.expected_revision)?;
    if command.semantic.trim().is_empty() {
        return Err("socket semantic required".into());
    }
    if !candidate.canvas.contains_local_point(command.point) {
        return Err("socket lies outside the supported Rig canvas".into());
    }
    let mut next = candidate.clone();
    let socket = next
        .sockets
        .iter_mut()
        .find(|socket| socket.socket_id == command.socket_id)
        .ok_or("unknown socket")?;
    if socket.bone_id == command.bone_id
        && socket.point == command.point
        && socket.semantic == command.semantic
    {
        return Err("socket is unchanged".into());
    }
    socket.bone_id = command.bone_id;
    socket.point = command.point;
    socket.semantic = command.semantic;
    validate_pivots_and_sockets(
        &next.pivots,
        &next.sockets,
        &next.bone_tree,
        Some(&next.primary_weapon.socket_semantic),
    )?;
    let primary = next
        .sockets
        .iter()
        .find(|socket| socket.socket_id == command.socket_id)
        .expect("socket found before edit");
    if matches!(
        primary.kind,
        f2s_domain::rig::pivots_sockets::SocketKind::PrimaryWeapon
    ) && primary.semantic != next.primary_weapon.socket_semantic
    {
        return Err("primary weapon socket semantic differs from StyleSpec".into());
    }
    next.pivot_socket_revision = next
        .pivot_socket_revision
        .checked_add(1)
        .ok_or("pivot/socket revision overflow")?;
    next.mark_edited()?;
    *candidate = next;
    Ok(())
}

pub fn set_slot(candidate: &mut RigCandidate, command: SetSlotCommand) -> Result<(), String> {
    require_current(candidate, command.expected_revision)?;
    let mut next = candidate.clone();
    next.slot_set.set_binding_and_draw_key(
        &command.slot_id,
        &command.bone_id,
        command.draw_key,
        &next.bone_tree,
    )?;
    next.mark_edited()?;
    *candidate = next;
    Ok(())
}

fn require_current(candidate: &RigCandidate, expected_revision: u64) -> Result<(), String> {
    if candidate.revision != expected_revision {
        Err("stale Rig candidate revision".into())
    } else if candidate.approval_state != RigApprovalState::Pending
        && candidate.approval_state != RigApprovalState::Approved
    {
        // This branch is intentionally exhaustive against future enum variants.
        Err("unsupported Rig approval state".into())
    } else {
        Ok(())
    }
}

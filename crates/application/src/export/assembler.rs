use super::{
    preflight::{valid_sha256, validate_export_id, validate_relative_png_path},
    publish_snapshot::{ActionApprovalBinding, AttachmentSnapshot, PublishSnapshot},
};
use f2s_domain::{
    ACTION_KEYS,
    animation::markers::GameplayMarkerKind,
    canonical::canonical_sha256,
    governance::Approval,
    motion::registry::requires_hit_frame,
    project::ProjectManifest,
    rig::{SPINE_CAPABILITY_ID, SPINE_PATCH, pivots_sockets::SocketKind},
};

/// Builds the immutable, exporter-facing view exclusively from the current project aggregates.
///
/// This is the authority boundary for built-in export. Callers must not assemble a
/// `PublishSnapshot` from UI projections or previously cached hashes: all approval payloads and
/// cross-aggregate bindings are recomputed here from the current manifest.
pub fn assemble_publish_snapshot(
    project: &ProjectManifest,
    export_id: impl Into<String>,
) -> Result<PublishSnapshot, String> {
    let export_id = export_id.into();
    validate_export_id(&export_id).map_err(|detail| assembly_error("EXPORT_ID_INVALID", detail))?;
    project
        .validate_cross_aggregate()
        .map_err(|detail| assembly_error("PROJECT_AGGREGATE_INVALID", detail))?;

    if project.time_base.numerator <= 0 || project.time_base.denominator <= 0 {
        return Err(assembly_error(
            "TIMEBASE_INVALID",
            "project timebase must be a positive rational",
        ));
    }

    let master = project
        .active_master
        .as_ref()
        .ok_or_else(|| assembly_error("MASTER_MISSING", "active master is required"))?;
    let master_approval = project.current_master_approval().ok_or_else(|| {
        assembly_error(
            "MASTER_APPROVAL_MISSING",
            "active master has no current canonical approval",
        )
    })?;
    validate_approval_record(master_approval, "master")?;

    let layer_set = project.active_layer_set.as_ref().ok_or_else(|| {
        assembly_error("LAYER_SET_MISSING", "active approved LayerSet is required")
    })?;
    let layer_approval = project.current_layer_approval().ok_or_else(|| {
        assembly_error(
            "LAYER_APPROVAL_MISSING",
            "active LayerSet has no current canonical approval",
        )
    })?;
    validate_approval_record(layer_approval, "layers")?;

    let rig = project
        .active_rig
        .as_ref()
        .ok_or_else(|| assembly_error("RIG_MISSING", "active approved Rig is required"))?;
    let rig_approval = project.current_rig_approval().ok_or_else(|| {
        assembly_error(
            "RIG_APPROVAL_MISSING",
            "active Rig has no current canonical approval",
        )
    })?;
    validate_approval_record(rig_approval, "rig")?;

    let motion = project.motion_content.as_ref().ok_or_else(|| {
        assembly_error(
            "MOTION_CONTENT_MISSING",
            "current MotionContent is required",
        )
    })?;
    if !motion.all_required_assets_approved() {
        return Err(assembly_error(
            "KEY_POSE_ASSET_APPROVALS_INCOMPLETE",
            "every required key-pose image must have a current human approval before export",
        ));
    }
    let animation = project.animation_set.as_ref().ok_or_else(|| {
        assembly_error("ANIMATION_SET_MISSING", "current AnimationSet is required")
    })?;

    let style_weapon = master.style_spec.primary_weapon.as_ref().ok_or_else(|| {
        assembly_error(
            "PRIMARY_WEAPON_MISSING",
            "approved StyleSpec must contain one primary weapon",
        )
    })?;
    if style_weapon != &rig.primary_weapon {
        return Err(assembly_error(
            "PRIMARY_WEAPON_STALE",
            "Rig primary weapon differs from the approved StyleSpec",
        ));
    }
    if rig.layer_set_approval_sha256 != layer_approval.target_sha256 {
        return Err(assembly_error(
            "LAYER_APPROVAL_BINDING_STALE",
            "Rig is not bound to the current LayerSet approval payload",
        ));
    }
    if animation.approved_rig_sha256 != rig_approval.target_sha256 {
        return Err(assembly_error(
            "RIG_APPROVAL_BINDING_STALE",
            "AnimationSet is not bound to the current Rig approval payload",
        ));
    }

    rig.constraint_capability
        .validate_verified()
        .map_err(|detail| assembly_error("SPINE_CAPABILITY_UNVERIFIED", detail))?;
    if rig.constraint_capability.capability_id != SPINE_CAPABILITY_ID
        || rig.constraint_capability.spine_patch != SPINE_PATCH
    {
        return Err(assembly_error(
            "SPINE_PATCH_MISMATCH",
            "only the verified Spine 4.2.43 contract is exportable",
        ));
    }

    let weapon_sockets = rig
        .sockets
        .iter()
        .filter(|socket| socket.kind == SocketKind::PrimaryWeapon)
        .collect::<Vec<_>>();
    let [weapon_socket] = weapon_sockets.as_slice() else {
        return Err(assembly_error(
            "PRIMARY_WEAPON_SOCKET_INVALID",
            "exactly one primary weapon socket is required",
        ));
    };
    if weapon_socket.semantic != style_weapon.socket_semantic {
        return Err(assembly_error(
            "PRIMARY_WEAPON_SOCKET_STALE",
            "primary weapon socket semantic differs from the approved StyleSpec",
        ));
    }

    for spec in &motion.specs {
        if spec.time_base != project.time_base {
            return Err(assembly_error(
                "TIMEBASE_MISMATCH",
                format!(
                    "MotionSpec {} does not use the project timebase",
                    spec.action_key
                ),
            ));
        }
    }
    for clip in &animation.clips {
        if clip.time_base != project.time_base {
            return Err(assembly_error(
                "TIMEBASE_MISMATCH",
                format!("clip {} does not use the project timebase", clip.action_key),
            ));
        }
    }

    // The current Spine writer stores one local coordinate per vertex. It cannot faithfully
    // represent a separate local coordinate for every influence, so genuinely skinned vertices
    // must remain blocked until that serializer contract is extended.
    for weights in &rig.weights {
        if weights
            .by_vertex
            .values()
            .any(|influences| influences.len() != 1 || influences[0].weight_ppm != 1_000_000)
        {
            return Err(assembly_error(
                "MULTI_BONE_WEIGHTS_UNSUPPORTED",
                format!(
                    "mesh {} contains a vertex that is not rigidly bound to one bone",
                    weights.mesh_id
                ),
            ));
        }
    }

    let hit_frames = animation
        .gameplay_markers
        .iter()
        .filter(|marker| marker.kind == GameplayMarkerKind::HitFrame)
        .collect::<Vec<_>>();
    if hit_frames.len() != 3
        || hit_frames.iter().any(|marker| {
            !requires_hit_frame(&marker.action_key)
                || marker.socket_id.as_deref() != Some(weapon_socket.socket_id.as_str())
        })
    {
        return Err(assembly_error(
            "HIT_FRAME_SET_INVALID",
            "the three attack hit frames must reference the reviewed primary weapon socket",
        ));
    }

    let mut action_approvals = Vec::with_capacity(ACTION_KEYS.len());
    for action_key in ACTION_KEYS {
        let clip = animation.clip(action_key).map_err(|detail| {
            assembly_error("ACTION_CLIP_MISSING", format!("{action_key}: {detail}"))
        })?;
        let pose_payload = animation
            .pose_payload(motion, action_key)
            .map_err(|detail| {
                assembly_error(
                    "POSE_APPROVAL_PAYLOAD_INVALID",
                    format!("{action_key}: {detail}"),
                )
            })?;
        let pose_approval = project.current_pose_approval(action_key).ok_or_else(|| {
            assembly_error(
                "POSE_APPROVAL_MISSING",
                format!("no current pose approval for {action_key}"),
            )
        })?;
        validate_action_approval(
            pose_approval,
            "poses",
            action_key,
            clip.revision,
            &pose_payload,
        )?;

        let hit_approval_sha256 = if requires_hit_frame(action_key) {
            let hit_payload = animation.hit_payload(action_key).map_err(|detail| {
                assembly_error(
                    "HIT_APPROVAL_PAYLOAD_INVALID",
                    format!("{action_key}: {detail}"),
                )
            })?;
            let hit_approval = project.current_hit_approval(action_key).ok_or_else(|| {
                assembly_error(
                    "HIT_APPROVAL_MISSING",
                    format!("no current hit approval for {action_key}"),
                )
            })?;
            validate_action_approval(
                hit_approval,
                "hits",
                action_key,
                clip.revision,
                &hit_payload,
            )?;
            Some(approval_record_sha256(hit_approval)?)
        } else {
            None
        };

        action_approvals.push(ActionApprovalBinding {
            action_key: action_key.into(),
            clip_sha256: canonical_sha256(clip).map_err(|detail| {
                assembly_error("CLIP_HASH_FAILED", format!("{action_key}: {detail}"))
            })?,
            pose_approval_sha256: approval_record_sha256(pose_approval)?,
            hit_approval_sha256,
        });
    }
    if !project.approval_closure_complete() {
        return Err(assembly_error(
            "APPROVAL_CLOSURE_INCOMPLETE",
            "ten pose approvals and three attack hit approvals are required",
        ));
    }

    let mut attachments = Vec::with_capacity(layer_set.layers.len());
    for (position, slot) in rig.slot_set.stable_draw_order().into_iter().enumerate() {
        let layer = layer_set
            .layers
            .iter()
            .find(|layer| layer.layer_id == slot.layer_id)
            .ok_or_else(|| {
                assembly_error(
                    "ATTACHMENT_LAYER_MISSING",
                    format!("slot {} references a missing layer", slot.slot_id),
                )
            })?;
        if !valid_sha256(&layer.attachment_sha256) {
            return Err(assembly_error(
                "ATTACHMENT_HASH_INVALID",
                format!("layer {} has an invalid attachment hash", layer.layer_id),
            ));
        }
        let logical_png_path = format!("images/layer-{position:03}.png");
        validate_relative_png_path(&logical_png_path)
            .map_err(|detail| assembly_error("ATTACHMENT_PATH_INVALID", detail))?;
        attachments.push(AttachmentSnapshot {
            attachment_id: layer.layer_id.clone(),
            slot_id: slot.slot_id.clone(),
            logical_png_path,
            source_sha256: layer.attachment_sha256.clone(),
            width: rig.canvas.width_px,
            height: rig.canvas.height_px,
        });
    }

    Ok(PublishSnapshot {
        export_id,
        rig_id: rig.rig_id.clone(),
        project_revision: project.revision,
        approved_layer_set_hash: layer_approval.target_sha256.clone(),
        approved_rig_hash: rig_approval.target_sha256.clone(),
        action_approvals,
        capability_id: SPINE_CAPABILITY_ID.into(),
        spine_patch: SPINE_PATCH.into(),
        primary_weapon: style_weapon.weapon_type.clone(),
        time_base: project.time_base,
        bones: rig.bone_tree.clone(),
        slots: rig.slot_set.clone(),
        pivots: rig.pivots.clone(),
        sockets: rig.sockets.clone(),
        meshes: rig.meshes.clone(),
        weights: rig.weights.clone(),
        constraints: rig.constraints.clone(),
        constraint_capability: rig.constraint_capability.clone(),
        attachments,
        clips: animation.clips.clone(),
        markers: animation.gameplay_markers.clone(),
    })
}

fn validate_action_approval(
    approval: &Approval,
    gate_id: &str,
    action_key: &str,
    clip_revision: u64,
    expected_payload: &str,
) -> Result<(), String> {
    validate_approval_record(approval, gate_id)?;
    if !approval.is_valid_for(action_key, clip_revision, expected_payload) {
        return Err(assembly_error(
            "ACTION_APPROVAL_STALE",
            format!("{gate_id} approval for {action_key} is not current"),
        ));
    }
    Ok(())
}

fn validate_approval_record(approval: &Approval, expected_gate: &str) -> Result<(), String> {
    if approval.gate_id != expected_gate
        || approval.invalidated
        || approval.approval_id.trim().is_empty()
        || approval.actor_id.trim().is_empty()
        || approval.approved_at_utc.trim().is_empty()
        || !valid_sha256(&approval.target_sha256)
    {
        return Err(assembly_error(
            "APPROVAL_RECORD_INVALID",
            format!("{expected_gate} approval is incomplete, invalidated, or malformed"),
        ));
    }
    Ok(())
}

fn approval_record_sha256(approval: &Approval) -> Result<String, String> {
    canonical_sha256(approval)
        .map_err(|detail| assembly_error("APPROVAL_HASH_FAILED", detail.to_string()))
}

fn assembly_error(code: &str, detail: impl AsRef<str>) -> String {
    format!("{code}: {}", detail.as_ref())
}

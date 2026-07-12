use super::publish_snapshot::PublishSnapshot;
use f2s_domain::{
    ACTION_KEYS,
    animation::markers::validate_markers,
    canonical::canonical_sha256,
    motion::registry::requires_hit_frame,
    rig::{constraints::validate_constraints, pivots_sockets::validate_pivots_and_sockets},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};

pub const FORBIDDEN_BUILTIN_EXTENSIONS: [&str; 3] = [".atlas", ".spine", ".skel"];
const WINDOWS_RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreflight {
    pub passed: bool,
    pub checks: Vec<String>,
    pub errors: Vec<String>,
    pub external_editor_status: String,
    pub publish_status: String,
}

pub fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|v| v.is_ascii_hexdigit() && !v.is_ascii_uppercase())
}

pub fn validate_export_id(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 80
        || value == "."
        || value == ".."
        || value.ends_with(['.', ' '])
        || value.bytes().any(|v| {
            matches!(
                v,
                b'/' | b'\\' | b':' | b'*' | b'?' | b'"' | b'<' | b'>' | b'|'
            )
        })
        || !value
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || matches!(v, b'-' | b'_'))
    {
        return Err("export id is not a safe Windows directory name".into());
    }
    if WINDOWS_RESERVED.contains(&value.to_ascii_uppercase().as_str()) {
        return Err("export id is a reserved Windows device name".into());
    }
    Ok(())
}

pub fn validate_relative_png_path(value: &str) -> Result<(), String> {
    if value.contains(':') || value.contains('\0') || value.ends_with(['.', ' ']) {
        return Err("attachment path contains forbidden Windows syntax".into());
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err("attachment path must be relative".into());
    }
    let components = path.components().collect::<Vec<_>>();
    if components.len() < 2 || components.first() != Some(&Component::Normal("images".as_ref())) {
        return Err("attachment path must live under images/".into());
    }
    for component in components {
        let Component::Normal(part) = component else {
            return Err("attachment path contains root, prefix, or dot traversal".into());
        };
        let text = part.to_str().ok_or("attachment path must be Unicode")?;
        let base = text.split('.').next().unwrap_or("").to_ascii_uppercase();
        if text.is_empty()
            || text.ends_with(['.', ' '])
            || WINDOWS_RESERVED.contains(&base.as_str())
        {
            return Err("attachment path has unsafe component".into());
        }
    }
    if path
        .extension()
        .and_then(|v| v.to_str())
        .map(|v| v.eq_ignore_ascii_case("png"))
        != Some(true)
    {
        return Err("attachment output must be PNG".into());
    }
    Ok(())
}

pub fn preflight(snapshot: &PublishSnapshot) -> ExportPreflight {
    let mut checks = Vec::new();
    let mut errors = Vec::new();
    if validate_export_id(&snapshot.export_id).is_err() {
        errors.push("EXPORT_ID_UNSAFE".into());
    }
    if snapshot.pinned_capability() {
        checks.push("SPINE_PATCH_EXACT".into());
    } else {
        errors.push("SPINE_PATCH_MISMATCH".into());
    }
    if !valid_sha256(&snapshot.approved_layer_set_hash)
        || !valid_sha256(&snapshot.approved_rig_hash)
    {
        errors.push("APPROVAL_HASH_INVALID".into());
    }
    if snapshot.bones.validate().is_err() {
        errors.push("BONE_TREE_INVALID".into());
    }
    let required_layers = snapshot
        .attachments
        .iter()
        .map(|v| v.attachment_id.clone())
        .collect::<Vec<_>>();
    if snapshot
        .slots
        .validate(&required_layers, &snapshot.bones)
        .is_err()
    {
        errors.push("SLOT_MAPPING_INVALID".into());
    }
    if validate_pivots_and_sockets(
        &snapshot.pivots,
        &snapshot.sockets,
        &snapshot.bones,
        Some(&snapshot.primary_weapon),
    )
    .is_err()
    {
        errors.push("PIVOT_SOCKET_INVALID".into());
    }
    for mesh in &snapshot.meshes {
        if mesh.validate().is_err() {
            errors.push(format!("MESH_INVALID:{}", mesh.mesh_id));
        }
        match snapshot.weights.iter().find(|v| v.mesh_id == mesh.mesh_id) {
            Some(weights) if weights.validate(mesh, &snapshot.bones).is_ok() => {}
            _ => errors.push(format!("WEIGHTS_INVALID:{}", mesh.mesh_id)),
        }
    }
    if snapshot.weights.iter().any(|weights| {
        weights
            .by_vertex
            .values()
            .any(|influences| influences.len() != 1 || influences[0].weight_ppm != 1_000_000)
    }) {
        errors.push("MULTI_BONE_WEIGHTS_UNSUPPORTED".into());
    }
    if validate_constraints(
        &snapshot.constraints,
        &snapshot.bones,
        &snapshot.constraint_capability,
    )
    .is_err()
    {
        errors.push("CONSTRAINTS_INVALID".into());
    }
    let actions = snapshot
        .clips
        .iter()
        .map(|v| v.action_key.as_str())
        .collect::<Vec<_>>();
    if actions != ACTION_KEYS {
        errors.push("TEN_ACTION_SET_INVALID".into());
    }
    let approval_actions = snapshot
        .action_approvals
        .iter()
        .map(|v| v.action_key.as_str())
        .collect::<Vec<_>>();
    if approval_actions != ACTION_KEYS {
        errors.push("ACTION_APPROVAL_SET_INVALID".into());
    }
    for clip in &snapshot.clips {
        if clip.validate().is_err() {
            errors.push(format!("CLIP_INVALID:{}", clip.action_key));
        }
        if clip.tracks.is_empty() {
            errors.push(format!("CLIP_EMPTY:{}", clip.action_key));
        }
        if clip.time_base != snapshot.time_base {
            errors.push(format!("TIMEBASE_MISMATCH:{}", clip.action_key));
        }
        let markers = snapshot
            .markers
            .iter()
            .filter(|v| v.action_key == clip.action_key)
            .cloned()
            .collect::<Vec<_>>();
        if requires_hit_frame(&clip.action_key)
            && validate_markers(&clip.action_key, clip.duration_ticks, &markers).is_err()
        {
            errors.push(format!("HIT_MARKER_INVALID:{}", clip.action_key));
        }
        match snapshot
            .action_approvals
            .iter()
            .find(|v| v.action_key == clip.action_key)
        {
            Some(binding) => {
                let actual = canonical_sha256(clip).unwrap_or_default();
                if binding.clip_sha256 != actual || !valid_sha256(&binding.pose_approval_sha256) {
                    errors.push(format!("ACTION_APPROVAL_STALE:{}", clip.action_key));
                }
                if requires_hit_frame(&clip.action_key) {
                    if !binding
                        .hit_approval_sha256
                        .as_deref()
                        .map(valid_sha256)
                        .unwrap_or(false)
                    {
                        errors.push(format!("HIT_APPROVAL_MISSING:{}", clip.action_key));
                    }
                } else if binding.hit_approval_sha256.is_some() {
                    errors.push(format!("UNEXPECTED_HIT_APPROVAL:{}", clip.action_key));
                }
            }
            None => errors.push(format!("ACTION_APPROVAL_MISSING:{}", clip.action_key)),
        }
    }
    let mut paths = BTreeSet::new();
    for attachment in &snapshot.attachments {
        if validate_relative_png_path(&attachment.logical_png_path).is_err()
            || !valid_sha256(&attachment.source_sha256)
        {
            errors.push(format!(
                "ATTACHMENT_PATH_OR_HASH_INVALID:{}",
                attachment.attachment_id
            ));
        }
        if !paths.insert(attachment.logical_png_path.to_ascii_lowercase()) {
            errors.push("DUPLICATE_ATTACHMENT_PATH".into());
        }
    }
    if errors.is_empty() {
        checks.extend([
            "APPROVAL_CLOSURE_VALID".into(),
            "BUILTIN_OUTPUT_SUBSET_OPEN".into(),
            "PATHS_CONFINED".into(),
        ]);
    }
    let passed = errors.is_empty();
    ExportPreflight {
        passed,
        checks,
        errors,
        external_editor_status: "EXTERNAL".into(),
        publish_status: if passed {
            "EXPORTED_UNVERIFIED".into()
        } else {
            "BLOCKED".into()
        },
    }
}

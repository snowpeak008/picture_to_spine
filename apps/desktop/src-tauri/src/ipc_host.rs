use crate::{
    diagnostics_export::{choose_and_write_report, redacted_report_bytes},
    spine_cli_host::{OpenExportGrant, SpineCliHost},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use f2s_adapters::{
    export::{
        package::{AttachmentBytes, commit_open_export},
        psd::psd_layer_from_png,
    },
    image::{
        DecodedImageReport, apply_mask_stroke, changed_attachment_pixels, decode_image_bounded,
        initial_mask, normalize_manual_layer_png, recomposition_metrics, render_masked_png,
        render_safe_preview_png, render_updated_layer_attachment_png,
    },
    safety::private_remote::{ExternalPrivateRemoteNotRun, FsRemoteGpuProfileStore},
    storage::{FsCas, FsProjectStore, ntfs_atomic::write_atomic},
};
use f2s_application::{
    animation::{
        approve_action_hit, approve_action_poses, diagnose_animation_set, initialize_animation_set,
        put_track, set_hit_frame_marker, set_review_pose_tick,
    },
    approvals::{
        HumanCredentialVerifier, VerifiedHumanActor, approve, master_rejection_payload,
        reject_master,
    },
    export::{assemble_publish_snapshot, preflight::preflight},
    import::promote_image,
    layers::{
        ApplyMaskStroke, add_layer, approve_layers, layer_approval_payload,
        recomposition_is_approvable, remove_optional_layer, reorder_layers,
        replace_layer_attachment,
    },
    master::create_master,
    motion::{
        approve_key_pose_asset, bind_key_pose_image, completeness::content_matrix,
        initialize_motion_content, replace_motion_spec, set_key_pose_alignment,
    },
    ports::{
        CasStore, ExternalCapabilityState, ImageFacts, IpcMethod, IpcRequest, IpcResponse,
        PrivateRemoteGpuTransport, ProjectStore, RemoteGpuProfileStore,
    },
    project::{create_project, open_project},
    rig::{
        ReparentBoneCommand, SetBoneTransformCommand, SetPivotCommand, SetSlotCommand,
        SetSocketCommand, approve_rig_candidate, diagnose_rig_candidate, reparent_bone,
        rig_approval_payload, set_bone_transform, set_layer_pivot, set_slot, set_socket,
    },
    storage::commit_project,
};
use f2s_domain::{
    animation::clip::Track,
    canonical::canonical_sha256,
    governance::CredentialAttestation,
    import::{ImportLimits, SourceArtifact},
    layers::{Layer, LayerRole, LayerSet, PixelOrigin, PixelProvenance, RecompositionMetrics},
    master::StyleSpec,
    motion::spec::MotionSpec,
    project::{ExportRecord, ProjectManifest},
    remote_gpu::RemoteGpuProfile,
    rig::{RigCanvas, build_default_side_view_humanoid_rig, constraints::ConstraintCapability},
    storage::CasRef,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime},
};
use uuid::Uuid;
use windows::{
    Win32::{
        Foundation::{ERROR_CANCELLED, HWND},
        System::{
            Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree},
            SystemInformation::GetSystemTime,
        },
        UI::{
            Controls::Dialogs::*,
            Shell::{
                FOS_FORCEFILESYSTEM, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS, FileOpenDialog,
                IFileOpenDialog, SIGDN_FILESYSPATH,
            },
            WindowsAndMessaging::{IDYES, MB_ICONQUESTION, MB_YESNO, MessageBoxW},
        },
    },
    core::{HRESULT, PCWSTR, PWSTR},
};

const ANIMATION_PREVIEW_RASTER_MAX_SIDE_PX: u32 = 256;
const ANIMATION_PREVIEW_DATA_URL_BUDGET_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone)]
enum PendingImportPurpose {
    MasterSource,
    LayerReplacement { layer_id: String },
    KeyPose { asset_spec_id: String },
}

#[derive(Debug, Clone)]
struct PendingImport {
    path: PathBuf,
    report: DecodedImageReport,
    purpose: PendingImportPurpose,
    project_id: String,
    project_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MasterReviewGrant {
    project_id: String,
    project_revision: u64,
    master_id: String,
    approval_payload_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KeyPoseReview {
    project_id: String,
    project_revision: u64,
    binding_id: String,
    source_sha256: String,
}

const REMOTE_GPU_PROFILE_MAX_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ActiveRemoteGpuProfilePointer {
    schema_version: String,
    profile_id: String,
    profile_sha256: String,
}

impl ActiveRemoteGpuProfilePointer {
    fn for_profile(profile: &RemoteGpuProfile) -> Result<Self, String> {
        Ok(Self {
            schema_version: "1.0.0".into(),
            profile_id: profile.profile_id.clone(),
            profile_sha256: profile.canonical_sha256()?,
        })
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema_version != "1.0.0"
            || self.profile_id.is_empty()
            || self.profile_sha256.len() != 64
            || !self
                .profile_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err("active private remote GPU pointer is invalid".into());
        }
        Ok(())
    }
}

pub struct HostState {
    pending: RefCell<HashMap<String, PendingImport>>,
    current_project: RefCell<Option<ProjectManifest>>,
    key_pose_reviews: RefCell<HashMap<String, KeyPoseReview>>,
    master_reviews: RefCell<HashMap<String, MasterReviewGrant>>,
    consumed_attestations: RefCell<HashSet<String>>,
    session_nonce: String,
    spine_cli: SpineCliHost,
}
impl Default for HostState {
    fn default() -> Self {
        Self {
            pending: RefCell::new(HashMap::new()),
            current_project: RefCell::new(None),
            key_pose_reviews: RefCell::new(HashMap::new()),
            master_reviews: RefCell::new(HashMap::new()),
            consumed_attestations: RefCell::new(HashSet::new()),
            session_nonce: Uuid::new_v4().simple().to_string(),
            spine_cli: SpineCliHost::default(),
        }
    }
}

struct NativeHumanVerifier<'a> {
    consumed: &'a RefCell<HashSet<String>>,
    session_nonce: &'a str,
}
impl HumanCredentialVerifier for NativeHumanVerifier<'_> {
    fn verify_and_consume(&self, attestation: &CredentialAttestation) -> Result<(), String> {
        if attestation.credential_ref != "windows-session/native-confirmation" {
            return Err("untrusted human credential source".into());
        }
        let expected_proof = format!(
            "{:x}",
            Sha256::digest(
                format!(
                    "{}:{}:{}:{}",
                    self.session_nonce,
                    attestation.attestation_id,
                    attestation.purpose,
                    attestation.payload_sha256
                )
                .as_bytes()
            )
        );
        if attestation.verification_proof_sha256 != expected_proof
            || attestation.issued_at_utc >= attestation.expires_at_utc
        {
            return Err("native confirmation proof or lifetime is invalid".into());
        }
        if !self
            .consumed
            .borrow_mut()
            .insert(attestation.attestation_id.clone())
        {
            return Err("human approval attestation replayed".into());
        }
        Ok(())
    }
}

fn local_data_root() -> Result<PathBuf, String> {
    let root = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or("LOCALAPPDATA is unavailable")?
        .join("FlashToSpine");
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    Ok(root)
}
fn stores() -> Result<(FsProjectStore, FsCas), String> {
    let root = local_data_root()?;
    let (key_id, key) = crate::local_security::load_or_create_project_integrity_key(&root)?;
    Ok((
        FsProjectStore::new_with_integrity_key(root.join("projects"), key_id, key)?,
        FsCas::new(root.join("cas")),
    ))
}

fn remote_gpu_root() -> Result<PathBuf, String> {
    let root = local_data_root()?.join("remote-gpu");
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    Ok(root)
}

fn remote_gpu_profile_store() -> Result<FsRemoteGpuProfileStore, String> {
    Ok(FsRemoteGpuProfileStore::new(remote_gpu_root()?))
}

fn active_remote_gpu_pointer_path() -> Result<PathBuf, String> {
    Ok(remote_gpu_root()?.join("active-profile.json"))
}

fn write_active_remote_gpu_profile(profile: &RemoteGpuProfile) -> Result<(), String> {
    let pointer = ActiveRemoteGpuProfilePointer::for_profile(profile)?;
    let bytes = serde_json::to_vec_pretty(&pointer).map_err(|error| error.to_string())?;
    write_atomic(&active_remote_gpu_pointer_path()?, &bytes)
}

fn load_active_remote_gpu_profile() -> Result<Option<RemoteGpuProfile>, String> {
    let path = active_remote_gpu_pointer_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = read_small_stable(
        &path,
        REMOTE_GPU_PROFILE_MAX_BYTES,
        "active profile pointer",
    )?;
    let pointer: ActiveRemoteGpuProfilePointer =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    pointer.validate()?;
    let profile = remote_gpu_profile_store()?
        .load_profile(&pointer.profile_id)?
        .ok_or("active private remote GPU profile file is missing")?;
    if profile.canonical_sha256()? != pointer.profile_sha256 {
        return Err("active private remote GPU profile hash does not match its pointer".into());
    }
    Ok(Some(profile))
}

fn parse_remote_gpu_profile(bytes: &[u8]) -> Result<RemoteGpuProfile, String> {
    let profile: RemoteGpuProfile =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid profile JSON: {error}"))?;
    profile.validate_configuration()?;
    Ok(profile)
}

fn archive_remote_gpu_profile(profile: &RemoteGpuProfile) -> Result<(), String> {
    let sha256 = profile.canonical_sha256()?;
    let path = remote_gpu_root()?
        .join("audit")
        .join(&profile.profile_id)
        .join(format!("{sha256}.json"));
    let bytes = serde_json::to_vec_pretty(profile).map_err(|error| error.to_string())?;
    if path.exists() {
        let existing = read_small_stable(&path, REMOTE_GPU_PROFILE_MAX_BYTES, "profile audit")?;
        let existing_profile = parse_remote_gpu_profile(&existing)?;
        if existing_profile.canonical_sha256()? == sha256 {
            return Ok(());
        }
        return Err("private remote GPU audit snapshot identity conflict".into());
    }
    fs::create_dir_all(path.parent().ok_or("profile audit path has no parent")?)
        .map_err(|error| error.to_string())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}

fn save_and_activate_remote_gpu_profile(profile: &RemoteGpuProfile) -> Result<(), String> {
    profile.validate_configuration()?;
    let store = remote_gpu_profile_store()?;
    if let Some(previous) = store.load_profile(&profile.profile_id)? {
        archive_remote_gpu_profile(&previous)?;
    }
    archive_remote_gpu_profile(profile)?;
    store.save_profile(profile)?;
    write_active_remote_gpu_profile(profile)
}

fn remote_gpu_status_projection() -> Result<Value, String> {
    let profile = load_active_remote_gpu_profile()?;
    let capability = ExternalPrivateRemoteNotRun.capability_report();
    let capability_state = match capability.state {
        ExternalCapabilityState::NotRunExternal => "NOT_RUN_EXTERNAL",
        ExternalCapabilityState::ContractMockOnly => "CONTRACT_MOCK_ONLY",
    };
    if capability.network_attempted {
        return Err("NOT_RUN remote transport reported an unexpected network attempt".into());
    }
    Ok(json!({
        "schemaVersion": "1.0.0",
        "activeProfile": profile,
        "credentialConfigured": Value::Null,
        "capability": {
            "state": capability_state,
            "transport": "EXTERNAL_NOT_INTEGRATED",
            "reason": capability.reason,
            "networkAttemptCount": 0,
            "networkAttempted": false
        },
        "policy": {
            "automaticConnection": false,
            "transportCredentialRead": false,
            "credentialPresenceCheck": "NOT_RUN_DEFAULT_NO_CREDENTIAL_READ",
            "publicProviderAllowed": false,
            "imageGenerationAllowed": false,
            "secretAcceptedByIpc": false,
            "profileStorage": "LOCAL_APPDATA_ONLY_NOT_PROJECT"
        }
    }))
}

fn diagnostics_status_projection(
    project: Option<&ProjectManifest>,
    spine_cli: &SpineCliHost,
) -> Result<Value, String> {
    let remote = remote_gpu_status_projection()?;
    let private_remote_gpu = match remote
        .get("activeProfile")
        .filter(|profile| !profile.is_null())
    {
        Some(profile) if profile.get("enabled").and_then(Value::as_bool) == Some(true) => {
            "CONFIGURED_ENABLED · NOT_RUN_EXTERNAL"
        }
        Some(_) => "CONFIGURED_DISABLED · NOT_RUN_EXTERNAL",
        None => "NOT_CONFIGURED · NOT_RUN_EXTERNAL",
    };
    let spine_status = spine_cli.status();
    let spine_state = spine_status
        .pointer("/assessment/state")
        .and_then(Value::as_str)
        .unwrap_or("NOT_RUN");
    let imported_source_observed = project.is_some_and(|value| !value.source_artifacts.is_empty());
    Ok(json!({
        "schemaVersion":"1.0.0",
        "ipc":"WIRED",
        "imageDecode": if imported_source_observed {
            "OBSERVED_CURRENT_PROJECT · ISOLATED_CHILD_BOUNDED"
        } else {
            "CAPABILITY_AVAILABLE · NOT_RUN_CURRENT_PROJECT"
        },
        "worker":"UNVERIFIED_EXCLUDED",
        "privateRemoteGpu":private_remote_gpu,
        "spineEditor":format!("EXTERNAL · {spine_state}"),
        "projectIntegrity":if project.is_some() {
            "OBSERVED_CURRENT_PROJECT · DPAPI_CURRENT_USER_HMAC_CHAIN"
        } else {
            "CAPABILITY_CONFIGURED · NOT_RUN_CURRENT_PROJECT"
        },
        "networkCallCount":0,
        "evidence":{
            "ipc":"OBSERVED_RUNTIME",
            "imageDecode":if imported_source_observed { "OBSERVED_CURRENT_PROJECT" } else { "CAPABILITY_ONLY" },
            "projectIntegrity":if project.is_some() { "OBSERVED_CURRENT_PROJECT" } else { "CAPABILITY_ONLY" },
            "networkCallCount":"STATIC_BOUNDARY · NO_REMOTE_TRANSPORT_BOUND",
            "spineEditor":"EXTERNAL_STATUS_PROJECTION",
            "worker":"PHYSICALLY_EXCLUDED_FROM_CORE_PACKAGE"
        }
    }))
}

fn current_gate(project: &ProjectManifest, gate_id: &str) -> bool {
    match gate_id {
        "master" => project.current_master_approval().is_some(),
        "layers" => project.current_layer_approval().is_some(),
        "rig" => project.current_rig_approval().is_some(),
        "poses" => f2s_domain::ACTION_KEYS
            .iter()
            .all(|action| project.current_pose_approval(action).is_some()),
        "hits" => ["attack_01", "attack_02", "attack_03"]
            .iter()
            .all(|action| project.current_hit_approval(action).is_some()),
        _ => project
            .approval_log
            .iter()
            .rev()
            .any(|approval| approval.gate_id == gate_id && !approval.invalidated),
    }
}

fn active_source(project: &ProjectManifest) -> Result<&SourceArtifact, String> {
    let master = project
        .active_master
        .as_ref()
        .ok_or("active master missing")?;
    project
        .source_artifacts
        .iter()
        .find(|source| source.artifact_id == master.source_artifact_id)
        .ok_or("active master source artifact missing".into())
}

fn cas_get(cas: &FsCas, sha256: &str, media_type: &str) -> Result<Vec<u8>, String> {
    cas.get(&CasRef {
        sha256: sha256.into(),
        byte_length: 0,
        media_type: media_type.into(),
    })
}

fn authoritative_layer_metrics(
    project: &ProjectManifest,
    cas: &FsCas,
) -> Result<RecompositionMetrics, String> {
    let source = active_source(project)?;
    let source_bytes = cas_get(cas, &source.sha256, &source.media_type)?;
    let layer_set = project
        .active_layer_set
        .as_ref()
        .ok_or("active layer set missing")?;
    let masks = layer_set
        .layers
        .iter()
        .map(|layer| cas_get(cas, &layer.mask_sha256, "application/vnd.f2s.alpha8"))
        .collect::<Result<Vec<_>, _>>()?;
    let attachments = layer_set
        .layers
        .iter()
        .map(|layer| cas_get(cas, &layer.attachment_sha256, "image/png"))
        .collect::<Result<Vec<_>, _>>()?;
    let mut metrics = recomposition_metrics(&source_bytes, &masks, source.width, source.height)?;
    metrics.changed_visible_pixels = changed_attachment_pixels(
        &source_bytes,
        &masks,
        &attachments,
        source.width,
        source.height,
    )?;
    Ok(metrics)
}

fn layer_projection(
    project: &ProjectManifest,
    metrics: RecompositionMetrics,
    cas: &FsCas,
    selected_layer_id: Option<&str>,
) -> Result<serde_json::Value, String> {
    let source = active_source(project)?;
    let source_bytes = cas_get(cas, &source.sha256, &source.media_type)?;
    let preview = render_safe_preview_png(&source_bytes, 256)?;
    let layer_set = project
        .active_layer_set
        .as_ref()
        .ok_or("active layer set missing")?;
    let selected = selected_layer_id
        .and_then(|id| layer_set.layers.iter().find(|layer| layer.layer_id == id))
        .or_else(|| {
            layer_set
                .layers
                .iter()
                .find(|layer| layer.role == LayerRole::Body)
        })
        .or_else(|| layer_set.layers.first())
        .ok_or("layer set contains no layers")?;
    let selected_bytes = cas_get(cas, &selected.attachment_sha256, "image/png")?;
    let selected_preview = render_safe_preview_png(&selected_bytes, 256)?;
    Ok(json!({
        "project": project_projection(project),
        "layerSet": project.active_layer_set.as_ref(),
        "metrics": metrics,
        "approvalQaPassed": recomposition_is_approvable(
            layer_set,
            metrics,
            &project.layer_provenance
        ),
        "requiredRoles": LayerRole::REQUIRED_V1,
        "canvas": {"width": source.width, "height": source.height},
        "safePreviewDataUrl": format!("data:image/png;base64,{}", BASE64_STANDARD.encode(preview)),
        "selectedLayerId": selected.layer_id,
        "selectedLayerPreviewDataUrl": format!("data:image/png;base64,{}", BASE64_STANDARD.encode(selected_preview)),
        "authority": "RUST_CAS_RECOMPUTED"
    }))
}

fn verified_constraint_capability() -> Result<ConstraintCapability, String> {
    ConstraintCapability::from_verified_manifest(
        include_bytes!("../../../../fixtures/m00/spine42-probe/capability-manifest.json"),
        &[
            (
                "rig-ir.json",
                include_bytes!("../../../../fixtures/m00/spine42-probe/rig-ir.json"),
            ),
            (
                "skeleton.json",
                include_bytes!("../../../../fixtures/m00/spine42-probe/skeleton.json"),
            ),
        ],
    )
}

fn safe_source_preview(project: &ProjectManifest, cas: &FsCas) -> Result<String, String> {
    let source = active_source(project)?;
    let bytes = cas_get(cas, &source.sha256, &source.media_type)?;
    let preview = render_safe_preview_png(&bytes, 256)?;
    Ok(format!(
        "data:image/png;base64,{}",
        BASE64_STANDARD.encode(preview)
    ))
}

fn rig_projection(project: &ProjectManifest, cas: &FsCas) -> Result<serde_json::Value, String> {
    let rig = project.active_rig.as_ref().ok_or("active Rig missing")?;
    let layer_set = project
        .active_layer_set
        .as_ref()
        .ok_or("active LayerSet missing")?;
    rig.validate(layer_set)?;
    let diagnostics = diagnose_rig_candidate(rig, layer_set);
    Ok(json!({
        "project": project_projection(project),
        "rig": rig,
        "diagnostics": diagnostics,
        "safePreviewDataUrl": safe_source_preview(project, cas)?,
        "authority": "RUST_PROJECT_HEAD_VALIDATED"
    }))
}

fn motion_projection(project: &ProjectManifest) -> Result<serde_json::Value, String> {
    let content = project
        .motion_content
        .as_ref()
        .ok_or("MotionContent missing")?;
    let master = project
        .active_master
        .as_ref()
        .ok_or("active master missing")?;
    content.validate(&master.style_spec)?;
    Ok(json!({
        "project": project_projection(project),
        "registry": f2s_domain::motion::registry::canonical_action_registry(),
        "content": content,
        "matrix": content_matrix(&content.specs, &content.assets, &content.prompt_pack),
        "authority": "RUST_CANONICAL_MOTION_CONTENT"
    }))
}

fn animation_projection(
    project: &ProjectManifest,
    cas: &FsCas,
) -> Result<serde_json::Value, String> {
    let animation = project
        .animation_set
        .as_ref()
        .ok_or("AnimationSet missing")?;
    let rig = project.active_rig.as_ref().ok_or("active Rig missing")?;
    let motion = project
        .motion_content
        .as_ref()
        .ok_or("MotionContent missing")?;
    let layer_set = project
        .active_layer_set
        .as_ref()
        .ok_or("active LayerSet missing")?;
    rig.validate(layer_set)?;
    animation.validate(
        motion,
        &rig.bone_tree,
        &rig.slot_set,
        &rig.sockets
            .iter()
            .map(|socket| socket.socket_id.clone())
            .collect::<Vec<_>>(),
    )?;
    let actions = f2s_domain::motion::registry::canonical_action_registry()
        .into_iter()
        .map(|definition| {
            let pose = project.current_pose_approval(&definition.key).is_some();
            let hit = project.current_hit_approval(&definition.key).is_some();
            json!({"definition":definition,"poseApproved":pose,"hitApproved":hit})
        })
        .collect::<Vec<_>>();
    let mut preview_attachments = Vec::with_capacity(layer_set.layers.len());
    let mut unsupported_layers = Vec::new();
    let mut preview_data_url_bytes = 0usize;
    for slot in rig.slot_set.stable_draw_order() {
        let layer = layer_set
            .layers
            .iter()
            .find(|layer| layer.layer_id == slot.layer_id)
            .ok_or("preview slot references a missing layer")?;
        let pivot = rig
            .pivots
            .iter()
            .find(|pivot| pivot.layer_id == layer.layer_id)
            .ok_or("preview layer pivot missing")?;
        let mesh = rig
            .meshes
            .iter()
            .find(|mesh| mesh.layer_id == layer.layer_id)
            .ok_or("preview layer mesh missing")?;
        let weights = rig
            .weights
            .iter()
            .find(|weights| weights.mesh_id == mesh.mesh_id)
            .ok_or("preview layer weights missing")?;
        let rigid_bone = rigid_preview_bone(weights);
        let supported = rigid_bone == Some(slot.bone_id.as_str());
        if !supported {
            unsupported_layers.push(layer.layer_id.clone());
        }
        let attachment_bytes = cas_get(cas, &layer.attachment_sha256, "image/png")?;
        let facts = decode_image_bounded(&attachment_bytes, &ImportLimits::default())?;
        if facts.width != rig.canvas.width_px || facts.height != rig.canvas.height_px {
            return Err("preview attachment does not match the Rig canvas".into());
        }
        let safe_png =
            render_safe_preview_png(&attachment_bytes, ANIMATION_PREVIEW_RASTER_MAX_SIDE_PX)?;
        let safe_png_data_url =
            bounded_animation_preview_data_url(&mut preview_data_url_bytes, &safe_png)?;
        preview_attachments.push(json!({
            "layerId": layer.layer_id,
            "layerName": layer.name,
            "attachmentSha256": layer.attachment_sha256,
            "slotId": slot.slot_id,
            "boneId": slot.bone_id,
            "drawKey": slot.draw_key,
            "pivot": pivot.point,
            "visible": layer.visible,
            "bindingMode": if supported { "RIGID_SINGLE_BONE" } else { "UNSUPPORTED_MULTI_BONE_OR_WEIGHT_MISMATCH" },
            "supported": supported,
            "meshId": mesh.mesh_id,
            "meshPreviewApplied": false,
            "multiBonePreviewApplied": false,
            "safePngDataUrl": safe_png_data_url
        }));
    }
    Ok(json!({
        "project": project_projection(project),
        "animation": animation,
        "rig": rig,
        "motion": motion,
        "actions": actions,
        "diagnostics": diagnose_animation_set(animation),
        "attachmentPreview": {
            "schemaVersion": "1.0.0",
            "layerSetRevision": layer_set.revision,
            "rigRevision": rig.revision,
            "canvas": rig.canvas,
            "attachments": preview_attachments,
            "diagnostics": {
                "mode": "RIGID_SINGLE_BONE_FULL_CANVAS_SPRITES",
                "supportedAttachmentCount": layer_set.layers.len() - unsupported_layers.len(),
                "unsupportedLayerIds": unsupported_layers,
                "rasterMaxSidePx": ANIMATION_PREVIEW_RASTER_MAX_SIDE_PX,
                "totalDataUrlBytes": preview_data_url_bytes,
                "maxDataUrlBytes": ANIMATION_PREVIEW_DATA_URL_BUDGET_BYTES,
                "fullResolutionRasterApplied": false,
                "slotColorApplied": true,
                "drawOrderApplied": true,
                "meshDeformationApplied": false,
                "multiBoneSkinningApplied": false,
                "note": "Diagnostic preview only. Full-canvas attachment sprites follow one rigid slot bone; mesh/deform and multi-bone skinning are intentionally not simulated."
            }
        },
        "authority": "RUST_ANIMATION_SET_VALIDATED"
    }))
}

fn bounded_animation_preview_data_url(
    running_total: &mut usize,
    safe_png: &[u8],
) -> Result<String, String> {
    let data_url = format!("data:image/png;base64,{}", BASE64_STANDARD.encode(safe_png));
    let next_total = running_total
        .checked_add(data_url.len())
        .ok_or("animation preview data URL budget overflow")?;
    if next_total > ANIMATION_PREVIEW_DATA_URL_BUDGET_BYTES {
        return Err("animation preview data URL budget exceeded; reduce the LayerSet size".into());
    }
    *running_total = next_total;
    Ok(data_url)
}

fn rigid_preview_bone(weights: &f2s_domain::rig::weights::WeightSet) -> Option<&str> {
    let mut expected = None;
    if weights.by_vertex.is_empty() {
        return None;
    }
    for influences in weights.by_vertex.values() {
        let [influence] = influences.as_slice() else {
            return None;
        };
        if influence.weight_ppm != 1_000_000 {
            return None;
        }
        match expected {
            Some(bone) if bone != influence.bone_id => return None,
            None => expected = Some(influence.bone_id.as_str()),
            _ => {}
        }
    }
    expected
}

fn utc_now() -> String {
    let value = unsafe { GetSystemTime() };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        value.wYear,
        value.wMonth,
        value.wDay,
        value.wHour,
        value.wMinute,
        value.wSecond,
        value.wMilliseconds
    )
}
fn project_projection(project: &ProjectManifest) -> serde_json::Value {
    let master_approved = current_gate(project, "master")
        && project
            .active_master
            .as_ref()
            .is_some_and(|master| master.approval_state == "APPROVED");
    let layers_approved = current_gate(project, "layers")
        && project
            .active_layer_set
            .as_ref()
            .is_some_and(|layer_set| layer_set.approval_state == "APPROVED");
    let rig_approved = current_gate(project, "rig");
    let poses_approved = current_gate(project, "poses");
    let hits_approved = current_gate(project, "hits");
    let pose_approval_count = f2s_domain::ACTION_KEYS
        .iter()
        .filter(|action| project.current_pose_approval(action).is_some())
        .count();
    let hit_approval_count = ["attack_01", "attack_02", "attack_03"]
        .iter()
        .filter(|action| project.current_hit_approval(action).is_some())
        .count();
    json!({
        "availability": "AVAILABLE",
        "diagnosticCode": Value::Null,
        "projectId": project.identity.project_id,
        "displayName": project.identity.display_name,
        "revision": project.revision,
        "workflowStage": project.workflow_stage,
        "sourceCount": project.source_artifacts.len(),
        "masterState": project.active_master.as_ref().map(|value| value.approval_state.as_str()).unwrap_or("MISSING"),
        "layerState": project.active_layer_set.as_ref().map(|value| value.approval_state.as_str()).unwrap_or("MISSING"),
        "rigState": project.active_rig.as_ref().map(|value| value.approval_state),
        "motionState": if project.motion_content.is_some() { "PRESENT" } else { "MISSING" },
        "animationState": if project.animation_set.is_some() { "PRESENT" } else { "MISSING" },
        "poseApprovalCount": pose_approval_count,
        "hitApprovalCount": hit_approval_count,
        "activeMaster": project.active_master.as_ref(),
        "activeLayerSet": project.active_layer_set.as_ref(),
        "gates": {
            "master": if master_approved { "APPROVED" } else { "PENDING" },
            "layers": if !master_approved { "LOCKED" } else if layers_approved { "APPROVED" } else { "PENDING" },
            "rig": if !layers_approved { "LOCKED" } else if rig_approved { "APPROVED" } else { "PENDING" },
            "poses": if !rig_approved || project.animation_set.is_none() { "LOCKED" } else if poses_approved { "APPROVED" } else { "PENDING" },
            "hits": if !poses_approved { "LOCKED" } else if hits_approved { "APPROVED" } else { "PENDING" }
        }
    })
}

fn unavailable_project_projection(project_id: &str, diagnostic_code: &str) -> Value {
    json!({
        "availability":"INTEGRITY_CHECK_FAILED",
        "diagnosticCode":diagnostic_code,
        "projectId":project_id,
        "displayName":"无法读取的本地项目",
        "revision":0,
        "workflowStage":"UNAVAILABLE",
        "sourceCount":0,
        "masterState":"UNAVAILABLE",
        "layerState":"UNAVAILABLE",
        "rigState":Value::Null,
        "motionState":"MISSING",
        "animationState":"MISSING",
        "poseApprovalCount":0,
        "hitApprovalCount":0,
        "activeMaster":Value::Null,
        "activeLayerSet":Value::Null,
        "gates":{
            "master":"UNAVAILABLE",
            "layers":"UNAVAILABLE",
            "rig":"UNAVAILABLE",
            "poses":"UNAVAILABLE",
            "hits":"UNAVAILABLE"
        }
    })
}

fn classify_ipc_command_error(message: &str) -> (&'static str, bool) {
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("stale") || normalized.contains("revision conflict") {
        ("F2S-REVISION-STALE", true)
    } else if normalized.contains("cancelled") || normalized.contains("canceled") {
        ("F2S-USER-CANCELLED", true)
    } else if ["integrity", "hmac", "tamper", "rollback", "fork"]
        .iter()
        .any(|needle| normalized.contains(needle))
    {
        ("F2S-INTEGRITY", false)
    } else if ["not open", "not found", "missing"]
        .iter()
        .any(|needle| normalized.contains(needle))
    {
        ("F2S-STATE-MISSING", true)
    } else if ["external", "not run"]
        .iter()
        .any(|needle| normalized.contains(needle))
    {
        ("F2S-EXTERNAL-NOT-RUN", false)
    } else if [
        "invalid",
        "required",
        "outside",
        "unsupported",
        "unchanged",
        "must ",
        "cannot ",
        "reject",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
    {
        ("F2S-VALIDATION", false)
    } else {
        ("F2S-IPC-COMMAND", false)
    }
}

fn recent_project_ids(root: &Path, limit: usize) -> Result<Vec<String>, String> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut projects = Vec::new();
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_ok_and(|value| value.is_dir()) {
            continue;
        }
        let Some(id) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if Uuid::parse_str(&id).is_err() {
            continue;
        }
        let modified = entry
            .path()
            .join("head.json")
            .metadata()
            .and_then(|metadata| metadata.modified())
            .or_else(|_| entry.metadata().and_then(|metadata| metadata.modified()))
            .unwrap_or(SystemTime::UNIX_EPOCH);
        projects.push((id, modified));
    }
    projects.sort_by(|(left_id, left_modified), (right_id, right_modified)| {
        right_modified
            .cmp(left_modified)
            .then_with(|| left_id.cmp(right_id))
    });
    Ok(projects.into_iter().take(limit).map(|(id, _)| id).collect())
}

fn export_projection(project: &ProjectManifest) -> Value {
    let export_id = format!("preflight-r{}", project.revision);
    let (report, snapshot_sha256) = match assemble_publish_snapshot(project, export_id) {
        Ok(snapshot) => {
            let report = preflight(&snapshot);
            let hash = canonical_sha256(&snapshot).ok();
            (serde_json::to_value(report).unwrap_or(Value::Null), hash)
        }
        Err(error) => (
            json!({
                "passed": false,
                "checks": [],
                "errors": [error],
                "externalEditorStatus": "EXTERNAL",
                "publishStatus": "BLOCKED"
            }),
            None,
        ),
    };
    let can_commit = report
        .get("passed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let candidate_state = if can_commit {
        "READY_FOR_COMMIT"
    } else {
        "BLOCKED"
    };
    json!({
        "project": project_projection(project),
        "preflight": report,
        "snapshotSha256": snapshot_sha256,
        "outputs": [
            {"path":"rig-ir.json","owner":"built-in open writer","state":candidate_state},
            {"path":"images/**/*.png","owner":"built-in straight-alpha sRGB","state":candidate_state},
            {"path":"character.psd","owner":"built-in minimal repairable subset","state":candidate_state},
            {"path":"character.spine.json","owner":"built-in Spine 4.2.43 contract","state":candidate_state},
            {"path":"atlas-input-manifest.json","owner":"built-in packing input only","state":candidate_state},
            {"path":"prompt-pack.json / prompt-pack.md","owner":"built-in offline text","state":candidate_state},
            {"path":"compatibility-manifest.json / checksums.sha256","owner":"built-in evidence","state":candidate_state},
            {"path":".atlas / .spine / .skel","owner":"user local Professional CLI only","state":"EXTERNAL"}
        ],
        "history": project.export_records,
        "authority": "RUST_PUBLISH_SNAPSHOT_ASSEMBLER"
    })
}

fn commit_manifest(project: &ProjectManifest) -> Result<(), String> {
    project.validate_cross_aggregate()?;
    let (projects, cas) = stores()?;
    let id = project.identity.project_id.to_string();
    let previous = projects.load_head(&id)?.map(|v| v.manifest_sha256);
    commit_project(&projects, &cas, &id, project.revision, previous, project)?;
    Ok(())
}
fn read_stable(path: &Path) -> Result<Vec<u8>, String> {
    let before = fs::metadata(path).map_err(|e| e.to_string())?;
    let limits = ImportLimits::default();
    if before.len() == 0 || before.len() > limits.max_file_bytes {
        return Err("selected image exceeds byte policy".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let after = fs::metadata(path).map_err(|e| e.to_string())?;
    if before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
        || bytes.len() as u64 != after.len()
    {
        return Err("selected image changed during stable read".into());
    }
    Ok(bytes)
}

fn read_small_stable(path: &Path, max_bytes: u64, label: &str) -> Result<Vec<u8>, String> {
    let link_metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !link_metadata.file_type().is_file() || link_metadata.file_type().is_symlink() {
        return Err(format!("{label} must be a regular local file"));
    }
    let before = fs::metadata(path).map_err(|error| error.to_string())?;
    if before.len() == 0 || before.len() > max_bytes {
        return Err(format!("{label} exceeds the {max_bytes}-byte policy"));
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let after = fs::metadata(path).map_err(|error| error.to_string())?;
    if before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
        || bytes.len() as u64 != after.len()
    {
        return Err(format!("{label} changed during stable read"));
    }
    Ok(bytes)
}

pub fn run_image_probe(path: &Path) -> i32 {
    let result = (|| {
        let bytes = read_stable(path)?;
        decode_image_bounded(&bytes, &ImportLimits::default())
    })();
    match result {
        Ok(report) => match serde_json::to_string(&report) {
            Ok(json) => {
                println!("{json}");
                0
            }
            Err(error) => {
                eprintln!("probe serialization failed: {error}");
                2
            }
        },
        Err(error) => {
            eprintln!("{error}");
            2
        }
    }
}

fn probe_in_child(path: &Path) -> Result<DecodedImageReport, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut command = Command::new(exe);
    command
        .arg("--image-probe")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("isolated image decode timed out".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if output.stdout.len() > 64 * 1024 || output.stderr.len() > 64 * 1024 {
        return Err("image probe output exceeded policy".into());
    }
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    serde_json::from_slice(&output.stdout).map_err(|e| format!("invalid image probe response: {e}"))
}

fn choose_image(hwnd: HWND) -> Result<Option<PathBuf>, String> {
    let mut file_buffer = vec![0u16; 32768];
    let filter = "Images (*.png;*.jpg;*.jpeg;*.webp)\0*.png;*.jpg;*.jpeg;*.webp\0\0"
        .encode_utf16()
        .collect::<Vec<_>>();
    let title = "选择要导入的角色图片\0".encode_utf16().collect::<Vec<_>>();
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(file_buffer.as_mut_ptr()),
        nMaxFile: file_buffer.len() as u32,
        lpstrTitle: PCWSTR(title.as_ptr()),
        Flags: OFN_EXPLORER
            | OFN_FILEMUSTEXIST
            | OFN_PATHMUSTEXIST
            | OFN_NOCHANGEDIR
            | OFN_DONTADDTORECENT,
        ..Default::default()
    };
    let selected = unsafe { GetOpenFileNameW(&mut dialog).as_bool() };
    if !selected {
        let error = unsafe { CommDlgExtendedError() };
        if error.0 == 0 {
            return Ok(None);
        }
        return Err(format!("native file dialog failed: {}", error.0));
    }
    let length = file_buffer
        .iter()
        .position(|v| *v == 0)
        .unwrap_or(file_buffer.len());
    Ok(Some(PathBuf::from(
        String::from_utf16(&file_buffer[..length]).map_err(|_| "file path is not valid UTF-16")?,
    )))
}

fn choose_remote_gpu_profile(hwnd: HWND) -> Result<Option<PathBuf>, String> {
    let mut file_buffer = vec![0u16; 32768];
    let filter = "Private Remote GPU Profile (*.json)\0*.json\0\0"
        .encode_utf16()
        .collect::<Vec<_>>();
    let title = "导入私有远程 GPU 本地配置\0"
        .encode_utf16()
        .collect::<Vec<_>>();
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(file_buffer.as_mut_ptr()),
        nMaxFile: file_buffer.len() as u32,
        lpstrTitle: PCWSTR(title.as_ptr()),
        Flags: OFN_EXPLORER
            | OFN_FILEMUSTEXIST
            | OFN_PATHMUSTEXIST
            | OFN_NOCHANGEDIR
            | OFN_DONTADDTORECENT,
        ..Default::default()
    };
    let selected = unsafe { GetOpenFileNameW(&mut dialog).as_bool() };
    if !selected {
        let error = unsafe { CommDlgExtendedError() };
        if error.0 == 0 {
            return Ok(None);
        }
        return Err(format!("native profile file dialog failed: {}", error.0));
    }
    let length = file_buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(file_buffer.len());
    let path = PathBuf::from(
        String::from_utf16(&file_buffer[..length])
            .map_err(|_| "profile file path is not valid UTF-16")?,
    );
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        return Err("private remote GPU profile must be a .json file".into());
    }
    Ok(Some(path))
}

fn choose_export_root(hwnd: HWND) -> Result<Option<PathBuf>, String> {
    let dialog: IFileOpenDialog = unsafe {
        CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("cannot create native export folder dialog: {error}"))?
    };
    unsafe {
        let options = dialog
            .GetOptions()
            .map_err(|error| format!("cannot read native folder dialog options: {error}"))?;
        dialog
            .SetOptions(options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST)
            .map_err(|error| format!("cannot configure native folder dialog: {error}"))?;
        dialog
            .SetTitle(windows::core::w!("选择不可覆盖导出的根目录"))
            .map_err(|error| format!("cannot set native folder dialog title: {error}"))?;
        if let Err(error) = dialog.Show(Some(hwnd)) {
            if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
                return Ok(None);
            }
            return Err(format!("native export folder dialog failed: {error}"));
        }
        let item = dialog
            .GetResult()
            .map_err(|error| format!("native folder dialog returned no result: {error}"))?;
        let raw = item
            .GetDisplayName(SIGDN_FILESYSPATH)
            .map_err(|error| format!("cannot resolve selected export folder: {error}"))?;
        let path = raw
            .to_string()
            .map(PathBuf::from)
            .map_err(|_| "selected export folder is not valid UTF-16".to_string());
        CoTaskMemFree(Some(raw.0.cast()));
        path.map(Some)
    }
}

fn validate_export_root(root: &Path) -> Result<(), String> {
    let selected = root.canonicalize().map_err(|error| error.to_string())?;
    let private_root = local_data_root()?
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if selected.starts_with(&private_root) {
        return Err("export root cannot be inside FlashToSpine private storage".into());
    }
    Ok(())
}

impl HostState {
    fn issue_master_review(
        &self,
        project_id: &str,
        project_revision: u64,
        master_id: &str,
        approval_payload_sha256: &str,
    ) -> String {
        let mut reviews = self.master_reviews.borrow_mut();
        // A master has a single current review grant. Project/revision changes
        // deliberately force the operator to render and inspect it again.
        reviews.retain(|_, review| {
            review.project_id == project_id
                && review.project_revision == project_revision
                && review.master_id != master_id
        });
        if reviews.len() >= 16 {
            reviews.clear();
        }
        let token = Uuid::new_v4().simple().to_string();
        reviews.insert(
            token.clone(),
            MasterReviewGrant {
                project_id: project_id.into(),
                project_revision,
                master_id: master_id.into(),
                approval_payload_sha256: approval_payload_sha256.into(),
            },
        );
        token
    }

    fn consume_master_review(
        &self,
        token: &str,
        project_id: &str,
        project_revision: u64,
        master_id: &str,
        approval_payload_sha256: &str,
    ) -> Result<(), String> {
        let review = self
            .master_reviews
            .borrow_mut()
            .remove(token)
            .ok_or("master reviewToken missing, expired, or already consumed")?;
        if review.project_id != project_id
            || review.project_revision != project_revision
            || review.master_id != master_id
            || review.approval_payload_sha256 != approval_payload_sha256
        {
            return Err(
                "master reviewToken is stale or bound to another complete candidate".into(),
            );
        }
        Ok(())
    }

    fn issue_key_pose_review(
        &self,
        project_id: &str,
        project_revision: u64,
        binding_id: &str,
        source_sha256: &str,
    ) -> String {
        let mut reviews = self.key_pose_reviews.borrow_mut();
        reviews.retain(|_, review| {
            review.project_id == project_id
                && review.project_revision == project_revision
                && review.binding_id != binding_id
        });
        if reviews.len() >= 64 {
            // Review grants are deliberately bounded. Clearing only makes an operator preview again.
            reviews.clear();
        }
        let token = Uuid::new_v4().simple().to_string();
        reviews.insert(
            token.clone(),
            KeyPoseReview {
                project_id: project_id.into(),
                project_revision,
                binding_id: binding_id.into(),
                source_sha256: source_sha256.into(),
            },
        );
        token
    }

    fn consume_key_pose_review(
        &self,
        token: &str,
        project_id: &str,
        project_revision: u64,
        binding_id: &str,
        current_source_sha256: Option<&str>,
    ) -> Result<(), String> {
        // Removal deliberately happens before any validation or native confirmation. A failed or
        // cancelled approval therefore cannot replay the review grant.
        let review = self
            .key_pose_reviews
            .borrow_mut()
            .remove(token)
            .ok_or("reviewToken missing, expired, or already consumed")?;
        if review.project_id != project_id
            || review.project_revision != project_revision
            || review.binding_id != binding_id
            || current_source_sha256 != Some(review.source_sha256.as_str())
        {
            return Err(
                "reviewToken is stale or bound to another project revision, binding, or source"
                    .into(),
            );
        }
        Ok(())
    }

    fn verified_human(
        &self,
        hwnd: HWND,
        purpose: &str,
        payload_sha256: &str,
        summary: &str,
    ) -> Result<VerifiedHumanActor, String> {
        let text = format!(
            "请确认人工审批\n\n{summary}\n\n目标摘要：{}…\n\n此操作会写入不可变审批记录。",
            &payload_sha256[..12]
        );
        let mut wide = text.encode_utf16().collect::<Vec<_>>();
        wide.push(0);
        let accepted = unsafe {
            MessageBoxW(
                Some(hwnd),
                PCWSTR(wide.as_ptr()),
                windows::core::w!("FlashToSpine 人工审批"),
                MB_YESNO | MB_ICONQUESTION,
            )
        };
        if accepted != IDYES {
            return Err("human approval was cancelled".into());
        }
        let attestation_id = Uuid::new_v4().simple().to_string();
        let proof = format!(
            "{:x}",
            Sha256::digest(
                format!(
                    "{}:{attestation_id}:{purpose}:{payload_sha256}",
                    self.session_nonce
                )
                .as_bytes()
            )
        );
        let attestation = CredentialAttestation {
            attestation_id,
            actor_id: "local-interactive-user".into(),
            actor_kind: "HUMAN".into(),
            credential_ref: "windows-session/native-confirmation".into(),
            purpose: purpose.into(),
            issued_at_utc: utc_now(),
            expires_at_utc: "9999-12-31T23:59:59.999Z".into(),
            payload_sha256: payload_sha256.into(),
            verification_proof_sha256: proof,
        };
        VerifiedHumanActor::verify(
            attestation,
            purpose,
            payload_sha256,
            &NativeHumanVerifier {
                consumed: &self.consumed_attestations,
                session_nonce: &self.session_nonce,
            },
        )
    }

    fn choose_and_stage_image(
        &self,
        hwnd: HWND,
        purpose: PendingImportPurpose,
        project_id: &str,
        project_revision: u64,
    ) -> Result<Value, String> {
        let Some(path) = choose_image(hwnd)? else {
            return Ok(json!({"cancelled":true}));
        };
        let report = probe_in_child(&path)?;
        let bytes = read_stable(&path)?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        if hash != report.source_sha256 {
            return Err("image changed between isolated decode and staging".into());
        }
        let token = Uuid::new_v4().simple().to_string();
        let staging = local_data_root()?.join("staging");
        fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
        let staged_path = staging.join(format!("{token}.image"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged_path)
            .map_err(|error| error.to_string())?;
        file.write_all(&bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        self.pending.borrow_mut().insert(
            token.clone(),
            PendingImport {
                path: staged_path,
                report: report.clone(),
                purpose,
                project_id: project_id.into(),
                project_revision,
            },
        );
        Ok(json!({
            "cancelled": false,
            "stagingToken": token,
            "fileName": path.file_name().and_then(|value| value.to_str()).unwrap_or("selected-image"),
            "report": report,
            "previewState": "WITHHELD_UNTIL_PURPOSE_BOUND_PROMOTION"
        }))
    }

    pub fn handle(&self, request: IpcRequest, hwnd: HWND) -> IpcResponse {
        let id = request.request_id.clone();
        let result: Result<serde_json::Value, String> = match request.method {
            IpcMethod::BootstrapStatus => Ok(
                json!({"productMode":"CORE_IMPLEMENTED_EXTERNALS_PENDING","ipc":"WIRED","spineTarget":"4.2.43","capabilityId":"F2S-SPINE-CAP-4.2.43-001","workerPack":"UNVERIFIED_EXCLUDED","networkAllowed":false,"releaseReady":false,"projectIntegrity":"DPAPI_CURRENT_USER_HMAC_CHAIN","currentProject":self.current_project.borrow().as_ref().map(project_projection)}),
            ),
            IpcMethod::RemoteGpuStatus => remote_gpu_status_projection(),
            IpcMethod::RemoteGpuImportProfile => (|| {
                let Some(path) = choose_remote_gpu_profile(hwnd)? else {
                    return Ok(json!({
                        "cancelled": true,
                        "status": remote_gpu_status_projection()?
                    }));
                };
                let bytes = read_small_stable(
                    &path,
                    REMOTE_GPU_PROFILE_MAX_BYTES,
                    "private remote GPU profile",
                )?;
                let profile = parse_remote_gpu_profile(&bytes)?;
                save_and_activate_remote_gpu_profile(&profile)?;
                Ok(json!({
                    "cancelled": false,
                    "status": remote_gpu_status_projection()?
                }))
            })(),
            IpcMethod::RemoteGpuDisable => (|| {
                let mut profile = load_active_remote_gpu_profile()?
                    .ok_or("no active private remote GPU profile is configured")?;
                profile.enabled = false;
                save_and_activate_remote_gpu_profile(&profile)?;
                Ok(remote_gpu_status_projection()?)
            })(),
            IpcMethod::SpineCliStatus => Ok(self.spine_cli.status()),
            IpcMethod::SpineCliSelectAndAssess => self.spine_cli.select_and_assess(hwnd),
            IpcMethod::SpineCliClear => self.spine_cli.clear_config(),
            IpcMethod::SpineCliJobStart => (|| {
                let current = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(current.revision) {
                    return Err("stale project revision".into());
                }
                let export_id = request
                    .payload
                    .get("exportId")
                    .and_then(Value::as_str)
                    .ok_or("exportId required")?;
                let operation_kind = request
                    .payload
                    .get("operationKind")
                    .and_then(Value::as_str)
                    .ok_or("operationKind required")?;
                self.spine_cli.start_job(
                    export_id,
                    operation_kind,
                    &current.identity.project_id.to_string(),
                    current.revision,
                    hwnd,
                )
            })(),
            IpcMethod::SpineCliJobStatus => (|| {
                let job_id = request
                    .payload
                    .get("jobId")
                    .and_then(Value::as_str)
                    .ok_or("jobId required")?;
                self.spine_cli.job_status(job_id)
            })(),
            IpcMethod::DiagnosticsStatus => diagnostics_status_projection(
                self.current_project.borrow().as_ref(),
                &self.spine_cli,
            ),
            IpcMethod::DiagnosticsExport => (|| {
                let status = diagnostics_status_projection(
                    self.current_project.borrow().as_ref(),
                    &self.spine_cli,
                )?;
                let generated_at_utc = utc_now();
                let bytes = redacted_report_bytes(
                    &status,
                    self.current_project.borrow().as_ref(),
                    &generated_at_utc,
                )?;
                let sha256 = format!("{:x}", Sha256::digest(&bytes));
                let root = local_data_root()?;
                let Some(path) = choose_and_write_report(hwnd, &bytes, &root)? else {
                    return Ok(json!({"cancelled":true}));
                };
                let file_name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or("diagnostics file name is not valid UTF-8")?;
                Ok(json!({
                    "cancelled":false,
                    "fileName":file_name,
                    "sha256":sha256,
                    "bytes":bytes.len(),
                    "status":"REDACTED_LOCAL_REPORT_WRITTEN",
                    "authority":"RUST_FIXED_WHITELIST_DIAGNOSTICS"
                }))
            })(),
            IpcMethod::ImageChooseAndPreflight => (|| {
                let project = self.current_project.borrow();
                let project = project.as_ref().ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                self.choose_and_stage_image(
                    hwnd,
                    PendingImportPurpose::MasterSource,
                    &project.identity.project_id.to_string(),
                    project.revision,
                )
            })(),
            IpcMethod::ImagePromote => (|| {
                let token = request
                    .payload
                    .get("stagingToken")
                    .and_then(|v| v.as_str())
                    .ok_or("stagingToken required")?;
                let pending = self
                    .pending
                    .borrow_mut()
                    .remove(token)
                    .ok_or("staging token missing or already consumed")?;
                if !matches!(pending.purpose, PendingImportPurpose::MasterSource) {
                    return Err("staging token is bound to a different image purpose".into());
                }
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("create or open a project before promoting an image")?;
                if request.expected_revision != Some(project.revision)
                    || pending.project_id != project.identity.project_id.to_string()
                    || pending.project_revision != project.revision
                {
                    return Err("staging token is stale or bound to another project".into());
                }
                let bytes = read_stable(&pending.path)?;
                let hash = format!("{:x}", Sha256::digest(&bytes));
                if hash != pending.report.source_sha256 {
                    return Err("staged image hash drift".into());
                }
                let facts = ImageFacts {
                    media_type: pending.report.media_type.clone(),
                    width: pending.report.width,
                    height: pending.report.height,
                    bit_depth: pending.report.bit_depth,
                    has_alpha: pending.report.has_alpha,
                };
                let artifact = promote_image(
                    &FsCas::new(local_data_root()?.join("cas")),
                    &bytes,
                    &facts,
                    "user-local-native-dialog",
                )?;
                project.add_source(artifact.clone());
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                fs::remove_file(&pending.path).map_err(|e| e.to_string())?;
                Ok(json!({"artifact":artifact,"project":project_projection(&project)}))
            })(),
            IpcMethod::ProjectCreate => (|| {
                let name = request
                    .payload
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or("project name required")?;
                let (projects, cas) = stores()?;
                let project = create_project(&projects, &cas, name)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                Ok(project_projection(&project))
            })(),
            IpcMethod::ProjectOpen => (|| {
                let id = request
                    .payload
                    .get("projectId")
                    .and_then(|v| v.as_str())
                    .ok_or("projectId required")?;
                let (projects, cas) = stores()?;
                let project = open_project(&projects, &cas, id)?.ok_or("project not found")?;
                *self.current_project.borrow_mut() = Some(project.clone());
                Ok(project_projection(&project))
            })(),
            IpcMethod::ProjectRecent => (|| {
                let (projects, cas) = stores()?;
                let root = local_data_root()?.join("projects");
                let mut values = Vec::new();
                if root.exists() {
                    for id in recent_project_ids(&root, 20)? {
                        match open_project(&projects, &cas, &id) {
                            Ok(Some(project)) => values.push(project_projection(&project)),
                            Ok(None) => values.push(unavailable_project_projection(
                                &id,
                                "F2S-PROJECT-RECENT-MISSING-HEAD",
                            )),
                            Err(_) => values.push(unavailable_project_projection(
                                &id,
                                "F2S-PROJECT-RECENT-INTEGRITY",
                            )),
                        }
                    }
                }
                Ok(Value::Array(values))
            })(),
            IpcMethod::MasterCreate => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let style: StyleSpec = serde_json::from_value(
                    request
                        .payload
                        .get("style")
                        .cloned()
                        .ok_or("style required")?,
                )
                .map_err(|e| format!("invalid StyleSpec: {e}"))?;
                let source = project
                    .source_artifacts
                    .last()
                    .ok_or("source artifact required")?;
                let master = create_master(source, style)?;
                project.set_master(master.clone());
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                Ok(json!({"master":master,"project":project_projection(&project)}))
            })(),
            IpcMethod::MasterPreview => (|| {
                let current = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(current.revision) {
                    return Err("stale project revision".into());
                }
                let master = current
                    .active_master
                    .as_ref()
                    .ok_or("active master missing")?;
                if master.approval_state != "PENDING" {
                    return Err(
                        "only the current pending master can be previewed for approval".into(),
                    );
                }
                let source = current
                    .source_artifacts
                    .iter()
                    .find(|source| source.artifact_id == master.source_artifact_id)
                    .ok_or("master source artifact missing")?;
                if source.sha256 != master.source_sha256 {
                    return Err("master source binding mismatch".into());
                }
                let (_, cas) = stores()?;
                let bytes = cas_get(&cas, &source.sha256, &source.media_type)?;
                if format!("{:x}", Sha256::digest(&bytes)) != source.sha256 {
                    return Err("master source CAS content hash mismatch".into());
                }
                let preview = render_safe_preview_png(&bytes, 320)?;
                let payload = master.approval_payload_sha256()?;
                let project_id = current.identity.project_id.to_string();
                let review_token = self.issue_master_review(
                    &project_id,
                    current.revision,
                    &master.master_id,
                    &payload,
                );
                Ok(json!({
                    "project": project_projection(&current),
                    "master": master,
                    "approvalPayloadSha256": payload,
                    "safePreviewDataUrl": format!(
                        "data:image/png;base64,{}",
                        BASE64_STANDARD.encode(preview)
                    ),
                    "reviewToken": review_token,
                    "authority": "RUST_CAS_BOUNDED_COMPLETE_MASTER_REVIEW"
                }))
            })(),
            IpcMethod::MasterApprove => (|| {
                let current = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(current.revision) {
                    return Err("stale project revision".into());
                }
                let mut master = current
                    .active_master
                    .clone()
                    .ok_or("active master missing")?;
                if master.approval_state != "PENDING" {
                    return Err("only the current pending master can be approved".into());
                }
                let review_token = request
                    .payload
                    .get("reviewToken")
                    .and_then(Value::as_str)
                    .ok_or("reviewToken required; preview the complete master before approval")?;
                let payload = master.approval_payload_sha256()?;
                self.consume_master_review(
                    review_token,
                    &current.identity.project_id.to_string(),
                    current.revision,
                    &master.master_id,
                    &payload,
                )?;
                let actor = self.verified_human(
                    hwnd,
                    "approve-master",
                    &payload,
                    &format!(
                        "母版 {} / revision {}",
                        master.master_id, master.candidate_revision
                    ),
                )?;
                let approval = approve(&mut master, actor, &utc_now())?;
                let mut project = current;
                project.active_master = Some(master);
                project.record_master_approval(approval.clone())?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                Ok(json!({"approval":approval,"project":project_projection(&project)}))
            })(),
            IpcMethod::MasterReject => (|| {
                let current = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(current.revision) {
                    return Err("stale project revision".into());
                }
                let reason = request
                    .payload
                    .get("reason")
                    .and_then(Value::as_str)
                    .ok_or("rejection reason required")?;
                let mut master = current
                    .active_master
                    .clone()
                    .ok_or("active master missing")?;
                let payload = master_rejection_payload(&master, reason)?;
                let actor = self.verified_human(
                    hwnd,
                    "reject-master",
                    &payload,
                    &format!(
                        "退回母版 {} / revision {}\n原因：{}",
                        master.master_id,
                        master.candidate_revision,
                        reason.trim()
                    ),
                )?;
                let review = reject_master(&mut master, reason, actor, &utc_now())?;
                let mut project = current;
                project.active_master = Some(master);
                project.record_master_rejection(review.clone())?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                Ok(json!({"review":review,"project":project_projection(&project)}))
            })(),
            IpcMethod::LayersInitialize => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                if project.active_layer_set.is_some() {
                    let (_, cas) = stores()?;
                    let metrics = authoritative_layer_metrics(&project, &cas)?;
                    return layer_projection(&project, metrics, &cas, None);
                }
                if !current_gate(&project, "master") {
                    return Err("approved master required before layering".into());
                }
                let master = project
                    .active_master
                    .as_ref()
                    .filter(|value| value.approval_state == "APPROVED")
                    .cloned()
                    .ok_or("approved active master missing")?;
                let source = active_source(&project)?.clone();
                let (_, cas) = stores()?;
                let source_bytes = cas_get(&cas, &source.sha256, &source.media_type)?;
                let full_mask = initial_mask(source.width, source.height, true)?;
                let empty_mask = initial_mask(source.width, source.height, false)?;
                let body_png =
                    render_masked_png(&source_bytes, &full_mask, source.width, source.height)?;
                let empty_png =
                    render_masked_png(&source_bytes, &empty_mask, source.width, source.height)?;
                let full_mask_ref = cas.put("application/vnd.f2s.alpha8", &full_mask)?;
                let empty_mask_ref = cas.put("application/vnd.f2s.alpha8", &empty_mask)?;
                let body_ref = cas.put("image/png", &body_png)?;
                let empty_ref = cas.put("image/png", &empty_png)?;
                let mut provenance = Vec::with_capacity(LayerRole::REQUIRED_V1.len());
                let layers = LayerRole::REQUIRED_V1
                    .iter()
                    .map(|role| {
                        let (attachment, mask) = if *role == LayerRole::Body {
                            (&body_ref, &full_mask_ref)
                        } else {
                            (&empty_ref, &empty_mask_ref)
                        };
                        provenance.push(PixelProvenance {
                            artifact_sha256: attachment.sha256.clone(),
                            origin: PixelOrigin::Source,
                            source_sha256: source.sha256.clone(),
                            prompt_pack_id: None,
                            receipt_ref: None,
                            accepted_by: None,
                            acceptance_attestation_sha256: None,
                        });
                        let name = serde_json::to_value(role)
                            .ok()
                            .and_then(|value| value.as_str().map(str::to_owned))
                            .unwrap_or_else(|| format!("{role:?}"));
                        Layer {
                            layer_id: Uuid::new_v4().to_string(),
                            name,
                            role: *role,
                            attachment_sha256: attachment.sha256.clone(),
                            mask_sha256: mask.sha256.clone(),
                            visible: true,
                            approved: false,
                        }
                    })
                    .collect();
                let layer_set = LayerSet {
                    layer_set_id: Uuid::new_v4().to_string(),
                    master_id: master.master_id,
                    revision: 0,
                    layers,
                    approval_state: "PENDING".into(),
                };
                project.set_layer_set(layer_set, provenance)?;
                let metrics = authoritative_layer_metrics(&project, &cas)?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                layer_projection(&project, metrics, &cas, None)
            })(),
            IpcMethod::LayersAdd => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let name = request
                    .payload
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty() && value.len() <= 120)
                    .ok_or("layer name must contain 1..120 bytes")?;
                let role: LayerRole = serde_json::from_value(
                    request
                        .payload
                        .get("role")
                        .cloned()
                        .ok_or("layer role required")?,
                )
                .map_err(|error| format!("invalid layer role: {error}"))?;
                let source = active_source(&project)?.clone();
                let (_, cas) = stores()?;
                let source_bytes = cas_get(&cas, &source.sha256, &source.media_type)?;
                let mask = initial_mask(source.width, source.height, false)?;
                let attachment =
                    render_masked_png(&source_bytes, &mask, source.width, source.height)?;
                let mask_ref = cas.put("application/vnd.f2s.alpha8", &mask)?;
                let attachment_ref = cas.put("image/png", &attachment)?;
                let mut layer_set = project
                    .active_layer_set
                    .clone()
                    .ok_or("initialize the layer set first")?;
                if layer_set.layers.len() >= 256 {
                    return Err("layer count limit reached".into());
                }
                if layer_set.approval_state == "APPROVED" {
                    layer_set.approval_state = "PENDING".into();
                    for layer in &mut layer_set.layers {
                        layer.approved = false;
                    }
                }
                add_layer(
                    &mut layer_set,
                    Layer {
                        layer_id: Uuid::new_v4().to_string(),
                        name: name.into(),
                        role,
                        attachment_sha256: attachment_ref.sha256.clone(),
                        mask_sha256: mask_ref.sha256,
                        visible: true,
                        approved: false,
                    },
                )?;
                let mut provenance = project.layer_provenance.clone();
                provenance.push(PixelProvenance {
                    artifact_sha256: attachment_ref.sha256,
                    origin: PixelOrigin::Manual,
                    source_sha256: source.sha256,
                    prompt_pack_id: None,
                    receipt_ref: None,
                    accepted_by: None,
                    acceptance_attestation_sha256: None,
                });
                project.set_layer_set(layer_set, provenance)?;
                let metrics = authoritative_layer_metrics(&project, &cas)?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                layer_projection(&project, metrics, &cas, None)
            })(),
            IpcMethod::LayersDelete => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let layer_id = request
                    .payload
                    .get("layerId")
                    .and_then(Value::as_str)
                    .ok_or("layerId required")?;
                let mut layer_set = project
                    .active_layer_set
                    .clone()
                    .ok_or("initialize the layer set first")?;
                if layer_set.approval_state == "APPROVED" {
                    layer_set.approval_state = "PENDING".into();
                    for layer in &mut layer_set.layers {
                        layer.approved = false;
                    }
                }
                remove_optional_layer(&mut layer_set, layer_id)?;
                project.set_layer_set(layer_set, project.layer_provenance.clone())?;
                let (_, cas) = stores()?;
                let metrics = authoritative_layer_metrics(&project, &cas)?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                layer_projection(&project, metrics, &cas, None)
            })(),
            IpcMethod::LayersReorder => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let ordered_ids = request
                    .payload
                    .get("layerIds")
                    .and_then(Value::as_array)
                    .ok_or("layerIds array required")?
                    .iter()
                    .map(|value| {
                        value
                            .as_str()
                            .map(str::to_owned)
                            .ok_or("layerIds must contain strings")
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let selected_layer_id = request
                    .payload
                    .get("selectedLayerId")
                    .and_then(Value::as_str);
                let mut layer_set = project
                    .active_layer_set
                    .clone()
                    .ok_or("initialize the layer set first")?;
                if layer_set.approval_state == "APPROVED" {
                    layer_set.approval_state = "PENDING".into();
                    for layer in &mut layer_set.layers {
                        layer.approved = false;
                    }
                }
                reorder_layers(&mut layer_set, &ordered_ids)?;
                project.set_layer_set(layer_set, project.layer_provenance.clone())?;
                let (_, cas) = stores()?;
                let metrics = authoritative_layer_metrics(&project, &cas)?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                layer_projection(&project, metrics, &cas, selected_layer_id)
            })(),
            IpcMethod::LayersStroke => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let stroke: ApplyMaskStroke = serde_json::from_value(request.payload.clone())
                    .map_err(|error| format!("invalid mask stroke: {error}"))?;
                stroke.validate()?;
                let source = active_source(&project)?.clone();
                let (_, cas) = stores()?;
                let source_bytes = cas_get(&cas, &source.sha256, &source.media_type)?;
                let mut layer_set = project
                    .active_layer_set
                    .clone()
                    .ok_or("initialize the layer set first")?;
                let layer = layer_set
                    .layers
                    .iter_mut()
                    .find(|layer| layer.layer_id == stroke.layer_id)
                    .ok_or("stroke layer not found")?;
                if layer.mask_sha256 != stroke.base_mask_sha256 {
                    return Err("stale base mask revision".into());
                }
                let old_mask = cas_get(&cas, &layer.mask_sha256, "application/vnd.f2s.alpha8")?;
                let current_attachment = cas_get(&cas, &layer.attachment_sha256, "image/png")?;
                let new_mask = apply_mask_stroke(&old_mask, source.width, source.height, &stroke)?;
                let attachment = render_updated_layer_attachment_png(
                    &current_attachment,
                    &source_bytes,
                    &old_mask,
                    &new_mask,
                    source.width,
                    source.height,
                )?;
                let mask_ref = cas.put("application/vnd.f2s.alpha8", &new_mask)?;
                let attachment_ref = cas.put("image/png", &attachment)?;
                layer.mask_sha256 = mask_ref.sha256;
                layer.attachment_sha256 = attachment_ref.sha256.clone();
                layer_set.revision = layer_set
                    .revision
                    .checked_add(1)
                    .ok_or("layer revision overflow")?;
                layer_set.approval_state = "PENDING".into();
                for layer in &mut layer_set.layers {
                    layer.approved = false;
                }
                let mut provenance = project.layer_provenance.clone();
                provenance.push(PixelProvenance {
                    artifact_sha256: attachment_ref.sha256,
                    origin: PixelOrigin::Manual,
                    source_sha256: source.sha256,
                    prompt_pack_id: None,
                    receipt_ref: None,
                    accepted_by: None,
                    acceptance_attestation_sha256: None,
                });
                project.set_layer_set(layer_set, provenance)?;
                let metrics = authoritative_layer_metrics(&project, &cas)?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                layer_projection(&project, metrics, &cas, Some(&stroke.layer_id))
            })(),
            IpcMethod::LayersReplacementChooseAndPreflight => {
                (|| {
                    let project = self.current_project.borrow();
                    let project = project.as_ref().ok_or("project not open")?;
                    if request.expected_revision != Some(project.revision) {
                        return Err("stale project revision".into());
                    }
                    let layer_id = request
                        .payload
                        .get("layerId")
                        .and_then(Value::as_str)
                        .ok_or("layerId required")?;
                    if !project.active_layer_set.as_ref().is_some_and(|set| {
                        set.layers.iter().any(|layer| layer.layer_id == layer_id)
                    }) {
                        return Err("replacement layer not found".into());
                    }
                    self.choose_and_stage_image(
                        hwnd,
                        PendingImportPurpose::LayerReplacement {
                            layer_id: layer_id.into(),
                        },
                        &project.identity.project_id.to_string(),
                        project.revision,
                    )
                })()
            }
            IpcMethod::LayersReplacementPromote => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let token = request
                    .payload
                    .get("stagingToken")
                    .and_then(Value::as_str)
                    .ok_or("stagingToken required")?;
                let pending = self
                    .pending
                    .borrow_mut()
                    .remove(token)
                    .ok_or("staging token missing or consumed")?;
                let PendingImportPurpose::LayerReplacement { layer_id } = pending.purpose.clone()
                else {
                    return Err("staging token is not bound to a layer replacement".into());
                };
                if pending.project_id != project.identity.project_id.to_string()
                    || pending.project_revision != project.revision
                {
                    return Err("staging token is stale or bound to another project".into());
                }
                let source = active_source(&project)?.clone();
                if pending.report.media_type != "image/png"
                    || !pending.report.has_alpha
                    || pending.report.width != source.width
                    || pending.report.height != source.height
                {
                    return Err(
                        "layer replacement must be a transparent PNG matching the master canvas"
                            .into(),
                    );
                }
                let bytes = read_stable(&pending.path)?;
                if format!("{:x}", Sha256::digest(&bytes)) != pending.report.source_sha256 {
                    return Err("staged layer replacement hash drift".into());
                }
                let (attachment, mask) =
                    normalize_manual_layer_png(&bytes, source.width, source.height)?;
                let (_, cas) = stores()?;
                let attachment_ref = cas.put("image/png", &attachment)?;
                let mask_ref = cas.put("application/vnd.f2s.alpha8", &mask)?;
                let mut layer_set = project
                    .active_layer_set
                    .clone()
                    .ok_or("initialize the layer set first")?;
                layer_set.approval_state = "PENDING".into();
                for layer in &mut layer_set.layers {
                    layer.approved = false;
                }
                replace_layer_attachment(
                    &mut layer_set,
                    &layer_id,
                    &attachment_ref.sha256,
                    &mask_ref.sha256,
                )?;
                let mut provenance = project.layer_provenance.clone();
                provenance.push(PixelProvenance {
                    artifact_sha256: attachment_ref.sha256,
                    origin: PixelOrigin::Manual,
                    source_sha256: source.sha256,
                    prompt_pack_id: None,
                    receipt_ref: None,
                    accepted_by: None,
                    acceptance_attestation_sha256: None,
                });
                project.set_layer_set(layer_set, provenance)?;
                let metrics = authoritative_layer_metrics(&project, &cas)?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                fs::remove_file(&pending.path).map_err(|error| error.to_string())?;
                layer_projection(&project, metrics, &cas, Some(&layer_id))
            })(),
            IpcMethod::LayersStatus => (|| {
                let project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request
                    .expected_revision
                    .is_some_and(|revision| revision != project.revision)
                {
                    return Err("stale project revision".into());
                }
                let (_, cas) = stores()?;
                let metrics = authoritative_layer_metrics(&project, &cas)?;
                let selected_layer_id = request.payload.get("layerId").and_then(Value::as_str);
                layer_projection(&project, metrics, &cas, selected_layer_id)
            })(),
            IpcMethod::LayersApprove => (|| {
                let current = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(current.revision) {
                    return Err("stale project revision".into());
                }
                let mut layer_set = current
                    .active_layer_set
                    .clone()
                    .ok_or("active layer set missing")?;
                if layer_set.approval_state != "PENDING" {
                    return Err("only the current pending layer set can be approved".into());
                }
                if layer_set.layers.iter().any(|layer| {
                    !current.layer_provenance.iter().any(|provenance| {
                        provenance.artifact_sha256 == layer.attachment_sha256
                            && provenance.can_enter_approved_layer()
                    })
                }) {
                    return Err("current layer attachment is missing accepted provenance".into());
                }
                let (_, cas) = stores()?;
                let metrics = authoritative_layer_metrics(&current, &cas)?;
                let payload = layer_approval_payload(&layer_set)?;
                let actor = self.verified_human(
                    hwnd,
                    "approve-layers",
                    &payload,
                    &format!(
                        "LayerSet {} / revision {} / {} layers",
                        layer_set.layer_set_id,
                        layer_set.revision,
                        layer_set.layers.len()
                    ),
                )?;
                let approval = approve_layers(
                    &mut layer_set,
                    metrics,
                    &current.layer_provenance,
                    actor,
                    &utc_now(),
                )?;
                let mut project = current;
                project.active_layer_set = Some(layer_set);
                project.record_layer_approval(approval.clone())?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                Ok(json!({
                    "approval": approval,
                    "project": project_projection(&project),
                    "layerSet": project.active_layer_set,
                    "metrics": metrics,
                    "authority": "RUST_CAS_RECOMPUTED"
                }))
            })(),
            IpcMethod::RigInitialize => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let (_, cas) = stores()?;
                if project.active_rig.is_some() {
                    return rig_projection(&project, &cas);
                }
                let layer_set = project
                    .active_layer_set
                    .as_ref()
                    .ok_or("approved LayerSet required")?;
                let layer_approval = project
                    .current_layer_approval()
                    .ok_or("current LayerSet approval required")?;
                let master = project
                    .active_master
                    .as_ref()
                    .ok_or("active master missing")?;
                let weapon = master
                    .style_spec
                    .primary_weapon
                    .clone()
                    .ok_or("primary weapon unresolved")?;
                let source = active_source(&project)?;
                let rig = build_default_side_view_humanoid_rig(
                    Uuid::new_v4().to_string(),
                    layer_set,
                    layer_approval.target_sha256.clone(),
                    weapon,
                    RigCanvas {
                        width_px: source.width,
                        height_px: source.height,
                    },
                    verified_constraint_capability()?,
                )?;
                project.set_rig(rig)?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                rig_projection(&project, &cas)
            })(),
            IpcMethod::RigStatus => (|| {
                let project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request
                    .expected_revision
                    .is_some_and(|revision| revision != project.revision)
                {
                    return Err("stale project revision".into());
                }
                let (_, cas) = stores()?;
                rig_projection(&project, &cas)
            })(),
            IpcMethod::RigSetBone => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let command: SetBoneTransformCommand =
                    serde_json::from_value(request.payload.clone())
                        .map_err(|error| format!("invalid bone transform command: {error}"))?;
                let mut rig = project.active_rig.clone().ok_or("active Rig missing")?;
                set_bone_transform(&mut rig, command)?;
                project.set_rig(rig)?;
                let (_, cas) = stores()?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                rig_projection(&project, &cas)
            })(),
            IpcMethod::RigSetSlot => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let command: SetSlotCommand = serde_json::from_value(request.payload.clone())
                    .map_err(|error| format!("invalid SetSlot command: {error}"))?;
                let mut rig = project.active_rig.clone().ok_or("active Rig missing")?;
                set_slot(&mut rig, command)?;
                project.set_rig(rig)?;
                let (_, cas) = stores()?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                rig_projection(&project, &cas)
            })(),
            IpcMethod::RigReparentBone => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let command: ReparentBoneCommand = serde_json::from_value(request.payload.clone())
                    .map_err(|error| format!("invalid reparent command: {error}"))?;
                let mut rig = project.active_rig.clone().ok_or("active Rig missing")?;
                reparent_bone(&mut rig, command)?;
                project.set_rig(rig)?;
                let (_, cas) = stores()?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                rig_projection(&project, &cas)
            })(),
            IpcMethod::RigSetPivot => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let command: SetPivotCommand = serde_json::from_value(request.payload.clone())
                    .map_err(|error| format!("invalid pivot command: {error}"))?;
                let mut rig = project.active_rig.clone().ok_or("active Rig missing")?;
                set_layer_pivot(&mut rig, command)?;
                project.set_rig(rig)?;
                let (_, cas) = stores()?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                rig_projection(&project, &cas)
            })(),
            IpcMethod::RigSetSocket => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let command: SetSocketCommand = serde_json::from_value(request.payload.clone())
                    .map_err(|error| format!("invalid socket command: {error}"))?;
                let mut rig = project.active_rig.clone().ok_or("active Rig missing")?;
                set_socket(&mut rig, command)?;
                project.set_rig(rig)?;
                let (_, cas) = stores()?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                rig_projection(&project, &cas)
            })(),
            IpcMethod::RigApprove => (|| {
                let current = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(current.revision) {
                    return Err("stale project revision".into());
                }
                let mut rig = current.active_rig.clone().ok_or("active Rig missing")?;
                let layers = current
                    .active_layer_set
                    .as_ref()
                    .ok_or("LayerSet missing")?;
                rig.validate(layers)?;
                let diagnostics = diagnose_rig_candidate(&rig, layers);
                let payload = rig_approval_payload(&rig)?;
                let actor = self.verified_human(
                    hwnd,
                    "approve-rig",
                    &payload,
                    &format!(
                        "Rig {} / revision {} / {} bones / {} slots",
                        rig.rig_id,
                        rig.revision,
                        rig.bone_tree.bones.len(),
                        rig.slot_set.slots.len()
                    ),
                )?;
                let approval =
                    approve_rig_candidate(&mut rig, layers, &diagnostics, actor, &utc_now())?;
                let mut project = current;
                project.active_rig = Some(rig);
                project.record_rig_approval(approval.clone())?;
                let (_, cas) = stores()?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                let mut value = rig_projection(&project, &cas)?;
                value["approval"] =
                    serde_json::to_value(approval).map_err(|error| error.to_string())?;
                Ok(value)
            })(),
            IpcMethod::MotionInitialize => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                if project.motion_content.is_none() {
                    let style = &project
                        .active_master
                        .as_ref()
                        .ok_or("active master missing")?
                        .style_spec;
                    let content = initialize_motion_content(style)?;
                    project.set_motion_content(content)?;
                    commit_manifest(&project)?;
                    *self.current_project.borrow_mut() = Some(project.clone());
                }
                motion_projection(&project)
            })(),
            IpcMethod::MotionStatus => (|| {
                let project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request
                    .expected_revision
                    .is_some_and(|revision| revision != project.revision)
                {
                    return Err("stale project revision".into());
                }
                motion_projection(&project)
            })(),
            IpcMethod::MotionSpecUpdate => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let spec: MotionSpec = serde_json::from_value(
                    request
                        .payload
                        .get("spec")
                        .cloned()
                        .ok_or("spec required")?,
                )
                .map_err(|error| format!("invalid MotionSpec: {error}"))?;
                let style = project
                    .active_master
                    .as_ref()
                    .ok_or("active master missing")?
                    .style_spec
                    .clone();
                let mut content = project
                    .motion_content
                    .clone()
                    .ok_or("MotionContent missing")?;
                replace_motion_spec(&mut content, &style, spec)?;
                project.set_motion_content(content)?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                motion_projection(&project)
            })(),
            IpcMethod::MotionKeyPoseChooseAndPreflight => (|| {
                let project = self.current_project.borrow();
                let project = project.as_ref().ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let asset_spec_id = request
                    .payload
                    .get("assetSpecId")
                    .and_then(Value::as_str)
                    .ok_or("assetSpecId required")?;
                if !project.motion_content.as_ref().is_some_and(|content| {
                    content
                        .assets
                        .iter()
                        .any(|asset| asset.asset_spec_id == asset_spec_id)
                }) {
                    return Err("AssetSpec not found".into());
                }
                self.choose_and_stage_image(
                    hwnd,
                    PendingImportPurpose::KeyPose {
                        asset_spec_id: asset_spec_id.into(),
                    },
                    &project.identity.project_id.to_string(),
                    project.revision,
                )
            })(),
            IpcMethod::MotionKeyPosePromote => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let token = request
                    .payload
                    .get("stagingToken")
                    .and_then(Value::as_str)
                    .ok_or("stagingToken required")?;
                let pending = self
                    .pending
                    .borrow_mut()
                    .remove(token)
                    .ok_or("staging token missing or consumed")?;
                let PendingImportPurpose::KeyPose { asset_spec_id } = pending.purpose.clone()
                else {
                    return Err("staging token is not bound to a key-pose asset".into());
                };
                if pending.project_id != project.identity.project_id.to_string()
                    || pending.project_revision != project.revision
                {
                    return Err("staging token is stale or bound to another project".into());
                }
                let bytes = read_stable(&pending.path)?;
                if format!("{:x}", Sha256::digest(&bytes)) != pending.report.source_sha256 {
                    return Err("staged image hash drift".into());
                }
                let facts = ImageFacts {
                    media_type: pending.report.media_type.clone(),
                    width: pending.report.width,
                    height: pending.report.height,
                    bit_depth: pending.report.bit_depth,
                    has_alpha: pending.report.has_alpha,
                };
                let artifact = promote_image(
                    &FsCas::new(local_data_root()?.join("cas")),
                    &bytes,
                    &facts,
                    "user-local-key-pose",
                )?;
                let mut content = project
                    .motion_content
                    .clone()
                    .ok_or("MotionContent missing")?;
                let binding = bind_key_pose_image(&mut content, &artifact, &asset_spec_id)?;
                project.set_motion_content(content)?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                fs::remove_file(&pending.path).map_err(|error| error.to_string())?;
                let mut value = motion_projection(&project)?;
                value["binding"] =
                    serde_json::to_value(binding).map_err(|error| error.to_string())?;
                Ok(value)
            })(),
            IpcMethod::MotionKeyPoseAlignmentSet => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let binding_id = request
                    .payload
                    .get("bindingId")
                    .and_then(Value::as_str)
                    .ok_or("bindingId required")?;
                let expected_binding_revision = request
                    .payload
                    .get("expectedBindingRevision")
                    .and_then(Value::as_u64)
                    .ok_or("expectedBindingRevision required")?;
                let ground_y_milli_px = request
                    .payload
                    .get("groundYMilliPx")
                    .and_then(Value::as_i64)
                    .ok_or("groundYMilliPx required")?;
                let scale_ppm_u64 = request
                    .payload
                    .get("scalePpm")
                    .and_then(Value::as_u64)
                    .ok_or("scalePpm required")?;
                let scale_ppm = u32::try_from(scale_ppm_u64)
                    .map_err(|_| "scalePpm exceeds the supported range")?;
                let mut content = project
                    .motion_content
                    .clone()
                    .ok_or("MotionContent missing")?;
                let binding = set_key_pose_alignment(
                    &mut content,
                    binding_id,
                    expected_binding_revision,
                    ground_y_milli_px,
                    scale_ppm,
                )?;
                project.set_motion_content(content)?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                let mut value = motion_projection(&project)?;
                value["binding"] =
                    serde_json::to_value(binding).map_err(|error| error.to_string())?;
                Ok(value)
            })(),
            IpcMethod::MotionKeyPosePreview => (|| {
                let current = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(current.revision) {
                    return Err("stale project revision".into());
                }
                let binding_id = request
                    .payload
                    .get("bindingId")
                    .and_then(Value::as_str)
                    .ok_or("bindingId required")?;
                let binding = current
                    .motion_content
                    .as_ref()
                    .ok_or("MotionContent missing")?
                    .key_pose_bindings
                    .iter()
                    .find(|binding| binding.binding_id == binding_id)
                    .cloned()
                    .ok_or("key-pose binding missing")?;
                let (_, cas) = stores()?;
                let bytes = cas_get(&cas, &binding.source_sha256, &binding.media_type)?;
                if format!("{:x}", Sha256::digest(&bytes)) != binding.source_sha256 {
                    return Err("key-pose CAS content hash mismatch".into());
                }
                let preview = render_safe_preview_png(&bytes, 256)?;
                let project_id = current.identity.project_id.to_string();
                let review_token = self.issue_key_pose_review(
                    &project_id,
                    current.revision,
                    &binding.binding_id,
                    &binding.source_sha256,
                );
                Ok(json!({
                    "projectId": project_id,
                    "projectRevision": current.revision,
                    "binding": binding,
                    "safePreviewDataUrl": format!(
                        "data:image/png;base64,{}",
                        BASE64_STANDARD.encode(preview)
                    ),
                    "reviewToken": review_token,
                    "authority": "RUST_CAS_BOUNDED_KEY_POSE_PREVIEW"
                }))
            })(),
            IpcMethod::MotionKeyPoseApprove => (|| {
                let current = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(current.revision) {
                    return Err("stale project revision".into());
                }
                let binding_id = request
                    .payload
                    .get("bindingId")
                    .and_then(Value::as_str)
                    .ok_or("bindingId required")?;
                let review_token = request
                    .payload
                    .get("reviewToken")
                    .and_then(Value::as_str)
                    .ok_or("reviewToken required; preview this binding before approval")?;
                let content = current
                    .motion_content
                    .as_ref()
                    .ok_or("MotionContent missing")?;
                let binding = content
                    .key_pose_bindings
                    .iter()
                    .find(|binding| binding.binding_id == binding_id);
                self.consume_key_pose_review(
                    review_token,
                    &current.identity.project_id.to_string(),
                    current.revision,
                    binding_id,
                    binding.map(|value| value.source_sha256.as_str()),
                )?;
                let binding = binding.ok_or("key-pose binding missing")?;
                let payload = content.binding_approval_payload(binding)?;
                let actor = self.verified_human(
                    hwnd,
                    "approve-key-pose-asset",
                    &payload,
                    &format!("关键姿势图片 {}/{}", binding.action_key, binding.pose_key),
                )?;
                let approval = approve_key_pose_asset(content, binding_id, actor, &utc_now())?;
                let mut project = current;
                project.record_key_pose_asset_approval(approval.clone())?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                let mut value = motion_projection(&project)?;
                value["approval"] =
                    serde_json::to_value(approval).map_err(|error| error.to_string())?;
                Ok(value)
            })(),
            IpcMethod::AnimationInitialize => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let (_, cas) = stores()?;
                if project.animation_set.is_none() {
                    let rig = project.active_rig.as_ref().ok_or("active Rig missing")?;
                    let rig_approval = project
                        .current_rig_approval()
                        .ok_or("current Rig approval required")?;
                    let motion = project
                        .motion_content
                        .as_ref()
                        .ok_or("MotionContent missing")?;
                    let animation =
                        initialize_animation_set(motion, rig, &rig_approval.target_sha256)?;
                    project.set_animation_set(animation)?;
                    commit_manifest(&project)?;
                    *self.current_project.borrow_mut() = Some(project.clone());
                }
                animation_projection(&project, &cas)
            })(),
            IpcMethod::AnimationStatus => (|| {
                let project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request
                    .expected_revision
                    .is_some_and(|revision| revision != project.revision)
                {
                    return Err("stale project revision".into());
                }
                let (_, cas) = stores()?;
                animation_projection(&project, &cas)
            })(),
            IpcMethod::AnimationTrackPut => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let action = request
                    .payload
                    .get("actionKey")
                    .and_then(Value::as_str)
                    .ok_or("actionKey required")?;
                let track: Track = serde_json::from_value(
                    request
                        .payload
                        .get("track")
                        .cloned()
                        .ok_or("track required")?,
                )
                .map_err(|error| format!("invalid Track: {error}"))?;
                let rig = project
                    .active_rig
                    .as_ref()
                    .ok_or("active Rig missing")?
                    .clone();
                let motion = project
                    .motion_content
                    .as_ref()
                    .ok_or("MotionContent missing")?
                    .clone();
                let mut animation = project
                    .animation_set
                    .clone()
                    .ok_or("AnimationSet missing")?;
                put_track(&mut animation, &motion, &rig, action, track)?;
                project.set_animation_set(animation)?;
                let (_, cas) = stores()?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                animation_projection(&project, &cas)
            })(),
            IpcMethod::AnimationPoseMarkerSet => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let action = request
                    .payload
                    .get("actionKey")
                    .and_then(Value::as_str)
                    .ok_or("actionKey required")?;
                let pose = request
                    .payload
                    .get("poseKey")
                    .and_then(Value::as_str)
                    .ok_or("poseKey required")?;
                let tick = request
                    .payload
                    .get("tick")
                    .and_then(Value::as_i64)
                    .ok_or("tick required")?;
                let rig = project
                    .active_rig
                    .as_ref()
                    .ok_or("active Rig missing")?
                    .clone();
                let motion = project
                    .motion_content
                    .as_ref()
                    .ok_or("MotionContent missing")?
                    .clone();
                let mut animation = project
                    .animation_set
                    .clone()
                    .ok_or("AnimationSet missing")?;
                set_review_pose_tick(&mut animation, &motion, &rig, action, pose, tick)?;
                project.set_animation_set(animation)?;
                let (_, cas) = stores()?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                animation_projection(&project, &cas)
            })(),
            IpcMethod::AnimationHitMarkerSet => (|| {
                let mut project = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(project.revision) {
                    return Err("stale project revision".into());
                }
                let expected_animation_revision = request
                    .payload
                    .get("expectedAnimationRevision")
                    .and_then(Value::as_u64)
                    .ok_or("expectedAnimationRevision required")?;
                let action = request
                    .payload
                    .get("actionKey")
                    .and_then(Value::as_str)
                    .ok_or("actionKey required")?;
                let tick = request
                    .payload
                    .get("tick")
                    .and_then(Value::as_i64)
                    .ok_or("tick required")?;
                let socket_id = request
                    .payload
                    .get("socketId")
                    .and_then(Value::as_str)
                    .ok_or("socketId required")?;
                let rig = project
                    .active_rig
                    .as_ref()
                    .ok_or("active Rig missing")?
                    .clone();
                let motion = project
                    .motion_content
                    .as_ref()
                    .ok_or("MotionContent missing")?
                    .clone();
                let mut animation = project
                    .animation_set
                    .clone()
                    .ok_or("AnimationSet missing")?;
                set_hit_frame_marker(
                    &mut animation,
                    &motion,
                    &rig,
                    expected_animation_revision,
                    action,
                    tick,
                    socket_id,
                )?;
                project.set_animation_set(animation)?;
                let (_, cas) = stores()?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                animation_projection(&project, &cas)
            })(),
            IpcMethod::AnimationPoseApprove => (|| {
                let current = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(current.revision) {
                    return Err("stale project revision".into());
                }
                let action = request
                    .payload
                    .get("actionKey")
                    .and_then(Value::as_str)
                    .ok_or("actionKey required")?;
                let animation = current
                    .animation_set
                    .as_ref()
                    .ok_or("AnimationSet missing")?;
                let motion = current
                    .motion_content
                    .as_ref()
                    .ok_or("MotionContent missing")?;
                if !motion.action_required_assets_approved(action) {
                    return Err(
                        "every required key-pose image for this action needs a current human approval"
                            .into(),
                    );
                }
                let payload = animation.pose_payload(motion, action)?;
                let actor = self.verified_human(
                    hwnd,
                    "approve-key-poses",
                    &payload,
                    &format!("动作 {} 的全部必需关键姿势", action),
                )?;
                let approval = approve_action_poses(animation, motion, action, actor, &utc_now())?;
                let mut project = current;
                project.record_pose_approval(approval.clone())?;
                let (_, cas) = stores()?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                let mut value = animation_projection(&project, &cas)?;
                value["approval"] =
                    serde_json::to_value(approval).map_err(|error| error.to_string())?;
                Ok(value)
            })(),
            IpcMethod::AnimationHitApprove => (|| {
                let current = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(current.revision) {
                    return Err("stale project revision".into());
                }
                let action = request
                    .payload
                    .get("actionKey")
                    .and_then(Value::as_str)
                    .ok_or("actionKey required")?;
                let animation = current
                    .animation_set
                    .as_ref()
                    .ok_or("AnimationSet missing")?;
                let motion = current
                    .motion_content
                    .as_ref()
                    .ok_or("MotionContent missing")?;
                let pose = current
                    .current_pose_approval(action)
                    .cloned()
                    .ok_or("current Pose approval required")?;
                let payload = animation.hit_payload(action)?;
                let actor = self.verified_human(
                    hwnd,
                    "approve-hit-frame",
                    &payload,
                    &format!("攻击 {} 的唯一命中帧与武器 socket", action),
                )?;
                let approval =
                    approve_action_hit(animation, motion, action, &pose, actor, &utc_now())?;
                let mut project = current;
                project.record_hit_approval(approval.clone())?;
                let (_, cas) = stores()?;
                commit_manifest(&project)?;
                *self.current_project.borrow_mut() = Some(project.clone());
                let mut value = animation_projection(&project, &cas)?;
                value["approval"] =
                    serde_json::to_value(approval).map_err(|error| error.to_string())?;
                Ok(value)
            })(),
            IpcMethod::ExportPreflight => (|| {
                let project = self.current_project.borrow();
                let project = project.as_ref().ok_or("project not open")?;
                if request
                    .expected_revision
                    .is_some_and(|revision| revision != project.revision)
                {
                    return Err("stale project revision".into());
                }
                Ok(export_projection(project))
            })(),
            IpcMethod::ExportChooseRootAndCommit => (|| {
                let current = self
                    .current_project
                    .borrow()
                    .as_ref()
                    .cloned()
                    .ok_or("project not open")?;
                if request.expected_revision != Some(current.revision) {
                    return Err("stale project revision".into());
                }
                let export_id = format!("f2s-r{}-{}", current.revision, Uuid::new_v4().simple());
                let snapshot = assemble_publish_snapshot(&current, export_id.clone())?;
                let report = preflight(&snapshot);
                if !report.passed {
                    return Err(format!(
                        "export preflight failed: {}",
                        report.errors.join(",")
                    ));
                }
                let Some(export_root) = choose_export_root(hwnd)? else {
                    return Ok(json!({
                        "cancelled": true,
                        "project": project_projection(&current),
                        "preflight": report,
                        "history": current.export_records
                    }));
                };
                validate_export_root(&export_root)?;
                let (_, cas) = stores()?;
                let layer_set = current
                    .active_layer_set
                    .as_ref()
                    .ok_or("active LayerSet missing")?;
                let mut attachment_bytes = Vec::with_capacity(snapshot.attachments.len());
                let mut psd_layers = Vec::with_capacity(snapshot.attachments.len());
                for attachment in &snapshot.attachments {
                    let layer = layer_set
                        .layers
                        .iter()
                        .find(|layer| layer.layer_id == attachment.attachment_id)
                        .ok_or("snapshot attachment no longer maps to current LayerSet")?;
                    let bytes = cas_get(&cas, &attachment.source_sha256, "image/png")?;
                    psd_layers.push(psd_layer_from_png(
                        layer.name.clone(),
                        &bytes,
                        layer.visible,
                    )?);
                    attachment_bytes.push(AttachmentBytes {
                        attachment_id: attachment.attachment_id.clone(),
                        bytes,
                    });
                }
                let prompt_pack = &current
                    .motion_content
                    .as_ref()
                    .ok_or("MotionContent missing")?
                    .prompt_pack;
                let commit = commit_open_export(
                    &snapshot,
                    &attachment_bytes,
                    &psd_layers,
                    (
                        current
                            .active_rig
                            .as_ref()
                            .ok_or("active Rig missing")?
                            .canvas
                            .width_px,
                        current
                            .active_rig
                            .as_ref()
                            .ok_or("active Rig missing")?
                            .canvas
                            .height_px,
                    ),
                    prompt_pack,
                    &export_root,
                )?;
                let snapshot_sha256 = canonical_sha256(&snapshot).map_err(|e| e.to_string())?;
                let mut project = current;
                let record = ExportRecord {
                    export_id: export_id.clone(),
                    snapshot_sha256: snapshot_sha256.clone(),
                    source_project_revision: snapshot.project_revision,
                    status: commit.status.clone(),
                    checksums: commit.checksums.clone(),
                    created_at_utc: utc_now(),
                    external_status: commit.external_editor_status.clone(),
                };
                project.append_export_record(record.clone())?;
                if let Err(error) = commit_manifest(&project) {
                    let recovery_root = local_data_root()?.join("export-recovery");
                    fs::create_dir_all(&recovery_root).map_err(|e| e.to_string())?;
                    let recovery_path = recovery_root.join(format!("{export_id}.json"));
                    let recovery = json!({
                        "schemaVersion":"1.0.0",
                        "reason":"PROJECT_HISTORY_COMMIT_FAILED_AFTER_IMMUTABLE_EXPORT",
                        "projectId":project.identity.project_id,
                        "record":record,
                        "directory":commit.directory,
                        "error":error
                    });
                    let bytes = serde_json::to_vec_pretty(&recovery).map_err(|e| e.to_string())?;
                    let mut file = OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&recovery_path)
                        .map_err(|e| e.to_string())?;
                    file.write_all(&bytes).map_err(|e| e.to_string())?;
                    file.sync_all().map_err(|e| e.to_string())?;
                    let package_token = SpineCliHost::path_token(&commit.directory);
                    let recovery_token = SpineCliHost::path_token(&recovery_path);
                    return Err(format!(
                        "immutable export completed but project history commit failed; package token: {package_token}; recovery token: {recovery_token}"
                    ));
                }
                self.spine_cli.register_open_export(OpenExportGrant {
                    export_id: export_id.clone(),
                    project_id: project.identity.project_id.to_string(),
                    project_revision: project.revision,
                    snapshot_sha256: snapshot_sha256.clone(),
                    directory: commit.directory.clone(),
                    checksums: commit.checksums.clone(),
                })?;
                let directory_token = SpineCliHost::path_token(&commit.directory);
                *self.current_project.borrow_mut() = Some(project.clone());
                Ok(json!({
                    "cancelled": false,
                    "project": project_projection(&project),
                    "preflight": report,
                    "exportId": export_id,
                    "snapshotSha256": snapshot_sha256,
                    "directoryToken": directory_token,
                    "status": commit.status,
                    "externalEditorStatus": commit.external_editor_status,
                    "checksums": commit.checksums,
                    "history": project.export_records,
                    "authority": "RUST_IMMUTABLE_OPEN_EXPORT_COMMIT"
                }))
            })(),
            IpcMethod::ExportHistory => (|| {
                let project = self.current_project.borrow();
                let project = project.as_ref().ok_or("project not open")?;
                Ok(export_projection(project))
            })(),
        };
        match result {
            Ok(value) => IpcResponse::success(id, value),
            Err(error) => {
                let (code, retryable) = classify_ipc_command_error(&error);
                IpcResponse::failure(id, code, error, retryable)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ANIMATION_PREVIEW_DATA_URL_BUDGET_BYTES, ActiveRemoteGpuProfilePointer, HostState,
        bounded_animation_preview_data_url, classify_ipc_command_error, parse_remote_gpu_profile,
        recent_project_ids, rigid_preview_bone,
    };
    use f2s_domain::rig::weights::{BoneWeight, WeightSet};
    use std::{collections::BTreeMap, fs, time::Duration};
    use uuid::Uuid;

    const SOURCE_SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn valid_remote_profile_json(extra: &str) -> Vec<u8> {
        format!(
            r#"{{
                "schemaVersion":"1.0.0",
                "enabled":true,
                "profileId":"studio-gpu-01",
                "ownership":"USER_CONTROLLED_PRIVATE",
                "origin":"https://10.4.0.8:8443",
                "allowedPorts":[8443],
                "certificateSpkiSha256":"{SOURCE_SHA256}",
                "organizationIdentitySha256":"{SOURCE_SHA256}",
                "credentialManagerTarget":"FlashToSpine/RemoteGpu/studio-gpu-01",
                "allowedMethods":["LAYER_SEGMENTATION_CANDIDATE"],
                "allowedInputMediaTypes":["IMAGE_PNG"],
                "allowedModelManifestSha256":["{SOURCE_SHA256}"],
                "maxUploadBytes":1048576,
                "maxResponseBytes":1048576,
                "requestTimeoutSeconds":30
                {extra}
            }}"#
        )
        .into_bytes()
    }

    #[test]
    fn private_remote_profile_import_is_strict_and_secret_free() {
        let profile = parse_remote_gpu_profile(&valid_remote_profile_json("")).unwrap();
        assert_eq!(profile.profile_id, "studio-gpu-01");
        assert!(
            parse_remote_gpu_profile(&valid_remote_profile_json(
                r#", "secret":"must-not-enter-ipc-or-storage""#
            ))
            .is_err()
        );
    }

    #[test]
    fn active_remote_profile_pointer_rejects_non_lowercase_hash() {
        let pointer = ActiveRemoteGpuProfilePointer {
            schema_version: "1.0.0".into(),
            profile_id: "studio-gpu-01".into(),
            profile_sha256: SOURCE_SHA256.into(),
        };
        pointer.validate().unwrap();
        let mut invalid = pointer;
        invalid.profile_sha256 = SOURCE_SHA256.to_ascii_uppercase();
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn ipc_command_errors_keep_stable_actionable_categories() {
        assert_eq!(
            classify_ipc_command_error("stale project revision"),
            ("F2S-REVISION-STALE", true)
        );
        assert_eq!(
            classify_ipc_command_error("native selection cancelled"),
            ("F2S-USER-CANCELLED", true)
        );
        assert_eq!(
            classify_ipc_command_error("project integrity check failed"),
            ("F2S-INTEGRITY", false)
        );
        assert_eq!(
            classify_ipc_command_error("primary weapon socket required"),
            ("F2S-VALIDATION", false)
        );
        assert_eq!(
            classify_ipc_command_error("project not open"),
            ("F2S-STATE-MISSING", true)
        );
    }

    #[test]
    fn recent_projects_are_modified_descending_bounded_and_uuid_only() {
        let root = std::env::temp_dir().join(format!("f2s-recent-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let mut expected = Vec::new();
        for index in 0..22u64 {
            let id = Uuid::new_v4().to_string();
            let directory = root.join(&id);
            fs::create_dir(&directory).unwrap();
            let file = fs::File::create(directory.join("head.json")).unwrap();
            file.set_modified(std::time::SystemTime::UNIX_EPOCH + Duration::from_secs(index + 1))
                .unwrap();
            expected.push(id);
        }
        fs::create_dir(root.join("not-a-project")).unwrap();
        expected.reverse();
        expected.truncate(20);
        assert_eq!(recent_project_ids(&root, 20).unwrap(), expected);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn animation_attachment_preview_accepts_only_one_full_weight_bone() {
        let rigid = WeightSet {
            mesh_id: "mesh-a".into(),
            topology_revision: 1,
            by_vertex: BTreeMap::from([
                (
                    0,
                    vec![BoneWeight {
                        bone_id: "arm".into(),
                        weight_ppm: 1_000_000,
                    }],
                ),
                (
                    1,
                    vec![BoneWeight {
                        bone_id: "arm".into(),
                        weight_ppm: 1_000_000,
                    }],
                ),
            ]),
        };
        assert_eq!(rigid_preview_bone(&rigid), Some("arm"));
        let mut split = rigid.clone();
        split.by_vertex.get_mut(&1).unwrap()[0].bone_id = "hand".into();
        assert_eq!(rigid_preview_bone(&split), None);
        let mut partial = rigid.clone();
        partial.by_vertex.get_mut(&0).unwrap()[0].weight_ppm = 999_999;
        assert_eq!(rigid_preview_bone(&partial), None);
        let mut multi = rigid;
        multi.by_vertex.get_mut(&0).unwrap().push(BoneWeight {
            bone_id: "hand".into(),
            weight_ppm: 0,
        });
        assert_eq!(rigid_preview_bone(&multi), None);
    }

    #[test]
    fn animation_attachment_preview_enforces_a_total_data_url_budget() {
        let mut total = 0;
        let value = bounded_animation_preview_data_url(&mut total, b"small-png").unwrap();
        assert!(value.starts_with("data:image/png;base64,"));
        assert_eq!(total, value.len());

        let mut almost_full = ANIMATION_PREVIEW_DATA_URL_BUDGET_BYTES - 1;
        assert!(bounded_animation_preview_data_url(&mut almost_full, b"x").is_err());
        assert_eq!(almost_full, ANIMATION_PREVIEW_DATA_URL_BUDGET_BYTES - 1);
    }

    #[test]
    fn master_review_token_is_one_time_and_complete_payload_bound() {
        let state = HostState::default();
        let token = state.issue_master_review("project-a", 4, "master-a", SOURCE_SHA256);
        state
            .consume_master_review(&token, "project-a", 4, "master-a", SOURCE_SHA256)
            .unwrap();
        assert!(
            state
                .consume_master_review(&token, "project-a", 4, "master-a", SOURCE_SHA256)
                .is_err()
        );

        let wrong_payload = state.issue_master_review("project-a", 4, "master-a", SOURCE_SHA256);
        assert!(
            state
                .consume_master_review(
                    &wrong_payload,
                    "project-a",
                    4,
                    "master-a",
                    "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                )
                .is_err()
        );
        assert!(
            state
                .consume_master_review(&wrong_payload, "project-a", 4, "master-a", SOURCE_SHA256,)
                .is_err()
        );
    }

    #[test]
    fn key_pose_review_token_is_one_time() {
        let state = HostState::default();
        let token = state.issue_key_pose_review("project-a", 7, "binding-a", SOURCE_SHA256);

        state
            .consume_key_pose_review(&token, "project-a", 7, "binding-a", Some(SOURCE_SHA256))
            .unwrap();
        assert!(
            state
                .consume_key_pose_review(&token, "project-a", 7, "binding-a", Some(SOURCE_SHA256),)
                .is_err()
        );
    }

    #[test]
    fn invalid_key_pose_review_attempt_consumes_token() {
        let state = HostState::default();
        let token = state.issue_key_pose_review("project-a", 7, "binding-a", SOURCE_SHA256);

        assert!(
            state
                .consume_key_pose_review(&token, "project-a", 8, "binding-a", Some(SOURCE_SHA256),)
                .is_err()
        );
        assert!(
            state
                .consume_key_pose_review(&token, "project-a", 7, "binding-a", Some(SOURCE_SHA256),)
                .is_err()
        );
    }

    #[test]
    fn key_pose_review_token_is_bound_to_binding_and_source() {
        let state = HostState::default();
        let wrong_binding = state.issue_key_pose_review("project-a", 7, "binding-a", SOURCE_SHA256);
        assert!(
            state
                .consume_key_pose_review(
                    &wrong_binding,
                    "project-a",
                    7,
                    "binding-b",
                    Some(SOURCE_SHA256),
                )
                .is_err()
        );

        let wrong_source = state.issue_key_pose_review("project-a", 7, "binding-a", SOURCE_SHA256);
        assert!(
            state
                .consume_key_pose_review(
                    &wrong_source,
                    "project-a",
                    7,
                    "binding-a",
                    Some("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
                )
                .is_err()
        );
    }
}

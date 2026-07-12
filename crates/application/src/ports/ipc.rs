use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IpcMethod {
    #[serde(rename = "bootstrap.status")]
    BootstrapStatus,
    #[serde(rename = "remoteGpu.status")]
    RemoteGpuStatus,
    #[serde(rename = "remoteGpu.importProfile")]
    RemoteGpuImportProfile,
    #[serde(rename = "remoteGpu.disable")]
    RemoteGpuDisable,
    #[serde(rename = "image.chooseAndPreflight")]
    ImageChooseAndPreflight,
    #[serde(rename = "image.promote")]
    ImagePromote,
    #[serde(rename = "project.create")]
    ProjectCreate,
    #[serde(rename = "project.open")]
    ProjectOpen,
    #[serde(rename = "project.recent")]
    ProjectRecent,
    #[serde(rename = "master.create")]
    MasterCreate,
    #[serde(rename = "master.preview")]
    MasterPreview,
    #[serde(rename = "master.approve")]
    MasterApprove,
    #[serde(rename = "master.reject")]
    MasterReject,
    #[serde(rename = "layers.initialize")]
    LayersInitialize,
    #[serde(rename = "layers.add")]
    LayersAdd,
    #[serde(rename = "layers.delete")]
    LayersDelete,
    #[serde(rename = "layers.reorder")]
    LayersReorder,
    #[serde(rename = "layers.stroke")]
    LayersStroke,
    #[serde(rename = "layers.replacement.chooseAndPreflight")]
    LayersReplacementChooseAndPreflight,
    #[serde(rename = "layers.replacement.promote")]
    LayersReplacementPromote,
    #[serde(rename = "layers.status")]
    LayersStatus,
    #[serde(rename = "layers.approve")]
    LayersApprove,
    #[serde(rename = "rig.initialize")]
    RigInitialize,
    #[serde(rename = "rig.status")]
    RigStatus,
    #[serde(rename = "rig.setBone")]
    RigSetBone,
    #[serde(rename = "rig.setSlot")]
    RigSetSlot,
    #[serde(rename = "rig.reparentBone")]
    RigReparentBone,
    #[serde(rename = "rig.setPivot")]
    RigSetPivot,
    #[serde(rename = "rig.setSocket")]
    RigSetSocket,
    #[serde(rename = "rig.approve")]
    RigApprove,
    #[serde(rename = "motion.initialize")]
    MotionInitialize,
    #[serde(rename = "motion.status")]
    MotionStatus,
    #[serde(rename = "motion.spec.update")]
    MotionSpecUpdate,
    #[serde(rename = "motion.keyPose.chooseAndPreflight")]
    MotionKeyPoseChooseAndPreflight,
    #[serde(rename = "motion.keyPose.promote")]
    MotionKeyPosePromote,
    #[serde(rename = "motion.keyPose.alignment.set")]
    MotionKeyPoseAlignmentSet,
    #[serde(rename = "motion.keyPose.preview")]
    MotionKeyPosePreview,
    #[serde(rename = "motion.keyPose.approve")]
    MotionKeyPoseApprove,
    #[serde(rename = "animation.initialize")]
    AnimationInitialize,
    #[serde(rename = "animation.status")]
    AnimationStatus,
    #[serde(rename = "animation.track.put")]
    AnimationTrackPut,
    #[serde(rename = "animation.poseMarker.set")]
    AnimationPoseMarkerSet,
    #[serde(rename = "animation.hitMarker.set")]
    AnimationHitMarkerSet,
    #[serde(rename = "animation.pose.approve")]
    AnimationPoseApprove,
    #[serde(rename = "animation.hit.approve")]
    AnimationHitApprove,
    #[serde(rename = "export.preflight")]
    ExportPreflight,
    #[serde(rename = "export.chooseRootAndCommit")]
    ExportChooseRootAndCommit,
    #[serde(rename = "export.history")]
    ExportHistory,
    #[serde(rename = "spineCli.status")]
    SpineCliStatus,
    #[serde(rename = "spineCli.selectAndAssess")]
    SpineCliSelectAndAssess,
    #[serde(rename = "spineCli.clear")]
    SpineCliClear,
    #[serde(rename = "spineCli.job.start")]
    SpineCliJobStart,
    #[serde(rename = "spineCli.job.status")]
    SpineCliJobStatus,
    #[serde(rename = "diagnostics.status")]
    DiagnosticsStatus,
    #[serde(rename = "diagnostics.export")]
    DiagnosticsExport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IpcRequest {
    pub schema_version: String,
    pub request_id: String,
    pub method: IpcMethod,
    pub expected_revision: Option<u64>,
    pub payload: Value,
}

impl IpcRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != "1.0.0"
            || self.request_id.is_empty()
            || self.request_id.len() > 128
            || !self
                .request_id
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || matches!(v, b'-' | b'_'))
            || !self.payload.is_object()
        {
            return Err("invalid IPC request envelope".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IpcError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IpcResponse {
    pub schema_version: String,
    pub request_id: String,
    pub ok: bool,
    pub result: Option<Value>,
    pub error: Option<IpcError>,
}

impl IpcResponse {
    pub fn success(request_id: impl Into<String>, result: Value) -> Self {
        Self {
            schema_version: "1.0.0".into(),
            request_id: request_id.into(),
            ok: true,
            result: Some(result),
            error: None,
        }
    }
    pub fn failure(
        request_id: impl Into<String>,
        code: &str,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            schema_version: "1.0.0".into(),
            request_id: request_id.into(),
            ok: false,
            result: None,
            error: Some(IpcError {
                code: code.into(),
                message: message.into(),
                retryable,
            }),
        }
    }
}

pub trait IpcPort {
    fn request(&self, request: &IpcRequest) -> IpcResponse;
}

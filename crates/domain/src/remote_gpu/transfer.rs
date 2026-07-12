use super::{
    RemoteGpuMethod, RemoteGpuProfile, RemoteMediaType, is_lower_hex_sha256, is_safe_identifier,
};
use crate::canonical::canonical_sha256;
use serde::{Deserialize, Serialize};

pub const REMOTE_TRANSFER_APPROVAL_PURPOSE: &str = "remote_gpu.transfer.approve.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RemoteInputPurpose {
    ApprovedMasterImage,
    SelectionMaskImage,
    ApprovedLayerImage,
    ApprovedLayerManifest,
    ApprovedRigIr,
    MotionSpec,
    KeyPoseImage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RemoteOutputPurpose {
    LayerCandidateManifest,
    LayerCandidateMask,
    RigCandidateManifest,
    AnimationCandidateManifest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteTransferItem {
    pub artifact_id: String,
    pub sha256: String,
    pub byte_length: u64,
    pub media_type: RemoteMediaType,
    pub purpose: RemoteInputPurpose,
}

impl RemoteTransferItem {
    fn validate(&self, method: RemoteGpuMethod) -> Result<(), String> {
        if !is_safe_identifier(&self.artifact_id)
            || !is_lower_hex_sha256(&self.sha256)
            || self.byte_length == 0
        {
            return Err(
                "remote transfer artifact identity, hash, or byte length is invalid".into(),
            );
        }
        if !input_media_matches_purpose(self.media_type, self.purpose)
            || !method_accepts_purpose(method, self.purpose)
        {
            return Err(
                "remote transfer media/purpose is not allowed for this fixed method".into(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteModelBinding {
    pub model_id: String,
    pub exact_version: String,
    pub manifest_sha256: String,
}

impl RemoteModelBinding {
    fn validate(&self) -> Result<(), String> {
        if !is_safe_identifier(&self.model_id)
            || self.exact_version.trim() != self.exact_version
            || self.exact_version.is_empty()
            || self.exact_version.len() > 96
            || self
                .exact_version
                .bytes()
                .any(|byte| byte.is_ascii_control())
            || !is_lower_hex_sha256(&self.manifest_sha256)
        {
            return Err("remote model binding must use an exact audited identity".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteRetentionPolicy {
    pub delete_after_seconds: u32,
    pub require_deletion_receipt: bool,
}

impl RemoteRetentionPolicy {
    fn validate(&self) -> Result<(), String> {
        if !(60..=86_400).contains(&self.delete_after_seconds) || !self.require_deletion_receipt {
            return Err("remote retention must be bounded and require a deletion receipt".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteTransferPlan {
    pub schema_version: String,
    pub operation_id: String,
    pub project_id: String,
    pub project_revision: u64,
    pub profile_id: String,
    pub profile_sha256: String,
    pub endpoint_origin: String,
    pub endpoint_spki_sha256: String,
    pub organization_identity_sha256: String,
    pub method: RemoteGpuMethod,
    pub method_schema: u32,
    pub normalized_params_sha256: String,
    pub model: RemoteModelBinding,
    pub items: Vec<RemoteTransferItem>,
    pub total_bytes: u64,
    pub retention: RemoteRetentionPolicy,
    pub plan_sha256: String,
    pub idempotency_key: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TransferPlanHashPayload<'a> {
    schema_version: &'a str,
    operation_id: &'a str,
    project_id: &'a str,
    project_revision: u64,
    profile_id: &'a str,
    profile_sha256: &'a str,
    endpoint_origin: &'a str,
    endpoint_spki_sha256: &'a str,
    organization_identity_sha256: &'a str,
    method: RemoteGpuMethod,
    method_schema: u32,
    normalized_params_sha256: &'a str,
    model: &'a RemoteModelBinding,
    items: &'a [RemoteTransferItem],
    total_bytes: u64,
    retention: &'a RemoteRetentionPolicy,
}

impl RemoteTransferPlan {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        operation_id: impl Into<String>,
        project_id: impl Into<String>,
        project_revision: u64,
        profile: &RemoteGpuProfile,
        method: RemoteGpuMethod,
        normalized_params_sha256: impl Into<String>,
        model: RemoteModelBinding,
        mut items: Vec<RemoteTransferItem>,
        retention: RemoteRetentionPolicy,
    ) -> Result<Self, String> {
        profile.require_enabled()?;
        let operation_id = operation_id.into();
        let project_id = project_id.into();
        let normalized_params_sha256 = normalized_params_sha256.into();
        if !is_safe_identifier(&operation_id) || !is_safe_identifier(&project_id) {
            return Err("remote operation and project identities are invalid".into());
        }
        if !profile.allowed_methods.contains(&method) || !method.is_candidate_only() {
            return Err("remote method is not explicitly allowed by the private profile".into());
        }
        if !is_lower_hex_sha256(&normalized_params_sha256) {
            return Err("normalized remote parameters hash is invalid".into());
        }
        model.validate()?;
        if !profile
            .allowed_model_manifest_sha256
            .contains(&model.manifest_sha256)
        {
            return Err("remote model manifest is not allowlisted by the private profile".into());
        }
        retention.validate()?;
        if items.is_empty() {
            return Err("remote transfer plan cannot be empty".into());
        }
        items.sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));
        if items
            .windows(2)
            .any(|pair| pair[0].artifact_id == pair[1].artifact_id)
        {
            return Err("remote transfer artifact identities must be unique".into());
        }
        let mut total_bytes = 0u64;
        for item in &items {
            item.validate(method)?;
            if !profile.allowed_input_media_types.contains(&item.media_type) {
                return Err(
                    "remote input media type is not allowlisted by the private profile".into(),
                );
            }
            total_bytes = total_bytes
                .checked_add(item.byte_length)
                .ok_or("remote transfer size overflow")?;
        }
        if total_bytes > profile.max_upload_bytes {
            return Err("remote transfer exceeds the profile upload budget".into());
        }
        let mut plan = Self {
            schema_version: "1.0.0".into(),
            operation_id,
            project_id,
            project_revision,
            profile_id: profile.profile_id.clone(),
            profile_sha256: profile.canonical_sha256()?,
            endpoint_origin: profile.origin.clone(),
            endpoint_spki_sha256: profile.certificate_spki_sha256.clone(),
            organization_identity_sha256: profile.organization_identity_sha256.clone(),
            method,
            method_schema: RemoteGpuMethod::METHOD_SCHEMA,
            normalized_params_sha256,
            model,
            items,
            total_bytes,
            retention,
            plan_sha256: String::new(),
            idempotency_key: String::new(),
        };
        plan.plan_sha256 = plan.recompute_sha256()?;
        plan.idempotency_key = format!("sha256:{}", plan.plan_sha256);
        plan.validate_against_profile(profile)?;
        Ok(plan)
    }

    pub fn recompute_sha256(&self) -> Result<String, String> {
        canonical_sha256(&TransferPlanHashPayload {
            schema_version: &self.schema_version,
            operation_id: &self.operation_id,
            project_id: &self.project_id,
            project_revision: self.project_revision,
            profile_id: &self.profile_id,
            profile_sha256: &self.profile_sha256,
            endpoint_origin: &self.endpoint_origin,
            endpoint_spki_sha256: &self.endpoint_spki_sha256,
            organization_identity_sha256: &self.organization_identity_sha256,
            method: self.method,
            method_schema: self.method_schema,
            normalized_params_sha256: &self.normalized_params_sha256,
            model: &self.model,
            items: &self.items,
            total_bytes: self.total_bytes,
            retention: &self.retention,
        })
        .map_err(|error| error.to_string())
    }

    pub fn validate_against_profile(&self, profile: &RemoteGpuProfile) -> Result<(), String> {
        profile.require_enabled()?;
        if self.schema_version != "1.0.0"
            || self.method_schema != RemoteGpuMethod::METHOD_SCHEMA
            || self.profile_id != profile.profile_id
            || self.profile_sha256 != profile.canonical_sha256()?
            || self.endpoint_origin != profile.origin
            || self.endpoint_spki_sha256 != profile.certificate_spki_sha256
            || self.organization_identity_sha256 != profile.organization_identity_sha256
            || self.idempotency_key != format!("sha256:{}", self.plan_sha256)
            || self.recompute_sha256()? != self.plan_sha256
            || !is_lower_hex_sha256(&self.plan_sha256)
        {
            return Err(
                "remote transfer plan no longer matches the approved canonical profile".into(),
            );
        }
        if !is_safe_identifier(&self.operation_id)
            || !is_safe_identifier(&self.project_id)
            || !profile.allowed_methods.contains(&self.method)
            || !is_lower_hex_sha256(&self.normalized_params_sha256)
        {
            return Err("remote transfer plan identity or method is invalid".into());
        }
        self.model.validate()?;
        self.retention.validate()?;
        if !profile
            .allowed_model_manifest_sha256
            .contains(&self.model.manifest_sha256)
            || self.items.is_empty()
            || self
                .items
                .windows(2)
                .any(|pair| pair[0].artifact_id >= pair[1].artifact_id)
        {
            return Err("remote transfer plan allowlist or ordering is invalid".into());
        }
        let mut total = 0u64;
        for item in &self.items {
            item.validate(self.method)?;
            if !profile.allowed_input_media_types.contains(&item.media_type) {
                return Err("remote transfer media is outside the profile allowlist".into());
            }
            total = total
                .checked_add(item.byte_length)
                .ok_or("remote transfer size overflow")?;
        }
        if total != self.total_bytes || total > profile.max_upload_bytes {
            return Err("remote transfer byte total is invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteOutputDescriptor {
    pub artifact_id: String,
    pub sha256: String,
    pub byte_length: u64,
    pub media_type: RemoteMediaType,
    pub purpose: RemoteOutputPurpose,
}

impl RemoteOutputDescriptor {
    pub fn validate_for(
        &self,
        method: RemoteGpuMethod,
        max_response_bytes: u64,
    ) -> Result<(), String> {
        if !is_safe_identifier(&self.artifact_id)
            || !is_lower_hex_sha256(&self.sha256)
            || self.byte_length == 0
            || self.byte_length > max_response_bytes
            || !output_matches_method(method, self.media_type, self.purpose)
        {
            return Err("remote output is not a bounded candidate artifact".into());
        }
        Ok(())
    }
}

fn input_media_matches_purpose(media: RemoteMediaType, purpose: RemoteInputPurpose) -> bool {
    use RemoteInputPurpose as Purpose;
    use RemoteMediaType as Media;
    match purpose {
        Purpose::ApprovedMasterImage
        | Purpose::SelectionMaskImage
        | Purpose::ApprovedLayerImage
        | Purpose::KeyPoseImage => {
            matches!(media, Media::ImagePng | Media::ImageJpeg | Media::ImageWebp)
        }
        Purpose::ApprovedLayerManifest | Purpose::MotionSpec => media == Media::ApplicationJson,
        Purpose::ApprovedRigIr => media == Media::ApplicationRigIrJson,
    }
}

fn method_accepts_purpose(method: RemoteGpuMethod, purpose: RemoteInputPurpose) -> bool {
    use RemoteGpuMethod as Method;
    use RemoteInputPurpose as Purpose;
    match method {
        Method::LayerSegmentationCandidate => {
            matches!(
                purpose,
                Purpose::ApprovedMasterImage | Purpose::SelectionMaskImage
            )
        }
        Method::RigProposalCandidate => {
            matches!(
                purpose,
                Purpose::ApprovedLayerImage | Purpose::ApprovedLayerManifest
            )
        }
        Method::MotionCurveCandidate => matches!(
            purpose,
            Purpose::ApprovedRigIr | Purpose::MotionSpec | Purpose::KeyPoseImage
        ),
    }
}

fn output_matches_method(
    method: RemoteGpuMethod,
    media: RemoteMediaType,
    purpose: RemoteOutputPurpose,
) -> bool {
    use RemoteGpuMethod as Method;
    use RemoteMediaType as Media;
    use RemoteOutputPurpose as Purpose;
    match method {
        Method::LayerSegmentationCandidate => matches!(
            (media, purpose),
            (Media::ApplicationJson, Purpose::LayerCandidateManifest)
                | (Media::ImagePng, Purpose::LayerCandidateMask)
        ),
        Method::RigProposalCandidate => {
            (media, purpose) == (Media::ApplicationJson, Purpose::RigCandidateManifest)
        }
        Method::MotionCurveCandidate => {
            (media, purpose) == (Media::ApplicationJson, Purpose::AnimationCandidateManifest)
        }
    }
}

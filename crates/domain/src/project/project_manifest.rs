use super::ProjectIdentity;
use crate::{
    ACTION_KEYS, TimeBase,
    animation::set::AnimationSet,
    governance::{Approval, ReviewOutcome, ReviewRecord},
    import::SourceArtifact,
    layers::{LayerSet, PixelProvenance},
    master::MasterCandidate,
    motion::{content::MotionContent, registry::requires_hit_frame},
    rig::{
        RigApprovalState, RigCandidate, layer_set_approval_payload_sha256,
        rig_approval_payload_sha256,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportRecord {
    pub export_id: String,
    pub snapshot_sha256: String,
    pub source_project_revision: u64,
    pub status: String,
    pub checksums: BTreeMap<String, String>,
    pub created_at_utc: String,
    pub external_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectManifest {
    pub schema_version: String,
    pub identity: ProjectIdentity,
    pub revision: u64,
    pub time_base: TimeBase,
    pub workflow_stage: String,
    pub source_artifacts: Vec<SourceArtifact>,
    pub active_master: Option<MasterCandidate>,
    #[serde(default)]
    pub active_layer_set: Option<LayerSet>,
    #[serde(default)]
    pub layer_provenance: Vec<PixelProvenance>,
    #[serde(default)]
    pub active_rig: Option<RigCandidate>,
    #[serde(default)]
    pub motion_content: Option<MotionContent>,
    #[serde(default)]
    pub animation_set: Option<AnimationSet>,
    pub approval_log: Vec<Approval>,
    #[serde(default)]
    pub review_log: Vec<ReviewRecord>,
    #[serde(default)]
    pub export_records: Vec<ExportRecord>,
}

fn lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_export_record_shape(record: &ExportRecord) -> bool {
    !record.export_id.is_empty()
        && record.export_id.len() <= 80
        && record
            .export_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        && lower_sha256(&record.snapshot_sha256)
        && record.status == "EXPORTED_UNVERIFIED"
        && record.external_status == "EXPORTED_UNVERIFIED"
        && !record.checksums.is_empty()
        && record
            .checksums
            .iter()
            .all(|(path, hash)| !path.trim().is_empty() && lower_sha256(hash))
        && !record.created_at_utc.trim().is_empty()
}

impl ProjectManifest {
    pub fn new(identity: ProjectIdentity) -> Self {
        Self {
            schema_version: "1.4.0".into(),
            identity,
            revision: 0,
            time_base: TimeBase::default(),
            workflow_stage: "draft".into(),
            source_artifacts: vec![],
            active_master: None,
            active_layer_set: None,
            layer_provenance: vec![],
            active_rig: None,
            motion_content: None,
            animation_set: None,
            approval_log: vec![],
            review_log: vec![],
            export_records: vec![],
        }
    }

    pub fn add_source(&mut self, source: SourceArtifact) {
        self.invalidate_gate_and_downstream("master");
        self.source_artifacts
            .retain(|existing| existing.artifact_id != source.artifact_id);
        self.source_artifacts.push(source);
        self.active_master = None;
        self.active_layer_set = None;
        self.layer_provenance.clear();
        self.active_rig = None;
        self.motion_content = None;
        self.animation_set = None;
        self.revision += 1;
        self.workflow_stage = "master".into();
    }

    pub fn set_master(&mut self, master: MasterCandidate) {
        self.invalidate_gate_and_downstream("master");
        self.active_master = Some(master);
        self.active_layer_set = None;
        self.layer_provenance.clear();
        self.active_rig = None;
        self.motion_content = None;
        self.animation_set = None;
        self.revision += 1;
        self.workflow_stage = "master".into();
    }

    pub fn record_master_approval(&mut self, approval: Approval) -> Result<(), String> {
        let master = self.active_master.as_ref().ok_or("active master missing")?;
        let payload = master.approval_payload_sha256()?;
        if master.approval_state != "APPROVED"
            || approval.gate_id != "master"
            || !approval.is_valid_for(&master.master_id, master.candidate_revision, &payload)
        {
            return Err("master approval is not bound to active candidate".into());
        }
        let target_id = master.master_id.clone();
        self.invalidate_exact_gate("master", &target_id);
        self.approval_log.push(approval);
        self.revision += 1;
        self.workflow_stage = "layers".into();
        Ok(())
    }

    pub fn record_master_rejection(&mut self, review: ReviewRecord) -> Result<(), String> {
        let master = self.active_master.as_ref().ok_or("active master missing")?;
        if master.approval_state != "REJECTED"
            || review.gate_id != "master"
            || review.outcome != ReviewOutcome::Rejected
            || review.target_id != master.master_id
            || review.target_revision != master.candidate_revision
            || review.target_sha256 != master.source_sha256
        {
            return Err("master rejection is not bound to active candidate".into());
        }
        self.invalidate_gate_and_downstream("master");
        self.active_layer_set = None;
        self.layer_provenance.clear();
        self.active_rig = None;
        self.motion_content = None;
        self.animation_set = None;
        self.review_log.push(review);
        self.revision += 1;
        self.workflow_stage = "master".into();
        Ok(())
    }

    pub fn set_layer_set(
        &mut self,
        layer_set: LayerSet,
        provenance: Vec<PixelProvenance>,
    ) -> Result<(), String> {
        let master = self.active_master.as_ref().ok_or("active master missing")?;
        if self.current_master_approval().is_none() {
            return Err("approved master required before layering".into());
        }
        if layer_set.master_id != master.master_id {
            return Err("layer set is not bound to active master".into());
        }
        if layer_set.approval_state != "PENDING"
            || layer_set.layers.iter().any(|layer| layer.approved)
        {
            return Err("set_layer_set accepts only an unapproved candidate".into());
        }
        layer_set.validate()?;
        self.invalidate_gate_and_downstream("layers");
        self.active_layer_set = Some(layer_set);
        self.layer_provenance = provenance;
        self.active_rig = None;
        self.animation_set = None;
        self.revision += 1;
        self.workflow_stage = "layers".into();
        Ok(())
    }

    pub fn record_layer_approval(&mut self, approval: Approval) -> Result<(), String> {
        let layer_set = self
            .active_layer_set
            .as_ref()
            .ok_or("active layer set missing")?;
        if layer_set.approval_state != "APPROVED"
            || layer_set.layers.iter().any(|layer| !layer.approved)
        {
            return Err("active layer set has not been approved".into());
        }
        let payload = layer_set_approval_payload_sha256(layer_set)?;
        if approval.gate_id != "layers"
            || !approval.is_valid_for(&layer_set.layer_set_id, layer_set.revision, &payload)
        {
            return Err("layer approval is not bound to active layer set".into());
        }
        let target_id = layer_set.layer_set_id.clone();
        self.invalidate_exact_gate("layers", &target_id);
        self.approval_log.push(approval);
        self.revision += 1;
        self.workflow_stage = "rig".into();
        Ok(())
    }

    pub fn set_rig(&mut self, rig: RigCandidate) -> Result<(), String> {
        let layer_set = self
            .active_layer_set
            .as_ref()
            .ok_or("active layer set missing")?;
        if self.current_layer_approval().is_none() {
            return Err("approved LayerSet required before Rig editing".into());
        }
        if rig.approval_state != RigApprovalState::Pending {
            return Err("set_rig accepts only a pending candidate".into());
        }
        rig.validate(layer_set)?;
        self.invalidate_gate_and_downstream("rig");
        self.active_rig = Some(rig);
        self.animation_set = None;
        self.revision += 1;
        self.workflow_stage = "rig".into();
        Ok(())
    }

    pub fn record_rig_approval(&mut self, approval: Approval) -> Result<(), String> {
        let rig = self.active_rig.as_ref().ok_or("active Rig missing")?;
        if rig.approval_state != RigApprovalState::Approved {
            return Err("active Rig has not been approved".into());
        }
        let payload = rig_approval_payload_sha256(rig)?;
        if approval.gate_id != "rig" || !approval.is_valid_for(&rig.rig_id, rig.revision, &payload)
        {
            return Err("Rig approval is not bound to active candidate".into());
        }
        let target_id = rig.rig_id.clone();
        self.invalidate_exact_gate("rig", &target_id);
        self.approval_log.push(approval);
        self.revision += 1;
        self.workflow_stage = if self.motion_content.is_some() {
            "animation".into()
        } else {
            "motion".into()
        };
        Ok(())
    }

    pub fn set_motion_content(&mut self, content: MotionContent) -> Result<(), String> {
        let master = self.active_master.as_ref().ok_or("active master missing")?;
        if self.current_master_approval().is_none() {
            return Err("approved master required before MotionContent".into());
        }
        content.validate(&master.style_spec)?;
        for asset in content
            .assets
            .iter()
            .filter(|asset| asset.state == crate::motion::assets::AssetState::Approved)
        {
            let binding = content
                .key_pose_bindings
                .iter()
                .find(|binding| binding.asset_spec_id == asset.asset_spec_id)
                .ok_or("approved AssetSpec binding missing")?;
            let payload = content.binding_approval_payload(binding)?;
            if !self.approval_log.iter().any(|approval| {
                approval.gate_id == "key-pose-asset"
                    && approval.is_valid_for(&binding.binding_id, binding.revision, &payload)
            }) {
                return Err("approved AssetSpec has no current human approval".into());
            }
        }
        let changed_binding_actions = self
            .motion_content
            .as_ref()
            .map(|previous| {
                ACTION_KEYS
                    .iter()
                    .filter(|action| {
                        previous
                            .key_pose_bindings
                            .iter()
                            .filter(|binding| binding.action_key == **action)
                            .ne(content
                                .key_pose_bindings
                                .iter()
                                .filter(|binding| binding.action_key == **action))
                    })
                    .copied()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let changed_or_removed_binding_ids = self
            .motion_content
            .as_ref()
            .map(|previous| {
                previous
                    .key_pose_bindings
                    .iter()
                    .filter(|old| {
                        !content
                            .key_pose_bindings
                            .iter()
                            .any(|new| new.binding_id == old.binding_id && new == *old)
                    })
                    .map(|binding| binding.binding_id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let previous_input = self
            .motion_content
            .as_ref()
            .map(MotionContent::animation_input_sha256)
            .transpose()?;
        let next_input = content.animation_input_sha256()?;
        if previous_input.as_deref() != Some(next_input.as_str()) {
            self.invalidate_motion_and_animation_approvals();
            self.animation_set = None;
        } else {
            for action in changed_binding_actions {
                self.invalidate_action_gate("poses", action);
                self.invalidate_action_gate("hits", action);
            }
        }
        for binding_id in changed_or_removed_binding_ids {
            self.invalidate_exact_gate("key-pose-asset", &binding_id);
        }
        self.invalidate_key_pose_approvals_not_in(&content);
        self.motion_content = Some(content);
        self.revision += 1;
        self.workflow_stage = if self.current_rig_approval().is_some() {
            "animation".into()
        } else {
            "motion".into()
        };
        Ok(())
    }

    pub fn record_key_pose_asset_approval(&mut self, approval: Approval) -> Result<(), String> {
        let content = self
            .motion_content
            .as_mut()
            .ok_or("MotionContent missing")?;
        let mut next = content.clone();
        next.apply_asset_approval(&approval)?;
        *content = next;
        self.invalidate_exact_gate("key-pose-asset", &approval.target_id);
        self.approval_log.push(approval);
        self.revision += 1;
        Ok(())
    }

    pub fn set_animation_set(&mut self, animation: AnimationSet) -> Result<(), String> {
        let rig = self.active_rig.as_ref().ok_or("active Rig missing")?;
        let motion = self
            .motion_content
            .as_ref()
            .ok_or("MotionContent missing")?;
        if self.current_rig_approval().is_none() {
            return Err("approved Rig required before AnimationSet".into());
        }
        animation.validate(
            motion,
            &rig.bone_tree,
            &rig.slot_set,
            &rig.sockets
                .iter()
                .map(|socket| socket.socket_id.clone())
                .collect::<Vec<_>>(),
        )?;
        let mut invalidations = Vec::new();
        if let Some(previous) = &self.animation_set {
            for action in ACTION_KEYS {
                let previous_clip = previous.clip(action)?;
                let next_clip = animation.clip(action)?;
                let clip_changed = previous_clip != next_clip;
                let poses_changed = previous
                    .review_pose_markers
                    .iter()
                    .filter(|marker| marker.action_key == action)
                    .ne(animation
                        .review_pose_markers
                        .iter()
                        .filter(|marker| marker.action_key == action));
                let hits_changed = previous
                    .gameplay_markers
                    .iter()
                    .filter(|marker| marker.action_key == action)
                    .ne(animation
                        .gameplay_markers
                        .iter()
                        .filter(|marker| marker.action_key == action));
                if clip_changed || poses_changed {
                    invalidations.push(("poses", action));
                    invalidations.push(("hits", action));
                } else if hits_changed {
                    invalidations.push(("hits", action));
                }
            }
        } else {
            // Creating the first AnimationSet must not revoke current human reviews of
            // key-pose source images. Those approvals bind MotionContent assets, while
            // only pose/hit reviews depend on the AnimationSet itself.
            self.invalidate_animation_approvals();
        }
        for (gate, action) in invalidations {
            self.invalidate_action_gate(gate, action);
        }
        self.animation_set = Some(animation);
        self.revision += 1;
        self.workflow_stage = "animation".into();
        Ok(())
    }

    pub fn record_pose_approval(&mut self, approval: Approval) -> Result<(), String> {
        let animation = self.animation_set.as_ref().ok_or("AnimationSet missing")?;
        let motion = self
            .motion_content
            .as_ref()
            .ok_or("MotionContent missing")?;
        let clip = animation.clip(&approval.target_id)?;
        let payload = animation.pose_payload(motion, &approval.target_id)?;
        if approval.gate_id != "poses"
            || !approval.is_valid_for(&clip.action_key, clip.revision, &payload)
        {
            return Err("pose approval is stale or bound to another action".into());
        }
        let action_key = clip.action_key.clone();
        self.invalidate_action_gate("poses", &action_key);
        self.invalidate_action_gate("hits", &action_key);
        self.approval_log.push(approval);
        self.revision += 1;
        self.workflow_stage = "review".into();
        Ok(())
    }

    pub fn record_hit_approval(&mut self, approval: Approval) -> Result<(), String> {
        let animation = self.animation_set.as_ref().ok_or("AnimationSet missing")?;
        let motion = self
            .motion_content
            .as_ref()
            .ok_or("MotionContent missing")?;
        let clip = animation.clip(&approval.target_id)?;
        let pose = self
            .current_pose_approval(&clip.action_key)
            .ok_or("current pose approval required before hit approval")?;
        let pose_payload = animation.pose_payload(motion, &clip.action_key)?;
        if !pose.is_valid_for(&clip.action_key, clip.revision, &pose_payload) {
            return Err("pose approval became stale".into());
        }
        let payload = animation.hit_payload(&clip.action_key)?;
        if approval.gate_id != "hits"
            || !approval.is_valid_for(&clip.action_key, clip.revision, &payload)
        {
            return Err("hit approval is stale or bound to another attack".into());
        }
        let action_key = clip.action_key.clone();
        self.invalidate_action_gate("hits", &action_key);
        self.approval_log.push(approval);
        self.revision += 1;
        self.workflow_stage = if self.approval_closure_complete() {
            "export".into()
        } else {
            "review".into()
        };
        Ok(())
    }

    pub fn append_export_record(&mut self, record: ExportRecord) -> Result<(), String> {
        if !valid_export_record_shape(&record)
            || record.source_project_revision != self.revision
            || !self.approval_closure_complete()
            || self
                .export_records
                .iter()
                .any(|existing| existing.export_id == record.export_id)
        {
            return Err("invalid or duplicate export record".into());
        }
        self.export_records.push(record);
        self.revision += 1;
        self.workflow_stage = "export".into();
        Ok(())
    }

    pub fn current_master_approval(&self) -> Option<&Approval> {
        let master = self.active_master.as_ref()?;
        let payload = master.approval_payload_sha256().ok()?;
        (master.approval_state == "APPROVED")
            .then_some(())
            .and_then(|_| {
                self.approval_log.iter().rev().find(|approval| {
                    approval.gate_id == "master"
                        && approval.is_valid_for(
                            &master.master_id,
                            master.candidate_revision,
                            &payload,
                        )
                })
            })
    }

    pub fn current_layer_approval(&self) -> Option<&Approval> {
        let layer_set = self.active_layer_set.as_ref()?;
        if layer_set.approval_state != "APPROVED"
            || layer_set.layers.iter().any(|layer| !layer.approved)
        {
            return None;
        }
        let payload = layer_set_approval_payload_sha256(layer_set).ok()?;
        self.approval_log.iter().rev().find(|approval| {
            approval.gate_id == "layers"
                && approval.is_valid_for(&layer_set.layer_set_id, layer_set.revision, &payload)
        })
    }

    pub fn current_rig_approval(&self) -> Option<&Approval> {
        let rig = self.active_rig.as_ref()?;
        if rig.approval_state != RigApprovalState::Approved {
            return None;
        }
        let payload = rig_approval_payload_sha256(rig).ok()?;
        self.approval_log.iter().rev().find(|approval| {
            approval.gate_id == "rig" && approval.is_valid_for(&rig.rig_id, rig.revision, &payload)
        })
    }

    pub fn current_pose_approval(&self, action_key: &str) -> Option<&Approval> {
        let animation = self.animation_set.as_ref()?;
        let motion = self.motion_content.as_ref()?;
        let clip = animation.clip(action_key).ok()?;
        let payload = animation.pose_payload(motion, action_key).ok()?;
        self.approval_log.iter().rev().find(|approval| {
            approval.gate_id == "poses"
                && approval.is_valid_for(action_key, clip.revision, &payload)
        })
    }

    pub fn current_hit_approval(&self, action_key: &str) -> Option<&Approval> {
        let animation = self.animation_set.as_ref()?;
        let clip = animation.clip(action_key).ok()?;
        let payload = animation.hit_payload(action_key).ok()?;
        self.approval_log.iter().rev().find(|approval| {
            approval.gate_id == "hits" && approval.is_valid_for(action_key, clip.revision, &payload)
        })
    }

    pub fn approval_closure_complete(&self) -> bool {
        ACTION_KEYS
            .iter()
            .all(|action| self.current_pose_approval(action).is_some())
            && ACTION_KEYS.iter().all(|action| {
                !requires_hit_frame(action) || self.current_hit_approval(action).is_some()
            })
    }

    pub fn validate_cross_aggregate(&self) -> Result<(), String> {
        if self.schema_version != "1.4.0" {
            return Err("project schema is not current".into());
        }
        let mut export_ids = std::collections::BTreeSet::new();
        if self.export_records.iter().any(|record| {
            !valid_export_record_shape(record)
                || record.source_project_revision >= self.revision
                || !export_ids.insert(record.export_id.as_str())
        }) {
            return Err("export history contains an invalid or duplicate record".into());
        }
        if self.active_master.is_some()
            && self
                .active_master
                .as_ref()
                .is_some_and(|master| master.approval_state == "APPROVED")
            && self.current_master_approval().is_none()
        {
            return Err("approved master has no current approval".into());
        }
        if let Some(layer_set) = &self.active_layer_set {
            layer_set.validate()?;
            if layer_set.approval_state == "APPROVED" && self.current_layer_approval().is_none() {
                return Err("approved LayerSet has no current approval".into());
            }
        }
        if let Some(rig) = &self.active_rig {
            rig.validate(
                self.active_layer_set
                    .as_ref()
                    .ok_or("Rig exists without LayerSet")?,
            )?;
            if rig.approval_state == RigApprovalState::Approved
                && self.current_rig_approval().is_none()
            {
                return Err("approved Rig has no current approval".into());
            }
        }
        if let Some(content) = &self.motion_content {
            content.validate(
                &self
                    .active_master
                    .as_ref()
                    .ok_or("MotionContent exists without master")?
                    .style_spec,
            )?;
            for asset in content
                .assets
                .iter()
                .filter(|asset| asset.state == crate::motion::assets::AssetState::Approved)
            {
                let binding = content
                    .key_pose_bindings
                    .iter()
                    .find(|binding| binding.asset_spec_id == asset.asset_spec_id)
                    .ok_or("approved asset binding missing")?;
                let payload = content.binding_approval_payload(binding)?;
                if !self.approval_log.iter().any(|approval| {
                    approval.gate_id == "key-pose-asset"
                        && approval.is_valid_for(&binding.binding_id, binding.revision, &payload)
                }) {
                    return Err("approved key-pose asset has no human approval".into());
                }
            }
        }
        if let Some(animation) = &self.animation_set {
            let rig = self
                .active_rig
                .as_ref()
                .ok_or("AnimationSet exists without Rig")?;
            let motion = self
                .motion_content
                .as_ref()
                .ok_or("AnimationSet exists without MotionContent")?;
            animation.validate(
                motion,
                &rig.bone_tree,
                &rig.slot_set,
                &rig.sockets
                    .iter()
                    .map(|socket| socket.socket_id.clone())
                    .collect::<Vec<_>>(),
            )?;
        }
        Ok(())
    }

    fn invalidate_exact_gate(&mut self, gate: &str, target: &str) {
        for approval in &mut self.approval_log {
            if approval.gate_id == gate && approval.target_id == target {
                approval.invalidated = true;
            }
        }
    }

    fn invalidate_action_gate(&mut self, gate: &str, action: &str) {
        self.invalidate_exact_gate(gate, action)
    }

    fn invalidate_motion_and_animation_approvals(&mut self) {
        for approval in &mut self.approval_log {
            if matches!(
                approval.gate_id.as_str(),
                "key-pose-asset" | "poses" | "hits"
            ) {
                approval.invalidated = true;
            }
        }
    }

    fn invalidate_animation_approvals(&mut self) {
        for approval in &mut self.approval_log {
            if matches!(approval.gate_id.as_str(), "poses" | "hits") {
                approval.invalidated = true;
            }
        }
    }

    fn invalidate_key_pose_approvals_not_in(&mut self, content: &MotionContent) {
        for approval in &mut self.approval_log {
            if approval.gate_id == "key-pose-asset"
                && !content
                    .key_pose_bindings
                    .iter()
                    .any(|binding| binding.binding_id == approval.target_id)
            {
                approval.invalidated = true;
            }
        }
    }

    fn invalidate_gate_and_downstream(&mut self, from_gate: &str) {
        for approval in &mut self.approval_log {
            let invalidate = match from_gate {
                "master" => true,
                "layers" => matches!(
                    approval.gate_id.as_str(),
                    "layers" | "rig" | "poses" | "hits"
                ),
                "rig" => matches!(approval.gate_id.as_str(), "rig" | "poses" | "hits"),
                "poses" => matches!(approval.gate_id.as_str(), "poses" | "hits"),
                "hits" => approval.gate_id == "hits",
                _ => false,
            };
            if invalidate {
                approval.invalidated = true;
            }
        }
    }
}

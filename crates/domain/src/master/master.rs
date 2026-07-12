use super::StyleSpec;
use crate::canonical::canonical_sha256;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterCandidate {
    pub master_id: String,
    pub source_artifact_id: String,
    pub candidate_revision: u64,
    pub source_sha256: String,
    pub style_spec: StyleSpec,
    pub approval_state: String,
    pub supersedes: Option<String>,
}
impl MasterCandidate {
    /// Canonical approval target for the complete candidate. Approval must
    /// cover identity, source, StyleSpec (including the single weapon),
    /// revision and supersession—not only the source image bytes.
    pub fn approval_payload_sha256(&self) -> Result<String, String> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct MasterApprovalPayload<'a> {
            master_id: &'a str,
            source_artifact_id: &'a str,
            candidate_revision: u64,
            source_sha256: &'a str,
            style_spec: &'a StyleSpec,
            supersedes: &'a Option<String>,
        }

        self.style_spec.validate()?;
        if self.master_id.trim().is_empty()
            || self.source_artifact_id.trim().is_empty()
            || self.source_sha256.len() != 64
            || !self
                .source_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("master approval candidate identity or source hash is invalid".into());
        }
        canonical_sha256(&MasterApprovalPayload {
            master_id: &self.master_id,
            source_artifact_id: &self.source_artifact_id,
            candidate_revision: self.candidate_revision,
            source_sha256: &self.source_sha256,
            style_spec: &self.style_spec,
            supersedes: &self.supersedes,
        })
        .map_err(|error| error.to_string())
    }

    pub fn revise(&self, id: String, style_spec: StyleSpec) -> Result<Self, String> {
        style_spec.validate()?;
        Ok(Self {
            master_id: id,
            source_artifact_id: self.source_artifact_id.clone(),
            candidate_revision: self.candidate_revision + 1,
            source_sha256: self.source_sha256.clone(),
            style_spec,
            approval_state: "PENDING".into(),
            supersedes: Some(self.master_id.clone()),
        })
    }
}

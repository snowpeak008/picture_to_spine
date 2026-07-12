use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobOutput {
    pub output_id: String,
    pub job_id: String,
    pub sha256: String,
    pub candidate_revision: Option<u64>,
    pub registered: bool,
}

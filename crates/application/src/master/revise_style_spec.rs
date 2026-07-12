use f2s_domain::master::{MasterCandidate, StyleSpec};
use uuid::Uuid;
pub fn revise(master: &MasterCandidate, spec: StyleSpec) -> Result<MasterCandidate, String> {
    master.revise(Uuid::new_v4().to_string(), spec)
}

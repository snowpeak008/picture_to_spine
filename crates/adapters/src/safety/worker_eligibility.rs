use serde::{Deserialize, Serialize};

pub const REQUIRED_CONTROLS: [&str; 5] = [
    "appcontainer-token",
    "network-capability-empty",
    "dedicated-job-root-acl",
    "job-object-kill-and-limits",
    "breakaway-denied",
];
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProbeState {
    Pass,
    Fail,
    NotRun,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxControl {
    pub id: String,
    pub state: ProbeState,
    pub evidence_sha256: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerEligibility {
    pub profile: String,
    pub controls: Vec<SandboxControl>,
    pub worker_pack_eligible: bool,
    pub reason: String,
}
pub fn evaluate_worker_eligibility(
    profile: &str,
    controls: Vec<SandboxControl>,
) -> WorkerEligibility {
    let complete = profile == "windows-appcontainer-v1"
        && REQUIRED_CONTROLS.iter().all(|required| {
            controls.iter().any(|v| {
                v.id == *required
                    && v.state == ProbeState::Pass
                    && v.evidence_sha256
                        .as_deref()
                        .map(|h| h.len() == 64)
                        .unwrap_or(false)
            })
        })
        && controls.len() == 5;
    WorkerEligibility {
        profile: profile.into(),
        controls,
        worker_pack_eligible: complete,
        reason: if complete {
            "all five native isolation controls passed".into()
        } else {
            "worker physically excluded until all five native controls have real evidence".into()
        },
    }
}

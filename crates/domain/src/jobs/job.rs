use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobState {
    Queued,
    Running,
    CancelRequested,
    Succeeded,
    Failed,
    Cancelled,
}
impl JobState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub job_id: String,
    pub kind: String,
    pub state: JobState,
    pub project_revision: u64,
    pub created_at_utc: String,
    pub terminal_sequence: Option<u64>,
}
impl Job {
    pub fn request_cancel(&mut self) {
        if !self.state.is_terminal() {
            self.state = JobState::CancelRequested
        }
    }
    pub fn accept_terminal(&mut self, state: JobState, sequence: u64) -> bool {
        if !state.is_terminal() || self.state.is_terminal() {
            return false;
        }
        self.state = state;
        self.terminal_sequence = Some(sequence);
        true
    }
}

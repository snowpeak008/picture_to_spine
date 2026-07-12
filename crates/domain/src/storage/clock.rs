use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockCheckpoint {
    pub epoch: u64,
    pub sequence: u64,
    pub observed_at_utc: String,
    pub previous_sha256: Option<String>,
}
impl ClockCheckpoint {
    pub fn can_follow(&self, previous: &Self) -> bool {
        (self.epoch == previous.epoch && self.sequence == previous.sequence + 1)
            || (self.epoch == previous.epoch + 1 && self.sequence == previous.sequence + 1)
    }
}

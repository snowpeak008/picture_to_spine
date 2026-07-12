use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Advisory = 10,
    Quality = 20,
    Approval = 30,
    Safety = 40,
    License = 50,
}
impl Priority {
    pub fn can_override(self, existing: Self) -> bool {
        self >= existing
    }
}

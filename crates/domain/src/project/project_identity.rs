use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectIdentity {
    pub project_id: Uuid,
    pub display_name: String,
}
impl ProjectIdentity {
    pub fn new(name: &str) -> Result<Self, String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 120 {
            return Err("project name must be 1..120 characters".into());
        }
        Ok(Self {
            project_id: Uuid::new_v4(),
            display_name: name.into(),
        })
    }
}

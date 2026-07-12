use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub project_id: String,
    pub display_name: String,
    pub last_opened_at_utc: String,
    pub root_reference: String,
}
pub fn upsert_recent(items: &mut Vec<RecentProject>, value: RecentProject) {
    items.retain(|v| v.project_id != value.project_id);
    items.insert(0, value);
    items.truncate(20)
}

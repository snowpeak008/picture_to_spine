use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservableEvent {
    pub code: String,
    pub severity: String,
    pub message_zh_cn: String,
    pub correlation_id: String,
}

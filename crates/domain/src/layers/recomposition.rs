use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecompositionMetrics {
    pub missing_pixels: u64,
    pub overlap_pixels: u64,
    pub changed_visible_pixels: u64,
    pub alpha_error_pixels: u64,
    pub empty_layer_masks: u64,
}
impl RecompositionMetrics {
    pub fn passes(self) -> bool {
        self.missing_pixels == 0
            && self.overlap_pixels == 0
            && self.changed_visible_pixels == 0
            && self.alpha_error_pixels == 0
            && self.empty_layer_masks == 0
    }
}

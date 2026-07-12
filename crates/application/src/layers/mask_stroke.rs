use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrokePoint {
    pub x_milli: i32,
    pub y_milli: i32,
    pub pressure_milli: u16,
    pub tick: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyMaskStroke {
    pub layer_id: String,
    pub base_mask_sha256: String,
    pub radius_milli: u32,
    pub mode: String,
    pub points: Vec<StrokePoint>,
}
impl ApplyMaskStroke {
    pub fn validate(&self) -> Result<(), String> {
        if self.points.is_empty() || self.points.len() > 100_000 {
            return Err("stroke point count out of range".into());
        }
        if self.radius_milli == 0 || self.radius_milli > 1_000_000 {
            return Err("stroke radius out of range".into());
        }
        if !matches!(self.mode.as_str(), "add" | "subtract") {
            return Err("unknown stroke mode".into());
        }
        if self.layer_id.trim().is_empty()
            || self.base_mask_sha256.len() != 64
            || !self
                .base_mask_sha256
                .bytes()
                .all(|value| value.is_ascii_hexdigit() && !value.is_ascii_uppercase())
        {
            return Err("invalid stroke target".into());
        }
        if self.points.iter().any(|point| point.pressure_milli > 1_000)
            || self
                .points
                .windows(2)
                .any(|pair| pair[0].tick > pair[1].tick)
        {
            return Err("invalid stroke pressure or tick ordering".into());
        }
        Ok(())
    }
}

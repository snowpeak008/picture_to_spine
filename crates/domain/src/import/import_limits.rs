use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportLimits {
    pub max_file_bytes: u64,
    pub max_pixels: u64,
    pub max_compression_ratio: u64,
    pub absolute_file_bytes: u64,
    pub absolute_pixels: u64,
    pub absolute_compression_ratio: u64,
}
impl Default for ImportLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 64 * 1024 * 1024,
            max_pixels: 16_777_216,
            max_compression_ratio: 200,
            absolute_file_bytes: 256 * 1024 * 1024,
            absolute_pixels: 67_108_864,
            absolute_compression_ratio: 500,
        }
    }
}
impl ImportLimits {
    pub fn validate(&self, bytes: u64, width: u32, height: u32) -> Result<(), String> {
        if bytes == 0 || bytes > self.absolute_file_bytes || bytes > self.max_file_bytes {
            return Err("image byte limit exceeded".into());
        }
        let pixels = u64::from(width) * u64::from(height);
        if pixels == 0 || pixels > self.absolute_pixels || pixels > self.max_pixels {
            return Err("image pixel limit exceeded".into());
        }
        let decoded = pixels.checked_mul(4).ok_or("decoded size overflow")?;
        let ratio = decoded.div_ceil(bytes);
        if ratio > self.absolute_compression_ratio || ratio > self.max_compression_ratio {
            return Err("image compression ratio exceeded".into());
        }
        Ok(())
    }
}

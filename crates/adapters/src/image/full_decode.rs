use super::BoundedImageInspector;
use f2s_application::{import::inspect_bounded, ports::ImageFacts};
use f2s_domain::import::ImportLimits;
use image::{GenericImageView, ImageReader, Limits};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Cursor;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecodedImageReport {
    pub media_type: String,
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub has_alpha: bool,
    pub source_sha256: String,
    pub encoded_bytes: u64,
    pub decoded_rgba_bytes: u64,
    pub compression_ratio_ceil: u64,
    pub complete_decode: bool,
}

pub fn decode_image_bounded(
    bytes: &[u8],
    limits: &ImportLimits,
) -> Result<DecodedImageReport, String> {
    let facts: ImageFacts = inspect_bounded(&BoundedImageInspector, limits, bytes)?;
    if facts.bit_depth != 8 {
        return Err("only 8-bit PNG/JPEG/WebP can enter the project".into());
    }
    let decoded_budget = limits
        .max_pixels
        .checked_mul(4)
        .and_then(|v| v.checked_add(16 * 1024 * 1024))
        .ok_or("decode budget overflow")?;
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut decode_limits = Limits::default();
    decode_limits.max_image_width = Some(facts.width);
    decode_limits.max_image_height = Some(facts.height);
    decode_limits.max_alloc = Some(decoded_budget);
    reader.limits(decode_limits);
    let image = reader
        .decode()
        .map_err(|e| format!("complete image decode failed: {e}"))?;
    if image.dimensions() != (facts.width, facts.height) {
        return Err("header and decoded dimensions disagree".into());
    }
    let decoded_rgba_bytes = u64::from(facts.width)
        .checked_mul(u64::from(facts.height))
        .and_then(|v| v.checked_mul(4))
        .ok_or("decoded byte overflow")?;
    let ratio = decoded_rgba_bytes.div_ceil(bytes.len() as u64);
    if ratio > limits.max_compression_ratio || ratio > limits.absolute_compression_ratio {
        return Err("decoded compression ratio exceeded".into());
    }
    Ok(DecodedImageReport {
        media_type: facts.media_type,
        width: facts.width,
        height: facts.height,
        bit_depth: facts.bit_depth,
        has_alpha: facts.has_alpha,
        source_sha256: format!("{:x}", Sha256::digest(bytes)),
        encoded_bytes: bytes.len() as u64,
        decoded_rgba_bytes,
        compression_ratio_ceil: ratio,
        complete_decode: true,
    })
}

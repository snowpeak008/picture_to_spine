use super::ImageFacts;
use f2s_domain::import::ImportLimits;
pub fn validate_preflight(
    limits: &ImportLimits,
    byte_length: u64,
    facts: &ImageFacts,
) -> Result<(), String> {
    if facts.bit_depth != 8 {
        return Err("only 8-bit images are accepted".into());
    }
    if !matches!(
        facts.media_type.as_str(),
        "image/png" | "image/jpeg" | "image/webp"
    ) {
        return Err("unsupported image type".into());
    }
    limits.validate(byte_length, facts.width, facts.height)
}

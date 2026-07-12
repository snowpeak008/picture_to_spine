use crate::ports::{ImageFacts, ImageInspector, validate_preflight};
use f2s_domain::import::ImportLimits;
pub fn inspect_bounded<I: ImageInspector>(
    inspector: &I,
    limits: &ImportLimits,
    bytes: &[u8],
) -> Result<ImageFacts, String> {
    if bytes.len() as u64 > limits.absolute_file_bytes {
        return Err("absolute byte limit exceeded before decode".into());
    }
    let facts = inspector.inspect(bytes)?;
    validate_preflight(limits, bytes.len() as u64, &facts)?;
    Ok(facts)
}

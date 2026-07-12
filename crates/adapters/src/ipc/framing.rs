pub const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
pub fn encode_frame(payload: &[u8]) -> Result<Vec<u8>, String> {
    if payload.len() > MAX_FRAME_BYTES {
        return Err("frame too large".into());
    }
    let mut out = Vec::with_capacity(payload.len() + 4);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    Ok(out)
}
pub fn decode_frame(frame: &[u8]) -> Result<&[u8], String> {
    if frame.len() < 4 {
        return Err("truncated frame".into());
    }
    let length = u32::from_be_bytes(frame[..4].try_into().unwrap()) as usize;
    if length > MAX_FRAME_BYTES || frame.len() != length + 4 {
        return Err("frame length mismatch".into());
    }
    Ok(&frame[4..])
}

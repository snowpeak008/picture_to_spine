use crate::ports::{CasStore, ImageFacts};
use f2s_domain::import::SourceArtifact;
use uuid::Uuid;
pub fn promote_image<C: CasStore>(
    cas: &C,
    bytes: &[u8],
    facts: &ImageFacts,
    provenance: &str,
) -> Result<SourceArtifact, String> {
    let reference = cas.put(&facts.media_type, bytes)?;
    Ok(SourceArtifact {
        artifact_id: Uuid::new_v4().to_string(),
        sha256: reference.sha256,
        media_type: facts.media_type.clone(),
        width: facts.width,
        height: facts.height,
        byte_length: bytes.len() as u64,
        bit_depth: facts.bit_depth,
        provenance: provenance.into(),
        approval_state: "UNAPPROVED".into(),
    })
}

use f2s_domain::layers::PixelProvenance;
pub fn register_completion(provenance: PixelProvenance) -> Result<PixelProvenance, String> {
    if provenance.can_enter_approved_layer() {
        return Err("AI completion must enter as unapproved candidate".into());
    }
    Ok(provenance)
}

use f2s_domain::jobs::JobOutput;
pub fn register(output: &mut JobOutput, candidate_revision: u64) -> Result<(), String> {
    if output.registered {
        return Err("output already registered".into());
    }
    output.registered = true;
    output.candidate_revision = Some(candidate_revision);
    Ok(())
}

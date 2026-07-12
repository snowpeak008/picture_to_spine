use f2s_domain::governance::Waiver;
pub fn propose(value: Waiver) -> Result<Waiver, String> {
    value.validate()?;
    Ok(value)
}

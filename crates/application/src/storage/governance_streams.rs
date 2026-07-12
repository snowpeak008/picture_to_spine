use f2s_domain::storage::GovernanceEvent;
pub fn validate_append(
    previous: Option<&GovernanceEvent>,
    next: &GovernanceEvent,
) -> Result<(), String> {
    match previous {
        None if next.sequence == 0 => Ok(()),
        Some(prev)
            if next.sequence == prev.sequence + 1 && next.previous_event_sha256.is_some() =>
        {
            Ok(())
        }
        _ => Err("governance stream sequence/hash mismatch".into()),
    }
}

use f2s_domain::governance::Invalidation;
pub fn propagate(
    source: &str,
    revision: u64,
    affected: Vec<String>,
    reason: &str,
    at: &str,
) -> Invalidation {
    Invalidation {
        source_id: source.into(),
        source_revision: revision,
        affected_ids: affected,
        reason: reason.into(),
        created_at_utc: at.into(),
    }
}

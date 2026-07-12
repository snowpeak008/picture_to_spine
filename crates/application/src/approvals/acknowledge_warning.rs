use f2s_domain::governance::WarningAck;
pub fn acknowledge(warning_id: &str, actor_id: &str, revision: u64, at: &str) -> WarningAck {
    WarningAck {
        warning_id: warning_id.into(),
        actor_id: actor_id.into(),
        target_revision: revision,
        acknowledged_at_utc: at.into(),
    }
}

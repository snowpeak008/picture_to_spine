use f2s_domain::observability::AuditEvent;
pub trait AuditSink {
    fn append(&self, event: &AuditEvent) -> Result<(), String>;
}

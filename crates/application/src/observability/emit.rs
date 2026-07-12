use crate::ports::{AuditSink, EventSink};
use f2s_domain::observability::{AuditEvent, ObservableEvent};
pub fn emit_both<A: AuditSink, E: EventSink>(
    audit: &A,
    events: &E,
    audit_event: &AuditEvent,
    event: &ObservableEvent,
) -> Result<(), String> {
    audit.append(audit_event)?;
    events.emit(event)
}

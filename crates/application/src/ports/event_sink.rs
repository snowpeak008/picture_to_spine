use f2s_domain::observability::ObservableEvent;
pub trait EventSink {
    fn emit(&self, event: &ObservableEvent) -> Result<(), String>;
}

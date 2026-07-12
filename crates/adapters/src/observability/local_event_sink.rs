use f2s_application::ports::EventSink;
use f2s_domain::observability::ObservableEvent;
use std::sync::Mutex;
#[derive(Default)]
pub struct LocalEventSink {
    events: Mutex<Vec<ObservableEvent>>,
}
impl EventSink for LocalEventSink {
    fn emit(&self, event: &ObservableEvent) -> Result<(), String> {
        self.events
            .lock()
            .map_err(|_| "event lock poisoned".to_owned())?
            .push(event.clone());
        Ok(())
    }
}
impl LocalEventSink {
    pub fn len(&self) -> usize {
        self.events.lock().map(|v| v.len()).unwrap_or_default()
    }
}

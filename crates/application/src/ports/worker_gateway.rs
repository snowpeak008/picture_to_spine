use serde_json::Value;
pub trait WorkerGateway {
    fn submit(&self, job_id: &str, payload: &Value) -> Result<(), String>;
    fn cancel(&self, job_id: &str) -> Result<(), String>;
}

use f2s_application::ports::{IpcPort, IpcRequest, IpcResponse};
use serde_json::json;
#[derive(Default)]
pub struct MockIpc {
    pub fail: bool,
    pub truncate: bool,
}
impl IpcPort for MockIpc {
    fn request(&self, request: &IpcRequest) -> IpcResponse {
        if self.fail || self.truncate {
            return IpcResponse::failure(
                &request.request_id,
                "F2S-IPC-MOCK",
                "mock transport failure",
                true,
            );
        }
        IpcResponse::success(
            &request.request_id,
            json!({"method":request.method,"payload":request.payload,"mock":true}),
        )
    }
}

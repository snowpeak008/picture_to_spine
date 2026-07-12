use f2s_application::ports::IpcRequest;
pub const MAX_WEBVIEW_MESSAGE_BYTES: usize = 1024 * 1024;
pub fn decode_webview_request(raw: &str) -> Result<IpcRequest, String> {
    if raw.len() > MAX_WEBVIEW_MESSAGE_BYTES {
        return Err("WebView message exceeds 1 MiB".into());
    }
    let request: IpcRequest =
        serde_json::from_str(raw).map_err(|e| format!("invalid IPC JSON: {e}"))?;
    request.validate()?;
    Ok(request)
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod diagnostics_export;
mod ipc_host;
mod local_security;
mod spine_cli_host;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use f2s_adapters::ipc::decode_webview_request;
use f2s_application::ports::IpcResponse;
use std::{
    cell::RefCell, mem, os::windows::ffi::OsStrExt, path::PathBuf, rc::Rc, sync::mpsc, thread,
    time::Duration,
};
use uuid::Uuid;
use webview2_com::{Microsoft::Web::WebView2::Win32::*, *};
use windows::Win32::UI::WindowsAndMessaging::WNDCLASSW;
use windows::{
    Win32::{
        Foundation::{E_POINTER, HINSTANCE, HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM},
        Graphics::Gdi,
        System::{Com::*, LibraryLoader},
        UI::{HiDpi, Input::KeyboardAndMouse, WindowsAndMessaging},
    },
    core::*,
};

type AppResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const BUILD_INPUT_SHA256: &str = match option_env!("F2S_BUILD_INPUT_SHA256") {
    Some(value) => value,
    None => "UNBOUND_DEV_BUILD",
};
const WINDOW_TITLE_READY: &str = "FlashToSpine Production Assist";
const WINDOW_TITLE_STARTING: &str = "FlashToSpine 正在验证界面…";
const NAVIGATE_TO_STRING_URI_PREFIX: &str = "data:text/html;charset=utf-8;base64,";
const DOM_READY_SCRIPT: &str = "(() => document.querySelector('.app-shell') !== null && document.getElementById('root')?.childElementCount > 0 ? 'F2S_DOM_READY' : 'F2S_DOM_PENDING')()";

fn main() {
    let args = std::env::args_os().collect::<Vec<_>>();
    if args.get(1).is_some_and(|v| v == "--smoke") {
        let output = args.get(2).map(std::path::PathBuf::from);
        std::process::exit(run_packaged_smoke(output.as_deref()));
    }
    if args.get(1).is_some_and(|v| v == "--image-probe") {
        let code = args
            .get(2)
            .map(std::path::PathBuf::from)
            .map_or(2, |path| ipc_host::run_image_probe(&path));
        std::process::exit(code);
    }
    if let Err(error) = run() {
        eprintln!("F2S-BOOT-001: {error}");
        let message_text = format!("FlashToSpine 启动失败\n\nF2S-BOOT-001: {error}");
        let message = CoTaskMemPWSTR::from(message_text.as_str());
        unsafe {
            let _ = WindowsAndMessaging::MessageBoxW(
                None,
                *message.as_ref().as_pcwstr(),
                w!("FlashToSpine"),
                WindowsAndMessaging::MB_OK | WindowsAndMessaging::MB_ICONERROR,
            );
        }
        std::process::exit(1);
    }
}

fn run_packaged_smoke(output: Option<&std::path::Path>) -> i32 {
    let html = embedded_ui("0123456789abcdef0123456789abcdef");
    let build_bound = BUILD_INPUT_SHA256.len() == 64
        && BUILD_INPUT_SHA256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'));
    let passed = build_bound
        && html.contains("FlashToSpine Production Assist")
        && html.contains("spineTarget:'4.2.43'")
        && html.contains("networkAllowed:false")
        && html.contains("ipc:'WIRED'");
    let report = serde_json::json!({
        "schemaVersion": "1.0.0",
        "status": if passed { "PASS" } else { "FAIL" },
        "product": "FlashToSpine",
        "packageKind": "windows-portable-core-internal",
        "buildInputSha256": BUILD_INPUT_SHA256,
        "uiEmbedded": passed,
        "networkAllowed": false,
        "webView2Runtime": "NOT_PROBED_SYSTEM_PREREQUISITE",
        "spineEditor": "EXTERNAL_NOT_INCLUDED",
        "appContainerWorker": "NOT_INCLUDED_UNVERIFIED",
        "signature": "NOT_RUN_EXTERNAL"
    });
    let bytes = match serde_json::to_vec_pretty(&report) {
        Ok(mut bytes) => {
            bytes.push(b'\n');
            bytes
        }
        Err(_) => return 2,
    };
    if let Some(path) = output {
        if std::fs::write(path, bytes).is_err() {
            return 3;
        }
    }
    if passed { 0 } else { 1 }
}

fn run() -> AppResult<()> {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
    unsafe { HiDpi::SetProcessDpiAwareness(HiDpi::PROCESS_PER_MONITOR_DPI_AWARE)? };
    let webview = WebView::create()?;
    webview.set_title(WINDOW_TITLE_STARTING)?;
    webview.set_size(1440, 900)?;
    let nonce = Uuid::new_v4().simple().to_string();
    let html = embedded_ui(&nonce);
    webview.install_security_and_ipc(trusted_navigate_to_string_uri(&html))?;
    webview.navigate_to_string_and_verify(&html)?;
    webview.set_title(WINDOW_TITLE_READY)?;
    webview.run()
}

fn embedded_ui(nonce: &str) -> String {
    let css = include_str!("../../../desktop-ui/dist/app.css");
    let javascript =
        include_str!("../../../desktop-ui/dist/app.js").replace("</script", "<\\/script");
    format!(
        r#"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'nonce-{nonce}'; script-src 'nonce-{nonce}'; connect-src 'none'; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'"><title>FlashToSpine Production Assist</title><style nonce="{nonce}">{css}</style></head><body><div id="root"></div><script nonce="{nonce}">window.__F2S_BOOTSTRAP__={{schemaVersion:'1.0.0',productMode:'CORE_IMPLEMENTED_EXTERNALS_PENDING',spineTarget:'4.2.43',workerState:'UNVERIFIED_EXCLUDED',networkAllowed:false,ipc:'WIRED'}};{javascript}</script></body></html>"#
    )
}

fn trusted_navigate_to_string_uri(html: &str) -> String {
    format!(
        "{NAVIGATE_TO_STRING_URI_PREFIX}{}",
        BASE64_STANDARD.encode(html.as_bytes())
    )
}

fn navigation_uri_is_allowed(uri: &str, trusted_document_uri: &str) -> bool {
    uri == "about:blank" || uri == trusted_document_uri
}

#[derive(Clone)]
struct FrameWindow {
    window: Rc<HWND>,
    size: Rc<RefCell<SIZE>>,
}
impl FrameWindow {
    fn new() -> AppResult<Self> {
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            lpszClassName: w!("FlashToSpineWebView"),
            ..Default::default()
        };
        let window = unsafe {
            WindowsAndMessaging::RegisterClassW(&class);
            WindowsAndMessaging::CreateWindowExW(
                Default::default(),
                w!("FlashToSpineWebView"),
                w!("FlashToSpine Production Assist"),
                WindowsAndMessaging::WS_OVERLAPPEDWINDOW,
                WindowsAndMessaging::CW_USEDEFAULT,
                WindowsAndMessaging::CW_USEDEFAULT,
                1440,
                900,
                None,
                None,
                LibraryLoader::GetModuleHandleW(None)
                    .ok()
                    .map(|h| HINSTANCE(h.0)),
                None,
            )?
        };
        Ok(Self {
            window: Rc::new(window),
            size: Rc::new(RefCell::new(SIZE::default())),
        })
    }
}

struct Controller(ICoreWebView2Controller);
impl Drop for Controller {
    fn drop(&mut self) {
        let _ = unsafe { self.0.Close() };
    }
}

#[derive(Clone)]
struct WebView {
    controller: Rc<Controller>,
    webview: Rc<ICoreWebView2>,
    frame: FrameWindow,
}
impl WebView {
    fn create() -> AppResult<Self> {
        let frame = FrameWindow::new()?;
        let parent = *frame.window;
        let user_data_folder = webview_user_data_folder()?;
        let environment = {
            let (tx, rx) = mpsc::channel();
            let user_data_folder_wide = user_data_folder
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect::<Vec<_>>();
            CreateCoreWebView2EnvironmentCompletedHandler::wait_for_async_operation(
                Box::new(move |handler| unsafe {
                    CreateCoreWebView2EnvironmentWithOptions(
                        PCWSTR::null(),
                        PCWSTR(user_data_folder_wide.as_ptr()),
                        None::<&ICoreWebView2EnvironmentOptions>,
                        &handler,
                    )
                    .map_err(webview2_com::Error::WindowsError)
                }),
                Box::new(move |code, environment| {
                    code?;
                    tx.send(environment.ok_or_else(|| windows::core::Error::from(E_POINTER)))
                        .expect("environment channel");
                    Ok(())
                }),
            )?;
            rx.recv()??
        };
        let controller = {
            let (tx, rx) = mpsc::channel();
            CreateCoreWebView2ControllerCompletedHandler::wait_for_async_operation(
                Box::new(move |handler| unsafe {
                    environment
                        .CreateCoreWebView2Controller(parent, &handler)
                        .map_err(webview2_com::Error::WindowsError)
                }),
                Box::new(move |code, controller| {
                    code?;
                    tx.send(controller.ok_or_else(|| windows::core::Error::from(E_POINTER)))
                        .expect("controller channel");
                    Ok(())
                }),
            )?;
            rx.recv()??
        };
        let size = get_window_size(parent);
        unsafe {
            controller.SetBounds(RECT {
                left: 0,
                top: 0,
                right: size.cx,
                bottom: size.cy,
            })?;
            controller.SetIsVisible(true)?
        };
        *frame.size.borrow_mut() = size;
        let webview = unsafe { controller.CoreWebView2()? };
        unsafe {
            let settings = webview.Settings()?;
            settings.SetAreDefaultContextMenusEnabled(false)?;
            settings.SetAreDevToolsEnabled(cfg!(debug_assertions))?;
            settings.SetIsStatusBarEnabled(false)?;
        }
        let result = Self {
            controller: Rc::new(Controller(controller)),
            webview: Rc::new(webview),
            frame,
        };
        Self::set_window_webview(parent, Some(Box::new(result.clone())));
        Ok(result)
    }
    fn set_title(&self, title: &str) -> AppResult<()> {
        let value = CoTaskMemPWSTR::from(title);
        unsafe {
            WindowsAndMessaging::SetWindowTextW(*self.frame.window, *value.as_ref().as_pcwstr())?
        };
        Ok(())
    }
    fn set_size(&self, width: i32, height: i32) -> AppResult<()> {
        *self.frame.size.borrow_mut() = SIZE {
            cx: width,
            cy: height,
        };
        unsafe {
            WindowsAndMessaging::SetWindowPos(
                *self.frame.window,
                None,
                0,
                0,
                width,
                height,
                WindowsAndMessaging::SWP_NOACTIVATE
                    | WindowsAndMessaging::SWP_NOZORDER
                    | WindowsAndMessaging::SWP_NOMOVE,
            )?
        };
        Ok(())
    }
    fn navigate_to_string_and_verify(&self, html: &str) -> AppResult<()> {
        let (tx, rx) = mpsc::channel();
        let handler = NavigationCompletedEventHandler::create(Box::new(move |_sender, args| {
            let args = args.ok_or_else(|| windows::core::Error::from(E_POINTER))?;
            let mut success = BOOL::default();
            unsafe { args.IsSuccess(&mut success)? };
            tx.send(success.as_bool())
                .expect("navigation result channel");
            Ok(())
        }));
        let mut token = 0;
        unsafe { self.webview.add_NavigationCompleted(&handler, &mut token)? };
        let html_value = CoTaskMemPWSTR::from(html);
        let navigation_result = unsafe {
            self.webview
                .NavigateToString(*html_value.as_ref().as_pcwstr())?;
            webview2_com::wait_with_pump(rx)
        };
        unsafe { self.webview.remove_NavigationCompleted(token)? };
        if !navigation_result? {
            return Err("F2S-BOOT-NAVIGATION: embedded UI navigation failed".into());
        }

        for _ in 0..20 {
            let result = self.execute_script_json(DOM_READY_SCRIPT)?;
            if serde_json::from_str::<String>(&result).ok().as_deref() == Some("F2S_DOM_READY") {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err("F2S-BOOT-DOM: embedded UI did not render the application root".into())
    }
    fn execute_script_json(&self, script: &str) -> AppResult<String> {
        let output = Rc::new(RefCell::new(None));
        let output_for_callback = output.clone();
        let webview = self.webview.clone();
        let script = script.to_owned();
        ExecuteScriptCompletedHandler::wait_for_async_operation(
            Box::new(move |handler| unsafe {
                let script = CoTaskMemPWSTR::from(script.as_str());
                webview
                    .ExecuteScript(*script.as_ref().as_pcwstr(), &handler)
                    .map_err(webview2_com::Error::WindowsError)
            }),
            Box::new(move |error_code, result| {
                error_code?;
                *output_for_callback.borrow_mut() = Some(result);
                Ok(())
            }),
        )?;
        let result = output
            .borrow_mut()
            .take()
            .ok_or("F2S-BOOT-DOM: WebView2 returned no script result")?;
        Ok(result)
    }
    fn install_security_and_ipc(&self, trusted_document_uri: String) -> AppResult<()> {
        let host = Rc::new(ipc_host::HostState::default());
        let hwnd = *self.frame.window;
        unsafe {
            let mut token = 0;
            let host_for_message = host.clone();
            self.webview.add_WebMessageReceived(
                &WebMessageReceivedEventHandler::create(Box::new(move |sender, args| {
                    let response = if let Some(args) = args {
                        let mut message = PWSTR::null();
                        match args.WebMessageAsJson(&mut message) {
                            Ok(()) => {
                                let message = CoTaskMemPWSTR::from(message).to_string();
                                match decode_webview_request(&message) {
                                    Ok(request) => host_for_message.handle(request, hwnd),
                                    Err(error) => IpcResponse::failure("invalid", "F2S-IPC-ENVELOPE", error, false),
                                }
                            }
                            Err(error) => IpcResponse::failure("invalid", "F2S-IPC-READ", error.to_string(), false),
                        }
                    } else {
                        IpcResponse::failure("invalid", "F2S-IPC-ARGS", "missing WebMessage args", false)
                    };
                    if let Some(sender) = sender {
                        let json = serde_json::to_string(&response).unwrap_or_else(|_| "{\"schemaVersion\":\"1.0.0\",\"requestId\":\"invalid\",\"ok\":false,\"result\":null,\"error\":{\"code\":\"F2S-IPC-SERIALIZE\",\"message\":\"response serialization failed\",\"retryable\":false}}".into());
                        let json = CoTaskMemPWSTR::from(json.as_str());
                        sender.PostWebMessageAsJson(*json.as_ref().as_pcwstr())?;
                    }
                    Ok(())
                })),
                &mut token,
            )?;
            let mut navigation_token = 0;
            self.webview.add_NavigationStarting(
                &NavigationStartingEventHandler::create(Box::new(move |_sender, args| {
                    if let Some(args) = args {
                        let mut uri = PWSTR::null();
                        args.Uri(&mut uri)?;
                        let uri = CoTaskMemPWSTR::from(uri).to_string();
                        if !navigation_uri_is_allowed(&uri, &trusted_document_uri) {
                            args.SetCancel(true)?;
                        }
                    }
                    Ok(())
                })),
                &mut navigation_token,
            )?;
            let mut new_window_token = 0;
            self.webview.add_NewWindowRequested(
                &NewWindowRequestedEventHandler::create(Box::new(move |_sender, args| {
                    if let Some(args) = args {
                        args.SetHandled(true)?;
                    }
                    Ok(())
                })),
                &mut new_window_token,
            )?;
            let mut permission_token = 0;
            self.webview.add_PermissionRequested(
                &PermissionRequestedEventHandler::create(Box::new(move |_sender, args| {
                    if let Some(args) = args {
                        args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY)?;
                    }
                    Ok(())
                })),
                &mut permission_token,
            )?;
            let webview4: ICoreWebView2_4 = self.webview.cast()?;
            let mut download_token = 0;
            webview4.add_DownloadStarting(
                &DownloadStartingEventHandler::create(Box::new(move |_sender, args| {
                    if let Some(args) = args {
                        args.SetCancel(true)?;
                    }
                    Ok(())
                })),
                &mut download_token,
            )?;
        }
        Ok(())
    }
    fn run(self) -> AppResult<()> {
        unsafe {
            let _ =
                WindowsAndMessaging::ShowWindow(*self.frame.window, WindowsAndMessaging::SW_SHOW);
            let _ = Gdi::UpdateWindow(*self.frame.window);
            let _ = KeyboardAndMouse::SetFocus(Some(*self.frame.window));
        }
        let mut msg = WindowsAndMessaging::MSG::default();
        loop {
            let value = unsafe { WindowsAndMessaging::GetMessageW(&mut msg, None, 0, 0).0 };
            match value {
                -1 => return Err(windows::core::Error::from_thread().into()),
                0 => return Ok(()),
                _ => unsafe {
                    let _ = WindowsAndMessaging::TranslateMessage(&msg);
                    WindowsAndMessaging::DispatchMessageW(&msg);
                },
            }
        }
    }
    fn set_window_webview(hwnd: HWND, webview: Option<Box<WebView>>) -> Option<Box<WebView>> {
        unsafe {
            match set_window_long(
                hwnd,
                match webview {
                    Some(value) => Box::into_raw(value) as _,
                    None => 0,
                },
            ) {
                0 => None,
                pointer => Some(Box::from_raw(pointer as *mut _)),
            }
        }
    }
    fn get_window_webview(hwnd: HWND) -> Option<Box<WebView>> {
        unsafe {
            let data = get_window_long(hwnd);
            if data == 0 {
                return None;
            }
            let raw = Box::from_raw(data as *mut WebView);
            let value = raw.clone();
            mem::forget(raw);
            Some(value)
        }
    }
}

fn webview_user_data_folder() -> AppResult<PathBuf> {
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .ok_or("LOCALAPPDATA is unavailable for the WebView2 user data folder")?;
    let folder = PathBuf::from(local_app_data)
        .join("FlashToSpine")
        .join("WebView2");
    std::fs::create_dir_all(&folder)?;
    Ok(folder)
}

extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let Some(webview) = WebView::get_window_webview(hwnd) else {
        return unsafe { WindowsAndMessaging::DefWindowProcW(hwnd, message, wparam, lparam) };
    };
    match message {
        WindowsAndMessaging::WM_SIZE => {
            let size = get_window_size(hwnd);
            unsafe {
                let _ = webview.controller.0.SetBounds(RECT {
                    left: 0,
                    top: 0,
                    right: size.cx,
                    bottom: size.cy,
                });
            }
            *webview.frame.size.borrow_mut() = size;
            LRESULT::default()
        }
        WindowsAndMessaging::WM_CLOSE => {
            unsafe {
                let _ = WindowsAndMessaging::DestroyWindow(hwnd);
            }
            LRESULT::default()
        }
        WindowsAndMessaging::WM_DESTROY => {
            WebView::set_window_webview(hwnd, None);
            unsafe { WindowsAndMessaging::PostQuitMessage(0) };
            LRESULT::default()
        }
        _ => unsafe { WindowsAndMessaging::DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}
fn get_window_size(hwnd: HWND) -> SIZE {
    let mut rect = RECT::default();
    unsafe {
        let _ = WindowsAndMessaging::GetClientRect(hwnd, &mut rect);
    }
    SIZE {
        cx: rect.right - rect.left,
        cy: rect.bottom - rect.top,
    }
}
#[cfg(target_pointer_width = "64")]
unsafe fn set_window_long(hwnd: HWND, value: isize) -> isize {
    unsafe {
        WindowsAndMessaging::SetWindowLongPtrW(hwnd, WindowsAndMessaging::GWLP_USERDATA, value)
    }
}
#[cfg(target_pointer_width = "64")]
unsafe fn get_window_long(hwnd: HWND) -> isize {
    unsafe { WindowsAndMessaging::GetWindowLongPtrW(hwnd, WindowsAndMessaging::GWLP_USERDATA) }
}
#[cfg(target_pointer_width = "32")]
unsafe fn set_window_long(hwnd: HWND, value: isize) -> isize {
    unsafe {
        WindowsAndMessaging::SetWindowLongW(hwnd, WindowsAndMessaging::GWLP_USERDATA, value as i32)
            as isize
    }
}
#[cfg(target_pointer_width = "32")]
unsafe fn get_window_long(hwnd: HWND) -> isize {
    unsafe {
        WindowsAndMessaging::GetWindowLongW(hwnd, WindowsAndMessaging::GWLP_USERDATA) as isize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_navigation_policy_allows_only_the_exact_trusted_document() {
        let html = embedded_ui("0123456789abcdef0123456789abcdef");
        let trusted_uri = trusted_navigate_to_string_uri(&html);
        let encoded = trusted_uri
            .strip_prefix(NAVIGATE_TO_STRING_URI_PREFIX)
            .expect("trusted NavigateToString URI prefix");
        assert_eq!(BASE64_STANDARD.decode(encoded).unwrap(), html.as_bytes());
        assert!(navigation_uri_is_allowed("about:blank", &trusted_uri));
        assert!(navigation_uri_is_allowed(&trusted_uri, &trusted_uri));

        let mut tampered = trusted_uri.clone();
        tampered.push('A');
        for rejected in [
            "data:text/html;charset=utf-8;base64,PGgxPm90aGVyPC9oMT4=",
            "data:text/html,<h1>other</h1>",
            "https://example.invalid/",
            tampered.as_str(),
        ] {
            assert!(!navigation_uri_is_allowed(rejected, &trusted_uri));
        }
    }

    #[test]
    fn dom_ready_probe_requires_the_real_application_shell() {
        assert!(DOM_READY_SCRIPT.contains(".app-shell"));
        assert!(DOM_READY_SCRIPT.contains("childElementCount > 0"));
        assert!(!DOM_READY_SCRIPT.contains("document.body !== null"));
    }
}

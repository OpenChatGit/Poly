use super::*;

mod native;
pub(super) use self::native::run_native_app as run_native_app_impl;

pub(super) fn open_url_window(url: &str, title: &str, width: u32, height: u32) {
    #[cfg(feature = "native")]
    {
        // Hide console window on Windows
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Console::{GetConsoleWindow, FreeConsole};
            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
            unsafe {
                let console = GetConsoleWindow();
                if !console.0.is_null() {
                    let _ = ShowWindow(console, SW_HIDE);
                    FreeConsole().ok();
                }
            }
        }

        let config = poly::NativeConfig::new(title)
            .with_size(width, height)
            .with_decorations(true)
            .with_dev_tools(true);

        if let Err(e) = poly::run_native_url(url, config) {
            eprintln!("{}error{}: Failed to open URL window: {}", RED, RESET, e);
        }
    }

    #[cfg(not(feature = "native"))]
    {
        eprintln!("{}error{}: Native feature not enabled", RED, RESET);
        let _ = (url, title, width, height); // Suppress unused warnings
    }
}

/// Run browser mode with separate UI and content WebViews
pub(super) fn run_browser_mode(url: &str, title: &str, width: u32, height: u32, ui_height: u32, ui_html_path: Option<String>) {
    #[cfg(feature = "native")]
    {
        // Hide console window on Windows
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Console::{GetConsoleWindow, FreeConsole};
            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
            unsafe {
                let console = GetConsoleWindow();
                if !console.0.is_null() {
                    let _ = ShowWindow(console, SW_HIDE);
                    FreeConsole().ok();
                }
            }
        }

        // Load UI HTML from file or use default
        let ui_html = if let Some(path) = ui_html_path {
            std::fs::read_to_string(&path).unwrap_or_else(|_| default_browser_ui())
        } else {
            default_browser_ui()
        };

        let config = poly::BrowserConfig {
            title: title.to_string(),
            width,
            height,
            ui_height,
            ui_html,
            start_url: url.to_string(),
            devtools: true,
            icon_path: None,
            decorations: false,
        };

        if let Err(e) = poly::run_browser_window(config) {
            eprintln!("{}error{}: Failed to run browser mode: {}", RED, RESET, e);
        }
    }

    #[cfg(not(feature = "native"))]
    {
        eprintln!("{}error{}: Native feature not enabled", RED, RESET);
        let _ = (url, title, width, height, ui_height, ui_html_path);
    }
}

/// Default browser UI HTML
#[allow(dead_code)]
fn default_browser_ui() -> String {
    // Minimal default - apps should provide their own UI via --ui-html
    r#"<!DOCTYPE html>
<html>
<head>
<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
body { background: #1a1a1f; height: 100%; display: flex; align-items: center; justify-content: center; -webkit-app-region: drag; }
.msg { color: #666; font-family: system-ui; font-size: 12px; }
</style>
</head>
<body><div class="msg">No UI provided. Use --ui-html to specify custom UI.</div></body>
</html>"#.to_string()
}

#[allow(unused_variables)]
fn respond_cef_frame_request(request: tiny_http::Request) {
    let query = request.url().split_once('?').map(|(_, query)| query).unwrap_or("").to_string();
    let query_value = |name: &str| query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then_some(value)
    });
    let id = query_value("id").and_then(|value| urlencoding::decode(value).ok()).unwrap_or_default();
    let since = query_value("since").and_then(|value| value.parse::<u64>().ok()).unwrap_or(0);
    // Long-poll quietly when CEF has no new paint yet. This avoids a 60 Hz
    // requestAnimationFrame loop for static pages while returning fresh frames
    // as soon as CEF produces them.
    let wait_ms = query_value("wait_ms").and_then(|value| value.parse::<u64>().ok()).unwrap_or(100);
    use std::io::Cursor;
    let response = match poly::cef_osr::read_frame_after_wait(&id, since, wait_ms) {
        Ok(Some(frame)) => {
            let headers = vec![
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/octet-stream"[..]).unwrap(),
                tiny_http::Header::from_bytes(&b"Cache-Control"[..], &b"no-store"[..]).unwrap(),
                tiny_http::Header::from_bytes(&b"X-Poly-Cef-Width"[..], frame.width.to_string().as_bytes()).unwrap(),
                tiny_http::Header::from_bytes(&b"X-Poly-Cef-Height"[..], frame.height.to_string().as_bytes()).unwrap(),
                tiny_http::Header::from_bytes(&b"X-Poly-Cef-Sequence"[..], frame.sequence.to_string().as_bytes()).unwrap(),
                tiny_http::Header::from_bytes(&b"X-Poly-Cef-Dirty-X"[..], frame.dirty_x.to_string().as_bytes()).unwrap(),
                tiny_http::Header::from_bytes(&b"X-Poly-Cef-Dirty-Y"[..], frame.dirty_y.to_string().as_bytes()).unwrap(),
                tiny_http::Header::from_bytes(&b"X-Poly-Cef-Dirty-Width"[..], frame.dirty_width.to_string().as_bytes()).unwrap(),
                tiny_http::Header::from_bytes(&b"X-Poly-Cef-Dirty-Height"[..], frame.dirty_height.to_string().as_bytes()).unwrap(),
            ];
            let length = frame.rgba.len();
            tiny_http::Response::new(tiny_http::StatusCode(200), headers, Cursor::new(frame.rgba), Some(length), None).boxed()
        }
        Ok(None) => tiny_http::Response::from_data(Vec::<u8>::new()).with_status_code(304).boxed(),
        Err(error) => tiny_http::Response::from_string(error).with_status_code(404).boxed(),
    };
    let _ = request.respond(response);
}

pub(super) fn run_app_result(path: &str, release: bool, native: bool, browser: bool, ui_height: u32) -> Result<(), String> {
    let project_path = Path::new(path);

    // If native mode, try to run in a WebView window
    if native {
        run_native_app_impl(project_path, release, browser, ui_height);
        return Ok(());
    }

    let _ = (release, browser, ui_height);
    Err("`poly run` launches desktop apps with `--native`; choose an HTML/JavaScript project entry point.".into())
}

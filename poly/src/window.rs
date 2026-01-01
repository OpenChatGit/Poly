//! Multi-Window Support for Poly
//! Create and manage multiple windows

use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};

#[cfg(feature = "native")]
use once_cell::sync::Lazy;

#[cfg(feature = "native")]
use std::sync::mpsc::{channel, Sender};

/// Window ID counter
#[cfg(feature = "native")]
static WINDOW_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Window configuration
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub url: Option<String>,
    pub html: Option<String>,
    pub resizable: bool,
    pub decorations: bool,
    pub always_on_top: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Poly Window".to_string(),
            width: 800,
            height: 600,
            url: None,
            html: None,
            resizable: true,
            decorations: false,
            always_on_top: false,
        }
    }
}

impl WindowConfig {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            ..Default::default()
        }
    }
    
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
    
    pub fn with_url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }
    
    pub fn with_html(mut self, html: &str) -> Self {
        self.html = Some(html.to_string());
        self
    }
}

/// Window handle
#[derive(Debug, Clone)]
pub struct WindowHandle {
    pub id: u64,
}

/// Commands to send to windows
#[cfg(feature = "native")]
#[derive(Debug)]
pub enum WindowCommand {
    Close,
    Minimize,
    Maximize,
    Show,
    Hide,
}

/// Window info stored in registry
#[cfg(feature = "native")]
struct WindowInfo {
    #[allow(dead_code)]
    title: String,
    sender: Sender<WindowCommand>,
}

/// Global window registry
#[cfg(feature = "native")]
pub static WINDOW_REGISTRY: Lazy<Arc<Mutex<HashMap<u64, WindowInfo>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});

/// Create a new window
#[cfg(feature = "native")]
pub fn create_window(config: WindowConfig) -> Result<WindowHandle, String> {
    use std::thread;
    
    let id = WINDOW_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    let title = config.title.clone();
    
    // Create channel for commands
    let (tx, rx) = channel::<WindowCommand>();
    
    // Register window
    {
        let mut registry = WINDOW_REGISTRY.lock().map_err(|e| format!("Lock error: {}", e))?;
        registry.insert(id, WindowInfo { title, sender: tx });
    }
    
    // Spawn window in new thread
    thread::spawn(move || {
        if let Err(e) = run_window(id, config, rx) {
            eprintln!("Window {} error: {}", id, e);
        }
        
        // Unregister on close
        if let Ok(mut registry) = WINDOW_REGISTRY.lock() {
            registry.remove(&id);
        }
    });
    
    Ok(WindowHandle { id })
}

/// Close a window by ID
#[cfg(feature = "native")]
pub fn close_window(id: u64) -> Result<(), String> {
    let registry = WINDOW_REGISTRY.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    if let Some(info) = registry.get(&id) {
        info.sender.send(WindowCommand::Close).map_err(|e| format!("Send error: {}", e))?;
        Ok(())
    } else {
        Err(format!("Window {} not found", id))
    }
}

/// Close all windows
#[cfg(feature = "native")]
pub fn close_all_windows() -> Result<(), String> {
    let registry = WINDOW_REGISTRY.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    for (_, info) in registry.iter() {
        let _ = info.sender.send(WindowCommand::Close);
    }
    Ok(())
}

/// Get window count
#[cfg(feature = "native")]
pub fn window_count() -> usize {
    WINDOW_REGISTRY.lock().map(|r| r.len()).unwrap_or(0)
}

/// List all window IDs
#[cfg(feature = "native")]
pub fn list_windows() -> Vec<u64> {
    WINDOW_REGISTRY.lock()
        .map(|r| r.keys().cloned().collect())
        .unwrap_or_default()
}

#[cfg(feature = "native")]
fn run_window(id: u64, config: WindowConfig, rx: std::sync::mpsc::Receiver<WindowCommand>) -> Result<(), String> {
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoopBuilder},
        window::WindowBuilder,
        platform::windows::EventLoopBuilderExtWindows,
    };
    use wry::WebViewBuilder;
    
    let event_loop = EventLoopBuilder::new()
        .with_any_thread(true)
        .build();
    
    let window = WindowBuilder::new()
        .with_title(&config.title)
        .with_inner_size(tao::dpi::LogicalSize::new(config.width, config.height))
        .with_resizable(config.resizable)
        .with_decorations(config.decorations)
        .with_always_on_top(config.always_on_top)
        .build(&event_loop)
        .map_err(|e| format!("Window build error: {}", e))?;
    
    let window = Arc::new(window);
    let window_clone = Arc::clone(&window);
    let window_for_cmd = Arc::clone(&window);
    
    // Set background color to prevent flickering/shadow pulsing
    let bg_color = (26, 26, 26, 255); // #1a1a1a - dark gray
    
    // Build webview
    let mut builder = WebViewBuilder::new()
        .with_devtools(true)
        .with_background_color(bg_color)
        .with_ipc_handler(move |req: wry::http::Request<String>| {
            let body = req.body();
            match body.as_str() {
                "minimize" => window_clone.set_minimized(true),
                "maximize" => {
                    if window_clone.is_maximized() {
                        window_clone.set_maximized(false);
                    } else {
                        window_clone.set_maximized(true);
                    }
                }
                "close" => {
                    window_clone.set_visible(false);
                }
                cmd if cmd.starts_with("drag") => {
                    let _ = window_clone.drag_window();
                }
                _ => {}
            }
        });
    
    // Set content - inject IPC bridge for window controls
    let ipc_script = r#"<script>
window.ipc = {
  postMessage: function(msg) {
    if (window.__TAURI_IPC__) window.__TAURI_IPC__(msg);
    else if (window.chrome && window.chrome.webview) window.chrome.webview.postMessage(msg);
    else if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.ipc) window.webkit.messageHandlers.ipc.postMessage(msg);
  }
};
window.poly = window.poly || {};
window.poly.window = {
  minimize: function() { window.ipc.postMessage('minimize'); },
  maximize: function() { window.ipc.postMessage('maximize'); },
  close: function() { window.ipc.postMessage('close'); },
  drag: function() { window.ipc.postMessage('drag'); }
};
</script>"#;

    if let Some(url) = &config.url {
        builder = builder.with_url(url);
    } else if let Some(html) = &config.html {
        // Inject IPC script into user's HTML
        let mut html_with_ipc = html.clone();
        if html_with_ipc.contains("</head>") {
            html_with_ipc = html_with_ipc.replace("</head>", &format!("{}</head>", ipc_script));
        } else if html_with_ipc.contains("<body") {
            html_with_ipc = html_with_ipc.replace("<body", &format!("{}<body", ipc_script));
        } else {
            html_with_ipc = format!("{}{}", ipc_script, html_with_ipc);
        }
        builder = builder.with_html(&html_with_ipc);
    } else {
        // Minimal default - user should provide their own HTML
        builder = builder.with_html(&format!(r#"<!DOCTYPE html>
<html>
<head><title>{}</title>{}</head>
<body style="margin:0;background:#1a1a1f;color:#fff;font-family:system-ui;padding:20px;">
<p>Window {} - Provide your own HTML via the 'html' option</p>
</body>
</html>"#, config.title, ipc_script, id));
    }
    
    let _webview = builder.build(&*window)
        .map_err(|e| format!("WebView build error: {}", e))?;
    
    // Proxy for event loop
    let proxy = event_loop.create_proxy();
    
    // Thread to receive commands
    std::thread::spawn(move || {
        while let Ok(cmd) = rx.recv() {
            match cmd {
                WindowCommand::Close => {
                    window_for_cmd.set_visible(false);
                    let _ = proxy.send_event(());
                    break;
                }
                WindowCommand::Minimize => window_for_cmd.set_minimized(true),
                WindowCommand::Maximize => {
                    if window_for_cmd.is_maximized() {
                        window_for_cmd.set_maximized(false);
                    } else {
                        window_for_cmd.set_maximized(true);
                    }
                }
                WindowCommand::Show => {
                    window_for_cmd.set_visible(true);
                    window_for_cmd.set_focus();
                }
                WindowCommand::Hide => window_for_cmd.set_visible(false),
            }
        }
    });
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(()) => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

// Stubs for non-native
#[cfg(not(feature = "native"))]
pub fn create_window(_config: WindowConfig) -> Result<WindowHandle, String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn close_window(_id: u64) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn close_all_windows() -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn window_count() -> usize {
    0
}

#[cfg(not(feature = "native"))]
pub fn list_windows() -> Vec<u64> {
    vec![]
}

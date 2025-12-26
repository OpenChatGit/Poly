//! Multi-Window Support for Poly
//! Create and manage multiple windows

use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};

#[cfg(feature = "native")]
use once_cell::sync::Lazy;

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
    pub parent_id: Option<u64>,
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
            decorations: false, // Frameless by default for custom titlebar
            always_on_top: false,
            parent_id: None,
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

/// Window handle for controlling a window
#[derive(Debug, Clone)]
pub struct WindowHandle {
    pub id: u64,
}

/// Global window registry
#[cfg(feature = "native")]
pub static WINDOW_REGISTRY: Lazy<Arc<Mutex<WindowRegistry>>> = Lazy::new(|| {
    Arc::new(Mutex::new(WindowRegistry::new()))
});

/// Registry to track all windows
#[cfg(feature = "native")]
pub struct WindowRegistry {
    windows: HashMap<u64, WindowInfo>,
}

#[cfg(feature = "native")]
struct WindowInfo {
    #[allow(dead_code)]
    title: String,
    #[allow(dead_code)]
    visible: bool,
}

#[cfg(feature = "native")]
impl WindowRegistry {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
        }
    }
    
    pub fn register(&mut self, id: u64, title: String) {
        self.windows.insert(id, WindowInfo { title, visible: true });
    }
    
    pub fn unregister(&mut self, id: u64) {
        self.windows.remove(&id);
    }
    
    pub fn count(&self) -> usize {
        self.windows.len()
    }
}

/// Create a new window (returns window ID)
/// Note: This creates a window in a separate thread
#[cfg(feature = "native")]
pub fn create_window(config: WindowConfig) -> Result<WindowHandle, String> {
    use std::thread;
    
    let id = WINDOW_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    let title = config.title.clone();
    
    // Register window
    {
        let mut registry = WINDOW_REGISTRY.lock().map_err(|e| format!("Lock error: {}", e))?;
        registry.register(id, title.clone());
    }
    
    // Spawn window in new thread
    thread::spawn(move || {
        if let Err(e) = run_window(id, config) {
            eprintln!("Window {} error: {}", id, e);
        }
        
        // Unregister on close
        if let Ok(mut registry) = WINDOW_REGISTRY.lock() {
            registry.unregister(id);
        }
    });
    
    Ok(WindowHandle { id })
}

#[cfg(feature = "native")]
fn run_window(id: u64, config: WindowConfig) -> Result<(), String> {
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    };
    use wry::WebViewBuilder;
    
    let event_loop = EventLoop::new();
    
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
    
    // Build webview
    let mut builder = WebViewBuilder::new()
        .with_devtools(true)
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
                "close" => std::process::exit(0),
                cmd if cmd.starts_with("drag") => {
                    let _ = window_clone.drag_window();
                }
                _ => {}
            }
        });
    
    // Set content
    if let Some(url) = &config.url {
        builder = builder.with_url(url);
    } else if let Some(html) = &config.html {
        builder = builder.with_html(html);
    } else {
        builder = builder.with_html(&format!(r#"
            <!DOCTYPE html>
            <html>
            <head><title>{}</title></head>
            <body style="margin:0;background:#1a1a1f;color:#fff;font-family:system-ui;">
                <div style="padding:20px;">
                    <h1>Window {}</h1>
                    <p>This is a Poly window.</p>
                </div>
            </body>
            </html>
        "#, config.title, id));
    }
    
    let _webview = builder.build(&*window)
        .map_err(|e| format!("WebView build error: {}", e))?;
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        
        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            *control_flow = ControlFlow::Exit;
        }
    });
}

/// Get window count
#[cfg(feature = "native")]
pub fn window_count() -> usize {
    WINDOW_REGISTRY.lock().map(|r| r.count()).unwrap_or(0)
}

// Stubs for non-native
#[cfg(not(feature = "native"))]
pub fn create_window(_config: WindowConfig) -> Result<WindowHandle, String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn window_count() -> usize {
    0
}

//! Multi-Window Support for Poly
//! Create and manage multiple windows with full control

#[cfg(feature = "native")]
use std::collections::HashMap;
#[cfg(feature = "native")]
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};

#[cfg(feature = "native")]
use once_cell::sync::Lazy;

#[cfg(feature = "native")]
use std::sync::mpsc::{channel, Sender};

/// Window ID counter
#[cfg(feature = "native")]
static WINDOW_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Window position
#[derive(Debug, Clone, Copy)]
pub enum WindowPosition {
    /// Center on screen
    Center,
    /// Specific coordinates
    At(i32, i32),
    /// Center on parent window
    CenterOnParent(u64),
}

impl Default for WindowPosition {
    fn default() -> Self {
        WindowPosition::Center
    }
}

/// Window configuration
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub url: Option<String>,
    pub html: Option<String>,
    pub resizable: bool,
    pub decorations: bool,
    pub always_on_top: bool,
    pub transparent: bool,
    pub fullscreen: bool,
    pub maximized: bool,
    pub visible: bool,
    pub focused: bool,
    pub icon_path: Option<String>,
    pub background_color: (u8, u8, u8, u8),
    pub position: WindowPosition,
    pub parent_id: Option<u64>,
    pub modal: bool,
    pub devtools: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Poly Window".to_string(),
            width: 800,
            height: 600,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            url: None,
            html: None,
            resizable: true,
            decorations: false,
            always_on_top: false,
            transparent: false,
            fullscreen: false,
            maximized: false,
            visible: true,
            focused: true,
            icon_path: None,
            background_color: (26, 26, 26, 255),
            position: WindowPosition::Center,
            parent_id: None,
            modal: false,
            devtools: false,
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
    
    pub fn with_min_size(mut self, width: u32, height: u32) -> Self {
        self.min_width = Some(width);
        self.min_height = Some(height);
        self
    }
    
    pub fn with_max_size(mut self, width: u32, height: u32) -> Self {
        self.max_width = Some(width);
        self.max_height = Some(height);
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
    
    pub fn with_icon(mut self, path: &str) -> Self {
        self.icon_path = Some(path.to_string());
        self
    }
    
    pub fn with_position(mut self, x: i32, y: i32) -> Self {
        self.position = WindowPosition::At(x, y);
        self
    }
    
    pub fn centered(mut self) -> Self {
        self.position = WindowPosition::Center;
        self
    }
    
    pub fn with_parent(mut self, parent_id: u64) -> Self {
        self.parent_id = Some(parent_id);
        self.position = WindowPosition::CenterOnParent(parent_id);
        self
    }
    
    pub fn modal(mut self) -> Self {
        self.modal = true;
        self
    }
    
    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }
    
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
    
    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }
    
    pub fn with_always_on_top(mut self, always_on_top: bool) -> Self {
        self.always_on_top = always_on_top;
        self
    }
    
    pub fn with_background_color(mut self, r: u8, g: u8, b: u8, a: u8) -> Self {
        self.background_color = (r, g, b, a);
        self
    }
    
    pub fn with_devtools(mut self, enabled: bool) -> Self {
        self.devtools = enabled;
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
    Restore,
    Show,
    Hide,
    Focus,
    SetTitle(String),
    SetSize(u32, u32),
    SetMinSize(Option<u32>, Option<u32>),
    SetMaxSize(Option<u32>, Option<u32>),
    SetPosition(i32, i32),
    SetResizable(bool),
    SetAlwaysOnTop(bool),
    SetFullscreen(bool),
    Navigate(String),
    LoadHtml(String),
    EvalScript(String),
}

/// Window state info
#[derive(Debug, Clone)]
pub struct WindowState {
    pub id: u64,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub is_visible: bool,
    pub is_minimized: bool,
    pub is_maximized: bool,
    pub is_fullscreen: bool,
    pub is_focused: bool,
}

/// Window info stored in registry
#[cfg(feature = "native")]
pub struct WindowInfo {
    pub title: String,
    pub sender: Sender<WindowCommand>,
    pub state: Arc<Mutex<WindowState>>,
}

/// Global window registry
#[cfg(feature = "native")]
pub static WINDOW_REGISTRY: Lazy<Arc<Mutex<HashMap<u64, WindowInfo>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});


// ============================================
// Window API Functions
// ============================================

/// Create a new window
#[cfg(feature = "native")]
pub fn create_window(config: WindowConfig) -> Result<WindowHandle, String> {
    use std::thread;
    
    let id = WINDOW_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    let title = config.title.clone();
    
    // Create channel for commands
    let (tx, rx) = channel::<WindowCommand>();
    
    // Create shared state
    let state = Arc::new(Mutex::new(WindowState {
        id,
        title: title.clone(),
        width: config.width,
        height: config.height,
        x: 0,
        y: 0,
        is_visible: config.visible,
        is_minimized: false,
        is_maximized: config.maximized,
        is_fullscreen: config.fullscreen,
        is_focused: config.focused,
    }));
    
    // Register window
    {
        let mut registry = WINDOW_REGISTRY.lock().map_err(|e| format!("Lock error: {}", e))?;
        registry.insert(id, WindowInfo { 
            title, 
            sender: tx,
            state: Arc::clone(&state),
        });
    }
    
    // Spawn window in new thread
    thread::spawn(move || {
        if let Err(e) = run_window(id, config, rx, state) {
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
    send_command(id, WindowCommand::Close)
}

/// Minimize a window
#[cfg(feature = "native")]
pub fn minimize_window(id: u64) -> Result<(), String> {
    send_command(id, WindowCommand::Minimize)
}

/// Maximize a window
#[cfg(feature = "native")]
pub fn maximize_window(id: u64) -> Result<(), String> {
    send_command(id, WindowCommand::Maximize)
}

/// Restore a window from minimized/maximized state
#[cfg(feature = "native")]
pub fn restore_window(id: u64) -> Result<(), String> {
    send_command(id, WindowCommand::Restore)
}

/// Show a window
#[cfg(feature = "native")]
pub fn show_window(id: u64) -> Result<(), String> {
    send_command(id, WindowCommand::Show)
}

/// Hide a window
#[cfg(feature = "native")]
pub fn hide_window(id: u64) -> Result<(), String> {
    send_command(id, WindowCommand::Hide)
}

/// Focus a window
#[cfg(feature = "native")]
pub fn focus_window(id: u64) -> Result<(), String> {
    send_command(id, WindowCommand::Focus)
}

/// Set window title
#[cfg(feature = "native")]
pub fn set_window_title(id: u64, title: &str) -> Result<(), String> {
    send_command(id, WindowCommand::SetTitle(title.to_string()))
}

/// Set window size
#[cfg(feature = "native")]
pub fn set_window_size(id: u64, width: u32, height: u32) -> Result<(), String> {
    send_command(id, WindowCommand::SetSize(width, height))
}

/// Set window position
#[cfg(feature = "native")]
pub fn set_window_position(id: u64, x: i32, y: i32) -> Result<(), String> {
    send_command(id, WindowCommand::SetPosition(x, y))
}

/// Set window always on top
#[cfg(feature = "native")]
pub fn set_window_always_on_top(id: u64, always_on_top: bool) -> Result<(), String> {
    send_command(id, WindowCommand::SetAlwaysOnTop(always_on_top))
}

/// Set window fullscreen
#[cfg(feature = "native")]
pub fn set_window_fullscreen(id: u64, fullscreen: bool) -> Result<(), String> {
    send_command(id, WindowCommand::SetFullscreen(fullscreen))
}

/// Navigate window to URL
#[cfg(feature = "native")]
pub fn navigate_window(id: u64, url: &str) -> Result<(), String> {
    send_command(id, WindowCommand::Navigate(url.to_string()))
}

/// Load HTML in window
#[cfg(feature = "native")]
pub fn load_window_html(id: u64, html: &str) -> Result<(), String> {
    send_command(id, WindowCommand::LoadHtml(html.to_string()))
}

/// Execute JavaScript in window
#[cfg(feature = "native")]
pub fn eval_window_script(id: u64, script: &str) -> Result<(), String> {
    send_command(id, WindowCommand::EvalScript(script.to_string()))
}

/// Get window state
#[cfg(feature = "native")]
pub fn get_window_state(id: u64) -> Result<WindowState, String> {
    let registry = WINDOW_REGISTRY.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    if let Some(info) = registry.get(&id) {
        let state = info.state.lock().map_err(|e| format!("State lock error: {}", e))?;
        Ok(state.clone())
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

/// List all window states
#[cfg(feature = "native")]
pub fn list_window_states() -> Vec<WindowState> {
    let registry = match WINDOW_REGISTRY.lock() {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    
    registry.values()
        .filter_map(|info| info.state.lock().ok().map(|s| s.clone()))
        .collect()
}

/// Helper to send command to window
#[cfg(feature = "native")]
fn send_command(id: u64, cmd: WindowCommand) -> Result<(), String> {
    let registry = WINDOW_REGISTRY.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    if let Some(info) = registry.get(&id) {
        info.sender.send(cmd).map_err(|e| format!("Send error: {}", e))?;
        Ok(())
    } else {
        Err(format!("Window {} not found", id))
    }
}


// ============================================
// Native Window Implementation
// ============================================

#[cfg(feature = "native")]
fn run_window(
    id: u64, 
    config: WindowConfig, 
    rx: std::sync::mpsc::Receiver<WindowCommand>,
    state: Arc<Mutex<WindowState>>
) -> Result<(), String> {
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoopBuilder},
        window::WindowBuilder,
        dpi::{LogicalSize, LogicalPosition},
        platform::windows::EventLoopBuilderExtWindows,
    };
    use wry::WebViewBuilder;
    
    let event_loop = EventLoopBuilder::new()
        .with_any_thread(true)
        .build();
    
    let mut builder = WindowBuilder::new()
        .with_title(&config.title)
        .with_inner_size(LogicalSize::new(config.width as f64, config.height as f64))
        .with_resizable(config.resizable)
        .with_decorations(config.decorations)
        .with_always_on_top(config.always_on_top)
        .with_transparent(config.transparent)
        .with_visible(config.visible)
        .with_focused(config.focused);
    
    // Min/max size
    if let (Some(w), Some(h)) = (config.min_width, config.min_height) {
        builder = builder.with_min_inner_size(LogicalSize::new(w as f64, h as f64));
    }
    if let (Some(w), Some(h)) = (config.max_width, config.max_height) {
        builder = builder.with_max_inner_size(LogicalSize::new(w as f64, h as f64));
    }
    
    // Position
    match config.position {
        WindowPosition::At(x, y) => {
            builder = builder.with_position(LogicalPosition::new(x as f64, y as f64));
        }
        WindowPosition::Center => {
            // Will be centered by default on most platforms
        }
        WindowPosition::CenterOnParent(_parent_id) => {
            // TODO: Get parent window position and center on it
        }
    }
    
    // Fullscreen
    if config.fullscreen {
        builder = builder.with_fullscreen(Some(tao::window::Fullscreen::Borderless(None)));
    }
    
    // Maximized
    if config.maximized {
        builder = builder.with_maximized(true);
    }
    
    // Icon
    if let Some(ref icon_path) = config.icon_path {
        if let Ok(icon) = load_icon(icon_path) {
            builder = builder.with_window_icon(Some(icon));
        }
    }
    
    let window = builder.build(&event_loop)
        .map_err(|e| format!("Window build error: {}", e))?;
    
    let window = Arc::new(window);
    let window_clone = Arc::clone(&window);
    let window_for_cmd = Arc::clone(&window);
    let window_for_events = Arc::clone(&window);
    
    // Build webview
    let mut wv_builder = WebViewBuilder::new()
        .with_devtools(config.devtools)
        .with_transparent(config.transparent)
        .with_background_color(config.background_color)
        .with_ipc_handler(move |req: wry::http::Request<String>| {
            let body = req.body();
            handle_ipc_message(body, &window_clone);
        });
    
    // Set content
    let ipc_script = get_ipc_script(id);
    
    if let Some(url) = &config.url {
        wv_builder = wv_builder.with_url(url);
    } else if let Some(html) = &config.html {
        let html_with_ipc = inject_ipc_script(html, &ipc_script);
        wv_builder = wv_builder.with_html(&html_with_ipc);
    } else {
        let default_html = format!(r#"<!DOCTYPE html>
<html>
<head><title>{}</title>{}</head>
<body style="margin:0;background:#1a1a1f;color:#fff;font-family:system-ui;display:flex;align-items:center;justify-content:center;min-height:100vh;">
<p style="opacity:0.5;">Window {} ready</p>
</body>
</html>"#, config.title, ipc_script, id);
        wv_builder = wv_builder.with_html(&default_html);
    }
    
    let webview = wv_builder.build(&*window)
        .map_err(|e| format!("WebView build error: {}", e))?;
    
    // Proxy for event loop
    let proxy = event_loop.create_proxy();
    
    // Queue for WebView commands (processed in event loop)
    let webview_cmd_queue: Arc<Mutex<Vec<WindowCommand>>> = Arc::new(Mutex::new(Vec::new()));
    let webview_cmd_queue_for_thread = Arc::clone(&webview_cmd_queue);
    
    // State for command thread
    let state_for_cmd = Arc::clone(&state);
    
    // Thread to receive commands - window commands are handled here,
    // webview commands are queued for the event loop
    std::thread::spawn(move || {
        while let Ok(cmd) = rx.recv() {
            match &cmd {
                WindowCommand::Close => {
                    window_for_cmd.set_visible(false);
                    let _ = proxy.send_event(());
                    break;
                }
                WindowCommand::Minimize => {
                    window_for_cmd.set_minimized(true);
                    if let Ok(mut s) = state_for_cmd.lock() {
                        s.is_minimized = true;
                    }
                }
                WindowCommand::Maximize => {
                    let is_max = window_for_cmd.is_maximized();
                    window_for_cmd.set_maximized(!is_max);
                    if let Ok(mut s) = state_for_cmd.lock() {
                        s.is_maximized = !is_max;
                    }
                }
                WindowCommand::Restore => {
                    window_for_cmd.set_minimized(false);
                    window_for_cmd.set_maximized(false);
                    if let Ok(mut s) = state_for_cmd.lock() {
                        s.is_minimized = false;
                        s.is_maximized = false;
                    }
                }
                WindowCommand::Show => {
                    window_for_cmd.set_visible(true);
                    window_for_cmd.set_focus();
                    if let Ok(mut s) = state_for_cmd.lock() {
                        s.is_visible = true;
                    }
                }
                WindowCommand::Hide => {
                    window_for_cmd.set_visible(false);
                    if let Ok(mut s) = state_for_cmd.lock() {
                        s.is_visible = false;
                    }
                }
                WindowCommand::Focus => {
                    window_for_cmd.set_focus();
                }
                WindowCommand::SetTitle(title) => {
                    window_for_cmd.set_title(title);
                    if let Ok(mut s) = state_for_cmd.lock() {
                        s.title = title.clone();
                    }
                }
                WindowCommand::SetSize(w, h) => {
                    window_for_cmd.set_inner_size(LogicalSize::new(*w as f64, *h as f64));
                    if let Ok(mut s) = state_for_cmd.lock() {
                        s.width = *w;
                        s.height = *h;
                    }
                }
                WindowCommand::SetMinSize(w, h) => {
                    let size = match (w, h) {
                        (Some(w), Some(h)) => Some(LogicalSize::new(*w as f64, *h as f64)),
                        _ => None,
                    };
                    window_for_cmd.set_min_inner_size(size);
                }
                WindowCommand::SetMaxSize(w, h) => {
                    let size = match (w, h) {
                        (Some(w), Some(h)) => Some(LogicalSize::new(*w as f64, *h as f64)),
                        _ => None,
                    };
                    window_for_cmd.set_max_inner_size(size);
                }
                WindowCommand::SetPosition(x, y) => {
                    window_for_cmd.set_outer_position(LogicalPosition::new(*x as f64, *y as f64));
                    if let Ok(mut s) = state_for_cmd.lock() {
                        s.x = *x;
                        s.y = *y;
                    }
                }
                WindowCommand::SetResizable(resizable) => {
                    window_for_cmd.set_resizable(*resizable);
                }
                WindowCommand::SetAlwaysOnTop(on_top) => {
                    window_for_cmd.set_always_on_top(*on_top);
                }
                WindowCommand::SetFullscreen(fullscreen) => {
                    if *fullscreen {
                        window_for_cmd.set_fullscreen(Some(tao::window::Fullscreen::Borderless(None)));
                    } else {
                        window_for_cmd.set_fullscreen(None);
                    }
                    if let Ok(mut s) = state_for_cmd.lock() {
                        s.is_fullscreen = *fullscreen;
                    }
                }
                // WebView commands - queue for event loop
                WindowCommand::Navigate(_) | WindowCommand::LoadHtml(_) | WindowCommand::EvalScript(_) => {
                    if let Ok(mut queue) = webview_cmd_queue_for_thread.lock() {
                        queue.push(cmd);
                    }
                }
            }
        }
    });
    
    // State for event loop
    let state_for_events = Arc::clone(&state);
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll; // Use Poll to process queued commands
        
        // Process queued WebView commands
        if let Ok(mut queue) = webview_cmd_queue.lock() {
            for cmd in queue.drain(..) {
                match cmd {
                    WindowCommand::Navigate(url) => {
                        let _ = webview.load_url(&url);
                    }
                    WindowCommand::LoadHtml(html) => {
                        let _ = webview.load_url(&format!("data:text/html,{}", urlencoding::encode(&html)));
                    }
                    WindowCommand::EvalScript(script) => {
                        let _ = webview.evaluate_script(&script);
                    }
                    _ => {}
                }
            }
        }
        
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::Resized(size) => {
                        if let Ok(mut s) = state_for_events.lock() {
                            s.width = size.width;
                            s.height = size.height;
                        }
                    }
                    WindowEvent::Moved(pos) => {
                        if let Ok(mut s) = state_for_events.lock() {
                            s.x = pos.x;
                            s.y = pos.y;
                        }
                    }
                    WindowEvent::Focused(focused) => {
                        if let Ok(mut s) = state_for_events.lock() {
                            s.is_focused = focused;
                        }
                    }
                    _ => {}
                }
            }
            Event::UserEvent(()) => {
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                // Small sleep to prevent busy-waiting
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            _ => {}
        }
        
        // Update visibility state
        if let Ok(mut s) = state_for_events.lock() {
            s.is_visible = window_for_events.is_visible();
            s.is_minimized = window_for_events.is_minimized();
            s.is_maximized = window_for_events.is_maximized();
        }
    });
}

#[cfg(feature = "native")]
fn handle_ipc_message(message: &str, window: &tao::window::Window) {
    match message {
        "minimize" => window.set_minimized(true),
        "maximize" => {
            window.set_maximized(!window.is_maximized());
        }
        "close" => window.set_visible(false),
        "drag" => { let _ = window.drag_window(); }
        "focus" => window.set_focus(),
        "restore" => {
            window.set_minimized(false);
            window.set_maximized(false);
        }
        _ => {}
    }
}

#[cfg(feature = "native")]
fn get_ipc_script(window_id: u64) -> String {
    format!(r#"<script>
window.ipc = {{
  postMessage: function(msg) {{
    if (window.chrome && window.chrome.webview) window.chrome.webview.postMessage(msg);
    else if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.ipc) window.webkit.messageHandlers.ipc.postMessage(msg);
  }}
}};
window.poly = window.poly || {{}};
window.poly.windowId = {};
window.poly.window = {{
  minimize: function() {{ window.ipc.postMessage('minimize'); }},
  maximize: function() {{ window.ipc.postMessage('maximize'); }},
  close: function() {{ window.ipc.postMessage('close'); }},
  drag: function() {{ window.ipc.postMessage('drag'); }},
  focus: function() {{ window.ipc.postMessage('focus'); }},
  restore: function() {{ window.ipc.postMessage('restore'); }}
}};
</script>"#, window_id)
}

#[cfg(feature = "native")]
fn inject_ipc_script(html: &str, ipc_script: &str) -> String {
    if html.contains("</head>") {
        html.replace("</head>", &format!("{}</head>", ipc_script))
    } else if html.contains("<body") {
        html.replace("<body", &format!("{}<body", ipc_script))
    } else {
        format!("{}{}", ipc_script, html)
    }
}

#[cfg(feature = "native")]
fn load_icon(path: &str) -> Result<tao::window::Icon, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::BufReader;
    use image::GenericImageView;
    
    let file = File::open(path)?;
    let img = image::load(BufReader::new(file), image::ImageFormat::Png)?;
    let (w, h) = img.dimensions();
    let size = w.max(h);
    
    let mut square = image::RgbaImage::new(size, size);
    for p in square.pixels_mut() { *p = image::Rgba([0,0,0,0]); }
    
    let (xo, yo) = ((size - w) / 2, (size - h) / 2);
    for (x, y, p) in img.to_rgba8().enumerate_pixels() {
        square.put_pixel(x + xo, y + yo, *p);
    }
    
    let resized = image::imageops::resize(&square, 64, 64, image::imageops::FilterType::Lanczos3);
    Ok(tao::window::Icon::from_rgba(resized.into_raw(), 64, 64)?)
}


// ============================================
// Non-Native Stubs
// ============================================

#[cfg(not(feature = "native"))]
pub fn create_window(_config: WindowConfig) -> Result<WindowHandle, String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn close_window(_id: u64) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn minimize_window(_id: u64) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn maximize_window(_id: u64) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn restore_window(_id: u64) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn show_window(_id: u64) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn hide_window(_id: u64) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn focus_window(_id: u64) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn set_window_title(_id: u64, _title: &str) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn set_window_size(_id: u64, _width: u32, _height: u32) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn set_window_position(_id: u64, _x: i32, _y: i32) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn set_window_always_on_top(_id: u64, _always_on_top: bool) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn set_window_fullscreen(_id: u64, _fullscreen: bool) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn navigate_window(_id: u64, _url: &str) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn load_window_html(_id: u64, _html: &str) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn eval_window_script(_id: u64, _script: &str) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn get_window_state(_id: u64) -> Result<WindowState, String> {
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

#[cfg(not(feature = "native"))]
pub fn list_window_states() -> Vec<WindowState> {
    vec![]
}

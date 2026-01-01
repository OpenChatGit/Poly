//! Multi-WebView Window API
//! 
//! Allows creating a window with multiple WebViews arranged in a layout.
//! Each WebView can display different content (HTML, URL) and communicate via IPC.
//!
//! Example usage from JavaScript:
//! ```javascript
//! // Create a browser-like window with UI and content areas
//! const win = await poly.multiview.create({
//!   title: "My Browser",
//!   width: 1200,
//!   height: 800,
//!   views: [
//!     { id: "content", url: "about:blank", bounds: { x: 0, y: 80, width: 1200, height: 720 } },
//!     { id: "ui", url: "http://localhost:3000/ui.html", bounds: { x: 0, y: 0, width: 1200, height: 80 } }
//!   ]
//! });
//!
//! // Navigate a specific view
//! await poly.multiview.navigate(win.id, "content", "https://google.com");
//!
//! // Send message between views
//! await poly.multiview.postMessage(win.id, "ui", { type: "urlChanged", url: "https://google.com" });
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// View configuration
#[derive(Debug, Clone)]
pub struct ViewConfig {
    pub id: String,
    pub url: String,
    pub html: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub transparent: bool,
    pub devtools: bool,
}

/// Window configuration with multiple views
#[derive(Debug, Clone)]
pub struct MultiViewWindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub decorations: bool,
    pub resizable: bool,
    pub views: Vec<ViewConfig>,
    pub icon_path: Option<String>,
}

impl Default for MultiViewWindowConfig {
    fn default() -> Self {
        Self {
            title: "Poly MultiView".to_string(),
            width: 1024,
            height: 768,
            decorations: false,
            resizable: true,
            views: Vec::new(),
            icon_path: None,
        }
    }
}

/// Message to send between views or to backend
#[derive(Debug, Clone)]
pub struct ViewMessage {
    pub window_id: u64,
    pub from_view: String,
    pub to_view: Option<String>, // None = broadcast to all
    pub data: String,
}

/// Pending operations for multi-view windows
#[derive(Debug, Clone)]
pub enum MultiViewOperation {
    Navigate { window_id: u64, view_id: String, url: String },
    PostMessage { window_id: u64, view_id: String, message: String },
    SetBounds { window_id: u64, view_id: String, x: i32, y: i32, width: u32, height: u32 },
    Close { window_id: u64 },
}

/// Global registry of multi-view windows
pub static MULTIVIEW_WINDOWS: Lazy<Arc<Mutex<HashMap<u64, MultiViewWindowInfo>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});

/// Pending operations queue
pub static MULTIVIEW_OPS: Lazy<Arc<Mutex<Vec<MultiViewOperation>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(Vec::new()))
});

/// Window counter
static WINDOW_COUNTER: Lazy<Arc<Mutex<u64>>> = Lazy::new(|| Arc::new(Mutex::new(0)));

/// Info about a multi-view window
#[derive(Debug, Clone)]
pub struct MultiViewWindowInfo {
    pub id: u64,
    pub title: String,
    pub views: Vec<String>, // View IDs
}

/// Create a new multi-view window (queues the operation)
pub fn create_window(config: MultiViewWindowConfig) -> u64 {
    let mut counter = WINDOW_COUNTER.lock().unwrap();
    *counter += 1;
    let id = *counter;
    
    let info = MultiViewWindowInfo {
        id,
        title: config.title.clone(),
        views: config.views.iter().map(|v| v.id.clone()).collect(),
    };
    
    MULTIVIEW_WINDOWS.lock().unwrap().insert(id, info);
    
    // The actual window creation happens in native code
    // We store the config for the native code to pick up
    PENDING_CONFIGS.lock().unwrap().insert(id, config);
    
    id
}

/// Pending window configs (for native code to create)
pub static PENDING_CONFIGS: Lazy<Arc<Mutex<HashMap<u64, MultiViewWindowConfig>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});

/// Take pending configs (called by native code)
pub fn take_pending_configs() -> Vec<(u64, MultiViewWindowConfig)> {
    let mut configs = PENDING_CONFIGS.lock().unwrap();
    let result: Vec<_> = configs.drain().collect();
    result
}

/// Queue a navigate operation
pub fn navigate(window_id: u64, view_id: &str, url: &str) {
    MULTIVIEW_OPS.lock().unwrap().push(MultiViewOperation::Navigate {
        window_id,
        view_id: view_id.to_string(),
        url: url.to_string(),
    });
}

/// Queue a post message operation
pub fn post_message(window_id: u64, view_id: &str, message: &str) {
    MULTIVIEW_OPS.lock().unwrap().push(MultiViewOperation::PostMessage {
        window_id,
        view_id: view_id.to_string(),
        message: message.to_string(),
    });
}

/// Queue a set bounds operation
pub fn set_view_bounds(window_id: u64, view_id: &str, x: i32, y: i32, width: u32, height: u32) {
    MULTIVIEW_OPS.lock().unwrap().push(MultiViewOperation::SetBounds {
        window_id,
        view_id: view_id.to_string(),
        x, y, width, height,
    });
}

/// Queue a close operation
pub fn close_window(window_id: u64) {
    MULTIVIEW_OPS.lock().unwrap().push(MultiViewOperation::Close { window_id });
    MULTIVIEW_WINDOWS.lock().unwrap().remove(&window_id);
}

/// Take pending operations (called by native code)
pub fn take_operations() -> Vec<MultiViewOperation> {
    std::mem::take(&mut *MULTIVIEW_OPS.lock().unwrap())
}

/// List all windows
pub fn list_windows() -> Vec<MultiViewWindowInfo> {
    MULTIVIEW_WINDOWS.lock().unwrap().values().cloned().collect()
}

/// Get window info
pub fn get_window(id: u64) -> Option<MultiViewWindowInfo> {
    MULTIVIEW_WINDOWS.lock().unwrap().get(&id).cloned()
}

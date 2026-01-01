//! Browser API for Poly
//! Allows creating browser-like applications with real WebView windows

use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};
use once_cell::sync::Lazy;

/// Browser tab information
#[derive(Debug, Clone)]
pub struct BrowserTab {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub is_loading: bool,
}

/// Global tab registry
static TAB_COUNTER: AtomicU64 = AtomicU64::new(1);
pub static TAB_REGISTRY: Lazy<Arc<Mutex<HashMap<u64, BrowserTab>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});

/// Navigation history for tabs
pub static TAB_HISTORY: Lazy<Arc<Mutex<HashMap<u64, TabHistory>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});

/// WebView handles for real browser windows
#[cfg(feature = "native")]
pub static WEBVIEW_HANDLES: Lazy<Arc<Mutex<HashMap<u64, WebViewHandle>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});

#[cfg(feature = "native")]
pub struct WebViewHandle {
    // We store the thread handle to communicate with the WebView
    pub command_tx: std::sync::mpsc::Sender<WebViewCommand>,
}

#[cfg(feature = "native")]
pub enum WebViewCommand {
    Navigate(String),
    Back,
    Forward,
    Reload,
    Close,
}

#[derive(Debug, Clone, Default)]
pub struct TabHistory {
    pub entries: Vec<String>,
    pub current_index: i32,
}

impl TabHistory {
    pub fn new() -> Self {
        Self { entries: Vec::new(), current_index: -1 }
    }
    
    pub fn push(&mut self, url: &str) {
        if self.current_index >= 0 && (self.current_index as usize) < self.entries.len() - 1 {
            self.entries.truncate((self.current_index + 1) as usize);
        }
        self.entries.push(url.to_string());
        self.current_index = (self.entries.len() - 1) as i32;
    }
    
    pub fn can_go_back(&self) -> bool {
        self.current_index > 0
    }
    
    pub fn can_go_forward(&self) -> bool {
        self.current_index >= 0 && (self.current_index as usize) < self.entries.len() - 1
    }
    
    pub fn go_back(&mut self) -> Option<String> {
        if self.can_go_back() {
            self.current_index -= 1;
            Some(self.entries[self.current_index as usize].clone())
        } else {
            None
        }
    }
    
    pub fn go_forward(&mut self) -> Option<String> {
        if self.can_go_forward() {
            self.current_index += 1;
            Some(self.entries[self.current_index as usize].clone())
        } else {
            None
        }
    }
    
    pub fn current(&self) -> Option<String> {
        if self.current_index >= 0 && (self.current_index as usize) < self.entries.len() {
            Some(self.entries[self.current_index as usize].clone())
        } else {
            None
        }
    }
}

/// Create a new browser tab (returns tab ID)
pub fn create_tab(url: Option<&str>) -> u64 {
    let id = TAB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let url = url.unwrap_or("about:blank").to_string();
    
    let tab = BrowserTab {
        id,
        url: url.clone(),
        title: "New Tab".to_string(),
        can_go_back: false,
        can_go_forward: false,
        is_loading: false,
    };
    
    TAB_REGISTRY.lock().unwrap().insert(id, tab);
    
    let mut history = TabHistory::new();
    if url != "about:blank" {
        history.push(&url);
    }
    TAB_HISTORY.lock().unwrap().insert(id, history);
    
    id
}

/// Pending WebView window requests (to be created on main thread)
#[cfg(feature = "native")]
pub static PENDING_WEBVIEWS: Lazy<Arc<Mutex<Vec<PendingWebView>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(Vec::new()))
});

#[cfg(feature = "native")]
pub struct PendingWebView {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
}

/// Browser window mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrowserMode {
    /// Open in system's default browser (most compatible)
    SystemBrowser,
    /// Open in a new Poly WebView window (requires poly executable)
    PolyWindow,
}

/// Open a URL - either in system browser or as a new Poly window
#[cfg(feature = "native")]
pub fn open_webview(url: &str, title: &str, width: u32, height: u32) -> Result<u64, String> {
    open_webview_with_mode(url, title, width, height, BrowserMode::PolyWindow)
}

/// Open a URL with specified mode
#[cfg(feature = "native")]
pub fn open_webview_with_mode(url: &str, title: &str, width: u32, height: u32, mode: BrowserMode) -> Result<u64, String> {
    let id = TAB_COUNTER.fetch_add(1, Ordering::Relaxed);
    
    // Create tab entry for tracking
    let tab = BrowserTab {
        id,
        url: url.to_string(),
        title: title.to_string(),
        can_go_back: false,
        can_go_forward: false,
        is_loading: true,
    };
    TAB_REGISTRY.lock().unwrap().insert(id, tab);
    
    // Initialize history
    let mut history = TabHistory::new();
    history.push(url);
    TAB_HISTORY.lock().unwrap().insert(id, history);
    
    match mode {
        BrowserMode::SystemBrowser => {
            // Open in system browser - most compatible
            open_in_system_browser(url);
        }
        BrowserMode::PolyWindow => {
            // Try to open as a new Poly WebView window
            if !open_poly_window(url, title, width, height) {
                // Fallback to system browser
                open_in_system_browser(url);
            }
        }
    }
    
    // Update tab state
    if let Some(tab) = TAB_REGISTRY.lock().unwrap().get_mut(&id) {
        tab.is_loading = false;
    }
    
    Ok(id)
}

#[cfg(feature = "native")]
fn open_in_system_browser(url: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let _ = Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let _ = Command::new("open")
            .arg(url)
            .spawn();
    }
    
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let _ = Command::new("xdg-open")
            .arg(url)
            .spawn();
    }
}

#[cfg(feature = "native")]
fn open_poly_window(url: &str, title: &str, width: u32, height: u32) -> bool {
    use std::process::Command;
    
    // Find poly executable
    let poly_exe = if let Ok(exe) = std::env::current_exe() {
        exe
    } else {
        return false;
    };
    
    // Start a new poly process with the URL
    // poly open-url <url> --title <title> --width <width> --height <height>
    let result = Command::new(&poly_exe)
        .args([
            "open-url",
            url,
            "--title", title,
            "--width", &width.to_string(),
            "--height", &height.to_string(),
        ])
        .spawn();
    
    result.is_ok()
}

#[cfg(not(feature = "native"))]
pub fn open_webview(_url: &str, _title: &str, _width: u32, _height: u32) -> Result<u64, String> {
    Err("WebView requires native feature".to_string())
}

/// Navigate WebView to URL
#[cfg(feature = "native")]
pub fn webview_navigate(id: u64, url: &str) -> Result<(), String> {
    let handles = WEBVIEW_HANDLES.lock().unwrap();
    if let Some(handle) = handles.get(&id) {
        handle.command_tx.send(WebViewCommand::Navigate(url.to_string()))
            .map_err(|e| e.to_string())?;
        
        // Update tab info
        if let Some(tab) = TAB_REGISTRY.lock().unwrap().get_mut(&id) {
            tab.url = url.to_string();
        }
        
        // Update history
        if let Some(history) = TAB_HISTORY.lock().unwrap().get_mut(&id) {
            history.push(url);
        }
        
        Ok(())
    } else {
        Err(format!("WebView {} not found", id))
    }
}

#[cfg(not(feature = "native"))]
pub fn webview_navigate(_id: u64, _url: &str) -> Result<(), String> {
    Err("WebView requires native feature".to_string())
}

/// Close WebView
#[cfg(feature = "native")]
pub fn webview_close(id: u64) -> Result<(), String> {
    let mut handles = WEBVIEW_HANDLES.lock().unwrap();
    if let Some(handle) = handles.remove(&id) {
        let _ = handle.command_tx.send(WebViewCommand::Close);
        TAB_REGISTRY.lock().unwrap().remove(&id);
        TAB_HISTORY.lock().unwrap().remove(&id);
        Ok(())
    } else {
        Err(format!("WebView {} not found", id))
    }
}

#[cfg(not(feature = "native"))]
pub fn webview_close(_id: u64) -> Result<(), String> {
    Err("WebView requires native feature".to_string())
}

/// Close a browser tab
pub fn close_tab(id: u64) -> bool {
    let removed = TAB_REGISTRY.lock().unwrap().remove(&id).is_some();
    TAB_HISTORY.lock().unwrap().remove(&id);
    removed
}

/// Get tab info
pub fn get_tab(id: u64) -> Option<BrowserTab> {
    let registry = TAB_REGISTRY.lock().unwrap();
    let mut tab = registry.get(&id).cloned();
    
    if let Some(ref mut t) = tab {
        if let Some(history) = TAB_HISTORY.lock().unwrap().get(&id) {
            t.can_go_back = history.can_go_back();
            t.can_go_forward = history.can_go_forward();
        }
    }
    
    tab
}

/// List all tabs
pub fn list_tabs() -> Vec<BrowserTab> {
    let registry = TAB_REGISTRY.lock().unwrap();
    let history_map = TAB_HISTORY.lock().unwrap();
    
    registry.values().map(|t| {
        let mut tab = t.clone();
        if let Some(history) = history_map.get(&t.id) {
            tab.can_go_back = history.can_go_back();
            tab.can_go_forward = history.can_go_forward();
        }
        tab
    }).collect()
}

/// Navigate tab to URL
pub fn navigate(id: u64, url: &str) -> Result<(), String> {
    let mut registry = TAB_REGISTRY.lock().unwrap();
    let mut history_map = TAB_HISTORY.lock().unwrap();
    
    if let Some(tab) = registry.get_mut(&id) {
        tab.url = url.to_string();
        tab.is_loading = true;
        
        if let Some(history) = history_map.get_mut(&id) {
            history.push(url);
            tab.can_go_back = history.can_go_back();
            tab.can_go_forward = history.can_go_forward();
        }
        
        Ok(())
    } else {
        Err(format!("Tab {} not found", id))
    }
}

/// Go back in tab history
pub fn go_back(id: u64) -> Result<Option<String>, String> {
    let mut registry = TAB_REGISTRY.lock().unwrap();
    let mut history_map = TAB_HISTORY.lock().unwrap();
    
    if let Some(tab) = registry.get_mut(&id) {
        if let Some(history) = history_map.get_mut(&id) {
            if let Some(url) = history.go_back() {
                tab.url = url.clone();
                tab.can_go_back = history.can_go_back();
                tab.can_go_forward = history.can_go_forward();
                return Ok(Some(url));
            }
        }
        Ok(None)
    } else {
        Err(format!("Tab {} not found", id))
    }
}

/// Go forward in tab history
pub fn go_forward(id: u64) -> Result<Option<String>, String> {
    let mut registry = TAB_REGISTRY.lock().unwrap();
    let mut history_map = TAB_HISTORY.lock().unwrap();
    
    if let Some(tab) = registry.get_mut(&id) {
        if let Some(history) = history_map.get_mut(&id) {
            if let Some(url) = history.go_forward() {
                tab.url = url.clone();
                tab.can_go_back = history.can_go_back();
                tab.can_go_forward = history.can_go_forward();
                return Ok(Some(url));
            }
        }
        Ok(None)
    } else {
        Err(format!("Tab {} not found", id))
    }
}

/// Update tab title
pub fn set_tab_title(id: u64, title: &str) {
    if let Some(tab) = TAB_REGISTRY.lock().unwrap().get_mut(&id) {
        tab.title = title.to_string();
        tab.is_loading = false;
    }
}

/// Set tab loading state
pub fn set_tab_loading(id: u64, loading: bool) {
    if let Some(tab) = TAB_REGISTRY.lock().unwrap().get_mut(&id) {
        tab.is_loading = loading;
    }
}

/// Get tab history
pub fn get_history(id: u64) -> Vec<String> {
    TAB_HISTORY.lock().unwrap()
        .get(&id)
        .map(|h| h.entries.clone())
        .unwrap_or_default()
}

/// Clear tab history
pub fn clear_history(id: u64) {
    if let Some(history) = TAB_HISTORY.lock().unwrap().get_mut(&id) {
        let current = history.current();
        history.entries.clear();
        history.current_index = -1;
        if let Some(url) = current {
            history.push(&url);
        }
    }
}

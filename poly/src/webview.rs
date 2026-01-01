//! Poly WebView API
//! 
//! Provides APIs for creating and managing WebViews within a window.
//! This is the core API for building browser-like applications.
//!
//! ## JavaScript API
//! 
//! ```javascript
//! // Create a WebView
//! await poly.webview.create('content', { url: 'https://example.com', ... });
//! 
//! // Navigation
//! await poly.webview.navigate('content', 'https://google.com');
//! await poly.webview.goBack('content');
//! await poly.webview.goForward('content');
//! await poly.webview.reload('content');
//! await poly.webview.stop('content');
//! 
//! // Events (user registers callbacks)
//! poly.webview.onNavigate('content', (url) => { ... });
//! poly.webview.onTitleChange('content', (title) => { ... });
//! poly.webview.onLoadStart('content', () => { ... });
//! poly.webview.onLoadFinish('content', () => { ... });
//! poly.webview.onNewWindow('content', (url, target) => { ... });
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

// ============================================
// WebView Configuration & State
// ============================================

/// WebView bounds (position and size)
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WebViewBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// WebView creation configuration
#[derive(Debug, Clone)]
pub struct WebViewConfig {
    pub id: String,
    pub url: String,
    pub html: Option<String>,
    pub bounds: WebViewBounds,
    pub visible: bool,
    pub transparent: bool,
    pub devtools: bool,
    pub user_agent: Option<String>,
    pub zoom_level: f64,
    pub autoplay: bool,
}

impl Default for WebViewConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            url: "about:blank".to_string(),
            html: None,
            bounds: WebViewBounds::default(),
            visible: true,
            transparent: false,
            devtools: false,
            user_agent: None,
            zoom_level: 1.0,
            autoplay: true,
        }
    }
}

/// Runtime state of a WebView
#[derive(Debug, Clone)]
pub struct WebViewState {
    pub id: String,
    pub url: String,
    pub title: String,
    pub bounds: WebViewBounds,
    pub visible: bool,
    pub is_loading: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub zoom_level: f64,
}

impl Default for WebViewState {
    fn default() -> Self {
        Self {
            id: String::new(),
            url: "about:blank".to_string(),
            title: String::new(),
            bounds: WebViewBounds::default(),
            visible: true,
            is_loading: false,
            can_go_back: false,
            can_go_forward: false,
            zoom_level: 1.0,
        }
    }
}

// ============================================
// Events - User can subscribe to these
// ============================================

/// Events emitted by WebViews
#[derive(Debug, Clone)]
pub enum WebViewEvent {
    /// Navigation started to a new URL
    NavigationStarted { id: String, url: String },
    /// Navigation completed
    NavigationFinished { id: String, url: String },
    /// Page title changed
    TitleChanged { id: String, title: String },
    /// Page started loading
    LoadStarted { id: String },
    /// Page finished loading
    LoadFinished { id: String },
    /// New window requested (e.g., target="_blank")
    NewWindowRequested { id: String, url: String, target: String },
    /// Download requested
    DownloadRequested { id: String, url: String, filename: String },
    /// WebView was closed/destroyed
    Closed { id: String },
    /// Error occurred
    Error { id: String, error: String },
    /// Favicon changed
    FaviconChanged { id: String, url: String },
    /// History state changed (can_go_back, can_go_forward)
    HistoryChanged { id: String, can_go_back: bool, can_go_forward: bool },
    /// Fullscreen requested
    FullscreenRequested { id: String, enter: bool },
    /// Permission requested (camera, microphone, geolocation, etc.)
    PermissionRequested { id: String, permission: String, origin: String },
}

// ============================================
// Operations - Commands to execute
// ============================================

/// Operations to be processed by native code
#[derive(Debug, Clone)]
pub enum WebViewOperation {
    // Lifecycle
    Create(WebViewConfig),
    Destroy { id: String },
    
    // Navigation
    Navigate { id: String, url: String },
    LoadHtml { id: String, html: String },
    GoBack { id: String },
    GoForward { id: String },
    Reload { id: String },
    Stop { id: String },
    
    // Display
    SetBounds { id: String, bounds: WebViewBounds },
    SetVisible { id: String, visible: bool },
    Focus { id: String },
    
    // Content
    Eval { id: String, script: String },
    SetZoom { id: String, level: f64 },
    SetUserAgent { id: String, user_agent: String },
    
    // Main WebView (the app's own WebView)
    SetMainBounds { bounds: WebViewBounds },
    
    // Permissions
    GrantPermission { id: String, permission: String, granted: bool },
}

// ============================================
// Global State
// ============================================

/// Registry of all WebViews and their states
pub static WEBVIEW_REGISTRY: Lazy<Arc<Mutex<HashMap<String, WebViewState>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});

/// Queue of pending operations (processed by native event loop)
pub static PENDING_OPERATIONS: Lazy<Arc<Mutex<Vec<WebViewOperation>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(Vec::new()))
});

/// Queue of events to be sent to JavaScript
pub static EVENT_QUEUE: Lazy<Arc<Mutex<Vec<WebViewEvent>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(Vec::new()))
});

// ============================================
// Public API - Called from JavaScript
// ============================================

/// Create a new WebView
pub fn create(config: WebViewConfig) -> Result<(), String> {
    if config.id.is_empty() {
        return Err("WebView ID cannot be empty".to_string());
    }
    
    // Lock registry and check for duplicates
    let mut registry = WEBVIEW_REGISTRY.lock()
        .map_err(|_| "Failed to acquire registry lock")?;
    
    if registry.contains_key(&config.id) {
        return Err(format!("WebView '{}' already exists", config.id));
    }
    
    // Create initial state
    let state = WebViewState {
        id: config.id.clone(),
        url: config.url.clone(),
        title: String::new(),
        bounds: config.bounds,
        visible: config.visible,
        is_loading: true,
        can_go_back: false,
        can_go_forward: false,
        zoom_level: config.zoom_level,
    };
    
    registry.insert(config.id.clone(), state);
    drop(registry); // Release lock before acquiring next
    
    // Queue operation
    PENDING_OPERATIONS.lock()
        .map_err(|_| "Failed to acquire operations lock")?
        .push(WebViewOperation::Create(config));
    
    Ok(())
}

/// Navigate to a URL
pub fn navigate(id: &str, url: &str) -> Result<(), String> {
    let mut registry = WEBVIEW_REGISTRY.lock()
        .map_err(|_| "Failed to acquire registry lock")?;
    
    let state = registry.get_mut(id)
        .ok_or_else(|| format!("WebView '{}' not found", id))?;
    
    state.url = url.to_string();
    state.is_loading = true;
    drop(registry);
    
    PENDING_OPERATIONS.lock()
        .map_err(|_| "Failed to acquire operations lock")?
        .push(WebViewOperation::Navigate {
            id: id.to_string(),
            url: url.to_string(),
        });
    
    Ok(())
}

/// Load HTML content directly
pub fn load_html(id: &str, html: &str) -> Result<(), String> {
    let mut registry = WEBVIEW_REGISTRY.lock()
        .map_err(|_| "Failed to acquire registry lock")?;
    
    let state = registry.get_mut(id)
        .ok_or_else(|| format!("WebView '{}' not found", id))?;
    
    state.is_loading = true;
    drop(registry);
    
    PENDING_OPERATIONS.lock()
        .map_err(|_| "Failed to acquire operations lock")?
        .push(WebViewOperation::LoadHtml {
            id: id.to_string(),
            html: html.to_string(),
        });
    
    Ok(())
}

/// Go back in history
pub fn go_back(id: &str) -> Result<(), String> {
    let registry = WEBVIEW_REGISTRY.lock()
        .map_err(|_| "Failed to acquire registry lock")?;
    
    let state = registry.get(id)
        .ok_or_else(|| format!("WebView '{}' not found", id))?;
    
    if !state.can_go_back {
        return Err("Cannot go back - no history".to_string());
    }
    drop(registry);
    
    PENDING_OPERATIONS.lock()
        .map_err(|_| "Failed to acquire operations lock")?
        .push(WebViewOperation::GoBack { id: id.to_string() });
    
    Ok(())
}

/// Go forward in history
pub fn go_forward(id: &str) -> Result<(), String> {
    let registry = WEBVIEW_REGISTRY.lock()
        .map_err(|_| "Failed to acquire registry lock")?;
    
    let state = registry.get(id)
        .ok_or_else(|| format!("WebView '{}' not found", id))?;
    
    if !state.can_go_forward {
        return Err("Cannot go forward - no history".to_string());
    }
    drop(registry);
    
    PENDING_OPERATIONS.lock()
        .map_err(|_| "Failed to acquire operations lock")?
        .push(WebViewOperation::GoForward { id: id.to_string() });
    
    Ok(())
}

/// Reload the page
pub fn reload(id: &str) -> Result<(), String> {
    check_exists(id)?;
    
    PENDING_OPERATIONS.lock()
        .map_err(|_| "Failed to acquire operations lock")?
        .push(WebViewOperation::Reload { id: id.to_string() });
    
    Ok(())
}

/// Stop loading
pub fn stop(id: &str) -> Result<(), String> {
    check_exists(id)?;
    
    PENDING_OPERATIONS.lock()
        .map_err(|_| "Failed to acquire operations lock")?
        .push(WebViewOperation::Stop { id: id.to_string() });
    
    Ok(())
}

/// Set WebView bounds
pub fn set_bounds(id: &str, bounds: WebViewBounds) -> Result<(), String> {
    let mut registry = WEBVIEW_REGISTRY.lock()
        .map_err(|_| "Failed to acquire registry lock")?;
    
    let state = registry.get_mut(id)
        .ok_or_else(|| format!("WebView '{}' not found", id))?;
    
    state.bounds = bounds;
    drop(registry);
    
    PENDING_OPERATIONS.lock()
        .map_err(|_| "Failed to acquire operations lock")?
        .push(WebViewOperation::SetBounds { id: id.to_string(), bounds });
    
    Ok(())
}

/// Get WebView bounds
pub fn get_bounds(id: &str) -> Result<WebViewBounds, String> {
    let registry = WEBVIEW_REGISTRY.lock()
        .map_err(|_| "Failed to acquire registry lock")?;
    
    registry.get(id)
        .map(|s| s.bounds)
        .ok_or_else(|| format!("WebView '{}' not found", id))
}

/// Execute JavaScript in WebView
pub fn eval(id: &str, script: &str) -> Result<(), String> {
    check_exists(id)?;
    
    PENDING_OPERATIONS.lock()
        .map_err(|_| "Failed to acquire operations lock")?
        .push(WebViewOperation::Eval {
            id: id.to_string(),
            script: script.to_string(),
        });
    
    Ok(())
}

/// Destroy a WebView
pub fn destroy(id: &str) -> Result<(), String> {
    let mut registry = WEBVIEW_REGISTRY.lock()
        .map_err(|_| "Failed to acquire registry lock")?;
    
    if registry.remove(id).is_none() {
        return Err(format!("WebView '{}' not found", id));
    }
    drop(registry);
    
    PENDING_OPERATIONS.lock()
        .map_err(|_| "Failed to acquire operations lock")?
        .push(WebViewOperation::Destroy { id: id.to_string() });
    
    Ok(())
}

/// Set visibility
pub fn set_visible(id: &str, visible: bool) -> Result<(), String> {
    let mut registry = WEBVIEW_REGISTRY.lock()
        .map_err(|_| "Failed to acquire registry lock")?;
    
    let state = registry.get_mut(id)
        .ok_or_else(|| format!("WebView '{}' not found", id))?;
    
    state.visible = visible;
    drop(registry);
    
    PENDING_OPERATIONS.lock()
        .map_err(|_| "Failed to acquire operations lock")?
        .push(WebViewOperation::SetVisible { id: id.to_string(), visible });
    
    Ok(())
}

/// Focus a WebView
pub fn focus(id: &str) -> Result<(), String> {
    check_exists(id)?;
    
    PENDING_OPERATIONS.lock()
        .map_err(|_| "Failed to acquire operations lock")?
        .push(WebViewOperation::Focus { id: id.to_string() });
    
    Ok(())
}

/// Set zoom level
pub fn set_zoom(id: &str, level: f64) -> Result<(), String> {
    let mut registry = WEBVIEW_REGISTRY.lock()
        .map_err(|_| "Failed to acquire registry lock")?;
    
    let state = registry.get_mut(id)
        .ok_or_else(|| format!("WebView '{}' not found", id))?;
    
    state.zoom_level = level;
    drop(registry);
    
    PENDING_OPERATIONS.lock()
        .map_err(|_| "Failed to acquire operations lock")?
        .push(WebViewOperation::SetZoom { id: id.to_string(), level });
    
    Ok(())
}

/// Set user agent
pub fn set_user_agent(id: &str, user_agent: &str) -> Result<(), String> {
    check_exists(id)?;
    
    PENDING_OPERATIONS.lock()
        .map_err(|_| "Failed to acquire operations lock")?
        .push(WebViewOperation::SetUserAgent {
            id: id.to_string(),
            user_agent: user_agent.to_string(),
        });
    
    Ok(())
}

/// Grant or deny a permission request
pub fn respond_to_permission(id: &str, permission: &str, granted: bool) -> Result<(), String> {
    check_exists(id)?;
    
    PENDING_OPERATIONS.lock()
        .map_err(|_| "Failed to acquire operations lock")?
        .push(WebViewOperation::GrantPermission {
            id: id.to_string(),
            permission: permission.to_string(),
            granted,
        });
    
    Ok(())
}

/// List all WebViews
pub fn list() -> Vec<WebViewState> {
    WEBVIEW_REGISTRY.lock()
        .map(|r| r.values().cloned().collect())
        .unwrap_or_default()
}

/// Get a specific WebView state
pub fn get(id: &str) -> Option<WebViewState> {
    WEBVIEW_REGISTRY.lock()
        .ok()
        .and_then(|r| r.get(id).cloned())
}

/// Set main WebView bounds (the app's own WebView)
pub fn set_main_bounds(bounds: WebViewBounds) {
    if let Ok(mut ops) = PENDING_OPERATIONS.lock() {
        ops.push(WebViewOperation::SetMainBounds { bounds });
    }
}

// ============================================
// Native Code Interface
// ============================================

/// Take pending operations (called by native event loop)
pub fn take_pending_operations() -> Vec<WebViewOperation> {
    PENDING_OPERATIONS.lock()
        .map(|mut ops| std::mem::take(&mut *ops))
        .unwrap_or_default()
}

/// Check if there are pending operations
pub fn has_pending_operations() -> bool {
    PENDING_OPERATIONS.lock()
        .map(|ops| !ops.is_empty())
        .unwrap_or(false)
}

/// Take pending events (called by JavaScript polling)
pub fn take_events() -> Vec<WebViewEvent> {
    EVENT_QUEUE.lock()
        .map(|mut events| std::mem::take(&mut *events))
        .unwrap_or_default()
}

/// Push an event (called by native code when something happens)
pub fn push_event(event: WebViewEvent) {
    if let Ok(mut events) = EVENT_QUEUE.lock() {
        events.push(event);
    }
}

// ============================================
// State Updates (called by native code)
// ============================================

/// Update URL after navigation
pub fn update_url(id: &str, url: &str) {
    if let Ok(mut registry) = WEBVIEW_REGISTRY.lock() {
        if let Some(state) = registry.get_mut(id) {
            state.url = url.to_string();
        }
    }
}

/// Update title
pub fn update_title(id: &str, title: &str) {
    if let Ok(mut registry) = WEBVIEW_REGISTRY.lock() {
        if let Some(state) = registry.get_mut(id) {
            state.title = title.to_string();
        }
    }
    push_event(WebViewEvent::TitleChanged {
        id: id.to_string(),
        title: title.to_string(),
    });
}

/// Update loading state
pub fn update_loading(id: &str, is_loading: bool) {
    if let Ok(mut registry) = WEBVIEW_REGISTRY.lock() {
        if let Some(state) = registry.get_mut(id) {
            state.is_loading = is_loading;
        }
    }
    
    let event = if is_loading {
        WebViewEvent::LoadStarted { id: id.to_string() }
    } else {
        WebViewEvent::LoadFinished { id: id.to_string() }
    };
    push_event(event);
}

/// Update history state
pub fn update_history(id: &str, can_go_back: bool, can_go_forward: bool) {
    if let Ok(mut registry) = WEBVIEW_REGISTRY.lock() {
        if let Some(state) = registry.get_mut(id) {
            state.can_go_back = can_go_back;
            state.can_go_forward = can_go_forward;
        }
    }
    push_event(WebViewEvent::HistoryChanged {
        id: id.to_string(),
        can_go_back,
        can_go_forward,
    });
}

// ============================================
// Helpers
// ============================================

fn check_exists(id: &str) -> Result<(), String> {
    WEBVIEW_REGISTRY.lock()
        .map_err(|_| "Failed to acquire registry lock")?
        .get(id)
        .ok_or_else(|| format!("WebView '{}' not found", id))?;
    Ok(())
}

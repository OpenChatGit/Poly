//! Custom Titlebar API for Poly
//! Allows apps to define a custom titlebar that persists across navigation

use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// Global titlebar configuration
pub static TITLEBAR_CONFIG: Lazy<Arc<Mutex<TitlebarConfig>>> = Lazy::new(|| {
    Arc::new(Mutex::new(TitlebarConfig::default()))
});

#[derive(Debug, Clone, Default)]
pub struct TitlebarConfig {
    /// Whether custom titlebar is enabled
    pub enabled: bool,
    /// Height of the titlebar in pixels
    pub height: u32,
    /// HTML content for the titlebar
    pub html: String,
    /// CSS styles for the titlebar
    pub css: String,
    /// JavaScript for the titlebar
    pub js: String,
    /// Background color (CSS value)
    pub background: String,
}

impl TitlebarConfig {
    pub fn new() -> Self {
        Self {
            enabled: false,
            height: 40,
            html: String::new(),
            css: String::new(),
            js: String::new(),
            background: "#1a1a1f".to_string(),
        }
    }
}

/// Set titlebar configuration
pub fn set_titlebar(config: TitlebarConfig) {
    *TITLEBAR_CONFIG.lock().unwrap() = config;
}

/// Get current titlebar configuration
pub fn get_titlebar() -> TitlebarConfig {
    TITLEBAR_CONFIG.lock().unwrap().clone()
}

/// Enable/disable custom titlebar
pub fn set_enabled(enabled: bool) {
    TITLEBAR_CONFIG.lock().unwrap().enabled = enabled;
}

/// Set titlebar height
pub fn set_height(height: u32) {
    TITLEBAR_CONFIG.lock().unwrap().height = height;
}

/// Set titlebar HTML content
pub fn set_html(html: &str) {
    TITLEBAR_CONFIG.lock().unwrap().html = html.to_string();
}

/// Set titlebar CSS
pub fn set_css(css: &str) {
    TITLEBAR_CONFIG.lock().unwrap().css = css.to_string();
}

/// Set titlebar JavaScript
pub fn set_js(js: &str) {
    TITLEBAR_CONFIG.lock().unwrap().js = js.to_string();
}

/// Generate the titlebar injection script
/// This script injects the titlebar into any page
pub fn generate_injection_script() -> String {
    let config = TITLEBAR_CONFIG.lock().unwrap();
    
    if !config.enabled {
        return String::new();
    }
    
    let html_escaped = config.html.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${");
    let css_escaped = config.css.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${");
    let js_escaped = config.js.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${");
    
    format!(r#"
(function() {{
    // Don't inject if already present
    if (document.getElementById('poly-titlebar')) return;
    
    // Create titlebar container
    const titlebar = document.createElement('div');
    titlebar.id = 'poly-titlebar';
    titlebar.innerHTML = `{html}`;
    
    // Create style element
    const style = document.createElement('style');
    style.id = 'poly-titlebar-style';
    style.textContent = `
        #poly-titlebar {{
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            height: {height}px;
            background: {background};
            z-index: 999999;
            -webkit-app-region: drag;
            user-select: none;
        }}
        #poly-titlebar * {{
            -webkit-app-region: no-drag;
        }}
        #poly-titlebar button, #poly-titlebar input, #poly-titlebar a {{
            -webkit-app-region: no-drag;
        }}
        body {{
            padding-top: {height}px !important;
        }}
        {css}
    `;
    
    // Inject into page
    document.head.appendChild(style);
    document.body.insertBefore(titlebar, document.body.firstChild);
    
    // Run titlebar JavaScript
    {js}
}})();
"#,
        html = html_escaped,
        css = css_escaped,
        js = js_escaped,
        height = config.height,
        background = config.background,
    )
}

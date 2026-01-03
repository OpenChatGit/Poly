//! Poly Configuration System
//! 
//! Central configuration loaded from poly.toml
//! All values that were previously hardcoded are now configurable here.

use std::path::Path;
use std::fs;

/// Complete Poly project configuration from poly.toml
#[derive(Debug, Clone)]
pub struct PolyConfig {
    pub package: PackageConfig,
    pub web: WebConfig,
    pub window: WindowConfig,
    pub dev: DevConfig,
    pub network: NetworkConfig,
    pub app: AppConfig,
    pub tray: TrayConfig,
    pub build: BuildConfig,
    pub browser: BrowserConfig,
}

/// [package] section
#[derive(Debug, Clone)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
}

/// [web] section
#[derive(Debug, Clone)]
pub struct WebConfig {
    pub dir: String,
}

/// [window] section - all window-related settings
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: Option<String>,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub background_color: String,
    pub transparent: bool,
    pub decorations: bool,
    pub always_on_top: bool,
    pub fullscreen: bool,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub default_popup_width: u32,
    pub default_popup_height: u32,
    pub icon_path: Option<String>,
}

/// [dev] section - development server settings
#[derive(Debug, Clone)]
pub struct DevConfig {
    pub port: u16,
    pub devtools: bool,
    pub reload_interval: u32,
    pub inject_alpine: bool,
    pub inject_lucide: bool,
}

/// [network] section - HTTP client settings
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub timeout: u32,
    pub user_agent: Option<String>,
    pub max_body_size: u64,
}

/// [app] section - application behavior
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub notification_timeout: u32,
}

/// [tray] section - system tray settings
#[derive(Debug, Clone)]
pub struct TrayConfig {
    pub enabled: bool,
    pub tooltip: Option<String>,
    pub icon_path: Option<String>,
    pub icon_size: u32,
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
}

/// [build] section - build settings
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub icon_size: u32,
    pub icon_path: Option<String>,
}

/// [browser] section - browser mode settings
#[derive(Debug, Clone)]
pub struct BrowserConfig {
    pub enabled: bool,
    pub ui_height: u32,
    pub width: u32,
    pub height: u32,
}

/// Predefined window configuration
#[derive(Debug, Clone)]
pub struct PredefinedWindow {
    pub id: String,
    pub title: Option<String>,
    pub width: u32,
    pub height: u32,
    pub url: Option<String>,
    pub resizable: bool,
    pub decorations: bool,
    pub always_on_top: bool,
    pub transparent: bool,
}

impl Default for PredefinedWindow {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            title: None,
            width: 800,
            height: 600,
            url: None,
            resizable: true,
            decorations: false,
            always_on_top: false,
            transparent: false,
        }
    }
}

impl Default for PolyConfig {
    fn default() -> Self {
        Self {
            package: PackageConfig::default(),
            web: WebConfig::default(),
            window: WindowConfig::default(),
            dev: DevConfig::default(),
            network: NetworkConfig::default(),
            app: AppConfig::default(),
            tray: TrayConfig::default(),
            build: BuildConfig::default(),
            browser: BrowserConfig::default(),
        }
    }
}

impl Default for PackageConfig {
    fn default() -> Self {
        Self {
            name: "Poly App".to_string(),
            version: "0.1.0".to_string(),
        }
    }
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            dir: "web".to_string(),
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: None,
            width: 1024,
            height: 768,
            resizable: true,
            background_color: "#1a1a1a".to_string(),
            transparent: false,
            decorations: true,
            always_on_top: false,
            fullscreen: false,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            default_popup_width: 800,
            default_popup_height: 600,
            icon_path: None,
        }
    }
}

impl Default for DevConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            devtools: false,
            reload_interval: 2000,
            inject_alpine: false,
            inject_lucide: false,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            timeout: 30,
            user_agent: None,
            max_body_size: 50_000_000,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            notification_timeout: 5000,
        }
    }
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            tooltip: None,
            icon_path: None,
            icon_size: 32,
            minimize_to_tray: false,
            close_to_tray: false,
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            icon_size: 64,
            icon_path: None,
        }
    }
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ui_height: 80,
            width: 1200,
            height: 800,
        }
    }
}

impl PolyConfig {
    /// Load configuration from poly.toml file
    pub fn load(path: &Path) -> Self {
        let toml_path = if path.is_file() && path.file_name().map(|n| n == "poly.toml").unwrap_or(false) {
            path.to_path_buf()
        } else {
            path.join("poly.toml")
        };
        
        if !toml_path.exists() {
            return Self::default();
        }
        
        let content = match fs::read_to_string(&toml_path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        
        Self::parse(&content)
    }
    
    /// Parse configuration from TOML content
    pub fn parse(content: &str) -> Self {
        let mut config = Self::default();
        let mut current_section = String::new();
        
        for line in content.lines() {
            let line = line.trim();
            
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Section header
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len()-1].to_string();
                continue;
            }
            
            // Key = value
            if let Some((key, value)) = parse_key_value(line) {
                match current_section.as_str() {
                    "package" => config.parse_package(&key, &value),
                    "web" => config.parse_web(&key, &value),
                    "window" => config.parse_window(&key, &value),
                    "dev" => config.parse_dev(&key, &value),
                    "network" => config.parse_network(&key, &value),
                    "app" => config.parse_app(&key, &value),
                    "tray" => config.parse_tray(&key, &value),
                    "build" => config.parse_build(&key, &value),
                    "browser" => config.parse_browser(&key, &value),
                    _ => {}
                }
            }
        }
        
        config
    }
    
    fn parse_package(&mut self, key: &str, value: &str) {
        match key {
            "name" => self.package.name = value.to_string(),
            "version" => self.package.version = value.to_string(),
            _ => {}
        }
    }
    
    fn parse_web(&mut self, key: &str, value: &str) {
        match key {
            "dir" => self.web.dir = value.to_string(),
            _ => {}
        }
    }
    
    fn parse_window(&mut self, key: &str, value: &str) {
        match key {
            "title" => self.window.title = Some(value.to_string()),
            "width" => self.window.width = value.parse().unwrap_or(1024),
            "height" => self.window.height = value.parse().unwrap_or(768),
            "resizable" => self.window.resizable = value == "true",
            "background_color" => self.window.background_color = value.to_string(),
            "transparent" => self.window.transparent = value == "true",
            "decorations" => self.window.decorations = value == "true",
            "always_on_top" => self.window.always_on_top = value == "true",
            "fullscreen" => self.window.fullscreen = value == "true",
            "min_width" => self.window.min_width = value.parse().ok(),
            "min_height" => self.window.min_height = value.parse().ok(),
            "max_width" => self.window.max_width = value.parse().ok(),
            "max_height" => self.window.max_height = value.parse().ok(),
            "default_popup_width" => self.window.default_popup_width = value.parse().unwrap_or(800),
            "default_popup_height" => self.window.default_popup_height = value.parse().unwrap_or(600),
            "icon" | "icon_path" => self.window.icon_path = Some(value.to_string()),
            _ => {}
        }
    }
    
    fn parse_dev(&mut self, key: &str, value: &str) {
        match key {
            "port" => self.dev.port = value.parse().unwrap_or(3000),
            "devtools" => self.dev.devtools = value == "true",
            "reload_interval" => self.dev.reload_interval = value.parse().unwrap_or(2000),
            "inject_alpine" => self.dev.inject_alpine = value == "true",
            "inject_lucide" => self.dev.inject_lucide = value == "true",
            _ => {}
        }
    }
    
    fn parse_network(&mut self, key: &str, value: &str) {
        match key {
            "timeout" => self.network.timeout = value.parse().unwrap_or(30),
            "user_agent" => self.network.user_agent = Some(value.to_string()),
            "max_body_size" => self.network.max_body_size = value.parse().unwrap_or(50_000_000),
            _ => {}
        }
    }
    
    fn parse_app(&mut self, key: &str, value: &str) {
        match key {
            "notification_timeout" => self.app.notification_timeout = value.parse().unwrap_or(5000),
            _ => {}
        }
    }
    
    fn parse_tray(&mut self, key: &str, value: &str) {
        match key {
            "enabled" => self.tray.enabled = value == "true",
            "tooltip" => self.tray.tooltip = Some(value.to_string()),
            "icon" | "icon_path" => self.tray.icon_path = Some(value.to_string()),
            "icon_size" => self.tray.icon_size = value.parse().unwrap_or(32),
            "minimize_to_tray" => self.tray.minimize_to_tray = value == "true",
            "close_to_tray" => self.tray.close_to_tray = value == "true",
            _ => {}
        }
    }
    
    fn parse_build(&mut self, key: &str, value: &str) {
        match key {
            "icon_size" => self.build.icon_size = value.parse().unwrap_or(64),
            "icon" | "icon_path" => self.build.icon_path = Some(value.to_string()),
            _ => {}
        }
    }
    
    fn parse_browser(&mut self, key: &str, value: &str) {
        self.browser.enabled = true;
        match key {
            "ui_height" => self.browser.ui_height = value.parse().unwrap_or(80),
            "width" => self.browser.width = value.parse().unwrap_or(1200),
            "height" => self.browser.height = value.parse().unwrap_or(800),
            _ => {}
        }
    }
    
    /// Get the effective window title
    pub fn get_title(&self) -> &str {
        self.window.title.as_deref().unwrap_or(&self.package.name)
    }
    
    /// Get the effective tray tooltip
    pub fn get_tray_tooltip(&self) -> &str {
        self.tray.tooltip.as_deref().unwrap_or(&self.package.name)
    }
    
    /// Parse background color from hex string to RGBA tuple
    pub fn get_background_rgba(&self) -> (u8, u8, u8, u8) {
        parse_hex_color(&self.window.background_color)
            .unwrap_or((26, 26, 26, 255))
    }
    
    /// Get user agent string
    pub fn get_user_agent(&self) -> String {
        self.network.user_agent.clone().unwrap_or_else(|| {
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string()
        })
    }
}

/// Parse a key = value line
fn parse_key_value(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }
    
    let key = parts[0].trim().to_string();
    let mut value = parts[1].trim().to_string();
    
    // Remove quotes from string values
    if (value.starts_with('"') && value.ends_with('"')) ||
       (value.starts_with('\'') && value.ends_with('\'')) {
        value = value[1..value.len()-1].to_string();
    }
    
    Some((key, value))
}

/// Parse hex color string to RGBA
fn parse_hex_color(color: &str) -> Option<(u8, u8, u8, u8)> {
    let color = color.trim_start_matches('#');
    
    match color.len() {
        6 => {
            let r = u8::from_str_radix(&color[0..2], 16).ok()?;
            let g = u8::from_str_radix(&color[2..4], 16).ok()?;
            let b = u8::from_str_radix(&color[4..6], 16).ok()?;
            Some((r, g, b, 255))
        }
        8 => {
            let r = u8::from_str_radix(&color[0..2], 16).ok()?;
            let g = u8::from_str_radix(&color[2..4], 16).ok()?;
            let b = u8::from_str_radix(&color[4..6], 16).ok()?;
            let a = u8::from_str_radix(&color[6..8], 16).ok()?;
            Some((r, g, b, a))
        }
        _ => None,
    }
}

/// Global config instance for easy access
use std::sync::OnceLock;
static GLOBAL_CONFIG: OnceLock<PolyConfig> = OnceLock::new();

/// Initialize global config from path
pub fn init_global_config(path: &Path) {
    let _ = GLOBAL_CONFIG.set(PolyConfig::load(path));
}

/// Get global config (returns default if not initialized)
pub fn get_config() -> &'static PolyConfig {
    GLOBAL_CONFIG.get_or_init(PolyConfig::default)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_config() {
        let toml = "[package]\nname = \"TestApp\"\nversion = \"1.0.0\"\n\n[window]\ntitle = \"My Test App\"\nwidth = 1280\nheight = 720\n\n[dev]\nport = 4000\ndevtools = true\n\n[network]\ntimeout = 60";
        
        let config = PolyConfig::parse(toml);
        
        assert_eq!(config.package.name, "TestApp");
        assert_eq!(config.window.title, Some("My Test App".to_string()));
        assert_eq!(config.window.width, 1280);
        assert_eq!(config.window.height, 720);
        assert_eq!(config.dev.port, 4000);
        assert_eq!(config.dev.devtools, true);
        assert_eq!(config.network.timeout, 60);
    }
    
    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#1a1a1a"), Some((26, 26, 26, 255)));
        assert_eq!(parse_hex_color("#ffffff"), Some((255, 255, 255, 255)));
        assert_eq!(parse_hex_color("#00000080"), Some((0, 0, 0, 128)));
    }
}

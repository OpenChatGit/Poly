//! SovereigntyEngine - Permission enforcement system
//! 
//! Ensures apps can only use APIs they have declared in poly.toml.
//! Protects end-users from malicious or privacy-invasive apps.

use std::collections::HashSet;
use std::path::Path;
use std::sync::RwLock;
use once_cell::sync::Lazy;

/// Global sovereignty configuration
pub static SOVEREIGNTY: Lazy<RwLock<SovereigntyConfig>> = Lazy::new(|| {
    RwLock::new(SovereigntyConfig::default())
});

/// Permission types that can be requested
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    // Clipboard
    ClipboardRead,
    ClipboardWrite,
    
    // File System
    FsRead(PathScope),
    FsWrite(PathScope),
    
    // Network
    HttpConnect(DomainScope),
    
    // Notifications
    Notifications,
    
    // Shell
    ShellOpen,
    ShellOpenPath,
    ShellExecute,
    
    // Database
    Database,
    
    // Window
    WindowCreate,
    WindowControl,
    
    // Deep Links
    DeepLinks,
    
    // System Tray
    SystemTray,
    
    // App Control
    AppExit,
    AppRelaunch,
    
    // OS Info (always allowed, read-only)
    OsInfo,
    
    // Dialogs (always allowed, user-initiated)
    Dialogs,
}

/// Scope for path-based permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathScope {
    /// Access to any path (dangerous, requires explicit declaration)
    Any,
    /// Access to app's own data directory
    AppData,
    /// Access to specific system folders
    Documents,
    Downloads,
    Desktop,
    Pictures,
    Music,
    Videos,
    Temp,
    /// Access to a specific custom path
    Custom(String),
}

/// Scope for domain-based permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DomainScope {
    /// Access to any domain (dangerous)
    Any,
    /// Access to specific domain
    Domain(String),
    /// Access to localhost only
    Localhost,
}

/// Sovereignty configuration loaded from poly.toml
#[derive(Debug, Clone, Default)]
pub struct SovereigntyConfig {
    /// Whether sovereignty is enabled (default: true for built apps)
    pub enabled: bool,
    /// Set of granted permissions
    pub permissions: HashSet<Permission>,
    /// Allowed HTTP domains
    pub http_allowlist: HashSet<String>,
    /// Blocked HTTP domains (tracking, analytics)
    pub http_blocklist: HashSet<String>,
    /// Allowed file system paths
    pub fs_allowlist: HashSet<PathScope>,
    /// Audit log enabled
    pub audit_log: bool,
    /// App name for logging
    pub app_name: String,
}

impl SovereigntyConfig {
    /// Create a permissive config for development mode
    pub fn development() -> Self {
        Self {
            enabled: false, // Disabled in dev mode
            permissions: HashSet::new(),
            http_allowlist: HashSet::new(),
            http_blocklist: HashSet::new(),
            fs_allowlist: HashSet::new(),
            audit_log: false,
            app_name: String::from("dev"),
        }
    }
    
    /// Parse sovereignty config from poly.toml content
    pub fn from_toml(content: &str, app_name: &str) -> Self {
        let mut config = Self {
            enabled: true,
            app_name: app_name.to_string(),
            ..Default::default()
        };
        
        let mut in_sovereignty = false;
        let mut in_permissions = false;
        let mut in_http_allowlist = false;
        let mut in_http_blocklist = false;
        let mut in_fs_allowlist = false;
        
        for line in content.lines() {
            let line = line.trim();
            
            // Section detection
            if line == "[sovereignty]" {
                in_sovereignty = true;
                in_permissions = false;
                in_http_allowlist = false;
                in_http_blocklist = false;
                in_fs_allowlist = false;
                continue;
            } else if line.starts_with('[') && line != "[sovereignty]" {
                in_sovereignty = false;
                in_permissions = false;
                in_http_allowlist = false;
                in_http_blocklist = false;
                in_fs_allowlist = false;
                continue;
            }
            
            if !in_sovereignty {
                continue;
            }
            
            // Parse sovereignty options
            if let Some(val) = line.strip_prefix("enabled").and_then(|s| s.trim().strip_prefix('=')) {
                config.enabled = val.trim() == "true";
            } else if let Some(val) = line.strip_prefix("audit_log").and_then(|s| s.trim().strip_prefix('=')) {
                config.audit_log = val.trim() == "true";
            } else if line.starts_with("permissions") && line.contains('[') {
                in_permissions = true;
            } else if line.starts_with("http_allowlist") && line.contains('[') {
                in_http_allowlist = true;
            } else if line.starts_with("http_blocklist") && line.contains('[') {
                in_http_blocklist = true;
            } else if line.starts_with("fs_allowlist") && line.contains('[') {
                in_fs_allowlist = true;
            } else if line == "]" {
                in_permissions = false;
                in_http_allowlist = false;
                in_http_blocklist = false;
                in_fs_allowlist = false;
            } else if in_permissions {
                // Parse permission string
                let perm = line.trim().trim_matches('"').trim_matches(',').trim_matches('"');
                if !perm.is_empty() {
                    if let Some(p) = parse_permission(perm) {
                        config.permissions.insert(p);
                    }
                }
            } else if in_http_allowlist {
                let domain = line.trim().trim_matches('"').trim_matches(',').trim_matches('"');
                if !domain.is_empty() {
                    config.http_allowlist.insert(domain.to_lowercase());
                }
            } else if in_http_blocklist {
                let domain = line.trim().trim_matches('"').trim_matches(',').trim_matches('"');
                if !domain.is_empty() {
                    config.http_blocklist.insert(domain.to_lowercase());
                }
            } else if in_fs_allowlist {
                let path = line.trim().trim_matches('"').trim_matches(',').trim_matches('"');
                if !path.is_empty() {
                    if let Some(scope) = parse_path_scope(path) {
                        config.fs_allowlist.insert(scope);
                    }
                }
            }
        }
        
        // If no sovereignty section, use permissive defaults for backwards compatibility
        if config.permissions.is_empty() && config.http_allowlist.is_empty() {
            config.enabled = false;
        }
        
        config
    }
}

/// Parse a permission string into a Permission enum
fn parse_permission(s: &str) -> Option<Permission> {
    let s = s.trim().to_lowercase();
    
    match s.as_str() {
        // Clipboard
        "clipboard" => Some(Permission::ClipboardRead), // Default to read
        "clipboard:read" => Some(Permission::ClipboardRead),
        "clipboard:write" => Some(Permission::ClipboardWrite),
        
        // Notifications
        "notifications" => Some(Permission::Notifications),
        
        // Shell
        "shell" => Some(Permission::ShellOpen),
        "shell:open" => Some(Permission::ShellOpen),
        "shell:open_path" => Some(Permission::ShellOpenPath),
        "shell:execute" => Some(Permission::ShellExecute),
        
        // Database
        "database" | "db" | "sqlite" => Some(Permission::Database),
        
        // Window
        "window" | "window:create" => Some(Permission::WindowCreate),
        "window:control" => Some(Permission::WindowControl),
        
        // Deep Links
        "deeplinks" | "deep_links" => Some(Permission::DeepLinks),
        
        // System Tray
        "tray" | "system_tray" => Some(Permission::SystemTray),
        
        // App Control
        "app:exit" => Some(Permission::AppExit),
        "app:relaunch" => Some(Permission::AppRelaunch),
        
        // File System with scope
        _ if s.starts_with("fs:") || s.starts_with("filesystem:") => {
            let rest = s.strip_prefix("fs:").or_else(|| s.strip_prefix("filesystem:"))?;
            parse_fs_permission(rest)
        }
        
        // HTTP with domain
        _ if s.starts_with("http:") || s.starts_with("network:") => {
            let rest = s.strip_prefix("http:").or_else(|| s.strip_prefix("network:"))?;
            Some(Permission::HttpConnect(parse_domain_scope(rest)))
        }
        
        _ => None,
    }
}

/// Parse file system permission with scope
fn parse_fs_permission(s: &str) -> Option<Permission> {
    let parts: Vec<&str> = s.split(':').collect();
    
    let (access, scope_str) = match parts.len() {
        1 => ("read", parts[0]),
        2 => (parts[0], parts[1]),
        _ => return None,
    };
    
    let scope = parse_path_scope(scope_str)?;
    
    match access {
        "read" => Some(Permission::FsRead(scope)),
        "write" => Some(Permission::FsWrite(scope)),
        "readwrite" | "rw" => Some(Permission::FsWrite(scope)), // Write implies read
        _ => None,
    }
}

/// Parse path scope string
fn parse_path_scope(s: &str) -> Option<PathScope> {
    let s = s.trim().to_lowercase();
    
    match s.as_str() {
        "*" | "any" => Some(PathScope::Any),
        "appdata" | "$appdata" | "app_data" => Some(PathScope::AppData),
        "documents" | "$documents" => Some(PathScope::Documents),
        "downloads" | "$downloads" => Some(PathScope::Downloads),
        "desktop" | "$desktop" => Some(PathScope::Desktop),
        "pictures" | "$pictures" => Some(PathScope::Pictures),
        "music" | "$music" => Some(PathScope::Music),
        "videos" | "$videos" => Some(PathScope::Videos),
        "temp" | "$temp" => Some(PathScope::Temp),
        _ => Some(PathScope::Custom(s.to_string())),
    }
}

/// Parse domain scope string
fn parse_domain_scope(s: &str) -> DomainScope {
    let s = s.trim().to_lowercase();
    
    match s.as_str() {
        "*" | "any" => DomainScope::Any,
        "localhost" | "127.0.0.1" => DomainScope::Localhost,
        _ => DomainScope::Domain(s),
    }
}

/// Check if a permission is granted
pub fn check_permission(permission: &Permission) -> Result<(), String> {
    let config = SOVEREIGNTY.read().unwrap();
    
    // If sovereignty is disabled, allow everything
    if !config.enabled {
        return Ok(());
    }
    
    // OS info and dialogs are always allowed (read-only, user-initiated)
    match permission {
        Permission::OsInfo | Permission::Dialogs => return Ok(()),
        _ => {}
    }
    
    // Check if permission is granted
    let granted = match permission {
        Permission::ClipboardRead => {
            config.permissions.contains(&Permission::ClipboardRead) ||
            config.permissions.contains(&Permission::ClipboardWrite)
        }
        Permission::ClipboardWrite => {
            config.permissions.contains(&Permission::ClipboardWrite)
        }
        Permission::FsRead(scope) => {
            check_fs_permission(&config, scope, false)
        }
        Permission::FsWrite(scope) => {
            check_fs_permission(&config, scope, true)
        }
        Permission::HttpConnect(domain) => {
            check_http_permission(&config, domain)
        }
        _ => config.permissions.contains(permission),
    };
    
    if granted {
        // Log if audit is enabled
        if config.audit_log {
            log_permission_use(&config.app_name, permission);
        }
        Ok(())
    } else {
        Err(format!(
            "Permission denied: {} not declared in poly.toml [sovereignty] section",
            permission_to_string(permission)
        ))
    }
}

/// Check file system permission with path scope
fn check_fs_permission(config: &SovereigntyConfig, scope: &PathScope, write: bool) -> bool {
    // Check if any matching permission exists
    for allowed in &config.fs_allowlist {
        if scope_matches(allowed, scope) {
            return true;
        }
    }
    
    // Check in permissions set
    for perm in &config.permissions {
        match perm {
            Permission::FsRead(s) if !write && scope_matches(s, scope) => return true,
            Permission::FsWrite(s) if scope_matches(s, scope) => return true,
            _ => {}
        }
    }
    
    false
}

/// Check if scope a allows scope b
fn scope_matches(allowed: &PathScope, requested: &PathScope) -> bool {
    match allowed {
        PathScope::Any => true,
        _ => allowed == requested,
    }
}

/// Check HTTP permission with domain
fn check_http_permission(config: &SovereigntyConfig, domain: &DomainScope) -> bool {
    // First check blocklist
    if let DomainScope::Domain(d) = domain {
        let d_lower = d.to_lowercase();
        for blocked in &config.http_blocklist {
            if d_lower.contains(blocked) || blocked.contains(&d_lower) {
                return false; // Explicitly blocked
            }
        }
    }
    
    // Check allowlist
    match domain {
        DomainScope::Any => {
            // Only allowed if explicitly granted
            config.permissions.contains(&Permission::HttpConnect(DomainScope::Any))
        }
        DomainScope::Localhost => {
            // Localhost is generally safe
            config.http_allowlist.contains("localhost") ||
            config.http_allowlist.contains("127.0.0.1") ||
            config.permissions.contains(&Permission::HttpConnect(DomainScope::Localhost)) ||
            config.permissions.contains(&Permission::HttpConnect(DomainScope::Any))
        }
        DomainScope::Domain(d) => {
            let d_lower = d.to_lowercase();
            
            // Check for wildcard (allow all)
            if config.http_allowlist.contains("*") {
                return true;
            }
            
            // Check explicit allowlist
            for allowed in &config.http_allowlist {
                if d_lower == *allowed || d_lower.ends_with(&format!(".{}", allowed)) {
                    return true;
                }
            }
            
            // Check permissions
            for perm in &config.permissions {
                match perm {
                    Permission::HttpConnect(DomainScope::Any) => return true,
                    Permission::HttpConnect(DomainScope::Domain(allowed)) => {
                        if d_lower == *allowed || d_lower.ends_with(&format!(".{}", allowed)) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            
            false
        }
    }
}

/// Convert permission to human-readable string
fn permission_to_string(permission: &Permission) -> String {
    match permission {
        Permission::ClipboardRead => "clipboard:read".to_string(),
        Permission::ClipboardWrite => "clipboard:write".to_string(),
        Permission::FsRead(scope) => format!("fs:read:{}", scope_to_string(scope)),
        Permission::FsWrite(scope) => format!("fs:write:{}", scope_to_string(scope)),
        Permission::HttpConnect(domain) => format!("http:{}", domain_to_string(domain)),
        Permission::Notifications => "notifications".to_string(),
        Permission::ShellOpen => "shell:open".to_string(),
        Permission::ShellOpenPath => "shell:open_path".to_string(),
        Permission::ShellExecute => "shell:execute".to_string(),
        Permission::Database => "database".to_string(),
        Permission::WindowCreate => "window:create".to_string(),
        Permission::WindowControl => "window:control".to_string(),
        Permission::DeepLinks => "deeplinks".to_string(),
        Permission::SystemTray => "system_tray".to_string(),
        Permission::AppExit => "app:exit".to_string(),
        Permission::AppRelaunch => "app:relaunch".to_string(),
        Permission::OsInfo => "os:info".to_string(),
        Permission::Dialogs => "dialogs".to_string(),
    }
}

fn scope_to_string(scope: &PathScope) -> String {
    match scope {
        PathScope::Any => "*".to_string(),
        PathScope::AppData => "$appdata".to_string(),
        PathScope::Documents => "$documents".to_string(),
        PathScope::Downloads => "$downloads".to_string(),
        PathScope::Desktop => "$desktop".to_string(),
        PathScope::Pictures => "$pictures".to_string(),
        PathScope::Music => "$music".to_string(),
        PathScope::Videos => "$videos".to_string(),
        PathScope::Temp => "$temp".to_string(),
        PathScope::Custom(p) => p.clone(),
    }
}

fn domain_to_string(domain: &DomainScope) -> String {
    match domain {
        DomainScope::Any => "*".to_string(),
        DomainScope::Localhost => "localhost".to_string(),
        DomainScope::Domain(d) => d.clone(),
    }
}

/// Log permission use for audit
fn log_permission_use(app_name: &str, permission: &Permission) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    
    eprintln!("[AUDIT] {} @ {}: {}", app_name, timestamp, permission_to_string(permission));
}

/// Initialize sovereignty from poly.toml
pub fn init_from_toml(toml_path: &Path, app_name: &str) {
    if let Ok(content) = std::fs::read_to_string(toml_path) {
        let config = SovereigntyConfig::from_toml(&content, app_name);
        *SOVEREIGNTY.write().unwrap() = config;
    }
}

/// Set development mode (disables sovereignty checks)
pub fn set_development_mode() {
    *SOVEREIGNTY.write().unwrap() = SovereigntyConfig::development();
}

/// Check if sovereignty is enabled
pub fn is_enabled() -> bool {
    SOVEREIGNTY.read().unwrap().enabled
}

/// Get list of granted permissions (for UI display)
pub fn get_granted_permissions() -> Vec<String> {
    let config = SOVEREIGNTY.read().unwrap();
    config.permissions.iter().map(permission_to_string).collect()
}

/// Helper functions for common permission checks
pub mod checks {
    use super::*;
    
    pub fn clipboard_read() -> Result<(), String> {
        check_permission(&Permission::ClipboardRead)
    }
    
    pub fn clipboard_write() -> Result<(), String> {
        check_permission(&Permission::ClipboardWrite)
    }
    
    pub fn fs_read(path: &str) -> Result<(), String> {
        let scope = path_to_scope(path);
        check_permission(&Permission::FsRead(scope))
    }
    
    pub fn fs_write(path: &str) -> Result<(), String> {
        let scope = path_to_scope(path);
        check_permission(&Permission::FsWrite(scope))
    }
    
    pub fn http(url: &str) -> Result<(), String> {
        let domain = url_to_domain(url);
        check_permission(&Permission::HttpConnect(domain))
    }
    
    pub fn notifications() -> Result<(), String> {
        check_permission(&Permission::Notifications)
    }
    
    pub fn shell_open() -> Result<(), String> {
        check_permission(&Permission::ShellOpen)
    }
    
    pub fn shell_open_path() -> Result<(), String> {
        check_permission(&Permission::ShellOpenPath)
    }
    
    pub fn database() -> Result<(), String> {
        check_permission(&Permission::Database)
    }
    
    pub fn window_create() -> Result<(), String> {
        check_permission(&Permission::WindowCreate)
    }
    
    pub fn deep_links() -> Result<(), String> {
        check_permission(&Permission::DeepLinks)
    }
    
    pub fn app_exit() -> Result<(), String> {
        check_permission(&Permission::AppExit)
    }
    
    pub fn app_relaunch() -> Result<(), String> {
        check_permission(&Permission::AppRelaunch)
    }
    
    /// Convert a file path to a PathScope
    fn path_to_scope(path: &str) -> PathScope {
        let path_lower = path.to_lowercase();
        
        // Check for special directories
        if path_lower.contains("appdata") || path_lower.contains("application data") {
            return PathScope::AppData;
        }
        if path_lower.contains("documents") || path_lower.contains("my documents") {
            return PathScope::Documents;
        }
        if path_lower.contains("downloads") {
            return PathScope::Downloads;
        }
        if path_lower.contains("desktop") {
            return PathScope::Desktop;
        }
        if path_lower.contains("pictures") || path_lower.contains("my pictures") {
            return PathScope::Pictures;
        }
        if path_lower.contains("music") || path_lower.contains("my music") {
            return PathScope::Music;
        }
        if path_lower.contains("videos") || path_lower.contains("my videos") {
            return PathScope::Videos;
        }
        if path_lower.contains("temp") || path_lower.contains("tmp") {
            return PathScope::Temp;
        }
        
        // Memory database is always allowed
        if path == ":memory:" {
            return PathScope::Temp;
        }
        
        PathScope::Custom(path.to_string())
    }
    
    /// Extract domain from URL
    fn url_to_domain(url: &str) -> DomainScope {
        // Remove protocol
        let url = url.trim_start_matches("http://")
                    .trim_start_matches("https://");
        
        // Get domain part (before first /)
        let domain = url.split('/').next().unwrap_or(url);
        
        // Remove port
        let domain = domain.split(':').next().unwrap_or(domain);
        
        if domain == "localhost" || domain == "127.0.0.1" {
            DomainScope::Localhost
        } else {
            DomainScope::Domain(domain.to_lowercase())
        }
    }
}

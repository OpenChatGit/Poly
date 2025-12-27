//! Deep Links / Custom URL Protocol for Poly
//! Register and handle custom URL schemes (e.g., myapp://)

use std::sync::Mutex;
use once_cell::sync::Lazy;

/// Store the last received deep link URL
static LAST_DEEP_LINK: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

/// Set the deep link (called from main when app is launched with URL)
pub fn set_deep_link(url: &str) {
    if let Ok(mut link) = LAST_DEEP_LINK.lock() {
        *link = Some(url.to_string());
    }
}

/// Get and clear the last deep link
pub fn get_deep_link() -> Option<String> {
    if let Ok(mut link) = LAST_DEEP_LINK.lock() {
        link.take()
    } else {
        None
    }
}

/// Check if there's a pending deep link
pub fn has_deep_link() -> bool {
    LAST_DEEP_LINK.lock().map(|l| l.is_some()).unwrap_or(false)
}

/// Register a custom URL protocol (Windows)
/// This writes to the Windows Registry - requires the app to run as admin for HKLM,
/// or uses HKCU for current user only
#[cfg(all(feature = "native", target_os = "windows"))]
pub fn register_protocol(protocol: &str, app_name: &str) -> Result<(), String> {
    use std::process::Command;
    
    // Get the path to the current executable
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?;
    let exe_path_str = exe_path.to_string_lossy();
    
    // Create registry entries using reg.exe (works without winreg crate)
    // HKEY_CURRENT_USER\Software\Classes\{protocol}
    let base_key = format!(r"HKCU\Software\Classes\{}", protocol);
    
    // Set default value (protocol description)
    let output = Command::new("reg")
        .args(["add", &base_key, "/ve", "/d", &format!("URL:{} Protocol", app_name), "/f"])
        .output()
        .map_err(|e| format!("Failed to run reg: {}", e))?;
    
    if !output.status.success() {
        return Err(format!("Failed to create registry key: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    // Set URL Protocol flag
    Command::new("reg")
        .args(["add", &base_key, "/v", "URL Protocol", "/d", "", "/f"])
        .output()
        .map_err(|e| format!("Failed to set URL Protocol: {}", e))?;
    
    // Create shell\open\command key
    let command_key = format!(r"{}\shell\open\command", base_key);
    let command_value = format!(r#""{}" "%1""#, exe_path_str);
    
    Command::new("reg")
        .args(["add", &command_key, "/ve", "/d", &command_value, "/f"])
        .output()
        .map_err(|e| format!("Failed to set command: {}", e))?;
    
    // Create DefaultIcon key (optional)
    let icon_key = format!(r"{}\DefaultIcon", base_key);
    Command::new("reg")
        .args(["add", &icon_key, "/ve", "/d", &format!("{},0", exe_path_str), "/f"])
        .output()
        .ok();
    
    Ok(())
}

/// Unregister a custom URL protocol (Windows)
#[cfg(all(feature = "native", target_os = "windows"))]
pub fn unregister_protocol(protocol: &str) -> Result<(), String> {
    use std::process::Command;
    
    let base_key = format!(r"HKCU\Software\Classes\{}", protocol);
    
    Command::new("reg")
        .args(["delete", &base_key, "/f"])
        .output()
        .map_err(|e| format!("Failed to delete registry key: {}", e))?;
    
    Ok(())
}

/// Check if a protocol is registered (Windows)
#[cfg(all(feature = "native", target_os = "windows"))]
pub fn is_protocol_registered(protocol: &str) -> bool {
    use std::process::Command;
    
    let base_key = format!(r"HKCU\Software\Classes\{}", protocol);
    
    Command::new("reg")
        .args(["query", &base_key])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// Stubs for non-Windows or non-native
#[cfg(not(all(feature = "native", target_os = "windows")))]
pub fn register_protocol(_protocol: &str, _app_name: &str) -> Result<(), String> {
    Err("Deep links only supported on Windows with native feature".to_string())
}

#[cfg(not(all(feature = "native", target_os = "windows")))]
pub fn unregister_protocol(_protocol: &str) -> Result<(), String> {
    Err("Deep links only supported on Windows with native feature".to_string())
}

#[cfg(not(all(feature = "native", target_os = "windows")))]
pub fn is_protocol_registered(_protocol: &str) -> bool {
    false
}

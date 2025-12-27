//! Native Notifications API for Poly
//! Send OS notifications from JavaScript

/// Show a native notification
#[cfg(feature = "native")]
pub fn show(title: &str, body: &str, icon: Option<&str>) -> Result<(), String> {
    use notify_rust::Notification;
    
    let mut notification = Notification::new();
    notification.summary(title);
    notification.body(body);
    
    // Set app name
    notification.appname("Poly");
    
    // Set icon if provided (path to image)
    if let Some(icon_path) = icon {
        notification.icon(icon_path);
    }
    
    notification.show()
        .map_err(|e| format!("Notification error: {}", e))?;
    
    Ok(())
}

/// Show notification with timeout (in milliseconds)
#[cfg(feature = "native")]
pub fn show_with_timeout(title: &str, body: &str, timeout_ms: u32) -> Result<(), String> {
    use notify_rust::Notification;
    use notify_rust::Timeout;
    
    Notification::new()
        .summary(title)
        .body(body)
        .appname("Poly")
        .timeout(Timeout::Milliseconds(timeout_ms))
        .show()
        .map_err(|e| format!("Notification error: {}", e))?;
    
    Ok(())
}

// Stubs for non-native
#[cfg(not(feature = "native"))]
pub fn show(_title: &str, _body: &str, _icon: Option<&str>) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn show_with_timeout(_title: &str, _body: &str, _timeout_ms: u32) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

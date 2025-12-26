//! Native WebView runner for Poly applications
//! Similar to Tauri/Electron but lightweight

#[cfg(feature = "native")]
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[cfg(feature = "native")]
use wry::WebViewBuilder;

/// Configuration for native window
#[derive(Debug, Clone)]
pub struct NativeConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub fullscreen: bool,
    pub transparent: bool,
    pub decorations: bool,
    pub always_on_top: bool,
    pub dev_tools: bool,
    pub icon_path: Option<String>,
    /// Enable system tray
    pub tray_enabled: bool,
    /// Tray icon path (uses icon_path if not set)
    pub tray_icon_path: Option<String>,
    /// Tray tooltip
    pub tray_tooltip: Option<String>,
    /// Minimize to tray instead of taskbar
    pub minimize_to_tray: bool,
    /// Close to tray instead of exiting
    pub close_to_tray: bool,
    /// Custom tray menu items: (id, label) - "separator" id for separator
    pub tray_menu_items: Vec<(String, String)>,
}

impl Default for NativeConfig {
    fn default() -> Self {
        Self {
            title: "Poly App".to_string(),
            width: 1024,
            height: 768,
            resizable: true,
            fullscreen: false,
            transparent: false,
            decorations: true,
            always_on_top: false,
            dev_tools: false,
            icon_path: None,
            tray_enabled: false,
            tray_icon_path: None,
            tray_tooltip: None,
            minimize_to_tray: false,
            close_to_tray: false,
            tray_menu_items: Vec::new(),
        }
    }
}

impl NativeConfig {
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
    
    pub fn with_dev_tools(mut self, enabled: bool) -> Self {
        self.dev_tools = enabled;
        self
    }
    
    pub fn with_icon(mut self, path: &str) -> Self {
        self.icon_path = Some(path.to_string());
        self
    }
    
    pub fn with_tray(mut self, enabled: bool) -> Self {
        self.tray_enabled = enabled;
        self
    }
    
    pub fn with_tray_icon(mut self, path: &str) -> Self {
        self.tray_icon_path = Some(path.to_string());
        self
    }
    
    pub fn with_minimize_to_tray(mut self, enabled: bool) -> Self {
        self.minimize_to_tray = enabled;
        self
    }
    
    pub fn with_close_to_tray(mut self, enabled: bool) -> Self {
        self.close_to_tray = enabled;
        self
    }
}

// ============================================
// Native Dialogs API
// ============================================

/// Show a file open dialog
#[cfg(feature = "native")]
pub fn dialog_open_file(title: Option<&str>, filters: Option<Vec<(&str, &[&str])>>) -> Option<String> {
    let mut dialog = rfd::FileDialog::new();
    
    if let Some(t) = title {
        dialog = dialog.set_title(t);
    }
    
    if let Some(f) = filters {
        for (name, exts) in f {
            dialog = dialog.add_filter(name, exts);
        }
    }
    
    dialog.pick_file().map(|p| p.to_string_lossy().to_string())
}

/// Show a file open dialog for multiple files
#[cfg(feature = "native")]
pub fn dialog_open_files(title: Option<&str>, filters: Option<Vec<(&str, &[&str])>>) -> Vec<String> {
    let mut dialog = rfd::FileDialog::new();
    
    if let Some(t) = title {
        dialog = dialog.set_title(t);
    }
    
    if let Some(f) = filters {
        for (name, exts) in f {
            dialog = dialog.add_filter(name, exts);
        }
    }
    
    dialog.pick_files()
        .map(|files| files.into_iter().map(|p| p.to_string_lossy().to_string()).collect())
        .unwrap_or_default()
}

/// Show a file save dialog
#[cfg(feature = "native")]
pub fn dialog_save_file(title: Option<&str>, default_name: Option<&str>, filters: Option<Vec<(&str, &[&str])>>) -> Option<String> {
    let mut dialog = rfd::FileDialog::new();
    
    if let Some(t) = title {
        dialog = dialog.set_title(t);
    }
    
    if let Some(name) = default_name {
        dialog = dialog.set_file_name(name);
    }
    
    if let Some(f) = filters {
        for (name, exts) in f {
            dialog = dialog.add_filter(name, exts);
        }
    }
    
    dialog.save_file().map(|p| p.to_string_lossy().to_string())
}

/// Show a folder picker dialog
#[cfg(feature = "native")]
pub fn dialog_pick_folder(title: Option<&str>) -> Option<String> {
    let mut dialog = rfd::FileDialog::new();
    
    if let Some(t) = title {
        dialog = dialog.set_title(t);
    }
    
    dialog.pick_folder().map(|p| p.to_string_lossy().to_string())
}

/// Message dialog level
#[derive(Debug, Clone, Copy)]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
}

/// Show a message dialog
#[cfg(feature = "native")]
pub fn dialog_message(title: &str, message: &str, level: MessageLevel) {
    let msg_level = match level {
        MessageLevel::Info => rfd::MessageLevel::Info,
        MessageLevel::Warning => rfd::MessageLevel::Warning,
        MessageLevel::Error => rfd::MessageLevel::Error,
    };
    
    rfd::MessageDialog::new()
        .set_title(title)
        .set_description(message)
        .set_level(msg_level)
        .show();
}

/// Show a confirm dialog (Yes/No)
#[cfg(feature = "native")]
pub fn dialog_confirm(title: &str, message: &str) -> bool {
    rfd::MessageDialog::new()
        .set_title(title)
        .set_description(message)
        .set_level(rfd::MessageLevel::Info)
        .set_buttons(rfd::MessageButtons::YesNo)
        .show() == rfd::MessageDialogResult::Yes
}

// Stubs for non-native builds
#[cfg(not(feature = "native"))]
pub fn dialog_open_file(_title: Option<&str>, _filters: Option<Vec<(&str, &[&str])>>) -> Option<String> { None }
#[cfg(not(feature = "native"))]
pub fn dialog_open_files(_title: Option<&str>, _filters: Option<Vec<(&str, &[&str])>>) -> Vec<String> { vec![] }
#[cfg(not(feature = "native"))]
pub fn dialog_save_file(_title: Option<&str>, _default_name: Option<&str>, _filters: Option<Vec<(&str, &[&str])>>) -> Option<String> { None }
#[cfg(not(feature = "native"))]
pub fn dialog_pick_folder(_title: Option<&str>) -> Option<String> { None }
#[cfg(not(feature = "native"))]
pub fn dialog_message(_title: &str, _message: &str, _level: MessageLevel) {}
#[cfg(not(feature = "native"))]
pub fn dialog_confirm(_title: &str, _message: &str) -> bool { false }


// ============================================
// Window Management
// ============================================

/// Run a native window with the given HTML content
#[cfg(feature = "native")]
pub fn run_native_window(html: &str, config: NativeConfig) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new();
    
    let mut window_builder = WindowBuilder::new()
        .with_title(&config.title)
        .with_inner_size(tao::dpi::LogicalSize::new(config.width, config.height))
        .with_resizable(config.resizable)
        .with_decorations(config.decorations)
        .with_always_on_top(config.always_on_top);
    
    if let Some(ref icon_path) = config.icon_path {
        if let Ok(icon) = load_icon(icon_path) {
            window_builder = window_builder.with_window_icon(Some(icon));
        }
    }
    
    let window = window_builder.build(&event_loop)?;
    
    let html_owned = html.to_string();
    let _webview = WebViewBuilder::new()
        .with_html(&html_owned)
        .with_devtools(config.dev_tools)
        .build(&window)?;
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}

/// Load an icon from a PNG file
#[cfg(feature = "native")]
fn load_icon(path: &str) -> Result<tao::window::Icon, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::BufReader;
    use image::GenericImageView;
    
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let img = image::load(reader, image::ImageFormat::Png)?;
    
    let (width, height) = img.dimensions();
    let size = width.max(height);
    let mut square = image::RgbaImage::new(size, size);
    
    for pixel in square.pixels_mut() {
        *pixel = image::Rgba([0, 0, 0, 0]);
    }
    
    let x_offset = (size - width) / 2;
    let y_offset = (size - height) / 2;
    
    for (x, y, pixel) in img.to_rgba8().enumerate_pixels() {
        square.put_pixel(x + x_offset, y + y_offset, *pixel);
    }
    
    let resized = image::imageops::resize(&square, 64, 64, image::imageops::FilterType::Lanczos3);
    let icon = tao::window::Icon::from_rgba(resized.into_raw(), 64, 64)?;
    Ok(icon)
}

/// Run a native window with a URL (frameless with custom titlebar)
#[cfg(feature = "native")]
pub fn run_native_url(url: &str, config: NativeConfig) -> Result<(), Box<dyn std::error::Error>> {
    #[allow(unused_imports)]
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    #[allow(unused_imports)]
    use tray_icon::{TrayIconBuilder, Icon as TrayIconType, menu::{Menu, MenuItem, PredefinedMenuItem, MenuEvent}};
    
    let event_loop = EventLoop::new();
    
    // Frameless window for custom titlebar
    let mut window_builder = WindowBuilder::new()
        .with_title(&config.title)
        .with_inner_size(tao::dpi::LogicalSize::new(config.width, config.height))
        .with_resizable(config.resizable)
        .with_decorations(false); // No native titlebar!
    
    if let Some(ref icon_path) = config.icon_path {
        if let Ok(icon) = load_icon(icon_path) {
            window_builder = window_builder.with_window_icon(Some(icon));
        }
    }
    
    let window = Arc::new(window_builder.build(&event_loop)?);
    let window_clone = Arc::clone(&window);
    let window_for_tray = Arc::clone(&window);
    
    // Track if we should close to tray
    let close_to_tray = config.close_to_tray;
    let minimize_to_tray = config.minimize_to_tray;
    
    // Create system tray if enabled
    let _tray = if config.tray_enabled {
        let tray_menu = Menu::new();
        
        // Track menu item IDs for event handling
        let mut menu_id_map: std::collections::HashMap<tray_icon::menu::MenuId, String> = std::collections::HashMap::new();
        let mut show_id: Option<tray_icon::menu::MenuId> = None;
        let mut quit_id: Option<tray_icon::menu::MenuId> = None;
        
        // Use custom menu items if provided, otherwise use defaults
        if config.tray_menu_items.is_empty() {
            // Default menu
            let show_item = MenuItem::new("Show", true, None);
            let quit_item = MenuItem::new("Quit", true, None);
            show_id = Some(show_item.id().clone());
            quit_id = Some(quit_item.id().clone());
            menu_id_map.insert(show_item.id().clone(), "show".to_string());
            menu_id_map.insert(quit_item.id().clone(), "quit".to_string());
            
            tray_menu.append(&show_item)?;
            tray_menu.append(&PredefinedMenuItem::separator())?;
            tray_menu.append(&quit_item)?;
        } else {
            // Custom menu from config
            for (id, label) in &config.tray_menu_items {
                if id == "separator" {
                    tray_menu.append(&PredefinedMenuItem::separator())?;
                } else {
                    let item = MenuItem::new(label, true, None);
                    menu_id_map.insert(item.id().clone(), id.clone());
                    
                    // Track special IDs
                    if id == "show" {
                        show_id = Some(item.id().clone());
                    } else if id == "quit" || id == "exit" {
                        quit_id = Some(item.id().clone());
                    }
                    
                    tray_menu.append(&item)?;
                }
            }
        }
        
        // Load tray icon
        let tray_icon_path = config.tray_icon_path.as_ref().or(config.icon_path.as_ref());
        let tray_icon = if let Some(path) = tray_icon_path {
            load_tray_icon_from_file(path)?
        } else {
            create_default_tray_icon()?
        };
        
        let tooltip = config.tray_tooltip.as_ref().unwrap_or(&config.title);
        
        let tray = TrayIconBuilder::new()
            .with_tooltip(tooltip)
            .with_icon(tray_icon)
            .with_menu(Box::new(tray_menu))
            .build()?;
        
        // Handle tray menu events in a separate thread
        let window_for_menu = Arc::clone(&window_for_tray);
        std::thread::spawn(move || {
            let receiver = MenuEvent::receiver();
            loop {
                if let Ok(event) = receiver.recv() {
                    // Check for built-in actions
                    if Some(&event.id) == show_id.as_ref() {
                        window_for_menu.set_visible(true);
                        window_for_menu.set_focus();
                    } else if Some(&event.id) == quit_id.as_ref() {
                        std::process::exit(0);
                    }
                    // Custom menu items will be handled via IPC
                    // The menu_id_map could be used to send events to JS
                    if let Some(custom_id) = menu_id_map.get(&event.id) {
                        // For now, just log custom menu clicks
                        // TODO: Send to webview via IPC
                        eprintln!("Tray menu clicked: {}", custom_id);
                    }
                }
            }
        });
        
        Some(tray)
    } else {
        None
    };
    
    let _webview = WebViewBuilder::new()
        .with_url(url)
        .with_devtools(config.dev_tools)
        .with_ipc_handler(move |req: wry::http::Request<String>| {
            let body = req.body();
            match body.as_str() {
                "minimize" => {
                    if minimize_to_tray {
                        window_clone.set_visible(false);
                    } else {
                        window_clone.set_minimized(true);
                    }
                }
                "maximize" => {
                    if window_clone.is_maximized() {
                        window_clone.set_maximized(false);
                    } else {
                        window_clone.set_maximized(true);
                    }
                }
                "close" => {
                    if close_to_tray {
                        window_clone.set_visible(false);
                    } else {
                        std::process::exit(0);
                    }
                }
                "hide" => {
                    window_clone.set_visible(false);
                }
                "show" => {
                    window_clone.set_visible(true);
                    window_clone.set_focus();
                }
                cmd if cmd.starts_with("drag") => {
                    let _ = window_clone.drag_window();
                }
                _ => {}
            }
        })
        .build(&*window)?;
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if close_to_tray && _tray.is_some() {
                    window_for_tray.set_visible(false);
                } else {
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
    });
}

/// Load tray icon from file
#[cfg(feature = "native")]
fn load_tray_icon_from_file(path: &str) -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::BufReader;
    use image::GenericImageView;
    
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    
    let format = if path.ends_with(".png") {
        image::ImageFormat::Png
    } else if path.ends_with(".ico") {
        image::ImageFormat::Ico
    } else {
        image::ImageFormat::Png
    };
    
    let img = image::load(reader, format)?;
    let (width, height) = img.dimensions();
    
    // Windows tray icons should be 32x32 or 64x64
    // Preserve aspect ratio by fitting into a square with transparency
    let size = 32u32;
    
    // Create a transparent square canvas
    let mut canvas = image::RgbaImage::new(size, size);
    for pixel in canvas.pixels_mut() {
        *pixel = image::Rgba([0, 0, 0, 0]);
    }
    
    // Calculate scaling to fit while preserving aspect ratio
    let scale = (size as f32 / width as f32).min(size as f32 / height as f32);
    let new_width = (width as f32 * scale) as u32;
    let new_height = (height as f32 * scale) as u32;
    
    // Resize the image preserving aspect ratio
    let resized = image::imageops::resize(
        &img.to_rgba8(),
        new_width,
        new_height,
        image::imageops::FilterType::Lanczos3
    );
    
    // Center the resized image on the canvas
    let x_offset = (size - new_width) / 2;
    let y_offset = (size - new_height) / 2;
    
    for (x, y, pixel) in resized.enumerate_pixels() {
        canvas.put_pixel(x + x_offset, y + y_offset, *pixel);
    }
    
    let icon = tray_icon::Icon::from_rgba(canvas.into_raw(), size, size)?;
    Ok(icon)
}

/// Create default tray icon (Poly cyan circle)
#[cfg(feature = "native")]
fn create_default_tray_icon() -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
    let size = 32u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    
    for y in 0..size {
        for x in 0..size {
            let cx = (x as f32 - size as f32 / 2.0).abs();
            let cy = (y as f32 - size as f32 / 2.0).abs();
            let dist = (cx * cx + cy * cy).sqrt();
            
            if dist < size as f32 / 2.0 - 2.0 {
                // Poly cyan: #5dc1d2
                rgba.push(93);
                rgba.push(193);
                rgba.push(210);
                rgba.push(255);
            } else if dist < size as f32 / 2.0 {
                let alpha = ((size as f32 / 2.0 - dist) * 127.0) as u8;
                rgba.push(93);
                rgba.push(193);
                rgba.push(210);
                rgba.push(alpha);
            } else {
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
            }
        }
    }
    
    let icon = tray_icon::Icon::from_rgba(rgba, size, size)?;
    Ok(icon)
}

/// Stub for when native feature is not enabled
#[cfg(not(feature = "native"))]
pub fn run_native_window(_html: &str, _config: NativeConfig) -> Result<(), Box<dyn std::error::Error>> {
    Err("Native feature not enabled. Build with --features native".into())
}

#[cfg(not(feature = "native"))]
pub fn run_native_url(_url: &str, _config: NativeConfig) -> Result<(), Box<dyn std::error::Error>> {
    Err("Native feature not enabled. Build with --features native".into())
}

/// Generate a standalone native app bundle
pub fn generate_native_bundle(
    project_path: &std::path::Path,
    html_content: &str,
    config: &NativeConfig,
    release: bool,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let dist = project_path.join(if release { "dist/release" } else { "dist/debug" });
    std::fs::create_dir_all(&dist)?;
    
    let html_path = dist.join("index.html");
    std::fs::write(&html_path, html_content)?;
    
    let config_json = serde_json::json!({
        "title": config.title,
        "width": config.width,
        "height": config.height,
        "resizable": config.resizable,
        "devTools": config.dev_tools,
    });
    std::fs::write(dist.join("poly.json"), serde_json::to_string_pretty(&config_json)?)?;
    
    Ok(dist)
}

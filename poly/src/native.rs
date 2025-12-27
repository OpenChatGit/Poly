//! Native WebView runner for Poly applications
//! Similar to Tauri/Electron but lightweight

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
    pub tray_enabled: bool,
    pub tray_icon_path: Option<String>,
    pub tray_tooltip: Option<String>,
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
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
        Self { title: title.to_string(), ..Default::default() }
    }
    
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width; self.height = height; self
    }
    
    pub fn with_dev_tools(mut self, enabled: bool) -> Self {
        self.dev_tools = enabled; self
    }
    
    pub fn with_icon(mut self, path: &str) -> Self {
        self.icon_path = Some(path.to_string()); self
    }
    
    pub fn with_tray(mut self, enabled: bool) -> Self {
        self.tray_enabled = enabled; self
    }
    
    pub fn with_tray_icon(mut self, path: &str) -> Self {
        self.tray_icon_path = Some(path.to_string()); self
    }
    
    pub fn with_minimize_to_tray(mut self, enabled: bool) -> Self {
        self.minimize_to_tray = enabled; self
    }
    
    pub fn with_close_to_tray(mut self, enabled: bool) -> Self {
        self.close_to_tray = enabled; self
    }
    
    pub fn with_decorations(mut self, enabled: bool) -> Self {
        self.decorations = enabled; self
    }
    
    pub fn with_transparent(mut self, enabled: bool) -> Self {
        self.transparent = enabled; self
    }
}

// ============================================
// Native Dialogs API
// ============================================

#[cfg(feature = "native")]
pub fn dialog_open_file(title: Option<&str>, filters: Option<Vec<(&str, &[&str])>>) -> Option<String> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(t) = title { dialog = dialog.set_title(t); }
    if let Some(f) = filters { for (name, exts) in f { dialog = dialog.add_filter(name, exts); } }
    dialog.pick_file().map(|p| p.to_string_lossy().to_string())
}

#[cfg(feature = "native")]
pub fn dialog_open_files(title: Option<&str>, filters: Option<Vec<(&str, &[&str])>>) -> Vec<String> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(t) = title { dialog = dialog.set_title(t); }
    if let Some(f) = filters { for (name, exts) in f { dialog = dialog.add_filter(name, exts); } }
    dialog.pick_files().map(|f| f.into_iter().map(|p| p.to_string_lossy().to_string()).collect()).unwrap_or_default()
}

#[cfg(feature = "native")]
pub fn dialog_save_file(title: Option<&str>, default_name: Option<&str>, filters: Option<Vec<(&str, &[&str])>>) -> Option<String> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(t) = title { dialog = dialog.set_title(t); }
    if let Some(name) = default_name { dialog = dialog.set_file_name(name); }
    if let Some(f) = filters { for (name, exts) in f { dialog = dialog.add_filter(name, exts); } }
    dialog.save_file().map(|p| p.to_string_lossy().to_string())
}

#[cfg(feature = "native")]
pub fn dialog_pick_folder(title: Option<&str>) -> Option<String> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(t) = title { dialog = dialog.set_title(t); }
    dialog.pick_folder().map(|p| p.to_string_lossy().to_string())
}

#[derive(Debug, Clone, Copy)]
pub enum MessageLevel { Info, Warning, Error }

#[cfg(feature = "native")]
pub fn dialog_message(title: &str, message: &str, level: MessageLevel) {
    let lvl = match level {
        MessageLevel::Info => rfd::MessageLevel::Info,
        MessageLevel::Warning => rfd::MessageLevel::Warning,
        MessageLevel::Error => rfd::MessageLevel::Error,
    };
    rfd::MessageDialog::new().set_title(title).set_description(message).set_level(lvl).show();
}

#[cfg(feature = "native")]
pub fn dialog_confirm(title: &str, message: &str) -> bool {
    rfd::MessageDialog::new()
        .set_title(title).set_description(message)
        .set_level(rfd::MessageLevel::Info)
        .set_buttons(rfd::MessageButtons::YesNo)
        .show() == rfd::MessageDialogResult::Yes
}

#[cfg(not(feature = "native"))]
pub fn dialog_open_file(_: Option<&str>, _: Option<Vec<(&str, &[&str])>>) -> Option<String> { None }
#[cfg(not(feature = "native"))]
pub fn dialog_open_files(_: Option<&str>, _: Option<Vec<(&str, &[&str])>>) -> Vec<String> { vec![] }
#[cfg(not(feature = "native"))]
pub fn dialog_save_file(_: Option<&str>, _: Option<&str>, _: Option<Vec<(&str, &[&str])>>) -> Option<String> { None }
#[cfg(not(feature = "native"))]
pub fn dialog_pick_folder(_: Option<&str>) -> Option<String> { None }
#[cfg(not(feature = "native"))]
pub fn dialog_message(_: &str, _: &str, _: MessageLevel) {}
#[cfg(not(feature = "native"))]
pub fn dialog_confirm(_: &str, _: &str) -> bool { false }


// ============================================
// Window Management
// ============================================

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

/// Run a native window with HTML content
#[cfg(feature = "native")]
pub fn run_native_window(html: &str, config: NativeConfig) -> Result<(), Box<dyn std::error::Error>> {
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    };
    
    let event_loop = EventLoop::new();
    
    let mut builder = WindowBuilder::new()
        .with_title(&config.title)
        .with_inner_size(tao::dpi::LogicalSize::new(config.width, config.height))
        .with_resizable(config.resizable)
        .with_decorations(config.decorations)
        .with_always_on_top(config.always_on_top)
        .with_transparent(config.transparent);
    
    if let Some(ref icon_path) = config.icon_path {
        if let Ok(icon) = load_icon(icon_path) {
            builder = builder.with_window_icon(Some(icon));
        }
    }
    
    let window = builder.build(&event_loop)?;
    
    let webview = WebViewBuilder::new()
        .with_html(html)
        .with_devtools(config.dev_tools)
        .with_transparent(config.transparent)
        .build(&window)?;
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                let _ = webview.set_bounds(wry::Rect {
                    position: wry::dpi::Position::Logical(wry::dpi::LogicalPosition::new(0.0, 0.0)),
                    size: wry::dpi::Size::Physical(wry::dpi::PhysicalSize::new(size.width, size.height)),
                });
            }
            _ => {}
        }
    });
}

/// Run a native window with a URL
#[cfg(feature = "native")]
pub fn run_native_url(url: &str, config: NativeConfig) -> Result<(), Box<dyn std::error::Error>> {
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    };
    use std::sync::{Arc, Mutex};
    
    let event_loop = EventLoop::new();
    
    let mut builder = WindowBuilder::new()
        .with_title(&config.title)
        .with_inner_size(tao::dpi::LogicalSize::new(config.width as f64, config.height as f64))
        .with_resizable(config.resizable)
        .with_decorations(config.decorations)
        .with_transparent(config.transparent);
    
    if let Some(ref icon_path) = config.icon_path {
        if let Ok(icon) = load_icon(icon_path) {
            builder = builder.with_window_icon(Some(icon));
        }
    }
    
    let window = builder.build(&event_loop)?;
    let window = Arc::new(window);
    let window_clone = Arc::clone(&window);
    
    // Create system tray if enabled
    let _tray = if config.tray_enabled {
        use crate::tray::{TrayConfig, TrayMenuItem, create_tray};
        
        // Build menu items from config
        let mut menu_items = Vec::new();
        for (id, label) in &config.tray_menu_items {
            if id == "separator" {
                menu_items.push(TrayMenuItem::Separator);
            } else {
                menu_items.push(TrayMenuItem::Item {
                    id: id.clone(),
                    label: label.clone(),
                    enabled: true,
                });
            }
        }
        
        // Add default items if none specified
        if menu_items.is_empty() {
            menu_items = vec![
                TrayMenuItem::Item { id: "show".to_string(), label: "Show".to_string(), enabled: true },
                TrayMenuItem::Separator,
                TrayMenuItem::Item { id: "quit".to_string(), label: "Quit".to_string(), enabled: true },
            ];
        }
        
        let mut tray_config = TrayConfig::new(&config.tray_tooltip.clone().unwrap_or_else(|| config.title.clone()))
            .with_menu(menu_items);
        
        if let Some(ref icon_path) = config.tray_icon_path {
            tray_config = tray_config.with_icon(icon_path);
        } else if let Some(ref icon_path) = config.icon_path {
            tray_config = tray_config.with_icon(icon_path);
        }
        
        match create_tray(tray_config) {
            Ok(handle) => Some(handle),
            Err(e) => {
                eprintln!("Warning: Failed to create system tray: {}", e);
                None
            }
        }
    } else {
        None
    };
    
    // Store tray handle and config for event handling
    let tray_handle = Arc::new(Mutex::new(_tray));
    let close_to_tray = config.close_to_tray;
    let minimize_to_tray = config.minimize_to_tray;
    let tray_enabled = config.tray_enabled;
    
    let webview = wry::WebViewBuilder::new()
        .with_url(url)
        .with_devtools(config.dev_tools)
        .with_transparent(config.transparent)
        .with_ipc_handler(move |msg: wry::http::Request<String>| {
            let body = msg.body();
            match body.as_str() {
                "minimize" => {
                    if minimize_to_tray && tray_enabled {
                        window_clone.set_visible(false);
                    } else {
                        window_clone.set_minimized(true);
                    }
                }
                "maximize" => {
                    window_clone.set_maximized(!window_clone.is_maximized());
                }
                "close" => {
                    if close_to_tray && tray_enabled {
                        window_clone.set_visible(false);
                    } else {
                        std::process::exit(0);
                    }
                }
                "drag" => {
                    let _ = window_clone.drag_window();
                }
                "hide" => {
                    window_clone.set_visible(false);
                }
                "show" => {
                    window_clone.set_visible(true);
                    window_clone.set_focus();
                }
                _ => {}
            }
        })
        .build(&window)?;
    
    let window_for_tray = Arc::clone(&window);
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        
        // Handle tray events
        if let Some(ref tray) = *tray_handle.lock().unwrap() {
            while let Some(tray_event) = tray.poll_event() {
                match tray_event {
                    crate::tray::TrayEvent::MenuClick { id } => {
                        match id.as_str() {
                            "show" => {
                                window_for_tray.set_visible(true);
                                window_for_tray.set_focus();
                            }
                            "quit" | "exit" => {
                                *control_flow = ControlFlow::Exit;
                            }
                            _ => {
                                // Send custom menu event to webview
                                let js = format!("window.dispatchEvent(new CustomEvent('polytray', {{ detail: {{ id: '{}' }} }}));", id);
                                let _ = webview.evaluate_script(&js);
                            }
                        }
                    }
                    crate::tray::TrayEvent::IconClick | crate::tray::TrayEvent::IconDoubleClick => {
                        window_for_tray.set_visible(true);
                        window_for_tray.set_focus();
                    }
                }
            }
        }
        
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                if close_to_tray && tray_enabled {
                    window_for_tray.set_visible(false);
                } else {
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                let _ = webview.set_bounds(wry::Rect {
                    position: wry::dpi::Position::Logical(wry::dpi::LogicalPosition::new(0.0, 0.0)),
                    size: wry::dpi::Size::Physical(wry::dpi::PhysicalSize::new(size.width, size.height)),
                });
            }
            _ => {}
        }
    });
}

// ============================================
// Tray Icon Helpers
// ============================================

#[cfg(feature = "native")]
#[allow(dead_code)]
fn load_tray_icon_from_file(path: &str) -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::BufReader;
    use image::GenericImageView;
    
    let file = File::open(path)?;
    let format = if path.ends_with(".ico") { image::ImageFormat::Ico } else { image::ImageFormat::Png };
    let img = image::load(BufReader::new(file), format)?;
    let (w, h) = img.dimensions();
    
    let size = 32u32;
    let mut canvas = image::RgbaImage::new(size, size);
    for p in canvas.pixels_mut() { *p = image::Rgba([0,0,0,0]); }
    
    let scale = (size as f32 / w as f32).min(size as f32 / h as f32);
    let (nw, nh) = ((w as f32 * scale) as u32, (h as f32 * scale) as u32);
    let resized = image::imageops::resize(&img.to_rgba8(), nw, nh, image::imageops::FilterType::Lanczos3);
    
    let (xo, yo) = ((size - nw) / 2, (size - nh) / 2);
    for (x, y, p) in resized.enumerate_pixels() { canvas.put_pixel(x + xo, y + yo, *p); }
    
    Ok(tray_icon::Icon::from_rgba(canvas.into_raw(), size, size)?)
}

#[cfg(feature = "native")]
#[allow(dead_code)]
fn create_default_tray_icon() -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
    let size = 32u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let dist = ((x as f32 - 16.0).powi(2) + (y as f32 - 16.0).powi(2)).sqrt();
            if dist < 14.0 {
                rgba.extend_from_slice(&[93, 193, 210, 255]); // Poly cyan
            } else if dist < 16.0 {
                rgba.extend_from_slice(&[93, 193, 210, ((16.0 - dist) * 127.0) as u8]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    Ok(tray_icon::Icon::from_rgba(rgba, size, size)?)
}

// ============================================
// Stubs
// ============================================

#[cfg(not(feature = "native"))]
pub fn run_native_window(_: &str, _: NativeConfig) -> Result<(), Box<dyn std::error::Error>> {
    Err("Native feature not enabled".into())
}

#[cfg(not(feature = "native"))]
pub fn run_native_url(_: &str, _: NativeConfig) -> Result<(), Box<dyn std::error::Error>> {
    Err("Native feature not enabled".into())
}

pub fn generate_native_bundle(
    project_path: &std::path::Path,
    html_content: &str,
    config: &NativeConfig,
    release: bool,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let dist = project_path.join(if release { "dist/release" } else { "dist/debug" });
    std::fs::create_dir_all(&dist)?;
    std::fs::write(dist.join("index.html"), html_content)?;
    std::fs::write(dist.join("poly.json"), serde_json::to_string_pretty(&serde_json::json!({
        "title": config.title, "width": config.width, "height": config.height,
        "resizable": config.resizable, "devTools": config.dev_tools,
    }))?)?;
    Ok(dist)
}

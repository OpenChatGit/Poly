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
    use std::sync::Arc;
    use tao::window::Window;
    
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
    
    let _webview = WebViewBuilder::new()
        .with_url(url)
        .with_devtools(config.dev_tools)
        .with_ipc_handler(move |req: wry::http::Request<String>| {
            let body = req.body();
            match body.as_str() {
                "minimize" => window_clone.set_minimized(true),
                "maximize" => {
                    if window_clone.is_maximized() {
                        window_clone.set_maximized(false);
                    } else {
                        window_clone.set_maximized(true);
                    }
                }
                "close" => std::process::exit(0),
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
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
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

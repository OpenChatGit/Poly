//! Native WebView Implementation
//!
//! Implements the WebView operations using wry/tao.
//! Handles all native WebView lifecycle and events.

// ============================================
// BrowserConfig - always available
// ============================================

/// Configuration for browser mode
#[derive(Debug, Clone)]
pub struct BrowserConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub ui_height: u32,
    pub ui_html: String,
    pub start_url: String,
    pub devtools: bool,
    pub icon_path: Option<String>,
    pub decorations: bool,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            title: "Poly Browser".to_string(),
            width: 1024,
            height: 768,
            ui_height: 50,
            ui_html: String::new(),
            start_url: "about:blank".to_string(),
            devtools: false,
            icon_path: None,
            decorations: false,
        }
    }
}

// ============================================
// Native implementation
// ============================================

#[cfg(feature = "native")]
mod native_impl {
    use super::BrowserConfig;
    use std::collections::HashMap;
    use wry::{WebView, WebViewBuilder, Rect};
    use wry::dpi::{LogicalPosition, LogicalSize};
    use crate::webview::{
        WebViewBounds, WebViewConfig, WebViewOperation, WebViewEvent,
        push_event, update_url, update_title, update_loading, update_history,
    };

    /// Manages multiple WebView instances within a window
    pub struct WebViewManager {
        instances: HashMap<String, WebView>,
        main_webview: Option<WebView>,
    }

    impl WebViewManager {
        pub fn new() -> Self {
            Self {
                instances: HashMap::new(),
                main_webview: None,
            }
        }
        
        pub fn set_main_webview(&mut self, webview: WebView) {
            self.main_webview = Some(webview);
        }
        
        pub fn main_webview(&self) -> Option<&WebView> {
            self.main_webview.as_ref()
        }

        /// Process all pending operations from the queue
        pub fn process_pending_operations(&mut self, window: &tao::window::Window) {
            let operations = crate::webview::take_pending_operations();
            
            for op in operations {
                let result = match op {
                    WebViewOperation::Create(config) => self.create_webview(window, config),
                    WebViewOperation::Destroy { id } => self.destroy_webview(&id),
                    WebViewOperation::Navigate { id, url } => self.navigate(&id, &url),
                    WebViewOperation::LoadHtml { id, html } => self.load_html(&id, &html),
                    WebViewOperation::GoBack { id } => self.go_back(&id),
                    WebViewOperation::GoForward { id } => self.go_forward(&id),
                    WebViewOperation::Reload { id } => self.reload(&id),
                    WebViewOperation::Stop { id } => self.stop(&id),
                    WebViewOperation::SetBounds { id, bounds } => self.set_bounds(&id, bounds),
                    WebViewOperation::SetVisible { id, visible } => self.set_visible(&id, visible),
                    WebViewOperation::Focus { id } => self.focus(&id),
                    WebViewOperation::Eval { id, script } => self.eval(&id, &script),
                    WebViewOperation::SetZoom { id, level } => self.set_zoom(&id, level),
                    WebViewOperation::SetUserAgent { id, user_agent } => self.set_user_agent(&id, &user_agent),
                    WebViewOperation::SetMainBounds { bounds } => self.set_main_bounds(bounds),
                    WebViewOperation::GrantPermission { .. } => Ok(()), // TODO: Implement
                };
                
                if let Err(e) = result {
                    eprintln!("[WebView] Operation error: {}", e);
                }
            }
        }

        fn create_webview(&mut self, window: &tao::window::Window, config: WebViewConfig) -> Result<(), String> {
            if self.instances.contains_key(&config.id) {
                return Err(format!("WebView '{}' already exists in native", config.id));
            }
            
            let id = config.id.clone();
            let id_for_nav = id.clone();
            let id_for_title = id.clone();
            
            let mut builder = WebViewBuilder::new()
                .with_bounds(Rect {
                    position: wry::dpi::Position::Logical(LogicalPosition::new(
                        config.bounds.x as f64,
                        config.bounds.y as f64
                    )),
                    size: wry::dpi::Size::Logical(LogicalSize::new(
                        config.bounds.width as f64,
                        config.bounds.height as f64
                    )),
                })
                .with_transparent(config.transparent)
                .with_devtools(config.devtools)
                .with_visible(config.visible)
                .with_autoplay(config.autoplay);
            
            // Set URL or HTML
            if let Some(ref html) = config.html {
                builder = builder.with_html(html);
            } else {
                builder = builder.with_url(&config.url);
            }
            
            // Navigation handler - fires when URL changes
            builder = builder.with_navigation_handler(move |url| {
                update_url(&id_for_nav, &url);
                push_event(WebViewEvent::NavigationStarted {
                    id: id_for_nav.clone(),
                    url: url.clone(),
                });
                true // Allow navigation
            });
            
            // Document title changed handler
            builder = builder.with_document_title_changed_handler(move |title| {
                update_title(&id_for_title, &title);
            });
            
            // New window handler (target="_blank" etc.)
            let id_for_new_window = id.clone();
            builder = builder.with_new_window_req_handler(move |url| {
                push_event(WebViewEvent::NewWindowRequested {
                    id: id_for_new_window.clone(),
                    url: url.clone(),
                    target: "_blank".to_string(),
                });
                false // Don't open automatically, let user handle it
            });
            
            // Download handler
            let id_for_download = id.clone();
            builder = builder.with_download_started_handler(move |url, filename| {
                push_event(WebViewEvent::DownloadRequested {
                    id: id_for_download.clone(),
                    url: url.to_string(),
                    filename: filename.to_string_lossy().to_string(),
                });
                true // Allow download
            });
            
            let webview = builder
                .build(window)
                .map_err(|e| e.to_string())?;
            
            self.instances.insert(id.clone(), webview);
            println!("[WebView] Created '{}'", id);
            
            Ok(())
        }

        fn destroy_webview(&mut self, id: &str) -> Result<(), String> {
            if self.instances.remove(id).is_some() {
                push_event(WebViewEvent::Closed { id: id.to_string() });
                println!("[WebView] Destroyed '{}'", id);
                Ok(())
            } else {
                Err(format!("WebView '{}' not found", id))
            }
        }

        fn navigate(&self, id: &str, url: &str) -> Result<(), String> {
            let wv = self.instances.get(id)
                .ok_or_else(|| format!("WebView '{}' not found", id))?;
            
            update_loading(id, true);
            wv.load_url(url).map_err(|e| e.to_string())?;
            
            // Navigation finished will be detected via navigation_handler
            Ok(())
        }

        fn load_html(&self, id: &str, html: &str) -> Result<(), String> {
            let wv = self.instances.get(id)
                .ok_or_else(|| format!("WebView '{}' not found", id))?;
            
            update_loading(id, true);
            wv.load_html(html).map_err(|e| e.to_string())?;
            update_loading(id, false);
            
            Ok(())
        }

        fn go_back(&self, id: &str) -> Result<(), String> {
            let _wv = self.instances.get(id)
                .ok_or_else(|| format!("WebView '{}' not found", id))?;
            
            // Note: wry doesn't expose go_back directly, need to use eval
            self.eval(id, "history.back()")
        }

        fn go_forward(&self, id: &str) -> Result<(), String> {
            let _wv = self.instances.get(id)
                .ok_or_else(|| format!("WebView '{}' not found", id))?;
            
            self.eval(id, "history.forward()")
        }

        fn reload(&self, id: &str) -> Result<(), String> {
            let _wv = self.instances.get(id)
                .ok_or_else(|| format!("WebView '{}' not found", id))?;
            
            update_loading(id, true);
            self.eval(id, "location.reload()")
        }

        fn stop(&self, id: &str) -> Result<(), String> {
            let _wv = self.instances.get(id)
                .ok_or_else(|| format!("WebView '{}' not found", id))?;
            
            self.eval(id, "window.stop()")?;
            update_loading(id, false);
            Ok(())
        }

        fn set_bounds(&self, id: &str, bounds: WebViewBounds) -> Result<(), String> {
            let wv = self.instances.get(id)
                .ok_or_else(|| format!("WebView '{}' not found", id))?;
            
            wv.set_bounds(Rect {
                position: wry::dpi::Position::Logical(LogicalPosition::new(
                    bounds.x as f64,
                    bounds.y as f64
                )),
                size: wry::dpi::Size::Logical(LogicalSize::new(
                    bounds.width as f64,
                    bounds.height as f64
                )),
            }).map_err(|e| e.to_string())
        }

        fn set_visible(&self, id: &str, visible: bool) -> Result<(), String> {
            let wv = self.instances.get(id)
                .ok_or_else(|| format!("WebView '{}' not found", id))?;
            
            wv.set_visible(visible).map_err(|e| e.to_string())
        }

        fn focus(&self, id: &str) -> Result<(), String> {
            let wv = self.instances.get(id)
                .ok_or_else(|| format!("WebView '{}' not found", id))?;
            
            wv.focus().map_err(|e| e.to_string())
        }

        fn eval(&self, id: &str, script: &str) -> Result<(), String> {
            let wv = self.instances.get(id)
                .ok_or_else(|| format!("WebView '{}' not found", id))?;
            
            wv.evaluate_script(script).map_err(|e| e.to_string())
        }

        fn set_zoom(&self, id: &str, level: f64) -> Result<(), String> {
            let wv = self.instances.get(id)
                .ok_or_else(|| format!("WebView '{}' not found", id))?;
            
            wv.zoom(level).map_err(|e| e.to_string())
        }

        fn set_user_agent(&self, _id: &str, _user_agent: &str) -> Result<(), String> {
            // Note: User agent must be set at creation time in wry
            // This is a limitation - we can't change it after creation
            Err("User agent can only be set at WebView creation time".to_string())
        }

        fn set_main_bounds(&self, bounds: WebViewBounds) -> Result<(), String> {
            if let Some(ref wv) = self.main_webview {
                wv.set_bounds(Rect {
                    position: wry::dpi::Position::Logical(LogicalPosition::new(
                        bounds.x as f64,
                        bounds.y as f64
                    )),
                    size: wry::dpi::Size::Logical(LogicalSize::new(
                        bounds.width as f64,
                        bounds.height as f64
                    )),
                }).map_err(|e| e.to_string())
            } else {
                Err("Main WebView not set".to_string())
            }
        }

        /// Handle window resize - resize all WebViews proportionally
        pub fn handle_resize(&self, _width: u32, _height: u32) {
            // User is responsible for handling resize via JavaScript
            // They can listen to window resize events and call setBounds
        }
    }

    pub fn has_pending_operations() -> bool {
        crate::webview::has_pending_operations()
    }

    // ============================================
    // Browser Window (Two-WebView Setup)
    // ============================================

    use std::sync::Mutex;
    lazy_static::lazy_static! {
        static ref NAV_QUEUE: Mutex<Vec<String>> = Mutex::new(Vec::new());
        static ref UI_EVENT_QUEUE: Mutex<Vec<String>> = Mutex::new(Vec::new());
    }

    /// Run browser window with separate UI and content WebViews
    /// Uses child window approach for Windows compatibility
    pub fn run_browser_window(config: BrowserConfig) -> Result<(), Box<dyn std::error::Error>> {
        use tao::{
            event::{Event, WindowEvent},
            event_loop::{ControlFlow, EventLoop},
            window::WindowBuilder,
        };
        
        let event_loop = EventLoop::new();
        
        let mut builder = WindowBuilder::new()
            .with_title(&config.title)
            .with_inner_size(tao::dpi::LogicalSize::new(config.width as f64, config.height as f64))
            .with_resizable(true)
            .with_decorations(config.decorations);
        
        if let Some(ref icon_path) = config.icon_path {
            if let Ok(icon) = load_icon(icon_path) {
                builder = builder.with_window_icon(Some(icon));
            }
        }
        
        let window = builder.build(&event_loop)?;
        let window = std::sync::Arc::new(window);
        let window_for_ipc = std::sync::Arc::clone(&window);
        let window_for_title = std::sync::Arc::clone(&window);
        
        let ui_height = config.ui_height;
        let content_height = config.height.saturating_sub(ui_height);
        
        println!("[Browser] Creating content WebView FIRST at y={}, height={}", ui_height, content_height);
        println!("[Browser] Start URL: {}", config.start_url);
        
        // Create content WebView FIRST (will be below in z-order on Windows)
        let content_webview = WebViewBuilder::new()
            .with_url(&config.start_url)
            .with_bounds(Rect {
                position: wry::dpi::Position::Logical(LogicalPosition::new(0.0, ui_height as f64)),
                size: wry::dpi::Size::Logical(LogicalSize::new(config.width as f64, content_height as f64)),
            })
            .with_devtools(config.devtools)
            .with_transparent(false)
            // Navigation started handler
            .with_navigation_handler(move |url| {
                println!("[Browser] Navigation started: {}", url);
                let event = format!("navstart:{}", url);
                UI_EVENT_QUEUE.lock().unwrap().push(event);
                true // Allow navigation
            })
            // Page title changed handler
            .with_document_title_changed_handler(move |title| {
                println!("[Browser] Title changed: {}", title);
                window_for_title.set_title(&title);
                let event = format!("title:{}", title);
                UI_EVENT_QUEUE.lock().unwrap().push(event);
            })
            .build(&window)?;
        
        println!("[Browser] Content WebView created");
        
        println!("[Browser] Creating UI WebView SECOND (height={})", ui_height);
        
        // Create UI WebView SECOND (will be on top in z-order on Windows)
        let ui_builder = if config.ui_html.starts_with("http://") || config.ui_html.starts_with("https://") {
            WebViewBuilder::new().with_url(&config.ui_html)
        } else {
            WebViewBuilder::new().with_html(&config.ui_html)
        };
        
        let ui_webview = ui_builder
            .with_bounds(Rect {
                position: wry::dpi::Position::Logical(LogicalPosition::new(0.0, 0.0)),
                size: wry::dpi::Size::Logical(LogicalSize::new(config.width as f64, ui_height as f64)),
            })
            .with_devtools(config.devtools)
            .with_transparent(false)
            .with_background_color((26, 26, 31, 255))
            .with_ipc_handler(move |msg: wry::http::Request<String>| {
                let body = msg.body();
                if body.starts_with("navigate:") {
                    let url = body.strip_prefix("navigate:").unwrap_or("about:blank");
                    NAV_QUEUE.lock().unwrap().push(url.to_string());
                } else if body == "poll" {
                    // UI is polling for events - handled in event loop
                } else {
                    match body.as_str() {
                        "minimize" => window_for_ipc.set_minimized(true),
                        "maximize" => window_for_ipc.set_maximized(!window_for_ipc.is_maximized()),
                        "close" => std::process::exit(0),
                        "drag" => { let _ = window_for_ipc.drag_window(); }
                        _ => {}
                    }
                }
            })
            .build(&window)?;
        
        println!("[Browser] UI WebView created");
        
        // Trigger initial layout by setting bounds explicitly
        let size = window.inner_size();
        let scale = window.scale_factor();
        let physical_ui_height = (ui_height as f64 * scale) as u32;
        let physical_content_height = size.height.saturating_sub(physical_ui_height);
        
        let _ = ui_webview.set_bounds(Rect {
            position: wry::dpi::Position::Physical(wry::dpi::PhysicalPosition::new(0, 0)),
            size: wry::dpi::Size::Physical(wry::dpi::PhysicalSize::new(size.width, physical_ui_height)),
        });
        let _ = content_webview.set_bounds(Rect {
            position: wry::dpi::Position::Physical(wry::dpi::PhysicalPosition::new(0, physical_ui_height as i32)),
            size: wry::dpi::Size::Physical(wry::dpi::PhysicalSize::new(size.width, physical_content_height)),
        });
        
        println!("[Browser] Initial bounds set");
        
        let mut last_url = config.start_url.clone();
        
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            
            // Process navigation queue
            if let Ok(mut queue) = NAV_QUEUE.lock() {
                for url in queue.drain(..) {
                    println!("[Browser] Navigating to: {}", url);
                    let _ = content_webview.load_url(&url);
                }
            }
            
            // Send UI events to UI WebView
            if let Ok(mut events) = UI_EVENT_QUEUE.lock() {
                for event in events.drain(..) {
                    if event.starts_with("navstart:") {
                        let url = event.strip_prefix("navstart:").unwrap_or("");
                        last_url = url.to_string();
                        let script = format!("if(window.onNavStart)window.onNavStart('{}');", url.replace("'", "\\'"));
                        let _ = ui_webview.evaluate_script(&script);
                    } else if event.starts_with("title:") {
                        let title = event.strip_prefix("title:").unwrap_or("");
                        // Title change often means page loaded
                        let script = format!(
                            "if(window.onTitleChange)window.onTitleChange('{}');if(window.onLoadEnd)window.onLoadEnd('{}');",
                            title.replace("'", "\\'"),
                            last_url.replace("'", "\\'")
                        );
                        let _ = ui_webview.evaluate_script(&script);
                    }
                }
            }
            
            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                    let scale = window.scale_factor();
                    let physical_ui_height = (ui_height as f64 * scale) as u32;
                    let new_content_height = size.height.saturating_sub(physical_ui_height);
                    
                    let _ = ui_webview.set_bounds(Rect {
                        position: wry::dpi::Position::Physical(wry::dpi::PhysicalPosition::new(0, 0)),
                        size: wry::dpi::Size::Physical(wry::dpi::PhysicalSize::new(size.width, physical_ui_height)),
                    });
                    let _ = content_webview.set_bounds(Rect {
                        position: wry::dpi::Position::Physical(wry::dpi::PhysicalPosition::new(0, physical_ui_height as i32)),
                        size: wry::dpi::Size::Physical(wry::dpi::PhysicalSize::new(size.width, new_content_height)),
                    });
                }
                Event::MainEventsCleared => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                _ => {}
            }
        });
    }

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
}

// Re-export native items
#[cfg(feature = "native")]
pub use native_impl::{WebViewManager, run_browser_window, has_pending_operations};

// ============================================
// Stubs for non-native builds
// ============================================

#[cfg(not(feature = "native"))]
pub struct WebViewManager;

#[cfg(not(feature = "native"))]
impl WebViewManager {
    pub fn new() -> Self { Self }
    pub fn set_main_webview(&mut self, _: ()) {}
    pub fn main_webview(&self) -> Option<&()> { None }
    pub fn process_pending_operations(&mut self, _: &()) {}
    pub fn handle_resize(&self, _: u32, _: u32) {}
}

#[cfg(not(feature = "native"))]
pub fn run_browser_window(_: BrowserConfig) -> Result<(), Box<dyn std::error::Error>> {
    Err("Native feature not enabled".into())
}

#[cfg(not(feature = "native"))]
pub fn has_pending_operations() -> bool { false }

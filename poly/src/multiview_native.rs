//! Native implementation for Multi-WebView Windows
//!
//! Creates windows with multiple WebViews that can communicate with each other.
//! 
//! IMPORTANT: On Windows/WebView2, WebViews are stacked in creation order.
//! The LAST created WebView appears ON TOP. So create bottom views first, top views last.

#[cfg(feature = "native")]
mod native_impl {
    #[allow(unused_imports)]
    use crate::multiview::{MultiViewWindowConfig, ViewConfig, MultiViewOperation};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::thread;
    
    /// Handle to a multi-view window
    pub struct MultiViewWindow {
        pub id: u64,
        #[allow(dead_code)]
        views: HashMap<String, ViewHandle>,
    }
    
    struct ViewHandle {
        // We can't store WebView directly due to thread safety
        // Instead we use message passing
    }
    
    /// Message queue for view operations
    use once_cell::sync::Lazy;
    
    // Per-window navigation queues
    pub static NAV_QUEUES: Lazy<Arc<Mutex<HashMap<u64, HashMap<String, Vec<String>>>>>> = Lazy::new(|| {
        Arc::new(Mutex::new(HashMap::new()))
    });
    
    /// Create and run a multi-view window in a new thread
    pub fn create_multiview_window(id: u64, config: MultiViewWindowConfig) -> Result<(), String> {
        // Initialize nav queue for this window
        NAV_QUEUES.lock().unwrap().insert(id, HashMap::new());
        for view in &config.views {
            NAV_QUEUES.lock().unwrap()
                .get_mut(&id).unwrap()
                .insert(view.id.clone(), Vec::new());
        }
        
        // Spawn window in new thread
        thread::spawn(move || {
            if let Err(e) = run_multiview_window(id, config) {
                eprintln!("[MultiView] Window {} error: {}", id, e);
            }
        });
        
        Ok(())
    }
    
    /// Run the multi-view window (blocking, runs in its own thread)
    fn run_multiview_window(window_id: u64, config: MultiViewWindowConfig) -> Result<(), Box<dyn std::error::Error>> {
        use tao::{
            event::{Event, WindowEvent},
            event_loop::{ControlFlow, EventLoopBuilder},
            window::WindowBuilder,
            platform::windows::EventLoopBuilderExtWindows,
        };
        use wry::{WebViewBuilder, Rect};
        use wry::dpi::{LogicalPosition, LogicalSize};
        
        // Use any_thread to allow creating event loop in non-main thread
        let event_loop = EventLoopBuilder::new().with_any_thread(true).build();
        
        let mut builder = WindowBuilder::new()
            .with_title(&config.title)
            .with_inner_size(tao::dpi::LogicalSize::new(config.width as f64, config.height as f64))
            .with_resizable(config.resizable)
            .with_decorations(config.decorations);
        
        // Load icon if specified
        if let Some(ref icon_path) = config.icon_path {
            if let Ok(icon) = load_icon(icon_path) {
                builder = builder.with_window_icon(Some(icon));
            }
        }
        
        let window = builder.build(&event_loop)?;
        let window = Arc::new(window);
        
        // Sort views by z-order: we want views with higher y to be created first (bottom)
        // and views with lower y to be created last (top, for UI)
        let mut sorted_views = config.views.clone();
        sorted_views.sort_by(|a, b| b.y.cmp(&a.y)); // Higher y first = bottom first
        
        println!("[MultiView] Creating {} views for window {}", sorted_views.len(), window_id);
        
        // Create WebViews in sorted order (bottom to top)
        let mut webviews: HashMap<String, wry::WebView> = HashMap::new();
        
        for view_config in &sorted_views {
            let window_clone = Arc::clone(&window);
            let view_id = view_config.id.clone();
            let wid = window_id;
            
            let webview_builder = if let Some(ref html) = view_config.html {
                WebViewBuilder::new().with_html(html)
            } else {
                WebViewBuilder::new().with_url(&view_config.url)
            };
            
            let webview = webview_builder
                .with_bounds(Rect {
                    position: wry::dpi::Position::Logical(LogicalPosition::new(
                        view_config.x as f64, 
                        view_config.y as f64
                    )),
                    size: wry::dpi::Size::Logical(LogicalSize::new(
                        view_config.width as f64, 
                        view_config.height as f64
                    )),
                })
                .with_transparent(view_config.transparent)
                .with_devtools(view_config.devtools)
                .with_background_color((26, 26, 31, 255))
                .with_ipc_handler(move |msg: wry::http::Request<String>| {
                    let body = msg.body();
                    handle_view_ipc(wid, &view_id, body, &window_clone);
                })
                .build(&window)?;
            
            println!("[MultiView] Created view '{}' at ({}, {}) size {}x{}", 
                view_config.id, view_config.x, view_config.y, view_config.width, view_config.height);
            
            webviews.insert(view_config.id.clone(), webview);
        }
        
        let window_width = config.width;
        let window_height = config.height;
        let views_config = config.views.clone();
        
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            
            // Process navigation queue for this window
            if let Ok(mut queues) = NAV_QUEUES.try_lock() {
                if let Some(window_queues) = queues.get_mut(&window_id) {
                    for (view_id, urls) in window_queues.iter_mut() {
                        for url in urls.drain(..) {
                            if let Some(wv) = webviews.get(view_id) {
                                println!("[MultiView] Navigating '{}' to: {}", view_id, url);
                                let _ = wv.load_url(&url);
                            }
                        }
                    }
                }
            }
            
            // Process pending operations
            let ops = crate::multiview::take_operations();
            for op in ops {
                match op {
                    MultiViewOperation::Navigate { window_id: wid, view_id, url } if wid == window_id => {
                        if let Some(wv) = webviews.get(&view_id) {
                            println!("[MultiView] Navigate '{}' to: {}", view_id, url);
                            let _ = wv.load_url(&url);
                        }
                    }
                    MultiViewOperation::PostMessage { window_id: wid, view_id, message } if wid == window_id => {
                        if let Some(wv) = webviews.get(&view_id) {
                            let js = format!("window.dispatchEvent(new CustomEvent('polymessage', {{ detail: {} }}));", message);
                            let _ = wv.evaluate_script(&js);
                        }
                    }
                    MultiViewOperation::SetBounds { window_id: wid, view_id, x, y, width, height } if wid == window_id => {
                        if let Some(wv) = webviews.get(&view_id) {
                            let _ = wv.set_bounds(Rect {
                                position: wry::dpi::Position::Logical(LogicalPosition::new(x as f64, y as f64)),
                                size: wry::dpi::Size::Logical(LogicalSize::new(width as f64, height as f64)),
                            });
                        }
                    }
                    MultiViewOperation::Close { window_id: wid } if wid == window_id => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            }
            
            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    // Clean up
                    NAV_QUEUES.lock().unwrap().remove(&window_id);
                    crate::multiview::MULTIVIEW_WINDOWS.lock().unwrap().remove(&window_id);
                    *control_flow = ControlFlow::Exit;
                }
                Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                    // Resize views proportionally
                    let scale_x = size.width as f64 / window_width as f64;
                    let scale_y = size.height as f64 / window_height as f64;
                    
                    for view_config in &views_config {
                        if let Some(wv) = webviews.get(&view_config.id) {
                            let new_x = (view_config.x as f64 * scale_x) as i32;
                            let new_y = (view_config.y as f64 * scale_y) as i32;
                            let new_w = (view_config.width as f64 * scale_x) as u32;
                            let new_h = (view_config.height as f64 * scale_y) as u32;
                            
                            let _ = wv.set_bounds(Rect {
                                position: wry::dpi::Position::Logical(LogicalPosition::new(new_x as f64, new_y as f64)),
                                size: wry::dpi::Size::Physical(wry::dpi::PhysicalSize::new(new_w, new_h)),
                            });
                        }
                    }
                }
                Event::MainEventsCleared => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                _ => {}
            }
        });
    }
    
    /// Handle IPC messages from a view
    fn handle_view_ipc(window_id: u64, view_id: &str, message: &str, window: &tao::window::Window) {
        // Check for special commands
        if message.starts_with("navigate:") {
            // Navigate another view: "navigate:viewId:url"
            let rest = message.strip_prefix("navigate:").unwrap_or("");
            if let Some((target_view, url)) = rest.split_once(':') {
                if let Ok(mut queues) = NAV_QUEUES.try_lock() {
                    if let Some(window_queues) = queues.get_mut(&window_id) {
                        if let Some(queue) = window_queues.get_mut(target_view) {
                            queue.push(url.to_string());
                        }
                    }
                }
            }
        } else {
            // Standard window commands
            match message {
                "minimize" => window.set_minimized(true),
                "maximize" => window.set_maximized(!window.is_maximized()),
                "close" => {
                    NAV_QUEUES.lock().unwrap().remove(&window_id);
                    crate::multiview::MULTIVIEW_WINDOWS.lock().unwrap().remove(&window_id);
                    std::process::exit(0);
                }
                "drag" => { let _ = window.drag_window(); }
                _ => {
                    println!("[MultiView] View '{}' sent: {}", view_id, message);
                }
            }
        }
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
    
    /// Queue navigation for a view
    pub fn queue_navigation(window_id: u64, view_id: &str, url: &str) {
        if let Ok(mut queues) = NAV_QUEUES.lock() {
            if let Some(window_queues) = queues.get_mut(&window_id) {
                if let Some(queue) = window_queues.get_mut(view_id) {
                    queue.push(url.to_string());
                }
            }
        }
    }
}

#[cfg(feature = "native")]
pub use native_impl::*;

// Stubs for non-native builds
#[cfg(not(feature = "native"))]
pub fn create_multiview_window(_id: u64, _config: crate::multiview::MultiViewWindowConfig) -> Result<(), String> {
    Err("Native feature not enabled".to_string())
}

#[cfg(not(feature = "native"))]
pub fn queue_navigation(_window_id: u64, _view_id: &str, _url: &str) {}

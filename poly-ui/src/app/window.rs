//! Window management and event loop

use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId, WindowAttributes},
    dpi::LogicalSize,
};
use std::sync::Arc;
use crate::render::{GpuRenderer, RenderList, Primitive};
use crate::core::context::Color;

/// Window configuration
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub decorations: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Poly UI".to_string(),
            width: 800,
            height: 600,
            resizable: true,
            decorations: true,
        }
    }
}

/// Application state during runtime
struct AppState {
    window: Arc<Window>,
    renderer: GpuRenderer,
    render_list: RenderList,
    ui_builder: Box<dyn Fn(&mut RenderList, u32, u32) + Send>,
}

/// Main application handler
pub struct PolyApp {
    config: WindowConfig,
    state: Option<AppState>,
    ui_builder: Option<Box<dyn Fn(&mut RenderList, u32, u32) + Send>>,
}

impl PolyApp {
    pub fn new(config: WindowConfig) -> Self {
        Self {
            config,
            state: None,
            ui_builder: None,
        }
    }
    
    pub fn with_ui<F>(mut self, builder: F) -> Self 
    where F: Fn(&mut RenderList, u32, u32) + Send + 'static {
        self.ui_builder = Some(Box::new(builder));
        self
    }
    
    pub fn run(self) {
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        event_loop.set_control_flow(ControlFlow::Wait);
        
        let mut app = self;
        event_loop.run_app(&mut app).expect("Event loop error");
    }
}

impl ApplicationHandler for PolyApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        
        let window_attrs = WindowAttributes::default()
            .with_title(&self.config.title)
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height))
            .with_resizable(self.config.resizable)
            .with_decorations(self.config.decorations);
        
        let window = Arc::new(
            event_loop.create_window(window_attrs).expect("Failed to create window")
        );
        
        let renderer = pollster::block_on(GpuRenderer::new(window.clone()));
        
        let ui_builder = self.ui_builder.take().unwrap_or_else(|| {
            Box::new(|_: &mut RenderList, _: u32, _: u32| {})
        });
        
        self.state = Some(AppState {
            window,
            renderer,
            render_list: RenderList::new(),
            ui_builder,
        });
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = &mut self.state else { return };
        
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                state.renderer.resize((physical_size.width, physical_size.height));
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                // Build UI
                state.render_list.clear();
                let size = state.renderer.size;
                (state.ui_builder)(&mut state.render_list, size.0, size.1);
                
                // Render
                match state.renderer.render(&state.render_list) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => {
                        state.renderer.resize(state.renderer.size);
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        event_loop.exit();
                    }
                    Err(e) => eprintln!("Render error: {:?}", e),
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                // Handle mouse move
                let _ = position;
            }
            WindowEvent::MouseInput { state: button_state, button, .. } => {
                // Handle mouse click
                let _ = (button_state, button);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // Handle keyboard
                let _ = event;
            }
            _ => {}
        }
    }
    
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}

/// Run a simple demo window
pub fn run_demo() {
    let config = WindowConfig {
        title: "Poly UI Demo".to_string(),
        width: 800,
        height: 600,
        ..Default::default()
    };
    
    PolyApp::new(config)
        .with_ui(|render_list, width, height| {
            // Background
            render_list.rect(0.0, 0.0, width as f32, height as f32, Color::rgb(25, 25, 35), 0.0);
            
            // Header bar
            render_list.rect(0.0, 0.0, width as f32, 60.0, Color::rgb(35, 35, 50), 0.0);
            
            // Main content area
            let padding = 20.0;
            let content_y = 80.0;
            
            // Card 1
            render_list.rect(padding, content_y, 200.0, 120.0, Color::rgb(45, 45, 65), 12.0);
            
            // Card 2
            render_list.rect(padding + 220.0, content_y, 200.0, 120.0, Color::rgb(45, 45, 65), 12.0);
            
            // Card 3
            render_list.rect(padding + 440.0, content_y, 200.0, 120.0, Color::rgb(45, 45, 65), 12.0);
            
            // Button
            render_list.rect(padding, content_y + 140.0, 120.0, 44.0, Color::rgb(0, 122, 255), 8.0);
            
            // Secondary button
            render_list.rect(padding + 140.0, content_y + 140.0, 120.0, 44.0, Color::rgb(88, 86, 214), 8.0);
            
            // Circle decoration
            render_list.primitives.push(Primitive::Circle {
                cx: width as f32 - 100.0,
                cy: height as f32 - 100.0,
                radius: 60.0,
                color: Color::rgba(0, 122, 255, 0.3),
            });
        })
        .run();
}

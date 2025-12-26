//! Application entry point

mod window;
mod hot_reload;

pub use window::*;
pub use hot_reload::*;

use crate::core::{Widget, Context, BoxedWidget};
use crate::render::Renderer;
use crate::layout::LayoutEngine;
use crate::core::context::Color;

/// Main application struct
pub struct App {
    title: String,
    width: u32,
    height: u32,
    root: Option<BoxedWidget>,
    context: Context,
    #[allow(dead_code)]
    renderer: Renderer,
    #[allow(dead_code)]
    layout_engine: LayoutEngine,
}

impl App {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 800,
            height: 600,
            root: None,
            context: Context::new(800.0, 600.0),
            renderer: Renderer::new(),
            layout_engine: LayoutEngine::new(),
        }
    }
    
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self.context = Context::new(width as f32, height as f32);
        self
    }
    
    pub fn root(mut self, widget: impl Widget + 'static) -> Self {
        self.root = Some(Box::new(widget));
        self
    }
    
    /// Run the application with GPU rendering
    pub fn run(self) {
        let config = WindowConfig {
            title: self.title.clone(),
            width: self.width,
            height: self.height,
            ..Default::default()
        };
        
        PolyApp::new(config)
            .with_ui(move |render_list, width, height| {
                // Background
                render_list.rect(0.0, 0.0, width as f32, height as f32, Color::rgb(18, 18, 24), 0.0);
                
                // Demo UI - will be replaced with actual widget rendering
                let padding = 20.0;
                
                // Title area
                render_list.rect(padding, padding, width as f32 - padding * 2.0, 50.0, Color::rgb(30, 30, 40), 8.0);
                
                // Content cards
                let card_width = (width as f32 - padding * 4.0) / 3.0;
                for i in 0..3 {
                    let x = padding + (card_width + padding) * i as f32;
                    render_list.rect(x, 90.0, card_width, 150.0, Color::rgb(35, 35, 50), 12.0);
                }
                
                // Buttons
                render_list.rect(padding, 260.0, 140.0, 44.0, Color::rgb(0, 122, 255), 8.0);
                render_list.rect(padding + 160.0, 260.0, 140.0, 44.0, Color::rgb(88, 86, 214), 8.0);
                render_list.rect(padding + 320.0, 260.0, 140.0, 44.0, Color::rgb(255, 59, 48), 8.0);
            })
            .run();
    }
}

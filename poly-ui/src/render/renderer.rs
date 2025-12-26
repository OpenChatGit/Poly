//! GPU Renderer using wgpu

use crate::render::RenderList;
use crate::core::context::Color;

/// GPU-accelerated renderer
pub struct Renderer {
    clear_color: Color,
    // wgpu resources will be added here
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            clear_color: Color::rgb(18, 18, 18),
        }
    }
    
    pub fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }
    
    /// Render a frame
    pub fn render(&mut self, _render_list: &RenderList) {
        // TODO: Implement wgpu rendering
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

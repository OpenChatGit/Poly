//! Render primitives

use crate::core::context::Color;

/// A drawable primitive
#[derive(Debug, Clone)]
pub enum Primitive {
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        border_radius: f32,
    },
    Text {
        x: f32,
        y: f32,
        text: String,
        size: f32,
        color: Color,
    },
    Image {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        texture_id: u32,
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        width: f32,
    },
    Circle {
        cx: f32,
        cy: f32,
        radius: f32,
        color: Color,
    },
}

/// Render command list
#[derive(Default)]
pub struct RenderList {
    pub primitives: Vec<Primitive>,
}

impl RenderList {
    pub fn new() -> Self {
        Self { primitives: Vec::new() }
    }
    
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color, radius: f32) {
        self.primitives.push(Primitive::Rect {
            x, y, width: w, height: h, color, border_radius: radius
        });
    }
    
    pub fn text(&mut self, x: f32, y: f32, text: String, size: f32, color: Color) {
        self.primitives.push(Primitive::Text { x, y, text, size, color });
    }
    
    pub fn clear(&mut self) {
        self.primitives.clear();
    }
}

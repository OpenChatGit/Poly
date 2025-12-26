//! Layout engine using Taffy (Flexbox)

use crate::core::WidgetId;

/// Layout node with computed position and size
#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub id: WidgetId,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub children: Vec<LayoutNode>,
}

impl LayoutNode {
    pub fn new(id: WidgetId, width: f32, height: f32) -> Self {
        Self {
            id,
            x: 0.0,
            y: 0.0,
            width,
            height,
            children: Vec::new(),
        }
    }
    
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width &&
        y >= self.y && y <= self.y + self.height
    }
}

/// Layout engine wrapper around Taffy
pub struct LayoutEngine {
    // Will use taffy::Taffy internally
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {}
    }
    
    pub fn compute(&mut self, _root: &dyn crate::core::Widget, _width: f32, _height: f32) -> LayoutNode {
        // TODO: Implement with Taffy
        LayoutNode::new(0, 0.0, 0.0)
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

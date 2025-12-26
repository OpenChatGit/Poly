//! Build context for widgets

use crate::core::{Store, WidgetId};
use std::collections::HashMap;

/// Context passed to widgets during build
pub struct Context {
    /// Global state store
    pub store: Store,
    /// Widget tree for lookups
    widget_tree: HashMap<WidgetId, WidgetInfo>,
    /// Widgets that need rebuild
    dirty_widgets: Vec<WidgetId>,
    /// Current theme
    pub theme: Theme,
    /// Screen dimensions
    pub screen_width: f32,
    pub screen_height: f32,
}

#[derive(Debug, Clone)]
pub struct WidgetInfo {
    pub id: WidgetId,
    pub parent_id: Option<WidgetId>,
    pub children: Vec<WidgetId>,
}

impl Context {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            store: Store::new(),
            widget_tree: HashMap::new(),
            dirty_widgets: Vec::new(),
            theme: Theme::dark(),
            screen_width: width,
            screen_height: height,
        }
    }
    
    /// Mark a widget as needing rebuild
    pub fn mark_dirty(&mut self, id: WidgetId) {
        if !self.dirty_widgets.contains(&id) {
            self.dirty_widgets.push(id);
        }
    }
    
    /// Get dirty widgets and clear the list
    pub fn take_dirty(&mut self) -> Vec<WidgetId> {
        std::mem::take(&mut self.dirty_widgets)
    }
    
    /// Register a widget in the tree
    pub fn register_widget(&mut self, id: WidgetId, parent_id: Option<WidgetId>) {
        self.widget_tree.insert(id, WidgetInfo {
            id,
            parent_id,
            children: Vec::new(),
        });
        
        if let Some(pid) = parent_id {
            if let Some(parent) = self.widget_tree.get_mut(&pid) {
                parent.children.push(id);
            }
        }
    }
}

/// Theme configuration
#[derive(Debug, Clone)]
pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub background: Color,
    pub surface: Color,
    pub text: Color,
    pub text_secondary: Color,
    pub error: Color,
    pub border_radius: f32,
    pub spacing: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }
    
    pub const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a,
        }
    }
    
    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            primary: Color::rgb(0, 122, 255),      // Blue
            secondary: Color::rgb(88, 86, 214),    // Purple
            background: Color::rgb(18, 18, 18),    // Dark gray
            surface: Color::rgb(30, 30, 30),       // Lighter gray
            text: Color::rgb(255, 255, 255),       // White
            text_secondary: Color::rgb(160, 160, 160),
            error: Color::rgb(255, 69, 58),        // Red
            border_radius: 8.0,
            spacing: 8.0,
        }
    }
    
    pub fn light() -> Self {
        Self {
            primary: Color::rgb(0, 122, 255),
            secondary: Color::rgb(88, 86, 214),
            background: Color::rgb(255, 255, 255),
            surface: Color::rgb(242, 242, 247),
            text: Color::rgb(0, 0, 0),
            text_secondary: Color::rgb(100, 100, 100),
            error: Color::rgb(255, 59, 48),
            border_radius: 8.0,
            spacing: 8.0,
        }
    }
}

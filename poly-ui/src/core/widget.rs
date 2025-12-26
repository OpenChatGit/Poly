//! Widget trait and core widget types

use crate::layout::LayoutNode;
use crate::style::Style;
use crate::core::{Context, Event};

/// Unique identifier for widgets
pub type WidgetId = u64;

/// Core trait that all UI components implement
pub trait Widget: Send + Sync {
    /// Returns the widget's unique identifier
    fn id(&self) -> WidgetId;
    
    /// Build the widget tree - returns child widgets
    fn build(&self, ctx: &mut Context) -> Vec<Box<dyn Widget>>;
    
    /// Get the widget's style
    fn style(&self) -> &Style;
    
    /// Handle events (clicks, keyboard, etc.)
    fn on_event(&mut self, event: &Event, ctx: &mut Context) -> bool {
        let _ = (event, ctx);
        false // Not handled by default
    }
    
    /// Called when widget is mounted
    fn on_mount(&mut self, ctx: &mut Context) {
        let _ = ctx;
    }
    
    /// Called when widget is unmounted
    fn on_unmount(&mut self, ctx: &mut Context) {
        let _ = ctx;
    }
    
    /// Layout calculation
    fn layout(&self, constraints: &LayoutConstraints) -> LayoutNode {
        LayoutNode::new(self.id(), constraints.max_width, constraints.max_height)
    }
}

/// Constraints passed during layout
#[derive(Debug, Clone, Copy)]
pub struct LayoutConstraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl LayoutConstraints {
    pub fn new(max_width: f32, max_height: f32) -> Self {
        Self {
            min_width: 0.0,
            max_width,
            min_height: 0.0,
            max_height,
        }
    }
    
    pub fn tight(width: f32, height: f32) -> Self {
        Self {
            min_width: width,
            max_width: width,
            min_height: height,
            max_height: height,
        }
    }
}

/// A boxed widget for dynamic dispatch
pub type BoxedWidget = Box<dyn Widget>;

/// Helper to generate unique widget IDs
pub fn next_widget_id() -> WidgetId {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

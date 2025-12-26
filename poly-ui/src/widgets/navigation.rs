//! Navigation widgets

use crate::core::{Widget, WidgetId, Context, next_widget_id, BoxedWidget};
use crate::core::context::Color;
use crate::style::Style;
use std::sync::Arc;

/// App bar / Header
pub struct AppBar {
    id: WidgetId,
    style: Style,
    #[allow(dead_code)]
    title: String,
    leading: Option<BoxedWidget>,
    actions: Vec<BoxedWidget>,
}

impl AppBar {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new()
                .height(56.0)
                .padding(16.0)
                .background(Color::rgb(30, 30, 40)),
            title: title.into(),
            leading: None,
            actions: Vec::new(),
        }
    }
    
    pub fn leading(mut self, widget: impl Widget + 'static) -> Self {
        self.leading = Some(Box::new(widget));
        self
    }
    
    pub fn action(mut self, widget: impl Widget + 'static) -> Self {
        self.actions.push(Box::new(widget));
        self
    }
    
    pub fn background(mut self, color: Color) -> Self {
        self.style.background = Some(color);
        self
    }
}

impl Widget for AppBar {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

/// Bottom navigation bar
pub struct BottomNav {
    id: WidgetId,
    style: Style,
    items: Vec<NavItem>,
    selected: usize,
    on_select: Option<Arc<dyn Fn(usize) + Send + Sync>>,
}

pub struct NavItem {
    pub icon: String,
    pub label: String,
}

impl BottomNav {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new()
                .height(56.0)
                .background(Color::rgb(25, 25, 35)),
            items: Vec::new(),
            selected: 0,
            on_select: None,
        }
    }
    
    pub fn item(mut self, icon: impl Into<String>, label: impl Into<String>) -> Self {
        self.items.push(NavItem { icon: icon.into(), label: label.into() });
        self
    }
    
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }
    
    pub fn on_select<F: Fn(usize) + Send + Sync + 'static>(mut self, handler: F) -> Self {
        self.on_select = Some(Arc::new(handler));
        self
    }
}

impl Default for BottomNav {
    fn default() -> Self { Self::new() }
}

impl Widget for BottomNav {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

/// Tab bar
pub struct TabBar {
    id: WidgetId,
    style: Style,
    tabs: Vec<String>,
    selected: usize,
    on_select: Option<Arc<dyn Fn(usize) + Send + Sync>>,
}

impl TabBar {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new().height(48.0),
            tabs: Vec::new(),
            selected: 0,
            on_select: None,
        }
    }
    
    pub fn tab(mut self, label: impl Into<String>) -> Self {
        self.tabs.push(label.into());
        self
    }
    
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }
    
    pub fn on_select<F: Fn(usize) + Send + Sync + 'static>(mut self, handler: F) -> Self {
        self.on_select = Some(Arc::new(handler));
        self
    }
}

impl Default for TabBar {
    fn default() -> Self { Self::new() }
}

impl Widget for TabBar {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

/// Drawer / Side panel
pub struct Drawer {
    id: WidgetId,
    style: Style,
    header: Option<BoxedWidget>,
    items: Vec<BoxedWidget>,
    width: f32,
}

impl Drawer {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new()
                .background(Color::rgb(30, 30, 40)),
            header: None,
            items: Vec::new(),
            width: 280.0,
        }
    }
    
    pub fn header(mut self, widget: impl Widget + 'static) -> Self {
        self.header = Some(Box::new(widget));
        self
    }
    
    pub fn item(mut self, widget: impl Widget + 'static) -> Self {
        self.items.push(Box::new(widget));
        self
    }
    
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
}

impl Default for Drawer {
    fn default() -> Self { Self::new() }
}

impl Widget for Drawer {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

//! Container widgets (Column, Row, Stack, etc.)

use crate::core::{Widget, WidgetId, Context, next_widget_id, BoxedWidget};
use crate::style::{Style, FlexDirection};

/// Vertical layout container (like Flutter's Column)
pub struct Column {
    id: WidgetId,
    style: Style,
    children: Vec<BoxedWidget>,
}

impl Column {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new().flex_direction(FlexDirection::Column),
            children: Vec::new(),
        }
    }
    
    pub fn with_children(mut self, children: Vec<BoxedWidget>) -> Self {
        self.children = children;
        self
    }
    
    pub fn child(mut self, child: impl Widget + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
    
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style.flex_direction(FlexDirection::Column);
        self
    }
    
    pub fn gap(mut self, gap: f32) -> Self {
        self.style.gap = gap;
        self
    }
    
    pub fn padding(mut self, padding: f32) -> Self {
        self.style.padding = crate::style::EdgeInsets::all(padding);
        self
    }
}

impl Default for Column {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Column {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> {
        // Return cloned children - in real impl would be more sophisticated
        Vec::new()
    }
}

/// Horizontal layout container (like Flutter's Row)
pub struct Row {
    id: WidgetId,
    style: Style,
    children: Vec<BoxedWidget>,
}

impl Row {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new().flex_direction(FlexDirection::Row),
            children: Vec::new(),
        }
    }
    
    pub fn with_children(mut self, children: Vec<BoxedWidget>) -> Self {
        self.children = children;
        self
    }
    
    pub fn child(mut self, child: impl Widget + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
    
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style.flex_direction(FlexDirection::Row);
        self
    }
    
    pub fn gap(mut self, gap: f32) -> Self {
        self.style.gap = gap;
        self
    }
}

impl Default for Row {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Row {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> {
        Vec::new()
    }
}

/// Stack container for overlapping widgets
pub struct Stack {
    id: WidgetId,
    style: Style,
    children: Vec<BoxedWidget>,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new(),
            children: Vec::new(),
        }
    }
    
    pub fn child(mut self, child: impl Widget + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Stack {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> {
        Vec::new()
    }
}

/// Generic container with custom styling
pub struct Container {
    id: WidgetId,
    style: Style,
    child: Option<BoxedWidget>,
}

impl Container {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new(),
            child: None,
        }
    }
    
    pub fn child(mut self, child: impl Widget + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }
    
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    
    pub fn width(mut self, w: f32) -> Self {
        self.style.width = crate::style::Dimension::Px(w);
        self
    }
    
    pub fn height(mut self, h: f32) -> Self {
        self.style.height = crate::style::Dimension::Px(h);
        self
    }
    
    pub fn background(mut self, color: crate::core::context::Color) -> Self {
        self.style.background = Some(color);
        self
    }
    
    pub fn padding(mut self, p: f32) -> Self {
        self.style.padding = crate::style::EdgeInsets::all(p);
        self
    }
    
    pub fn border_radius(mut self, r: f32) -> Self {
        self.style.border_radius = r;
        self
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Container {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> {
        Vec::new()
    }
}

/// Spacer widget for flexible spacing
pub struct Spacer {
    id: WidgetId,
    style: Style,
}

impl Spacer {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new().flex_grow(1.0),
        }
    }
}

impl Default for Spacer {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Spacer {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

/// Fixed-size gap
pub struct Gap {
    id: WidgetId,
    style: Style,
}

impl Gap {
    pub fn new(size: f32) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new().size(size, size),
        }
    }
    
    pub fn horizontal(size: f32) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new().width(size).height(0.0),
        }
    }
    
    pub fn vertical(size: f32) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new().width(0.0).height(size),
        }
    }
}

impl Widget for Gap {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

//! Progress indicators

use crate::core::{Widget, WidgetId, Context, next_widget_id, BoxedWidget};
use crate::core::context::Color;
use crate::style::Style;

/// Linear progress bar
pub struct ProgressBar {
    id: WidgetId,
    style: Style,
    value: f32,  // 0.0 to 1.0
    color: Color,
    background_color: Color,
}

impl ProgressBar {
    pub fn new(value: f32) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new().height(8.0).border_radius(4.0),
            value: value.clamp(0.0, 1.0),
            color: Color::rgb(0, 122, 255),
            background_color: Color::rgb(50, 50, 60),
        }
    }
    
    pub fn value(mut self, value: f32) -> Self {
        self.value = value.clamp(0.0, 1.0);
        self
    }
    
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
    
    pub fn height(mut self, h: f32) -> Self {
        self.style.height = crate::style::Dimension::Px(h);
        self.style.border_radius = h / 2.0;
        self
    }
    
    pub fn get_value(&self) -> f32 { self.value }
    pub fn get_color(&self) -> Color { self.color }
    pub fn get_background(&self) -> Color { self.background_color }
}

impl Widget for ProgressBar {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

/// Circular progress indicator
pub struct CircularProgress {
    id: WidgetId,
    style: Style,
    value: Option<f32>,  // None = indeterminate
    size: f32,
    stroke_width: f32,
    color: Color,
}

impl CircularProgress {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new().size(40.0, 40.0),
            value: None,
            size: 40.0,
            stroke_width: 4.0,
            color: Color::rgb(0, 122, 255),
        }
    }
    
    pub fn determinate(mut self, value: f32) -> Self {
        self.value = Some(value.clamp(0.0, 1.0));
        self
    }
    
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self.style = self.style.size(size, size);
        self
    }
    
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }
    
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl Default for CircularProgress {
    fn default() -> Self { Self::new() }
}

impl Widget for CircularProgress {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

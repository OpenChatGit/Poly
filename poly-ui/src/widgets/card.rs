//! Card and Panel widgets

use crate::core::{Widget, WidgetId, Context, next_widget_id, BoxedWidget};
use crate::core::context::Color;
use crate::style::{Style, Shadow};

/// Card widget with elevation and shadow
pub struct Card {
    id: WidgetId,
    style: Style,
    child: Option<BoxedWidget>,
    elevation: f32,
}

impl Card {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new()
                .padding(16.0)
                .border_radius(12.0)
                .background(Color::rgb(35, 35, 45)),
            child: None,
            elevation: 2.0,
        }
    }
    
    pub fn child(mut self, child: impl Widget + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }
    
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = elevation;
        self.style.shadow = Some(Shadow::new(0.0, elevation * 2.0, elevation * 4.0, Color::rgba(0, 0, 0, 0.3)));
        self
    }
    
    pub fn padding(mut self, p: f32) -> Self {
        self.style.padding = crate::style::EdgeInsets::all(p);
        self
    }
}

impl Default for Card {
    fn default() -> Self { Self::new() }
}

impl Widget for Card {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

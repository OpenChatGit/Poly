//! Scrollable containers

use crate::core::{Widget, WidgetId, Context, Event, next_widget_id, BoxedWidget};
use crate::style::Style;

/// Scrollable container
pub struct ScrollView {
    id: WidgetId,
    style: Style,
    child: Option<BoxedWidget>,
    scroll_x: f32,
    scroll_y: f32,
    horizontal: bool,
    vertical: bool,
}

impl ScrollView {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new(),
            child: None,
            scroll_x: 0.0,
            scroll_y: 0.0,
            horizontal: false,
            vertical: true,
        }
    }
    
    pub fn child(mut self, child: impl Widget + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }
    
    pub fn horizontal(mut self) -> Self {
        self.horizontal = true;
        self.vertical = false;
        self
    }
    
    pub fn both(mut self) -> Self {
        self.horizontal = true;
        self.vertical = true;
        self
    }
}

impl Default for ScrollView {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for ScrollView {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> {
        Vec::new()
    }
    
    fn on_event(&mut self, event: &Event, _ctx: &mut Context) -> bool {
        if let Event::Scroll { delta_x, delta_y } = event {
            if self.horizontal {
                self.scroll_x += delta_x;
            }
            if self.vertical {
                self.scroll_y += delta_y;
            }
            return true;
        }
        false
    }
}

/// Single child scrollable (like SingleChildScrollView in Flutter)
pub struct SingleChildScrollView {
    inner: ScrollView,
}

impl SingleChildScrollView {
    pub fn new() -> Self {
        Self {
            inner: ScrollView::new(),
        }
    }
    
    pub fn child(mut self, child: impl Widget + 'static) -> Self {
        self.inner = self.inner.child(child);
        self
    }
}

impl Default for SingleChildScrollView {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for SingleChildScrollView {
    fn id(&self) -> WidgetId { self.inner.id() }
    fn style(&self) -> &Style { self.inner.style() }
    fn build(&self, ctx: &mut Context) -> Vec<BoxedWidget> { self.inner.build(ctx) }
}

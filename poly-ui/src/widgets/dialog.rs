//! Dialog and Modal widgets

use crate::core::{Widget, WidgetId, Context, next_widget_id, BoxedWidget};
use crate::core::context::Color;
use crate::style::Style;
use std::sync::Arc;

pub type OnClose = Arc<dyn Fn() + Send + Sync>;

/// Modal dialog
pub struct Dialog {
    id: WidgetId,
    style: Style,
    title: Option<String>,
    content: Option<BoxedWidget>,
    actions: Vec<BoxedWidget>,
    on_close: Option<OnClose>,
    dismissible: bool,
}

impl Dialog {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new()
                .padding(24.0)
                .border_radius(16.0)
                .background(Color::rgb(40, 40, 50)),
            title: None,
            content: None,
            actions: Vec::new(),
            on_close: None,
            dismissible: true,
        }
    }
    
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
    
    pub fn content(mut self, content: impl Widget + 'static) -> Self {
        self.content = Some(Box::new(content));
        self
    }
    
    pub fn action(mut self, action: impl Widget + 'static) -> Self {
        self.actions.push(Box::new(action));
        self
    }
    
    pub fn on_close<F: Fn() + Send + Sync + 'static>(mut self, handler: F) -> Self {
        self.on_close = Some(Arc::new(handler));
        self
    }
    
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }
}

impl Default for Dialog {
    fn default() -> Self { Self::new() }
}

impl Widget for Dialog {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

/// Snackbar/Toast notification
pub struct Snackbar {
    id: WidgetId,
    style: Style,
    #[allow(dead_code)]
    message: String,
    action_label: Option<String>,
    on_action: Option<Arc<dyn Fn() + Send + Sync>>,
    duration_ms: u32,
}

impl Snackbar {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new()
                .padding(16.0)
                .border_radius(8.0)
                .background(Color::rgb(50, 50, 60)),
            message: message.into(),
            action_label: None,
            on_action: None,
            duration_ms: 4000,
        }
    }
    
    pub fn action<F: Fn() + Send + Sync + 'static>(mut self, label: impl Into<String>, handler: F) -> Self {
        self.action_label = Some(label.into());
        self.on_action = Some(Arc::new(handler));
        self
    }
    
    pub fn duration(mut self, ms: u32) -> Self {
        self.duration_ms = ms;
        self
    }
}

impl Widget for Snackbar {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

//! Button widgets

use crate::core::{Widget, WidgetId, Context, Event, next_widget_id, BoxedWidget};
use crate::core::context::Color;
use crate::style::Style;
use std::sync::Arc;

/// Callback type for button clicks
pub type OnClick = Arc<dyn Fn() + Send + Sync>;

/// Standard button widget
pub struct Button {
    id: WidgetId,
    style: Style,
    label: String,
    on_click: Option<OnClick>,
    disabled: bool,
    loading: bool,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new()
                .padding(12.0)
                .border_radius(8.0)
                .background(Color::rgb(0, 122, 255)),
            label: label.into(),
            on_click: None,
            disabled: false,
            loading: false,
        }
    }
    
    pub fn on_click<F: Fn() + Send + Sync + 'static>(mut self, handler: F) -> Self {
        self.on_click = Some(Arc::new(handler));
        self
    }
    
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
    
    pub fn primary(mut self) -> Self {
        self.style.background = Some(Color::rgb(0, 122, 255));
        self.style.text_color = Some(Color::rgb(255, 255, 255));
        self
    }
    
    pub fn secondary(mut self) -> Self {
        self.style.background = Some(Color::rgb(88, 86, 214));
        self.style.text_color = Some(Color::rgb(255, 255, 255));
        self
    }
    
    pub fn outline(mut self) -> Self {
        self.style.background = None;
        self.style.border_width = 2.0;
        self.style.border_color = Some(Color::rgb(0, 122, 255));
        self.style.text_color = Some(Color::rgb(0, 122, 255));
        self
    }
    
    pub fn danger(mut self) -> Self {
        self.style.background = Some(Color::rgb(255, 59, 48));
        self.style.text_color = Some(Color::rgb(255, 255, 255));
        self
    }
    
    pub fn label(&self) -> &str {
        &self.label
    }
}

impl Widget for Button {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> {
        Vec::new()
    }
    
    fn on_event(&mut self, event: &Event, _ctx: &mut Context) -> bool {
        if self.disabled || self.loading {
            return false;
        }
        
        match event {
            Event::MouseUp { .. } => {
                if let Some(ref handler) = self.on_click {
                    handler();
                    return true;
                }
            }
            _ => {}
        }
        false
    }
}

/// Icon button (just an icon, no text)
pub struct IconButton {
    id: WidgetId,
    style: Style,
    #[allow(dead_code)]
    icon: String,
    on_click: Option<OnClick>,
}

impl IconButton {
    pub fn new(icon: impl Into<String>) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new()
                .size(40.0, 40.0)
                .border_radius(20.0),
            icon: icon.into(),
            on_click: None,
        }
    }
    
    pub fn on_click<F: Fn() + Send + Sync + 'static>(mut self, handler: F) -> Self {
        self.on_click = Some(Arc::new(handler));
        self
    }
}

impl Widget for IconButton {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
    
    fn on_event(&mut self, event: &Event, _ctx: &mut Context) -> bool {
        if let Event::MouseUp { .. } = event {
            if let Some(ref handler) = self.on_click {
                handler();
                return true;
            }
        }
        false
    }
}

/// Floating action button
pub struct Fab {
    id: WidgetId,
    style: Style,
    #[allow(dead_code)]
    icon: String,
    on_click: Option<OnClick>,
}

impl Fab {
    pub fn new(icon: impl Into<String>) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new()
                .size(56.0, 56.0)
                .border_radius(28.0)
                .background(Color::rgb(0, 122, 255)),
            icon: icon.into(),
            on_click: None,
        }
    }
    
    pub fn on_click<F: Fn() + Send + Sync + 'static>(mut self, handler: F) -> Self {
        self.on_click = Some(Arc::new(handler));
        self
    }
}

impl Widget for Fab {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

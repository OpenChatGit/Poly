//! Input widgets

use crate::core::{Widget, WidgetId, Context, Event, next_widget_id, BoxedWidget};
use crate::core::context::Color;
use crate::style::Style;
use std::sync::Arc;

pub type OnChange = Arc<dyn Fn(String) + Send + Sync>;
pub type OnSubmit = Arc<dyn Fn(String) + Send + Sync>;

/// Text input field
pub struct TextInput {
    id: WidgetId,
    style: Style,
    value: String,
    placeholder: String,
    on_change: Option<OnChange>,
    on_submit: Option<OnSubmit>,
    password: bool,
    multiline: bool,
    max_length: Option<usize>,
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new()
                .padding(12.0)
                .border_radius(8.0)
                .border(1.0, Color::rgb(60, 60, 60))
                .background(Color::rgb(30, 30, 30)),
            value: String::new(),
            placeholder: String::new(),
            on_change: None,
            on_submit: None,
            password: false,
            multiline: false,
            max_length: None,
        }
    }
    
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }
    
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }
    
    pub fn on_change<F: Fn(String) + Send + Sync + 'static>(mut self, handler: F) -> Self {
        self.on_change = Some(Arc::new(handler));
        self
    }
    
    pub fn on_submit<F: Fn(String) + Send + Sync + 'static>(mut self, handler: F) -> Self {
        self.on_submit = Some(Arc::new(handler));
        self
    }
    
    pub fn password(mut self) -> Self {
        self.password = true;
        self
    }
    
    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }
    
    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for TextInput {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> {
        Vec::new()
    }
    
    fn on_event(&mut self, event: &Event, _ctx: &mut Context) -> bool {
        match event {
            Event::TextInput { text } => {
                if let Some(max) = self.max_length {
                    if self.value.len() + text.len() > max {
                        return false;
                    }
                }
                self.value.push_str(text);
                if let Some(ref handler) = self.on_change {
                    handler(self.value.clone());
                }
                true
            }
            Event::KeyDown { key, .. } => {
                match key {
                    crate::core::Key::Backspace => {
                        self.value.pop();
                        if let Some(ref handler) = self.on_change {
                            handler(self.value.clone());
                        }
                        true
                    }
                    crate::core::Key::Enter if !self.multiline => {
                        if let Some(ref handler) = self.on_submit {
                            handler(self.value.clone());
                        }
                        true
                    }
                    _ => false
                }
            }
            _ => false
        }
    }
}

/// Checkbox widget
pub struct Checkbox {
    id: WidgetId,
    style: Style,
    checked: bool,
    label: Option<String>,
    on_change: Option<Arc<dyn Fn(bool) + Send + Sync>>,
}

impl Checkbox {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new().size(24.0, 24.0),
            checked: false,
            label: None,
            on_change: None,
        }
    }
    
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
    
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    
    pub fn on_change<F: Fn(bool) + Send + Sync + 'static>(mut self, handler: F) -> Self {
        self.on_change = Some(Arc::new(handler));
        self
    }
}

impl Default for Checkbox {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Checkbox {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
    
    fn on_event(&mut self, event: &Event, _ctx: &mut Context) -> bool {
        if let Event::MouseUp { .. } = event {
            self.checked = !self.checked;
            if let Some(ref handler) = self.on_change {
                handler(self.checked);
            }
            return true;
        }
        false
    }
}

/// Toggle/Switch widget
pub struct Toggle {
    id: WidgetId,
    style: Style,
    on: bool,
    on_change: Option<Arc<dyn Fn(bool) + Send + Sync>>,
}

impl Toggle {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new().size(50.0, 30.0).border_radius(15.0),
            on: false,
            on_change: None,
        }
    }
    
    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }
    
    pub fn on_change<F: Fn(bool) + Send + Sync + 'static>(mut self, handler: F) -> Self {
        self.on_change = Some(Arc::new(handler));
        self
    }
}

impl Default for Toggle {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Toggle {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

/// Slider widget
pub struct Slider {
    id: WidgetId,
    style: Style,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    on_change: Option<Arc<dyn Fn(f32) + Send + Sync>>,
}

impl Slider {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new().height(40.0),
            value: 0.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            on_change: None,
        }
    }
    
    pub fn value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }
    
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }
    
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }
    
    pub fn on_change<F: Fn(f32) + Send + Sync + 'static>(mut self, handler: F) -> Self {
        self.on_change = Some(Arc::new(handler));
        self
    }
}

impl Default for Slider {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Slider {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

//! Image widget

use crate::core::{Widget, WidgetId, Context, next_widget_id, BoxedWidget};
use crate::style::{Style, Dimension};

/// Image display widget
pub struct Image {
    id: WidgetId,
    style: Style,
    #[allow(dead_code)]
    source: ImageSource,
    fit: ImageFit,
}

#[derive(Clone)]
pub enum ImageSource {
    /// Load from file path
    File(String),
    /// Load from URL
    Url(String),
    /// Raw bytes (PNG, JPEG, etc.)
    Bytes(Vec<u8>),
    /// Asset bundled with app
    Asset(String),
}

#[derive(Clone, Copy)]
pub enum ImageFit {
    /// Scale to fill, may crop
    Cover,
    /// Scale to fit, may have letterboxing
    Contain,
    /// Stretch to fill exactly
    Fill,
    /// No scaling
    None,
}

impl Image {
    pub fn file(path: impl Into<String>) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new(),
            source: ImageSource::File(path.into()),
            fit: ImageFit::Contain,
        }
    }
    
    pub fn url(url: impl Into<String>) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new(),
            source: ImageSource::Url(url.into()),
            fit: ImageFit::Contain,
        }
    }
    
    pub fn asset(name: impl Into<String>) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new(),
            source: ImageSource::Asset(name.into()),
            fit: ImageFit::Contain,
        }
    }
    
    pub fn bytes(data: Vec<u8>) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new(),
            source: ImageSource::Bytes(data),
            fit: ImageFit::Contain,
        }
    }
    
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }
    
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.style.width = Dimension::Px(width);
        self.style.height = Dimension::Px(height);
        self
    }
    
    pub fn border_radius(mut self, radius: f32) -> Self {
        self.style.border_radius = radius;
        self
    }
}

impl Widget for Image {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

/// Circular avatar image
pub struct Avatar {
    id: WidgetId,
    style: Style,
    source: Option<ImageSource>,
    fallback: String,
}

impl Avatar {
    pub fn new(size: f32) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new()
                .size(size, size)
                .border_radius(size / 2.0),
            source: None,
            fallback: String::new(),
        }
    }
    
    pub fn image(mut self, source: ImageSource) -> Self {
        self.source = Some(source);
        self
    }
    
    pub fn fallback(mut self, text: impl Into<String>) -> Self {
        self.fallback = text.into();
        self
    }
}

impl Widget for Avatar {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

/// Icon widget (vector icons)
pub struct Icon {
    id: WidgetId,
    style: Style,
    #[allow(dead_code)]
    name: String,
    size: f32,
}

impl Icon {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new().size(24.0, 24.0),
            name: name.into(),
            size: 24.0,
        }
    }
    
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self.style.width = Dimension::Px(size);
        self.style.height = Dimension::Px(size);
        self
    }
    
    pub fn color(mut self, color: crate::core::context::Color) -> Self {
        self.style.text_color = Some(color);
        self
    }
}

impl Widget for Icon {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

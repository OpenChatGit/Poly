//! Text widget

use crate::core::{Widget, WidgetId, Context, next_widget_id, BoxedWidget};
use crate::core::context::Color;
use crate::style::{Style, FontWeight, TextAlign};

/// Text display widget
pub struct Text {
    id: WidgetId,
    style: Style,
    content: String,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new(),
            content: content.into(),
        }
    }
    
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    
    pub fn size(mut self, size: f32) -> Self {
        self.style.font_size = size;
        self
    }
    
    pub fn color(mut self, color: Color) -> Self {
        self.style.text_color = Some(color);
        self
    }
    
    pub fn bold(mut self) -> Self {
        self.style.font_weight = FontWeight::Bold;
        self
    }
    
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.style.font_weight = weight;
        self
    }
    
    pub fn align(mut self, align: TextAlign) -> Self {
        self.style.text_align = align;
        self
    }
    
    pub fn center(mut self) -> Self {
        self.style.text_align = TextAlign::Center;
        self
    }
    
    pub fn content(&self) -> &str {
        &self.content
    }
}

impl Widget for Text {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> {
        Vec::new() // Text has no children
    }
}

/// Heading variants
pub struct H1(Text);
pub struct H2(Text);
pub struct H3(Text);

impl H1 {
    pub fn new(content: impl Into<String>) -> Self {
        Self(Text::new(content).size(32.0).bold())
    }
}

impl H2 {
    pub fn new(content: impl Into<String>) -> Self {
        Self(Text::new(content).size(24.0).bold())
    }
}

impl H3 {
    pub fn new(content: impl Into<String>) -> Self {
        Self(Text::new(content).size(20.0).bold())
    }
}

impl Widget for H1 {
    fn id(&self) -> WidgetId { self.0.id() }
    fn style(&self) -> &Style { self.0.style() }
    fn build(&self, ctx: &mut Context) -> Vec<BoxedWidget> { self.0.build(ctx) }
}

impl Widget for H2 {
    fn id(&self) -> WidgetId { self.0.id() }
    fn style(&self) -> &Style { self.0.style() }
    fn build(&self, ctx: &mut Context) -> Vec<BoxedWidget> { self.0.build(ctx) }
}

impl Widget for H3 {
    fn id(&self) -> WidgetId { self.0.id() }
    fn style(&self) -> &Style { self.0.style() }
    fn build(&self, ctx: &mut Context) -> Vec<BoxedWidget> { self.0.build(ctx) }
}

/// Rich text with multiple styled spans
pub struct RichText {
    id: WidgetId,
    style: Style,
    spans: Vec<TextSpan>,
}

pub struct TextSpan {
    pub text: String,
    pub style: SpanStyle,
}

pub struct SpanStyle {
    pub color: Option<Color>,
    pub size: Option<f32>,
    pub weight: Option<FontWeight>,
    pub italic: bool,
    pub underline: bool,
}

impl Default for SpanStyle {
    fn default() -> Self {
        Self {
            color: None,
            size: None,
            weight: None,
            italic: false,
            underline: false,
        }
    }
}

impl RichText {
    pub fn new() -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new(),
            spans: Vec::new(),
        }
    }
    
    pub fn span(mut self, text: impl Into<String>) -> Self {
        self.spans.push(TextSpan {
            text: text.into(),
            style: SpanStyle::default(),
        });
        self
    }
    
    pub fn styled_span(mut self, text: impl Into<String>, style: SpanStyle) -> Self {
        self.spans.push(TextSpan {
            text: text.into(),
            style,
        });
        self
    }
}

impl Default for RichText {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for RichText {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

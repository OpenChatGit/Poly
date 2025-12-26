//! Styling system for Poly UI

use crate::core::context::Color;

/// Complete style definition for a widget
#[derive(Debug, Clone)]
pub struct Style {
    // Layout
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_height: Option<f32>,
    
    // Flexbox
    pub flex_direction: FlexDirection,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_self: AlignSelf,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_wrap: FlexWrap,
    pub gap: f32,
    
    // Spacing
    pub margin: EdgeInsets,
    pub padding: EdgeInsets,
    
    // Visual
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub border_radius: f32,
    
    // Text
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub text_color: Option<Color>,
    pub text_align: TextAlign,
    
    // Effects
    pub opacity: f32,
    pub shadow: Option<Shadow>,
    
    // Positioning
    pub position: Position,
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub left: Option<f32>,
    pub z_index: i32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            align_self: AlignSelf::Auto,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_wrap: FlexWrap::NoWrap,
            gap: 0.0,
            margin: EdgeInsets::zero(),
            padding: EdgeInsets::zero(),
            background: None,
            border_color: None,
            border_width: 0.0,
            border_radius: 0.0,
            font_size: 16.0,
            font_weight: FontWeight::Normal,
            text_color: None,
            text_align: TextAlign::Left,
            opacity: 1.0,
            shadow: None,
            position: Position::Relative,
            top: None,
            right: None,
            bottom: None,
            left: None,
            z_index: 0,
        }
    }
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }
    
    // Builder methods
    pub fn width(mut self, w: impl Into<Dimension>) -> Self {
        self.width = w.into();
        self
    }
    
    pub fn height(mut self, h: impl Into<Dimension>) -> Self {
        self.height = h.into();
        self
    }
    
    pub fn size(mut self, w: impl Into<Dimension>, h: impl Into<Dimension>) -> Self {
        self.width = w.into();
        self.height = h.into();
        self
    }
    
    pub fn flex_grow(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }
    
    pub fn flex_direction(mut self, dir: FlexDirection) -> Self {
        self.flex_direction = dir;
        self
    }
    
    pub fn justify_content(mut self, jc: JustifyContent) -> Self {
        self.justify_content = jc;
        self
    }
    
    pub fn align_items(mut self, ai: AlignItems) -> Self {
        self.align_items = ai;
        self
    }
    
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
    
    pub fn padding(mut self, p: impl Into<EdgeInsets>) -> Self {
        self.padding = p.into();
        self
    }
    
    pub fn margin(mut self, m: impl Into<EdgeInsets>) -> Self {
        self.margin = m.into();
        self
    }
    
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }
    
    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = Some(color);
        self
    }
    
    pub fn border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }
    
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }
    
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }
    
    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Dimension {
    Auto,
    Px(f32),
    Percent(f32),
}

impl From<f32> for Dimension {
    fn from(v: f32) -> Self {
        Dimension::Px(v)
    }
}

impl From<i32> for Dimension {
    fn from(v: i32) -> Self {
        Dimension::Px(v as f32)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Debug, Clone, Copy)]
pub enum JustifyContent {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy)]
pub enum AlignItems {
    Start,
    End,
    Center,
    Stretch,
    Baseline,
}

#[derive(Debug, Clone, Copy)]
pub enum AlignSelf {
    Auto,
    Start,
    End,
    Center,
    Stretch,
}

#[derive(Debug, Clone, Copy)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Debug, Clone, Copy)]
pub enum Position {
    Relative,
    Absolute,
}

#[derive(Debug, Clone, Copy)]
pub enum FontWeight {
    Thin,
    Light,
    Normal,
    Medium,
    Bold,
    Black,
}

#[derive(Debug, Clone, Copy)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub struct EdgeInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeInsets {
    pub fn zero() -> Self {
        Self { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 }
    }
    
    pub fn all(v: f32) -> Self {
        Self { top: v, right: v, bottom: v, left: v }
    }
    
    pub fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self { top: vertical, right: horizontal, bottom: vertical, left: horizontal }
    }
    
    pub fn only(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self { top, right, bottom, left }
    }
}

impl From<f32> for EdgeInsets {
    fn from(v: f32) -> Self {
        Self::all(v)
    }
}

impl From<(f32, f32)> for EdgeInsets {
    fn from((h, v): (f32, f32)) -> Self {
        Self::symmetric(h, v)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Shadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Color,
}

impl Shadow {
    pub fn new(offset_x: f32, offset_y: f32, blur: f32, color: Color) -> Self {
        Self {
            offset_x,
            offset_y,
            blur,
            spread: 0.0,
            color,
        }
    }
}

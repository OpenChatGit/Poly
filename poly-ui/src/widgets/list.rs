//! List widgets for efficient rendering of many items

use crate::core::{Widget, WidgetId, Context, next_widget_id, BoxedWidget};
use crate::style::Style;
use std::sync::Arc;

/// Builder function for list items
pub type ItemBuilder<T> = Arc<dyn Fn(&T, usize) -> BoxedWidget + Send + Sync>;

/// Virtualized list for efficient rendering of large datasets
pub struct ListView<T: Clone + Send + Sync + 'static> {
    id: WidgetId,
    style: Style,
    #[allow(dead_code)]
    items: Vec<T>,
    item_builder: Option<ItemBuilder<T>>,
    item_height: f32,
}

impl<T: Clone + Send + Sync + 'static> ListView<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self {
            id: next_widget_id(),
            style: Style::new(),
            items,
            item_builder: None,
            item_height: 48.0,
        }
    }
    
    pub fn builder<F>(mut self, builder: F) -> Self 
    where F: Fn(&T, usize) -> BoxedWidget + Send + Sync + 'static {
        self.item_builder = Some(Arc::new(builder));
        self
    }
    
    pub fn item_height(mut self, height: f32) -> Self {
        self.item_height = height;
        self
    }
}

impl<T: Clone + Send + Sync + 'static> Widget for ListView<T> {
    fn id(&self) -> WidgetId { self.id }
    fn style(&self) -> &Style { &self.style }
    fn build(&self, _ctx: &mut Context) -> Vec<BoxedWidget> { Vec::new() }
}

//! Entity Component System (ECS) for Data-Oriented UI
//!
//! This module provides a high-performance, cache-friendly UI architecture
//! that can handle millions of UI elements efficiently.
//!
//! Key benefits over traditional object-oriented UI:
//! - Extreme cache locality through contiguous memory layout
//! - Efficient batch processing of similar components
//! - Easy parallelization of UI updates
//! - Minimal memory overhead per entity

use std::any::{Any, TypeId};
use std::collections::HashMap;

// ============================================
// Entity
// ============================================

/// Unique identifier for a UI element
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(pub u32);

impl Entity {
    pub fn id(&self) -> u32 {
        self.0
    }
}

/// Generation counter to detect stale entity references
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Generation(pub u32);

/// Entity with generation for safe references
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId {
    pub index: u32,
    pub generation: u32,
}

// ============================================
// Components
// ============================================

/// Marker trait for components
pub trait Component: 'static + Send + Sync {}

/// Transform component - position and size
#[derive(Debug, Clone, Copy, Default)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
}

impl Transform {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x, y, width, height,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
    
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.width &&
        py >= self.y && py <= self.y + self.height
    }
}

impl Component for Transform {}

/// Style component - visual appearance
#[derive(Debug, Clone, Default)]
pub struct Style {
    pub background_color: [f32; 4],
    pub border_color: [f32; 4],
    pub border_width: f32,
    pub border_radius: f32,
    pub opacity: f32,
    pub visible: bool,
    pub z_index: i32,
}

impl Style {
    pub fn new() -> Self {
        Self {
            background_color: [0.0, 0.0, 0.0, 0.0],
            border_color: [0.0, 0.0, 0.0, 0.0],
            border_width: 0.0,
            border_radius: 0.0,
            opacity: 1.0,
            visible: true,
            z_index: 0,
        }
    }
    
    pub fn with_background(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.background_color = [r, g, b, a];
        self
    }
    
    pub fn with_border(mut self, r: f32, g: f32, b: f32, width: f32) -> Self {
        self.border_color = [r, g, b, 1.0];
        self.border_width = width;
        self
    }
    
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }
}

impl Component for Style {}

/// Text component
#[derive(Debug, Clone, Default)]
pub struct Text {
    pub content: String,
    pub font_size: f32,
    pub color: [f32; 4],
    pub font_family: String,
    pub align: TextAlign,
    pub line_height: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl Text {
    pub fn new(content: &str) -> Self {
        Self {
            content: content.to_string(),
            font_size: 16.0,
            color: [1.0, 1.0, 1.0, 1.0],
            font_family: "system-ui".to_string(),
            align: TextAlign::Left,
            line_height: 1.5,
        }
    }
    
    pub fn with_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }
    
    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color = [r, g, b, 1.0];
        self
    }
}

impl Component for Text {}

/// Interactive component - handles user input
#[derive(Debug, Clone, Default)]
pub struct Interactive {
    pub enabled: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub cursor: Cursor,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Cursor {
    #[default]
    Default,
    Pointer,
    Text,
    Move,
    NotAllowed,
}

impl Interactive {
    pub fn new() -> Self {
        Self {
            enabled: true,
            hovered: false,
            pressed: false,
            focused: false,
            cursor: Cursor::Default,
        }
    }
    
    pub fn clickable() -> Self {
        Self {
            enabled: true,
            cursor: Cursor::Pointer,
            ..Default::default()
        }
    }
}

impl Component for Interactive {}

/// Parent-child relationship
#[derive(Debug, Clone, Default)]
pub struct Hierarchy {
    pub parent: Option<Entity>,
    pub children: Vec<Entity>,
    pub depth: u32,
}

impl Component for Hierarchy {}

/// Layout component for flexbox-like layout
#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub direction: FlexDirection,
    pub justify: JustifyContent,
    pub align: AlignItems,
    pub gap: f32,
    pub padding: [f32; 4], // top, right, bottom, left
    pub margin: [f32; 4],
    pub flex_grow: f32,
    pub flex_shrink: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum JustifyContent {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum AlignItems {
    #[default]
    Start,
    End,
    Center,
    Stretch,
}

impl Component for Layout {}

/// Animation component
#[derive(Debug, Clone, Default)]
pub struct Animation {
    pub active: bool,
    pub duration: f32,
    pub elapsed: f32,
    pub easing: Easing,
    pub property: AnimatedProperty,
    pub from: f32,
    pub to: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Easing {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum AnimatedProperty {
    #[default]
    Opacity,
    X,
    Y,
    Width,
    Height,
    Rotation,
    Scale,
}

impl Animation {
    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
        match self.easing {
            Easing::Linear => t,
            Easing::EaseIn => t * t,
            Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
        }
    }
    
    pub fn current_value(&self) -> f32 {
        self.from + (self.to - self.from) * self.progress()
    }
}

impl Component for Animation {}

// ============================================
// Component Storage
// ============================================

/// Type-erased component storage
pub trait ComponentStorage: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn remove(&mut self, entity: Entity);
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
}

/// Dense storage for components - cache-friendly contiguous memory
pub struct DenseStorage<T: Component> {
    /// Actual component data in contiguous memory
    data: Vec<T>,
    /// Maps entity index to data index
    entity_to_data: HashMap<u32, usize>,
    /// Maps data index back to entity
    data_to_entity: Vec<Entity>,
}

impl<T: Component> DenseStorage<T> {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            entity_to_data: HashMap::new(),
            data_to_entity: Vec::new(),
        }
    }
    
    pub fn insert(&mut self, entity: Entity, component: T) {
        if let Some(&idx) = self.entity_to_data.get(&entity.0) {
            // Update existing
            self.data[idx] = component;
        } else {
            // Insert new
            let idx = self.data.len();
            self.data.push(component);
            self.entity_to_data.insert(entity.0, idx);
            self.data_to_entity.push(entity);
        }
    }
    
    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.entity_to_data.get(&entity.0).map(|&idx| &self.data[idx])
    }
    
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        if let Some(&idx) = self.entity_to_data.get(&entity.0) {
            Some(&mut self.data[idx])
        } else {
            None
        }
    }
    
    /// Iterate over all components with their entities
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.data_to_entity.iter().zip(self.data.iter()).map(|(&e, c)| (e, c))
    }
    
    /// Iterate mutably over all components
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.data_to_entity.iter().zip(self.data.iter_mut()).map(|(&e, c)| (e, c))
    }
    
    /// Get raw slice for batch processing
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }
    
    /// Get mutable slice for batch processing
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }
}

impl<T: Component> Default for DenseStorage<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Component> ComponentStorage for DenseStorage<T> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
    
    fn remove(&mut self, entity: Entity) {
        if let Some(idx) = self.entity_to_data.remove(&entity.0) {
            // Swap-remove for O(1) deletion
            let last_idx = self.data.len() - 1;
            if idx != last_idx {
                self.data.swap(idx, last_idx);
                self.data_to_entity.swap(idx, last_idx);
                // Update the swapped entity's index
                let swapped_entity = self.data_to_entity[idx];
                self.entity_to_data.insert(swapped_entity.0, idx);
            }
            self.data.pop();
            self.data_to_entity.pop();
        }
    }
    
    fn len(&self) -> usize { self.data.len() }
}

// ============================================
// World - The ECS Container
// ============================================

/// The main ECS world containing all entities and components
pub struct World {
    /// Next entity ID to assign
    next_entity: u32,
    /// All component storages by type
    storages: HashMap<TypeId, Box<dyn ComponentStorage>>,
    /// Free list of recycled entity IDs
    free_entities: Vec<u32>,
    /// Entity generations for detecting stale references
    generations: Vec<u32>,
}

impl World {
    pub fn new() -> Self {
        Self {
            next_entity: 0,
            storages: HashMap::new(),
            free_entities: Vec::new(),
            generations: Vec::new(),
        }
    }
    
    /// Create a new entity
    pub fn spawn(&mut self) -> Entity {
        let id = if let Some(recycled) = self.free_entities.pop() {
            recycled
        } else {
            let id = self.next_entity;
            self.next_entity += 1;
            self.generations.push(0);
            id
        };
        Entity(id)
    }
    
    /// Destroy an entity and all its components
    pub fn despawn(&mut self, entity: Entity) {
        // Remove from all storages
        for storage in self.storages.values_mut() {
            storage.remove(entity);
        }
        // Increment generation and add to free list
        if (entity.0 as usize) < self.generations.len() {
            self.generations[entity.0 as usize] += 1;
            self.free_entities.push(entity.0);
        }
    }
    
    /// Add a component to an entity
    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) {
        let type_id = TypeId::of::<T>();
        let storage = self.storages
            .entry(type_id)
            .or_insert_with(|| Box::new(DenseStorage::<T>::new()));
        
        storage.as_any_mut()
            .downcast_mut::<DenseStorage<T>>()
            .unwrap()
            .insert(entity, component);
    }
    
    /// Get a component reference
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.storages.get(&type_id)?
            .as_any()
            .downcast_ref::<DenseStorage<T>>()?
            .get(entity)
    }
    
    /// Get a mutable component reference
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        self.storages.get_mut(&type_id)?
            .as_any_mut()
            .downcast_mut::<DenseStorage<T>>()?
            .get_mut(entity)
    }
    
    /// Get component storage for batch processing
    pub fn storage<T: Component>(&self) -> Option<&DenseStorage<T>> {
        let type_id = TypeId::of::<T>();
        self.storages.get(&type_id)?
            .as_any()
            .downcast_ref::<DenseStorage<T>>()
    }
    
    /// Get mutable component storage for batch processing
    pub fn storage_mut<T: Component>(&mut self) -> Option<&mut DenseStorage<T>> {
        let type_id = TypeId::of::<T>();
        self.storages.get_mut(&type_id)?
            .as_any_mut()
            .downcast_mut::<DenseStorage<T>>()
    }
    
    /// Query entities with specific components
    pub fn query<T: Component>(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.storage::<T>()
            .map(|s| s.iter())
            .into_iter()
            .flatten()
    }
    
    /// Entity count
    pub fn entity_count(&self) -> u32 {
        self.next_entity - self.free_entities.len() as u32
    }
}

impl Default for World {
    fn default() -> Self { Self::new() }
}

// ============================================
// Systems
// ============================================

/// System trait for processing entities
pub trait System: Send + Sync {
    fn run(&mut self, world: &mut World, delta: f32);
}

/// Animation system - updates all animations
pub struct AnimationSystem;

impl System for AnimationSystem {
    fn run(&mut self, world: &mut World, delta: f32) {
        // Get animation storage
        if let Some(animations) = world.storage_mut::<Animation>() {
            for (entity, anim) in animations.iter_mut() {
                if !anim.active {
                    continue;
                }
                
                anim.elapsed += delta;
                
                // Apply animation value to target property
                let value = anim.current_value();
                
                // Animation complete?
                if anim.elapsed >= anim.duration {
                    anim.active = false;
                }
                
                // Note: In a real implementation, we'd update the target component here
                // This requires a more sophisticated approach with deferred updates
                let _ = (entity, value); // Suppress unused warning
            }
        }
    }
}

/// Layout system - computes positions based on flexbox rules
pub struct LayoutSystem;

impl System for LayoutSystem {
    fn run(&mut self, world: &mut World, _delta: f32) {
        // Collect root entities (no parent)
        let roots: Vec<Entity> = world.query::<Hierarchy>()
            .filter(|(_, h)| h.parent.is_none())
            .map(|(e, _)| e)
            .collect();
        
        // Layout each root tree
        for root in roots {
            self.layout_tree(world, root, 0.0, 0.0);
        }
    }
}

impl LayoutSystem {
    fn layout_tree(&self, world: &mut World, entity: Entity, x: f32, y: f32) {
        // Get layout and transform
        let (layout, children) = {
            let layout = world.get::<Layout>(entity).cloned().unwrap_or_default();
            let hierarchy = world.get::<Hierarchy>(entity).cloned().unwrap_or_default();
            (layout, hierarchy.children)
        };
        
        // Update transform position
        if let Some(transform) = world.get_mut::<Transform>(entity) {
            transform.x = x + layout.margin[3]; // left margin
            transform.y = y + layout.margin[0]; // top margin
        }
        
        // Layout children
        let mut offset_x = layout.padding[3];
        let mut offset_y = layout.padding[0];
        
        for child in children {
            self.layout_tree(world, child, x + offset_x, y + offset_y);
            
            // Get child size for offset calculation
            if let Some(child_transform) = world.get::<Transform>(child) {
                match layout.direction {
                    FlexDirection::Row | FlexDirection::RowReverse => {
                        offset_x += child_transform.width + layout.gap;
                    }
                    FlexDirection::Column | FlexDirection::ColumnReverse => {
                        offset_y += child_transform.height + layout.gap;
                    }
                }
            }
        }
    }
}

/// Hit testing system - determines which entity is under cursor
pub struct HitTestSystem {
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub hovered_entity: Option<Entity>,
}

impl HitTestSystem {
    pub fn new() -> Self {
        Self {
            cursor_x: 0.0,
            cursor_y: 0.0,
            hovered_entity: None,
        }
    }
    
    pub fn set_cursor(&mut self, x: f32, y: f32) {
        self.cursor_x = x;
        self.cursor_y = y;
    }
}

impl Default for HitTestSystem {
    fn default() -> Self { Self::new() }
}

impl System for HitTestSystem {
    fn run(&mut self, world: &mut World, _delta: f32) {
        // Reset all hover states
        if let Some(interactive) = world.storage_mut::<Interactive>() {
            for (_, inter) in interactive.iter_mut() {
                inter.hovered = false;
            }
        }
        
        // Find topmost entity under cursor
        let mut best_entity: Option<(Entity, i32)> = None;
        
        // Query all interactive entities with transforms
        let candidates: Vec<(Entity, bool, i32)> = world.query::<Transform>()
            .filter_map(|(e, t)| {
                if t.contains(self.cursor_x, self.cursor_y) {
                    let z = world.get::<Style>(e).map(|s| s.z_index).unwrap_or(0);
                    let enabled = world.get::<Interactive>(e).map(|i| i.enabled).unwrap_or(false);
                    Some((e, enabled, z))
                } else {
                    None
                }
            })
            .collect();
        
        for (entity, enabled, z) in candidates {
            if enabled {
                if best_entity.map(|(_, bz)| z > bz).unwrap_or(true) {
                    best_entity = Some((entity, z));
                }
            }
        }
        
        // Set hover state
        if let Some((entity, _)) = best_entity {
            if let Some(inter) = world.get_mut::<Interactive>(entity) {
                inter.hovered = true;
            }
            self.hovered_entity = Some(entity);
        } else {
            self.hovered_entity = None;
        }
    }
}

// ============================================
// Builder Pattern for Easy Entity Creation
// ============================================

/// Builder for creating UI entities
pub struct EntityBuilder<'a> {
    world: &'a mut World,
    entity: Entity,
}

impl<'a> EntityBuilder<'a> {
    pub fn new(world: &'a mut World) -> Self {
        let entity = world.spawn();
        Self { world, entity }
    }
    
    pub fn with_transform(self, x: f32, y: f32, w: f32, h: f32) -> Self {
        self.world.insert(self.entity, Transform::new(x, y, w, h));
        self
    }
    
    pub fn with_style(self, style: Style) -> Self {
        self.world.insert(self.entity, style);
        self
    }
    
    pub fn with_text(self, text: Text) -> Self {
        self.world.insert(self.entity, text);
        self
    }
    
    pub fn with_layout(self, layout: Layout) -> Self {
        self.world.insert(self.entity, layout);
        self
    }
    
    pub fn interactive(self) -> Self {
        self.world.insert(self.entity, Interactive::clickable());
        self
    }
    
    pub fn with_parent(self, parent: Entity) -> Self {
        // Update parent's children
        if let Some(parent_hierarchy) = self.world.get_mut::<Hierarchy>(parent) {
            parent_hierarchy.children.push(self.entity);
        }
        
        // Set this entity's parent
        let depth = self.world.get::<Hierarchy>(parent)
            .map(|h| h.depth + 1)
            .unwrap_or(0);
        
        self.world.insert(self.entity, Hierarchy {
            parent: Some(parent),
            children: Vec::new(),
            depth,
        });
        
        self
    }
    
    pub fn build(self) -> Entity {
        // Ensure hierarchy exists
        if self.world.get::<Hierarchy>(self.entity).is_none() {
            self.world.insert(self.entity, Hierarchy::default());
        }
        self.entity
    }
}

// ============================================
// Tests
// ============================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_entity_creation() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        assert_ne!(e1, e2);
        assert_eq!(world.entity_count(), 2);
    }
    
    #[test]
    fn test_component_insert_get() {
        let mut world = World::new();
        let entity = world.spawn();
        
        world.insert(entity, Transform::new(10.0, 20.0, 100.0, 50.0));
        
        let transform = world.get::<Transform>(entity).unwrap();
        assert_eq!(transform.x, 10.0);
        assert_eq!(transform.y, 20.0);
    }
    
    #[test]
    fn test_entity_despawn() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Transform::default());
        
        assert!(world.get::<Transform>(entity).is_some());
        
        world.despawn(entity);
        
        // Entity ID is recycled, but component is removed
        assert!(world.get::<Transform>(entity).is_none());
    }
    
    #[test]
    fn test_batch_iteration() {
        let mut world = World::new();
        
        // Create 1000 entities
        for i in 0..1000 {
            let entity = world.spawn();
            world.insert(entity, Transform::new(i as f32, 0.0, 10.0, 10.0));
        }
        
        // Batch process all transforms
        let count = world.query::<Transform>().count();
        assert_eq!(count, 1000);
    }
    
    #[test]
    fn test_builder_pattern() {
        let mut world = World::new();
        
        let button = EntityBuilder::new(&mut world)
            .with_transform(100.0, 100.0, 200.0, 50.0)
            .with_style(Style::new().with_background(0.2, 0.2, 0.2, 1.0))
            .with_text(Text::new("Click me"))
            .interactive()
            .build();
        
        assert!(world.get::<Transform>(button).is_some());
        assert!(world.get::<Style>(button).is_some());
        assert!(world.get::<Text>(button).is_some());
        assert!(world.get::<Interactive>(button).is_some());
    }
    
    #[test]
    fn test_hierarchy() {
        let mut world = World::new();
        
        let parent = EntityBuilder::new(&mut world)
            .with_transform(0.0, 0.0, 400.0, 300.0)
            .build();
        
        let child = EntityBuilder::new(&mut world)
            .with_transform(10.0, 10.0, 100.0, 50.0)
            .with_parent(parent)
            .build();
        
        let parent_hierarchy = world.get::<Hierarchy>(parent).unwrap();
        assert!(parent_hierarchy.children.contains(&child));
        
        let child_hierarchy = world.get::<Hierarchy>(child).unwrap();
        assert_eq!(child_hierarchy.parent, Some(parent));
        assert_eq!(child_hierarchy.depth, 1);
    }
}

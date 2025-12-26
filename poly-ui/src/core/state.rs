//! Reactive state management

use std::sync::{Arc, RwLock};
use std::any::Any;

/// Reactive state container - triggers rebuilds when changed
pub struct State<T: Clone + Send + Sync + 'static> {
    value: Arc<RwLock<T>>,
    listeners: Arc<RwLock<Vec<Box<dyn Fn() + Send + Sync>>>>,
}

impl<T: Clone + Send + Sync + 'static> State<T> {
    pub fn new(initial: T) -> Self {
        Self {
            value: Arc::new(RwLock::new(initial)),
            listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Get the current value
    pub fn get(&self) -> T {
        self.value.read().unwrap().clone()
    }
    
    /// Set a new value and notify listeners
    pub fn set(&self, new_value: T) {
        {
            let mut value = self.value.write().unwrap();
            *value = new_value;
        }
        self.notify();
    }
    
    /// Update value with a function
    pub fn update<F: FnOnce(&mut T)>(&self, f: F) {
        {
            let mut value = self.value.write().unwrap();
            f(&mut *value);
        }
        self.notify();
    }
    
    /// Subscribe to changes
    pub fn subscribe<F: Fn() + Send + Sync + 'static>(&self, callback: F) {
        let mut listeners = self.listeners.write().unwrap();
        listeners.push(Box::new(callback));
    }
    
    fn notify(&self) {
        let listeners = self.listeners.read().unwrap();
        for listener in listeners.iter() {
            listener();
        }
    }
}

impl<T: Clone + Send + Sync + 'static> Clone for State<T> {
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
            listeners: Arc::clone(&self.listeners),
        }
    }
}

/// Global app state store
pub struct Store {
    states: Arc<RwLock<std::collections::HashMap<String, Box<dyn Any + Send + Sync>>>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    pub fn set<T: Any + Send + Sync + 'static>(&self, key: &str, value: T) {
        let mut states = self.states.write().unwrap();
        states.insert(key.to_string(), Box::new(value));
    }
    
    pub fn get<T: Any + Clone + 'static>(&self, key: &str) -> Option<T> {
        let states = self.states.read().unwrap();
        states.get(key).and_then(|v| v.downcast_ref::<T>().cloned())
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

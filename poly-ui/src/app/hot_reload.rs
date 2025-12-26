//! Hot reload support for development

use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::sync::mpsc::{channel, Receiver};
use std::path::Path;
use std::time::Duration;

/// Hot reload watcher
pub struct HotReloader {
    _watcher: notify::RecommendedWatcher,
    receiver: Receiver<Result<Event, notify::Error>>,
}

impl HotReloader {
    /// Create a new hot reloader watching the given path
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, notify::Error> {
        let (tx, rx) = channel();
        
        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })?;
        
        watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;
        
        Ok(Self {
            _watcher: watcher,
            receiver: rx,
        })
    }
    
    /// Check if any files have changed
    pub fn check_changes(&self) -> Vec<String> {
        let mut changed_files = Vec::new();
        
        // Non-blocking check for changes
        while let Ok(result) = self.receiver.try_recv() {
            if let Ok(event) = result {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        for path in event.paths {
                            if let Some(ext) = path.extension() {
                                // Watch for Poly files and Rust files
                                if ext == "poly" || ext == "rs" {
                                    if let Some(path_str) = path.to_str() {
                                        changed_files.push(path_str.to_string());
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        
        changed_files
    }
    
    /// Wait for a change with timeout
    pub fn wait_for_change(&self, timeout: Duration) -> Option<Vec<String>> {
        match self.receiver.recv_timeout(timeout) {
            Ok(Ok(event)) => {
                let mut changed = Vec::new();
                for path in event.paths {
                    if let Some(path_str) = path.to_str() {
                        changed.push(path_str.to_string());
                    }
                }
                Some(changed)
            }
            _ => None,
        }
    }
}

/// Development server with hot reload
pub struct DevServer {
    reloader: Option<HotReloader>,
    on_reload: Option<Box<dyn Fn(&[String]) + Send>>,
}

impl DevServer {
    pub fn new() -> Self {
        Self {
            reloader: None,
            on_reload: None,
        }
    }
    
    /// Watch a directory for changes
    pub fn watch<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.reloader = HotReloader::new(path).ok();
        self
    }
    
    /// Set callback for when files change
    pub fn on_reload<F: Fn(&[String]) + Send + 'static>(mut self, callback: F) -> Self {
        self.on_reload = Some(Box::new(callback));
        self
    }
    
    /// Check for changes and trigger reload if needed
    pub fn poll(&self) {
        if let Some(ref reloader) = self.reloader {
            let changes = reloader.check_changes();
            if !changes.is_empty() {
                println!("[Hot Reload] Files changed: {:?}", changes);
                if let Some(ref callback) = self.on_reload {
                    callback(&changes);
                }
            }
        }
    }
}

impl Default for DevServer {
    fn default() -> Self {
        Self::new()
    }
}

//! Poly UI - High-performance cross-platform UI framework
//! 
//! A Rust UI framework prototype built on native Poly APIs.

pub mod core;
pub mod widgets;
pub mod render;
pub mod layout;
pub mod style;
pub mod app;
pub mod runtime;

#[cfg(target_arch = "wasm32")]
pub mod web;

pub use app::{App, PolyApp, WindowConfig, run_demo};
pub use widgets::*;
pub use style::*;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::app::{App, PolyApp, WindowConfig};
    pub use crate::widgets::*;
    pub use crate::style::*;
    pub use crate::core::{Widget, State, Context};
}

//! Built-in widgets for Poly UI

mod container;
mod text;
mod button;
mod input;
mod image;
mod scroll;
mod list;
mod card;
mod dialog;
mod progress;
mod navigation;

pub use container::*;
pub use text::*;
pub use button::*;
pub use input::*;
pub use self::image::*;
pub use scroll::*;
pub use list::*;
pub use card::*;
pub use dialog::*;
pub use progress::*;
pub use navigation::*;

/// Helper macro for creating widgets
#[macro_export]
macro_rules! widget {
    ($name:ident { $($field:ident: $value:expr),* $(,)? }) => {
        $name::new()$(.with_$field($value))*
    };
}

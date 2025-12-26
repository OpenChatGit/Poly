//! Event system for Poly UI

/// All possible UI events
#[derive(Debug, Clone)]
pub enum Event {
    // Mouse events
    MouseDown { x: f32, y: f32, button: MouseButton },
    MouseUp { x: f32, y: f32, button: MouseButton },
    MouseMove { x: f32, y: f32 },
    MouseEnter,
    MouseLeave,
    Scroll { delta_x: f32, delta_y: f32 },
    
    // Touch events
    TouchStart { id: u64, x: f32, y: f32 },
    TouchMove { id: u64, x: f32, y: f32 },
    TouchEnd { id: u64 },
    
    // Keyboard events
    KeyDown { key: Key, modifiers: Modifiers },
    KeyUp { key: Key, modifiers: Modifiers },
    TextInput { text: String },
    
    // Focus events
    Focus,
    Blur,
    
    // Window events
    Resize { width: f32, height: f32 },
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,  // Cmd on Mac, Win on Windows
}

impl Default for Modifiers {
    fn default() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
            meta: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    
    // Numbers
    Num0, Num1, Num2, Num3, Num4,
    Num5, Num6, Num7, Num8, Num9,
    
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    
    // Special keys
    Escape, Tab, CapsLock, Shift, Control, Alt, Meta,
    Space, Enter, Backspace, Delete,
    
    // Navigation
    Up, Down, Left, Right,
    Home, End, PageUp, PageDown,
    
    // Other
    Unknown,
}

/// Event handler callback type
pub type EventHandler = Box<dyn Fn(&Event) -> bool + Send + Sync>;

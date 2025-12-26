//! System Tray support for Poly applications
//! Provides tray icon, menu, and event handling

#[cfg(feature = "native")]
use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu, CheckMenuItem},
    Icon,
};

#[cfg(feature = "native")]
use std::sync::{Arc, Mutex, mpsc};

/// Tray menu item types
#[derive(Debug, Clone)]
pub enum TrayMenuItem {
    /// Regular clickable item
    Item { id: String, label: String, enabled: bool },
    /// Checkbox item
    Check { id: String, label: String, checked: bool, enabled: bool },
    /// Separator line
    Separator,
    /// Submenu with nested items
    Submenu { id: String, label: String, items: Vec<TrayMenuItem> },
}

/// Configuration for system tray
#[derive(Debug, Clone)]
pub struct TrayConfig {
    pub tooltip: String,
    pub icon_path: Option<String>,
    pub menu_items: Vec<TrayMenuItem>,
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            tooltip: "Poly App".to_string(),
            icon_path: None,
            menu_items: vec![
                TrayMenuItem::Item { id: "show".to_string(), label: "Show".to_string(), enabled: true },
                TrayMenuItem::Separator,
                TrayMenuItem::Item { id: "quit".to_string(), label: "Quit".to_string(), enabled: true },
            ],
        }
    }
}

impl TrayConfig {
    pub fn new(tooltip: &str) -> Self {
        Self {
            tooltip: tooltip.to_string(),
            ..Default::default()
        }
    }
    
    pub fn with_icon(mut self, path: &str) -> Self {
        self.icon_path = Some(path.to_string());
        self
    }
    
    pub fn with_menu(mut self, items: Vec<TrayMenuItem>) -> Self {
        self.menu_items = items;
        self
    }
}

/// Event from tray interaction
#[derive(Debug, Clone)]
pub enum TrayEvent {
    /// Menu item clicked
    MenuClick { id: String },
    /// Tray icon clicked (left click)
    IconClick,
    /// Tray icon double-clicked
    IconDoubleClick,
}

/// System tray handle for controlling the tray
#[cfg(feature = "native")]
pub struct TrayHandle {
    _tray: TrayIcon,
    event_receiver: mpsc::Receiver<TrayEvent>,
    #[allow(dead_code)]
    menu_items: Arc<Mutex<std::collections::HashMap<String, String>>>,
}

#[cfg(feature = "native")]
impl TrayHandle {
    /// Poll for tray events (non-blocking)
    pub fn poll_event(&self) -> Option<TrayEvent> {
        self.event_receiver.try_recv().ok()
    }
    
    /// Wait for next tray event (blocking)
    pub fn wait_event(&self) -> Option<TrayEvent> {
        self.event_receiver.recv().ok()
    }
}

/// Create a system tray icon
#[cfg(feature = "native")]
pub fn create_tray(config: TrayConfig) -> Result<TrayHandle, Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();
    let menu_items: Arc<Mutex<std::collections::HashMap<String, String>>> = Arc::new(Mutex::new(std::collections::HashMap::new()));
    let menu_items_clone = Arc::clone(&menu_items);
    
    // Build menu
    let menu = Menu::new();
    build_menu(&menu, &config.menu_items, &menu_items)?;
    
    // Load icon
    let icon = if let Some(ref path) = config.icon_path {
        load_tray_icon(path)?
    } else {
        // Default icon (simple colored square)
        create_default_icon()?
    };
    
    // Create tray
    let tray = TrayIconBuilder::new()
        .with_tooltip(&config.tooltip)
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()?;
    
    // Start menu event listener thread
    let tx_menu = tx.clone();
    std::thread::spawn(move || {
        let receiver = MenuEvent::receiver();
        loop {
            if let Ok(event) = receiver.recv() {
                let items = menu_items_clone.lock().unwrap();
                if let Some(id) = items.get(&event.id.0) {
                    let _ = tx_menu.send(TrayEvent::MenuClick { id: id.clone() });
                }
            }
        }
    });
    
    Ok(TrayHandle {
        _tray: tray,
        event_receiver: rx,
        menu_items,
    })
}

#[cfg(feature = "native")]
fn build_menu(
    menu: &Menu,
    items: &[TrayMenuItem],
    id_map: &Arc<Mutex<std::collections::HashMap<String, String>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    for item in items {
        match item {
            TrayMenuItem::Item { id, label, enabled } => {
                let menu_item = MenuItem::new(label, *enabled, None);
                {
                    let mut map = id_map.lock().unwrap();
                    map.insert(menu_item.id().0.clone(), id.clone());
                }
                menu.append(&menu_item)?;
            }
            TrayMenuItem::Check { id, label, checked, enabled } => {
                let check_item = CheckMenuItem::new(label, *enabled, *checked, None);
                {
                    let mut map = id_map.lock().unwrap();
                    map.insert(check_item.id().0.clone(), id.clone());
                }
                menu.append(&check_item)?;
            }
            TrayMenuItem::Separator => {
                menu.append(&PredefinedMenuItem::separator())?;
            }
            TrayMenuItem::Submenu { id: _, label, items: sub_items } => {
                let submenu = Submenu::new(label, true);
                build_submenu(&submenu, sub_items, id_map)?;
                menu.append(&submenu)?;
            }
        }
    }
    Ok(())
}

#[cfg(feature = "native")]
fn build_submenu(
    submenu: &Submenu,
    items: &[TrayMenuItem],
    id_map: &Arc<Mutex<std::collections::HashMap<String, String>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    for item in items {
        match item {
            TrayMenuItem::Item { id, label, enabled } => {
                let menu_item = MenuItem::new(label, *enabled, None);
                {
                    let mut map = id_map.lock().unwrap();
                    map.insert(menu_item.id().0.clone(), id.clone());
                }
                submenu.append(&menu_item)?;
            }
            TrayMenuItem::Check { id, label, checked, enabled } => {
                let check_item = CheckMenuItem::new(label, *enabled, *checked, None);
                {
                    let mut map = id_map.lock().unwrap();
                    map.insert(check_item.id().0.clone(), id.clone());
                }
                submenu.append(&check_item)?;
            }
            TrayMenuItem::Separator => {
                submenu.append(&PredefinedMenuItem::separator())?;
            }
            TrayMenuItem::Submenu { id: _, label, items: sub_items } => {
                let nested = Submenu::new(label, true);
                build_submenu(&nested, sub_items, id_map)?;
                submenu.append(&nested)?;
            }
        }
    }
    Ok(())
}

#[cfg(feature = "native")]
fn load_tray_icon(path: &str) -> Result<Icon, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::BufReader;
    use image::GenericImageView;
    
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    
    // Detect format from extension
    let format = if path.ends_with(".png") {
        image::ImageFormat::Png
    } else if path.ends_with(".ico") {
        image::ImageFormat::Ico
    } else {
        image::ImageFormat::Png // Default to PNG
    };
    
    let img = image::load(reader, format)?;
    let (_width, _height) = img.dimensions();
    
    // Resize to standard tray icon size (32x32 or 64x64)
    let size = 32u32;
    let resized = image::imageops::resize(
        &img.to_rgba8(),
        size,
        size,
        image::imageops::FilterType::Lanczos3
    );
    
    let icon = Icon::from_rgba(resized.into_raw(), size, size)?;
    Ok(icon)
}

#[cfg(feature = "native")]
fn create_default_icon() -> Result<Icon, Box<dyn std::error::Error>> {
    // Create a simple 32x32 icon with Poly colors
    let size = 32u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    
    for y in 0..size {
        for x in 0..size {
            // Create a simple gradient square with rounded corners
            let cx = (x as f32 - size as f32 / 2.0).abs();
            let cy = (y as f32 - size as f32 / 2.0).abs();
            let dist = (cx * cx + cy * cy).sqrt();
            
            if dist < size as f32 / 2.0 - 2.0 {
                // Poly cyan color: #5dc1d2
                rgba.push(93);  // R
                rgba.push(193); // G
                rgba.push(210); // B
                rgba.push(255); // A
            } else if dist < size as f32 / 2.0 {
                // Anti-aliased edge
                let alpha = ((size as f32 / 2.0 - dist) * 127.0) as u8;
                rgba.push(93);
                rgba.push(193);
                rgba.push(210);
                rgba.push(alpha);
            } else {
                // Transparent
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
            }
        }
    }
    
    let icon = Icon::from_rgba(rgba, size, size)?;
    Ok(icon)
}

// ============================================
// Stubs for non-native builds
// ============================================

#[cfg(not(feature = "native"))]
pub struct TrayHandle;

#[cfg(not(feature = "native"))]
impl TrayHandle {
    pub fn poll_event(&self) -> Option<TrayEvent> { None }
    pub fn wait_event(&self) -> Option<TrayEvent> { None }
}

#[cfg(not(feature = "native"))]
pub fn create_tray(_config: TrayConfig) -> Result<TrayHandle, Box<dyn std::error::Error>> {
    Err("Native feature not enabled. Build with --features native".into())
}

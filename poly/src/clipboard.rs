//! Clipboard API for Poly
//! Read and write text/images to system clipboard

#[cfg(feature = "native")]
use std::sync::Mutex;

#[cfg(feature = "native")]
use once_cell::sync::Lazy;

/// Global clipboard instance
#[cfg(feature = "native")]
static CLIPBOARD: Lazy<Mutex<Option<arboard::Clipboard>>> = Lazy::new(|| {
    Mutex::new(arboard::Clipboard::new().ok())
});

/// Read text from clipboard
#[cfg(feature = "native")]
pub fn read_text() -> Result<String, String> {
    let mut guard = CLIPBOARD.lock().map_err(|e| format!("Lock error: {}", e))?;
    let clipboard = guard.as_mut().ok_or("Clipboard not available")?;
    clipboard.get_text().map_err(|e| format!("Read error: {}", e))
}

/// Write text to clipboard
#[cfg(feature = "native")]
pub fn write_text(text: &str) -> Result<(), String> {
    let mut guard = CLIPBOARD.lock().map_err(|e| format!("Lock error: {}", e))?;
    let clipboard = guard.as_mut().ok_or("Clipboard not available")?;
    clipboard.set_text(text).map_err(|e| format!("Write error: {}", e))
}

/// Read image from clipboard (returns PNG bytes)
#[cfg(feature = "native")]
pub fn read_image() -> Result<Vec<u8>, String> {
    let mut guard = CLIPBOARD.lock().map_err(|e| format!("Lock error: {}", e))?;
    let clipboard = guard.as_mut().ok_or("Clipboard not available")?;
    
    let img = clipboard.get_image().map_err(|e| format!("Read error: {}", e))?;
    
    // Convert to PNG using image crate
    use image::{ImageBuffer, Rgba};
    
    let img_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(
        img.width as u32,
        img.height as u32,
        img.bytes.to_vec(),
    ).ok_or("Failed to create image buffer")?;
    
    let mut png_data = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_data);
    img_buffer.write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("Encode error: {}", e))?;
    
    Ok(png_data)
}

/// Write image to clipboard (from PNG bytes)
#[cfg(feature = "native")]
pub fn write_image(png_data: &[u8]) -> Result<(), String> {
    let mut guard = CLIPBOARD.lock().map_err(|e| format!("Lock error: {}", e))?;
    let clipboard = guard.as_mut().ok_or("Clipboard not available")?;
    
    // Decode PNG
    let img = image::load_from_memory(png_data)
        .map_err(|e| format!("Decode error: {}", e))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    
    let img_data = arboard::ImageData {
        width: width as usize,
        height: height as usize,
        bytes: std::borrow::Cow::Owned(rgba.into_raw()),
    };
    
    clipboard.set_image(img_data).map_err(|e| format!("Write error: {}", e))
}

/// Clear clipboard
#[cfg(feature = "native")]
pub fn clear() -> Result<(), String> {
    let mut guard = CLIPBOARD.lock().map_err(|e| format!("Lock error: {}", e))?;
    let clipboard = guard.as_mut().ok_or("Clipboard not available")?;
    clipboard.clear().map_err(|e| format!("Clear error: {}", e))
}

// Stubs for non-native builds
#[cfg(not(feature = "native"))]
pub fn read_text() -> Result<String, String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn write_text(_text: &str) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn read_image() -> Result<Vec<u8>, String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn write_image(_png_data: &[u8]) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[cfg(not(feature = "native"))]
pub fn clear() -> Result<(), String> {
    Err("Requires native feature".to_string())
}

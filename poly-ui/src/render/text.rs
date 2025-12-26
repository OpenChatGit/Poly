//! Text rendering using cosmic-text

use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, Family, FontSystem, Metrics, Shaping, SwashCache,
};
use crate::core::context::Color;

/// Text renderer using cosmic-text
pub struct TextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl TextRenderer {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
        }
    }
    
    /// Render text to a pixel buffer
    pub fn render_text(
        &mut self,
        text: &str,
        font_size: f32,
        color: Color,
        max_width: f32,
    ) -> TextImage {
        let metrics = Metrics::new(font_size, font_size * 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        
        buffer.set_size(&mut self.font_system, Some(max_width), None);
        
        let attrs = Attrs::new()
            .family(Family::SansSerif);
        
        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);
        
        // Calculate dimensions
        let width = max_width as u32;
        let height = (buffer.lines.len() as f32 * font_size * 1.2) as u32;
        let height = height.max(font_size as u32 + 4);
        
        // Create pixel buffer
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        
        // Render glyphs
        let text_color = CosmicColor::rgba(
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            (color.a * 255.0) as u8,
        );
        
        buffer.draw(&mut self.font_system, &mut self.swash_cache, text_color, |x, y, w, h, color| {
            let x = x as u32;
            let y = y as u32;
            
            for dy in 0..h {
                for dx in 0..w {
                    let px = x + dx;
                    let py = y + dy;
                    
                    if px < width && py < height {
                        let idx = ((py * width + px) * 4) as usize;
                        if idx + 3 < pixels.len() {
                            // Alpha blend
                            let alpha = color.a() as f32 / 255.0;
                            pixels[idx] = ((pixels[idx] as f32 * (1.0 - alpha)) + (color.r() as f32 * alpha)) as u8;
                            pixels[idx + 1] = ((pixels[idx + 1] as f32 * (1.0 - alpha)) + (color.g() as f32 * alpha)) as u8;
                            pixels[idx + 2] = ((pixels[idx + 2] as f32 * (1.0 - alpha)) + (color.b() as f32 * alpha)) as u8;
                            pixels[idx + 3] = (pixels[idx + 3] as f32 + (alpha * 255.0 * (1.0 - pixels[idx + 3] as f32 / 255.0))) as u8;
                        }
                    }
                }
            }
        });
        
        TextImage {
            width,
            height,
            pixels,
        }
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Rendered text as pixel data
pub struct TextImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

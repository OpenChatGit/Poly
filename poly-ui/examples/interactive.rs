//! Interactive demo with mouse tracking

use poly_ui::app::{PolyApp, WindowConfig};
use poly_ui::render::Primitive;
use poly_ui::core::context::Color;
use std::sync::{Arc, Mutex};

fn main() {
    let config = WindowConfig {
        title: "Poly UI - Interactive Demo".to_string(),
        width: 900,
        height: 600,
        ..Default::default()
    };
    
    // Shared state for animation
    let frame = Arc::new(Mutex::new(0u64));
    let frame_clone = frame.clone();
    
    PolyApp::new(config)
        .with_ui(move |render_list, width, height| {
            let mut f = frame_clone.lock().unwrap();
            *f += 1;
            let frame_num = *f;
            drop(f);
            
            let w = width as f32;
            let h = height as f32;
            
            // Animated background gradient (simulated with rects)
            let bg_hue = ((frame_num as f32 * 0.5) % 360.0) / 360.0;
            render_list.rect(0.0, 0.0, w, h, hsl_to_rgb(bg_hue, 0.15, 0.08), 0.0);
            
            // Header
            render_list.rect(0.0, 0.0, w, 70.0, Color::rgba(0, 0, 0, 0.3), 0.0);
            
            // Animated circles
            let num_circles = 5;
            for i in 0..num_circles {
                let phase = (frame_num as f32 * 0.02) + (i as f32 * 0.5);
                let cx = w * 0.5 + phase.sin() * 150.0;
                let cy = h * 0.5 + phase.cos() * 100.0;
                let radius = 30.0 + (phase * 2.0).sin() * 10.0;
                let alpha = 0.3 + (i as f32 * 0.1);
                
                render_list.primitives.push(Primitive::Circle {
                    cx,
                    cy,
                    radius,
                    color: Color::rgba(0, 122, 255, alpha),
                });
            }
            
            // Cards
            let card_width = 250.0;
            let card_height = 180.0;
            let padding = 30.0;
            let start_y = 100.0;
            
            for i in 0..3 {
                let x = padding + (card_width + padding) * i as f32;
                let hover_offset = ((frame_num as f32 * 0.03) + i as f32).sin() * 5.0;
                
                // Card shadow
                render_list.rect(
                    x + 4.0, start_y + 4.0 + hover_offset,
                    card_width, card_height,
                    Color::rgba(0, 0, 0, 0.3), 16.0
                );
                
                // Card
                render_list.rect(
                    x, start_y + hover_offset,
                    card_width, card_height,
                    Color::rgb(35, 38, 48), 16.0
                );
                
                // Card accent
                let accent_colors = [
                    Color::rgb(0, 122, 255),
                    Color::rgb(88, 86, 214),
                    Color::rgb(255, 149, 0),
                ];
                render_list.rect(
                    x, start_y + hover_offset,
                    card_width, 4.0,
                    accent_colors[i], 16.0
                );
            }
            
            // Buttons row
            let button_y = start_y + card_height + 40.0;
            let buttons = [
                ("Primary", Color::rgb(0, 122, 255)),
                ("Success", Color::rgb(52, 199, 89)),
                ("Warning", Color::rgb(255, 149, 0)),
                ("Danger", Color::rgb(255, 59, 48)),
            ];
            
            for (i, (_, color)) in buttons.iter().enumerate() {
                let x = padding + (130.0 + 15.0) * i as f32;
                render_list.rect(x, button_y, 130.0, 44.0, *color, 8.0);
            }
            
            // Progress bar
            let progress_y = button_y + 70.0;
            let progress = ((frame_num as f32 * 0.01).sin() + 1.0) / 2.0;
            
            // Background
            render_list.rect(padding, progress_y, w - padding * 2.0, 8.0, Color::rgb(50, 50, 60), 4.0);
            // Fill
            render_list.rect(padding, progress_y, (w - padding * 2.0) * progress, 8.0, Color::rgb(0, 122, 255), 4.0);
            
            // Stats circles at bottom
            let stats_y = h - 120.0;
            for i in 0..4 {
                let cx = padding + 60.0 + (i as f32 * 120.0);
                let cy = stats_y + 40.0;
                
                // Outer ring
                render_list.primitives.push(Primitive::Circle {
                    cx, cy, radius: 40.0,
                    color: Color::rgb(50, 50, 60),
                });
                
                // Inner fill (animated)
                let fill_radius = 35.0 * (0.5 + 0.5 * ((frame_num as f32 * 0.02) + i as f32).sin());
                render_list.primitives.push(Primitive::Circle {
                    cx, cy, radius: fill_radius,
                    color: Color::rgb(0, 122, 255),
                });
            }
        })
        .run();
}

// Helper: HSL to RGB conversion
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    
    let (r, g, b) = match (h * 6.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    
    Color {
        r: r + m,
        g: g + m,
        b: b + m,
        a: 1.0,
    }
}

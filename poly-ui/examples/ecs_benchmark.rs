//! ECS Benchmark - Demonstrates handling millions of UI elements
//!
//! Run with: cargo run -p poly-ui --example ecs_benchmark --release

use poly_ui::core::ecs::*;
use std::time::Instant;

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║           Poly UI - ECS Performance Benchmark            ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    
    // Test different entity counts
    for &count in &[1_000, 10_000, 100_000, 1_000_000] {
        benchmark_entities(count);
    }
    
    println!();
    println!("Benchmark complete!");
    println!();
    println!("Key takeaways:");
    println!("  - ECS uses contiguous memory for cache-friendly iteration");
    println!("  - Batch processing is O(n) with minimal overhead");
    println!("  - Entity creation/destruction is O(1) amortized");
    println!("  - Component queries avoid virtual dispatch overhead");
}

fn benchmark_entities(count: usize) {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Testing with {} entities", format_number(count));
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let mut world = World::new();
    
    // 1. Entity Creation
    let start = Instant::now();
    let entities: Vec<Entity> = (0..count).map(|_| world.spawn()).collect();
    let creation_time = start.elapsed();
    println!("  Entity creation:     {:>10.2?} ({:.0} entities/ms)", 
             creation_time, 
             count as f64 / creation_time.as_millis().max(1) as f64);
    
    // 2. Component Insertion
    let start = Instant::now();
    for (i, &entity) in entities.iter().enumerate() {
        let x = (i % 1000) as f32 * 10.0;
        let y = (i / 1000) as f32 * 10.0;
        world.insert(entity, Transform::new(x, y, 100.0, 50.0));
        world.insert(entity, Style::new()
            .with_background(0.2, 0.2, 0.2, 1.0)
            .with_border(0.0, 0.8, 1.0, 1.0));
    }
    let insert_time = start.elapsed();
    println!("  Component insertion: {:>10.2?} ({:.0} components/ms)", 
             insert_time,
             (count * 2) as f64 / insert_time.as_millis().max(1) as f64);
    
    // 3. Batch Query (read)
    let start = Instant::now();
    let mut total_area = 0.0f64;
    for (_, transform) in world.query::<Transform>() {
        total_area += (transform.width * transform.height) as f64;
    }
    let query_time = start.elapsed();
    println!("  Batch query (read):  {:>10.2?} ({:.0} entities/ms)", 
             query_time,
             count as f64 / query_time.as_micros().max(1) as f64 * 1000.0);
    
    // 4. Batch Update (write)
    let start = Instant::now();
    if let Some(transforms) = world.storage_mut::<Transform>() {
        for (_, transform) in transforms.iter_mut() {
            transform.x += 1.0;
            transform.y += 1.0;
        }
    }
    let update_time = start.elapsed();
    println!("  Batch update (write):{:>10.2?} ({:.0} entities/ms)", 
             update_time,
             count as f64 / update_time.as_micros().max(1) as f64 * 1000.0);
    
    // 5. Hit Testing (simulated)
    let start = Instant::now();
    let test_points = [(500.0, 500.0), (1000.0, 1000.0), (5000.0, 5000.0)];
    let mut hits = 0;
    for (px, py) in test_points {
        for (_, transform) in world.query::<Transform>() {
            if transform.contains(px, py) {
                hits += 1;
            }
        }
    }
    let hit_test_time = start.elapsed();
    println!("  Hit testing (3 pts): {:>10.2?} ({} hits found)", 
             hit_test_time, hits);
    
    // 6. Entity Destruction
    let start = Instant::now();
    for entity in entities {
        world.despawn(entity);
    }
    let destroy_time = start.elapsed();
    println!("  Entity destruction:  {:>10.2?} ({:.0} entities/ms)", 
             destroy_time,
             count as f64 / destroy_time.as_millis().max(1) as f64);
    
    // Memory estimate
    let transform_size = std::mem::size_of::<Transform>();
    let style_size = std::mem::size_of::<Style>();
    let estimated_mb = (count * (transform_size + style_size)) as f64 / 1_000_000.0;
    println!("  Estimated memory:    {:>10.2} MB", estimated_mb);
    
    // Prevent optimization from removing calculations
    println!("  (Total area: {:.0})", total_area);
    println!();
}

fn format_number(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{}M", n / 1_000_000)
    } else if n >= 1_000 {
        format!("{}K", n / 1_000)
    } else {
        n.to_string()
    }
}

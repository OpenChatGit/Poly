//! Ad Blocking Engine - Brave's adblock-rust Integration
//! 
//! This module integrates Brave's high-performance adblock-rust engine
//! for network-level ad blocking without JavaScript interference.

use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;

#[cfg(feature = "native")]
use adblock::engine::Engine;
#[cfg(feature = "native")]
use adblock::lists::FilterSet;

/// Global ad blocking engine
#[cfg(feature = "native")]
pub static ADBLOCK_ENGINE: Lazy<Arc<RwLock<Option<Engine>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(None))
});

/// Statistics
#[cfg(feature = "native")]
pub static ADBLOCK_STATS: Lazy<Arc<RwLock<(usize, usize)>>> = Lazy::new(|| {
    Arc::new(RwLock::new((0, 0))) // (blocked, total)
});

/// Initialize the ad blocker with Brave's engine
#[cfg(feature = "native")]
pub fn init_brave_adblock(filter_lists: Vec<String>) -> Result<(), String> {
    println!("[AdBlock] Initializing Brave's adblock-rust engine...");
    
    // Combine all filter lists
    let mut all_rules = Vec::new();
    for list in &filter_lists {
        for line in list.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('!') && !line.starts_with('[') {
                all_rules.push(line.to_string());
            }
        }
    }
    
    println!("[AdBlock] Loaded {} filter rules", all_rules.len());
    
    // Create engine with Brave's implementation
    let mut filter_set = FilterSet::new(false);
    filter_set.add_filters(&all_rules, Default::default());
    
    let engine = Engine::from_filter_set(filter_set, true);
    
    // Store in global state
    let mut blocker = ADBLOCK_ENGINE.write()
        .map_err(|_| "Failed to acquire ad blocker lock")?;
    *blocker = Some(engine);
    
    println!("[AdBlock] ✅ Brave engine initialized with {} rules", all_rules.len());
    Ok(())
}

/// Check if a URL should be blocked using Brave's engine
#[cfg(feature = "native")]
pub fn should_block_brave(url: &str, source_url: &str, resource_type: &str) -> bool {
    // Ensure ad blocker is initialized (thread-safe, only runs once)
    crate::adblock_init::ensure_adblock_initialized();
    
    // Update stats
    if let Ok(mut stats) = ADBLOCK_STATS.write() {
        stats.1 += 1; // Total requests
    }
    
    let engine_lock = match ADBLOCK_ENGINE.read() {
        Ok(lock) => lock,
        Err(_) => return false,
    };
    
    let engine = match engine_lock.as_ref() {
        Some(e) => e,
        None => {
            // Engine not ready yet (still downloading lists)
            return false;
        }
    };
    
    // Use Brave's check_network_request method (API changed in v0.12)
    use adblock::request::Request;
    
    // Try to create request - if it fails, allow the request (don't panic)
    let request = match Request::new(url, source_url, resource_type) {
        Ok(req) => req,
        Err(e) => {
            eprintln!("[AdBlock] ⚠️ Failed to parse URL '{}': {:?}", url, e);
            return false; // Allow invalid URLs (don't block)
        }
    };
    
    let result = engine.check_network_request(&request);
    
    let blocked = result.matched;
    
    if blocked {
        // Update stats
        if let Ok(mut stats) = ADBLOCK_STATS.write() {
            stats.0 += 1; // Blocked requests
        }
        
        println!("[AdBlock] 🚫 Blocked: {} (type: {})", url, resource_type);
    }
    
    blocked
}

/// Get cosmetic filters for a URL using Brave's engine
#[cfg(feature = "native")]
pub fn get_cosmetic_filters(url: &str) -> Vec<String> {
    let engine_lock = match ADBLOCK_ENGINE.read() {
        Ok(lock) => lock,
        Err(_) => return Vec::new(),
    };
    
    let engine = match engine_lock.as_ref() {
        Some(e) => e,
        None => return Vec::new(),
    };
    
    // Get URL-specific cosmetic resources
    let resources = engine.url_cosmetic_resources(url);
    
    // In v0.12, hide_selectors is a HashSet, convert to Vec
    resources.hide_selectors.into_iter().collect()
}

/// Get statistics about blocked requests
#[cfg(feature = "native")]
pub fn get_stats() -> (usize, usize) {
    ADBLOCK_STATS.read()
        .map(|stats| *stats)
        .unwrap_or((0, 0))
}

/// Download EasyList filter list
#[cfg(feature = "native")]
pub fn download_easylist() -> Result<String, String> {
    let url = "https://easylist.to/easylist/easylist.txt";
    
    let response = ureq::get(url)
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .map_err(|e| format!("Failed to download EasyList: {}", e))?;
    
    let body = response.into_string()
        .map_err(|e| format!("Failed to read EasyList: {}", e))?;
    
    Ok(body)
}

/// Download EasyPrivacy filter list
#[cfg(feature = "native")]
pub fn download_easyprivacy() -> Result<String, String> {
    let url = "https://easylist.to/easylist/easyprivacy.txt";
    
    let response = ureq::get(url)
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .map_err(|e| format!("Failed to download EasyPrivacy: {}", e))?;
    
    let body = response.into_string()
        .map_err(|e| format!("Failed to read EasyPrivacy: {}", e))?;
    
    Ok(body)
}

// Stubs for non-native builds
#[cfg(not(feature = "native"))]
pub fn init_brave_adblock(_filter_lists: Vec<String>) -> Result<(), String> {
    Err("Ad blocking not available in non-native builds".to_string())
}

#[cfg(not(feature = "native"))]
pub fn should_block_brave(_url: &str, _source_url: &str, _resource_type: &str) -> bool {
    false
}

#[cfg(not(feature = "native"))]
pub fn get_cosmetic_filters(_url: &str) -> Vec<String> {
    Vec::new()
}

#[cfg(not(feature = "native"))]
pub fn download_easylist() -> Result<String, String> {
    Err("Ad blocking not available in non-native builds".to_string())
}

#[cfg(not(feature = "native"))]
pub fn download_easyprivacy() -> Result<String, String> {
    Err("Ad blocking not available in non-native builds".to_string())
}

#[cfg(not(feature = "native"))]
pub fn get_stats() -> (usize, usize) {
    (0, 0)
}

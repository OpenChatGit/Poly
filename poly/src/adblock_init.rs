//! Ad Blocking Initialization
//! 
//! Automatically initializes the ad blocker when the browser starts.
//! Downloads filter lists and sets up Brave's adblock-rust engine.

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize ad blocking on first call (thread-safe)
#[cfg(feature = "native")]
pub fn ensure_adblock_initialized() {
    println!("[AdBlock] ensure_adblock_initialized() called");
    
    INIT.call_once(|| {
        println!("[AdBlock] 🚀 Starting initialization (first time)...");
        
        // Spawn a background thread to download and initialize
        std::thread::spawn(|| {
            println!("[AdBlock] 📥 Background thread started");
            match initialize_adblock_sync() {
                Ok(()) => println!("[AdBlock] ✅ Initialization complete"),
                Err(e) => eprintln!("[AdBlock] ❌ Initialization failed: {}", e),
            }
        });
        
        println!("[AdBlock] Background thread spawned, continuing...");
    });
    
    println!("[AdBlock] ensure_adblock_initialized() finished");
}

#[cfg(feature = "native")]
fn initialize_adblock_sync() -> Result<(), String> {
    println!("[AdBlock] 📋 initialize_adblock_sync() started");
    println!("[AdBlock] 🌐 Downloading filter lists...");
    
    // Download filter lists in parallel
    println!("[AdBlock] Starting parallel downloads...");
    let (easylist_result, easyprivacy_result) = rayon::join(
        || {
            println!("[AdBlock] Downloading EasyList...");
            let result = crate::adblock_engine::download_easylist();
            match &result {
                Ok(content) => println!("[AdBlock] ✅ EasyList downloaded: {} bytes", content.len()),
                Err(e) => eprintln!("[AdBlock] ❌ EasyList download failed: {}", e),
            }
            result
        },
        || {
            println!("[AdBlock] Downloading EasyPrivacy...");
            let result = crate::adblock_engine::download_easyprivacy();
            match &result {
                Ok(content) => println!("[AdBlock] ✅ EasyPrivacy downloaded: {} bytes", content.len()),
                Err(e) => eprintln!("[AdBlock] ❌ EasyPrivacy download failed: {}", e),
            }
            result
        },
    );
    
    println!("[AdBlock] Downloads complete, processing results...");
    
    let easylist = easylist_result?;
    let easyprivacy = easyprivacy_result?;
    
    println!("[AdBlock] Downloaded EasyList: {} bytes", easylist.len());
    println!("[AdBlock] Downloaded EasyPrivacy: {} bytes", easyprivacy.len());
    
    // Initialize Brave's engine with both lists
    println!("[AdBlock] Initializing Brave's engine...");
    let filter_lists = vec![easylist, easyprivacy];
    crate::adblock_engine::init_brave_adblock(filter_lists)?;
    
    println!("[AdBlock] ✅ Engine initialized successfully");
    Ok(())
}

#[cfg(not(feature = "native"))]
pub fn ensure_adblock_initialized() {
    // No-op for non-native builds
}

//! Single Instance support for Poly applications
//! Ensures only one instance of the app runs at a time

use std::sync::atomic::{AtomicBool, Ordering};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

static INSTANCE_ACQUIRED: AtomicBool = AtomicBool::new(false);

// Store the lock file handle globally
static mut LOCK_FILE: Option<File> = None;

/// Get the lock file path for an app
fn get_lock_path(app_id: &str) -> PathBuf {
    let safe_id = app_id.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_");
    
    #[cfg(target_os = "windows")]
    {
        // Use %LOCALAPPDATA%\Poly\locks
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let lock_dir = PathBuf::from(local_app_data).join("Poly").join("locks");
            let _ = std::fs::create_dir_all(&lock_dir);
            return lock_dir.join(format!("{}.lock", safe_id));
        }
    }
    
    // Fallback to temp directory
    std::env::temp_dir().join(format!("poly_{}.lock", safe_id))
}

/// Try to acquire single instance lock
#[cfg(target_os = "windows")]
fn try_acquire_lock(app_id: &str) -> Result<bool, String> {
    use std::os::windows::fs::OpenOptionsExt;
    
    let lock_path = get_lock_path(app_id);
    
    // Try to open with exclusive access
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .share_mode(0) // No sharing - exclusive access
        .open(&lock_path);
    
    match file {
        Ok(f) => {
            // Got exclusive access
            unsafe { LOCK_FILE = Some(f); }
            Ok(true)
        }
        Err(e) => {
            // Check if it's a sharing violation (another instance has the lock)
            if e.raw_os_error() == Some(32) { // ERROR_SHARING_VIOLATION
                Ok(false)
            } else {
                Err(e.to_string())
            }
        }
    }
}

/// Try to acquire single instance lock (non-Windows)
#[cfg(not(target_os = "windows"))]
fn try_acquire_lock(app_id: &str) -> Result<bool, String> {
    let lock_path = get_lock_path(app_id);
    
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&lock_path)
        .map_err(|e| e.to_string())?;
    
    // Try to get exclusive lock using flock
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let fd = file.as_raw_fd();
        let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
        
        if result == 0 {
            unsafe { LOCK_FILE = Some(file); }
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    #[cfg(not(unix))]
    {
        // Fallback: just assume we got the lock
        unsafe { LOCK_FILE = Some(file); }
        Ok(true)
    }
}

/// Release the lock file
fn release_lock() {
    unsafe {
        LOCK_FILE = None;
    }
}

/// Configuration for single instance behavior
#[derive(Debug, Clone)]
pub struct SingleInstanceConfig {
    /// Unique identifier for the app (usually package name)
    pub app_id: String,
}

impl SingleInstanceConfig {
    pub fn new(app_id: &str) -> Self {
        Self {
            app_id: app_id.to_string(),
        }
    }
}

/// Try to acquire single instance lock
/// Returns true if this is the first instance, false if another instance is running
pub fn try_acquire_instance(config: &SingleInstanceConfig) -> Result<bool, String> {
    if INSTANCE_ACQUIRED.load(Ordering::SeqCst) {
        return Ok(true); // Already acquired in this process
    }
    
    let acquired = try_acquire_lock(&config.app_id)?;
    
    if acquired {
        INSTANCE_ACQUIRED.store(true, Ordering::SeqCst);
    }
    
    Ok(acquired)
}

/// Release the single instance lock
pub fn release_instance() {
    if INSTANCE_ACQUIRED.load(Ordering::SeqCst) {
        release_lock();
        INSTANCE_ACQUIRED.store(false, Ordering::SeqCst);
    }
}

/// Check if this process holds the single instance lock
pub fn is_primary_instance() -> bool {
    INSTANCE_ACQUIRED.load(Ordering::SeqCst)
}

/// Check if single instance is enabled and this is the primary instance
/// Simple helper function for use in main.rs
pub fn check_single_instance(app_id: &str) -> bool {
    let config = SingleInstanceConfig::new(app_id);
    match try_acquire_instance(&config) {
        Ok(is_primary) => is_primary,
        Err(_) => true, // On error, allow running
    }
}

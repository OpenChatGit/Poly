//! Global hotkey registration for Poly.
//!
//! Global hotkeys are implemented on **Windows only** today, using the Win32
//! `RegisterHotKey`/`UnregisterHotKey` API plus a background message-loop thread.
//!
//! On other platforms the same public API is still available (when the `native`
//! feature is enabled) so that host applications compile unchanged, but
//! `register_hotkey` returns an `Err` stating that global hotkeys are only
//! implemented on Windows, `unregister_hotkey` returns `Ok(())`, and
//! `poll_hotkey_events` returns an empty `Vec`.
//!
//! API surface:
//! - `register_hotkey(combo, callback_name) -> Result<i32, String>`
//! - `unregister_hotkey(id) -> Result<(), String>`
//! - `poll_hotkey_events() -> Vec<String>`
//!
//! When the `native` feature is disabled this module is intentionally empty.

#[cfg(all(feature = "native", target_os = "windows"))]
use std::collections::HashMap;
#[cfg(all(feature = "native", target_os = "windows"))]
use std::sync::{Arc, Mutex};
#[cfg(all(feature = "native", target_os = "windows"))]
use once_cell::sync::Lazy;

#[cfg(all(feature = "native", target_os = "windows"))]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey,
    MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, HOT_KEY_MODIFIERS,
};
#[cfg(all(feature = "native", target_os = "windows"))]
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};
#[cfg(all(feature = "native", target_os = "windows"))]
use windows::Win32::Foundation::HWND;

/// Maps hotkey id -> callback function name
#[cfg(all(feature = "native", target_os = "windows"))]
static HOTKEY_MAP: Lazy<Arc<Mutex<HashMap<i32, String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Pending hotkey events (callback names) to be polled
#[cfg(all(feature = "native", target_os = "windows"))]
static HOTKEY_EVENTS: Lazy<Arc<Mutex<Vec<String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

#[cfg(all(feature = "native", target_os = "windows"))]
static HOTKEY_THREAD_STARTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(all(feature = "native", target_os = "windows"))]
static ID_CTR: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1);

/// Parse a combo string like "ctrl+shift+a" into (modifiers, vk_code)
#[cfg(all(feature = "native", target_os = "windows"))]
fn parse_combo(combo: &str) -> Result<(HOT_KEY_MODIFIERS, u32), String> {
    let lower = combo.to_lowercase();
    let parts: Vec<&str> = lower.split('+').collect();
    let mut mods = HOT_KEY_MODIFIERS(0);
    let mut vk: Option<u32> = None;

    for part in parts {
        match part.trim() {
            "ctrl" | "control" => mods |= MOD_CONTROL,
            "alt"              => mods |= MOD_ALT,
            "shift"            => mods |= MOD_SHIFT,
            "win" | "super"    => mods |= MOD_WIN,
            key => {
                if key.len() == 1 {
                    let c = key.chars().next().unwrap().to_ascii_uppercase();
                    vk = Some(c as u32);
                } else if let Some(n) = key.strip_prefix('f') {
                    if let Ok(num) = n.parse::<u32>() {
                        if (1..=12).contains(&num) {
                            vk = Some(0x6F + num); // VK_F1=0x70
                        }
                    }
                } else {
                    vk = Some(match key {
                        "space"  => 0x20,
                        "enter"  => 0x0D,
                        "tab"    => 0x09,
                        "esc"    => 0x1B,
                        "delete" => 0x2E,
                        "home"   => 0x24,
                        "end"    => 0x23,
                        "pgup"   => 0x21,
                        "pgdn"   => 0x22,
                        other    => return Err(format!("Unknown key: {}", other)),
                    });
                }
            }
        }
    }

    let vk = vk.ok_or_else(|| format!("No key specified in combo: {}", combo))?;
    Ok((mods, vk))
}

/// Start the background message loop thread (once)
#[cfg(all(feature = "native", target_os = "windows"))]
fn ensure_thread() {
    if HOTKEY_THREAD_STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    let map = Arc::clone(&HOTKEY_MAP);
    let events = Arc::clone(&HOTKEY_EVENTS);
    std::thread::spawn(move || unsafe {
        let mut msg = MSG::default();
        loop {
            if GetMessageW(&mut msg, HWND(std::ptr::null_mut()), WM_HOTKEY, WM_HOTKEY).as_bool() {
                let id = msg.wParam.0 as i32;
                if let Ok(m) = map.lock() {
                    if let Some(cb) = m.get(&id) {
                        if let Ok(mut ev) = events.lock() {
                            ev.push(cb.clone());
                        }
                    }
                }
            }
        }
    });
}

/// Register a global hotkey. Returns the hotkey id.
#[cfg(all(feature = "native", target_os = "windows"))]
pub fn register_hotkey(combo: &str, callback: &str) -> Result<i32, String> {
    let (mods, vk) = parse_combo(combo)?;
    ensure_thread();

    let id = ID_CTR.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    unsafe {
        RegisterHotKey(HWND(std::ptr::null_mut()), id, mods, vk)
            .map_err(|e| format!("RegisterHotKey failed: {}", e))?;
    }

    HOTKEY_MAP
        .lock()
        .map_err(|e| format!("hotkey map lock: {}", e))?
        .insert(id, callback.to_string());

    Ok(id)
}

/// Unregister a hotkey by id.
#[cfg(all(feature = "native", target_os = "windows"))]
pub fn unregister_hotkey(id: i32) -> Result<(), String> {
    unsafe {
        UnregisterHotKey(HWND(std::ptr::null_mut()), id)
            .map_err(|e| format!("UnregisterHotKey failed: {}", e))?;
    }
    HOTKEY_MAP
        .lock()
        .map_err(|e| format!("hotkey map lock: {}", e))?
        .remove(&id);
    Ok(())
}

/// Poll pending hotkey events registered by the host app.
#[cfg(all(feature = "native", target_os = "windows"))]
pub fn poll_hotkey_events() -> Vec<String> {
    HOTKEY_EVENTS
        .lock()
        .map(|mut v| std::mem::take(&mut *v))
        .unwrap_or_default()
}

/// Register a global hotkey. Returns the hotkey id.
///
/// Global hotkeys are only implemented on Windows; this stub always fails.
#[cfg(all(feature = "native", not(target_os = "windows")))]
pub fn register_hotkey(_combo: &str, _callback: &str) -> Result<i32, String> {
    Err("Global hotkeys are only implemented on Windows.".to_string())
}

/// Unregister a hotkey by id.
///
/// Global hotkeys are only implemented on Windows; this stub is a no-op.
#[cfg(all(feature = "native", not(target_os = "windows")))]
pub fn unregister_hotkey(_id: i32) -> Result<(), String> {
    Ok(())
}

/// Poll pending hotkey events registered by the host app.
///
/// Global hotkeys are only implemented on Windows; this stub always returns
/// an empty list.
#[cfg(all(feature = "native", not(target_os = "windows")))]
pub fn poll_hotkey_events() -> Vec<String> {
    vec![]
}

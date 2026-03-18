//! Global hotkey registration for Poly (Windows only)
//! hotkey_register(combo, callback_name) -> id
//! hotkey_unregister(id)
//! hotkey_poll() -> list of triggered callback names

#[cfg(feature = "native")]
use std::collections::HashMap;
#[cfg(feature = "native")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "native")]
use once_cell::sync::Lazy;

#[cfg(feature = "native")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey,
    MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, HOT_KEY_MODIFIERS,
};
#[cfg(feature = "native")]
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};
#[cfg(feature = "native")]
use windows::Win32::Foundation::HWND;

/// Maps hotkey id -> callback function name
#[cfg(feature = "native")]
static HOTKEY_MAP: Lazy<Arc<Mutex<HashMap<i32, String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Pending hotkey events (callback names) to be polled
#[cfg(feature = "native")]
static HOTKEY_EVENTS: Lazy<Arc<Mutex<Vec<String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

#[cfg(feature = "native")]
static HOTKEY_THREAD_STARTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "native")]
static ID_CTR: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1);

/// Parse a combo string like "ctrl+shift+a" into (modifiers, vk_code)
#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
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

/// Poll pending hotkey events (callback names). Called from Poly scripts.
#[cfg(feature = "native")]
pub fn poll_hotkey_events() -> Vec<String> {
    HOTKEY_EVENTS
        .lock()
        .map(|mut v| std::mem::take(&mut *v))
        .unwrap_or_default()
}

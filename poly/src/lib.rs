#![doc = include_str!("../README.md")]
#![allow(rustdoc::bare_urls)]
#![doc = include_str!("../docs/RUST_API.md")]
#![doc = include_str!("../docs/FRAMEWORK_API.md")]
#![doc = "\n\n---\n\n# Optional CEF Renderer Guide\n\nCEF is an optional backend. Its runtime and packaging requirements are documented below.\n"]
#![doc = include_str!("../docs/CEF.md")]

pub mod web;
pub mod native;
pub mod updater;
pub mod tray;
pub mod ai;
pub mod clipboard;
pub mod window;
pub mod notification;
pub mod deeplink;
pub mod templates;
pub mod single_instance;
pub mod permissions;
pub mod browser;
pub mod titlebar;
pub mod webview;
pub mod webview_native;
pub mod cef_osr;
#[cfg(feature = "cef")]
/// Direct CEF API access for Poly render backends and native extensions.
pub use cef as cef_api;
pub mod multiview;
pub mod multiview_native;
pub mod polyview;
pub mod config;
pub mod cookies;
pub mod adblock;
pub mod adblock_engine;
pub mod adblock_init;
#[cfg(feature = "experimental-virtual-drive")]
#[path = "virtual_drive.rs"]
pub mod virtual_drive;
#[cfg(not(feature = "experimental-virtual-drive"))]
#[path = "virtual_drive_stub.rs"]
#[doc(hidden)]
pub mod virtual_drive;
pub mod theme;
pub mod db;
pub mod hotkeys;

// Re-export WebView types
pub use webview::{WebViewConfig, WebViewBounds, WebViewState, WebViewEvent, WebViewOperation};

// Re-export WebView functions for JavaScript API
pub use webview::{
    create as webview_create,
    navigate as webview_navigate,
    go_back as webview_go_back,
    go_forward as webview_go_forward,
    reload as webview_reload,
    set_bounds as webview_set_bounds,
    set_visible as webview_set_visible,
    focus as webview_focus,
    destroy as webview_destroy,
    take_events as webview_poll_events,  // Expose take_events as pollEvents
    take_events_json as webview_poll_events_json,  // JSON version for easier JS integration
    execute_script as webview_execute_script,  // Execute JS and get result
};

// Re-export browser mode types (BrowserConfig is always available, run_browser_window only with native)
pub use webview_native::BrowserConfig;
#[cfg(feature = "native")]
pub use webview_native::run_browser_window;

// Re-export multiview types
pub use multiview::{MultiViewWindowConfig, ViewConfig};
#[cfg(feature = "native")]
pub use multiview_native::create_multiview_window;



pub use native::{NativeConfig, run_native_window, run_native_url, generate_native_bundle};
#[cfg(feature = "native")]
pub use native::get_main_window_size;
pub use updater::{UpdateConfig, UpdateInfo, check_github_updates, check_custom_updates, download_update, install_update};
pub use tray::{TrayConfig, TrayMenuItem, TrayEvent, TrayHandle, create_tray};
pub use ai::{AiProvider, ChatMessage, ChatRequest, ChatResponse, StreamEvent, MessageRole, check_ollama, list_ollama_models, chat as ai_chat};
pub use single_instance::{SingleInstanceConfig, try_acquire_instance, release_instance, is_primary_instance, check_single_instance};
pub use permissions::{PermissionsConfig, Permission, check_permission, is_enabled as permissions_enabled};
pub use config::{PolyConfig, init_global_config, get_config};
pub use adblock::{init_adblock, should_block, download_easylist, download_easyprivacy, download_pihole_blocklist, parse_pihole_hosts, get_stats as adblock_stats};
pub use adblock_engine::{init_brave_adblock, should_block_brave, get_cosmetic_filters, get_stats as brave_adblock_stats};
pub use adblock_init::ensure_adblock_initialized;

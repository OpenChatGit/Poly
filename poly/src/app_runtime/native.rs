use super::*;

/// Resolve files from the project's top-level `assets/` directory without
/// allowing a request to escape that directory.
fn project_asset_path(assets_dir: &Path, url_path: &str) -> Option<std::path::PathBuf> {
    let relative = url_path.strip_prefix("assets/")?;
    if relative.is_empty() {
        return None;
    }

    let relative_path = Path::new(relative);
    if relative_path
        .components()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return None;
    }

    let path = assets_dir.join(relative_path);
    path.is_file().then_some(path)
}

fn resolve_configured_icon(icon_path: Option<&str>, app_root: &Path) -> Option<std::path::PathBuf> {
    let configured = Path::new(icon_path?);
    let resolved = if configured.is_absolute() {
        configured.to_path_buf()
    } else {
        app_root.join(configured)
    };
    resolved.is_file().then_some(resolved)
}

#[cfg(windows)]
fn stop_dev_process_tree(child: &mut std::process::Child) {
    use std::os::windows::process::CommandExt;

    let status = std::process::Command::new("taskkill")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .creation_flags(0x08000000)
        .status();
    if !matches!(status, Ok(status) if status.success()) {
        let _ = child.kill();
    }
    let _ = child.wait();
}

#[cfg(not(windows))]
fn stop_dev_process_tree(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod asset_path_tests {
    use super::{project_asset_path, resolve_configured_icon};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn resolves_project_assets_and_rejects_paths_outside_assets() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let project_dir = std::env::temp_dir().join(format!(
            "poly-assets-{}-{unique}",
            std::process::id()
        ));
        let assets_dir = project_dir.join("assets");
        fs::create_dir_all(assets_dir.join("logos")).unwrap();
        fs::write(assets_dir.join("logos/duck.svg"), "<svg/>").unwrap();

        assert_eq!(
            project_asset_path(&assets_dir, "assets/logos/duck.svg"),
            Some(assets_dir.join("logos/duck.svg"))
        );
        assert_eq!(project_asset_path(&assets_dir, "assets/missing.svg"), None);
        assert_eq!(project_asset_path(&assets_dir, "assets/../poly.toml"), None);
        assert_eq!(project_asset_path(&assets_dir, "other/duck.svg"), None);

        fs::remove_dir_all(project_dir).unwrap();
    }

    #[test]
    fn resolves_relative_window_icons_from_the_app_root() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let app_root = std::env::temp_dir().join(format!(
            "poly-window-icon-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(app_root.join("assets")).unwrap();
        fs::write(app_root.join("assets/duck.png"), b"png").unwrap();

        assert_eq!(
            resolve_configured_icon(Some("assets/duck.png"), &app_root),
            Some(app_root.join("assets/duck.png"))
        );
        fs::remove_dir_all(app_root).unwrap();
    }
}

pub(crate) fn run_native_app(project_path: &Path, release: bool, browser_flag: bool, ui_height_arg: u32) {
    #[cfg(not(feature = "native"))]
    let _ = (release, browser_flag, ui_height_arg);

    // CEF must be initialized on Poly's main application thread. Its own browser
    // UI loop runs separately; the CEF command actor is started lazily afterward.
    #[cfg(feature = "cef")]
    if let Err(error) = poly::cef_osr::initialize() {
        eprintln!("{}warning{}: CEF browser surfaces are unavailable: {}", YELLOW, RESET, error);
    }

    use std::sync::Arc;
    use std::thread;

    // Check if running from bundle (bundle folder exists)
    let bundle_dir = project_path.join("bundle");
    let is_bundled = bundle_dir.exists();
    let effective_path = if is_bundled { &bundle_dir } else { project_path };
    let runtime_config = poly::PolyConfig::load(if is_bundled { &bundle_dir } else { project_path });
    #[cfg(feature = "native")]
    let use_vite_dev_server = !is_bundled
        && !release
        && project_path.join("package.json").is_file()
        && crate::scaffold::has_vite_config(project_path);

    // Find web directory
    let web_dir = if effective_path.is_file() {
        effective_path.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        let web_path = effective_path.join(&runtime_config.web.dir);
        if web_path.exists() { web_path }
        else if effective_path.join("web").exists() { effective_path.join("web") }
        else if effective_path.join("index.html").exists() { effective_path.to_path_buf() }
        else {
            eprintln!("{}error{}: No web directory found", RED, RESET);
            std::process::exit(1);
        }
    };

    // Check for index.html
    if !web_dir.join("index.html").exists() {
        eprintln!("{}error{}: No index.html found in {}", RED, RESET, web_dir.display());
        std::process::exit(1);
    }

    // Get title from poly.toml or folder name
    let poly_toml_path = if is_bundled {
        bundle_dir.join("poly.toml")
    } else {
        project_path.join("poly.toml")
    };

    let title = if poly_toml_path.exists() {
        if let Ok(content) = fs::read_to_string(&poly_toml_path) {
            extract_toml_value(&content, "name")
                .unwrap_or_else(|| project_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Poly App")
                    .to_string())
        } else {
            project_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Poly App")
                .to_string()
        }
    } else {
        project_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Poly App")
            .to_string()
    };

    // Check for [browser] section in poly.toml OR --browser flag
    // Must be an actual section, not a comment
    #[cfg(feature = "native")]
    let browser_mode = browser_flag || if poly_toml_path.exists() {
        if let Ok(content) = fs::read_to_string(&poly_toml_path) {
            // Check for actual [browser] section (not commented out)
            content.lines().any(|line| {
                let trimmed = line.trim();
                trimmed == "[browser]"
            })
        } else {
            false
        }
    } else {
        false
    };

    // If browser mode is enabled, use the dual-WebView browser window
    #[cfg(feature = "native")]
    if browser_mode {
        // Parse browser config - use arg if provided, otherwise from poly.toml
        let mut ui_height: u32 = if ui_height_arg != 80 { ui_height_arg } else { 80 };
        let mut browser_width: u32 = 1200;
        let mut browser_height: u32 = 800;
        let mut dev_port: u16 = 0;
        let mut start_url: String = "https://google.com".to_string();

        if let Ok(content) = fs::read_to_string(&poly_toml_path) {
            let mut in_browser_section = false;
            let mut in_window_section = false;
            let mut in_dev_section = false;
            for line in content.lines() {
                let line = line.trim();
                if line == "[browser]" {
                    in_browser_section = true;
                    in_window_section = false;
                    in_dev_section = false;
                } else if line == "[window]" {
                    in_window_section = true;
                    in_browser_section = false;
                    in_dev_section = false;
                } else if line == "[dev]" {
                    in_dev_section = true;
                    in_browser_section = false;
                    in_window_section = false;
                } else if line.starts_with('[') {
                    in_browser_section = false;
                    in_window_section = false;
                    in_dev_section = false;
                } else if in_browser_section {
                    // Only use poly.toml ui_height if not overridden by arg
                    if ui_height_arg == 80 {
                        if let Some(val) = line.strip_prefix("ui_height").and_then(|s| s.trim().strip_prefix('=')) {
                            ui_height = val.trim().parse().unwrap_or(80);
                        }
                    }
                    // Parse start_url
                    if let Some(val) = line.strip_prefix("start_url").and_then(|s| s.trim().strip_prefix('=')) {
                        let val = val.trim().trim_matches('"');
                        if !val.is_empty() {
                            start_url = val.to_string();
                        }
                    }
                } else if in_window_section {
                    if let Some(val) = line.strip_prefix("width").and_then(|s| s.trim().strip_prefix('=')) {
                        browser_width = val.trim().parse().unwrap_or(1200);
                    } else if let Some(val) = line.strip_prefix("height").and_then(|s| s.trim().strip_prefix('=')) {
                        browser_height = val.trim().parse().unwrap_or(800);
                    }
                } else if in_dev_section {
                    if let Some(val) = line.strip_prefix("port").and_then(|s| s.trim().strip_prefix('=')) {
                        dev_port = val.trim().parse().unwrap_or(0);
                    }
                }
            }
        }

        // Find a free port
        let port = if dev_port > 0 { dev_port } else { find_free_port().unwrap_or(9473) };

        println!();
        println!("  {}POLY{} {}  {}browser mode{}", CYAN, RESET, VERSION, DIM, RESET);
        println!();
        println!("  {}>{} Local server: http://localhost:{}", DIM, RESET, port);
        println!("  {}>{} UI height: {}px", DIM, RESET, ui_height);
        println!("  {}>{} Window: {}x{}", DIM, RESET, browser_width, browser_height);
        println!("  {}>{} Start URL: {}", DIM, RESET, start_url);
        println!();

        // Load config for packages_dir
        let config = poly::PolyConfig::load(project_path);
        let packages_dir = effective_path.join(&config.web.packages_dir);
        let assets_dir = effective_path.join("assets");
        println!("  {}>{} Packages dir: {}", DIM, RESET, packages_dir.display());

        // Start local HTTP server for UI assets
        let web_dir_clone = web_dir.clone();
        let packages_dir_clone = packages_dir.clone();
        let assets_dir_clone = assets_dir.clone();
        thread::spawn(move || {
            let server = tiny_http::Server::http(format!("127.0.0.1:{}", port))
                .expect("Failed to start HTTP server");

            for request in server.incoming_requests() {
                let url = request.url().to_string();
                // `request.url()` includes the query string. Keep it for the browser, but don't
                // treat it as part of a static file's name (for example `index.html?embed=1`).
                let url_path = url.split('?').next().unwrap_or(&url).trim_start_matches('/');

                // Debug endpoint
                if url_path == "__poly_debug" {
                    let debug_info = format!("web_dir: {:?}\npackages_dir: {:?}", web_dir_clone, packages_dir_clone);
                    let _ = request.respond(tiny_http::Response::from_string(debug_info));
                    continue;
                }

                // Try to find the file: web/ first, then packages/
                let mut file_path = if url_path.is_empty() || url_path == "index.html" {
                    web_dir_clone.join("index.html")
                } else if url_path.starts_with("packages/") {
                    // Serve from packages directory
                    let pkg_path = url_path.strip_prefix("packages/").unwrap_or(url_path);
                    let full_path = packages_dir_clone.join(pkg_path);
                    eprintln!("[HTTP] packages request: {} -> {} (exists: {})", url_path, full_path.display(), full_path.exists());
                    full_path
                } else {
                    web_dir_clone.join(url_path)
                };

                if !file_path.is_file() {
                    if let Some(asset_path) = project_asset_path(&assets_dir_clone, url_path) {
                        file_path = asset_path;
                    }
                }

                // Serve app directories from their index document so SPAs
                // keep the configured base path when opened in a Poly surface.
                if file_path.is_dir() {
                    file_path = file_path.join("index.html");
                }

                let response = if file_path.exists() && file_path.is_file() {
                    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    let content_type = match ext {
                        "html" => "text/html; charset=utf-8",
                        "css" => "text/css; charset=utf-8",
                        "js" => "application/javascript; charset=utf-8",
                        "wasm" => "application/wasm",
                        "png" => "image/png",
                        "jpg" | "jpeg" => "image/jpeg",
                        "gif" => "image/gif",
                        "ico" => "image/x-icon",
                        "svg" => "image/svg+xml",
                        "woff" => "font/woff",
                        "woff2" => "font/woff2",
                        "ttf" => "font/ttf",
                        "otf" => "font/otf",
                        "json" => "application/json",
                        "webmanifest" => "application/manifest+json",
                        _ => "text/plain",
                    };

                    // Read binary files as bytes, text files as strings
                    if matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "ico" | "svg" | "wasm" | "woff" | "woff2" | "ttf" | "otf") {
                        match fs::read(&file_path) {
                            Ok(content) => tiny_http::Response::from_data(content)
                                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap()),
                            Err(_) => tiny_http::Response::from_string("Read Error").with_status_code(500),
                        }
                    } else {
                        match fs::read_to_string(&file_path) {
                            Ok(content) => tiny_http::Response::from_string(content)
                                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap()),
                            Err(_) => tiny_http::Response::from_string("Read Error").with_status_code(500),
                        }
                    }
                } else {
                    tiny_http::Response::from_string("Not Found").with_status_code(404)
                };

                let _ = request.respond(response);
            }
        });

        // Give server time to start
        thread::sleep(std::time::Duration::from_millis(100));

        // Look for icon
        let icon = resolve_configured_icon(
            runtime_config.window.icon_path.as_deref().or(runtime_config.build.icon_path.as_deref()),
            effective_path,
        )
            .or_else(|| {
                let fallback = effective_path.join("assets/icon.png");
                fallback.is_file().then_some(fallback)
            })
            .map(|path| path.to_string_lossy().to_string());

        // Use URL instead of HTML content
        let ui_url = format!("http://localhost:{}/index.html", port);

        let config = poly::BrowserConfig {
            title: title.clone(),
            width: browser_width,
            height: browser_height,
            ui_height,
            ui_html: ui_url, // This is now a URL, not HTML content
            start_url,
            devtools: true,
            icon_path: icon,
            decorations: false,
        };

        if let Err(e) = poly::run_browser_window(config) {
            eprintln!("{}error{}: Failed to run browser mode: {}", RED, RESET, e);
        }
        return;
    }

    // Initialize Permissions from poly.toml
    if poly_toml_path.exists() {
        poly::permissions::init_from_toml(&poly_toml_path, &title);
        if poly::permissions::is_enabled() {
            println!("  {}>{} Permissions: {}enabled{}", DIM, RESET, GREEN, RESET);
        }
    } else {
        // No poly.toml = development mode, permissions disabled
        poly::permissions::set_development_mode();
    }

    // Window config defaults

    // Window config defaults
    let mut window_width: u32 = 1024;
    let mut window_height: u32 = 768;
    let mut window_decorations = true; // Native titlebar by default
    let mut window_resizable = true;
    let mut window_transparent = false; // Transparent background for frameless
    let mut single_instance = false;

    // Dev/Server config
    let mut dev_port: u16 = 0; // 0 = auto-find free port

    // Parse window section from poly.toml
    if poly_toml_path.exists() {
        if let Ok(content) = fs::read_to_string(&poly_toml_path) {
            let mut in_window_section = false;
            let mut in_dev_section = false;
            for line in content.lines() {
                let line = line.trim();
                if line == "[window]" {
                    in_window_section = true;
                    in_dev_section = false;
                } else if line == "[dev]" {
                    in_dev_section = true;
                    in_window_section = false;
                } else if line.starts_with('[') && line != "[window]" && line != "[dev]" {
                    in_window_section = false;
                    in_dev_section = false;
                } else if in_window_section {
                    if let Some(val) = line.strip_prefix("width").and_then(|s| s.trim().strip_prefix('=')) {
                        window_width = val.trim().parse().unwrap_or(1024);
                    } else if let Some(val) = line.strip_prefix("height").and_then(|s| s.trim().strip_prefix('=')) {
                        window_height = val.trim().parse().unwrap_or(768);
                    } else if let Some(val) = line.strip_prefix("decorations").and_then(|s| s.trim().strip_prefix('=')) {
                        window_decorations = val.trim() == "true";
                    } else if let Some(val) = line.strip_prefix("resizable").and_then(|s| s.trim().strip_prefix('=')) {
                        window_resizable = val.trim() == "true";
                    } else if let Some(val) = line.strip_prefix("transparent").and_then(|s| s.trim().strip_prefix('=')) {
                        window_transparent = val.trim() == "true";
                    } else if let Some(val) = line.strip_prefix("single_instance").and_then(|s| s.trim().strip_prefix('=')) {
                        single_instance = val.trim() == "true";
                    }
                } else if in_dev_section {
                    if let Some(val) = line.strip_prefix("port").and_then(|s| s.trim().strip_prefix('=')) {
                        dev_port = val.trim().parse().unwrap_or(0);
                    }
                }
            }
        }
    }

    // Check single instance before starting
    if single_instance && !poly::check_single_instance(&title) {
        println!("  {}>{} Another instance is already running", YELLOW, RESET);
        std::process::exit(0);
    }

    let (tray_enabled, tray_tooltip, minimize_to_tray, close_to_tray, tray_menu_items) = if poly_toml_path.exists() {
        if let Ok(content) = fs::read_to_string(&poly_toml_path) {
            // Simple TOML parsing for tray section
            let mut enabled = false;
            let mut tooltip = title.to_string();
            let mut min_to_tray = false;
            let mut close_to_tray_val = false;
            let mut menu_items: Vec<(String, String)> = Vec::new();
            let mut in_tray_section = false;
            let mut in_menu_item = false;
            let mut current_id = String::new();
            let mut current_label = String::new();

            for line in content.lines() {
                let line = line.trim();
                if line == "[tray]" {
                    in_tray_section = true;
                    in_menu_item = false;
                } else if line == "[[tray.menu]]" {
                    // Save previous menu item if exists
                    if !current_id.is_empty() {
                        menu_items.push((current_id.clone(), current_label.clone()));
                    }
                    current_id = String::new();
                    current_label = String::new();
                    in_menu_item = true;
                    in_tray_section = false;
                } else if line.starts_with('[') && !line.starts_with("[[tray.menu]]") {
                    // Save last menu item
                    if !current_id.is_empty() {
                        menu_items.push((current_id.clone(), current_label.clone()));
                        current_id = String::new();
                        current_label = String::new();
                    }
                    in_tray_section = false;
                    in_menu_item = false;
                } else if in_tray_section {
                    if let Some(val) = line.strip_prefix("enabled").and_then(|s| s.trim().strip_prefix('=')) {
                        enabled = val.trim().trim_matches('"') == "true";
                    } else if let Some(val) = line.strip_prefix("tooltip").and_then(|s| s.trim().strip_prefix('=')) {
                        tooltip = val.trim().trim_matches('"').to_string();
                    } else if let Some(val) = line.strip_prefix("minimize_to_tray").and_then(|s| s.trim().strip_prefix('=')) {
                        min_to_tray = val.trim().trim_matches('"') == "true";
                    } else if let Some(val) = line.strip_prefix("close_to_tray").and_then(|s| s.trim().strip_prefix('=')) {
                        close_to_tray_val = val.trim().trim_matches('"') == "true";
                    }
                } else if in_menu_item {
                    if let Some(val) = line.strip_prefix("id").and_then(|s| s.trim().strip_prefix('=')) {
                        current_id = val.trim().trim_matches('"').to_string();
                    } else if let Some(val) = line.strip_prefix("label").and_then(|s| s.trim().strip_prefix('=')) {
                        current_label = val.trim().trim_matches('"').to_string();
                    }
                }
            }
            // Don't forget the last menu item
            if !current_id.is_empty() {
                menu_items.push((current_id, current_label));
            }

            (enabled, tooltip, min_to_tray, close_to_tray_val, menu_items)
        } else {
            (false, title.to_string(), false, false, Vec::new())
        }
    } else {
        (false, title.to_string(), false, false, Vec::new())
    };

    // Find a free port or use configured port
    let port = if dev_port > 0 {
        dev_port
    } else {
        find_free_port().unwrap_or(9473)
    };

    println!();
    println!("  {}POLY{} {}  {}native{}", CYAN, RESET, VERSION, DIM, RESET);
    println!();
    println!("  {}>{} Local server: http://localhost:{}", DIM, RESET, port);
    println!("  {}>{} Web dir: {}", DIM, RESET, web_dir.display());

    // Look for window icon file (PNG for taskbar/dock)
    // Check bundle/assets first, then project assets
    let icon_candidates = if is_bundled {
        vec![
            bundle_dir.join("assets/icon.png"),
            project_path.join("assets/icon.png"),
            project_path.join("icon.png"),
        ]
    } else {
        vec![
            project_path.join("assets/icon.png"),
            project_path.join("icon.png"),
        ]
    };
    let icon_path = resolve_configured_icon(
        runtime_config.window.icon_path.as_deref().or(runtime_config.build.icon_path.as_deref()),
        effective_path,
    )
        .or_else(|| icon_candidates.iter().find(|p| p.exists())
        .cloned()
        .or_else(|| {
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let candidates = [
                        exe_dir.join("assets/PolyNew.png"),
                        exe_dir.join("../poly/assets/PolyNew.png"),
                        exe_dir.join("../../poly/assets/PolyNew.png"),
                    ];
                    for c in candidates {
                        if c.exists() { return Some(c); }
                    }
                }
            }
            None
        }));

    let mut config = poly::NativeConfig::new(&title)
        .with_size(window_width, window_height)
        .with_decorations(window_decorations)
        .with_transparent(window_transparent)
        .with_dev_tools(true)
        .with_tray(tray_enabled)
        .with_minimize_to_tray(minimize_to_tray)
        .with_close_to_tray(close_to_tray)
        .with_single_instance(single_instance)
        .with_polyview(!project_path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.eq_ignore_ascii_case("ncos")));

    config.resizable = window_resizable;

    // Debug: show window config
    println!("  {}>{} Window: {}x{}, decorations={}", DIM, RESET, window_width, window_height, window_decorations);

    config.tray_tooltip = Some(tray_tooltip.clone());
    config.tray_menu_items = tray_menu_items;

    if let Some(icon) = icon_path.clone() {
        println!("  {}>{} Icon: {}", DIM, RESET, icon.display());
        config = config.with_icon(&icon.to_string_lossy());
    }

    // Use same icon for tray if available
    if let Some(ref icon) = icon_path {
        config = config.with_tray_icon(&icon.to_string_lossy());
    }

    if tray_enabled {
        println!("  {}>{} System Tray: enabled", DIM, RESET);
        if close_to_tray {
            println!("  {}>{} Close to tray: yes", DIM, RESET);
        }
    }

    if single_instance {
        println!("  {}>{} Single instance: enabled", DIM, RESET);
    }

    // Hot reload watcher info
    println!("  {}>{} Hot reload: watching {}", DIM, RESET, web_dir.display());

    println!();

    // Hot reload counter for native mode
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    let reload_counter = Arc::new(AtomicU64::new(1));
    let reload_counter_server = Arc::clone(&reload_counter);
    let reload_counter_watcher = Arc::clone(&reload_counter);

    // Start local HTTP server in background thread
    let web_dir_arc = Arc::new(web_dir);
    let web_dir_server = Arc::clone(&web_dir_arc);
    let active_wallpaper_streams = Arc::new(AtomicUsize::new(0));
    let active_ncos_media_streams = Arc::new(AtomicUsize::new(0));
    let project_path_for_watcher = project_path.to_path_buf();
    let project_path_owned = project_path.to_path_buf();
    let assets_dir_server = effective_path.join("assets");
    let decorations_for_server = window_decorations;

    // Keep CEF's high-frequency frame reads from blocking static files, app
    // API requests, and page navigation on the main HTTP accept loop.
    let (cef_frame_tx, cef_frame_rx) = std::sync::mpsc::sync_channel::<tiny_http::Request>(2);
    thread::Builder::new().name("poly-cef-frame-http".into()).spawn(move || {
        while let Ok(request) = cef_frame_rx.recv() {
            respond_cef_frame_request(request);
        }
    }).expect("Failed to start CEF frame response worker");

    thread::spawn(move || {
        let server = tiny_http::Server::http(format!("127.0.0.1:{}", port))
            .expect("Failed to start local server");


        for mut request in server.incoming_requests() {
            let url = request.url().to_string();

            // Serve VFS media with byte ranges so native HTML media elements can
            // seek and decode files without loading whole files into memory.
            if url.starts_with("/__ncos/drive/media?") {
                if request.method().as_str() != "GET" {
                    let _ = request.respond(tiny_http::Response::from_string("GET is required.").with_status_code(405));
                    continue;
                }
                let (path, content_type) = match virtual_drive_media_info(&url) {
                    Ok(value) => value,
                    Err(error) => {
                        let _ = request.respond(tiny_http::Response::from_string(error).with_status_code(400));
                        continue;
                    }
                };
                let mut active = active_ncos_media_streams.load(Ordering::Acquire);
                let stream_slot_acquired = loop {
                    if active >= 4 { break false; }
                    match active_ncos_media_streams.compare_exchange_weak(active, active + 1, Ordering::AcqRel, Ordering::Acquire) {
                        Ok(_) => break true,
                        Err(actual) => active = actual,
                    }
                };
                if !stream_slot_acquired {
                    let _ = request.respond(tiny_http::Response::from_string("NCOS media stream limit reached").with_status_code(503));
                    continue;
                }
                let range = request.headers().iter()
                    .find(|header| header.field.as_str().as_str().eq_ignore_ascii_case("range"))
                    .map(|header| header.value.as_str().to_string());
                let active_streams = Arc::clone(&active_ncos_media_streams);
                thread::spawn(move || {
                    let response = range_file_response(&path, range.as_deref(), &content_type);
                    let _ = request.respond(response);
                    active_streams.fetch_sub(1, Ordering::AcqRel);
                });
                continue;
            }

            // Stream wallpaper video data on a separate worker and support byte ranges,
            // so large videos neither allocate their full size in memory nor hold the UI server loop.
            if let Some((path, content_type)) = wallpaper_video_info(&url, &web_dir_server) {
                let mut active = active_wallpaper_streams.load(Ordering::Acquire);
                let stream_slot_acquired = loop {
                    if active >= 2 { break false; }
                    match active_wallpaper_streams.compare_exchange_weak(active, active + 1, Ordering::AcqRel, Ordering::Acquire) {
                        Ok(_) => break true,
                        Err(actual) => active = actual,
                    }
                };
                if !stream_slot_acquired {
                    let _ = request.respond(tiny_http::Response::from_string("Wallpaper video stream limit reached")
                        .with_status_code(503));
                    continue;
                }
                let range = request.headers().iter()
                    .find(|header| header.field.as_str().as_str().eq_ignore_ascii_case("range"))
                    .map(|header| header.value.as_str().to_string());
                let active_streams = Arc::clone(&active_wallpaper_streams);
                thread::spawn(move || {
                    let response = range_file_response(&path, range.as_deref(), &content_type);
                    let _ = request.respond(response);
                    active_streams.fetch_sub(1, Ordering::AcqRel);
                });
                continue;
            }

            if url.starts_with("/__poly_cef_frame") {
                match cef_frame_tx.try_send(request) {
                    Ok(()) => {}
                    Err(std::sync::mpsc::TrySendError::Full(request)) => {
                        let _ = request.respond(tiny_http::Response::from_string("CEF frame worker is busy")
                            .with_status_code(503));
                    }
                    Err(std::sync::mpsc::TrySendError::Disconnected(request)) => {
                        let _ = request.respond(tiny_http::Response::from_string("CEF frame worker is unavailable")
                            .with_status_code(503));
                    }
                }
                continue;
            }

            // NCOS wallpaper catalog, generated from the app's wallpaper folder.
            if url == "/__poly_invoke_stream" {
                let mut body = String::new();
                request.as_reader().read_to_string(&mut body).ok();
                let response = match invoke_ipc_plugin_stream(&body, web_dir_server.as_path()) {
                    Ok(reader) => tiny_http::Response::new(
                        tiny_http::StatusCode(200),
                        vec![
                            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/x-ndjson; charset=utf-8"[..]).unwrap(),
                            tiny_http::Header::from_bytes(&b"Cache-Control"[..], &b"no-cache, no-transform"[..]).unwrap(),
                        ],
                        reader,
                        None,
                        None,
                    ).boxed(),
                    Err(error) => tiny_http::Response::from_string(format!("{}\n", serde_json::json!({"type":"error","error":error})))
                        .with_status_code(500)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/x-ndjson; charset=utf-8"[..]).unwrap())
                        .boxed(),
                };
                let _ = request.respond(response);
                continue;
            }

            let response = if url == "/__ncos_wallpapers" {
                let wallpaper_dir = web_dir_server.join("assets").join("wallpapers");
                let mut files: Vec<String> = fs::read_dir(&wallpaper_dir)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter_map(|entry| {
                        let path = entry.path();
                        if !path.is_file() { return None; }
                        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
                        if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "avif" | "gif" | "bmp" | "apng" | "mp4" | "webm" | "m4v" | "ogv" | "mov" | "mkv" | "avi" | "wmv" | "flv" | "3gp" | "mpeg" | "mpg" | "ts") { return None; }
                        path.file_name()?.to_str().map(str::to_owned)
                    })
                    .collect();
                files.sort_by_key(|name| name.to_ascii_lowercase());
                tiny_http::Response::from_string(serde_json::to_string(&files).unwrap_or_else(|_| "[]".to_string()))
                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap())
            }
            // Import a host-selected file into the NCOS virtual C: drive.
            else if url.starts_with("/__ncos/drive/import?") {
                let query = url.split_once('?').map(|(_, query)| query).unwrap_or("");
                let mut directory = None;
                let mut name = None;
                for pair in query.split('&') {
                    if let Some((key, value)) = pair.split_once('=') {
                        let key = urlencoding::decode(key).unwrap_or_default();
                        let value = urlencoding::decode(value).unwrap_or_default().into_owned();
                        match key.as_ref() { "directory" => directory = Some(value), "name" => name = Some(value), _ => {} }
                    }
                }
                let content_length = request.headers().iter()
                    .find(|header| header.field.as_str().as_str().eq_ignore_ascii_case("content-length"))
                    .and_then(|header| header.value.as_str().parse::<u64>().ok());
                let result = if request.method().as_str() != "POST" {
                    Err("POST is required.".to_string())
                } else if content_length.is_some_and(|length| length > poly::virtual_drive::MAX_IMPORTED_FILE_BYTES) {
                    Err("Imported files must be 1 GiB or smaller.".to_string())
                } else if let (Some(directory), Some(name)) = (directory, name) {
                    poly::virtual_drive::import_file(&directory, &name, request.as_reader())
                } else {
                    Err("The destination directory or file name is missing.".to_string())
                };
                match result {
                    Ok(value) => tiny_http::Response::from_string(value.to_string())
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap()),
                    Err(error) => tiny_http::Response::from_string(serde_json::json!({"ok": false, "error": error}).to_string())
                        .with_status_code(400)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap()),
                }
            }
            // Install a bundled NCOS app package into the user's virtual C: drive.
            else if let Some(app_id) = url.strip_prefix("/__ncos/apps/install/") {
                if request.method().as_str() != "POST" {
                    tiny_http::Response::from_string(r#"{"ok":false,"error":"POST is required."}"#)
                        .with_status_code(405)
                } else if app_id.contains('/') || app_id.contains('\\') || poly::virtual_drive::validate_app_id(app_id).is_err() {
                    tiny_http::Response::from_string(r#"{"ok":false,"error":"Invalid app package ID."}"#)
                        .with_status_code(400)
                } else {
                    let mut body = String::new();
                    let _ = request.as_reader().read_to_string(&mut body);
                    let version = serde_json::from_str::<serde_json::Value>(&body).ok()
                        .and_then(|value| value.get("version").and_then(|version| version.as_str()).map(str::to_owned))
                        .unwrap_or_else(|| "1.0.0".to_string());
                    let (source, version, entry, module, styles) = if app_id == "notehaven" {
                        (web_dir_server.join("installed").join("note-haven"), version, "/installed/note-haven/index.html".to_string(), Some("/installed/note-haven/assets/ncos-entry.js".to_string()), vec!["/installed/note-haven/assets/ncos-entry.css"])
                    } else {
                        (web_dir_server.join("app-packages").join(app_id), version, format!("/__ncos/apps/{app_id}/index.html"), None, Vec::<&str>::new())
                    };
                    if !source.join("index.html").is_file() {
                        tiny_http::Response::from_string(serde_json::json!({ "ok": false, "error": format!("No bundled app package exists for '{app_id}'.") }).to_string())
                            .with_status_code(404)
                            .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap())
                    } else {
                        match poly::virtual_drive::install_bundled_app(app_id, &source, &version) {
                            Ok(installed) => tiny_http::Response::from_string(serde_json::json!({
                                "ok": true,
                                "installed": installed,
                                "path": format!("C:/System/Apps/{app_id}/Package"),
                                "entry": entry,
                                "module": module,
                                "styles": styles
                            }).to_string())
                                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap()),
                            Err(error) => tiny_http::Response::from_string(serde_json::json!({ "ok": false, "error": error }).to_string())
                                .with_status_code(500)
                                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap()),
                        }
                    }
                }
            }
            // Hot reload endpoint
            else if url == "/__poly_reload" {
                let current = reload_counter_server.load(Ordering::Relaxed);
                tiny_http::Response::from_string(format!(r#"{{"version":{}}}"#, current))
                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
            }
            // Proxy endpoint - fetches external URLs and returns content
            // Usage: /__poly_proxy?url=https://example.com/path
            else if url.starts_with("/__poly_proxy") {
                let target_url = url.strip_prefix("/__poly_proxy?url=")
                    .or_else(|| url.strip_prefix("/__poly_proxy/?url="))
                    .unwrap_or("");

                // URL decode
                let target_url = urlencoding::decode(target_url).unwrap_or_default().to_string();

                if target_url.is_empty() {
                    tiny_http::Response::from_string(r#"{"error":"Missing url parameter"}"#)
                        .with_status_code(400)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                } else {
                    // Check permissions
                    let allowed = if poly::permissions_enabled() {
                        let domain = target_url.split("://")
                            .nth(1)
                            .and_then(|s| s.split('/').next())
                            .unwrap_or(&target_url);
                        let perm = poly::Permission::HttpConnect(poly::permissions::DomainScope::Domain(domain.to_string()));
                        poly::check_permission(&perm).is_ok()
                    } else {
                        true
                    };

                    if !allowed {
                        tiny_http::Response::from_string(r#"{"error":"Domain not allowed"}"#)
                            .with_status_code(403)
                            .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                    } else {
                        match reqwest::blocking::get(&target_url) {
                            Ok(resp) => {
                                let content_type = resp.headers()
                                    .get("content-type")
                                    .and_then(|v| v.to_str().ok())
                                    .unwrap_or("application/octet-stream")
                                    .to_string();

                                match resp.bytes() {
                                    Ok(body) => {
                                        let mut response = tiny_http::Response::from_data(body.to_vec());
                                        response = response.with_header(
                                            tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap()
                                        );
                                        // Add CORS headers
                                        response = response.with_header(
                                            tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap()
                                        );
                                        response
                                    }
                                    Err(e) => {
                                        tiny_http::Response::from_string(format!(r#"{{"error":"{}"}}"#, e))
                                            .with_status_code(500)
                                            .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                                    }
                                }
                            }
                            Err(e) => {
                                tiny_http::Response::from_string(format!(r#"{{"error":"{}"}}"#, e))
                                    .with_status_code(500)
                                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                            }
                        }
                    }
                }
            }
            // PolyView Proxy - "iframe2" that bypasses all iframe restrictions
            // Usage: /__polyview/?url=https://example.com
            // Supports GET and POST requests
            else if url.starts_with("/__polyview") {
                let target_url = url.strip_prefix("/__polyview/?url=")
                    .or_else(|| url.strip_prefix("/__polyview?url="))
                    .or_else(|| url.strip_prefix("/__polyview/"))
                    .or_else(|| url.strip_prefix("/__polyview"))
                    .unwrap_or("");

                // URL decode
                let target_url = urlencoding::decode(target_url).unwrap_or_default().to_string();

                if target_url.is_empty() {
                    tiny_http::Response::from_string(r#"<html><body>Missing URL parameter</body></html>"#)
                        .with_status_code(400)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap())
                } else {
                    // Check request method
                    let method = request.method().as_str();
                    if method == "POST" {
                        // Read POST body
                        let mut body = Vec::new();
                        request.as_reader().read_to_end(&mut body).ok();

                        // Get content type
                        let content_type = request.headers()
                            .iter()
                            .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case("content-type"))
                            .map(|h| h.value.as_str().to_string())
                            .unwrap_or_else(|| "application/x-www-form-urlencoded".to_string());

                        handle_polyview_proxy_post(&target_url, port, &body, &content_type)
                    } else {
                        // GET request
                        handle_polyview_proxy(&target_url, port)
                    }
                }
            }
            // Serve window config (decorations, etc.)
            else if url == "/__poly_config" {
                tiny_http::Response::from_string(format!(r#"{{"decorations":{}}}"#, decorations_for_server))
                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
            }
            // Handle JavaScript system APIs
            else if url == "/__poly_invoke" {
                let mut body = String::new();
                request.as_reader().read_to_string(&mut body).ok();
                let result = invoke_ipc_system_only(&body, Some(web_dir_server.as_path()));
                tiny_http::Response::from_string(result)
                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
            } else {
                // Try web dir first, then packages dir
                // Strip query parameters before resolving the file path. Embedded app pages use
                // them for startup options, e.g. `vectorcraft/index.html?polyEmbed=1`.
                let request_path = url.split('?').next().unwrap_or(&url).trim_start_matches('/');
                let decoded_url_path = urlencoding::decode(request_path).unwrap_or_else(|_| request_path.into());
                let url_path = decoded_url_path.as_ref();
                let mut file_path = if url_path.is_empty() || url_path == "index.html" {
                    web_dir_server.join("index.html")
                } else if url_path.starts_with("packages/") {
                    // Serve from packages directory (project root)
                    project_path_owned.join(url_path)
                } else if url_path.starts_with("__ncos/apps/") {
                    let package_route = url_path.strip_prefix("__ncos/apps/").unwrap_or_default();
                    if let Some((app_id, relative)) = package_route.split_once('/') {
                        let safe_relative = !relative.split('/').any(|part| part == "." || part == ".." || part.contains(':'));
                        if safe_relative && poly::virtual_drive::validate_app_id(app_id).is_ok() {
                            poly::virtual_drive::installed_app_file(app_id, relative)
                                .unwrap_or_else(|_| web_dir_server.join("__missing_ncos_app_package__"))
                        } else {
                            web_dir_server.join("__invalid_ncos_package_path__")
                        }
                    } else {
                        web_dir_server.join("__invalid_ncos_package_path__")
                    }
                } else if let Some(relative) = url_path.strip_prefix("installed/note-haven/") {
                    let safe_relative = !relative.is_empty()
                        && !relative.split('/').any(|part| part.is_empty() || part == "." || part == ".." || part.contains(':'));
                    if safe_relative {
                        poly::virtual_drive::installed_app_file("notehaven", relative)
                            .unwrap_or_else(|_| web_dir_server.join(url_path))
                    } else {
                        web_dir_server.join("__invalid_ncos_package_path__")
                    }
                } else {
                    web_dir_server.join(url_path)
                };

                if !file_path.is_file() {
                    if let Some(asset_path) = project_asset_path(
                        &assets_dir_server,
                        url_path,
                    ) {
                        file_path = asset_path;
                    }
                }

                if file_path.exists() && file_path.is_file() {
                    let content = fs::read(&file_path).unwrap_or_default();
                    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                    let ct = match ext.as_str() {
                        "html" => "text/html; charset=utf-8",
                        "css" => "text/css; charset=utf-8",
                        "js" => "application/javascript; charset=utf-8",
                        "wasm" => "application/wasm",
                        "json" => "application/json",
                        "png" => "image/png",
                        "jpg" | "jpeg" => "image/jpeg",
                        "gif" => "image/gif",
                        "bmp" => "image/bmp",
                        "apng" => "image/apng",
                        "webp" => "image/webp",
                        "avif" => "image/avif",
                        "mp4" | "m4v" => "video/mp4",
                        "mov" => "video/quicktime",
                        "webm" => "video/webm",
                        "ogv" => "video/ogg",
                        "mkv" => "video/x-matroska",
                        "avi" => "video/x-msvideo",
                        "wmv" => "video/x-ms-wmv",
                        "flv" => "video/x-flv",
                        "3gp" => "video/3gpp",
                        "mpeg" | "mpg" => "video/mpeg",
                        "ts" => "video/mp2t",
                        "svg" => "image/svg+xml",
                        "woff" => "font/woff",
                        "woff2" => "font/woff2",
                        "ttf" => "font/ttf",
                        "otf" => "font/otf",
                        _ => "application/octet-stream",
                    };

                    tiny_http::Response::from_data(content)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], ct.as_bytes()).unwrap())
                } else {
                    tiny_http::Response::from_string("Not Found").with_status_code(404)
                }
            };
            let _ = request.respond(response);
        }
    });


    // File watcher for hot reload
    thread::spawn(move || {
        use notify::{Watcher, RecursiveMode};
        use std::sync::mpsc::channel;

        let (tx, rx) = channel();
        let mut watcher = match notify::recommended_watcher(move |res| { let _ = tx.send(res); }) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("  {}warning{}: Could not start file watcher: {}", YELLOW, RESET, e);
                return;
            }
        };

        // Watch the web directory within the project
        let web_path = project_path_for_watcher.join("web");
        let watch_path = if web_path.exists() { web_path } else { project_path_for_watcher.clone() };

        if let Err(e) = watcher.watch(&watch_path, RecursiveMode::Recursive) {
            eprintln!("  {}warning{}: Could not watch directory: {}", YELLOW, RESET, e);
        }

        loop {
            if let Ok(Ok(event)) = rx.recv() {
                    // Reload frontend assets after relevant file changes (HTML/CSS/JS/JSON/SVG/Poly)
                    let dominated_by_relevant = event.paths.iter().any(|p| {
                        let ext = p.extension().and_then(|e| e.to_str());
                        matches!(ext, Some("html") | Some("css") | Some("js") | Some("json") | Some("svg"))
                    });

                    if dominated_by_relevant {
                        reload_counter_watcher.fetch_add(1, Ordering::Relaxed);
                    }
            }
        }
    });

    // Give server time to start
    std::thread::sleep(std::time::Duration::from_millis(100));

    #[cfg(feature = "native")]
    let vite_port = if use_vite_dev_server {
        match std::net::TcpListener::bind(("127.0.0.1", 0)) {
            Ok(listener) => listener.local_addr().map(|address| address.port()).unwrap_or(5173),
            Err(_) => 5173,
        }
    } else {
        5173
    };

    #[cfg(feature = "native")]
    let mut vite_process = if use_vite_dev_server {
        let backend_url = format!("http://127.0.0.1:{port}");
        let vite_cli = project_path.join("node_modules/vite/bin/vite.js");
        let mut command = if vite_cli.is_file() {
            let mut command = std::process::Command::new("node");
            command.arg(vite_cli).args(["--host", "127.0.0.1", "--port"]);
            command
        } else {
            let mut command = crate::scaffold::npm_command();
            command.args(["run", "dev", "--", "--host", "127.0.0.1", "--port"]);
            command
        };
        command.arg(vite_port.to_string())
            .current_dir(project_path)
            // The dev server never reads interactive input. Keep it from sharing
            // the foreground terminal's stdin with the Poly process.
            .stdin(std::process::Stdio::null())
            .env("POLY_BACKEND_URL", &backend_url);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        match command.spawn() {
            Ok(mut child) => {
                let mut ready = false;
                for _ in 0..100 {
                    if std::net::TcpStream::connect_timeout(
                        &format!("127.0.0.1:{vite_port}").parse().unwrap(),
                        std::time::Duration::from_millis(100),
                    ).is_ok() {
                        ready = true;
                        break;
                    }
                    if let Ok(Some(status)) = child.try_wait() {
                        eprintln!("{}error{}: Vite exited during startup with {}", RED, RESET, status);
                        std::process::exit(1);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                if !ready {
                    let _ = child.kill();
                    eprintln!("{}error{}: Vite did not start on 127.0.0.1:{}", RED, RESET, vite_port);
                    std::process::exit(1);
                }
                println!("  {}>{} Frontend: http://127.0.0.1:{} (Vite + HMR)", DIM, RESET, vite_port);
                Some(child)
            }
            Err(error) => {
                eprintln!("{}error{}: Could not start Vite: {}", RED, RESET, error);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    // Standard mode: Single WebView - App handles everything via APIs
    #[cfg(feature = "native")]
    let url = if vite_process.is_some() {
        format!("http://127.0.0.1:{vite_port}")
    } else {
        format!("http://localhost:{}", port)
    };
    #[cfg(not(feature = "native"))]
    let url = format!("http://localhost:{}", port);

    #[cfg(all(feature = "native", feature = "experimental-virtual-drive"))]
    poly::native::register_shutdown_hook(native_app::shutdown_all);
    let run_result = poly::run_native_url(&url, config);
    #[cfg(feature = "native")]
    if let Some(mut child) = vite_process.take() {
        stop_dev_process_tree(&mut child);
    }
    if let Err(e) = run_result {
        eprintln!("{}error{}: {}", RED, RESET, e);
        eprintln!();
        eprintln!("  {}hint{}: Build with native feature enabled:", DIM, RESET);
        eprintln!("    cargo build -p poly-framework --features native");
        std::process::exit(1);
    }
}

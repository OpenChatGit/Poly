use super::*;

pub(crate) fn run_native_app(project_path: &Path, _release: bool, browser_flag: bool, ui_height_arg: u32) {
    #[cfg(not(feature = "native"))]
    let _ = (browser_flag, ui_height_arg);

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

        // Hide console window on Windows
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Console::{GetConsoleWindow, FreeConsole};
            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
            unsafe {
                let console = GetConsoleWindow();
                if !console.0.is_null() {
                    let _ = ShowWindow(console, SW_HIDE);
                    FreeConsole().ok();
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
        println!("  {}>{} Packages dir: {}", DIM, RESET, packages_dir.display());

        // Start local HTTP server for UI assets
        let web_dir_clone = web_dir.clone();
        let packages_dir_clone = packages_dir.clone();
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
        let icon_path = project_path.join("assets/icon.png");
        let icon = if icon_path.exists() { Some(icon_path.to_string_lossy().to_string()) } else { None };

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
    let mut inject_alpine = false;  // Default: don't inject (user decides)
    let mut inject_lucide = false;  // Default: don't inject (user decides)

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
                    } else if let Some(val) = line.strip_prefix("inject_alpine").and_then(|s| s.trim().strip_prefix('=')) {
                        inject_alpine = val.trim() == "true";
                    } else if let Some(val) = line.strip_prefix("inject_lucide").and_then(|s| s.trim().strip_prefix('=')) {
                        inject_lucide = val.trim() == "true";
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
    let icon_path = icon_candidates.iter().find(|p| p.exists())
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
        });

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

    // Create persistent interpreter for native mode
    use std::sync::Mutex;
    let interpreter: Arc<Mutex<poly::interpreter::Interpreter>> = Arc::new(Mutex::new(poly::create_interpreter()));
    let interpreter_server = Arc::clone(&interpreter);
    let interpreter_watcher = Arc::clone(&interpreter);

    // Start local HTTP server in background thread
    let web_dir_arc = Arc::new(web_dir);
    let web_dir_server = Arc::clone(&web_dir_arc);
    let active_wallpaper_streams = Arc::new(AtomicUsize::new(0));
    let active_ncos_media_streams = Arc::new(AtomicUsize::new(0));
    let project_path_for_watcher = project_path.to_path_buf();
    let project_path_owned = project_path.to_path_buf();
    let entry_path = find_entry_point(effective_path);
    let entry_path_for_init = entry_path.clone();
    let entry_path_for_server = entry_path.clone();
    let decorations_for_server = window_decorations;

    // Keep CEF's high-frequency frame reads from blocking static files, app
    // API requests, and page navigation on the main HTTP accept loop.
    let (cef_frame_tx, cef_frame_rx) = std::sync::mpsc::sync_channel::<tiny_http::Request>(2);
    thread::Builder::new().name("poly-cef-frame-http".into()).spawn(move || {
        while let Ok(request) = cef_frame_rx.recv() {
            respond_cef_frame_request(request);
        }
    }).expect("Failed to start CEF frame response worker");

    // Initialize interpreter with source
    if let Some(ref entry) = entry_path_for_init {
        let source = match fs::read_to_string(entry) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("\n  {}×{} {}Could not read Poly entry point{}\n    {}\n    {}", RED, RESET, BOLD, RESET, entry.display(), error);
                std::process::exit(1);
            }
        };
        let mut interp = interpreter.lock().unwrap();
        if let Err(e) = poly::init_interpreter(&mut interp, &source) {
            eprintln!("{}error{}: Failed to initialize interpreter: {}", RED, RESET, e);
        }
    }

    thread::spawn(move || {
        let server = tiny_http::Server::http(format!("127.0.0.1:{}", port))
            .expect("Failed to start local server");

        let entry_path = entry_path_for_server;

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
            // Handle IPC invoke (stateful)
            else if url == "/__poly_invoke" {
                let mut body = String::new();
                request.as_reader().read_to_string(&mut body).ok();

                let result = if entry_path.is_some() {
                    invoke_ipc_stateful(&interpreter_server, &body)
                } else {
                    // No entry point, only handle system APIs
                    invoke_ipc_system_only(&body)
                };

                tiny_http::Response::from_string(result)
                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
            } else {
                // Try web dir first, then packages dir
                // Strip query parameters before resolving the file path. Embedded app pages use
                // them for startup options, e.g. `vectorcraft/index.html?polyEmbed=1`.
                let request_path = url.split('?').next().unwrap_or(&url).trim_start_matches('/');
                let decoded_url_path = urlencoding::decode(request_path).unwrap_or_else(|_| request_path.into());
                let url_path = decoded_url_path.as_ref();
                let file_path = if url_path.is_empty() || url_path == "index.html" {
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

                    // Inject scripts based on poly.toml [dev] section config
                    let final_content = if ct.starts_with("text/html") {
                        let mut html = String::from_utf8_lossy(&content).to_string();

                        // Only inject if enabled in poly.toml [dev] section
                        if inject_alpine && !html.contains("alpine") && !html.contains("Alpine") {
                            let alpine_script = r#"<script defer src="https://unpkg.com/alpinejs@3/dist/cdn.min.js"></script>"#;
                            if html.contains("</head>") {
                                html = html.replace("</head>", &format!("{}</head>", alpine_script));
                            }
                        }
                        if inject_lucide && !html.contains("lucide") {
                            let lucide_script = r#"<script src="https://unpkg.com/lucide@latest/dist/umd/lucide.min.js"></script>"#;
                            if html.contains("</head>") {
                                html = html.replace("</head>", &format!("{}</head>", lucide_script));
                            }
                        }

                        // Inject IPC Bridge and Poly API before </body>
                        let body_script = r##"<script>
// Poly Window API - User builds their own titlebar
(function() {
  const isNative = typeof window.ipc !== 'undefined' || window.location.port === '9473';

  if (isNative) {
    // Helper function to send IPC message
    const sendIPC = (msg) => {
      if (window.ipc && window.ipc.postMessage) {
        window.ipc.postMessage(msg);
      }
    };

    // Expose window control API
    window.polyWindow = {
      minimize: () => sendIPC('minimize'),
      maximize: () => sendIPC('maximize'),
      close: () => sendIPC('close'),
      drag: () => sendIPC('drag'),
      // Check if running in frameless mode
      isFrameless: () => fetch('/__poly_config').then(r => r.json()).then(d => d.decorations === false).catch(() => false)
    };

    console.log('[Poly] Window API ready: polyWindow.minimize(), polyWindow.maximize(), polyWindow.close(), polyWindow.drag()');
  }
})();

// Poly Dialog System - Custom In-App Dialogs
(function() {
  const style = document.createElement('style');
  style.textContent = `
    .poly-dialog-overlay {
      position: fixed; inset: 0; background: rgba(0,0,0,0.6); backdrop-filter: blur(4px);
      display: flex; align-items: center; justify-content: center; z-index: 99999;
      opacity: 0; transition: opacity 0.15s ease;
    }
    .poly-dialog-overlay.show { opacity: 1; }
    .poly-dialog {
      background: #1a1a1f; border: 1px solid rgba(255,255,255,0.1); border-radius: 12px;
      padding: 1.5rem; min-width: 320px; max-width: 90vw; box-shadow: 0 25px 50px rgba(0,0,0,0.5);
      transform: scale(0.95) translateY(-10px); transition: transform 0.15s ease;
    }
    .poly-dialog-overlay.show .poly-dialog { transform: scale(1) translateY(0); }
    .poly-dialog-icon { width: 48px; height: 48px; border-radius: 50%; display: flex; align-items: center; justify-content: center; margin: 0 auto 1rem; }
    .poly-dialog-icon.info { background: rgba(59,130,246,0.2); color: #3b82f6; }
    .poly-dialog-icon.warning { background: rgba(245,158,11,0.2); color: #f59e0b; }
    .poly-dialog-icon.error { background: rgba(239,68,68,0.2); color: #ef4444; }
    .poly-dialog-icon.confirm { background: rgba(93,193,210,0.2); color: #5dc1d2; }
    .poly-dialog-icon svg { width: 24px; height: 24px; }
    .poly-dialog-title { font-size: 1.1rem; font-weight: 600; color: #fff; text-align: center; margin-bottom: 0.5rem; }
    .poly-dialog-message { color: #888; text-align: center; font-size: 0.9rem; line-height: 1.5; margin-bottom: 1.5rem; }
    .poly-dialog-buttons { display: flex; gap: 0.75rem; justify-content: center; }
    .poly-dialog-btn {
      padding: 0.6rem 1.25rem; border-radius: 8px; font-size: 0.85rem; font-weight: 500;
      cursor: pointer; border: none; transition: all 0.15s;
    }
    .poly-dialog-btn-primary { background: linear-gradient(135deg, #5dc1d2, #1e80ad); color: #fff; }
    .poly-dialog-btn-primary:hover { transform: translateY(-1px); box-shadow: 0 4px 12px rgba(93,193,210,0.3); }
    .poly-dialog-btn-secondary { background: rgba(255,255,255,0.1); color: #888; }
    .poly-dialog-btn-secondary:hover { background: rgba(255,255,255,0.15); color: #fff; }
  `;
  document.head.appendChild(style);

  const icons = {
    info: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/></svg>',
    warning: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0zM12 9v4M12 17h.01"/></svg>',
    error: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M15 9l-6 6M9 9l6 6"/></svg>',
    confirm: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 015.83 1c0 2-3 3-3 3M12 17h.01"/></svg>'
  };

  window.__polyDialog = {
    show(type, title, message, buttons) {
      return new Promise(resolve => {
        const overlay = document.createElement('div');
        overlay.className = 'poly-dialog-overlay';
        overlay.innerHTML = `
          <div class="poly-dialog">
            <div class="poly-dialog-icon ${type}">${icons[type] || icons.info}</div>
            <div class="poly-dialog-title">${title}</div>
            <div class="poly-dialog-message">${message}</div>
            <div class="poly-dialog-buttons"></div>
          </div>
        `;
        const btnContainer = overlay.querySelector('.poly-dialog-buttons');
        buttons.forEach((btn, i) => {
          const el = document.createElement('button');
          el.className = `poly-dialog-btn ${btn.primary ? 'poly-dialog-btn-primary' : 'poly-dialog-btn-secondary'}`;
          el.textContent = btn.text;
          el.onclick = () => { overlay.classList.remove('show'); setTimeout(() => overlay.remove(), 150); resolve(btn.value); };
          btnContainer.appendChild(el);
        });
        document.body.appendChild(overlay);
        requestAnimationFrame(() => overlay.classList.add('show'));
        overlay.addEventListener('click', e => { if (e.target === overlay) { overlay.classList.remove('show'); setTimeout(() => overlay.remove(), 150); resolve(null); } });
      });
    }
  };
})();

// Poly IPC Bridge
window.poly = {
  async invoke(fn, args = {}) {
    const r = await fetch('/__poly_invoke', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ fn, args })
    });
    const d = await r.json();
    if (d.error) throw new Error(d.error);
    return Object.prototype.hasOwnProperty.call(d, 'result') ? d.result : d;
  },
  dialog: {
    async open(options = {}) { return poly.invoke('__poly_dialog_open', options); },
    async openMultiple(options = {}) { return poly.invoke('__poly_dialog_open_multiple', options); },
    async save(options = {}) { return poly.invoke('__poly_dialog_save', options); },
    async folder(options = {}) { return poly.invoke('__poly_dialog_folder', options); },
    async message(title, message, level = 'info') {
      return __polyDialog.show(level, title, message, [{ text: 'OK', value: true, primary: true }]);
    },
    async confirm(title, message) {
      return __polyDialog.show('confirm', title, message, [
        { text: 'Cancel', value: false },
        { text: 'Confirm', value: true, primary: true }
      ]);
    },
    async custom(options) {
      return __polyDialog.show(options.type || 'info', options.title, options.message, options.buttons || [{ text: 'OK', value: true, primary: true }]);
    }
  },
  fs: {
    async read(path) { return poly.invoke('__poly_fs_read', { path }); },
    async write(path, content) { return poly.invoke('__poly_fs_write', { path, content }); },
    async exists(path) { return poly.invoke('__poly_fs_exists', { path }); },
    async readDir(path) { return poly.invoke('__poly_fs_read_dir', { path }); }
  },
  __POLY_NCOS_API__
  updater: {
    async checkGithub(repo, currentVersion) {
      return poly.invoke('__poly_updater_check_github', { repo, currentVersion });
    },
    async checkUrl(url, currentVersion) {
      return poly.invoke('__poly_updater_check_url', { url, currentVersion });
    },
    async download(url) {
      return poly.invoke('__poly_updater_download', { url });
    },
    async install(path) {
      return poly.invoke('__poly_updater_install', { path });
    },
    // Convenience method: check, prompt, download, install
    async checkAndPrompt(options = {}) {
      const { repo, url, currentVersion = '0.0.0' } = options;

      let info;
      if (repo) {
        info = await this.checkGithub(repo, currentVersion);
      } else if (url) {
        info = await this.checkUrl(url, currentVersion);
      } else {
        throw new Error('Must provide repo or url');
      }

      if (!info.update_available) {
        return { updated: false, info };
      }

      // Show update dialog
      const shouldUpdate = await poly.dialog.custom({
        type: 'info',
        title: 'Update Available',
        message: `Version ${info.latest_version} is available (current: ${info.current_version}).\n\n${info.release_notes || 'Would you like to update now?'}`,
        buttons: [
          { text: 'Later', value: false },
          { text: 'Update Now', value: true, primary: true }
        ]
      });

      if (!shouldUpdate) {
        return { updated: false, info };
      }

      if (!info.download_url) {
        throw new Error('No download URL available for this platform');
      }

      // Download
      await poly.dialog.message('Downloading...', 'Please wait while the update is downloaded.', 'info');
      const downloadPath = await this.download(info.download_url);

      // Install
      await this.install(downloadPath);

      return { updated: true, info };
    }
  },
  window: {
    minimize() { if (window.ipc) window.ipc.postMessage('minimize'); },
    maximize() { if (window.ipc) window.ipc.postMessage('maximize'); },
    close() { if (window.ipc) window.ipc.postMessage('close'); },
    hide() { if (window.ipc) window.ipc.postMessage('hide'); },
    show() { if (window.ipc) window.ipc.postMessage('show'); },
    async setTitle(title) { return poly.invoke('__poly_window_set_title', { title }); },
    async getTitle() { return poly.invoke('__poly_window_get_title', {}); },
    async center() { return poly.invoke('__poly_window_center', {}); },
    async setSize(width, height) { return poly.invoke('__poly_window_set_size', { width, height }); },
    async getSize() { return poly.invoke('__poly_window_get_size', {}); },
    async setPosition(x, y) { return poly.invoke('__poly_window_set_position', { x, y }); },
    async getPosition() { return poly.invoke('__poly_window_get_position', {}); },
    async setMinSize(width, height) { return poly.invoke('__poly_window_set_min_size', { width, height }); },
    async setMaxSize(width, height) { return poly.invoke('__poly_window_set_max_size', { width, height }); },
    async setAlwaysOnTop(value) { return poly.invoke('__poly_window_set_always_on_top', { value }); },
    async setFullscreen(value) { return poly.invoke('__poly_window_set_fullscreen', { value }); },
    async isFullscreen() { return poly.invoke('__poly_window_is_fullscreen', {}); },
    async isMaximized() { return poly.invoke('__poly_window_is_maximized', {}); },
    async isMinimized() { return poly.invoke('__poly_window_is_minimized', {}); }
  },
  clipboard: {
    async read() { return poly.invoke('__poly_clipboard_read', {}); },
    async write(text) { return poly.invoke('__poly_clipboard_write', { text }); },
    async clear() { return poly.invoke('__poly_clipboard_clear', {}); }
  },
  windows: {
    async create(options = {}) { return poly.invoke('__poly_window_create', options); },
    async close(id) { return poly.invoke('__poly_window_close', { id }); },
    async closeAll() { return poly.invoke('__poly_window_close_all', {}); },
    async list() { return poly.invoke('__poly_window_list', {}); },
    async count() { return poly.invoke('__poly_window_count', {}); },
    async minimize(id) { return poly.invoke('__poly_window_minimize', { id }); },
    async maximize(id) { return poly.invoke('__poly_window_maximize', { id }); },
    async restore(id) { return poly.invoke('__poly_window_restore', { id }); },
    async show(id) { return poly.invoke('__poly_window_show', { id }); },
    async hide(id) { return poly.invoke('__poly_window_hide', { id }); },
    async focus(id) { return poly.invoke('__poly_window_focus', { id }); },
    async setTitle(id, title) { return poly.invoke('__poly_window_set_title', { id, title }); },
    async setSize(id, width, height) { return poly.invoke('__poly_window_set_size', { id, width, height }); },
    async setPosition(id, x, y) { return poly.invoke('__poly_window_set_position', { id, x, y }); },
    async setAlwaysOnTop(id, value) { return poly.invoke('__poly_window_set_always_on_top', { id, value }); },
    async setFullscreen(id, value) { return poly.invoke('__poly_window_set_fullscreen', { id, value }); },
    async navigate(id, url) { return poly.invoke('__poly_window_navigate', { id, url }); },
    async loadHtml(id, html) { return poly.invoke('__poly_window_load_html', { id, html }); },
    async eval(id, script) { return poly.invoke('__poly_window_eval', { id, script }); },
    async getState(id) { return poly.invoke('__poly_window_get_state', { id }); },
    async listStates() { return poly.invoke('__poly_window_list_states', {}); }
  },
  notification: {
    async show(title, body, icon) { return poly.invoke('__poly_notification_show', { title, body, icon }); },
    async showWithTimeout(title, body, timeout) { return poly.invoke('__poly_notification_show_timeout', { title, body, timeout }); }
  },
  deeplink: {
    async register(protocol, appName) { return poly.invoke('__poly_deeplink_register', { protocol, appName }); },
    async unregister(protocol) { return poly.invoke('__poly_deeplink_unregister', { protocol }); },
    async isRegistered(protocol) { return poly.invoke('__poly_deeplink_is_registered', { protocol }); },
    async get() { return poly.invoke('__poly_deeplink_get', {}); },
    async has() { return poly.invoke('__poly_deeplink_has', {}); }
  },
  tray: {
    // Listen for tray menu clicks
    onMenuClick(callback) {
      window.addEventListener('polytray', (e) => callback(e.detail.id));
    },
    // Check if tray is enabled (from poly.toml config)
    async isEnabled() { return poly.invoke('__poly_tray_is_enabled', {}); }
  },
  shell: {
    async open(url) { return poly.invoke('__poly_shell_open', { url }); },
    async openPath(path) { return poly.invoke('__poly_shell_open_path', { path }); },
    async openWith(path, app) { return poly.invoke('__poly_shell_open_with', { path, app }); }
  },
  app: {
    async getVersion() { return poly.invoke('__poly_app_get_version', {}); },
    async getName() { return poly.invoke('__poly_app_get_name', {}); },
    async getPath(name) { return poly.invoke('__poly_app_get_path', { name }); },
    async exit(code = 0) { return poly.invoke('__poly_app_exit', { code }); },
    async relaunch() { return poly.invoke('__poly_app_relaunch', {}); }
  },
  os: {
    async platform() { return poly.invoke('__poly_os_platform', {}); },
    async arch() { return poly.invoke('__poly_os_arch', {}); },
    async version() { return poly.invoke('__poly_os_version', {}); },
    async hostname() { return poly.invoke('__poly_os_hostname', {}); },
    async homedir() { return poly.invoke('__poly_os_homedir', {}); },
    async tempdir() { return poly.invoke('__poly_os_tempdir', {}); }
  },
  // Network API - HTTP requests
  http: {
    async get(url, options = {}) { return poly.invoke('__poly_http_get', { url, ...options }); },
    async post(url, body, options = {}) { return poly.invoke('__poly_http_post', { url, body, ...options }); },
    async put(url, body, options = {}) { return poly.invoke('__poly_http_put', { url, body, ...options }); },
    async patch(url, body, options = {}) { return poly.invoke('__poly_http_patch', { url, body, ...options }); },
    async delete(url, options = {}) { return poly.invoke('__poly_http_delete', { url, ...options }); },
    async request(options) { return poly.invoke('__poly_http_request', options); }
  },
  // SQLite Database API
  db: {
    async open(path) { return poly.invoke('__poly_db_open', { path }); },
    async close(id) { return poly.invoke('__poly_db_close', { id }); },
    async execute(id, sql, params = []) { return poly.invoke('__poly_db_execute', { id, sql, params }); },
    async query(id, sql, params = []) { return poly.invoke('__poly_db_query', { id, sql, params }); },
    async queryOne(id, sql, params = []) { return poly.invoke('__poly_db_query_one', { id, sql, params }); }
  },
  // Managed CEF off-screen browser surfaces
  cef: {
    async available() { return poly.invoke('__poly_cef_available', {}); },
    async create(id, url, width = 800, height = 600, bounds = {}) { return poly.invoke('__poly_cef_create', { id, url, width, height, ...bounds }); },
    async navigate(id, url) { return poly.invoke('__poly_cef_navigate', { id, url }); },
    async goBack(id) { return poly.invoke('__poly_cef_history', { id, direction: 'back' }); },
    async goForward(id) { return poly.invoke('__poly_cef_history', { id, direction: 'forward' }); },
    async reload(id) { return poly.invoke('__poly_cef_reload', { id }); },
    async resize(id, width, height) { return poly.invoke('__poly_cef_resize', { id, width, height }); },
    async setBounds(id, bounds) { return poly.invoke('__poly_cef_bounds', { id, ...bounds }); },
    async setVisible(id, visible) { return poly.invoke('__poly_cef_visible', { id, visible }); },
    async mouseMove(id, x, y, leave = false) { return poly.invoke('__poly_cef_mouse_move', { id, x, y, leave }); },
    async mouseButton(id, x, y, down) { return poly.invoke('__poly_cef_mouse_button', { id, x, y, down }); },
    async mouseWheel(id, x, y, deltaX, deltaY) { return poly.invoke('__poly_cef_mouse_wheel', { id, x, y, deltaX, deltaY }); },
    async key(id, type, keyCode, character = 0, modifiers = 0) { return poly.invoke('__poly_cef_key', { id, type, keyCode, character, modifiers }); },
    async destroy(id) { return poly.invoke('__poly_cef_destroy', { id }); },
    async favicon(id) { return poly.invoke('__poly_cef_favicon', { id }); },
    async metadata(id) { return poly.invoke('__poly_cef_metadata', { id }); },
    async readFrame(id, since = 0) {
      const response = await fetch('/__poly_cef_frame?id=' + encodeURIComponent(id) + '&since=' + Number(since || 0) + '&wait_ms=100', { cache: 'no-store' });
      if (response.status === 304) return null;
      if (!response.ok) throw new Error('CEF frame is not ready.');
      return { width: Number(response.headers.get('X-Poly-Cef-Width')), height: Number(response.headers.get('X-Poly-Cef-Height')), sequence: Number(response.headers.get('X-Poly-Cef-Sequence')), dirtyX: Number(response.headers.get('X-Poly-Cef-Dirty-X')), dirtyY: Number(response.headers.get('X-Poly-Cef-Dirty-Y')), dirtyWidth: Number(response.headers.get('X-Poly-Cef-Dirty-Width')), dirtyHeight: Number(response.headers.get('X-Poly-Cef-Dirty-Height')), rgba: await response.arrayBuffer() };
    }
  },
  // Browser API - Build browsers with Poly
  browser: {
    async createTab(url) { return poly.invoke('__poly_browser_create_tab', { url }); },
    async closeTab(id) { return poly.invoke('__poly_browser_close_tab', { id }); },
    async getTab(id) { return poly.invoke('__poly_browser_get_tab', { id }); },
    async listTabs() { return poly.invoke('__poly_browser_list_tabs', {}); },
    async navigate(id, url) { return poly.invoke('__poly_browser_navigate', { id, url }); },
    async back(id) { return poly.invoke('__poly_browser_back', { id }); },
    async forward(id) { return poly.invoke('__poly_browser_forward', { id }); },
    async setTitle(id, title) { return poly.invoke('__poly_browser_set_title', { id, title }); },
    async setLoading(id, loading) { return poly.invoke('__poly_browser_set_loading', { id, loading }); },
    async getHistory(id) { return poly.invoke('__poly_browser_get_history', { id }); },
    async clearHistory(id) { return poly.invoke('__poly_browser_clear_history', { id }); },
    async fetch(url) { return poly.invoke('__poly_browser_fetch', { url }); },
    // Proxy URL - use this to load external resources through the local server
    proxyUrl(url) { return '/__poly_proxy?url=' + encodeURIComponent(url); },
    // Navigate the current WebView to a URL (replaces current page)
    loadUrl(url) { window.location.href = url; },
    // Go back in browser history
    goBack() { window.history.back(); },
    // Go forward in browser history
    goForward() { window.history.forward(); },
    // Open a real WebView window (full browser functionality)
    async openWindow(url, options = {}) { return poly.invoke('__poly_browser_open_window', { url, ...options }); },
    async windowNavigate(id, url) { return poly.invoke('__poly_browser_window_navigate', { id, url }); },
    async windowClose(id) { return poly.invoke('__poly_browser_window_close', { id }); }
  },
  // Titlebar API - Custom persistent titlebar for browser apps
  titlebar: {
    // Set custom titlebar (persists across navigation)
    async set(config) { return poly.invoke('__poly_titlebar_set', config); },
    // Get current titlebar config
    async get() { return poly.invoke('__poly_titlebar_get', {}); },
    // Enable/disable titlebar
    async setEnabled(enabled) { return poly.invoke('__poly_titlebar_set_enabled', { enabled }); },
    // Set titlebar height
    async setHeight(height) { return poly.invoke('__poly_titlebar_set_height', { height }); },
    // Update titlebar HTML
    async setHtml(html) { return poly.invoke('__poly_titlebar_set_html', { html }); },
    // Update titlebar CSS
    async setCss(css) { return poly.invoke('__poly_titlebar_set_css', { css }); },
    // Update titlebar JavaScript
    async setJs(js) { return poly.invoke('__poly_titlebar_set_js', { js }); },
    // Navigate the main content area to a URL (keeps titlebar)
    async navigate(url) { return poly.invoke('__poly_titlebar_navigate', { url }); }
  },
  // WebView API - Multi-WebView management for browser apps
  webview: {
    // Create a new WebView
    // Options: url, html, x, y, width, height, visible, transparent, devtools, initScripts (array of scripts to run before page loads)
    async create(id, options = {}) { return poly.invoke('__poly_webview_create', { id, ...options }); },
    // Navigate a WebView to URL
    async navigate(id, url) { return poly.invoke('__poly_webview_navigate', { id, url }); },
    // Load HTML content directly
    async loadHtml(id, html) { return poly.invoke('__poly_webview_load_html', { id, html }); },
    // Go back in history
    async goBack(id) { return poly.invoke('__poly_webview_go_back', { id }); },
    // Go forward in history
    async goForward(id) { return poly.invoke('__poly_webview_go_forward', { id }); },
    // Reload the page
    async reload(id) { return poly.invoke('__poly_webview_reload', { id }); },
    // Stop loading
    async stop(id) { return poly.invoke('__poly_webview_stop', { id }); },
    // Set WebView bounds (position and size)
    async setBounds(id, bounds) { return poly.invoke('__poly_webview_set_bounds', { id, ...bounds }); },
    // Get WebView bounds
    async getBounds(id) { return poly.invoke('__poly_webview_get_bounds', { id }); },
    // Execute JavaScript in a WebView
    async eval(id, script) { return poly.invoke('__poly_webview_eval', { id, script }); },
    // Destroy a WebView
    async destroy(id) { return poly.invoke('__poly_webview_destroy', { id }); },
    // List all WebViews
    async list() { return poly.invoke('__poly_webview_list', {}); },
    // Get WebView info (includes isLoading, canGoBack, canGoForward)
    async get(id) { return poly.invoke('__poly_webview_get', { id }); },
    // Show/hide a WebView
    async setVisible(id, visible) { return poly.invoke('__poly_webview_set_visible', { id, visible }); },
    // Focus a WebView
    async focus(id) { return poly.invoke('__poly_webview_focus', { id }); },
    // Set zoom level (1.0 = 100%)
    async setZoom(id, level) { return poly.invoke('__poly_webview_set_zoom', { id, level }); },
    // Set main WebView bounds (the app's original WebView)
    async setMainBounds(bounds) { return poly.invoke('__poly_webview_set_main_bounds', bounds); },
    // Poll for events (navigation, title change, etc.)
    async pollEvents() { return poly.invoke('__poly_webview_poll_events', {}); },
    // Grant or deny a permission request
    async respondToPermission(id, permission, granted) { return poly.invoke('__poly_webview_respond_permission', { id, permission, granted }); },
    // Event listeners (client-side convenience)
    _listeners: {},
    on(event, id, callback) {
      const key = `${event}:${id}`;
      if (!this._listeners[key]) this._listeners[key] = [];
      this._listeners[key].push(callback);
    },
    off(event, id, callback) {
      const key = `${event}:${id}`;
      if (this._listeners[key]) {
        this._listeners[key] = this._listeners[key].filter(cb => cb !== callback);
      }
    },
    _emit(event, id, data) {
      const key = `${event}:${id}`;
      if (this._listeners[key]) {
        this._listeners[key].forEach(cb => cb(data));
      }
      // Also emit to wildcard listeners
      const wildcardKey = `${event}:*`;
      if (this._listeners[wildcardKey]) {
        this._listeners[wildcardKey].forEach(cb => cb(id, data));
      }
    },
    // Convenience event registration
    onNavigate(id, cb) { this.on('navigate', id, cb); },
    onTitleChange(id, cb) { this.on('titleChange', id, cb); },
    onLoadStart(id, cb) { this.on('loadStart', id, cb); },
    onLoadFinish(id, cb) { this.on('loadFinish', id, cb); },
    onNewWindow(id, cb) { this.on('newWindow', id, cb); },
    onDownload(id, cb) { this.on('download', id, cb); },
    onClose(id, cb) { this.on('close', id, cb); },
    onHistoryChange(id, cb) { this.on('historyChange', id, cb); }
  },
  // Ad Blocking API
  adblock: {
    // Initialize ad blocker with filter lists
    async init(filterLists) { return poly.invoke('__poly_adblock_init', { filterLists }); },
    // Check if a URL should be blocked
    async shouldBlock(url, sourceUrl, resourceType = 'other') {
      return poly.invoke('__poly_adblock_should_block', { url, sourceUrl, resourceType });
    },
    // Download EasyList filter list
    async downloadEasyList() { return poly.invoke('__poly_adblock_download_easylist', {}); },
    // Download EasyPrivacy filter list
    async downloadEasyPrivacy() { return poly.invoke('__poly_adblock_download_easyprivacy', {}); },
    // Download Pi-hole block list
    async downloadPiHole() { return poly.invoke('__poly_adblock_download_pihole', {}); },
    // Parse Pi-hole hosts file
    async parsePiHole(content) { return poly.invoke('__poly_adblock_parse_pihole', { content }); },
    // Get blocking statistics
    async getStats() { return poly.invoke('__poly_adblock_get_stats', {}); }
  },
  // MultiView API - Create windows with multiple WebViews
  multiview: {
    // Create a new multi-view window
    // views: array of { id, url, x, y, width, height }
    // Views are stacked: first in array = bottom, last = top (for UI)
    async create(options) { return poly.invoke('__poly_multiview_create', options); },
    // Navigate a view to URL
    async navigate(windowId, viewId, url) { return poly.invoke('__poly_multiview_navigate', { windowId, viewId, url }); },
    // Send message to a view (triggers 'polymessage' event)
    async postMessage(windowId, viewId, message) { return poly.invoke('__poly_multiview_post_message', { windowId, viewId, message: JSON.stringify(message) }); },
    // Set view bounds
    async setBounds(windowId, viewId, bounds) { return poly.invoke('__poly_multiview_set_bounds', { windowId, viewId, ...bounds }); },
    // Close a multi-view window
    async close(windowId) { return poly.invoke('__poly_multiview_close', { windowId }); },
    // List all multi-view windows
    async list() { return poly.invoke('__poly_multiview_list', {}); },
    // Get window info
    async get(windowId) { return poly.invoke('__poly_multiview_get', { windowId }); }
  },
  // AI/LLM API - Chat with AI models
  ai: {
    async ollama(model, messages, options = {}) {
      return poly.invoke('__poly_ai_ollama', { model, messages, ...options });
    },
    async openai(model, messages, apiKey, options = {}) {
      return poly.invoke('__poly_ai_openai', { model, messages, apiKey, ...options });
    },
    async anthropic(model, messages, apiKey, options = {}) {
      return poly.invoke('__poly_ai_anthropic', { model, messages, apiKey, ...options });
    },
    async custom(baseUrl, model, messages, options = {}) {
      return poly.invoke('__poly_ai_custom', { baseUrl, model, messages, ...options });
    },
    async checkOllama() {
      return poly.invoke('__poly_ai_check_ollama', {});
    },
    async listModels() {
      return poly.invoke('__poly_ai_list_models', {});
    },
    async chat(options) {
      return poly.invoke('__poly_ai_chat', options);
    },
    // Streaming API
    stream: {
      async start(model, messages, options = {}) {
        return poly.invoke('__poly_ai_stream_start', { model, messages, ...options });
      },
      async poll(streamId) {
        return poly.invoke('__poly_ai_stream_poll', { streamId });
      },
      async cancel(streamId) {
        return poly.invoke('__poly_ai_stream_cancel', { streamId });
      },
      async list() {
        return poly.invoke('__poly_ai_stream_list', {});
      },
      async run(model, messages, options = {}, onChunk) {
        const result = await this.start(model, messages, options);
        if (result.error) throw new Error(result.error);
        const streamId = result.streamId;

        const poll = async () => {
          const { chunks, done } = await this.poll(streamId);
          for (const chunk of chunks) {
            if (onChunk) onChunk(chunk);
          }
          if (!done) {
            await new Promise(r => setTimeout(r, 16));
            await poll();
          }
        };

        await poll();
        return streamId;
      }
    }
  }
};
// Initialize Lucide Icons
if (typeof lucide !== 'undefined') {
  lucide.createIcons();
  document.addEventListener('alpine:initialized', () => lucide.createIcons());
}
// Hot Reload for Native Mode (disabled in production builds)
// Note: Hot reload is only active during development
</script>"##;
                        let body_script = body_script.replace("__POLY_NCOS_API__", NCOS_VIRTUAL_DRIVE_API);
                        if html.contains("</body>") {
                            html = html.replace("</body>", &format!("{}</body>", body_script));
                        }

                        html.into_bytes()
                    } else {
                        content
                    };

                    tiny_http::Response::from_data(final_content)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], ct.as_bytes()).unwrap())
                } else {
                    tiny_http::Response::from_string("Not Found").with_status_code(404)
                }
            };
            let _ = request.respond(response);
        }
    });

    // Clone entry_path for watcher thread
    let entry_path_watcher = entry_path.clone();

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

        // Also watch src directory for .poly files
        let src_path = project_path_for_watcher.join("src");

        if let Err(e) = watcher.watch(&watch_path, RecursiveMode::Recursive) {
            eprintln!("  {}warning{}: Could not watch directory: {}", YELLOW, RESET, e);
        }

        if src_path.exists() {
            if let Err(e) = watcher.watch(&src_path, RecursiveMode::Recursive) {
                eprintln!("  {}warning{}: Could not watch src directory: {}", YELLOW, RESET, e);
            }
        }

        loop {
            if let Ok(Ok(event)) = rx.recv() {
                    // Check for .poly file changes
                    let has_poly_change = event.paths.iter().any(|p| {
                        p.extension().and_then(|e| e.to_str()) == Some("poly")
                    });

                    // Reload interpreter for .poly changes
                    if has_poly_change {
                        if let Some(ref entry) = entry_path_watcher {
                            let source = fs::read_to_string(entry).unwrap_or_default();
                            let mut interp = interpreter_watcher.lock().unwrap();
                            *interp = poly::create_interpreter();
                            if let Err(e) = poly::init_interpreter(&mut interp, &source) {
                                eprintln!("  {}error{}: Reload failed: {}", RED, RESET, e);
                            }
                        }
                    }

                    // Reload for relevant file changes (HTML/CSS/JS/JSON/SVG/Poly)
                    let dominated_by_relevant = event.paths.iter().any(|p| {
                        let ext = p.extension().and_then(|e| e.to_str());
                        matches!(ext, Some("html") | Some("css") | Some("js") | Some("json") | Some("svg") | Some("poly"))
                    });

                    if dominated_by_relevant {
                        reload_counter_watcher.fetch_add(1, Ordering::Relaxed);
                    }
            }
        }
    });

    // Give server time to start
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Standard mode: Single WebView - App handles everything via APIs
    let url = format!("http://localhost:{}", port);

    #[cfg(all(feature = "native", feature = "experimental-virtual-drive"))]
    poly::native::register_shutdown_hook(native_app::shutdown_all);
    if let Err(e) = poly::run_native_url(&url, config) {
        eprintln!("{}error{}: {}", RED, RESET, e);
        eprintln!();
        eprintln!("  {}hint{}: Build with native feature enabled:", DIM, RESET);
        eprintln!("    cargo build -p poly --features native");
        std::process::exit(1);
    }
}

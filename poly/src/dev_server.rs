use super::*;

pub(super) fn run_dev_server(path: &str, port: u16, open_browser: bool) {
    use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
    use std::thread;

    // Disable Permissions in development mode
    poly::permissions::set_development_mode();

    let project_path = Path::new(path);
    let project_path_owned = project_path.to_path_buf();

    // Load config from poly.toml
    let config = poly::PolyConfig::load(project_path);

    // Get values from config
    let inject_alpine = config.dev.inject_alpine;
    let inject_lucide = config.dev.inject_lucide;
    let reload_interval = config.dev.reload_interval;
    let web_dir_relative = config.web.dir.clone();
    let vite_project = project_path.join("package.json").is_file()
        && crate::scaffold::has_vite_config(project_path);

    // Use config port if set, otherwise CLI arg (CLI arg takes precedence if not default)
    let port = if port != 3000 { port } else { config.dev.port };

    let entry = find_frontend_entry(project_path);
    if entry.is_none() && vite_project {
        println!();
        println!("  {}POLY{} v{}  {}frontend dev server{}", CYAN, RESET, VERSION, DIM, RESET);
        println!("  {}>{} Starting the Vite frontend...{}", DIM, RESET, RESET);
        let mut command = crate::scaffold::npm_command();
        command.args(["run", "dev"]);
        if open_browser { command.args(["--", "--open"]); }
        let status = command.current_dir(project_path).status()
            .map_err(|error| format!("Could not start the Vite dev server: {error}"));
        match status {
            Ok(status) if status.success() => return,
            Ok(status) => {
                eprintln!("{}error{}: Vite dev server exited with {status}.", RED, RESET);
                std::process::exit(status.code().unwrap_or(1));
            }
            Err(error) => {
                eprintln!("{}error{}: {}", RED, RESET, error);
                std::process::exit(1);
            }
        }
    }
    if entry.is_none() {
        eprintln!("{}error{}: No entry point found. Create web/index.html", RED, RESET);
        std::process::exit(1);
    }
    let entry = entry.unwrap();

    println!();
    println!("  {}POLY{} v{}  {}dev server{}", CYAN, RESET, VERSION, DIM, RESET);
    println!();
    println!("  {}>{} Local:   {}http://localhost:{}{}", GREEN, RESET, CYAN, port, RESET);
    println!("  {}>{} Entry:   {}{}{}", DIM, RESET, DIM, entry.display(), RESET);
    println!();

    let reload_counter = Arc::new(AtomicU64::new(0));
    let reload_counter_http = Arc::clone(&reload_counter);
    let project_path_http = project_path_owned.clone();
    let web_dir_http = web_dir_relative.clone();
    let entry_http = entry.clone();
    // HTTP server thread
    thread::spawn(move || {
        let server = tiny_http::Server::http(format!("0.0.0.0:{}", port))
            .expect("Failed to start HTTP server");

        for mut request in server.incoming_requests() {
            let url = request.url().to_string();

            if url == "/__poly_invoke_stream" {
                let mut body = String::new();
                request.as_reader().read_to_string(&mut body).ok();
                let web_root = project_path_http.join(&web_dir_http);
                let response = match handle_ipc_plugin_stream(&body, &web_root) {
                    Ok(reader) => tiny_http::Response::new(
                        tiny_http::StatusCode(200),
                        vec![
                            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/x-ndjson; charset=utf-8"[..]).unwrap(),
                            tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
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

            let response = match url.as_str() {
                "/" | "/index.html" => {
                    // Check for custom index.html in web folder
                    let custom_index = project_path_http.join(&web_dir_http).join("index.html");
                    if custom_index.exists() {
                        let mut html = fs::read_to_string(&custom_index).unwrap_or_default();

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

                        // Keep hot reload in the development server. Poly's JavaScript API is now an
                        // imported frontend package instead of a runtime-injected global.
                        let reload_script = format!(r#"<script>
(function() {{
  let version = {initial_version};
  let polling = true;
  async function check() {{
    if (!document.hidden) {{
      try {{
        const response = await fetch('/__poly_reload');
        const data = await response.json();
        if (data.version > version) {{ version = data.version; location.reload(); }}
      }} catch {{}}
    }}
    if (polling) setTimeout(check, {reload_interval});
  }}
  document.addEventListener('visibilitychange', () => {{ polling = !document.hidden; if (polling) check(); }});
  check();
}})();
</script>"#, initial_version = reload_counter_http.load(Ordering::Relaxed), reload_interval = reload_interval);
                        if html.contains("</body>") {
                            html = html.replace("</body>", &format!("{}</body>", reload_script));
                        } else {
                            html.push_str(&reload_script);
                        }
                        tiny_http::Response::from_string(html)
                            .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap())
                    } else {
                        let html = generate_dev_html(&project_path_http, &entry_http, reload_counter_http.load(Ordering::Relaxed));
                        tiny_http::Response::from_string(html)
                            .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap())
                    }
                }
                "/__poly_reload" => {
                    let current = reload_counter_http.load(Ordering::Relaxed);
                    tiny_http::Response::from_string(format!(r#"{{"version":{}}}"#, current))
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                }
                "/__poly_invoke" => {
                    // IPC bridge for JavaScript framework APIs
                    let mut body = String::new();
                    request.as_reader().read_to_string(&mut body).ok();

                    let web_root = project_path_http.join(&web_dir_http);
                    let result = handle_ipc_invoke_system_only(&body, Some(&web_root));
                    tiny_http::Response::from_string(result)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                        .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
                }
                _ if url.starts_with("/__polyview") => {
                    // PolyView Proxy - "iframe2" that bypasses all iframe restrictions
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
                _ => {
                    // First try the exact path, then try in web/ folder, then packages/
                    let url_path = url.trim_start_matches('/');
                    let file_path = project_path_http.join(url_path);
                    let web_file_path = project_path_http.join(&web_dir_http).join(url_path);
                    let packages_file_path = project_path_http.join(url_path);

                    let actual_path = if file_path.exists() && file_path.is_file() {
                        Some(file_path)
                    } else if web_file_path.exists() && web_file_path.is_file() {
                        Some(web_file_path)
                    } else if packages_file_path.exists() && packages_file_path.is_file() {
                        Some(packages_file_path)
                    } else {
                        None
                    };

                    if let Some(path) = actual_path {
                        // Use read for binary files, read_to_string for text
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        let is_binary = matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "ico" | "woff" | "woff2" | "ttf" | "eot");

                        if is_binary {
                            match fs::read(&path) {
                                Ok(content) => {
                                    let ct = match ext {
                                        "png" => "image/png",
                                        "jpg" | "jpeg" => "image/jpeg",
                                        "gif" => "image/gif",
                                        "ico" => "image/x-icon",
                                        "woff" => "font/woff",
                                        "woff2" => "font/woff2",
                                        "ttf" => "font/ttf",
                                        "eot" => "application/vnd.ms-fontobject",
                                        _ => "application/octet-stream",
                                    };
                                    tiny_http::Response::from_data(content)
                                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], ct.as_bytes()).unwrap())
                                }
                                Err(_) => tiny_http::Response::from_string("Read Error").with_status_code(500)
                            }
                        } else {
                            let content = fs::read_to_string(&path).unwrap_or_default();
                            let ct = match ext {
                                "html" => "text/html; charset=utf-8",
                                "css" => "text/css; charset=utf-8",
                                "js" => "application/javascript; charset=utf-8",
                                "json" => "application/json",
                                "svg" => "image/svg+xml",
                                _ => "text/plain",
                            };
                            tiny_http::Response::from_string(content)
                                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], ct.as_bytes()).unwrap())
                        }
                    } else {
                        tiny_http::Response::from_string("Not Found").with_status_code(404)
                    }
                }
            };
            let _ = request.respond(response);
        }
    });

    let mut vite_process = if vite_project {
        let backend = format!("http://127.0.0.1:{port}");
        match npm_command()
            .args(["run", "dev"])
            .current_dir(project_path)
            .env("POLY_BACKEND_URL", &backend)
            .spawn()
        {
            Ok(child) => {
                let mut child = child;
                let mut ready = false;
                for _ in 0..80 {
                    if std::net::TcpStream::connect_timeout(
                        &"127.0.0.1:5173".parse().unwrap(),
                        Duration::from_millis(100),
                    ).is_ok() {
                        ready = true;
                        break;
                    }
                    if let Ok(Some(status)) = child.try_wait() {
                        eprintln!("  {}×{} Vite exited during startup with {}", RED, RESET, status);
                        std::process::exit(1);
                    }
                    std::thread::sleep(Duration::from_millis(125));
                }
                if !ready {
                    let _ = child.kill();
                    eprintln!("  {}×{} Vite did not start on 127.0.0.1:5173 within 10 seconds", RED, RESET);
                    std::process::exit(1);
                }
                println!("  {}>{} Frontend: {}http://127.0.0.1:5173{} (Vite + HMR)", GREEN, RESET, CYAN, RESET);
                Some(child)
            }
            Err(error) => {
                eprintln!("  {}×{} Could not start the Vite dev server: {}", RED, RESET, error);
                std::process::exit(1);
            }
        }
    } else { None };

    // File watcher
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(move |res| { let _ = tx.send(res); })
        .expect("Failed to create file watcher");
    watcher.watch(project_path, RecursiveMode::Recursive).expect("Failed to watch directory");

    // Initial build
    print!("  {}Building...{}", DIM, RESET);
    io::stdout().flush().unwrap();
    let start = std::time::Instant::now();
    let _success = entry.is_file();
    println!("\r  {}ready{} in {}{}ms{}                    ", GREEN, RESET, BOLD, start.elapsed().as_millis(), RESET);

    if open_browser {
        let url = if vite_project { "http://127.0.0.1:5173".to_string() }
            else { format!("http://localhost:{}", port) };
        let _ = open_in_browser(&url);
    }

    println!();
    println!("  {}press h to show help{}", DIM, RESET);
    println!();

    // Watch loop
    loop {
        if let Some(child) = vite_process.as_mut() {
            if let Ok(Some(status)) = child.try_wait() {
                eprintln!("\n  {}×{} Vite dev server stopped with {}", RED, RESET, status);
                break;
            }
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                if should_reload(&event) {
                    let changed: Vec<_> = event.paths.iter()
                        .filter_map(|p| p.file_name())
                        .filter_map(|n| n.to_str())
                        .collect();

                    let time = chrono_time();
                    print!("{}{}{}  {}hmr update{} {}", DIM, time, RESET, YELLOW, RESET, changed.join(", "));
                    io::stdout().flush().unwrap();

                    std::thread::sleep(Duration::from_millis(50));
                    reload_counter.fetch_add(1, Ordering::Relaxed);

                    println!();

                }
            }
            Ok(Err(e)) => eprintln!("{}error{}: {:?}", RED, RESET, e),
            Err(_) => {}
        }
    }
}

fn chrono_time() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let secs = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}

pub(super) fn handle_ipc_invoke_system_only(body: &str, web_root: Option<&Path>) -> String {
    let request: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return serde_json::json!({"error": format!("Invalid JSON: {}", e)}).to_string(),
    };

    let fn_name = match request.get("fn").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => return serde_json::json!({"error": "Missing 'fn' field"}).to_string(),
    };

    let args = request.get("args").cloned().unwrap_or(serde_json::json!({}));

    if fn_name == "__poly_plugin_invoke" {
        let Some(web_root) = web_root else {
            return serde_json::json!({"error": "Native Rust plugins are unavailable in this context."}).to_string();
        };
        let plugin_id = match args.get("pluginId").and_then(|value| value.as_str()) {
            Some(value) => value,
            None => return serde_json::json!({"error": "Missing pluginId."}).to_string(),
        };
        let method = match args.get("method").and_then(|value| value.as_str()) {
            Some(value) => value,
            None => return serde_json::json!({"error": "Missing native plugin method."}).to_string(),
        };
        let payload = args.get("args").cloned().unwrap_or(serde_json::json!({}));
        poly::plugin_runtime::invoke(web_root, plugin_id, method, &payload)
    } else if fn_name.starts_with("__poly_") {
        handle_system_api(fn_name, &args)
    } else {
        serde_json::json!({"error": "No Poly backend available"}).to_string()
    }
}

pub(super) fn handle_ipc_plugin_stream(
    body: &str,
    web_root: &Path,
) -> Result<Box<dyn std::io::Read + Send>, String> {
    let request: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| format!("Invalid JSON: {error}"))?;
    if request.get("fn").and_then(|value| value.as_str()) != Some("__poly_plugin_invoke_stream") {
        return Err("Unsupported native plugin stream request.".into());
    }
    let args = request.get("args").cloned().unwrap_or(serde_json::json!({}));
    let plugin_id = args.get("pluginId").and_then(|value| value.as_str()).ok_or("Missing pluginId.")?;
    let method = args.get("method").and_then(|value| value.as_str()).ok_or("Missing native plugin method.")?;
    let payload = args.get("args").cloned().unwrap_or(serde_json::json!({}));
    poly::plugin_runtime::invoke_stream(web_root, plugin_id, method, &payload)
}



fn generate_dev_html(project_path: &Path, entry: &Path, version: u64) -> String {
    let name = project_path.file_name().and_then(|n| n.to_str()).unwrap_or("Poly App");
    let entry_name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("index.html");

    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - Poly</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: system-ui, -apple-system, sans-serif; background: #0f0f0f; color: #fafafa; min-height: 100vh; }}
        .header {{ background: #1a1a1a; padding: 12px 20px; display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid #2a2a2a; }}
        .logo {{ font-weight: 600; color: #00d9ff; }}
        .status {{ display: flex; align-items: center; gap: 8px; font-size: 13px; color: #888; }}
        .dot {{ width: 8px; height: 8px; border-radius: 50%; background: #22c55e; }}
        .container {{ display: flex; height: calc(100vh - 49px); }}
        .sidebar {{ width: 220px; background: #141414; border-right: 1px solid #2a2a2a; padding: 16px; }}
        .sidebar-title {{ font-size: 11px; text-transform: uppercase; color: #666; margin-bottom: 12px; letter-spacing: 0.5px; }}
        .file {{ padding: 8px 12px; border-radius: 4px; font-size: 13px; color: #888; cursor: pointer; }}
        .file:hover {{ background: #1a1a1a; }}
        .file.active {{ background: #1f1f1f; color: #00d9ff; }}
        .main {{ flex: 1; display: flex; flex-direction: column; }}
        .output {{ flex: 1; padding: 20px; overflow: auto; font-family: 'SF Mono', Consolas, monospace; font-size: 13px; line-height: 1.7; }}
        .line {{ padding: 2px 0; }}
        .line.error {{ color: #ef4444; }}
        .toolbar {{ padding: 12px 20px; background: #141414; border-top: 1px solid #2a2a2a; display: flex; gap: 8px; }}
        .btn {{ background: #00d9ff; color: #000; border: none; padding: 8px 16px; border-radius: 4px; font-weight: 500; cursor: pointer; font-size: 13px; }}
        .btn:hover {{ background: #00c4e6; }}
        .btn-ghost {{ background: transparent; color: #888; border: 1px solid #333; }}
        .btn-ghost:hover {{ background: #1a1a1a; color: #fff; }}
        .toast {{ position: fixed; top: 60px; right: 20px; background: #1a1a1a; border: 1px solid #333; padding: 10px 16px; border-radius: 6px; font-size: 13px; opacity: 0; transition: opacity 0.2s; }}
        .toast.show {{ opacity: 1; }}
    </style>
</head>
<body>
    <div class="header">
        <div class="logo">POLY</div>
        <div class="status"><div class="dot"></div>Ready</div>
    </div>
    <div class="container">
        <div class="sidebar">
            <div class="sidebar-title">Files</div>
            <div class="file active">{}</div>
        </div>
        <div class="main">
            <div class="output" id="output"><div class="line" style="color:#666">Loading...</div></div>
            <div class="toolbar">
                <button class="btn" onclick="runCode()">Run</button>
                <button class="btn btn-ghost" onclick="clearOutput()">Clear</button>
            </div>
        </div>
    </div>
    <div class="toast" id="toast">Reloading...</div>
    <script>
        let ver = {}, polling = false;
        async function runCode() {{
            document.getElementById('output').innerHTML = '<div class="line" style="color:#666">Running...</div>';
            try {{
                const r = await fetch('/__poly_run');
                const d = await r.json();
                document.getElementById('output').innerHTML = d.success
                    ? d.output.map(l => `<div class="line">${{esc(l)}}</div>`).join('')
                    : `<div class="line error">Error: ${{esc(d.error)}}</div>`;
            }} catch(e) {{ document.getElementById('output').innerHTML = `<div class="line error">${{e}}</div>`; }}
        }}
        function clearOutput() {{ document.getElementById('output').innerHTML = ''; }}
        function esc(t) {{ const d = document.createElement('div'); d.textContent = t; return d.innerHTML; }}
        async function check() {{
            if (!document.hidden) {{
                try {{
                    const r = await fetch('/__poly_reload');
                    const d = await r.json();
                    if (d.version > ver) {{ ver = d.version; document.getElementById('toast').classList.add('show'); await runCode(); setTimeout(() => document.getElementById('toast').classList.remove('show'), 800); }}
                }} catch(e) {{}}
            }}
            if (polling) setTimeout(check, 2000);
        }}
        function start() {{ if (!polling) {{ polling = true; check(); }} }}
        function stop() {{ polling = false; }}
        document.addEventListener('visibilitychange', () => document.hidden ? stop() : start());
        runCode(); start();
    </script>
</body>
</html>"#, name, entry_name, version)
}

fn should_reload(event: &Event) -> bool {
    matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) &&
    event.paths.iter().any(|p| {
        // Skip target directory and Rust files
        let path_str = p.to_string_lossy();
        if path_str.contains("target") || path_str.contains(".git") || path_str.contains("node_modules") || path_str.contains("dist") {
            return false;
        }

        p.extension().and_then(|e| e.to_str())
            .map(|ext| matches!(ext, "html" | "css" | "js"))
            .unwrap_or(false)
    })
}

pub(super) fn find_frontend_entry(project_path: &Path) -> Option<std::path::PathBuf> {
    let source_index = project_path.join("web/index.html");
    if source_index.is_file() {
        return Some(source_index);
    }

    let config = poly::PolyConfig::load(project_path);
    let configured_index = project_path.join(config.web.dir).join("index.html");
    configured_index.is_file().then_some(configured_index)
}

fn open_in_browser(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Create a lock file to prevent opening multiple browser windows
    let lock_file = std::env::temp_dir().join(".poly_browser_lock");
    let lock_age = lock_file.metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .map(|d| d.as_secs())
        .unwrap_or(u64::MAX);

    // Only open browser if lock file doesn't exist or is older than 5 seconds
    // This prevents multiple windows when restarting the dev server quickly
    if lock_age > 5 {
        fs::write(&lock_file, "").ok();

        #[cfg(target_os = "windows")]
        std::process::Command::new("cmd").args(["/C", "start", url]).spawn()?;
        #[cfg(target_os = "macos")]
        std::process::Command::new("open").arg(url).spawn()?;
        #[cfg(target_os = "linux")]
        std::process::Command::new("xdg-open").arg(url).spawn()?;
    }
    Ok(())
}

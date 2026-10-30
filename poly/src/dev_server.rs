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
        && project_path.join("vite.config.js").is_file();

    // Use config port if set, otherwise CLI arg (CLI arg takes precedence if not default)
    let port = if port != 3000 { port } else { config.dev.port };

    let entry = find_entry_point(project_path);
    if entry.is_none() {
        eprintln!("{}error{}: No entry point found. Create main.poly or src/main.poly", RED, RESET);
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

    // Create persistent interpreter wrapped in Arc<Mutex>
    use std::sync::Mutex;
    let interpreter = Arc::new(Mutex::new(poly::create_interpreter()));
    let interpreter_http = Arc::clone(&interpreter);

    // Initialize interpreter with source
    {
        let source = fs::read_to_string(&entry).unwrap_or_default();
        let mut interp = interpreter.lock().unwrap();
        if let Err(e) = poly::init_interpreter(&mut interp, &source) {
            let diagnostic = render_poly_diagnostic(&entry, &source, &e);
            eprintln!("\n  {}×{} {}Runtime initialization failed{}", RED, RESET, BOLD, RESET);
            for line in diagnostic.lines() { eprintln!("    {}", line); }
            std::process::exit(1);
        }
    }

    // Channel for reloading interpreter
    let (_reload_tx, reload_rx) = std::sync::mpsc::channel::<String>();
    let interpreter_reload = Arc::clone(&interpreter);

    // Interpreter reload thread
    std::thread::spawn(move || {
        for source in reload_rx {
            let mut interp = interpreter_reload.lock().unwrap();
            // Reset interpreter and reinitialize
            *interp = poly::create_interpreter();
            if let Err(e) = poly::init_interpreter(&mut interp, &source) {
                eprintln!("{}error{}: Reload failed: {}", RED, RESET, e);
            }
        }
    });

    // HTTP server thread
    thread::spawn(move || {
        let server = tiny_http::Server::http(format!("0.0.0.0:{}", port))
            .expect("Failed to start HTTP server");

        for mut request in server.incoming_requests() {
            let url = request.url().to_string();

            // Handle streaming endpoint separately (different response type)
            if url.as_str() == "/__poly_invoke_stream" {
                let mut body = String::new();
                request.as_reader().read_to_string(&mut body).ok();

                let response = match handle_ipc_invoke_streaming(&interpreter_http, &body) {
                    Ok(reader) => {
                        tiny_http::Response::new(
                            tiny_http::StatusCode(200),
                            vec![
                                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/plain"[..]).unwrap(),
                                tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap()
                            ],
                            reader,
                            None,
                            None
                        )
                    },
                    Err(e) => {
                        let error_json = serde_json::json!({"error": e}).to_string();
                        let error_reader: Box<dyn std::io::Read + Send> = Box::new(std::io::Cursor::new(error_json.into_bytes()));
                        tiny_http::Response::new(
                            tiny_http::StatusCode(400),
                            vec![
                                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
                                tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap()
                            ],
                            error_reader,
                            None,
                            None
                        )
                    }
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

                        // Inject hot reload script, IPC bridge, and icon initialization
                        let reload_script = format!(r#"<script>
// Poly Dialog System - Custom In-App Dialogs
(function() {{
  const style = document.createElement('style');
  style.textContent = `
    .poly-dialog-overlay {{
      position: fixed; inset: 0; background: rgba(0,0,0,0.6); backdrop-filter: blur(4px);
      display: flex; align-items: center; justify-content: center; z-index: 99999;
      opacity: 0; transition: opacity 0.15s ease;
    }}
    .poly-dialog-overlay.show {{ opacity: 1; }}
    .poly-dialog {{
      background: #1a1a1f; border: 1px solid rgba(255,255,255,0.1); border-radius: 12px;
      padding: 1.5rem; min-width: 320px; max-width: 90vw; box-shadow: 0 25px 50px rgba(0,0,0,0.5);
      transform: scale(0.95) translateY(-10px); transition: transform 0.15s ease;
    }}
    .poly-dialog-overlay.show .poly-dialog {{ transform: scale(1) translateY(0); }}
    .poly-dialog-icon {{ width: 48px; height: 48px; border-radius: 50%; display: flex; align-items: center; justify-content: center; margin: 0 auto 1rem; }}
    .poly-dialog-icon.info {{ background: rgba(59,130,246,0.2); color: #3b82f6; }}
    .poly-dialog-icon.warning {{ background: rgba(245,158,11,0.2); color: #f59e0b; }}
    .poly-dialog-icon.error {{ background: rgba(239,68,68,0.2); color: #ef4444; }}
    .poly-dialog-icon.confirm {{ background: rgba(93,193,210,0.2); color: #5dc1d2; }}
    .poly-dialog-icon svg {{ width: 24px; height: 24px; }}
    .poly-dialog-title {{ font-size: 1.1rem; font-weight: 600; color: #fff; text-align: center; margin-bottom: 0.5rem; }}
    .poly-dialog-message {{ color: #888; text-align: center; font-size: 0.9rem; line-height: 1.5; margin-bottom: 1.5rem; }}
    .poly-dialog-buttons {{ display: flex; gap: 0.75rem; justify-content: center; }}
    .poly-dialog-btn {{
      padding: 0.6rem 1.25rem; border-radius: 8px; font-size: 0.85rem; font-weight: 500;
      cursor: pointer; border: none; transition: all 0.15s;
    }}
    .poly-dialog-btn-primary {{ background: linear-gradient(135deg, #5dc1d2, #1e80ad); color: #fff; }}
    .poly-dialog-btn-primary:hover {{ transform: translateY(-1px); box-shadow: 0 4px 12px rgba(93,193,210,0.3); }}
    .poly-dialog-btn-secondary {{ background: rgba(255,255,255,0.1); color: #888; }}
    .poly-dialog-btn-secondary:hover {{ background: rgba(255,255,255,0.15); color: #fff; }}
  `;
  document.head.appendChild(style);

  const icons = {{
    info: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/></svg>',
    warning: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0zM12 9v4M12 17h.01"/></svg>',
    error: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M15 9l-6 6M9 9l6 6"/></svg>',
    confirm: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 015.83 1c0 2-3 3-3 3M12 17h.01"/></svg>'
  }};

  window.__polyDialog = {{
    show(type, title, message, buttons) {{
      return new Promise(resolve => {{
        const overlay = document.createElement('div');
        overlay.className = 'poly-dialog-overlay';
        overlay.innerHTML = `
          <div class="poly-dialog">
            <div class="poly-dialog-icon ${{type}}">${{icons[type] || icons.info}}</div>
            <div class="poly-dialog-title">${{title}}</div>
            <div class="poly-dialog-message">${{message}}</div>
            <div class="poly-dialog-buttons"></div>
          </div>
        `;
        const btnContainer = overlay.querySelector('.poly-dialog-buttons');
        buttons.forEach((btn, i) => {{
          const el = document.createElement('button');
          el.className = `poly-dialog-btn ${{btn.primary ? 'poly-dialog-btn-primary' : 'poly-dialog-btn-secondary'}}`;
          el.textContent = btn.text;
          el.onclick = () => {{ overlay.classList.remove('show'); setTimeout(() => overlay.remove(), 150); resolve(btn.value); }};
          btnContainer.appendChild(el);
        }});
        document.body.appendChild(overlay);
        requestAnimationFrame(() => overlay.classList.add('show'));
        overlay.addEventListener('click', e => {{ if (e.target === overlay) {{ overlay.classList.remove('show'); setTimeout(() => overlay.remove(), 150); resolve(null); }} }});
      }});
    }}
  }};
}})();

// Poly IPC Bridge
window.poly = {{
  async call(fn, ...args) {{ return poly.invoke(fn, args); }},
  async invoke(fn, args = {{}}) {{
    const r = await fetch('/__poly_invoke', {{
      method: 'POST',
      headers: {{ 'Content-Type': 'application/json' }},
      body: JSON.stringify({{ fn, args }})
    }});
    const d = await r.json();
    if (d.error) throw new Error(d.error);
    // System APIs return their JSON value directly; interpreted Poly functions
    // use the {{ result }} envelope. Preserve both shapes for their JS callers.
    return Object.prototype.hasOwnProperty.call(d, 'result') ? d.result : d;
  }},
  async stream(fn, ...args) {{
      const onData = args.pop();
      if (typeof onData !== 'function') throw new Error("Last argument must be callback");

      const r = await fetch('/__poly_invoke_stream', {{
          method: 'POST',
          headers: {{ 'Content-Type': 'application/json' }},
          body: JSON.stringify({{ fn, args }})
      }});

      if (!r.ok) {{
        const e = await r.json();
        throw new Error(e.error || 'Stream failed');
      }}

      const reader = r.body.getReader();
      const decoder = new TextDecoder();

      while (true) {{
          const {{ done, value }} = await reader.read();
          if (done) break;
          onData(decoder.decode(value, {{ stream: true }}));
      }}
  }},
  dialog: {{
    async open(options = {{}}) {{ return poly.invoke('__poly_dialog_open', options); }},
    async openMultiple(options = {{}}) {{ return poly.invoke('__poly_dialog_open_multiple', options); }},
    async save(options = {{}}) {{ return poly.invoke('__poly_dialog_save', options); }},
    async folder(options = {{}}) {{ return poly.invoke('__poly_dialog_folder', options); }},
    async message(title, message, level = 'info') {{
      return __polyDialog.show(level, title, message, [{{ text: 'OK', value: true, primary: true }}]);
    }},
    async confirm(title, message) {{
      return __polyDialog.show('confirm', title, message, [
        {{ text: 'Cancel', value: false }},
        {{ text: 'Confirm', value: true, primary: true }}
      ]);
    }},
    async custom(options) {{
      return __polyDialog.show(options.type || 'info', options.title, options.message, options.buttons || [{{ text: 'OK', value: true, primary: true }}]);
    }}
  }},
  fs: {{
    async read(path) {{ return poly.invoke('__poly_fs_read', {{ path }}); }},
    async write(path, content) {{ return poly.invoke('__poly_fs_write', {{ path, content }}); }},
    async exists(path) {{ return poly.invoke('__poly_fs_exists', {{ path }}); }},
    async readDir(path) {{ return poly.invoke('__poly_fs_read_dir', {{ path }}); }}
  }},
  {NCOS_VIRTUAL_DRIVE_API}
  updater: {{
    async checkGithub(repo, currentVersion) {{ return poly.invoke('__poly_updater_check_github', {{ repo, currentVersion }}); }},
    async checkUrl(url, currentVersion) {{ return poly.invoke('__poly_updater_check_url', {{ url, currentVersion }}); }},
    async download(url) {{ return poly.invoke('__poly_updater_download', {{ url }}); }},
    async install(path) {{ return poly.invoke('__poly_updater_install', {{ path }}); }},
    async checkAndPrompt(options) {{
      const info = options.repo
        ? await poly.updater.checkGithub(options.repo, options.currentVersion)
        : await poly.updater.checkUrl(options.url, options.currentVersion);
      if (info.update_available) {{
        const confirmed = await poly.dialog.confirm('Update Available',
          `Version ${{info.latest_version}} is available. Download now?`);
        if (confirmed && info.download_url) {{
          const path = await poly.updater.download(info.download_url);
          await poly.updater.install(path);
        }}
      }}
      return info;
    }}
  }},
  window: {{
    minimize() {{ if (window.ipc) window.ipc.postMessage('minimize'); }},
    maximize() {{ if (window.ipc) window.ipc.postMessage('maximize'); }},
    close() {{ if (window.ipc) window.ipc.postMessage('close'); }},
    hide() {{ if (window.ipc) window.ipc.postMessage('hide'); }},
    show() {{ if (window.ipc) window.ipc.postMessage('show'); }},
    async setTitle(title) {{ return poly.invoke('__poly_window_set_title', {{ title }}); }},
    async getTitle() {{ return poly.invoke('__poly_window_get_title', {{}}); }},
    async center() {{ return poly.invoke('__poly_window_center', {{}}); }},
    async setSize(width, height) {{ return poly.invoke('__poly_window_set_size', {{ width, height }}); }},
    async getSize() {{ return poly.invoke('__poly_window_get_size', {{}}); }},
    async setPosition(x, y) {{ return poly.invoke('__poly_window_set_position', {{ x, y }}); }},
    async getPosition() {{ return poly.invoke('__poly_window_get_position', {{}}); }},
    async setMinSize(width, height) {{ return poly.invoke('__poly_window_set_min_size', {{ width, height }}); }},
    async setMaxSize(width, height) {{ return poly.invoke('__poly_window_set_max_size', {{ width, height }}); }},
    async setAlwaysOnTop(value) {{ return poly.invoke('__poly_window_set_always_on_top', {{ value }}); }},
    async setFullscreen(value) {{ return poly.invoke('__poly_window_set_fullscreen', {{ value }}); }},
    async isFullscreen() {{ return poly.invoke('__poly_window_is_fullscreen', {{}}); }},
    async isMaximized() {{ return poly.invoke('__poly_window_is_maximized', {{}}); }},
    async isMinimized() {{ return poly.invoke('__poly_window_is_minimized', {{}}); }}
  }},
  clipboard: {{
    async read() {{ return poly.invoke('__poly_clipboard_read', {{}}); }},
    async write(text) {{ return poly.invoke('__poly_clipboard_write', {{ text }}); }},
    async clear() {{ return poly.invoke('__poly_clipboard_clear', {{}}); }}
  }},
  windows: {{
    async create(options = {{}}) {{ return poly.invoke('__poly_window_create', options); }},
    async close(id) {{ return poly.invoke('__poly_window_close', {{ id }}); }},
    async closeAll() {{ return poly.invoke('__poly_window_close_all', {{}}); }},
    async list() {{ return poly.invoke('__poly_window_list', {{}}); }},
    async count() {{ return poly.invoke('__poly_window_count', {{}}); }},
    async minimize(id) {{ return poly.invoke('__poly_window_minimize', {{ id }}); }},
    async maximize(id) {{ return poly.invoke('__poly_window_maximize', {{ id }}); }},
    async restore(id) {{ return poly.invoke('__poly_window_restore', {{ id }}); }},
    async show(id) {{ return poly.invoke('__poly_window_show', {{ id }}); }},
    async hide(id) {{ return poly.invoke('__poly_window_hide', {{ id }}); }},
    async focus(id) {{ return poly.invoke('__poly_window_focus', {{ id }}); }},
    async setTitle(id, title) {{ return poly.invoke('__poly_window_set_title', {{ id, title }}); }},
    async setSize(id, width, height) {{ return poly.invoke('__poly_window_set_size', {{ id, width, height }}); }},
    async setPosition(id, x, y) {{ return poly.invoke('__poly_window_set_position', {{ id, x, y }}); }},
    async setAlwaysOnTop(id, value) {{ return poly.invoke('__poly_window_set_always_on_top', {{ id, value }}); }},
    async setFullscreen(id, value) {{ return poly.invoke('__poly_window_set_fullscreen', {{ id, value }}); }},
    async navigate(id, url) {{ return poly.invoke('__poly_window_navigate', {{ id, url }}); }},
    async loadHtml(id, html) {{ return poly.invoke('__poly_window_load_html', {{ id, html }}); }},
    async eval(id, script) {{ return poly.invoke('__poly_window_eval', {{ id, script }}); }},
    async getState(id) {{ return poly.invoke('__poly_window_get_state', {{ id }}); }},
    async listStates() {{ return poly.invoke('__poly_window_list_states', {{}}); }}
  }},
  notification: {{
    async show(title, body, icon) {{ return poly.invoke('__poly_notification_show', {{ title, body, icon }}); }},
    async showWithTimeout(title, body, timeout) {{ return poly.invoke('__poly_notification_show_timeout', {{ title, body, timeout }}); }}
  }},
  deeplink: {{
    async register(protocol, appName) {{ return poly.invoke('__poly_deeplink_register', {{ protocol, appName }}); }},
    async unregister(protocol) {{ return poly.invoke('__poly_deeplink_unregister', {{ protocol }}); }},
    async isRegistered(protocol) {{ return poly.invoke('__poly_deeplink_is_registered', {{ protocol }}); }},
    async get() {{ return poly.invoke('__poly_deeplink_get', {{}}); }},
    async has() {{ return poly.invoke('__poly_deeplink_has', {{}}); }}
  }},
  tray: {{
    // Listen for tray menu clicks
    onMenuClick(callback) {{
      window.addEventListener('polytray', (e) => callback(e.detail.id));
    }},
    // Check if tray is enabled (from poly.toml config)
    async isEnabled() {{ return poly.invoke('__poly_tray_is_enabled', {{}}); }}
  }},
  shell: {{
    async open(url) {{ return poly.invoke('__poly_shell_open', {{ url }}); }},
    async openPath(path) {{ return poly.invoke('__poly_shell_open_path', {{ path }}); }},
    async openWith(path, app) {{ return poly.invoke('__poly_shell_open_with', {{ path, app }}); }}
  }},
  app: {{
    async getVersion() {{ return poly.invoke('__poly_app_get_version', {{}}); }},
    async getName() {{ return poly.invoke('__poly_app_get_name', {{}}); }},
    async getPath(name) {{ return poly.invoke('__poly_app_get_path', {{ name }}); }},
    async exit(code = 0) {{ return poly.invoke('__poly_app_exit', {{ code }}); }},
    async relaunch() {{ return poly.invoke('__poly_app_relaunch', {{}}); }}
  }},
  os: {{
    async platform() {{ return poly.invoke('__poly_os_platform', {{}}); }},
    async arch() {{ return poly.invoke('__poly_os_arch', {{}}); }},
    async version() {{ return poly.invoke('__poly_os_version', {{}}); }},
    async hostname() {{ return poly.invoke('__poly_os_hostname', {{}}); }},
    async homedir() {{ return poly.invoke('__poly_os_homedir', {{}}); }},
    async tempdir() {{ return poly.invoke('__poly_os_tempdir', {{}}); }}
  }},
  // Network API - HTTP requests
  http: {{
    async get(url, options = {{}}) {{ return poly.invoke('__poly_http_get', {{ url, ...options }}); }},
    async post(url, body, options = {{}}) {{ return poly.invoke('__poly_http_post', {{ url, body, ...options }}); }},
    async put(url, body, options = {{}}) {{ return poly.invoke('__poly_http_put', {{ url, body, ...options }}); }},
    async patch(url, body, options = {{}}) {{ return poly.invoke('__poly_http_patch', {{ url, body, ...options }}); }},
    async delete(url, options = {{}}) {{ return poly.invoke('__poly_http_delete', {{ url, ...options }}); }},
    async request(options) {{ return poly.invoke('__poly_http_request', options); }}
  }},
  // SQLite Database API
  db: {{
    async open(path) {{ return poly.invoke('__poly_db_open', {{ path }}); }},
    async close(id) {{ return poly.invoke('__poly_db_close', {{ id }}); }},
    async execute(id, sql, params = []) {{ return poly.invoke('__poly_db_execute', {{ id, sql, params }}); }},
    async query(id, sql, params = []) {{ return poly.invoke('__poly_db_query', {{ id, sql, params }}); }},
    async queryOne(id, sql, params = []) {{ return poly.invoke('__poly_db_query_one', {{ id, sql, params }}); }}
  }},
  // Browser API - Build browsers with Poly
  browser: {{
    async createTab(url) {{ return poly.invoke('__poly_browser_create_tab', {{ url }}); }},
    async closeTab(id) {{ return poly.invoke('__poly_browser_close_tab', {{ id }}); }},
    async getTab(id) {{ return poly.invoke('__poly_browser_get_tab', {{ id }}); }},
    async listTabs() {{ return poly.invoke('__poly_browser_list_tabs', {{}}); }},
    async navigate(id, url) {{ return poly.invoke('__poly_browser_navigate', {{ id, url }}); }},
    async back(id) {{ return poly.invoke('__poly_browser_back', {{ id }}); }},
    async forward(id) {{ return poly.invoke('__poly_browser_forward', {{ id }}); }},
    async setTitle(id, title) {{ return poly.invoke('__poly_browser_set_title', {{ id, title }}); }},
    async setLoading(id, loading) {{ return poly.invoke('__poly_browser_set_loading', {{ id, loading }}); }},
    async getHistory(id) {{ return poly.invoke('__poly_browser_get_history', {{ id }}); }},
    async clearHistory(id) {{ return poly.invoke('__poly_browser_clear_history', {{ id }}); }},
    async fetch(url) {{ return poly.invoke('__poly_browser_fetch', {{ url }}); }},
    // Proxy URL - use this to load external resources through the local server
    proxyUrl(url) {{ return '/__poly_proxy?url=' + encodeURIComponent(url); }},
    // Navigate the current WebView to a URL (replaces current page)
    loadUrl(url) {{ window.location.href = url; }},
    // Go back in browser history
    goBack() {{ window.history.back(); }},
    // Go forward in browser history
    goForward() {{ window.history.forward(); }},
    // Open a real WebView window (full browser functionality)
    async openWindow(url, options = {{}}) {{ return poly.invoke('__poly_browser_open_window', {{ url, ...options }}); }},
    async windowNavigate(id, url) {{ return poly.invoke('__poly_browser_window_navigate', {{ id, url }}); }},
    async windowClose(id) {{ return poly.invoke('__poly_browser_window_close', {{ id }}); }}
  }},
  // Titlebar API - Custom persistent titlebar for browser apps
  titlebar: {{
    // Set custom titlebar (persists across navigation)
    async set(config) {{ return poly.invoke('__poly_titlebar_set', config); }},
    // Get current titlebar config
    async get() {{ return poly.invoke('__poly_titlebar_get', {{}}); }},
    // Enable/disable titlebar
    async setEnabled(enabled) {{ return poly.invoke('__poly_titlebar_set_enabled', {{ enabled }}); }},
    // Set titlebar height
    async setHeight(height) {{ return poly.invoke('__poly_titlebar_set_height', {{ height }}); }},
    // Update titlebar HTML
    async setHtml(html) {{ return poly.invoke('__poly_titlebar_set_html', {{ html }}); }},
    // Update titlebar CSS
    async setCss(css) {{ return poly.invoke('__poly_titlebar_set_css', {{ css }}); }},
    // Update titlebar JavaScript
    async setJs(js) {{ return poly.invoke('__poly_titlebar_set_js', {{ js }}); }},
    // Navigate the main content area to a URL (keeps titlebar)
    async navigate(url) {{ return poly.invoke('__poly_titlebar_navigate', {{ url }}); }}
  }},
  // WebView API - Multi-WebView management for browser apps
  webview: {{
    // Create a new WebView
    // Options: url, html, x, y, width, height, visible, transparent, devtools, initScripts (array of scripts to run before page loads)
    async create(id, options = {{}}) {{ return poly.invoke('__poly_webview_create', {{ id, ...options }}); }},
    // Navigate a WebView to URL
    async navigate(id, url) {{ return poly.invoke('__poly_webview_navigate', {{ id, url }}); }},
    // Load HTML content directly
    async loadHtml(id, html) {{ return poly.invoke('__poly_webview_load_html', {{ id, html }}); }},
    // Go back in history
    async goBack(id) {{ return poly.invoke('__poly_webview_go_back', {{ id }}); }},
    // Go forward in history
    async goForward(id) {{ return poly.invoke('__poly_webview_go_forward', {{ id }}); }},
    // Reload the page
    async reload(id) {{ return poly.invoke('__poly_webview_reload', {{ id }}); }},
    // Stop loading
    async stop(id) {{ return poly.invoke('__poly_webview_stop', {{ id }}); }},
    // Set WebView bounds (position and size)
    async setBounds(id, bounds) {{ return poly.invoke('__poly_webview_set_bounds', {{ id, ...bounds }}); }},
    // Get WebView bounds
    async getBounds(id) {{ return poly.invoke('__poly_webview_get_bounds', {{ id }}); }},
    // Execute JavaScript in a WebView
    async eval(id, script) {{ return poly.invoke('__poly_webview_eval', {{ id, script }}); }},
    // Destroy a WebView
    async destroy(id) {{ return poly.invoke('__poly_webview_destroy', {{ id }}); }},
    // List all WebViews
    async list() {{ return poly.invoke('__poly_webview_list', {{}}); }},
    // Get WebView info (includes isLoading, canGoBack, canGoForward)
    async get(id) {{ return poly.invoke('__poly_webview_get', {{ id }}); }},
    // Show/hide a WebView
    async setVisible(id, visible) {{ return poly.invoke('__poly_webview_set_visible', {{ id, visible }}); }},
    // Focus a WebView
    async focus(id) {{ return poly.invoke('__poly_webview_focus', {{ id }}); }},
    // Set zoom level (1.0 = 100%)
    async setZoom(id, level) {{ return poly.invoke('__poly_webview_set_zoom', {{ id, level }}); }},
    // Set main WebView bounds (the app's original WebView)
    async setMainBounds(bounds) {{ return poly.invoke('__poly_webview_set_main_bounds', bounds); }},
    // Poll for events (navigation, title change, etc.)
    async pollEvents() {{ return poly.invoke('__poly_webview_poll_events', {{}}); }},
    // Grant or deny a permission request
    async respondToPermission(id, permission, granted) {{ return poly.invoke('__poly_webview_respond_permission', {{ id, permission, granted }}); }},
    // Event listeners (client-side convenience)
    _listeners: {{}},
    on(event, id, callback) {{
      const key = `${{event}}:${{id}}`;
      if (!this._listeners[key]) this._listeners[key] = [];
      this._listeners[key].push(callback);
    }},
    off(event, id, callback) {{
      const key = `${{event}}:${{id}}`;
      if (this._listeners[key]) {{
        this._listeners[key] = this._listeners[key].filter(cb => cb !== callback);
      }}
    }},
    _emit(event, id, data) {{
      const key = `${{event}}:${{id}}`;
      if (this._listeners[key]) {{
        this._listeners[key].forEach(cb => cb(data));
      }}
      // Also emit to wildcard listeners
      const wildcardKey = `${{event}}:*`;
      if (this._listeners[wildcardKey]) {{
        this._listeners[wildcardKey].forEach(cb => cb(id, data));
      }}
    }},
    // Convenience event registration
    onNavigate(id, cb) {{ this.on('navigate', id, cb); }},
    onTitleChange(id, cb) {{ this.on('titleChange', id, cb); }},
    onLoadStart(id, cb) {{ this.on('loadStart', id, cb); }},
    onLoadFinish(id, cb) {{ this.on('loadFinish', id, cb); }},
    onNewWindow(id, cb) {{ this.on('newWindow', id, cb); }},
    onDownload(id, cb) {{ this.on('download', id, cb); }},
    onClose(id, cb) {{ this.on('close', id, cb); }},
    onHistoryChange(id, cb) {{ this.on('historyChange', id, cb); }}
  }},
  // Managed CEF off-screen browser surfaces
  cef: {{
    async available() {{ return poly.invoke('__poly_cef_available', {{}}); }},
    async create(id, url, width = 800, height = 600, bounds = {{}}) {{ return poly.invoke('__poly_cef_create', {{ id, url, width, height, ...bounds }}); }},
    async navigate(id, url) {{ return poly.invoke('__poly_cef_navigate', {{ id, url }}); }},
    async goBack(id) {{ return poly.invoke('__poly_cef_history', {{ id, direction: 'back' }}); }},
    async goForward(id) {{ return poly.invoke('__poly_cef_history', {{ id, direction: 'forward' }}); }},
    async reload(id) {{ return poly.invoke('__poly_cef_reload', {{ id }}); }},
    async resize(id, width, height) {{ return poly.invoke('__poly_cef_resize', {{ id, width, height }}); }},
    async setBounds(id, bounds) {{ return poly.invoke('__poly_cef_bounds', {{ id, ...bounds }}); }},
    async setVisible(id, visible) {{ return poly.invoke('__poly_cef_visible', {{ id, visible }}); }},
    async mouseMove(id, x, y, leave = false) {{ return poly.invoke('__poly_cef_mouse_move', {{ id, x, y, leave }}); }},
    async mouseButton(id, x, y, down) {{ return poly.invoke('__poly_cef_mouse_button', {{ id, x, y, down }}); }},
    async mouseWheel(id, x, y, deltaX, deltaY) {{ return poly.invoke('__poly_cef_mouse_wheel', {{ id, x, y, deltaX, deltaY }}); }},
    async key(id, type, keyCode, character = 0, modifiers = 0) {{ return poly.invoke('__poly_cef_key', {{ id, type, keyCode, character, modifiers }}); }},
    async destroy(id) {{ return poly.invoke('__poly_cef_destroy', {{ id }}); }},
    async favicon(id) {{ return poly.invoke('__poly_cef_favicon', {{ id }}); }},
    async metadata(id) {{ return poly.invoke('__poly_cef_metadata', {{ id }}); }},
    async readFrame(id, since = 0) {{
      const response = await fetch('/__poly_cef_frame?id=' + encodeURIComponent(id) + '&since=' + Number(since || 0) + '&wait_ms=100', {{ cache: 'no-store' }});
      if (response.status === 304) return null;
      if (!response.ok) throw new Error('CEF frame is not ready.');
      return {{ width: Number(response.headers.get('X-Poly-Cef-Width')), height: Number(response.headers.get('X-Poly-Cef-Height')), sequence: Number(response.headers.get('X-Poly-Cef-Sequence')), dirtyX: Number(response.headers.get('X-Poly-Cef-Dirty-X')), dirtyY: Number(response.headers.get('X-Poly-Cef-Dirty-Y')), dirtyWidth: Number(response.headers.get('X-Poly-Cef-Dirty-Width')), dirtyHeight: Number(response.headers.get('X-Poly-Cef-Dirty-Height')), rgba: await response.arrayBuffer() }};
    }}
  }},
  // Ad Blocking API
  adblock: {{
    // Initialize ad blocker with filter lists
    async init(filterLists) {{ return poly.invoke('__poly_adblock_init', {{ filterLists }}); }},
    // Check if a URL should be blocked
    async shouldBlock(url, sourceUrl, resourceType = 'other') {{
      return poly.invoke('__poly_adblock_should_block', {{ url, sourceUrl, resourceType }});
    }},
    // Download EasyList filter list
    async downloadEasyList() {{ return poly.invoke('__poly_adblock_download_easylist', {{}}); }},
    // Download EasyPrivacy filter list
    async downloadEasyPrivacy() {{ return poly.invoke('__poly_adblock_download_easyprivacy', {{}}); }},
    // Download Pi-hole block list
    async downloadPiHole() {{ return poly.invoke('__poly_adblock_download_pihole', {{}}); }},
    // Parse Pi-hole hosts file
    async parsePiHole(content) {{ return poly.invoke('__poly_adblock_parse_pihole', {{ content }}); }},
    // Get blocking statistics
    async getStats() {{ return poly.invoke('__poly_adblock_get_stats', {{}}); }}
  }},
  // MultiView API - Create windows with multiple WebViews
  multiview: {{
    // Create a new multi-view window
    // views: array of {{ id, url, x, y, width, height }}
    // Views are stacked: first in array = bottom, last = top (for UI)
    async create(options) {{ return poly.invoke('__poly_multiview_create', options); }},
    // Navigate a view to URL
    async navigate(windowId, viewId, url) {{ return poly.invoke('__poly_multiview_navigate', {{ windowId, viewId, url }}); }},
    // Send message to a view (triggers 'polymessage' event)
    async postMessage(windowId, viewId, message) {{ return poly.invoke('__poly_multiview_post_message', {{ windowId, viewId, message: JSON.stringify(message) }}); }},
    // Set view bounds
    async setBounds(windowId, viewId, bounds) {{ return poly.invoke('__poly_multiview_set_bounds', {{ windowId, viewId, ...bounds }}); }},
    // Close a multi-view window
    async close(windowId) {{ return poly.invoke('__poly_multiview_close', {{ windowId }}); }},
    // List all multi-view windows
    async list() {{ return poly.invoke('__poly_multiview_list', {{}}); }},
    // Get window info
    async get(windowId) {{ return poly.invoke('__poly_multiview_get', {{ windowId }}); }}
  }},
  // PolyView API - "iframe2" that bypasses all iframe restrictions
  // Use <poly-view src="https://example.com"></poly-view> in your HTML
  polyview: {{
    // Get the proxy URL for a target URL
    proxyUrl(url) {{ return '/__polyview/?url=' + encodeURIComponent(url); }},
    // Navigate a poly-view element
    navigate(element, url) {{
      if (element && element.navigate) element.navigate(url);
      else if (element) element.src = this.proxyUrl(url);
    }},
    // Check if PolyView is available
    isAvailable() {{ return typeof customElements !== 'undefined' && customElements.get('poly-view') !== undefined; }}
  }},
  // AI/LLM API - Chat with AI models
  ai: {{
    // Chat with Ollama (local)
    async ollama(model, messages, options = {{}}) {{
      return poly.invoke('__poly_ai_ollama', {{ model, messages, ...options }});
    }},
    // Chat with OpenAI
    async openai(model, messages, apiKey, options = {{}}) {{
      return poly.invoke('__poly_ai_openai', {{ model, messages, apiKey, ...options }});
    }},
    // Chat with Anthropic Claude
    async anthropic(model, messages, apiKey, options = {{}}) {{
      return poly.invoke('__poly_ai_anthropic', {{ model, messages, apiKey, ...options }});
    }},
    // Chat with custom OpenAI-compatible API
    async custom(baseUrl, model, messages, options = {{}}) {{
      return poly.invoke('__poly_ai_custom', {{ baseUrl, model, messages, ...options }});
    }},
    // Check if Ollama is running
    async checkOllama() {{
      return poly.invoke('__poly_ai_check_ollama', {{}});
    }},
    // List available Ollama models
    async listModels() {{
      return poly.invoke('__poly_ai_list_models', {{}});
    }},
    // Generic chat function (auto-detects provider)
    async chat(options) {{
      return poly.invoke('__poly_ai_chat', options);
    }},
    // Streaming API
    stream: {{
      // Start streaming chat with Ollama
      async start(model, messages, options = {{}}) {{
        return poly.invoke('__poly_ai_stream_start', {{ model, messages, ...options }});
      }},
      // Poll for new chunks (returns {{ chunks: [], done: bool }})
      async poll(streamId) {{
        return poly.invoke('__poly_ai_stream_poll', {{ streamId }});
      }},
      // Cancel/stop a stream
      async cancel(streamId) {{
        return poly.invoke('__poly_ai_stream_cancel', {{ streamId }});
      }},
      // List active streams
      async list() {{
        return poly.invoke('__poly_ai_stream_list', {{}});
      }},
      // Helper: Stream with callback (handles polling automatically)
      async run(model, messages, options = {{}}, onChunk) {{
        const result = await this.start(model, messages, options);
        if (result.error) throw new Error(result.error);
        const streamId = result.streamId;

        const poll = async () => {{
          const {{ chunks, done }} = await this.poll(streamId);
          for (const chunk of chunks) {{
            if (onChunk) onChunk(chunk);
          }}
          if (!done) {{
            await new Promise(r => setTimeout(r, 16)); // ~60fps polling
            await poll();
          }}
        }};

        await poll();
        return streamId;
      }}
    }}
  }}
}};
// Initialize Lucide Icons
if (typeof lucide !== 'undefined') lucide.createIcons();
// Hot Reload
(function() {{
  let v = {}, polling = false;
  async function check() {{
    if (!document.hidden) {{
      try {{ const r = await fetch('/__poly_reload'); const d = await r.json(); if (d.version > v) {{ v = d.version; location.reload(); }} }} catch(e) {{}}
    }}
    if (polling) setTimeout(check, {reload_interval});
  }}
  function start() {{ if (!polling) {{ polling = true; check(); }} }}
  function stop() {{ polling = false; }}
  document.addEventListener('visibilitychange', () => document.hidden ? stop() : start());
  start();
}})();
</script>"#, reload_counter_http.load(Ordering::Relaxed), reload_interval = reload_interval);
                        // Insert before </body> or at end
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
                    // IPC Bridge - call Poly functions from JavaScript (stateful)
                    let mut body = String::new();
                    request.as_reader().read_to_string(&mut body).ok();

                    let result = handle_ipc_invoke_stateful(&interpreter_http, &body);
                    tiny_http::Response::from_string(result)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                        .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
                }
                "/__poly_run" => {
                    let output = execute_poly_for_web(&entry_http);
                    tiny_http::Response::from_string(output)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
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
    let _success = poly::run(&fs::read_to_string(&entry).unwrap_or_default()).is_ok();
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

                    // Only reload Poly interpreter for .poly file changes
                    let has_poly_change = event.paths.iter().any(|p| {
                        p.extension().and_then(|e| e.to_str()) == Some("poly")
                    });

                    if has_poly_change {
                        let start = std::time::Instant::now();
                        let source = fs::read_to_string(&entry).unwrap_or_default();

                        // Reload the persistent interpreter
                        let mut interp = interpreter.lock().unwrap();
                        *interp = poly::create_interpreter();
                        match poly::init_interpreter(&mut interp, &source) {
                            Ok(_) => println!(" {}({}ms){}", DIM, start.elapsed().as_millis(), RESET),
                            Err(e) => println!("\n  {}error{}: {}", RED, RESET, e),
                        }
                    } else {
                        // For HTML/CSS/JS, just signal reload (no Poly execution needed)
                        println!();
                    }
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

fn execute_poly_for_web(entry: &Path) -> String {
    match fs::read_to_string(entry) {
        Ok(source) => match poly::run(&source) {
            Ok(output) => serde_json::json!({"success": true, "output": output}).to_string(),
            Err(e) => serde_json::json!({"success": false, "error": e}).to_string(),
        },
        Err(e) => serde_json::json!({"success": false, "error": format!("Failed to read: {}", e)}).to_string(),
    }
}

/// Handle IPC invoke with a persistent (stateful) interpreter
struct ChannelReader {
    rx: std::sync::mpsc::Receiver<String>,
    buffer: Vec<u8>,
}

impl std::io::Read for ChannelReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        while self.buffer.is_empty() {
             match self.rx.recv() {
                 Ok(data) => {
                     if !data.is_empty() {
                        self.buffer.extend_from_slice(data.as_bytes());
                        break;
                     }
                 }
                 Err(_) => return Ok(0),
             }
        }
        let len = std::cmp::min(self.buffer.len(), buf.len());
        buf[0..len].copy_from_slice(&self.buffer[0..len]);
        self.buffer.drain(0..len);
        Ok(len)
    }
}

fn handle_ipc_invoke_streaming(
    interpreter: &std::sync::Arc<std::sync::Mutex<poly::interpreter::Interpreter>>,
    body: &str
) -> Result<Box<dyn std::io::Read + Send>, String> {
    // Parse JSON
    let request: serde_json::Value = serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {}", e))?;
    let fn_name = request.get("fn").and_then(|v| v.as_str()).ok_or("Missing 'fn' field")?.to_string();
    let args = request.get("args").cloned().unwrap_or(serde_json::json!({}));

    // Build args string
    let args_str = if let Some(arr) = args.as_array() {
        arr.iter().map(json_to_poly_value).collect::<Vec<_>>().join(", ")
    } else if args.is_object() && args.as_object().map(|o| o.is_empty()).unwrap_or(false) {
        String::new()
    } else {
        json_to_poly_value(&args)
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let interpreter_clone = interpreter.clone();

    std::thread::spawn(move || {
        // Lock interpreter
        // Note: This blocks until interpreter is free
        let mut interp = interpreter_clone.lock().unwrap();

        // Set up streaming
        interp.set_stream_sender(Some(tx.clone()));

        // Execute
        let _ = poly::call_function(&mut interp, &fn_name, &args_str);

        // Clean up
        interp.set_stream_sender(None);
        // tx is dropped here, closing channel
    });

    Ok(Box::new(ChannelReader { rx, buffer: Vec::new() }))
}

pub(super) fn handle_ipc_invoke_stateful(interpreter: &std::sync::Arc<std::sync::Mutex<poly::interpreter::Interpreter>>, body: &str) -> String {
    // Parse the request: { "fn": "function_name", "args": { ... } }
    let request: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return serde_json::json!({"error": format!("Invalid JSON: {}", e)}).to_string(),
    };

    let fn_name = match request.get("fn").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => return serde_json::json!({"error": "Missing 'fn' field"}).to_string(),
    };

    let args = request.get("args").cloned().unwrap_or(serde_json::json!({}));

    // Handle built-in system APIs (prefixed with __poly_)
    if fn_name.starts_with("__poly_") {
        return handle_system_api(fn_name, &args);
    }

    // Build argument string from JSON - support array unpacking or skip empty objects
    let args_str = if let Some(arr) = args.as_array() {
        // Unpack array arguments: [1, 2] -> "1, 2"
        arr.iter().map(json_to_poly_value).collect::<Vec<_>>().join(", ")
    } else if args.is_object() && args.as_object().map(|o| o.is_empty()).unwrap_or(false) {
        String::new() // No arguments
    } else {
        json_to_poly_value(&args)
    };

    // Call function on persistent interpreter
    let mut interp = interpreter.lock().unwrap();
    match poly::call_function(&mut interp, fn_name, &args_str) {
        Ok(json_result) => {
            format!(r#"{{"result":{}}}"#, json_result)
        }
        Err(e) => serde_json::json!({"error": e}).to_string(),
    }
}

#[allow(dead_code)]
fn handle_ipc_invoke(entry: &Path, body: &str) -> String {
    // Parse the request: { "fn": "function_name", "args": { ... } }
    let request: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return serde_json::json!({"error": format!("Invalid JSON: {}", e)}).to_string(),
    };

    let fn_name = match request.get("fn").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => return serde_json::json!({"error": "Missing 'fn' field"}).to_string(),
    };

    let args = request.get("args").cloned().unwrap_or(serde_json::json!({}));

    // Handle built-in system APIs (prefixed with __poly_)
    if fn_name.starts_with("__poly_") {
        return handle_system_api(fn_name, &args);
    }

    // Read the Poly source
    let source = match fs::read_to_string(entry) {
        Ok(s) => s,
        Err(e) => return serde_json::json!({"error": format!("Failed to read: {}", e)}).to_string(),
    };

    // Build argument string from JSON - support array unpacking or skip empty objects
    let args_str = if let Some(arr) = args.as_array() {
        // Unpack array arguments: [1, 2] -> "1, 2"
        arr.iter().map(json_to_poly_value).collect::<Vec<_>>().join(", ")
    } else if args.is_object() && args.as_object().map(|o| o.is_empty()).unwrap_or(false) {
        String::new() // No arguments
    } else {
        json_to_poly_value(&args)
    };

    // Create a call expression - the result will be the last evaluated value
    let call_expr = format!("{fn_name}({args_str})");
    let full_source = format!("{source}\n{call_expr}");

    // Use eval_json to get proper JSON output
    match poly::eval_json(&full_source) {
        Ok(json_result) => {
            // The result is already valid JSON, wrap it in the response
            format!(r#"{{"result":{}}}"#, json_result)
        }
        Err(e) => serde_json::json!({"error": e}).to_string(),
    }
}

/// Handle IPC when there's no Poly backend (system APIs only)
pub(super) fn handle_ipc_invoke_system_only(body: &str) -> String {
    let request: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return serde_json::json!({"error": format!("Invalid JSON: {}", e)}).to_string(),
    };

    let fn_name = match request.get("fn").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => return serde_json::json!({"error": "Missing 'fn' field"}).to_string(),
    };

    let args = request.get("args").cloned().unwrap_or(serde_json::json!({}));

    if fn_name.starts_with("__poly_") {
        handle_system_api(fn_name, &args)
    } else {
        serde_json::json!({"error": "No Poly backend available"}).to_string()
    }
}


fn json_to_poly_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "none".to_string(),
        serde_json::Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("\"{}\"", s.replace("\"", "\\\"")),
        serde_json::Value::Array(arr) => {
            let items: Vec<_> = arr.iter().map(json_to_poly_value).collect();
            format!("[{}]", items.join(", "))
        }
        serde_json::Value::Object(obj) => {
            let items: Vec<_> = obj.iter()
                .map(|(k, v)| format!("\"{}\": {}", k, json_to_poly_value(v)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
    }
}


fn generate_dev_html(project_path: &Path, entry: &Path, version: u64) -> String {
    let name = project_path.file_name().and_then(|n| n.to_str()).unwrap_or("Poly App");
    let entry_name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("main.poly");

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
            .map(|ext| matches!(ext, "poly" | "html" | "css" | "js"))
            .unwrap_or(false)
    })
}

pub(super) fn find_entry_point(project_path: &Path) -> Option<std::path::PathBuf> {
    for candidate in ["main.poly", "src/main.poly", "app.poly", "src/app.poly", "index.poly"] {
        let path = project_path.join(candidate);
        if path.exists() { return Some(path); }
    }
    None
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

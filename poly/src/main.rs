use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Write, BufRead, Read};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use serde_json;

// ANSI color codes for terminal output
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

const VERSION: &str = "0.1.5";
const GITHUB_REPO: &str = "OpenChatGit/Poly";

#[derive(Parser)]
#[command(name = "poly")]
#[command(about = "Poly - Build native apps with web technologies", long_about = None)]
#[command(disable_version_flag = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Source file to execute
    file: Option<String>,
    
    /// Run in REPL mode
    #[arg(short, long)]
    repl: bool,
    
    /// Evaluate a single expression
    #[arg(short, long)]
    eval: Option<String>,
    
    /// Show version and check for updates
    #[arg(short = 'V', long = "version")]
    version: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start development server with hot reload
    Dev {
        /// Project directory (default: current directory)
        #[arg(default_value = ".")]
        path: String,
        
        /// Port for dev server
        #[arg(short, long, default_value = "3000")]
        port: u16,
        
        /// Open browser automatically
        #[arg(long)]
        open: bool,
    },
    
    /// Run the application
    Run {
        /// Project directory or file
        #[arg(default_value = ".")]
        path: String,
        
        /// Run in release mode
        #[arg(long)]
        release: bool,
        
        /// Run as native window (requires --features native)
        #[arg(long)]
        native: bool,
    },
    
    /// Build the application
    Build {
        /// Project directory
        #[arg(default_value = ".")]
        path: String,
        
        /// Build target (native, web, all)
        #[arg(short, long, default_value = "native")]
        target: String,
        
        /// Release build
        #[arg(long)]
        release: bool,
    },
    
    /// Create a new Poly project
    New {
        /// Project name
        name: String,
        
        /// Project template (app, lib)
        #[arg(short, long, default_value = "app")]
        template: String,
    },
    
    /// Initialize Poly in existing directory
    Init {
        /// Project template
        #[arg(short, long, default_value = "app")]
        template: String,
    },
    
    /// Check for updates
    Update,
}

fn main() {
    let cli = Cli::parse();
    
    // Handle version flag with update check
    if cli.version {
        print_version_with_update_check();
        return;
    }
    
    let result = match cli.command {
        Some(Commands::Dev { path, port, open }) => { run_dev_server(&path, port, open); Ok(()) },
        Some(Commands::Run { path, release, native }) => run_app_result(&path, release, native),
        Some(Commands::Build { path, target, release }) => { build_app(&path, &target, release); Ok(()) },
        Some(Commands::New { name, template }) => { create_project(&name, &template); Ok(()) },
        Some(Commands::Init { template }) => { init_project(&template); Ok(()) },
        Some(Commands::Update) => { check_for_updates_interactive(); Ok(()) },
        None => {
            if let Some(expr) = cli.eval {
                match poly::eval(&expr) {
                    Ok(result) => { println!("{}", result); Ok(()) },
                    Err(e) => Err(e),
                }
            } else if let Some(file) = cli.file {
                run_file_result(&file)
            } else if cli.repl || atty::is(atty::Stream::Stdin) {
                run_repl();
                Ok(())
            } else {
                let stdin = io::stdin();
                let source: String = stdin.lock().lines()
                    .filter_map(|l| l.ok())
                    .collect::<Vec<_>>()
                    .join("\n");
                
                poly::run(&source).map(|_| ())
            }
        }
    };
    
    if let Err(e) = result {
        eprintln!("{}error{}: {}", RED, RESET, e);
        std::process::exit(1);
    }
}

/// Print version and check for updates
fn print_version_with_update_check() {
    println!("poly {}", VERSION);
    println!();
    
    // Check for updates
    print!("  {}Checking for updates...{}", DIM, RESET);
    io::stdout().flush().ok();
    
    #[cfg(feature = "native")]
    {
        match poly::check_github_updates(GITHUB_REPO, VERSION) {
            Ok(info) => {
                // Clear the "Checking..." line
                print!("\r                                  \r");
                
                if info.update_available {
                    println!("  {}> New version available:{} {}", GREEN, RESET, info.latest_version);
                    println!();
                    
                    if let Some(notes) = &info.release_notes {
                        let short_notes: String = notes.lines().take(3).collect::<Vec<_>>().join("\n    ");
                        if !short_notes.is_empty() {
                            println!("  {}Release notes:{}", DIM, RESET);
                            println!("    {}", short_notes);
                            println!();
                        }
                    }
                    
                    print!("  {}Download and install now? [y/N]:{} ", CYAN, RESET);
                    io::stdout().flush().ok();
                    
                    let mut input = String::new();
                    if io::stdin().read_line(&mut input).is_ok() {
                        let input = input.trim().to_lowercase();
                        if input == "y" || input == "yes" {
                            println!();
                            download_and_install_update(&info);
                        }
                    }
                } else {
                    println!("  {}>{} You're on the latest version", GREEN, RESET);
                }
            }
            Err(_e) => {
                // Clear the "Checking..." line
                print!("\r                                  \r");
                println!("  {}(Could not check for updates){}", DIM, RESET);
            }
        }
    }
    
    #[cfg(not(feature = "native"))]
    {
        print!("\r                              \r");
        println!("{}(Update check requires native feature){}", DIM, RESET);
    }
}

/// Interactive update check command
fn check_for_updates_interactive() {
    println!();
    println!("  {}POLY{} v{}  {}update{}", CYAN, RESET, VERSION, DIM, RESET);
    println!();
    
    print!("  {}Checking for updates...{}", DIM, RESET);
    io::stdout().flush().ok();
    
    #[cfg(feature = "native")]
    {
        match poly::check_github_updates(GITHUB_REPO, VERSION) {
            Ok(info) => {
                print!("\r                                    \r");
                
                if info.update_available {
                    println!("  {}>{} New version available: {}{}{}", 
                        GREEN, RESET, BOLD, info.latest_version, RESET);
                    println!("  {}>{} Current version: {}", DIM, RESET, VERSION);
                    
                    if let Some(notes) = &info.release_notes {
                        println!();
                        println!("  {}Release notes:{}", DIM, RESET);
                        for line in notes.lines().take(10) {
                            println!("  {}", line);
                        }
                    }
                    
                    println!();
                    print!("  {}Download and install? [y/N]:{} ", CYAN, RESET);
                    io::stdout().flush().ok();
                    
                    let mut input = String::new();
                    if io::stdin().read_line(&mut input).is_ok() {
                        let input = input.trim().to_lowercase();
                        if input == "y" || input == "yes" {
                            println!();
                            download_and_install_update(&info);
                        } else {
                            println!();
                            println!("  {}Update cancelled.{}", DIM, RESET);
                        }
                    }
                } else {
                    println!("  {}>{} You're already on the latest version ({})", 
                        GREEN, RESET, VERSION);
                }
            }
            Err(e) => {
                print!("\r                                    \r");
                println!("  {}>{} {}Failed to check for updates:{} {}", RED, RESET, RED, RESET, e);
            }
        }
    }
    
    #[cfg(not(feature = "native"))]
    {
        print!("\r                                    \r");
        println!("  {}>{} Update check requires native feature", YELLOW, RESET);
        println!("  {}>{} Rebuild with: cargo install --path poly --features native", DIM, RESET);
    }
    
    println!();
}

/// Download and install update
#[cfg(feature = "native")]
fn download_and_install_update(info: &poly::UpdateInfo) {
    if let Some(url) = &info.download_url {
        print!("  {}Downloading update...{}", DIM, RESET);
        io::stdout().flush().ok();
        
        // Progress callback
        let progress = |downloaded: u64, total: u64| {
            if total > 0 {
                let percent = (downloaded * 100) / total;
                print!("\r  {}Downloading update... {}%{}", DIM, percent, RESET);
                io::stdout().flush().ok();
            }
        };
        
        match poly::download_update(url, Some(Box::new(progress))) {
            Ok(path) => {
                println!("\r  {}>{} Downloaded to: {}                    ", GREEN, RESET, path.display());
                print!("  {}Installing...{}", DIM, RESET);
                io::stdout().flush().ok();
                
                match poly::install_update(&path) {
                    Ok(_) => {
                        println!("\r  {}>{} Update installed successfully!          ", GREEN, RESET);
                        println!("  {}>{} Please restart Poly to use the new version.", DIM, RESET);
                    }
                    Err(e) => {
                        println!("\r  {}>{} {}Installation failed:{} {}          ", RED, RESET, RED, RESET, e);
                        println!("  {}>{} You can manually install from: {}", DIM, RESET, path.display());
                    }
                }
            }
            Err(e) => {
                println!("\r  {}>{} {}Download failed:{} {}          ", RED, RESET, RED, RESET, e);
            }
        }
    } else {
        println!("  {}>{} {}No download URL available for your platform{}", YELLOW, RESET, YELLOW, RESET);
        println!("  {}>{} Please download manually from GitHub", DIM, RESET);
    }
}

#[cfg(not(feature = "native"))]
fn download_and_install_update(_info: &poly::UpdateInfo) {
    println!("  {}>{} Update requires native feature", YELLOW, RESET);
}

fn run_file_result(file: &str) -> Result<(), String> {
    let source = fs::read_to_string(file)
        .map_err(|e| format!("Failed to read '{}': {}", file, e))?;
    poly::run(&source).map(|_| ())
}

fn run_repl() {
    println!();
    println!("  {}POLY{} v0.1.5", CYAN, RESET);
    println!("  {}Type 'exit' to quit{}", DIM, RESET);
    println!();
    
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    loop {
        print!("{}>{} ", CYAN, RESET);
        stdout.flush().unwrap();
        
        let mut input = String::new();
        if stdin.read_line(&mut input).is_err() {
            break;
        }
        
        let input = input.trim();
        if input.is_empty() { continue; }
        if input == "exit" || input == "quit" { break; }
        
        match poly::eval(input) {
            Ok(result) => {
                if result != "none" {
                    println!("{}", result);
                }
            }
            Err(_) => {
                if let Err(e) = poly::run(input) {
                    eprintln!("{}error{}: {}", RED, RESET, e);
                }
            }
        }
    }
}


fn run_dev_server(path: &str, port: u16, open_browser: bool) {
    use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
    use std::thread;
    
    let project_path = Path::new(path);
    let project_path_owned = project_path.to_path_buf();
    
    let entry = find_entry_point(project_path);
    if entry.is_none() {
        eprintln!("{}error{}: No entry point found. Create main.poly or src/main.poly", RED, RESET);
        std::process::exit(1);
    }
    let entry = entry.unwrap();
    
    println!();
    println!("  {}POLY{} v0.1.5  {}dev server{}", CYAN, RESET, DIM, RESET);
    println!();
    println!("  {}>{} Local:   {}http://localhost:{}{}", GREEN, RESET, CYAN, port, RESET);
    println!("  {}>{} Entry:   {}{}{}", DIM, RESET, DIM, entry.display(), RESET);
    println!();
    
    let reload_counter = Arc::new(AtomicU64::new(0));
    let reload_counter_http = Arc::clone(&reload_counter);
    let project_path_http = project_path_owned.clone();
    let entry_http = entry.clone();
    
    // HTTP server thread
    thread::spawn(move || {
        let server = tiny_http::Server::http(format!("0.0.0.0:{}", port))
            .expect("Failed to start HTTP server");
        
        for mut request in server.incoming_requests() {
            let url = request.url().to_string();
            let response = match url.as_str() {
                "/" | "/index.html" => {
                    // Check for custom index.html in web folder
                    let custom_index = project_path_http.join("web/index.html");
                    if custom_index.exists() {
                        let mut html = fs::read_to_string(&custom_index).unwrap_or_default();
                        
                        // Inject Alpine.js and Lucide Icons in <head>
                        let head_scripts = r#"<script defer src="https://unpkg.com/alpinejs@3/dist/cdn.min.js"></script>
<script src="https://unpkg.com/lucide@latest/dist/umd/lucide.min.js"></script>"#;
                        if html.contains("</head>") {
                            html = html.replace("</head>", &format!("{}</head>", head_scripts));
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
  async invoke(fn, args = {{}}) {{
    const r = await fetch('/__poly_invoke', {{
      method: 'POST',
      headers: {{ 'Content-Type': 'application/json' }},
      body: JSON.stringify({{ fn, args }})
    }});
    const d = await r.json();
    if (d.error) throw new Error(d.error);
    return d.result;
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
    show() {{ if (window.ipc) window.ipc.postMessage('show'); }}
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
    if (polling) setTimeout(check, 1000);
  }}
  function start() {{ if (!polling) {{ polling = true; check(); }} }}
  function stop() {{ polling = false; }}
  document.addEventListener('visibilitychange', () => document.hidden ? stop() : start());
  start();
}})();
</script>"#, reload_counter_http.load(Ordering::Relaxed));
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
                    // IPC Bridge - call Poly functions from JavaScript
                    let mut body = String::new();
                    request.as_reader().read_to_string(&mut body).ok();
                    
                    let result = handle_ipc_invoke(&entry_http, &body);
                    tiny_http::Response::from_string(result)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                        .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
                }
                "/__poly_run" => {
                    let output = execute_poly_for_web(&entry_http);
                    tiny_http::Response::from_string(output)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                }
                _ => {
                    // First try the exact path, then try in web/ folder
                    let url_path = url.trim_start_matches('/');
                    let file_path = project_path_http.join(url_path);
                    let web_file_path = project_path_http.join("web").join(url_path);
                    
                    let actual_path = if file_path.exists() && file_path.is_file() {
                        Some(file_path)
                    } else if web_file_path.exists() && web_file_path.is_file() {
                        Some(web_file_path)
                    } else {
                        None
                    };
                    
                    if let Some(path) = actual_path {
                        let content = fs::read_to_string(&path).unwrap_or_default();
                        let ct = match path.extension().and_then(|e| e.to_str()) {
                            Some("html") => "text/html; charset=utf-8",
                            Some("css") => "text/css; charset=utf-8",
                            Some("js") => "application/javascript; charset=utf-8",
                            Some("json") => "application/json",
                            Some("svg") => "image/svg+xml",
                            Some("png") => "image/png",
                            Some("jpg") | Some("jpeg") => "image/jpeg",
                            _ => "text/plain",
                        };
                        tiny_http::Response::from_string(content)
                            .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], ct.as_bytes()).unwrap())
                    } else {
                        tiny_http::Response::from_string("Not Found").with_status_code(404)
                    }
                }
            };
            let _ = request.respond(response);
        }
    });
    
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
        let _ = open_in_browser(&format!("http://localhost:{}", port));
    }
    
    println!();
    println!("  {}press h to show help{}", DIM, RESET);
    println!();
    
    // Watch loop
    loop {
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
                    
                    // Only run Poly interpreter for .poly file changes
                    let has_poly_change = event.paths.iter().any(|p| {
                        p.extension().and_then(|e| e.to_str()) == Some("poly")
                    });
                    
                    if has_poly_change {
                        let start = std::time::Instant::now();
                        match poly::run(&fs::read_to_string(&entry).unwrap_or_default()) {
                            Ok(_) => println!(" {}({}ms){}", DIM, start.elapsed().as_millis(), RESET),
                            Err(e) => println!("\n  {}error{}: {}", RED, RESET, e),
                        }
                    } else {
                        // For HTML/CSS/JS, just signal reload (no Poly execution needed)
                        println!("");
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
    
    // Build argument string from JSON (positional args)
    let args_str = if let Some(obj) = args.as_object() {
        obj.iter()
            .map(|(_, v)| json_to_poly_value(v))
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        String::new()
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

/// Handle built-in system APIs
fn handle_system_api(fn_name: &str, args: &serde_json::Value) -> String {
    match fn_name {
        // File Dialogs
        "__poly_dialog_open" => {
            let title = args.get("title").and_then(|v| v.as_str());
            let filters = parse_filters(args.get("filters"));
            
            #[cfg(feature = "native")]
            {
                let result = poly::native::dialog_open_file(title, filters);
                serde_json::json!({"result": result}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": null}).to_string()
            }
        }
        "__poly_dialog_open_multiple" => {
            let title = args.get("title").and_then(|v| v.as_str());
            let filters = parse_filters(args.get("filters"));
            
            #[cfg(feature = "native")]
            {
                let result = poly::native::dialog_open_files(title, filters);
                serde_json::json!({"result": result}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": []}).to_string()
            }
        }
        "__poly_dialog_save" => {
            let title = args.get("title").and_then(|v| v.as_str());
            let default_name = args.get("defaultName").and_then(|v| v.as_str());
            let filters = parse_filters(args.get("filters"));
            
            #[cfg(feature = "native")]
            {
                let result = poly::native::dialog_save_file(title, default_name, filters);
                serde_json::json!({"result": result}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": null}).to_string()
            }
        }
        "__poly_dialog_folder" => {
            let title = args.get("title").and_then(|v| v.as_str());
            
            #[cfg(feature = "native")]
            {
                let result = poly::native::dialog_pick_folder(title);
                serde_json::json!({"result": result}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": null}).to_string()
            }
        }
        "__poly_dialog_message" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Message");
            let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
            let level = args.get("level").and_then(|v| v.as_str()).unwrap_or("info");
            
            #[cfg(feature = "native")]
            {
                let msg_level = match level {
                    "warning" => poly::native::MessageLevel::Warning,
                    "error" => poly::native::MessageLevel::Error,
                    _ => poly::native::MessageLevel::Info,
                };
                poly::native::dialog_message(title, message, msg_level);
                serde_json::json!({"result": true}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": false}).to_string()
            }
        }
        "__poly_dialog_confirm" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Confirm");
            let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
            
            #[cfg(feature = "native")]
            {
                let result = poly::native::dialog_confirm(title, message);
                serde_json::json!({"result": result}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": false}).to_string()
            }
        }
        // File System
        "__poly_fs_read" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            match fs::read_to_string(path) {
                Ok(content) => serde_json::json!({"result": content}).to_string(),
                Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
            }
        }
        "__poly_fs_write" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            match fs::write(path, content) {
                Ok(_) => serde_json::json!({"result": true}).to_string(),
                Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
            }
        }
        "__poly_fs_exists" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let exists = Path::new(path).exists();
            serde_json::json!({"result": exists}).to_string()
        }
        "__poly_fs_read_dir" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            match fs::read_dir(path) {
                Ok(entries) => {
                    let files: Vec<serde_json::Value> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| {
                            let path = e.path();
                            serde_json::json!({
                                "name": e.file_name().to_string_lossy(),
                                "path": path.to_string_lossy(),
                                "isDir": path.is_dir(),
                            })
                        })
                        .collect();
                    serde_json::json!({"result": files}).to_string()
                }
                Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
            }
        }
        // Auto-Updater APIs
        "__poly_updater_check_github" => {
            let repo = args.get("repo").and_then(|v| v.as_str()).unwrap_or("");
            let current_version = args.get("currentVersion").and_then(|v| v.as_str()).unwrap_or("0.0.0");
            
            #[cfg(feature = "native")]
            {
                match poly::check_github_updates(repo, current_version) {
                    Ok(info) => serde_json::json!({"result": info}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_updater_check_url" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let current_version = args.get("currentVersion").and_then(|v| v.as_str()).unwrap_or("0.0.0");
            
            #[cfg(feature = "native")]
            {
                match poly::check_custom_updates(url, current_version) {
                    Ok(info) => serde_json::json!({"result": info}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_updater_download" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            
            #[cfg(feature = "native")]
            {
                match poly::download_update(url, None) {
                    Ok(path) => serde_json::json!({"result": path.to_string_lossy()}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_updater_install" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            
            #[cfg(feature = "native")]
            {
                match poly::install_update(Path::new(path)) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        _ => serde_json::json!({"error": format!("Unknown system API: {}", fn_name)}).to_string(),
    }
}

/// Parse filter array from JSON: [["Images", ["png", "jpg"]], ["All", ["*"]]]
fn parse_filters(_value: Option<&serde_json::Value>) -> Option<Vec<(&str, &[&str])>> {
    // For simplicity, we don't parse complex filters in this version
    // Users can pass simple filters or none
    None
}

/// Handle IPC when there's no Poly backend (system APIs only)
fn handle_ipc_invoke_system_only(body: &str) -> String {
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
            if (polling) setTimeout(check, 1000);
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
        if path_str.contains("target") || path_str.contains(".git") {
            return false;
        }
        
        p.extension().and_then(|e| e.to_str())
            .map(|ext| matches!(ext, "poly" | "html" | "css" | "js"))
            .unwrap_or(false)
    })
}

fn find_entry_point(project_path: &Path) -> Option<std::path::PathBuf> {
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


fn run_app_result(path: &str, release: bool, native: bool) -> Result<(), String> {
    let project_path = Path::new(path);
    
    // If native mode, try to run in a WebView window
    if native {
        run_native_app(project_path, release);
        return Ok(());
    }
    
    if project_path.is_file() {
        let start = std::time::Instant::now();
        let source = fs::read_to_string(project_path)
            .map_err(|e| format!("{}", e))?;
        poly::run(&source)?;
        println!("\n  {}done{} in {}ms", GREEN, RESET, start.elapsed().as_millis());
        return Ok(());
    }
    
    let entry = find_entry_point(project_path)
        .ok_or_else(|| "No entry point found".to_string())?;
    
    println!();
    println!("  {}POLY{} v0.1.5  {}{}{}", CYAN, RESET, DIM, if release { "release" } else { "debug" }, RESET);
    println!();
    
    let start = std::time::Instant::now();
    let source = fs::read_to_string(&entry)
        .map_err(|e| format!("{}", e))?;
    poly::run(&source)?;
    println!("\n  {}done{} in {}ms", GREEN, RESET, start.elapsed().as_millis());
    Ok(())
}

fn run_native_app(project_path: &Path, _release: bool) {
    use std::sync::Arc;
    use std::thread;
    
    // Find web directory
    let web_dir = if project_path.is_file() {
        project_path.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        let web_path = project_path.join("web");
        if web_path.exists() { web_path }
        else if project_path.join("index.html").exists() { project_path.to_path_buf() }
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
    
    let title = project_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Poly App");
    
    // Parse poly.toml for configuration
    let poly_toml_path = project_path.join("poly.toml");
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
    
    // Find a free port
    let port = 9473u16;
    
    println!();
    println!("  {}POLY{} v0.1.5  {}native{}", CYAN, RESET, DIM, RESET);
    println!();
    println!("  {}>{} Local server: http://localhost:{}", DIM, RESET, port);
    println!("  {}>{} Web dir: {}", DIM, RESET, web_dir.display());
    
    // Look for titlebar icon (SVG preferred for quality)
    let titlebar_icon_candidates = [
        project_path.join("assets/icon.svg"),
        project_path.join("assets/titlebar-icon.svg"),
        project_path.join("icon.svg"),
    ];
    let titlebar_icon_svg = titlebar_icon_candidates.iter()
        .find(|p| p.exists())
        .and_then(|p| fs::read_to_string(p).ok())
        .map(|svg| {
            // Clean up SVG for inline use (remove XML declaration, newlines)
            svg.lines()
                .filter(|l| !l.trim().starts_with("<?xml"))
                .collect::<Vec<_>>()
                .join("")
                .replace('\n', "")
                .replace('\r', "")
                .replace('\'', "\\'")
        });
    
    // Look for window icon file (PNG for taskbar/dock)
    let icon_candidates = [
        project_path.join("assets/icon.png"),
        project_path.join("icon.png"),
    ];
    let icon_path = icon_candidates.iter().find(|p| p.exists())
        .cloned()
        .or_else(|| {
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let candidates = [
                        exe_dir.join("assets/Polybarsmall@2x.png"),
                        exe_dir.join("../poly/assets/Polybarsmall@2x.png"),
                        exe_dir.join("../../poly/assets/Polybarsmall@2x.png"),
                    ];
                    for c in candidates {
                        if c.exists() { return Some(c); }
                    }
                }
            }
            None
        });
    
    let mut config = poly::NativeConfig::new(title)
        .with_size(1024, 768)
        .with_dev_tools(true)
        .with_tray(tray_enabled)
        .with_minimize_to_tray(minimize_to_tray)
        .with_close_to_tray(close_to_tray);
    
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
    
    println!();
    
    // Start local HTTP server in background thread
    let web_dir_arc = Arc::new(web_dir);
    let web_dir_server = Arc::clone(&web_dir_arc);
    let project_path_owned = project_path.to_path_buf();
    let entry_path = find_entry_point(&project_path_owned);
    let titlebar_icon_for_server = titlebar_icon_svg.clone();
    
    thread::spawn(move || {
        let server = tiny_http::Server::http(format!("127.0.0.1:{}", port))
            .expect("Failed to start local server");
        
        for mut request in server.incoming_requests() {
            let url = request.url().to_string();
            
            // Serve titlebar icon
            let response = if url == "/__poly_titlebar_icon" {
                let icon_svg = titlebar_icon_for_server.as_deref().unwrap_or("");
                tiny_http::Response::from_string(format!(r#"{{"icon":{}}}"#, 
                    if icon_svg.is_empty() { "null".to_string() } else { format!("\"{}\"", icon_svg.replace('"', "\\\"")) }
                ))
                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
            }
            // Handle IPC invoke
            else if url == "/__poly_invoke" {
                let mut body = String::new();
                request.as_reader().read_to_string(&mut body).ok();
                
                let result = if let Some(ref entry) = entry_path {
                    handle_ipc_invoke(entry, &body)
                } else {
                    // No entry point, only handle system APIs
                    handle_ipc_invoke_system_only(&body)
                };
                
                tiny_http::Response::from_string(result)
                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
            } else {
                let file_path = if url == "/" || url == "/index.html" {
                    web_dir_server.join("index.html")
                } else {
                    web_dir_server.join(url.trim_start_matches('/'))
                };
                
                if file_path.exists() && file_path.is_file() {
                    let content = fs::read(&file_path).unwrap_or_default();
                    let ct = match file_path.extension().and_then(|e| e.to_str()) {
                        Some("html") => "text/html; charset=utf-8",
                        Some("css") => "text/css; charset=utf-8",
                        Some("js") => "application/javascript; charset=utf-8",
                        Some("json") => "application/json",
                        Some("png") => "image/png",
                        Some("jpg") | Some("jpeg") => "image/jpeg",
                        Some("svg") => "image/svg+xml",
                        Some("woff") => "font/woff",
                        Some("woff2") => "font/woff2",
                        _ => "application/octet-stream",
                    };
                    
                    // Inject Alpine.js, Lucide Icons, and IPC Bridge into HTML files
                    let final_content = if ct.starts_with("text/html") {
                        let mut html = String::from_utf8_lossy(&content).to_string();
                        
                        // Inject scripts in <head>
                        let head_scripts = r#"<script defer src="https://unpkg.com/alpinejs@3/dist/cdn.min.js"></script>
<script src="https://unpkg.com/lucide@latest/dist/umd/lucide.min.js"></script>"#;
                        if html.contains("</head>") {
                            html = html.replace("</head>", &format!("{}</head>", head_scripts));
                        }
                        
                        // Inject IPC Bridge and Lucide initialization before </body>
                        let body_script = r##"<script>
// Poly Custom Titlebar System
(function() {
  // Check if running in native mode (wry injects window.ipc)
  // We also check for the port 9473 which is used by native mode
  const isNative = typeof window.ipc !== 'undefined' || window.location.port === '9473';
  
  if (isNative) {
    const style = document.createElement('style');
    style.textContent = `
      /* Custom Titlebar */
      .poly-titlebar {
        position: fixed; top: 0; left: 0; right: 0; height: 28px; z-index: 99998;
        background: #0f0f13;
        display: flex; align-items: center; justify-content: space-between;
        -webkit-app-region: drag; user-select: none; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
      }
      .poly-titlebar-left { display: flex; align-items: center; gap: 8px; padding-left: 12px; }
      .poly-titlebar-logo { width: 14px; height: 14px; }
      .poly-titlebar-title { font-size: 11px; font-weight: 500; color: rgba(255,255,255,0.5); }
      .poly-titlebar-controls { display: flex; height: 100%; -webkit-app-region: no-drag; }
      .poly-titlebar-btn {
        width: 40px; height: 100%; border: none; background: transparent;
        display: flex; align-items: center; justify-content: center;
        cursor: pointer; transition: background 0.1s;
      }
      .poly-titlebar-btn svg { width: 9px; height: 9px; stroke: rgba(255,255,255,0.5); stroke-width: 1.5; fill: none; }
      .poly-titlebar-btn:hover { background: rgba(255,255,255,0.1); }
      .poly-titlebar-btn:hover svg { stroke: #fff; }
      .poly-titlebar-btn.close:hover { background: #e81123; }
      .poly-titlebar-btn.close:hover svg { stroke: #fff; }
      /* macOS style traffic lights */
      .poly-titlebar-macos { padding-left: 10px; gap: 6px; display: flex; align-items: center; -webkit-app-region: no-drag; }
      .poly-titlebar-macos-btn {
        width: 11px; height: 11px; border-radius: 50%; border: none; cursor: pointer;
        display: flex; align-items: center; justify-content: center; transition: filter 0.1s;
      }
      .poly-titlebar-macos-btn svg { width: 5px; height: 5px; opacity: 0; transition: opacity 0.1s; }
      .poly-titlebar-macos:hover .poly-titlebar-macos-btn svg { opacity: 1; }
      .poly-titlebar-macos-btn.close { background: #ff5f57; }
      .poly-titlebar-macos-btn.close svg { stroke: #820005; stroke-width: 2; }
      .poly-titlebar-macos-btn.minimize { background: #febc2e; }
      .poly-titlebar-macos-btn.minimize svg { stroke: #9a6a00; stroke-width: 2; }
      .poly-titlebar-macos-btn.maximize { background: #28c840; }
      .poly-titlebar-macos-btn.maximize svg { stroke: #006500; stroke-width: 2; }
      .poly-titlebar-macos-btn:hover { filter: brightness(1.1); }
      /* Body padding for titlebar */
      body { padding-top: 28px !important; box-sizing: border-box; }
      html { overflow: hidden; }
      body { overflow: auto; height: calc(100vh - 28px); }
    `;
    document.head.appendChild(style);

    // Default Poly Logo SVG with all polygons and gradients
    const defaultLogo = '<svg viewBox="0 0 367 475" xmlns="http://www.w3.org/2000/svg"><defs><linearGradient id="a" x1="0" y1="237" x2="367" y2="237" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#1e6e95"/><stop offset="1" stop-color="#1971a2"/></linearGradient><linearGradient id="b" x1="301" y1="213" x2="314" y2="168" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#1e76a4"/><stop offset="1" stop-color="#2689af"/></linearGradient><linearGradient id="c" x1="280" y1="261" x2="367" y2="261" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#2696bb"/><stop offset="1" stop-color="#2689af"/></linearGradient><linearGradient id="d" x1="319" y1="165" x2="325" y2="2" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#62c2d0"/><stop offset="1" stop-color="#6ec5ce"/></linearGradient><linearGradient id="e" x1="269" y1="180" x2="282" y2="0" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#1cb3c9"/><stop offset="1" stop-color="#52beca"/></linearGradient><linearGradient id="f" x1="240" y1="144" x2="230" y2="4" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#288ca6"/><stop offset="1" stop-color="#3ebac5"/></linearGradient><linearGradient id="g" x1="181" y1="141" x2="41" y2="-33" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#16a3bf"/><stop offset="1" stop-color="#1f9eb8"/></linearGradient><linearGradient id="h" x1="-14" y1="13" x2="103" y2="139" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#5dc1d2"/><stop offset="1" stop-color="#19abc6"/></linearGradient><linearGradient id="i" x1="34" y1="194" x2="13" y2="-1" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#2892b1"/><stop offset="1" stop-color="#19abc6"/></linearGradient><linearGradient id="j" x1="119" y1="316" x2="95" y2="119" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#2786b9"/><stop offset="1" stop-color="#19abc6"/></linearGradient><linearGradient id="k" x1="96" y1="413" x2="82" y2="147" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#174e99"/><stop offset="1" stop-color="#1e80ad"/></linearGradient><linearGradient id="l" x1="44" y1="347" x2="30" y2="151" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#17428b"/><stop offset="1" stop-color="#176b9f"/></linearGradient><linearGradient id="m" x1="97" y1="443" x2="28" y2="371" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#26326e"/><stop offset="1" stop-color="#1c3e7d"/></linearGradient><linearGradient id="n" x1="31" y1="474" x2="36" y2="344" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#272f65"/><stop offset="1" stop-color="#24346b"/></linearGradient><linearGradient id="o" x1="29" y1="409" x2="14" y2="197" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#273169"/><stop offset="1" stop-color="#114b8a"/></linearGradient><linearGradient id="p" x1="129" y1="231" x2="213" y2="231" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#2180b6"/><stop offset="1" stop-color="#268db8"/></linearGradient><linearGradient id="q" x1="187" y1="251" x2="280" y2="251" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#278cb9"/><stop offset="1" stop-color="#268ab7"/></linearGradient><linearGradient id="r" x1="105" y1="294" x2="158" y2="231" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#213567"/><stop offset="1" stop-color="#1d5f95"/></linearGradient><linearGradient id="s" x1="327" y1="210" x2="383" y2="95" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#3abacb"/><stop offset="1" stop-color="#54bfce"/></linearGradient><linearGradient id="t" x1="190" y1="344" x2="225" y2="273" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#124b94"/><stop offset="1" stop-color="#1e619d"/></linearGradient><linearGradient id="u" x1="0" y1="30" x2="280" y2="30" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#5dc1d2"/><stop offset="1" stop-color="#6cc5d1"/></linearGradient></defs><path fill="url(#a)" d="M328 267l39-38V86L280 0H0v411l64 64 65-64V115l84 1 29 29v33l-29 29H130v107h151l48-47z"/><polygon fill="url(#b)" points="300 164 242 178 284 208 367 229 300 164"/><polygon fill="url(#c)" points="280 314 284 208 367 229 280 314"/><polygon fill="url(#d)" points="280 0 300 164 367 86 280 0"/><polygon fill="url(#e)" points="242 144 280 0 300 164 242 178 242 144"/><polygon fill="url(#f)" points="213 115 183 60 280 0 242 144 213 115"/><polygon fill="url(#g)" points="213 115 129 115 0 0 183 60 213 115"/><polygon fill="url(#h)" points="129 115 63 148 0 0 129 115"/><polygon fill="url(#i)" points="63 148 0 198 0 0 63 148"/><polygon fill="url(#j)" points="129 115 63 148 129 314 129 115"/><polygon fill="url(#k)" points="63 148 53 347 129 411 129 314 63 148"/><polygon fill="url(#l)" points="0 198 53 347 63 148 0 198"/><polygon fill="url(#m)" points="64 475 53 347 129 411 64 475"/><polygon fill="url(#n)" points="53 347 0 411 64 475 53 347"/><polygon fill="url(#o)" points="0 411 53 347 0 198 0 411"/><polygon fill="url(#p)" points="129 207 187 255 213 207 129 207"/><polygon fill="url(#q)" points="280 314 187 255 213 207 233 187 280 314"/><polygon fill="url(#r)" points="129 207 129 314 187 255 129 207"/><polygon fill="url(#s)" points="300 164 367 229 367 86 300 164"/><polygon fill="url(#t)" points="129 314 187 255 280 314 129 314"/><polygon fill="url(#u)" points="280 0 183 60 0 0 280 0"/></svg>';

    // Detect platform (Windows vs macOS)
    const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
    
    const titlebar = document.createElement('div');
    titlebar.className = 'poly-titlebar';
    
    // Helper function to send IPC message
    const sendIPC = (msg) => {
      if (window.ipc && window.ipc.postMessage) {
        window.ipc.postMessage(msg);
      }
    };
    
    // Build titlebar HTML
    const buildTitlebar = (logoSvg) => {
      if (isMac) {
        titlebar.innerHTML = `
          <div class="poly-titlebar-macos">
            <button class="poly-titlebar-macos-btn close" data-action="close">
              <svg viewBox="0 0 10 10"><path d="M1 1l8 8M9 1l-8 8"/></svg>
            </button>
            <button class="poly-titlebar-macos-btn minimize" data-action="minimize">
              <svg viewBox="0 0 10 10"><path d="M1 5h8"/></svg>
            </button>
            <button class="poly-titlebar-macos-btn maximize" data-action="maximize">
              <svg viewBox="0 0 10 10"><path d="M1 1h8v8H1z"/></svg>
            </button>
          </div>
          <div class="poly-titlebar-left" style="padding-left: 8px;">
            <div class="poly-titlebar-logo">${logoSvg}</div>
            <span class="poly-titlebar-title">${document.title || 'Poly App'}</span>
          </div>
          <div style="width: 60px;"></div>
        `;
      } else {
        titlebar.innerHTML = `
          <div class="poly-titlebar-left">
            <div class="poly-titlebar-logo">${logoSvg}</div>
            <span class="poly-titlebar-title">${document.title || 'Poly App'}</span>
          </div>
          <div class="poly-titlebar-controls">
            <button class="poly-titlebar-btn" data-action="minimize">
              <svg viewBox="0 0 10 10"><path d="M0 5h10"/></svg>
            </button>
            <button class="poly-titlebar-btn" data-action="maximize">
              <svg viewBox="0 0 10 10"><rect x="0" y="0" width="10" height="10" rx="1"/></svg>
            </button>
            <button class="poly-titlebar-btn close" data-action="close">
              <svg viewBox="0 0 10 10"><path d="M0 0l10 10M10 0l-10 10"/></svg>
            </button>
          </div>
        `;
      }
      
      // Add click handlers
      titlebar.querySelectorAll('[data-action]').forEach(btn => {
        btn.addEventListener('click', (e) => {
          e.stopPropagation();
          sendIPC(btn.dataset.action);
        });
      });
      
      // Enable window dragging
      titlebar.addEventListener('mousedown', (e) => {
        if (!e.target.closest('[data-action]') && !e.target.closest('.poly-titlebar-macos')) {
          sendIPC('drag');
        }
      });
    };
    
    // Try to load custom icon from assets/icon.svg
    fetch('/__poly_titlebar_icon')
      .then(r => r.json())
      .then(data => {
        buildTitlebar(data.icon || defaultLogo);
      })
      .catch(() => {
        buildTitlebar(defaultLogo);
      });
    
    // Insert titlebar when DOM is ready
    if (document.body) {
      document.body.insertBefore(titlebar, document.body.firstChild);
    } else {
      document.addEventListener('DOMContentLoaded', () => {
        document.body.insertBefore(titlebar, document.body.firstChild);
      });
    }
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
    return d.result;
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
    show() { if (window.ipc) window.ipc.postMessage('show'); }
  }
};
// Initialize Lucide Icons
if (typeof lucide !== 'undefined') {
  lucide.createIcons();
  document.addEventListener('alpine:initialized', () => lucide.createIcons());
}
</script>"##;
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
    
    // Give server time to start
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Run native window with URL instead of HTML content
    let url = format!("http://localhost:{}", port);
    if let Err(e) = poly::run_native_url(&url, config) {
        eprintln!("{}error{}: {}", RED, RESET, e);
        eprintln!();
        eprintln!("  {}hint{}: Build with native feature enabled:", DIM, RESET);
        eprintln!("    cargo build -p poly --features native");
        std::process::exit(1);
    }
}

fn build_app(path: &str, target: &str, release: bool) {
    let project_path = Path::new(path);
    
    println!();
    println!("  {}POLY{} v0.1.5  {}build{}", CYAN, RESET, DIM, RESET);
    println!();
    println!("  {}>{} Target:  {}", DIM, RESET, target);
    println!("  {}>{} Mode:    {}", DIM, RESET, if release { "release" } else { "debug" });
    println!();
    
    let start = std::time::Instant::now();
    
    match target {
        "web" | "wasm" => build_web(project_path, release),
        "native" => build_native(project_path, release),
        "all" => { build_native(project_path, release); build_web(project_path, release); }
        _ => { eprintln!("{}error{}: Unknown target '{}'. Use: native, web, all", RED, RESET, target); std::process::exit(1); }
    }
    
    println!();
    println!("  {}done{} in {}ms", GREEN, RESET, start.elapsed().as_millis());
}

fn build_web(project_path: &Path, _release: bool) {
    let dist = project_path.join("dist/web");
    fs::create_dir_all(&dist).ok();
    
    let web_dir = project_path.join("web");
    if web_dir.exists() { copy_dir_recursive(&web_dir, &dist).ok(); }
    
    let index = dist.join("index.html");
    if !index.exists() {
        let name = project_path.file_name().and_then(|n| n.to_str()).unwrap_or("App");
        fs::write(&index, format!(r#"<!DOCTYPE html><html><head><meta charset="UTF-8"><title>{}</title></head><body><div id="app"></div></body></html>"#, name)).ok();
    }
    
    println!("  {}>{} dist/web/index.html", GREEN, RESET);
}

fn build_native(project_path: &Path, _release: bool) {
    let dist = project_path.join("dist/native");
    fs::create_dir_all(&dist).ok();
    
    if let Some(entry) = find_entry_point(project_path) {
        let dest = dist.join("app.poly");
        fs::copy(&entry, &dest).ok();
        println!("  {}>{} dist/native/app.poly", GREEN, RESET);
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dest = dst.join(entry.file_name());
        if entry.path().is_dir() { copy_dir_recursive(&entry.path(), &dest)?; }
        else { fs::copy(&entry.path(), &dest)?; }
    }
    Ok(())
}


fn create_project(name: &str, template: &str) {
    println!();
    println!("  {}POLY{} v0.1.5", CYAN, RESET);
    println!();
    
    let project_path = Path::new(name);
    if project_path.exists() {
        eprintln!("{}error{}: Directory '{}' already exists", RED, RESET, name);
        std::process::exit(1);
    }
    
    print!("  Creating project {}{}{}...", BOLD, name, RESET);
    io::stdout().flush().unwrap();
    
    fs::create_dir_all(project_path.join("src")).expect("Failed to create directory");
    fs::create_dir_all(project_path.join("web")).expect("Failed to create directory");
    fs::create_dir_all(project_path.join("assets")).expect("Failed to create directory");
    
    // Copy default Poly icon if available
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let icon_candidates = [
                exe_dir.join("assets/Polybarsmall@2x.png"),
                exe_dir.join("../poly/assets/Polybarsmall@2x.png"),
                exe_dir.join("../../poly/assets/Polybarsmall@2x.png"),
            ];
            for src in icon_candidates {
                if src.exists() {
                    let _ = fs::copy(&src, project_path.join("assets/icon.png"));
                    break;
                }
            }
        }
    }
    
    // poly.toml - entry is now optional (for backend logic only)
    fs::write(project_path.join("poly.toml"), format!(r#"[package]
name = "{}"
version = "0.1.0"

[web]
dir = "web"

[window]
width = 1024
height = 768
resizable = true

# System Tray (optional)
# [tray]
# enabled = true
# tooltip = "My App"
# minimize_to_tray = false
# close_to_tray = false
"#, name)).ok();
    
    // Direct HTML/CSS/JS files (Tauri/Electron style - edit directly, hot reload works)
    match template {
        "app" => {
            // Create web/index.html directly
            fs::write(project_path.join("web/index.html"), format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{name}</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <div class="app" x-data="app()">
    <div class="container">
      <!-- Logo -->
      <div class="logo-wrapper">
        <svg viewBox="0 0 366.77 474.9" class="logo">
          <defs>
            <linearGradient id="a" x1="0" y1="237.45" x2="366.77" y2="237.45" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#1e6e95"/><stop offset="1" stop-color="#1971a2"/></linearGradient>
            <linearGradient id="b" x1="301.44" y1="213.46" x2="314.35" y2="168.43" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#1e76a4"/><stop offset="1" stop-color="#2689af"/></linearGradient>
            <linearGradient id="c" x1="280.28" y1="261.32" x2="366.77" y2="261.32" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#2696bb"/><stop offset="1" stop-color="#2689af"/></linearGradient>
            <linearGradient id="d" x1="319.27" y1="165" x2="324.98" y2="1.56" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#62c2d0"/><stop offset="1" stop-color="#6ec5ce"/></linearGradient>
            <linearGradient id="e" x1="269.17" y1="179.74" x2="281.73" y2=".1" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#1cb3c9"/><stop offset="1" stop-color="#52beca"/></linearGradient>
            <linearGradient id="f" x1="239.82" y1="144.42" x2="229.97" y2="3.52" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#288ca6"/><stop offset="1" stop-color="#3ebac5"/></linearGradient>
            <linearGradient id="g" x1="181.37" y1="140.88" x2="40.64" y2="-32.91" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#16a3bf"/><stop offset="1" stop-color="#1f9eb8"/></linearGradient>
            <linearGradient id="h" x1="-14.29" y1="13.32" x2="103.18" y2="139.29" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#5dc1d2"/><stop offset="1" stop-color="#19abc6"/></linearGradient>
            <linearGradient id="i" x1="33.68" y1="193.98" x2="13.15" y2="-1.38" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#2892b1"/><stop offset="1" stop-color="#19abc6"/></linearGradient>
            <linearGradient id="j" x1="118.66" y1="315.53" x2="94.56" y2="119.26" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#2786b9"/><stop offset="1" stop-color="#19abc6"/></linearGradient>
            <linearGradient id="k" x1="95.67" y1="412.92" x2="81.75" y2="147.37" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#174e99"/><stop offset="1" stop-color="#1e80ad"/></linearGradient>
            <linearGradient id="l" x1="43.67" y1="347.32" x2="29.91" y2="150.67" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#17428b"/><stop offset="1" stop-color="#176b9f"/></linearGradient>
            <linearGradient id="m" x1="96.99" y1="443.24" x2="27.66" y2="371.45" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#26326e"/><stop offset="1" stop-color="#1c3e7d"/></linearGradient>
            <linearGradient id="n" x1="31.03" y1="473.74" x2="35.55" y2="344.11" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#272f65"/><stop offset="1" stop-color="#24346b"/></linearGradient>
            <linearGradient id="o" x1="28.77" y1="408.67" x2="13.94" y2="196.55" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#273169"/><stop offset="1" stop-color="#114b8a"/></linearGradient>
            <linearGradient id="p" x1="129.22" y1="230.68" x2="212.86" y2="230.68" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#2180b6"/><stop offset="1" stop-color="#268db8"/></linearGradient>
            <linearGradient id="q" x1="186.81" y1="250.64" x2="280.28" y2="250.64" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#278cb9"/><stop offset="1" stop-color="#268ab7"/></linearGradient>
            <linearGradient id="r" x1="105.04" y1="293.82" x2="158.16" y2="230.51" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#213567"/><stop offset="1" stop-color="#1d5f95"/></linearGradient>
            <linearGradient id="s" x1="327.06" y1="209.69" x2="383.23" y2="94.53" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#3abacb"/><stop offset="1" stop-color="#54bfce"/></linearGradient>
            <linearGradient id="t" x1="190.32" y1="343.95" x2="224.87" y2="273.11" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#124b94"/><stop offset="1" stop-color="#1e619d"/></linearGradient>
            <linearGradient id="u" x1="0" y1="30.13" x2="280.28" y2="30.13" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#5dc1d2"/><stop offset="1" stop-color="#6cc5d1"/></linearGradient>
          </defs>
          <path fill="url(#a)" d="M328.43,266.8c12.78-12.58,25.56-25.16,38.33-37.74V86.49C337.94,57.66,309.11,28.83,280.28,0H0v410.68c21.4,21.4,42.81,42.81,64.21,64.21,21.71-21.25,43.43-42.5,65.14-63.74-.04-98.72-.09-197.43-.13-296.15,27.9.11,55.79.22,83.69.33l28.94,28.94v33.55c-3.08,3.08-6.16,6.16-9.24,9.23-6.59,6.58-13.17,13.17-19.76,19.75h-83.64c.04,35.8.09,71.6.13,107.4h150.93c16.05-15.8,32.1-31.61,48.15-47.41Z"/>
          <polygon fill="url(#b)" points="300.04 164.32 241.85 177.83 283.88 208.42 366.77 229.06 300.04 164.32"/>
          <polygon fill="url(#c)" points="280.28 314.21 283.88 208.42 366.77 229.06 280.28 314.21"/>
          <polygon fill="url(#d)" points="280.28 0 300.04 164.32 366.77 86.49 280.28 0"/>
          <polygon fill="url(#e)" points="241.85 144.28 280.28 0 300.04 164.32 241.85 177.83 241.85 144.28"/>
          <polygon fill="url(#f)" points="212.91 115.34 183.38 60.26 280.28 0 241.85 144.28 212.91 115.34"/>
          <polygon fill="url(#g)" points="212.91 115.34 129.22 115 0 0 183.38 60.26 212.91 115.34"/>
          <polygon fill="url(#h)" points="129.22 115 62.94 148.36 0 0 129.22 115"/>
          <polygon fill="url(#i)" points="62.94 148.36 0 197.52 0 0 62.94 148.36"/>
          <polygon fill="url(#j)" points="129.22 115 62.94 148.36 129.35 314.21 129.22 115"/>
          <polygon fill="url(#k)" points="62.94 148.36 53.34 346.65 129.35 411.15 129.35 314.21 62.94 148.36"/>
          <polygon fill="url(#l)" points="0 197.52 53.34 346.65 62.94 148.36 0 197.52"/>
          <polygon fill="url(#m)" points="64.21 474.9 53.34 346.65 129.35 411.15 64.21 474.9"/>
          <polygon fill="url(#n)" points="53.34 346.65 0 410.68 64.21 474.9 53.34 346.65"/>
          <polygon fill="url(#o)" points="0 410.68 53.34 346.65 0 197.52 0 410.68"/>
          <polygon fill="url(#p)" points="129.22 206.81 186.81 254.55 212.86 206.81 129.22 206.81"/>
          <polygon fill="url(#q)" points="280.28 314.21 186.81 254.55 212.86 206.81 232.62 187.06 280.28 314.21"/>
          <polygon fill="url(#r)" points="129.22 206.81 129.35 314.21 186.81 254.55 129.22 206.81"/>
          <polygon fill="url(#s)" points="300.04 164.32 366.77 229.06 366.77 86.49 300.04 164.32"/>
          <polygon fill="url(#t)" points="129.35 314.21 186.81 254.55 280.28 314.21 129.35 314.21"/>
          <polygon fill="url(#u)" points="280.28 0 183.38 60.26 0 0 280.28 0"/>
        </svg>
      </div>

      <!-- Welcome -->
      <h1>Welcome to <span class="gradient">{name}</span></h1>
      <p class="subtitle">Your Poly app is ready. Start building something amazing.</p>

      <!-- Features -->
      <div class="features">
        <div class="feature-card">
          <div class="feature-icon"><i data-lucide="zap"></i></div>
          <h3>Hot Reload</h3>
          <p>Changes appear instantly</p>
        </div>
        <div class="feature-card">
          <div class="feature-icon"><i data-lucide="cpu"></i></div>
          <h3>IPC Bridge</h3>
          <p>Call Poly from JavaScript</p>
        </div>
        <div class="feature-card">
          <div class="feature-icon"><i data-lucide="package"></i></div>
          <h3>Native Build</h3>
          <p>Desktop apps with WebView</p>
        </div>
      </div>

      <!-- Demo -->
      <div class="demo-section">
        <h2>Try it out</h2>
        <div class="demo-card">
          <div class="demo-row">
            <button class="btn btn-primary" @click="count++">
              <i data-lucide="plus"></i>
              Count: <span x-text="count"></span>
            </button>
            <button class="btn btn-secondary" @click="count = 0">Reset</button>
          </div>
          <div class="demo-row">
            <input type="text" x-model="name" placeholder="Enter your name..." class="input">
          </div>
          <p class="greeting" x-show="name.length > 0" x-transition>
            Hello, <span x-text="name"></span>! 👋
          </p>
        </div>
      </div>

      <!-- Quick Start -->
      <div class="quickstart">
        <h2>Quick Start</h2>
        <div class="code-block">
          <code>web/index.html</code> - Your app's HTML<br>
          <code>web/styles.css</code> - Your styles<br>
          <code>web/app.js</code> - Your JavaScript<br>
          <code>src/main.poly</code> - Backend logic (optional)
        </div>
      </div>

      <footer>Built with <span class="heart">♥</span> using Poly</footer>
    </div>
  </div>
  <script src="app.js"></script>
</body>
</html>"##, name = name)).ok();

            // Create web/styles.css
            fs::write(project_path.join("web/styles.css"), r#"* { margin: 0; padding: 0; box-sizing: border-box; }

:root {
  --bg: #0a0a0f;
  --surface: rgba(255, 255, 255, 0.03);
  --border: rgba(255, 255, 255, 0.08);
  --text: #fafafa;
  --text-muted: #888;
  --text-dim: #555;
  --accent: #5dc1d2;
  --accent-dark: #1e80ad;
}

body {
  font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  background: var(--bg);
  color: var(--text);
  min-height: 100vh;
  line-height: 1.6;
}

.app {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
}

.container { max-width: 720px; text-align: center; }

/* Logo */
.logo-wrapper { margin-bottom: 2rem; }

.logo {
  width: 80px;
  height: auto;
  filter: drop-shadow(0 20px 40px rgba(93, 193, 210, 0.2));
  animation: float 4s ease-in-out infinite;
}

@keyframes float {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-8px); }
}

/* Typography */
h1 { font-size: 2rem; font-weight: 600; margin-bottom: 0.5rem; }

.gradient {
  background: linear-gradient(135deg, var(--accent) 0%, var(--accent-dark) 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.subtitle { color: var(--text-muted); font-size: 1rem; margin-bottom: 3rem; }

h2 {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-dim);
  margin-bottom: 1rem;
}

/* Feature Cards */
.features {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1rem;
  margin-bottom: 3rem;
}

.feature-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 1.5rem 1rem;
  transition: all 0.2s;
}

.feature-card:hover {
  border-color: rgba(93, 193, 210, 0.3);
  transform: translateY(-2px);
}

.feature-icon {
  width: 40px;
  height: 40px;
  background: linear-gradient(135deg, var(--accent) 0%, var(--accent-dark) 100%);
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 0.75rem;
}

.feature-icon i, .feature-icon svg { width: 20px; height: 20px; color: white; }

.feature-card h3 { font-size: 0.9rem; font-weight: 600; margin-bottom: 0.25rem; }
.feature-card p { font-size: 0.8rem; color: var(--text-muted); }

/* Demo Section */
.demo-section { margin-bottom: 3rem; }

.demo-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 1.5rem;
}

.demo-row {
  display: flex;
  gap: 0.75rem;
  justify-content: center;
  margin-bottom: 1rem;
}

.demo-row:last-child { margin-bottom: 0; }

/* Buttons */
.btn {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.6rem 1.2rem;
  border: none;
  border-radius: 8px;
  font-size: 0.85rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn i, .btn svg { width: 16px; height: 16px; }

.btn-primary {
  background: linear-gradient(135deg, var(--accent) 0%, var(--accent-dark) 100%);
  color: white;
}

.btn-primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(93, 193, 210, 0.3);
}

.btn-secondary {
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text-muted);
}

.btn-secondary:hover {
  background: var(--surface);
  border-color: rgba(255, 255, 255, 0.2);
  color: var(--text);
}

/* Input */
.input {
  flex: 1;
  max-width: 250px;
  padding: 0.6rem 1rem;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text);
  font-size: 0.85rem;
  outline: none;
  transition: all 0.2s;
}

.input:focus {
  border-color: rgba(93, 193, 210, 0.5);
  background: rgba(255, 255, 255, 0.08);
}

.input::placeholder { color: var(--text-dim); }

.greeting { color: var(--accent); font-size: 1rem; margin-top: 0.5rem; }

/* Quick Start */
.quickstart { margin-bottom: 3rem; }

.code-block {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 1.25rem;
  text-align: left;
  font-size: 0.85rem;
  line-height: 1.8;
  color: var(--text-muted);
}

.code-block code {
  background: rgba(93, 193, 210, 0.15);
  color: var(--accent);
  padding: 0.15rem 0.4rem;
  border-radius: 4px;
  font-family: 'SF Mono', Monaco, 'Cascadia Code', monospace;
  font-size: 0.8rem;
}

/* Footer */
footer { color: var(--text-dim); font-size: 0.8rem; }
.heart { color: #ef4444; }

/* Responsive */
@media (max-width: 600px) {
  .features { grid-template-columns: 1fr; }
  h1 { font-size: 1.5rem; }
}
"#).ok();

            // Create web/app.js
            fs::write(project_path.join("web/app.js"), r#"// App State
function app() {
  return {
    count: 0,
    name: '',
    
    // Example: Call Poly backend
    async callBackend() {
      try {
        const result = await poly.invoke('greet', { name: this.name || 'World' });
        console.log('Backend says:', result);
      } catch (e) {
        console.log('Backend not available');
      }
    }
  };
}

console.log('App loaded - Edit web/ files to customize');
"#).ok();

            // src/main.poly is now optional - for backend logic only
            fs::write(project_path.join("src/main.poly"), format!(r#"# {} - Backend Logic (optional)
# This file is for server-side logic, APIs, etc.
# Your frontend is in web/index.html, web/styles.css, web/app.js

print("Backend ready")
"#, name)).ok();
        },
        "lib" => {
            fs::write(project_path.join("src/main.poly"), format!(r#"# {} - A Poly Library

fn greet(name="World"):
    return f"Hello, {{name}}!"

fn add(a, b):
    return a + b
"#, name)).ok();
        },
        _ => { eprintln!("{}error{}: Unknown template '{}'", RED, RESET, template); std::process::exit(1); }
    };
    
    // .gitignore - don't ignore HTML files (they're the source now!)
    fs::write(project_path.join(".gitignore"), "/dist\n/target\n").ok();
    
    println!(" {}done{}", GREEN, RESET);
    println!();
    println!("  {}Next steps:{}", DIM, RESET);
    println!("    cd {}", name);
    println!("    poly dev");
    println!();
    println!("  {}Edit web/index.html directly - hot reload is automatic{}", DIM, RESET);
    println!();
}

fn init_project(_template: &str) {
    println!();
    println!("  {}POLY{} v0.1.5", CYAN, RESET);
    println!();
    
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let name = cwd.file_name().and_then(|n| n.to_str()).unwrap_or("app");
    
    fs::create_dir_all("src").ok();
    fs::create_dir_all("web").ok();
    fs::create_dir_all("assets").ok();
    
    if !Path::new("poly.toml").exists() {
        fs::write("poly.toml", format!(r#"[package]
name = "{}"
version = "0.1.0"

[web]
dir = "web"
"#, name)).ok();
    }
    
    // Create web/index.html directly (Tauri/Electron style)
    if !Path::new("web/index.html").exists() {
        fs::write("web/index.html", format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{name}</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <div class="container">
    <h1>Welcome to <span>{name}</span></h1>
    <p>Edit <code>web/index.html</code> to get started</p>
  </div>
  <script src="app.js"></script>
</body>
</html>"##, name = name)).ok();
    }
    
    if !Path::new("web/styles.css").exists() {
        fs::write("web/styles.css", r#"* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: system-ui, -apple-system, sans-serif;
  background: linear-gradient(135deg, #0a0a0a 0%, #0f1419 100%);
  color: #fafafa;
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
}

.container {
  text-align: center;
  padding: 2rem;
}

h1 {
  font-size: 1.5rem;
  font-weight: 500;
  color: #888;
  margin-bottom: 1rem;
}

h1 span {
  background: linear-gradient(135deg, #5dc1d2 0%, #1e80ad 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

p { color: #666; font-size: 0.9rem; }
code { background: rgba(255,255,255,0.1); padding: 0.2rem 0.5rem; border-radius: 4px; }
"#).ok();
    }
    
    if !Path::new("web/app.js").exists() {
        fs::write("web/app.js", "// Your app logic goes here\nconsole.log('App loaded');\n").ok();
    }
    
    // src/main.poly is optional - for backend logic only
    if !Path::new("src/main.poly").exists() {
        fs::write("src/main.poly", format!(r#"# {} - Backend Logic (optional)
# Your frontend is in web/index.html, web/styles.css, web/app.js

print("Backend ready")
"#, name)).ok();
    }
    
    println!("  {}Initialized{} Poly project", GREEN, RESET);
    println!();
    println!("  {}Next:{} poly dev", DIM, RESET);
    println!("  {}Edit web/index.html directly - hot reload is automatic{}", DIM, RESET);
    println!();
}

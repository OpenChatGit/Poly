use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Write, BufRead};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use serde_json;

mod packages;
mod build;

// ANSI color codes for terminal output
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

const VERSION: &str = "0.3.2";
#[allow(dead_code)]
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
        
        /// Browser mode: UI WebView + Content WebView (for browser-like apps)
        #[arg(long)]
        browser: bool,
        
        /// UI height in pixels for browser mode (default: 80)
        #[arg(long, default_value = "80")]
        ui_height: u32,
    },
    
    /// Build the application
    Build {
        /// Project directory
        #[arg(default_value = ".")]
        path: String,
        
        /// Target platform (windows, macos, linux, current)
        #[arg(short, long, default_value = "current")]
        target: String,
        
        /// Release build (optimized)
        #[arg(long)]
        release: bool,
        
        /// Create installer/package
        #[arg(long)]
        installer: bool,
        
        /// Sign the executable (requires certificate)
        #[arg(long)]
        sign: bool,
        
        /// Generate GitHub Actions CI workflow
        #[arg(long)]
        ci: bool,
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
    
    /// Add a JavaScript package from npm/CDN
    Add {
        /// Package name (e.g., alpinejs, chart.js, three)
        package: String,
        
        /// Specific version (default: latest)
        #[arg(short, long)]
        version: Option<String>,
    },
    
    /// Remove a JavaScript package
    Remove {
        /// Package name to remove
        package: String,
    },
    
    /// Install packages from poly.lock (reproducible builds)
    Install {
        /// Verify hashes without reinstalling
        #[arg(long)]
        verify: bool,
    },
    
    /// Open a URL in a new Poly WebView window (internal use)
    #[command(hide = true)]
    OpenUrl {
        /// URL to open
        url: String,
        
        /// Window title
        #[arg(long, default_value = "Poly Browser")]
        title: String,
        
        /// Window width
        #[arg(long, default_value = "1024")]
        width: u32,
        
        /// Window height
        #[arg(long, default_value = "768")]
        height: u32,
    },
    
    /// Open browser mode with separate UI and content WebViews (internal use)
    #[command(hide = true)]
    Browser {
        /// Start URL
        #[arg(default_value = "about:blank")]
        url: String,
        
        /// Window title
        #[arg(long, default_value = "Poly Browser")]
        title: String,
        
        /// Window width
        #[arg(long, default_value = "1024")]
        width: u32,
        
        /// Window height
        #[arg(long, default_value = "768")]
        height: u32,
        
        /// UI height (titlebar + nav bar)
        #[arg(long, default_value = "80")]
        ui_height: u32,
        
        /// Path to UI HTML file
        #[arg(long)]
        ui_html: Option<String>,
    },
}

/// Cookie storage for PolyView proxy
use std::sync::Mutex;
lazy_static::lazy_static! {
    static ref POLYVIEW_COOKIES: Mutex<std::collections::HashMap<String, Vec<String>>> = Mutex::new(std::collections::HashMap::new());
}

/// Handle PolyView proxy request - fetches URL and rewrites content to bypass iframe restrictions
/// This is "iframe2" - better than normal iframes because it bypasses ALL restrictions
fn handle_polyview_proxy(target_url: &str, proxy_port: u16) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    
    
    println!("[PolyView] Proxying: {}", target_url);
    
    // Parse URL to get domain for cookies
    let domain = url::Url::parse(target_url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()))
        .unwrap_or_default();
    
    // Build a client that looks like a real browser
    // Key: Don't follow redirects automatically - we need to rewrite redirect URLs
    let client = ureq::AgentBuilder::new()
        .redirects(0) // Handle redirects manually to rewrite them
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build();
    
    // Get stored cookies for this domain
    let cookie_header = {
        let cookies = POLYVIEW_COOKIES.lock().unwrap();
        if let Some(domain_cookies) = cookies.get(&domain) {
            domain_cookies.join("; ")
        } else {
            String::new()
        }
    };
    
    let mut req = client.get(target_url)
        .set("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8")
        .set("Accept-Language", "en-US,en;q=0.9,de;q=0.8")
        .set("Accept-Encoding", "identity") // Don't request compression
        .set("Cache-Control", "no-cache")
        .set("Pragma", "no-cache")
        .set("Sec-Ch-Ua", "\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\", \"Google Chrome\";v=\"120\"")
        .set("Sec-Ch-Ua-Mobile", "?0")
        .set("Sec-Ch-Ua-Platform", "\"Windows\"")
        .set("Sec-Fetch-Dest", "document")
        .set("Sec-Fetch-Mode", "navigate")
        .set("Sec-Fetch-Site", "none")
        .set("Sec-Fetch-User", "?1")
        .set("Upgrade-Insecure-Requests", "1");
    
    // Add cookies if we have them
    if !cookie_header.is_empty() {
        req = req.set("Cookie", &cookie_header);
    }
    
    match req.call() {
        Ok(response) => {
            process_polyview_response(response, target_url, proxy_port, &domain, 200)
        }
        Err(ureq::Error::Status(code, response)) => {
            // Handle redirects (3xx) and error responses
            if (300..400).contains(&code) {
                // Handle redirect - rewrite Location header
                if let Some(location) = response.header("Location") {
                    let absolute_url = resolve_polyview_url(target_url, location);
                    let proxied_location = format!(
                        "http://localhost:{}/__polyview/?url={}",
                        proxy_port,
                        urlencoding::encode(&absolute_url)
                    );
                    
                    // Store any cookies from redirect response
                    store_polyview_cookies(&response, &domain);
                    
                    return tiny_http::Response::from_string("")
                        .with_status_code(code)
                        .with_header(tiny_http::Header::from_bytes(&b"Location"[..], proxied_location.as_bytes()).unwrap())
                        .with_header(tiny_http::Header::from_bytes(&b"X-Frame-Options"[..], &b"ALLOWALL"[..]).unwrap())
                        .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap());
                }
            }
            process_polyview_response(response, target_url, proxy_port, &domain, code)
        }
        Err(e) => {
            println!("[PolyView] Error: {}", e);
            let error_html = format!(
                r#"<!DOCTYPE html>
                <html>
                <head><title>Error</title></head>
                <body style="font-family:system-ui;padding:40px;background:#1a1a1f;color:#fff">
                <h1>Failed to load page</h1>
                <p style="color:#888">{}</p>
                <p style="color:#666;font-size:12px">{}</p>
                </body>
                </html>"#,
                target_url, e
            );
            tiny_http::Response::from_string(error_html)
                .with_status_code(502)
                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap())
        }
    }
}

/// Store cookies from response
fn store_polyview_cookies(response: &ureq::Response, domain: &str) {
    let mut cookies = POLYVIEW_COOKIES.lock().unwrap();
    let entry = cookies.entry(domain.to_string()).or_insert_with(Vec::new);
    
    // Get all Set-Cookie headers
    for name in response.headers_names() {
        if name.eq_ignore_ascii_case("set-cookie") {
            if let Some(value) = response.header(&name) {
                // Extract just the cookie name=value part (before ;)
                let cookie = value.split(';').next().unwrap_or(value).to_string();
                let cookie_name = cookie.split('=').next().unwrap_or("");
                
                // Update or add cookie
                entry.retain(|c| !c.starts_with(&format!("{}=", cookie_name)));
                entry.push(cookie);
            }
        }
    }
}

/// Process PolyView response
fn process_polyview_response(
    response: ureq::Response,
    target_url: &str,
    proxy_port: u16,
    domain: &str,
    status: u16
) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    use std::io::Read;
    
    let content_type = response.content_type().to_string();
    let is_html = content_type.contains("text/html");
    
    // Store cookies from response
    store_polyview_cookies(&response, domain);
    
    // Read body
    let body = if is_html {
        let mut body_str = String::new();
        let reader = response.into_reader();
        let _ = reader.take(50_000_000).read_to_string(&mut body_str);
        rewrite_polyview_html(&body_str, target_url, proxy_port).into_bytes()
    } else {
        let mut body = Vec::new();
        let reader = response.into_reader();
        let _ = reader.take(50_000_000).read_to_end(&mut body);
        body
    };
    
    // Build response - strip ALL blocking headers
    let mut resp = tiny_http::Response::from_data(body)
        .with_status_code(status);
    
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap()
    );
    // Remove X-Frame-Options completely by setting ALLOWALL
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"X-Frame-Options"[..], &b"ALLOWALL"[..]).unwrap()
    );
    // Override CSP to allow framing from anywhere AND allow all content
    resp = resp.with_header(
        tiny_http::Header::from_bytes(
            &b"Content-Security-Policy"[..], 
            &b"frame-ancestors *; default-src * 'unsafe-inline' 'unsafe-eval' data: blob:; script-src * 'unsafe-inline' 'unsafe-eval'; style-src * 'unsafe-inline';"[..]
        ).unwrap()
    );
    // Allow all origins
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap()
    );
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, PUT, DELETE, OPTIONS"[..]).unwrap()
    );
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"*"[..]).unwrap()
    );
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Credentials"[..], &b"true"[..]).unwrap()
    );
    
    resp
}

/// Handle PolyView POST request - for form submissions
fn handle_polyview_proxy_post(target_url: &str, proxy_port: u16, body: &[u8], content_type: &str) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    
    
    println!("[PolyView] POST: {}", target_url);
    
    let domain = url::Url::parse(target_url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()))
        .unwrap_or_default();
    
    let client = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build();
    
    // Get stored cookies
    let cookie_header = {
        let cookies = POLYVIEW_COOKIES.lock().unwrap();
        if let Some(domain_cookies) = cookies.get(&domain) {
            domain_cookies.join("; ")
        } else {
            String::new()
        }
    };
    
    let mut req = client.post(target_url)
        .set("Content-Type", content_type)
        .set("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .set("Accept-Language", "en-US,en;q=0.9")
        .set("Origin", &format!("http://localhost:{}", proxy_port))
        .set("Sec-Ch-Ua", "\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\"")
        .set("Sec-Ch-Ua-Mobile", "?0")
        .set("Sec-Ch-Ua-Platform", "\"Windows\"")
        .set("Sec-Fetch-Dest", "document")
        .set("Sec-Fetch-Mode", "navigate")
        .set("Sec-Fetch-Site", "same-origin")
        .set("Upgrade-Insecure-Requests", "1");
    
    if !cookie_header.is_empty() {
        req = req.set("Cookie", &cookie_header);
    }
    
    match req.send_bytes(body) {
        Ok(response) => {
            process_polyview_response(response, target_url, proxy_port, &domain, 200)
        }
        Err(ureq::Error::Status(code, response)) => {
            if (300..400).contains(&code) {
                if let Some(location) = response.header("Location") {
                    let absolute_url = resolve_polyview_url(target_url, location);
                    let proxied_location = format!(
                        "http://localhost:{}/__polyview/?url={}",
                        proxy_port,
                        urlencoding::encode(&absolute_url)
                    );
                    
                    store_polyview_cookies(&response, &domain);
                    
                    return tiny_http::Response::from_string("")
                        .with_status_code(code)
                        .with_header(tiny_http::Header::from_bytes(&b"Location"[..], proxied_location.as_bytes()).unwrap())
                        .with_header(tiny_http::Header::from_bytes(&b"X-Frame-Options"[..], &b"ALLOWALL"[..]).unwrap());
                }
            }
            process_polyview_response(response, target_url, proxy_port, &domain, code)
        }
        Err(e) => {
            println!("[PolyView] POST Error: {}", e);
            let error_html = format!(
                r#"<!DOCTYPE html><html><body style="font-family:system-ui;padding:40px;background:#1a1a1f;color:#fff">
                <h1>POST Failed</h1><p style="color:#888">{}</p></body></html>"#,
                e
            );
            tiny_http::Response::from_string(error_html)
                .with_status_code(502)
                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap())
        }
    }
}

/// Resolve a relative URL against a base URL for PolyView
fn resolve_polyview_url(base: &str, relative: &str) -> String {
    if relative.starts_with("http://") || relative.starts_with("https://") {
        relative.to_string()
    } else if relative.starts_with("//") {
        if base.starts_with("https://") {
            format!("https:{}", relative)
        } else {
            format!("http:{}", relative)
        }
    } else if let Ok(base_url) = url::Url::parse(base) {
        base_url.join(relative)
            .map(|u| u.to_string())
            .unwrap_or_else(|_| relative.to_string())
    } else {
        relative.to_string()
    }
}

/// Rewrite HTML to proxy all URLs through PolyView
/// This is the core of "iframe2" - making the content work seamlessly in an iframe
fn rewrite_polyview_html(html: &str, base_url: &str, proxy_port: u16) -> String {
    let proxy_base = format!("http://localhost:{}/__polyview/?url=", proxy_port);
    
    let mut result = html.to_string();
    
    // Inject base tag for relative URLs
    let base_tag = format!(r#"<base href="{}">"#, base_url);
    if let Some(head_pos) = result.to_lowercase().find("<head") {
        if let Some(close_pos) = result[head_pos..].find('>') {
            let insert_pos = head_pos + close_pos + 1;
            result.insert_str(insert_pos, &base_tag);
        }
    }
    
    // Rewrite href, src, action attributes
    result = rewrite_polyview_attribute(&result, "href", base_url, &proxy_base);
    result = rewrite_polyview_attribute(&result, "src", base_url, &proxy_base);
    result = rewrite_polyview_attribute(&result, "action", base_url, &proxy_base);
    
    // Inject PolyView client script - this is what makes iframe2 special
    // It intercepts navigation, form submissions, and reports state to parent
    let client_script = format!(r#"
<script>
(function() {{
    // PolyView Client - "iframe2" magic
    const PROXY_BASE = 'http://localhost:{}/__polyview/?url=';
    const BASE_URL = '{}';
    const PROXY_PORT = {};
    
    // Report to parent window
    function reportToParent(type, data) {{
        if (window.parent && window.parent !== window) {{
            window.parent.postMessage({{ type: 'polyview:' + type, ...data }}, '*');
        }}
    }}
    
    // Convert URL to proxy URL
    function toProxyUrl(url) {{
        if (!url || url.startsWith('javascript:') || url.startsWith('data:') || 
            url.startsWith('mailto:') || url.startsWith('tel:') || url.startsWith('#') ||
            url.includes('/__polyview/')) {{
            return url;
        }}
        // Make absolute
        try {{
            const absolute = new URL(url, BASE_URL).href;
            return PROXY_BASE + encodeURIComponent(absolute);
        }} catch {{
            return url;
        }}
    }}
    
    // Intercept link clicks
    document.addEventListener('click', function(e) {{
        const link = e.target.closest('a[href]');
        if (link) {{
            const href = link.getAttribute('href');
            if (href && !href.startsWith('javascript:') && !href.startsWith('#') && 
                !href.startsWith('mailto:') && !href.startsWith('tel:')) {{
                e.preventDefault();
                const proxyUrl = toProxyUrl(href);
                window.location.href = proxyUrl;
            }}
        }}
    }}, true);
    
    // Intercept form submissions
    document.addEventListener('submit', function(e) {{
        const form = e.target;
        if (form.tagName === 'FORM') {{
            const action = form.getAttribute('action') || window.location.href;
            if (!action.includes('/__polyview/')) {{
                e.preventDefault();
                const proxyAction = toProxyUrl(action);
                form.setAttribute('action', proxyAction);
                form.submit();
            }}
        }}
    }}, true);
    
    // Intercept window.open
    const originalOpen = window.open;
    window.open = function(url, target, features) {{
        if (url && !url.startsWith('javascript:')) {{
            url = toProxyUrl(url);
        }}
        return originalOpen.call(window, url, target, features);
    }};
    
    // Intercept location changes
    const locationProxy = new Proxy(window.location, {{
        set: function(target, prop, value) {{
            if (prop === 'href' && value && !value.includes('/__polyview/')) {{
                value = toProxyUrl(value);
            }}
            target[prop] = value;
            return true;
        }}
    }});
    
    // Report page load
    window.addEventListener('load', function() {{
        reportToParent('loaded', {{ url: BASE_URL, title: document.title }});
    }});
    
    // Watch for title changes
    const titleEl = document.querySelector('title');
    if (titleEl) {{
        new MutationObserver(function() {{
            reportToParent('title', {{ title: document.title }});
        }}).observe(titleEl, {{ childList: true, characterData: true, subtree: true }});
    }}
    
    // Report initial navigation
    reportToParent('navigate', {{ url: BASE_URL }});
    
    // Override fetch to handle CORS
    const originalFetch = window.fetch;
    window.fetch = function(url, options) {{
        if (typeof url === 'string' && url.startsWith('http') && !url.includes('localhost')) {{
            // Route through proxy for cross-origin requests
            url = toProxyUrl(url);
        }}
        return originalFetch.call(window, url, options);
    }};
    
    // Override XMLHttpRequest
    const originalXHROpen = XMLHttpRequest.prototype.open;
    XMLHttpRequest.prototype.open = function(method, url, ...args) {{
        if (typeof url === 'string' && url.startsWith('http') && !url.includes('localhost')) {{
            url = toProxyUrl(url);
        }}
        return originalXHROpen.call(this, method, url, ...args);
    }};
    
    console.log('[PolyView] iframe2 client loaded for:', BASE_URL);
}})();
</script>
"#, proxy_port, base_url, proxy_port);
    
    if let Some(pos) = result.to_lowercase().find("</head>") {
        result.insert_str(pos, &client_script);
    } else if let Some(pos) = result.to_lowercase().find("<body") {
        result.insert_str(pos, &client_script);
    }
    
    result
}

/// Rewrite a specific attribute in HTML for PolyView
fn rewrite_polyview_attribute(html: &str, attr: &str, base_url: &str, proxy_base: &str) -> String {
    let mut result = String::with_capacity(html.len() * 2);
    let mut remaining = html;
    
    let patterns = [
        format!(r#"{}=""#, attr),
        format!(r#"{}='"#, attr),
    ];
    
    while !remaining.is_empty() {
        let mut found = false;
        
        for pattern in &patterns {
            if let Some(pos) = remaining.to_lowercase().find(&pattern.to_lowercase()) {
                result.push_str(&remaining[..pos]);
                
                let after_attr = &remaining[pos + pattern.len()..];
                let quote = if pattern.ends_with('"') { '"' } else { '\'' };
                let end_pos = after_attr.find(quote).unwrap_or(after_attr.len());
                let url_value = &after_attr[..end_pos];
                
                let should_proxy = !url_value.starts_with('#') 
                    && !url_value.starts_with("javascript:")
                    && !url_value.starts_with("data:")
                    && !url_value.starts_with("mailto:")
                    && !url_value.starts_with("tel:")
                    && !url_value.contains("/__polyview/")
                    && !url_value.is_empty();
                
                if should_proxy {
                    let absolute_url = resolve_polyview_url(base_url, url_value);
                    let proxied_url = format!("{}{}", proxy_base, urlencoding::encode(&absolute_url));
                    result.push_str(&format!("{}=\"{}", attr, proxied_url));
                } else {
                    result.push_str(&remaining[pos..pos + pattern.len()]);
                    result.push_str(url_value);
                }
                
                result.push(quote);
                remaining = &after_attr[end_pos + 1..];
                found = true;
                break;
            }
        }
        
        if !found {
            result.push_str(remaining);
            break;
        }
    }
    
    result
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
        Some(Commands::Run { path, release, native, browser, ui_height }) => run_app_result(&path, release, native, browser, ui_height),
        Some(Commands::Build { path, target, release, installer, sign, ci }) => { 
            if ci {
                if let Err(e) = build::generate_ci_workflow(Path::new(&path)) {
                    eprintln!("{}error{}: {}", RED, RESET, e);
                    std::process::exit(1);
                }
            } else {
                build_app(&path, &target, release, installer, sign); 
            }
            Ok(()) 
        },
        Some(Commands::New { name, template }) => { create_project(&name, &template); Ok(()) },
        Some(Commands::Init { template }) => { init_project(&template); Ok(()) },
        Some(Commands::Update) => { check_for_updates_interactive(); Ok(()) },
        Some(Commands::Add { package, version }) => packages::add_package(&package, version.as_deref()),
        Some(Commands::Remove { package }) => packages::remove_package(&package),
        Some(Commands::Install { verify }) => packages::install_packages(verify),
        Some(Commands::OpenUrl { url, title, width, height }) => {
            open_url_window(&url, &title, width, height);
            Ok(())
        },
        Some(Commands::Browser { url, title, width, height, ui_height, ui_html }) => {
            run_browser_mode(&url, &title, width, height, ui_height, ui_html);
            Ok(())
        },
        None => {
            // Check if we're running as a bundled app (bundle folder or poly.toml next to exe)
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let bundle_dir = exe_dir.join("bundle");
                    let poly_toml = exe_dir.join("poly.toml");
                    let bundle_poly_toml = bundle_dir.join("poly.toml");
                    
                    // If bundle exists, run as native app
                    if bundle_dir.exists() && (bundle_poly_toml.exists() || poly_toml.exists()) {
                        // Hide console window on Windows when running as bundled app
                        #[cfg(all(target_os = "windows", feature = "native"))]
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
                        
                        // Run native app from bundle (default: no browser mode, ui_height 80)
                        run_native_app(exe_dir, false, false, 80);
                        return;
                    }
                }
            }
            
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
#[allow(dead_code)]
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
    println!("  {}POLY{} v{}", CYAN, RESET, VERSION);
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
    
    // Disable SovereigntyEngine in development mode
    poly::sovereignty::set_development_mode();
    
    let project_path = Path::new(path);
    let project_path_owned = project_path.to_path_buf();
    
    // Load config from poly.toml
    let config = poly::PolyConfig::load(project_path);
    
    // Get values from config
    let inject_alpine = config.dev.inject_alpine;
    let inject_lucide = config.dev.inject_lucide;
    let reload_interval = config.dev.reload_interval;
    
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
            eprintln!("{}error{}: Failed to initialize interpreter: {}", RED, RESET, e);
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
            let response = match url.as_str() {
                "/" | "/index.html" => {
                    // Check for custom index.html in web folder
                    let custom_index = project_path_http.join("web/index.html");
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
                _ => {
                    // First try the exact path, then try in web/ folder, then packages/
                    let url_path = url.trim_start_matches('/');
                    let file_path = project_path_http.join(url_path);
                    let web_file_path = project_path_http.join("web").join(url_path);
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

/// Handle IPC invoke with a persistent (stateful) interpreter
fn handle_ipc_invoke_stateful(interpreter: &std::sync::Arc<std::sync::Mutex<poly::interpreter::Interpreter>>, body: &str) -> String {
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
    
    // Build argument string from JSON - skip empty objects
    let args_str = if args.is_object() && args.as_object().map(|o| o.is_empty()).unwrap_or(false) {
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
    
    // Build argument string from JSON - skip empty objects
    let args_str = if args.is_object() && args.as_object().map(|o| o.is_empty()).unwrap_or(false) {
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

/// Handle built-in system APIs
#[allow(unused_variables)]
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
        // Clipboard APIs
        "__poly_clipboard_read" => {
            #[cfg(feature = "native")]
            {
                match poly::clipboard::read_text() {
                    Ok(text) => serde_json::json!({"result": text}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_clipboard_write" => {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            
            #[cfg(feature = "native")]
            {
                match poly::clipboard::write_text(text) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_clipboard_clear" => {
            #[cfg(feature = "native")]
            {
                match poly::clipboard::clear() {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        // Multi-Window APIs
        "__poly_window_create" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("New Window");
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(800) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(600) as u32;
            let url = args.get("url").and_then(|v| v.as_str());
            let html = args.get("html").and_then(|v| v.as_str());
            
            #[cfg(feature = "native")]
            {
                let mut config = poly::window::WindowConfig::new(title)
                    .with_size(width, height);
                
                if let Some(u) = url {
                    config = config.with_url(u);
                }
                if let Some(h) = html {
                    config = config.with_html(h);
                }
                
                match poly::window::create_window(config) {
                    Ok(handle) => serde_json::json!({"result": {"id": handle.id}}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_count" => {
            #[cfg(feature = "native")]
            {
                let count = poly::window::window_count();
                serde_json::json!({"result": count}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": 0}).to_string()
            }
        }
        "__poly_window_close" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            
            #[cfg(feature = "native")]
            {
                match poly::window::close_window(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_close_all" => {
            #[cfg(feature = "native")]
            {
                match poly::window::close_all_windows() {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_list" => {
            #[cfg(feature = "native")]
            {
                let ids = poly::window::list_windows();
                serde_json::json!({"result": ids}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": []}).to_string()
            }
        }
        "__poly_window_minimize" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            #[cfg(feature = "native")]
            {
                match poly::window::minimize_window(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_maximize" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            #[cfg(feature = "native")]
            {
                match poly::window::maximize_window(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_restore" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            #[cfg(feature = "native")]
            {
                match poly::window::restore_window(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_show" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            #[cfg(feature = "native")]
            {
                match poly::window::show_window(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_hide" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            #[cfg(feature = "native")]
            {
                match poly::window::hide_window(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_focus" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            #[cfg(feature = "native")]
            {
                match poly::window::focus_window(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_set_title" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("");
            #[cfg(feature = "native")]
            {
                match poly::window::set_window_title(id, title) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_set_size" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(800) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(600) as u32;
            #[cfg(feature = "native")]
            {
                match poly::window::set_window_size(id, width, height) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_set_position" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            #[cfg(feature = "native")]
            {
                match poly::window::set_window_position(id, x, y) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_set_always_on_top" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let value = args.get("value").and_then(|v| v.as_bool()).unwrap_or(false);
            #[cfg(feature = "native")]
            {
                match poly::window::set_window_always_on_top(id, value) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_set_fullscreen" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let value = args.get("value").and_then(|v| v.as_bool()).unwrap_or(false);
            #[cfg(feature = "native")]
            {
                match poly::window::set_window_fullscreen(id, value) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_navigate" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            #[cfg(feature = "native")]
            {
                match poly::window::navigate_window(id, url) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_load_html" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let html = args.get("html").and_then(|v| v.as_str()).unwrap_or("");
            #[cfg(feature = "native")]
            {
                match poly::window::load_window_html(id, html) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_eval" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let script = args.get("script").and_then(|v| v.as_str()).unwrap_or("");
            #[cfg(feature = "native")]
            {
                match poly::window::eval_window_script(id, script) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_get_state" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            #[cfg(feature = "native")]
            {
                match poly::window::get_window_state(id) {
                    Ok(state) => serde_json::json!({
                        "result": {
                            "id": state.id,
                            "title": state.title,
                            "width": state.width,
                            "height": state.height,
                            "x": state.x,
                            "y": state.y,
                            "isVisible": state.is_visible,
                            "isMinimized": state.is_minimized,
                            "isMaximized": state.is_maximized,
                            "isFullscreen": state.is_fullscreen,
                            "isFocused": state.is_focused
                        }
                    }).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_list_states" => {
            #[cfg(feature = "native")]
            {
                let states: Vec<_> = poly::window::list_window_states()
                    .into_iter()
                    .map(|s| serde_json::json!({
                        "id": s.id,
                        "title": s.title,
                        "width": s.width,
                        "height": s.height,
                        "x": s.x,
                        "y": s.y,
                        "isVisible": s.is_visible,
                        "isMinimized": s.is_minimized,
                        "isMaximized": s.is_maximized,
                        "isFullscreen": s.is_fullscreen,
                        "isFocused": s.is_focused
                    }))
                    .collect();
                serde_json::json!({"result": states}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": []}).to_string()
            }
        }
        // Notification APIs
        "__poly_notification_show" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Notification");
            let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
            let icon = args.get("icon").and_then(|v| v.as_str());
            
            #[cfg(feature = "native")]
            {
                match poly::notification::show(title, body, icon) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_notification_show_timeout" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Notification");
            let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
            let timeout = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(5000) as u32;
            
            #[cfg(feature = "native")]
            {
                match poly::notification::show_with_timeout(title, body, timeout) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        // Deep Link APIs
        "__poly_deeplink_register" => {
            let protocol = args.get("protocol").and_then(|v| v.as_str()).unwrap_or("");
            let app_name = args.get("appName").and_then(|v| v.as_str()).unwrap_or("Poly App");
            
            if protocol.is_empty() {
                return serde_json::json!({"error": "Protocol name required"}).to_string();
            }
            
            match poly::deeplink::register_protocol(protocol, app_name) {
                Ok(_) => serde_json::json!({"result": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_deeplink_unregister" => {
            let protocol = args.get("protocol").and_then(|v| v.as_str()).unwrap_or("");
            
            if protocol.is_empty() {
                return serde_json::json!({"error": "Protocol name required"}).to_string();
            }
            
            match poly::deeplink::unregister_protocol(protocol) {
                Ok(_) => serde_json::json!({"result": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_deeplink_is_registered" => {
            let protocol = args.get("protocol").and_then(|v| v.as_str()).unwrap_or("");
            let registered = poly::deeplink::is_protocol_registered(protocol);
            serde_json::json!({"result": registered}).to_string()
        }
        "__poly_deeplink_get" => {
            let link = poly::deeplink::get_deep_link();
            serde_json::json!({"result": link}).to_string()
        }
        "__poly_deeplink_has" => {
            let has = poly::deeplink::has_deep_link();
            serde_json::json!({"result": has}).to_string()
        }
        "__poly_tray_is_enabled" => {
            // This is determined by poly.toml config, return false in dev mode
            // In native mode, the actual tray state is managed by the window
            serde_json::json!({"result": false}).to_string()
        }
        // Shell APIs
        "__poly_shell_open" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "start", "", url])
                    .spawn();
            }
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open")
                    .arg(url)
                    .spawn();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("xdg-open")
                    .arg(url)
                    .spawn();
            }
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_shell_open_path" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("explorer")
                    .arg(path)
                    .spawn();
            }
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open")
                    .arg(path)
                    .spawn();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("xdg-open")
                    .arg(path)
                    .spawn();
            }
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_shell_open_with" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let app = args.get("app").and_then(|v| v.as_str()).unwrap_or("");
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new(app)
                    .arg(path)
                    .spawn();
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = std::process::Command::new(app)
                    .arg(path)
                    .spawn();
            }
            serde_json::json!({"result": true}).to_string()
        }
        // App APIs
        "__poly_app_get_version" => {
            serde_json::json!({"result": VERSION}).to_string()
        }
        "__poly_app_get_name" => {
            let name = std::env::current_exe()
                .ok()
                .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
                .unwrap_or_else(|| "Poly App".to_string());
            serde_json::json!({"result": name}).to_string()
        }
        "__poly_app_get_path" => {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("data");
            let path = match name {
                "exe" => std::env::current_exe().ok().map(|p| p.to_string_lossy().to_string()),
                "data" | "appData" => dirs_path("data"),
                "config" => dirs_path("config"),
                "cache" => dirs_path("cache"),
                "temp" => Some(std::env::temp_dir().to_string_lossy().to_string()),
                "home" => dirs_path("home"),
                "desktop" => dirs_path("desktop"),
                "documents" => dirs_path("documents"),
                "downloads" => dirs_path("downloads"),
                _ => None,
            };
            match path {
                Some(p) => serde_json::json!({"result": p}).to_string(),
                None => serde_json::json!({"result": null}).to_string(),
            }
        }
        "__poly_app_exit" => {
            let code = args.get("code").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            std::process::exit(code);
        }
        "__poly_app_relaunch" => {
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::process::Command::new(exe)
                    .args(std::env::args().skip(1))
                    .spawn();
            }
            std::process::exit(0);
        }
        // OS APIs
        "__poly_os_platform" => {
            let platform = if cfg!(target_os = "windows") { "windows" }
                else if cfg!(target_os = "macos") { "macos" }
                else if cfg!(target_os = "linux") { "linux" }
                else { "unknown" };
            serde_json::json!({"result": platform}).to_string()
        }
        "__poly_os_arch" => {
            let arch = if cfg!(target_arch = "x86_64") { "x64" }
                else if cfg!(target_arch = "aarch64") { "arm64" }
                else if cfg!(target_arch = "x86") { "x86" }
                else { "unknown" };
            serde_json::json!({"result": arch}).to_string()
        }
        "__poly_os_version" => {
            serde_json::json!({"result": std::env::consts::OS}).to_string()
        }
        "__poly_os_hostname" => {
            #[cfg(target_os = "windows")]
            let hostname = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".to_string());
            #[cfg(not(target_os = "windows"))]
            let hostname = std::env::var("HOSTNAME")
                .or_else(|_| std::env::var("HOST"))
                .unwrap_or_else(|_| "unknown".to_string());
            serde_json::json!({"result": hostname}).to_string()
        }
        "__poly_os_homedir" => {
            let home = dirs_path("home").unwrap_or_default();
            serde_json::json!({"result": home}).to_string()
        }
        "__poly_os_tempdir" => {
            let temp = std::env::temp_dir().to_string_lossy().to_string();
            serde_json::json!({"result": temp}).to_string()
        }
        // HTTP API
        "__poly_http_get" | "__poly_http_post" | "__poly_http_put" | "__poly_http_patch" | "__poly_http_delete" | "__poly_http_request" => {
            handle_http_request(fn_name, args)
        }
        // AI/LLM API
        "__poly_ai_check_ollama" => {
            #[cfg(feature = "native")]
            {
                match poly::ai::check_ollama() {
                    Ok(available) => serde_json::json!({"result": available}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_ai_list_models" => {
            #[cfg(feature = "native")]
            {
                match poly::ai::list_ollama_models() {
                    Ok(models) => serde_json::json!({"result": models}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_ai_ollama" => {
            #[cfg(feature = "native")]
            {
                let model = args.get("model").and_then(|v| v.as_str()).unwrap_or("llama3");
                let messages = args.get("messages").and_then(|v| v.as_array());
                let temperature = args.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
                let max_tokens = args.get("maxTokens").and_then(|v| v.as_u64()).map(|v| v as u32);
                let think = args.get("think").and_then(|v| v.as_bool()).unwrap_or(false);
                let tools_json = args.get("tools").and_then(|v| v.as_array());
                let tool_choice = args.get("toolChoice").and_then(|v| v.as_str()).unwrap_or("auto");
                
                let chat_messages: Vec<poly::ai::ChatMessage> = messages
                    .map(|arr| arr.iter().filter_map(|m| {
                        let role = m.get("role")?.as_str()?;
                        let content = m.get("content")?.as_str()?;
                        Some(poly::ai::ChatMessage {
                            role: match role {
                                "system" => poly::ai::MessageRole::System,
                                "assistant" => poly::ai::MessageRole::Assistant,
                                _ => poly::ai::MessageRole::User,
                            },
                            content: content.to_string(),
                        })
                    }).collect())
                    .unwrap_or_default();
                
                // Parse tools from JSON
                let tools: Vec<poly::ai::Tool> = tools_json
                    .map(|arr| arr.iter().filter_map(|t| {
                        serde_json::from_value(t.clone()).ok()
                    }).collect())
                    .unwrap_or_default();
                
                let request = poly::ai::ChatRequest {
                    provider: poly::ai::AiProvider::Ollama,
                    base_url: None,
                    api_key: None,
                    model: model.to_string(),
                    messages: chat_messages,
                    temperature,
                    max_tokens,
                    stream: false,
                    enable_thinking: think,
                    thinking_budget: None,
                    tools,
                    tool_choice: tool_choice.to_string(),
                };
                
                match poly::ai::chat(&request) {
                    Ok(resp) => serde_json::json!({
                        "result": {
                            "content": resp.content,
                            "thinking": resp.thinking,
                            "model": resp.model,
                            "usage": resp.usage,
                            "toolCalls": resp.tool_calls,
                            "finishReason": resp.finish_reason
                        }
                    }).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_ai_openai" => {
            #[cfg(feature = "native")]
            {
                let model = args.get("model").and_then(|v| v.as_str()).unwrap_or("gpt-4");
                let messages = args.get("messages").and_then(|v| v.as_array());
                let api_key = args.get("apiKey").and_then(|v| v.as_str());
                let temperature = args.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
                let max_tokens = args.get("maxTokens").and_then(|v| v.as_u64()).map(|v| v as u32);
                let tools_json = args.get("tools").and_then(|v| v.as_array());
                let tool_choice = args.get("toolChoice").and_then(|v| v.as_str()).unwrap_or("auto");
                
                let chat_messages: Vec<poly::ai::ChatMessage> = messages
                    .map(|arr| arr.iter().filter_map(|m| {
                        let role = m.get("role")?.as_str()?;
                        let content = m.get("content")?.as_str()?;
                        Some(poly::ai::ChatMessage {
                            role: match role {
                                "system" => poly::ai::MessageRole::System,
                                "assistant" => poly::ai::MessageRole::Assistant,
                                _ => poly::ai::MessageRole::User,
                            },
                            content: content.to_string(),
                        })
                    }).collect())
                    .unwrap_or_default();
                
                // Parse tools from JSON
                let tools: Vec<poly::ai::Tool> = tools_json
                    .map(|arr| arr.iter().filter_map(|t| {
                        serde_json::from_value(t.clone()).ok()
                    }).collect())
                    .unwrap_or_default();
                
                let request = poly::ai::ChatRequest {
                    provider: poly::ai::AiProvider::OpenAI,
                    base_url: None,
                    api_key: api_key.map(|s| s.to_string()),
                    model: model.to_string(),
                    messages: chat_messages,
                    temperature,
                    max_tokens,
                    stream: false,
                    enable_thinking: false,
                    thinking_budget: None,
                    tools,
                    tool_choice: tool_choice.to_string(),
                };
                
                match poly::ai::chat(&request) {
                    Ok(resp) => serde_json::json!({
                        "result": {
                            "content": resp.content,
                            "thinking": resp.thinking,
                            "model": resp.model,
                            "usage": resp.usage,
                            "toolCalls": resp.tool_calls,
                            "finishReason": resp.finish_reason
                        }
                    }).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_ai_anthropic" => {
            #[cfg(feature = "native")]
            {
                let model = args.get("model").and_then(|v| v.as_str()).unwrap_or("claude-3-5-sonnet-20241022");
                let messages = args.get("messages").and_then(|v| v.as_array());
                let api_key = args.get("apiKey").and_then(|v| v.as_str());
                let temperature = args.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
                let max_tokens = args.get("maxTokens").and_then(|v| v.as_u64()).map(|v| v as u32);
                let enable_thinking = args.get("enableThinking").and_then(|v| v.as_bool()).unwrap_or(false);
                let thinking_budget = args.get("thinkingBudget").and_then(|v| v.as_u64()).map(|v| v as u32);
                let tools_json = args.get("tools").and_then(|v| v.as_array());
                let tool_choice = args.get("toolChoice").and_then(|v| v.as_str()).unwrap_or("auto");
                
                let chat_messages: Vec<poly::ai::ChatMessage> = messages
                    .map(|arr| arr.iter().filter_map(|m| {
                        let role = m.get("role")?.as_str()?;
                        let content = m.get("content")?.as_str()?;
                        Some(poly::ai::ChatMessage {
                            role: match role {
                                "system" => poly::ai::MessageRole::System,
                                "assistant" => poly::ai::MessageRole::Assistant,
                                _ => poly::ai::MessageRole::User,
                            },
                            content: content.to_string(),
                        })
                    }).collect())
                    .unwrap_or_default();
                
                // Parse tools from JSON
                let tools: Vec<poly::ai::Tool> = tools_json
                    .map(|arr| arr.iter().filter_map(|t| {
                        serde_json::from_value(t.clone()).ok()
                    }).collect())
                    .unwrap_or_default();
                
                let request = poly::ai::ChatRequest {
                    provider: poly::ai::AiProvider::Anthropic,
                    base_url: None,
                    api_key: api_key.map(|s| s.to_string()),
                    model: model.to_string(),
                    messages: chat_messages,
                    temperature,
                    max_tokens,
                    stream: false,
                    enable_thinking,
                    thinking_budget,
                    tools,
                    tool_choice: tool_choice.to_string(),
                };
                
                match poly::ai::chat(&request) {
                    Ok(resp) => serde_json::json!({
                        "result": {
                            "content": resp.content,
                            "thinking": resp.thinking,
                            "model": resp.model,
                            "usage": resp.usage,
                            "toolCalls": resp.tool_calls,
                            "finishReason": resp.finish_reason
                        }
                    }).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_ai_custom" => {
            #[cfg(feature = "native")]
            {
                let base_url = args.get("baseUrl").and_then(|v| v.as_str()).unwrap_or("http://localhost:8080");
                let model = args.get("model").and_then(|v| v.as_str()).unwrap_or("default");
                let messages = args.get("messages").and_then(|v| v.as_array());
                let api_key = args.get("apiKey").and_then(|v| v.as_str());
                let temperature = args.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
                let max_tokens = args.get("maxTokens").and_then(|v| v.as_u64()).map(|v| v as u32);
                let tools_json = args.get("tools").and_then(|v| v.as_array());
                let tool_choice = args.get("toolChoice").and_then(|v| v.as_str()).unwrap_or("auto");
                
                let chat_messages: Vec<poly::ai::ChatMessage> = messages
                    .map(|arr| arr.iter().filter_map(|m| {
                        let role = m.get("role")?.as_str()?;
                        let content = m.get("content")?.as_str()?;
                        Some(poly::ai::ChatMessage {
                            role: match role {
                                "system" => poly::ai::MessageRole::System,
                                "assistant" => poly::ai::MessageRole::Assistant,
                                _ => poly::ai::MessageRole::User,
                            },
                            content: content.to_string(),
                        })
                    }).collect())
                    .unwrap_or_default();
                
                // Parse tools from JSON
                let tools: Vec<poly::ai::Tool> = tools_json
                    .map(|arr| arr.iter().filter_map(|t| {
                        serde_json::from_value(t.clone()).ok()
                    }).collect())
                    .unwrap_or_default();
                
                let request = poly::ai::ChatRequest {
                    provider: poly::ai::AiProvider::Custom,
                    base_url: Some(base_url.to_string()),
                    api_key: api_key.map(|s| s.to_string()),
                    model: model.to_string(),
                    messages: chat_messages,
                    temperature,
                    max_tokens,
                    stream: false,
                    enable_thinking: false,
                    thinking_budget: None,
                    tools,
                    tool_choice: tool_choice.to_string(),
                };
                
                match poly::ai::chat(&request) {
                    Ok(resp) => serde_json::json!({
                        "result": {
                            "content": resp.content,
                            "thinking": resp.thinking,
                            "model": resp.model,
                            "usage": resp.usage,
                            "toolCalls": resp.tool_calls,
                            "finishReason": resp.finish_reason
                        }
                    }).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_ai_chat" => {
            #[cfg(feature = "native")]
            {
                let provider_str = args.get("provider").and_then(|v| v.as_str()).unwrap_or("ollama");
                let model = args.get("model").and_then(|v| v.as_str()).unwrap_or("llama3");
                let messages = args.get("messages").and_then(|v| v.as_array());
                let base_url = args.get("baseUrl").and_then(|v| v.as_str());
                let api_key = args.get("apiKey").and_then(|v| v.as_str());
                let temperature = args.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
                let max_tokens = args.get("maxTokens").and_then(|v| v.as_u64()).map(|v| v as u32);
                let enable_thinking = args.get("enableThinking").and_then(|v| v.as_bool()).unwrap_or(false);
                let thinking_budget = args.get("thinkingBudget").and_then(|v| v.as_u64()).map(|v| v as u32);
                let tools_json = args.get("tools").and_then(|v| v.as_array());
                let tool_choice = args.get("toolChoice").and_then(|v| v.as_str()).unwrap_or("auto");
                
                let provider = match provider_str {
                    "openai" => poly::ai::AiProvider::OpenAI,
                    "anthropic" => poly::ai::AiProvider::Anthropic,
                    "custom" => poly::ai::AiProvider::Custom,
                    _ => poly::ai::AiProvider::Ollama,
                };
                
                let chat_messages: Vec<poly::ai::ChatMessage> = messages
                    .map(|arr| arr.iter().filter_map(|m| {
                        let role = m.get("role")?.as_str()?;
                        let content = m.get("content")?.as_str()?;
                        Some(poly::ai::ChatMessage {
                            role: match role {
                                "system" => poly::ai::MessageRole::System,
                                "assistant" => poly::ai::MessageRole::Assistant,
                                _ => poly::ai::MessageRole::User,
                            },
                            content: content.to_string(),
                        })
                    }).collect())
                    .unwrap_or_default();
                
                // Parse tools from JSON
                let tools: Vec<poly::ai::Tool> = tools_json
                    .map(|arr| arr.iter().filter_map(|t| {
                        serde_json::from_value(t.clone()).ok()
                    }).collect())
                    .unwrap_or_default();
                
                let request = poly::ai::ChatRequest {
                    provider,
                    base_url: base_url.map(|s| s.to_string()),
                    api_key: api_key.map(|s| s.to_string()),
                    model: model.to_string(),
                    messages: chat_messages,
                    temperature,
                    max_tokens,
                    stream: false,
                    enable_thinking,
                    thinking_budget,
                    tools,
                    tool_choice: tool_choice.to_string(),
                };
                
                match poly::ai::chat(&request) {
                    Ok(resp) => serde_json::json!({
                        "result": {
                            "content": resp.content,
                            "thinking": resp.thinking,
                            "model": resp.model,
                            "usage": resp.usage,
                            "provider": resp.provider,
                            "toolCalls": resp.tool_calls,
                            "finishReason": resp.finish_reason
                        }
                    }).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        // AI Streaming API
        "__poly_ai_stream_start" => {
            #[cfg(feature = "native")]
            {
                let model = args.get("model").and_then(|v| v.as_str()).unwrap_or("llama3");
                let messages = args.get("messages").and_then(|v| v.as_array());
                let temperature = args.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
                let think = args.get("think").and_then(|v| v.as_bool()).unwrap_or(false);
                
                let chat_messages: Vec<poly::ai::ChatMessage> = messages
                    .map(|arr| arr.iter().filter_map(|m| {
                        let role = m.get("role")?.as_str()?;
                        let content = m.get("content")?.as_str()?;
                        Some(poly::ai::ChatMessage {
                            role: match role {
                                "system" => poly::ai::MessageRole::System,
                                "assistant" => poly::ai::MessageRole::Assistant,
                                _ => poly::ai::MessageRole::User,
                            },
                            content: content.to_string(),
                        })
                    }).collect())
                    .unwrap_or_default();
                
                match poly::ai::stream_start_ollama(model, chat_messages, temperature, think) {
                    Ok(stream_id) => serde_json::json!({"result": {"streamId": stream_id}}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_ai_stream_poll" => {
            let stream_id = args.get("streamId").and_then(|v| v.as_str()).unwrap_or("");
            match poly::ai::stream_poll(stream_id) {
                Ok((chunks, done)) => serde_json::json!({
                    "result": {
                        "chunks": chunks,
                        "done": done
                    }
                }).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_ai_stream_cancel" => {
            let stream_id = args.get("streamId").and_then(|v| v.as_str()).unwrap_or("");
            match poly::ai::stream_cancel(stream_id) {
                Ok(_) => serde_json::json!({"result": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_ai_stream_list" => {
            let streams = poly::ai::stream_list();
            serde_json::json!({"result": streams}).to_string()
        }
        // Database API
        "__poly_db_open" | "__poly_db_close" | "__poly_db_execute" | "__poly_db_query" | "__poly_db_query_one" => {
            handle_db_request(fn_name, args)
        }
        // Browser API - Tab Management
        "__poly_browser_create_tab" => {
            let url = args.get("url").and_then(|v| v.as_str());
            let id = poly::browser::create_tab(url);
            serde_json::json!({"result": id}).to_string()
        }
        "__poly_browser_close_tab" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let success = poly::browser::close_tab(id);
            serde_json::json!({"result": success}).to_string()
        }
        "__poly_browser_get_tab" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            match poly::browser::get_tab(id) {
                Some(tab) => serde_json::json!({
                    "result": {
                        "id": tab.id,
                        "url": tab.url,
                        "title": tab.title,
                        "canGoBack": tab.can_go_back,
                        "canGoForward": tab.can_go_forward,
                        "isLoading": tab.is_loading
                    }
                }).to_string(),
                None => serde_json::json!({"result": null}).to_string(),
            }
        }
        "__poly_browser_list_tabs" => {
            let tabs: Vec<serde_json::Value> = poly::browser::list_tabs().iter().map(|tab| {
                serde_json::json!({
                    "id": tab.id,
                    "url": tab.url,
                    "title": tab.title,
                    "canGoBack": tab.can_go_back,
                    "canGoForward": tab.can_go_forward,
                    "isLoading": tab.is_loading
                })
            }).collect();
            serde_json::json!({"result": tabs}).to_string()
        }
        "__poly_browser_navigate" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            match poly::browser::navigate(id, url) {
                Ok(_) => serde_json::json!({"result": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_browser_back" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            match poly::browser::go_back(id) {
                Ok(url) => serde_json::json!({"result": url}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_browser_forward" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            match poly::browser::go_forward(id) {
                Ok(url) => serde_json::json!({"result": url}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_browser_set_title" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("");
            poly::browser::set_tab_title(id, title);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_browser_set_loading" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let loading = args.get("loading").and_then(|v| v.as_bool()).unwrap_or(false);
            poly::browser::set_tab_loading(id, loading);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_browser_get_history" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let history = poly::browser::get_history(id);
            serde_json::json!({"result": history}).to_string()
        }
        "__poly_browser_clear_history" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            poly::browser::clear_history(id);
            serde_json::json!({"result": true}).to_string()
        }
        // Browser API - Fetch URL content (bypasses iframe restrictions)
        "__poly_browser_fetch" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            
            // Check sovereignty - extract domain from URL
            if poly::sovereignty_enabled() {
                let domain = url.split("://")
                    .nth(1)
                    .and_then(|s| s.split('/').next())
                    .unwrap_or(url);
                let perm = poly::Permission::HttpConnect(poly::sovereignty::DomainScope::Domain(domain.to_string()));
                if let Err(e) = poly::check_permission(&perm) {
                    return serde_json::json!({"error": e}).to_string();
                }
            }
            
            match reqwest::blocking::get(url) {
                Ok(response) => {
                    let status = response.status().as_u16();
                    let headers: std::collections::HashMap<String, String> = response.headers()
                        .iter()
                        .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
                        .collect();
                    
                    match response.text() {
                        Ok(body) => serde_json::json!({
                            "result": {
                                "status": status,
                                "headers": headers,
                                "body": body
                            }
                        }).to_string(),
                        Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
                    }
                }
                Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
            }
        }
        // Browser API - Open real WebView window
        "__poly_browser_open_window" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("about:blank");
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Browser");
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(1024) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(768) as u32;
            
            #[cfg(feature = "native")]
            {
                match poly::browser::open_webview(url, title, width, height) {
                    Ok(id) => serde_json::json!({"result": id}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "WebView requires native feature"}).to_string()
            }
        }
        "__poly_browser_window_navigate" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            
            #[cfg(feature = "native")]
            {
                match poly::browser::webview_navigate(id, url) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "WebView requires native feature"}).to_string()
            }
        }
        "__poly_browser_window_close" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            
            #[cfg(feature = "native")]
            {
                match poly::browser::webview_close(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "WebView requires native feature"}).to_string()
            }
        }
        // Titlebar API
        "__poly_titlebar_set" => {
            let config = poly::titlebar::TitlebarConfig {
                enabled: args.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
                height: args.get("height").and_then(|v| v.as_u64()).unwrap_or(40) as u32,
                html: args.get("html").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                css: args.get("css").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                js: args.get("js").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                background: args.get("background").and_then(|v| v.as_str()).unwrap_or("#1a1a1f").to_string(),
            };
            poly::titlebar::set_titlebar(config);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_titlebar_get" => {
            let config = poly::titlebar::get_titlebar();
            serde_json::json!({
                "result": {
                    "enabled": config.enabled,
                    "height": config.height,
                    "html": config.html,
                    "css": config.css,
                    "js": config.js,
                    "background": config.background
                }
            }).to_string()
        }
        "__poly_titlebar_set_enabled" => {
            let enabled = args.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
            poly::titlebar::set_enabled(enabled);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_titlebar_set_height" => {
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(40) as u32;
            poly::titlebar::set_height(height);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_titlebar_set_html" => {
            let html = args.get("html").and_then(|v| v.as_str()).unwrap_or("");
            poly::titlebar::set_html(html);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_titlebar_set_css" => {
            let css = args.get("css").and_then(|v| v.as_str()).unwrap_or("");
            poly::titlebar::set_css(css);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_titlebar_set_js" => {
            let js = args.get("js").and_then(|v| v.as_str()).unwrap_or("");
            poly::titlebar::set_js(js);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_titlebar_navigate" => {
            // This will be handled specially - it navigates the WebView but injects titlebar
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            // Store the URL for the WebView to navigate to
            // The actual navigation happens in the native code
            serde_json::json!({"result": url, "action": "navigate"}).to_string()
        }
        // WebView API - Multi-WebView management
        "__poly_webview_create" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("about:blank");
            let html = args.get("html").and_then(|v| v.as_str()).map(|s| s.to_string());
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(800) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(600) as u32;
            let visible = args.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
            let transparent = args.get("transparent").and_then(|v| v.as_bool()).unwrap_or(false);
            let devtools = args.get("devtools").and_then(|v| v.as_bool()).unwrap_or(false);
            let user_agent = args.get("userAgent").and_then(|v| v.as_str()).map(|s| s.to_string());
            let zoom_level = args.get("zoomLevel").and_then(|v| v.as_f64()).unwrap_or(1.0);
            let autoplay = args.get("autoplay").and_then(|v| v.as_bool()).unwrap_or(true);
            
            let config = poly::webview::WebViewConfig {
                id: id.to_string(),
                url: url.to_string(),
                html,
                bounds: poly::webview::WebViewBounds { x, y, width, height },
                visible,
                transparent,
                devtools,
                user_agent,
                zoom_level,
                autoplay,
            };
            
            match poly::webview::create(config) {
                Ok(_) => serde_json::json!({"success": true, "id": id}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_navigate" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("about:blank");
            match poly::webview::navigate(id, url) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_load_html" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let html = args.get("html").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::load_html(id, html) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_go_back" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::go_back(id) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_go_forward" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::go_forward(id) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_reload" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::reload(id) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_stop" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::stop(id) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_set_bounds" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(800) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(600) as u32;
            match poly::webview::set_bounds(id, poly::webview::WebViewBounds { x, y, width, height }) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_get_bounds" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::get_bounds(id) {
                Ok(b) => serde_json::json!({"x": b.x, "y": b.y, "width": b.width, "height": b.height}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_eval" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let script = args.get("script").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::eval(id, script) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_destroy" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::destroy(id) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_list" => {
            let list: Vec<_> = poly::webview::list().iter().map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "url": s.url,
                    "title": s.title,
                    "visible": s.visible,
                    "isLoading": s.is_loading,
                    "canGoBack": s.can_go_back,
                    "canGoForward": s.can_go_forward,
                    "zoomLevel": s.zoom_level,
                    "bounds": { "x": s.bounds.x, "y": s.bounds.y, "width": s.bounds.width, "height": s.bounds.height }
                })
            }).collect();
            serde_json::json!(list).to_string()
        }
        "__poly_webview_get" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::get(id) {
                Some(s) => serde_json::json!({
                    "id": s.id,
                    "url": s.url,
                    "title": s.title,
                    "visible": s.visible,
                    "isLoading": s.is_loading,
                    "canGoBack": s.can_go_back,
                    "canGoForward": s.can_go_forward,
                    "zoomLevel": s.zoom_level,
                    "bounds": { "x": s.bounds.x, "y": s.bounds.y, "width": s.bounds.width, "height": s.bounds.height }
                }).to_string(),
                None => serde_json::json!({"error": format!("WebView '{}' not found", id)}).to_string(),
            }
        }
        "__poly_webview_set_zoom" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let level = args.get("level").and_then(|v| v.as_f64()).unwrap_or(1.0);
            match poly::webview::set_zoom(id, level) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_poll_events" => {
            let events = poly::webview::take_events();
            let json_events: Vec<_> = events.iter().map(|e| {
                match e {
                    poly::webview::WebViewEvent::NavigationStarted { id, url } => 
                        serde_json::json!({"type": "navigate", "id": id, "url": url}),
                    poly::webview::WebViewEvent::NavigationFinished { id, url } => 
                        serde_json::json!({"type": "navigateFinish", "id": id, "url": url}),
                    poly::webview::WebViewEvent::TitleChanged { id, title } => 
                        serde_json::json!({"type": "titleChange", "id": id, "title": title}),
                    poly::webview::WebViewEvent::LoadStarted { id } => 
                        serde_json::json!({"type": "loadStart", "id": id}),
                    poly::webview::WebViewEvent::LoadFinished { id } => 
                        serde_json::json!({"type": "loadFinish", "id": id}),
                    poly::webview::WebViewEvent::NewWindowRequested { id, url, target } => 
                        serde_json::json!({"type": "newWindow", "id": id, "url": url, "target": target}),
                    poly::webview::WebViewEvent::DownloadRequested { id, url, filename } => 
                        serde_json::json!({"type": "download", "id": id, "url": url, "filename": filename}),
                    poly::webview::WebViewEvent::Closed { id } => 
                        serde_json::json!({"type": "close", "id": id}),
                    poly::webview::WebViewEvent::Error { id, error } => 
                        serde_json::json!({"type": "error", "id": id, "error": error}),
                    poly::webview::WebViewEvent::FaviconChanged { id, url } => 
                        serde_json::json!({"type": "favicon", "id": id, "url": url}),
                    poly::webview::WebViewEvent::HistoryChanged { id, can_go_back, can_go_forward } => 
                        serde_json::json!({"type": "historyChange", "id": id, "canGoBack": can_go_back, "canGoForward": can_go_forward}),
                    poly::webview::WebViewEvent::FullscreenRequested { id, enter } => 
                        serde_json::json!({"type": "fullscreen", "id": id, "enter": enter}),
                    poly::webview::WebViewEvent::PermissionRequested { id, permission, origin } => 
                        serde_json::json!({"type": "permission", "id": id, "permission": permission, "origin": origin}),
                }
            }).collect();
            serde_json::json!({"events": json_events}).to_string()
        }
        "__poly_webview_respond_permission" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let permission = args.get("permission").and_then(|v| v.as_str()).unwrap_or("");
            let granted = args.get("granted").and_then(|v| v.as_bool()).unwrap_or(false);
            match poly::webview::respond_to_permission(id, permission, granted) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_set_visible" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let visible = args.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
            match poly::webview::set_visible(id, visible) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_focus" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::focus(id) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_set_main_bounds" => {
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(800) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(600) as u32;
            poly::webview::set_main_bounds(poly::webview::WebViewBounds { x, y, width, height });
            serde_json::json!({"success": true}).to_string()
        }
        // MultiView API - Create windows with multiple WebViews
        "__poly_multiview_create" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Poly MultiView").to_string();
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(1024) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(768) as u32;
            let decorations = args.get("decorations").and_then(|v| v.as_bool()).unwrap_or(false);
            let resizable = args.get("resizable").and_then(|v| v.as_bool()).unwrap_or(true);
            let icon_path = args.get("icon").and_then(|v| v.as_str()).map(|s| s.to_string());
            
            let views_arr = args.get("views").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let views: Vec<poly::multiview::ViewConfig> = views_arr.iter().map(|v| {
                poly::multiview::ViewConfig {
                    id: v.get("id").and_then(|x| x.as_str()).unwrap_or("view").to_string(),
                    url: v.get("url").and_then(|x| x.as_str()).unwrap_or("about:blank").to_string(),
                    html: v.get("html").and_then(|x| x.as_str()).map(|s| s.to_string()),
                    x: v.get("x").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
                    y: v.get("y").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
                    width: v.get("width").and_then(|x| x.as_u64()).unwrap_or(800) as u32,
                    height: v.get("height").and_then(|x| x.as_u64()).unwrap_or(600) as u32,
                    transparent: v.get("transparent").and_then(|x| x.as_bool()).unwrap_or(false),
                    devtools: v.get("devtools").and_then(|x| x.as_bool()).unwrap_or(false),
                }
            }).collect();
            
            let config = poly::multiview::MultiViewWindowConfig {
                title,
                width,
                height,
                decorations,
                resizable,
                views,
                icon_path,
            };
            
            let window_id = poly::multiview::create_window(config.clone());
            
            // In native mode, actually create the window
            #[cfg(feature = "native")]
            {
                if let Err(e) = poly::multiview_native::create_multiview_window(window_id, config) {
                    return serde_json::json!({"error": e}).to_string();
                }
            }
            
            serde_json::json!({"id": window_id}).to_string()
        }
        "__poly_multiview_navigate" => {
            let window_id = args.get("windowId").and_then(|v| v.as_u64()).unwrap_or(0);
            let view_id = args.get("viewId").and_then(|v| v.as_str()).unwrap_or("");
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            
            poly::multiview::navigate(window_id, view_id, url);
            
            #[cfg(feature = "native")]
            poly::multiview_native::queue_navigation(window_id, view_id, url);
            
            serde_json::json!({"success": true}).to_string()
        }
        "__poly_multiview_post_message" => {
            let window_id = args.get("windowId").and_then(|v| v.as_u64()).unwrap_or(0);
            let view_id = args.get("viewId").and_then(|v| v.as_str()).unwrap_or("");
            let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("{}");
            
            poly::multiview::post_message(window_id, view_id, message);
            serde_json::json!({"success": true}).to_string()
        }
        "__poly_multiview_set_bounds" => {
            let window_id = args.get("windowId").and_then(|v| v.as_u64()).unwrap_or(0);
            let view_id = args.get("viewId").and_then(|v| v.as_str()).unwrap_or("");
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(800) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(600) as u32;
            
            poly::multiview::set_view_bounds(window_id, view_id, x, y, width, height);
            serde_json::json!({"success": true}).to_string()
        }
        "__poly_multiview_close" => {
            let window_id = args.get("windowId").and_then(|v| v.as_u64()).unwrap_or(0);
            poly::multiview::close_window(window_id);
            serde_json::json!({"success": true}).to_string()
        }
        "__poly_multiview_list" => {
            let windows = poly::multiview::list_windows();
            let list: Vec<serde_json::Value> = windows.iter().map(|w| {
                serde_json::json!({
                    "id": w.id,
                    "title": w.title,
                    "views": w.views
                })
            }).collect();
            serde_json::json!({"windows": list}).to_string()
        }
        "__poly_multiview_get" => {
            let window_id = args.get("windowId").and_then(|v| v.as_u64()).unwrap_or(0);
            match poly::multiview::get_window(window_id) {
                Some(w) => serde_json::json!({
                    "id": w.id,
                    "title": w.title,
                    "views": w.views
                }).to_string(),
                None => serde_json::json!({"error": "Window not found"}).to_string(),
            }
        }
        _ => serde_json::json!({"error": format!("Unknown system API: {}", fn_name)}).to_string(),
    }
}

/// Get common directory paths
fn dirs_path(name: &str) -> Option<String> {
    match name {
        "home" => std::env::var("HOME").ok()
            .or_else(|| std::env::var("USERPROFILE").ok()),
        "data" | "appData" => {
            #[cfg(target_os = "windows")]
            { std::env::var("APPDATA").ok() }
            #[cfg(target_os = "macos")]
            { std::env::var("HOME").ok().map(|h| format!("{}/Library/Application Support", h)) }
            #[cfg(target_os = "linux")]
            { std::env::var("XDG_DATA_HOME").ok().or_else(|| std::env::var("HOME").ok().map(|h| format!("{}/.local/share", h))) }
        }
        "config" => {
            #[cfg(target_os = "windows")]
            { std::env::var("APPDATA").ok() }
            #[cfg(target_os = "macos")]
            { std::env::var("HOME").ok().map(|h| format!("{}/Library/Preferences", h)) }
            #[cfg(target_os = "linux")]
            { std::env::var("XDG_CONFIG_HOME").ok().or_else(|| std::env::var("HOME").ok().map(|h| format!("{}/.config", h))) }
        }
        "cache" => {
            #[cfg(target_os = "windows")]
            { std::env::var("LOCALAPPDATA").ok().map(|p| format!("{}\\Temp", p)) }
            #[cfg(target_os = "macos")]
            { std::env::var("HOME").ok().map(|h| format!("{}/Library/Caches", h)) }
            #[cfg(target_os = "linux")]
            { std::env::var("XDG_CACHE_HOME").ok().or_else(|| std::env::var("HOME").ok().map(|h| format!("{}/.cache", h))) }
        }
        "desktop" => {
            #[cfg(target_os = "windows")]
            { std::env::var("USERPROFILE").ok().map(|h| format!("{}\\Desktop", h)) }
            #[cfg(not(target_os = "windows"))]
            { std::env::var("HOME").ok().map(|h| format!("{}/Desktop", h)) }
        }
        "documents" => {
            #[cfg(target_os = "windows")]
            { std::env::var("USERPROFILE").ok().map(|h| format!("{}\\Documents", h)) }
            #[cfg(not(target_os = "windows"))]
            { std::env::var("HOME").ok().map(|h| format!("{}/Documents", h)) }
        }
        "downloads" => {
            #[cfg(target_os = "windows")]
            { std::env::var("USERPROFILE").ok().map(|h| format!("{}\\Downloads", h)) }
            #[cfg(not(target_os = "windows"))]
            { std::env::var("HOME").ok().map(|h| format!("{}/Downloads", h)) }
        }
        _ => None,
    }
}

/// Handle HTTP requests
fn handle_http_request(fn_name: &str, args: &serde_json::Value) -> String {
    #[cfg(feature = "native")]
    {
        use reqwest::blocking::Client;
        use std::time::Duration;
        
        let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if url.is_empty() {
            return serde_json::json!({"error": "URL is required"}).to_string();
        }
        
        // Sovereignty check - verify HTTP permission for this domain
        if let Err(e) = poly::sovereignty::checks::http(url) {
            return serde_json::json!({"error": e}).to_string();
        }
        
        let client = Client::builder()
            .timeout(Duration::from_secs(args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30)))
            .build();
        
        let client = match client {
            Ok(c) => c,
            Err(e) => return serde_json::json!({"error": format!("Failed to create HTTP client: {}", e)}).to_string(),
        };
        
        // Build headers
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(h) = args.get("headers").and_then(|v| v.as_object()) {
            for (key, value) in h {
                if let Some(val) = value.as_str() {
                    if let (Ok(name), Ok(val)) = (
                        reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                        reqwest::header::HeaderValue::from_str(val)
                    ) {
                        headers.insert(name, val);
                    }
                }
            }
        }
        
        let method = match fn_name {
            "__poly_http_get" => "GET",
            "__poly_http_post" => "POST",
            "__poly_http_put" => "PUT",
            "__poly_http_patch" => "PATCH",
            "__poly_http_delete" => "DELETE",
            "__poly_http_request" => args.get("method").and_then(|v| v.as_str()).unwrap_or("GET"),
            _ => "GET",
        };
        
        let mut request = match method.to_uppercase().as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "PATCH" => client.patch(url),
            "DELETE" => client.delete(url),
            "HEAD" => client.head(url),
            _ => client.get(url),
        };
        
        request = request.headers(headers);
        
        // Add body for POST/PUT/PATCH
        if let Some(body) = args.get("body") {
            if body.is_string() {
                request = request.body(body.as_str().unwrap_or("").to_string());
            } else {
                request = request.json(body);
            }
        }
        
        match request.send() {
            Ok(response) => {
                let status = response.status().as_u16();
                let headers: std::collections::HashMap<String, String> = response.headers()
                    .iter()
                    .filter_map(|(k, v)| v.to_str().ok().map(|val| (k.to_string(), val.to_string())))
                    .collect();
                
                let body = response.text().unwrap_or_default();
                
                // Try to parse as JSON
                let data: serde_json::Value = serde_json::from_str(&body)
                    .unwrap_or_else(|_| serde_json::Value::String(body));
                
                serde_json::json!({
                    "result": {
                        "status": status,
                        "headers": headers,
                        "data": data
                    }
                }).to_string()
            }
            Err(e) => serde_json::json!({"error": format!("HTTP request failed: {}", e)}).to_string(),
        }
    }
    
    #[cfg(not(feature = "native"))]
    {
        let _ = (fn_name, args);
        serde_json::json!({"error": "HTTP API requires native feature"}).to_string()
    }
}

/// Database connection storage
#[cfg(feature = "native")]
static DB_CONNECTIONS: once_cell::sync::Lazy<std::sync::Mutex<std::collections::HashMap<u64, rusqlite::Connection>>> = 
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

#[cfg(feature = "native")]
static DB_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Handle database requests
fn handle_db_request(fn_name: &str, args: &serde_json::Value) -> String {
    #[cfg(feature = "native")]
    {
        use std::sync::atomic::Ordering;
        
        // Sovereignty check - verify database permission
        if let Err(e) = poly::sovereignty::checks::database() {
            return serde_json::json!({"error": e}).to_string();
        }
        
        match fn_name {
            "__poly_db_open" => {
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(":memory:");
                
                // Additional check for file path if not in-memory
                if path != ":memory:" {
                    if let Err(e) = poly::sovereignty::checks::fs_write(path) {
                        return serde_json::json!({"error": e}).to_string();
                    }
                }
                
                match rusqlite::Connection::open(path) {
                    Ok(conn) => {
                        let id = DB_COUNTER.fetch_add(1, Ordering::SeqCst);
                        DB_CONNECTIONS.lock().unwrap().insert(id, conn);
                        serde_json::json!({"result": {"id": id}}).to_string()
                    }
                    Err(e) => serde_json::json!({"error": format!("Failed to open database: {}", e)}).to_string(),
                }
            }
            "__poly_db_close" => {
                let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                
                if DB_CONNECTIONS.lock().unwrap().remove(&id).is_some() {
                    serde_json::json!({"result": true}).to_string()
                } else {
                    serde_json::json!({"error": "Database connection not found"}).to_string()
                }
            }
            "__poly_db_execute" => {
                let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                let sql = args.get("sql").and_then(|v| v.as_str()).unwrap_or("");
                let params = args.get("params").and_then(|v| v.as_array());
                
                let connections = DB_CONNECTIONS.lock().unwrap();
                if let Some(conn) = connections.get(&id) {
                    let params_vec: Vec<Box<dyn rusqlite::ToSql>> = params
                        .map(|arr| arr.iter().map(json_to_sql_param).collect())
                        .unwrap_or_default();
                    
                    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
                    
                    match conn.execute(sql, params_refs.as_slice()) {
                        Ok(rows) => serde_json::json!({"result": {"changes": rows}}).to_string(),
                        Err(e) => serde_json::json!({"error": format!("SQL error: {}", e)}).to_string(),
                    }
                } else {
                    serde_json::json!({"error": "Database connection not found"}).to_string()
                }
            }
            "__poly_db_query" => {
                let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                let sql = args.get("sql").and_then(|v| v.as_str()).unwrap_or("");
                let params = args.get("params").and_then(|v| v.as_array());
                
                let connections = DB_CONNECTIONS.lock().unwrap();
                if let Some(conn) = connections.get(&id) {
                    let params_vec: Vec<Box<dyn rusqlite::ToSql>> = params
                        .map(|arr| arr.iter().map(json_to_sql_param).collect())
                        .unwrap_or_default();
                    
                    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
                    
                    match conn.prepare(sql) {
                        Ok(mut stmt) => {
                            let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
                            
                            match stmt.query(params_refs.as_slice()) {
                                Ok(mut rows) => {
                                    let mut results: Vec<serde_json::Value> = Vec::new();
                                    
                                    while let Ok(Some(row)) = rows.next() {
                                        let mut obj = serde_json::Map::new();
                                        for (i, name) in column_names.iter().enumerate() {
                                            let value = sql_value_to_json(row, i);
                                            obj.insert(name.clone(), value);
                                        }
                                        results.push(serde_json::Value::Object(obj));
                                    }
                                    
                                    serde_json::json!({"result": results}).to_string()
                                }
                                Err(e) => serde_json::json!({"error": format!("Query error: {}", e)}).to_string(),
                            }
                        }
                        Err(e) => serde_json::json!({"error": format!("Prepare error: {}", e)}).to_string(),
                    }
                } else {
                    serde_json::json!({"error": "Database connection not found"}).to_string()
                }
            }
            "__poly_db_query_one" => {
                let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                let sql = args.get("sql").and_then(|v| v.as_str()).unwrap_or("");
                let params = args.get("params").and_then(|v| v.as_array());
                
                let connections = DB_CONNECTIONS.lock().unwrap();
                if let Some(conn) = connections.get(&id) {
                    let params_vec: Vec<Box<dyn rusqlite::ToSql>> = params
                        .map(|arr| arr.iter().map(json_to_sql_param).collect())
                        .unwrap_or_default();
                    
                    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
                    
                    match conn.prepare(sql) {
                        Ok(mut stmt) => {
                            let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
                            
                            match stmt.query_row(params_refs.as_slice(), |row| {
                                let mut obj = serde_json::Map::new();
                                for (i, name) in column_names.iter().enumerate() {
                                    let value = sql_value_to_json(row, i);
                                    obj.insert(name.clone(), value);
                                }
                                Ok(serde_json::Value::Object(obj))
                            }) {
                                Ok(result) => serde_json::json!({"result": result}).to_string(),
                                Err(rusqlite::Error::QueryReturnedNoRows) => serde_json::json!({"result": null}).to_string(),
                                Err(e) => serde_json::json!({"error": format!("Query error: {}", e)}).to_string(),
                            }
                        }
                        Err(e) => serde_json::json!({"error": format!("Prepare error: {}", e)}).to_string(),
                    }
                } else {
                    serde_json::json!({"error": "Database connection not found"}).to_string()
                }
            }
            _ => serde_json::json!({"error": "Unknown database operation"}).to_string(),
        }
    }
    
    #[cfg(not(feature = "native"))]
    {
        let _ = (fn_name, args);
        serde_json::json!({"error": "Database API requires native feature"}).to_string()
    }
}

/// Convert JSON value to SQL parameter
#[cfg(feature = "native")]
fn json_to_sql_param(value: &serde_json::Value) -> Box<dyn rusqlite::ToSql> {
    match value {
        serde_json::Value::Null => Box::new(rusqlite::types::Null),
        serde_json::Value::Bool(b) => Box::new(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Box::new(i)
            } else if let Some(f) = n.as_f64() {
                Box::new(f)
            } else {
                Box::new(n.to_string())
            }
        }
        serde_json::Value::String(s) => Box::new(s.clone()),
        _ => Box::new(value.to_string()),
    }
}

/// Convert SQL value to JSON
#[cfg(feature = "native")]
fn sql_value_to_json(row: &rusqlite::Row, idx: usize) -> serde_json::Value {
    // Try different types
    if let Ok(v) = row.get::<_, i64>(idx) {
        return serde_json::Value::Number(v.into());
    }
    if let Ok(v) = row.get::<_, f64>(idx) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.get::<_, String>(idx) {
        return serde_json::Value::String(v);
    }
    if let Ok(v) = row.get::<_, Vec<u8>>(idx) {
        return serde_json::Value::String(base64_encode(&v));
    }
    if let Ok(v) = row.get::<_, bool>(idx) {
        return serde_json::Value::Bool(v);
    }
    serde_json::Value::Null
}

/// Simple base64 encoding for blob data
#[cfg(feature = "native")]
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        
        result.push(CHARS[b0 >> 2] as char);
        result.push(CHARS[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        
        if chunk.len() > 1 {
            result.push(CHARS[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }
        
        if chunk.len() > 2 {
            result.push(CHARS[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }
    result
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


fn run_app_result(path: &str, release: bool, native: bool, browser: bool, ui_height: u32) -> Result<(), String> {
    let project_path = Path::new(path);
    
    // If native mode, try to run in a WebView window
    if native {
        run_native_app(project_path, release, browser, ui_height);
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
    println!("  {}POLY{} v{}  {}{}{}", CYAN, RESET, VERSION, DIM, if release { "release" } else { "debug" }, RESET);
    println!();
    
    let start = std::time::Instant::now();
    let source = fs::read_to_string(&entry)
        .map_err(|e| format!("{}", e))?;
    poly::run(&source)?;
    println!("\n  {}done{} in {}ms", GREEN, RESET, start.elapsed().as_millis());
    Ok(())
}

/// Open a URL in a standalone Poly WebView window
fn open_url_window(url: &str, title: &str, width: u32, height: u32) {
    #[cfg(feature = "native")]
    {
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
        
        let config = poly::NativeConfig::new(title)
            .with_size(width, height)
            .with_decorations(true)
            .with_dev_tools(true);
        
        if let Err(e) = poly::run_native_url(url, config) {
            eprintln!("{}error{}: Failed to open URL window: {}", RED, RESET, e);
        }
    }
    
    #[cfg(not(feature = "native"))]
    {
        eprintln!("{}error{}: Native feature not enabled", RED, RESET);
        let _ = (url, title, width, height); // Suppress unused warnings
    }
}

/// Run browser mode with separate UI and content WebViews
fn run_browser_mode(url: &str, title: &str, width: u32, height: u32, ui_height: u32, ui_html_path: Option<String>) {
    #[cfg(feature = "native")]
    {
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
        
        // Load UI HTML from file or use default
        let ui_html = if let Some(path) = ui_html_path {
            std::fs::read_to_string(&path).unwrap_or_else(|_| default_browser_ui())
        } else {
            default_browser_ui()
        };
        
        let config = poly::BrowserConfig {
            title: title.to_string(),
            width,
            height,
            ui_height,
            ui_html,
            start_url: url.to_string(),
            devtools: true,
            icon_path: None,
            decorations: false,
        };
        
        if let Err(e) = poly::run_browser_window(config) {
            eprintln!("{}error{}: Failed to run browser mode: {}", RED, RESET, e);
        }
    }
    
    #[cfg(not(feature = "native"))]
    {
        eprintln!("{}error{}: Native feature not enabled", RED, RESET);
        let _ = (url, title, width, height, ui_height, ui_html_path);
    }
}

/// Default browser UI HTML
#[allow(dead_code)]
fn default_browser_ui() -> String {
    // Minimal default - apps should provide their own UI via --ui-html
    r#"<!DOCTYPE html>
<html>
<head>
<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
body { background: #1a1a1f; height: 100%; display: flex; align-items: center; justify-content: center; -webkit-app-region: drag; }
.msg { color: #666; font-family: system-ui; font-size: 12px; }
</style>
</head>
<body><div class="msg">No UI provided. Use --ui-html to specify custom UI.</div></body>
</html>"#.to_string()
}

#[allow(unused_variables)]
fn run_native_app(project_path: &Path, _release: bool, browser_flag: bool, ui_height_arg: u32) {
    use std::sync::Arc;
    use std::thread;
    
    // Check if running from bundle (bundle folder exists)
    let bundle_dir = project_path.join("bundle");
    let is_bundled = bundle_dir.exists();
    let effective_path = if is_bundled { &bundle_dir } else { project_path };
    
    // Find web directory
    let web_dir = if effective_path.is_file() {
        effective_path.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        let web_path = effective_path.join("web");
        if web_path.exists() { web_path }
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
                let url_path = url.trim_start_matches('/');
                
                // Debug endpoint
                if url_path == "__poly_debug" {
                    let debug_info = format!("web_dir: {:?}\npackages_dir: {:?}", web_dir_clone, packages_dir_clone);
                    let _ = request.respond(tiny_http::Response::from_string(debug_info));
                    continue;
                }
                
                // Try to find the file: web/ first, then packages/
                let file_path = if url_path.is_empty() || url_path == "index.html" {
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
                
                let response = if file_path.exists() && file_path.is_file() {
                    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    let content_type = match ext {
                        "html" => "text/html; charset=utf-8",
                        "css" => "text/css; charset=utf-8",
                        "js" => "application/javascript; charset=utf-8",
                        "png" => "image/png",
                        "jpg" | "jpeg" => "image/jpeg",
                        "svg" => "image/svg+xml",
                        "json" => "application/json",
                        _ => "text/plain",
                    };
                    
                    if matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "ico") {
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
    
    // Initialize SovereigntyEngine from poly.toml
    if poly_toml_path.exists() {
        poly::sovereignty::init_from_toml(&poly_toml_path, &title);
        if poly::sovereignty::is_enabled() {
            println!("  {}>{} SovereigntyEngine: {}enabled{}", DIM, RESET, GREEN, RESET);
        }
    } else {
        // No poly.toml = development mode, sovereignty disabled
        poly::sovereignty::set_development_mode();
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
    if single_instance {
        if !poly::check_single_instance(&title) {
            println!("  {}>{} Another instance is already running", YELLOW, RESET);
            std::process::exit(0);
        }
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
    
    let mut config = poly::NativeConfig::new(&title)
        .with_size(window_width, window_height)
        .with_decorations(window_decorations)
        .with_transparent(window_transparent)
        .with_dev_tools(true)
        .with_tray(tray_enabled)
        .with_minimize_to_tray(minimize_to_tray)
        .with_close_to_tray(close_to_tray)
        .with_single_instance(single_instance);
    
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
    use std::sync::atomic::{AtomicU64, Ordering};
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
    let project_path_for_watcher = project_path.to_path_buf();
    let project_path_owned = project_path.to_path_buf();
    let entry_path = find_entry_point(&project_path_owned);
    let entry_path_for_init = entry_path.clone();
    let entry_path_for_server = entry_path.clone();
    let decorations_for_server = window_decorations;
    
    // Initialize interpreter with source
    if let Some(ref entry) = entry_path_for_init {
        let source = fs::read_to_string(entry).unwrap_or_default();
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
            
            // Hot reload endpoint
            let response = if url == "/__poly_reload" {
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
                    // Check sovereignty
                    let allowed = if poly::sovereignty_enabled() {
                        let domain = target_url.split("://")
                            .nth(1)
                            .and_then(|s| s.split('/').next())
                            .unwrap_or(&target_url);
                        let perm = poly::Permission::HttpConnect(poly::sovereignty::DomainScope::Domain(domain.to_string()));
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
                    handle_ipc_invoke_stateful(&interpreter_server, &body)
                } else {
                    // No entry point, only handle system APIs
                    handle_ipc_invoke_system_only(&body)
                };
                
                tiny_http::Response::from_string(result)
                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
            } else {
                // Try web dir first, then packages dir
                let url_path = url.trim_start_matches('/');
                let file_path = if url == "/" || url == "/index.html" {
                    web_dir_server.join("index.html")
                } else if url_path.starts_with("packages/") {
                    // Serve from packages directory (project root)
                    project_path_owned.join(url_path)
                } else {
                    web_dir_server.join(url_path)
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
                eprintln!("  {}warning{}: Could not start file watcher: {}", "\x1b[33m", RESET, e);
                return;
            }
        };
        
        // Watch the web directory within the project
        let web_path = project_path_for_watcher.join("web");
        let watch_path = if web_path.exists() { web_path } else { project_path_for_watcher.clone() };
        
        // Also watch src directory for .poly files
        let src_path = project_path_for_watcher.join("src");
        
        if let Err(e) = watcher.watch(&watch_path, RecursiveMode::Recursive) {
            eprintln!("  {}warning{}: Could not watch directory: {}", "\x1b[33m", RESET, e);
        }
        
        if src_path.exists() {
            if let Err(e) = watcher.watch(&src_path, RecursiveMode::Recursive) {
                eprintln!("  {}warning{}: Could not watch src directory: {}", "\x1b[33m", RESET, e);
            }
        }
        
        loop {
            match rx.recv() {
                Ok(Ok(event)) => {
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
                Ok(Err(_)) | Err(_) => {}
            }
        }
    });
    
    // Give server time to start
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Standard mode: Single WebView - App handles everything via APIs
    let url = format!("http://localhost:{}", port);
    
    if let Err(e) = poly::run_native_url(&url, config) {
        eprintln!("{}error{}: {}", RED, RESET, e);
        eprintln!();
        eprintln!("  {}hint{}: Build with native feature enabled:", DIM, RESET);
        eprintln!("    cargo build -p poly --features native");
        std::process::exit(1);
    }
}

fn build_app(path: &str, target: &str, release: bool, installer: bool, sign: bool) {
    let project_path = Path::new(path);
    
    // Check if it's the old-style target (web, native, all)
    match target {
        "web" | "wasm" => {
            println!();
            println!("  {}POLY{} {}build{}", CYAN, RESET, DIM, RESET);
            println!();
            build_web(project_path, release);
            println!();
            return;
        }
        "all" => {
            // Build both web and native
            println!();
            println!("  {}POLY{} {}build{}", CYAN, RESET, DIM, RESET);
            println!();
            build_web(project_path, release);
            // Fall through to native build
        }
        _ => {}
    }
    
    // Use new cross-platform build system
    let platform = build::Platform::from_str(target).unwrap_or(build::Platform::Current);
    
    let config = build::BuildConfig {
        project_path: project_path.to_path_buf(),
        platform,
        release,
        bundle: true,
        installer,
        sign,
    };
    
    if let Err(e) = build::build(&config) {
        eprintln!("{}error{}: {}", RED, RESET, e);
        std::process::exit(1);
    }
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

fn extract_toml_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(&format!("{} =", key)) || line.starts_with(&format!("{}=", key)) {
            if let Some(value) = line.split('=').nth(1) {
                return Some(value.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

/// Find a free port to use for the local server
fn find_free_port() -> Option<u16> {
    use std::net::TcpListener;
    
    // Try to bind to port 0 to get a random free port
    if let Ok(listener) = TcpListener::bind("127.0.0.1:0") {
        if let Ok(addr) = listener.local_addr() {
            return Some(addr.port());
        }
    }
    
    // Fallback: try common ports
    for port in [9473, 9474, 9475, 9476, 9477, 9478, 9479, 9480] {
        if TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok() {
            return Some(port);
        }
    }
    
    None
}


fn create_project(name: &str, template: &str) {
    println!();
    println!("  {}POLY{} v{}", CYAN, RESET, VERSION);
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
    
    // Write embedded default icon to assets/icon.png
    fs::write(project_path.join("assets/icon.png"), poly::templates::DEFAULT_ICON_PNG).ok();
    
    // poly.toml - with icon_path configured
    fs::write(project_path.join("poly.toml"), format!(r#"[package]
name = "{}"
version = "0.1.0"

[web]
dir = "web"

[window]
width = 1024
height = 768
resizable = true
icon_path = "assets/icon.png"

[dev]
# Auto-inject libraries (default: false - you control your dependencies)
# inject_alpine = true
# inject_lucide = true

# System Tray (optional)
# [tray]
# enabled = true
# tooltip = "My App"
# icon_path = "assets/icon.png"
# minimize_to_tray = false
# close_to_tray = false

# Browser Mode (dual WebView: UI + Content)
# [browser]
# ui_height = 80

# JavaScript Dependencies (managed by poly add/remove)
[dependencies]
"#, name)).ok();

    // Create .gitignore
    fs::write(project_path.join(".gitignore"), r#"/dist
/target
packages/
"#).ok();
    
    // Direct HTML/CSS/JS files (Tauri/Electron style - edit directly, hot reload works)
    match template {
        "app" => {
            // Use templates module for welcome screen
            fs::write(project_path.join("web/index.html"), poly::templates::welcome_html(name)).ok();
            fs::write(project_path.join("web/styles.css"), poly::templates::welcome_css()).ok();
            fs::write(project_path.join("web/app.js"), poly::templates::welcome_js()).ok();
            fs::write(project_path.join("src/main.poly"), poly::templates::main_poly(name)).ok();
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
    
    // .gitignore is already created above with packages/ included
    
    println!(" {}done{}", GREEN, RESET);
    println!();
    println!("  {}Next steps:{}", DIM, RESET);
    println!("    cd {}", name);
    println!("    poly dev");
    println!();
    println!("  {}Add packages:{} poly add alpinejs", DIM, RESET);
    println!("  {}Edit web/index.html directly - hot reload is automatic{}", DIM, RESET);
    println!();
}

fn init_project(_template: &str) {
    println!();
    println!("  {}POLY{} v{}", CYAN, RESET, VERSION);
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

[dev]
# Auto-inject libraries (default: false - you control your dependencies)
# inject_alpine = true
# inject_lucide = true

# JavaScript Dependencies (managed by poly add/remove)
[dependencies]
"#, name)).ok();
    }
    
    // Create .gitignore if not exists
    if !Path::new(".gitignore").exists() {
        fs::write(".gitignore", r#"/dist
/target
packages/
"#).ok();
    } else {
        // Add packages/ to existing .gitignore if not present
        packages::add_to_gitignore("packages/").ok();
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
    println!("  {}Add packages:{} poly add alpinejs", DIM, RESET);
    println!("  {}Edit web/index.html directly - hot reload is automatic{}", DIM, RESET);
    println!();
}


//! PolyView - "iframe2"
//!
//! A proxy-based iframe replacement that bypasses all iframe restrictions.
//! Works by routing all requests through a local proxy that strips blocking headers.
//!
//! Features:
//! - No X-Frame-Options blocking
//! - No CSP frame-ancestors blocking  
//! - No CORS issues
//! - Full cookie/session support
//! - URL rewriting for relative links
//! - Works in any WebView or browser

use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicU16, AtomicU64, Ordering}};
use std::thread;

// ============================================
// PolyView Proxy Server
// ============================================

static PROXY_PORT: AtomicU16 = AtomicU16::new(0);
static PROXY_STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static VIEW_COUNTER: AtomicU64 = AtomicU64::new(1);

lazy_static::lazy_static! {
    // Store cookies per domain
    static ref COOKIE_JAR: Arc<Mutex<HashMap<String, Vec<String>>>> = Arc::new(Mutex::new(HashMap::new()));
    // Store view states (URL, title, loading, etc.)
    static ref VIEW_STATES: Arc<Mutex<HashMap<u64, ViewState>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Debug, Clone)]
pub struct ViewState {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub loading: bool,
}

/// Get the proxy port (starts server if not running)
pub fn get_proxy_port() -> u16 {
    ensure_proxy_running();
    PROXY_PORT.load(Ordering::Relaxed)
}

/// Get proxy URL for a target URL
pub fn get_proxy_url(target_url: &str) -> String {
    let port = get_proxy_port();
    format!("http://localhost:{}/__polyview/?url={}", port, urlencoding::encode(target_url))
}

/// Create a new PolyView and return its ID
pub fn create_view(url: &str) -> u64 {
    let id = VIEW_COUNTER.fetch_add(1, Ordering::Relaxed);
    let state = ViewState {
        id,
        url: url.to_string(),
        title: "Loading...".to_string(),
        loading: true,
    };
    VIEW_STATES.lock().unwrap().insert(id, state);
    id
}

/// Get view state
pub fn get_view(id: u64) -> Option<ViewState> {
    VIEW_STATES.lock().unwrap().get(&id).cloned()
}

/// Update view URL
pub fn navigate_view(id: u64, url: &str) {
    if let Some(state) = VIEW_STATES.lock().unwrap().get_mut(&id) {
        state.url = url.to_string();
        state.loading = true;
    }
}

/// Close a view
pub fn close_view(id: u64) {
    VIEW_STATES.lock().unwrap().remove(&id);
}

/// Ensure proxy server is running
fn ensure_proxy_running() {
    if PROXY_STARTED.swap(true, Ordering::SeqCst) {
        return; // Already started
    }
    
    // Find free port
    let port = find_free_port().unwrap_or(19999);
    PROXY_PORT.store(port, Ordering::Relaxed);
    
    // Start proxy server in background thread
    thread::spawn(move || {
        if let Err(e) = run_proxy_server(port) {
            eprintln!("[PolyView] Proxy server error: {}", e);
            PROXY_STARTED.store(false, Ordering::SeqCst);
        }
    });
    
    // Give server time to start
    thread::sleep(std::time::Duration::from_millis(100));
    println!("[PolyView] Proxy server started on port {}", port);
}

fn find_free_port() -> Option<u16> {
    std::net::TcpListener::bind("127.0.0.1:0")
        .ok()
        .and_then(|l| l.local_addr().ok())
        .map(|a| a.port())
}

/// Run the proxy server
fn run_proxy_server(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = tiny_http::Server::http(format!("127.0.0.1:{}", port))
        .map_err(|e| format!("Failed to start proxy: {}", e))?;
    
    for request in server.incoming_requests() {
        handle_proxy_request(request);
    }
    
    Ok(())
}

/// Handle a proxy request
fn handle_proxy_request(request: tiny_http::Request) {
    let url_path = request.url().to_string();
    let method = request.method().as_str();
    
    // Handle CORS preflight requests
    if method == "OPTIONS" {
        let response = tiny_http::Response::from_string("")
            .with_status_code(204)
            .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
            .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, PUT, DELETE, OPTIONS, HEAD, PATCH"[..]).unwrap())
            .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"*"[..]).unwrap())
            .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Credentials"[..], &b"true"[..]).unwrap())
            .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Max-Age"[..], &b"86400"[..]).unwrap());
        let _ = request.respond(response);
        return;
    }
    
    // Parse the target URL from query string
    if url_path.starts_with("/__polyview/") {
        // Extract target URL
        if let Some(query_start) = url_path.find('?') {
            let query = &url_path[query_start + 1..];
            if let Some(url_param) = query.strip_prefix("url=") {
                let target_url = urlencoding::decode(url_param)
                    .unwrap_or_else(|_| url_param.into())
                    .to_string();
                
                proxy_request(request, &target_url);
                return;
            }
        }
    }
    
    // Return 404 for invalid requests
    let response = tiny_http::Response::from_string("Not Found")
        .with_status_code(404);
    let _ = request.respond(response);
}

/// Proxy a request to the target URL
fn proxy_request(request: tiny_http::Request, target_url: &str) {
    // Use ureq for HTTP requests
    let client = ureq::AgentBuilder::new()
        .redirects(0) // Handle redirects manually to rewrite them
        .build();
    
    // Build the request
    let method = request.method().as_str();
    let mut req = match method {
        "GET" => client.get(target_url),
        "POST" => client.post(target_url),
        "PUT" => client.put(target_url),
        "DELETE" => client.delete(target_url),
        "HEAD" => client.head(target_url),
        _ => client.get(target_url),
    };
    
    // Forward relevant headers
    for header in request.headers() {
        let name = header.field.as_str().as_str();
        let value = header.value.as_str();
        
        // Skip hop-by-hop headers and host
        if !matches!(name.to_lowercase().as_str(), 
            "host" | "connection" | "keep-alive" | "transfer-encoding" | 
            "te" | "trailer" | "upgrade" | "proxy-authorization" | "proxy-authenticate"
        ) {
            req = req.set(name, value);
        }
    }
    
    // Add cookies from jar
    if let Ok(url) = url::Url::parse(target_url) {
        if let Some(domain) = url.host_str() {
            if let Ok(jar) = COOKIE_JAR.lock() {
                if let Some(cookies) = jar.get(domain) {
                    let cookie_header = cookies.join("; ");
                    if !cookie_header.is_empty() {
                        req = req.set("Cookie", &cookie_header);
                    }
                }
            }
        }
    }
    
    // Make the request
    match req.call() {
        Ok(response) => {
            send_proxied_response(request, response, target_url);
        }
        Err(ureq::Error::Status(code, response)) => {
            // Handle error responses (4xx, 5xx)
            send_proxied_response_with_code(request, response, target_url, code);
        }
        Err(e) => {
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
            let response = tiny_http::Response::from_string(error_html)
                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap())
                .with_status_code(502);
            let _ = request.respond(response);
        }
    }
}

fn send_proxied_response(request: tiny_http::Request, response: ureq::Response, base_url: &str) {
    send_proxied_response_with_code(request, response, base_url, 200);
}

fn send_proxied_response_with_code(request: tiny_http::Request, response: ureq::Response, base_url: &str, status: u16) {
    let content_type = response.content_type().to_string();
    let is_html = content_type.contains("text/html");
    let is_redirect = (300..400).contains(&status);
    
    // Store cookies
    store_cookies_from_response(&response, base_url);
    
    // Handle redirects - rewrite Location header
    if is_redirect {
        if let Some(location) = response.header("Location") {
            let absolute_url = resolve_url(base_url, location);
            let proxy_port = PROXY_PORT.load(Ordering::Relaxed);
            let proxied_location = format!(
                "http://localhost:{}/__polyview/?url={}",
                proxy_port,
                urlencoding::encode(&absolute_url)
            );
            
            let resp = tiny_http::Response::from_string("")
                .with_status_code(status)
                .with_header(tiny_http::Header::from_bytes(&b"Location"[..], proxied_location.as_bytes()).unwrap());
            let _ = request.respond(resp);
            return;
        }
    }
    
    // Read body
    let body = if is_html {
        // For HTML, rewrite URLs
        let mut body_str = String::new();
        let reader = response.into_reader();
        use std::io::Read;
        let _ = reader.take(50_000_000).read_to_string(&mut body_str);
        rewrite_html(&body_str, base_url).into_bytes()
    } else {
        // For other content, pass through
        let mut body = Vec::new();
        let reader = response.into_reader();
        use std::io::Read;
        let _ = reader.take(50_000_000).read_to_end(&mut body);
        body
    };
    
    // Build response - strip ALL blocking headers and add permissive ones
    let mut resp = tiny_http::Response::from_data(body)
        .with_status_code(status);
    
    // Set content type
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap()
    );
    
    // === CORS Headers - Allow everything ===
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap()
    );
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, PUT, DELETE, OPTIONS, HEAD"[..]).unwrap()
    );
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"*"[..]).unwrap()
    );
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Credentials"[..], &b"true"[..]).unwrap()
    );
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"Access-Control-Expose-Headers"[..], &b"*"[..]).unwrap()
    );
    
    // === Frame Options - Allow all framing ===
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"X-Frame-Options"[..], &b"ALLOWALL"[..]).unwrap()
    );
    
    // === CSP - Completely permissive ===
    // This overrides any CSP from the original response
    resp = resp.with_header(
        tiny_http::Header::from_bytes(
            &b"Content-Security-Policy"[..], 
            &b"default-src * 'unsafe-inline' 'unsafe-eval' data: blob:; frame-ancestors *; script-src * 'unsafe-inline' 'unsafe-eval'; style-src * 'unsafe-inline'; img-src * data: blob:; font-src * data:; connect-src *; media-src * blob:; object-src *; frame-src *;"[..]
        ).unwrap()
    );
    
    // Remove X-Content-Type-Options to allow content sniffing
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"X-Content-Type-Options"[..], &b"nosniff"[..]).unwrap()
    );
    
    let _ = request.respond(resp);
}

/// Store cookies from response
fn store_cookies_from_response(response: &ureq::Response, base_url: &str) {
    if let Ok(url) = url::Url::parse(base_url) {
        if let Some(domain) = url.host_str() {
            // Get all Set-Cookie headers
            let cookies: Vec<String> = response.headers_names()
                .iter()
                .filter(|name| name.eq_ignore_ascii_case("set-cookie"))
                .filter_map(|name| response.header(name))
                .map(|v| {
                    // Extract just the cookie name=value part
                    v.split(';').next().unwrap_or(v).to_string()
                })
                .collect();
            
            if !cookies.is_empty() {
                if let Ok(mut jar) = COOKIE_JAR.lock() {
                    let entry = jar.entry(domain.to_string()).or_insert_with(Vec::new);
                    for cookie in cookies {
                        // Update or add cookie
                        let cookie_name = cookie.split('=').next().unwrap_or("");
                        entry.retain(|c| !c.starts_with(&format!("{}=", cookie_name)));
                        entry.push(cookie);
                    }
                }
            }
        }
    }
}

/// Rewrite HTML to enable iframe2 functionality
/// OPTIMIZED: Inject base-tag, client-script, and only rewrite navigation links (not static resources)
fn rewrite_html(html: &str, base_url: &str) -> String {
    let proxy_port = PROXY_PORT.load(Ordering::Relaxed);
    let proxy_base = format!("http://localhost:{}/__polyview/?url=", proxy_port);
    
    let mut result = html.to_string();
    
    // For Steam sites, inject application_config FIRST (before any other scripts)
    let is_steam = base_url.contains("steampowered.com") || base_url.contains("steamcommunity.com");
    if is_steam {
        let steam_config = r#"<script id="application_config" type="application/json">{"LANGUAGE":"german","COUNTRY":"DE","MEDIA_CDN_COMMUNITY_URL":"https://steamcommunity-a.akamaihd.net/","MEDIA_CDN_URL":"https://steamcdn-a.akamaihd.net/","STORE_BASE_URL":"https://store.steampowered.com/","COMMUNITY_BASE_URL":"https://steamcommunity.com/","IN_CLIENT":false,"USE_POPUPS":false,"EUNIVERSE":1,"WEBAPI_BASE_URL":"https://api.steampowered.com/","PUBLIC_SHARED_URL":"https://steamcommunity.com/public/","CHAT_BASE_URL":"https://steamloopback.host","LOGININFO_URL":"https://steamcommunity.com/login/home/"}</script>"#;
        result = format!("{}{}", steam_config, result);
    }
    
    // CRITICAL: Inject PolyView client script at the ABSOLUTE START of the document
    // This MUST be BEFORE <!DOCTYPE html> to ensure XMLHttpRequest is replaced
    // before any external scripts (like jQuery) can capture the original
    let client_script = get_polyview_client_script(base_url, proxy_port);
    result = format!("{}{}", client_script, result);
    
    // Inject base tag for relative URLs - this makes static resources load directly!
    let base_tag = format!(r#"<base href="{}">"#, base_url);
    if let Some(head_pos) = result.to_lowercase().find("<head") {
        if let Some(close_pos) = result[head_pos..].find('>') {
            let insert_pos = head_pos + close_pos + 1;
            result.insert_str(insert_pos, &base_tag);
        }
    }
    
    // Rewrite href attributes for navigation links (but NOT src for static resources)
    result = rewrite_navigation_links(&result, base_url, &proxy_base);
    
    // Rewrite form action attributes
    result = rewrite_attribute(&result, "action", base_url, &proxy_base);
    
    result
}

/// Rewrite only navigation links (href), skipping static resources
fn rewrite_navigation_links(html: &str, base_url: &str, proxy_base: &str) -> String {
    let mut result = String::with_capacity(html.len() * 2);
    let mut remaining = html;
    
    let patterns = [
        r#"href=""#.to_string(),
        r#"href='"#.to_string(),
    ];
    
    // Static extensions to skip
    let static_extensions = [".js", ".css", ".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".ico", 
                            ".woff", ".woff2", ".ttf", ".eot", ".mp3", ".mp4", ".webm", ".json", ".xml"];
    
    while !remaining.is_empty() {
        let mut found = false;
        
        for pattern in &patterns {
            if let Some(pos) = remaining.to_lowercase().find(&pattern.to_lowercase()) {
                result.push_str(&remaining[..pos]);
                
                let after_attr = &remaining[pos + pattern.len()..];
                let quote = if pattern.ends_with('"') { '"' } else { '\'' };
                let end_pos = after_attr.find(quote).unwrap_or(after_attr.len());
                let url_value = &after_attr[..end_pos];
                
                // Check if this is a static resource or special URL
                let lower_url = url_value.to_lowercase();
                let is_static = static_extensions.iter().any(|ext| lower_url.contains(ext))
                    || lower_url.starts_with('#')
                    || lower_url.starts_with("javascript:")
                    || lower_url.starts_with("data:")
                    || lower_url.starts_with("mailto:")
                    || lower_url.starts_with("tel:")
                    || lower_url.starts_with("steam://")
                    || lower_url.contains("/__polyview/")
                    || lower_url.contains("steamstatic.com")
                    || lower_url.contains("akamaihd.net");
                
                if !is_static && !url_value.is_empty() {
                    let absolute_url = resolve_url(base_url, url_value);
                    let proxied_url = format!("{}{}", proxy_base, urlencoding::encode(&absolute_url));
                    result.push_str(&format!("href=\"{}", proxied_url));
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

/// Rewrite a specific attribute in HTML
fn rewrite_attribute(html: &str, attr: &str, base_url: &str, proxy_base: &str) -> String {
    let mut result = String::with_capacity(html.len() * 2);
    let mut remaining = html;
    
    let patterns = [
        format!(r#"{}=""#, attr),
        format!(r#"{}='"#, attr),
        format!("{}=", attr),
    ];
    
    while !remaining.is_empty() {
        let mut found = false;
        
        for pattern in &patterns {
            if let Some(pos) = remaining.to_lowercase().find(&pattern.to_lowercase()) {
                // Add everything before the attribute
                result.push_str(&remaining[..pos]);
                
                let after_attr = &remaining[pos + pattern.len()..];
                let quote = if pattern.ends_with('"') {
                    '"'
                } else if pattern.ends_with('\'') {
                    '\''
                } else {
                    ' '
                };
                
                // Find end of attribute value
                let end_pos = if quote == ' ' {
                    after_attr.find(|c: char| c.is_whitespace() || c == '>').unwrap_or(after_attr.len())
                } else {
                    after_attr.find(quote).unwrap_or(after_attr.len())
                };
                
                let url_value = &after_attr[..end_pos];
                
                // Decide if we should proxy this URL
                let should_proxy = !url_value.starts_with('#') 
                    && !url_value.starts_with("javascript:")
                    && !url_value.starts_with("data:")
                    && !url_value.starts_with("mailto:")
                    && !url_value.starts_with("tel:")
                    && !url_value.contains("/__polyview/");
                
                if should_proxy && !url_value.is_empty() {
                    let absolute_url = resolve_url(base_url, url_value);
                    let proxied_url = format!("{}{}", proxy_base, urlencoding::encode(&absolute_url));
                    
                    result.push_str(&format!("{}=\"{}", attr, proxied_url));
                } else {
                    result.push_str(&remaining[pos..pos + pattern.len()]);
                    result.push_str(url_value);
                }
                
                if quote != ' ' {
                    result.push(quote);
                    remaining = &after_attr[end_pos + 1..];
                } else {
                    remaining = &after_attr[end_pos..];
                }
                
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

/// Resolve a relative URL against a base URL
fn resolve_url(base: &str, relative: &str) -> String {
    if relative.starts_with("http://") || relative.starts_with("https://") || relative.starts_with("//") {
        if relative.starts_with("//") {
            // Protocol-relative URL
            if base.starts_with("https://") {
                format!("https:{}", relative)
            } else {
                format!("http:{}", relative)
            }
        } else {
            relative.to_string()
        }
    } else if let Ok(base_url) = url::Url::parse(base) {
        base_url.join(relative)
            .map(|u| u.to_string())
            .unwrap_or_else(|_| relative.to_string())
    } else {
        relative.to_string()
    }
}

/// Get the PolyView client-side JavaScript
fn get_polyview_client_script(base_url: &str, proxy_port: u16) -> String {
    format!(r#"
<script id="polyview-client">
(function() {{
    'use strict';
    
    // PolyView Client (iframe2) - Advanced injection for full compatibility
    const PROXY_PORT = {};
    const PROXY_BASE = 'http://localhost:' + PROXY_PORT + '/__polyview/?url=';
    const BASE_URL = '{}';
    const IS_STEAM = BASE_URL.includes('steampowered.com') || BASE_URL.includes('steamcommunity.com');
    
    console.log('[PolyView] iframe2 client loaded (optimized) for:', BASE_URL);
    
    // === CROSS-ORIGIN SAFETY WRAPPER ===
    // Prevent errors when accessing parent/top from sandboxed context
    const safePostMessage = function(data) {{
        try {{
            if (window.parent && window.parent !== window) {{
                window.parent.postMessage(data, '*');
            }}
        }} catch(e) {{
            // Silently ignore cross-origin errors
        }}
    }};
    
    // === MOCK MISSING STEAM APIs ===
    if (IS_STEAM) {{
        // Mock application_config if missing (Steam expects this)
        if (!document.getElementById('application_config')) {{
            const configScript = document.createElement('script');
            configScript.id = 'application_config';
            configScript.type = 'application/json';
            configScript.textContent = JSON.stringify({{
                "LANGUAGE": "english",
                "COUNTRY": "US",
                "MEDIA_CDN_COMMUNITY_URL": "https://steamcommunity-a.akamaihd.net/",
                "MEDIA_CDN_URL": "https://steamcdn-a.akamaihd.net/",
                "STORE_BASE_URL": "https://store.steampowered.com/",
                "COMMUNITY_BASE_URL": "https://steamcommunity.com/",
                "IN_CLIENT": false,
                "USE_POPUPS": false,
                "EUNIVERSE": 1
            }});
            document.head.appendChild(configScript);
        }}
        
        // Mock Steam modal functions to prevent errors
        window.ShowDialog = window.ShowDialog || function() {{ return {{ Dismiss: function() {{}} }}; }};
        window.ShowConfirmDialog = window.ShowConfirmDialog || function(title, msg) {{ return Promise.resolve(true); }};
        window.ShowAlertDialog = window.ShowAlertDialog || function(title, msg) {{ alert(msg); }};
        window.ShowBlockingWaitDialog = window.ShowBlockingWaitDialog || function() {{ return {{ Dismiss: function() {{}} }}; }};
        window.DismissActiveModal = window.DismissActiveModal || function() {{}};
        
        // Mock CModal if missing
        if (typeof window.CModal === 'undefined') {{
            window.CModal = function() {{
                this.Show = function() {{}};
                this.Dismiss = function() {{}};
                this.OnDismiss = function() {{}};
            }};
        }}
        
        // Prevent modal content errors by providing stubs
        window.onModalHidden = window.onModalHidden || function() {{}};
        window.fnOnModalHidden = window.fnOnModalHidden || function() {{}};
    }}
    
    // === PARENT COMMUNICATION ===
    function reportNavigation(url) {{
        safePostMessage({{ type: 'polyview:navigate', url: url }});
    }}
    
    function reportTitle(title) {{
        safePostMessage({{ type: 'polyview:title', title: title }});
    }}
    
    function reportLoaded() {{
        safePostMessage({{
            type: 'polyview:loaded',
            url: BASE_URL,
            title: document.title
        }});
    }}
    
    // === URL REWRITING FOR DYNAMIC CONTENT ===
    // PERFORMANCE OPTIMIZATION: Only proxy HTML pages and API calls, NOT static resources
    
    // Static file extensions that should load directly (much faster!)
    const STATIC_EXTENSIONS = ['.js', '.css', '.png', '.jpg', '.jpeg', '.gif', '.webp', '.svg', '.ico', '.woff', '.woff2', '.ttf', '.eot', '.mp3', '.mp4', '.webm', '.ogg', '.json', '.xml', '.map'];
    
    // CDN domains that are safe to load directly
    const DIRECT_DOMAINS = ['steamstatic.com', 'akamaihd.net', 'cloudflare.com', 'fastly.steamstatic.com', 'steamcdn', 'steampowered.com/public/', 'steamcommunity.com/public/'];
    
    function isStaticResource(url) {{
        if (!url) return false;
        const lower = url.toLowerCase();
        // Check file extensions
        for (const ext of STATIC_EXTENSIONS) {{
            if (lower.indexOf(ext) !== -1) return true;
        }}
        // Check CDN domains
        for (const domain of DIRECT_DOMAINS) {{
            if (lower.indexOf(domain) !== -1) return true;
        }}
        return false;
    }}
    
    // Helper to check if URL needs proxying - ONLY proxy HTML pages and API calls
    function needsProxy(url) {{
        if (!url || typeof url !== 'string') return false;
        // Skip already proxied URLs
        if (url.indexOf('/__polyview/') !== -1) return false;
        // Skip localhost URLs (our proxy)
        if (url.indexOf('localhost:' + PROXY_PORT) !== -1) return false;
        // Skip special protocols
        if (url.indexOf('data:') === 0 || url.indexOf('blob:') === 0 || 
            url.indexOf('javascript:') === 0 || url.indexOf('about:') === 0) return false;
        // SKIP static resources - load them directly for speed!
        if (isStaticResource(url)) return false;
        // Only proxy HTTP/HTTPS URLs that aren't static
        if (url.indexOf('http://') === 0 || url.indexOf('https://') === 0 || url.indexOf('//') === 0) {{
            return true;
        }}
        return false;
    }}
    
    function makeProxyUrl(url) {{
        try {{
            let absoluteUrl = url;
            if (url.indexOf('http://') !== 0 && url.indexOf('https://') !== 0) {{
                if (url.indexOf('//') === 0) {{
                    absoluteUrl = 'https:' + url;
                }} else {{
                    absoluteUrl = new URL(url, BASE_URL).href;
                }}
            }}
            return PROXY_BASE + encodeURIComponent(absoluteUrl);
        }} catch(e) {{
            return null;
        }}
    }}
    
    // Intercept fetch to proxy API calls (not static resources)
    const _originalFetch = window.fetch;
    window.fetch = function(input, options) {{
        try {{
            let url = typeof input === 'string' ? input : (input && input.url);
            if (needsProxy(url)) {{
                const proxyUrl = makeProxyUrl(url);
                if (proxyUrl) {{
                    if (typeof input === 'string') {{
                        return _originalFetch.call(this, proxyUrl, options);
                    }} else {{
                        return _originalFetch.call(this, new Request(proxyUrl, input), options);
                    }}
                }}
            }}
        }} catch(e) {{ }}
        return _originalFetch.apply(this, arguments);
    }};
    
    // Intercept XMLHttpRequest - only proxy non-static resources
    const _OriginalXHR = window.XMLHttpRequest;
    function ProxiedXHR() {{
        const xhr = new _OriginalXHR();
        const _originalOpen = xhr.open;
        
        xhr.open = function(method, url) {{
            let finalUrl = url;
            try {{
                if (needsProxy(url)) {{
                    const proxyUrl = makeProxyUrl(url);
                    if (proxyUrl) finalUrl = proxyUrl;
                }}
            }} catch(e) {{ }}
            const args = Array.prototype.slice.call(arguments);
            args[1] = finalUrl;
            return _originalOpen.apply(this, args);
        }};
        
        return xhr;
    }}
    ProxiedXHR.prototype = _OriginalXHR.prototype;
    ProxiedXHR.UNSENT = 0;
    ProxiedXHR.OPENED = 1;
    ProxiedXHR.HEADERS_RECEIVED = 2;
    ProxiedXHR.LOADING = 3;
    ProxiedXHR.DONE = 4;
    window.XMLHttpRequest = ProxiedXHR;
    
    // === HISTORY API INTERCEPTION ===
    // Prevent SecurityError when scripts try to pushState/replaceState with external URLs
    const _originalPushState = history.pushState.bind(history);
    const _originalReplaceState = history.replaceState.bind(history);
    
    function safeHistoryUrl(url) {{
        if (!url) return url;
        try {{
            // If URL is absolute and external, convert to proxy URL
            if (typeof url === 'string' && (url.startsWith('http://') || url.startsWith('https://'))) {{
                // Don't modify if already localhost
                if (url.includes('localhost')) return url;
                // Convert to proxy URL to maintain same origin
                return PROXY_BASE + encodeURIComponent(url);
            }}
        }} catch(e) {{ }}
        return url;
    }}
    
    history.pushState = function(state, title, url) {{
        try {{
            return _originalPushState(state, title, safeHistoryUrl(url));
        }} catch(e) {{
            // If still fails, just update state without URL
            try {{ return _originalPushState(state, title); }} catch(e2) {{ }}
        }}
    }};
    
    history.replaceState = function(state, title, url) {{
        try {{
            return _originalReplaceState(state, title, safeHistoryUrl(url));
        }} catch(e) {{
            // If still fails, just update state without URL
            try {{ return _originalReplaceState(state, title); }} catch(e2) {{ }}
        }}
    }};
    
    // === WEBSOCKET INTERCEPTION ===
    // WebSockets can't be proxied, but handle gracefully
    const _OriginalWebSocket = window.WebSocket;
    window.WebSocket = function(url, protocols) {{
        let finalUrl = url;
        if (typeof url === 'string') {{
            // Skip Steam client local WebSocket (will fail anyway)
            if (url.includes('127.0.0.1:27060') || url.includes('localhost:27060')) {{
                // Return a mock WebSocket that doesn't throw errors
                const mockWs = {{
                    readyState: 3, // CLOSED
                    send: function() {{}},
                    close: function() {{}},
                    addEventListener: function() {{}},
                    removeEventListener: function() {{}},
                    onopen: null, onclose: null, onerror: null, onmessage: null
                }};
                setTimeout(() => {{ if (mockWs.onclose) mockWs.onclose({{ code: 1006 }}); }}, 0);
                return mockWs;
            }}
            // Resolve relative WebSocket URLs
            if (url.startsWith('/') && !url.startsWith('//')) {{
                try {{
                    const baseUrl = new URL(BASE_URL);
                    const wsProtocol = baseUrl.protocol === 'https:' ? 'wss:' : 'ws:';
                    finalUrl = wsProtocol + '//' + baseUrl.host + url;
                }} catch(e) {{ }}
            }}
        }}
        
        try {{
            if (protocols) {{
                return new _OriginalWebSocket(finalUrl, protocols);
            }}
            return new _OriginalWebSocket(finalUrl);
        }} catch(e) {{
            // Return mock on error
            return {{ readyState: 3, send: function(){{}}, close: function(){{}}, addEventListener: function(){{}}, removeEventListener: function(){{}}, onopen: null, onclose: null, onerror: null, onmessage: null }};
        }}
    }};
    window.WebSocket.prototype = _OriginalWebSocket.prototype;
    window.WebSocket.CONNECTING = 0;
    window.WebSocket.OPEN = 1;
    window.WebSocket.CLOSING = 2;
    window.WebSocket.CLOSED = 3;
    
    // === EVENTSOURCE (SSE) INTERCEPTION ===
    const _OriginalEventSource = window.EventSource;
    if (_OriginalEventSource) {{
        window.EventSource = function(url, options) {{
            let finalUrl = url;
            if (needsProxy(url)) {{
                const proxyUrl = makeProxyUrl(url);
                if (proxyUrl) finalUrl = proxyUrl;
            }}
            return new _OriginalEventSource(finalUrl, options);
        }};
        window.EventSource.prototype = _OriginalEventSource.prototype;
    }}
    
    // Also patch jQuery if it exists or when it loads
    function patchJQuery($) {{
        if (!$ || !$.ajax || $._polyviewPatched) return;
        $._polyviewPatched = true;
        
        const originalAjax = $.ajax;
        $.ajax = function(urlOrSettings, settings) {{
            let opts = typeof urlOrSettings === 'string' 
                ? {{ url: urlOrSettings, ...(settings || {{}}) }}
                : urlOrSettings;
            
            if (opts.url && needsProxy(opts.url)) {{
                const proxyUrl = makeProxyUrl(opts.url);
                if (proxyUrl) opts.url = proxyUrl;
            }}
            
            return originalAjax.call(this, opts);
        }};
        
        // Also patch $.get, $.post, $.getJSON
        ['get', 'post', 'getJSON'].forEach(function(method) {{
            if ($[method]) {{
                const original = $[method];
                $[method] = function(url, data, callback, type) {{
                    if (url && needsProxy(url)) {{
                        const proxyUrl = makeProxyUrl(url);
                        if (proxyUrl) url = proxyUrl;
                    }}
                    return original.call(this, url, data, callback, type);
                }};
            }}
        }});
    }}
    
    // Patch jQuery immediately if available
    if (window.jQuery) patchJQuery(window.jQuery);
    if (window.$) patchJQuery(window.$);
    
    // Watch for jQuery being added later
    Object.defineProperty(window, 'jQuery', {{
        get: function() {{ return this._jQuery; }},
        set: function(val) {{ 
            this._jQuery = val;
            if (val) patchJQuery(val);
        }},
        configurable: true
    }});
    Object.defineProperty(window, '$', {{
        get: function() {{ return this._$; }},
        set: function(val) {{ 
            this._$ = val;
            if (val) patchJQuery(val);
        }},
        configurable: true
    }});
    
    // === LINK CLICK INTERCEPTION ===
    // Only intercept actual navigation links, not buttons or interactive elements
    document.addEventListener('click', function(e) {{
        // Don't intercept if default was already prevented
        if (e.defaultPrevented) return;
        
        const link = e.target.closest('a');
        if (!link) return;
        
        // Get the actual href attribute (not the resolved href property)
        const hrefAttr = link.getAttribute('href');
        if (!hrefAttr) return;
        
        // Skip empty or hash-only links (often used for JS actions)
        if (!hrefAttr || hrefAttr === '#' || hrefAttr.startsWith('#')) {{
            return;
        }}
        
        // Skip javascript: links
        if (hrefAttr.startsWith('javascript:')) {{
            return;
        }}
        
        // Skip special protocols
        if (hrefAttr.startsWith('mailto:') || hrefAttr.startsWith('tel:') || 
            hrefAttr.startsWith('steam://') || hrefAttr.startsWith('data:')) {{
            return;
        }}
        
        // Skip if already proxied
        if (hrefAttr.includes('/__polyview/') || (link.href && link.href.includes('/__polyview/'))) {{
            return;
        }}
        
        // Skip static resources - let them load directly
        if (isStaticResource(hrefAttr) || isStaticResource(link.href)) {{
            return;
        }}
        
        // Skip links with onclick handlers (they handle their own behavior)
        if (link.onclick || link.getAttribute('onclick')) {{
            return;
        }}
        
        try {{
            const linkUrl = new URL(link.href || hrefAttr, BASE_URL);
            
            // For _blank links, open proxied in new window
            if (link.target === '_blank') {{
                e.preventDefault();
                e.stopPropagation();
                const proxyUrl = PROXY_BASE + encodeURIComponent(linkUrl.href);
                window.open(proxyUrl, '_blank');
                return;
            }}
            
            // For normal navigation links, go through proxy
            e.preventDefault();
            e.stopPropagation();
            const proxyUrl = PROXY_BASE + encodeURIComponent(linkUrl.href);
            window.location.href = proxyUrl;
            
        }} catch(err) {{
            // Let browser handle invalid URLs
        }}
    }}, true);
    
    // === POPUP HANDLING ===
    // Intercept window.open to either proxy or allow
    const originalOpen = window.open;
    window.open = function(url, target, features) {{
        if (url && typeof url === 'string') {{
            // For Steam login popups, allow them
            if (url.includes('steampowered.com/login') || url.includes('steamcommunity.com/login')) {{
                return originalOpen.call(this, url, target, features);
            }}
            // For other URLs, proxy them if same domain
            if (url.startsWith('http') && !url.includes('/__polyview/')) {{
                const proxyUrl = PROXY_BASE + encodeURIComponent(url);
                return originalOpen.call(this, proxyUrl, target, features);
            }}
        }}
        return originalOpen.apply(this, arguments);
    }};
    
    // === IFRAME2 NESTED IFRAME INTERCEPTION ===
    // This is what makes iframe2 special - we intercept ALL nested iframes!
    
    // Proxy iframe src when set via property
    function proxyIframeSrc(iframe, url) {{
        if (!url || typeof url !== 'string') return url;
        if (url.includes('/__polyview/')) return url;
        if (url.startsWith('about:') || url.startsWith('javascript:') || url.startsWith('data:') || url.startsWith('blob:')) return url;
        
        const proxyUrl = makeProxyUrl(url);
        if (proxyUrl) return proxyUrl;
        return url;
    }}
    
    // Intercept iframe.src setter
    const iframeProto = HTMLIFrameElement.prototype;
    const originalSrcDescriptor = Object.getOwnPropertyDescriptor(iframeProto, 'src');
    if (originalSrcDescriptor) {{
        Object.defineProperty(iframeProto, 'src', {{
            get: function() {{
                return originalSrcDescriptor.get.call(this);
            }},
            set: function(value) {{
                const proxied = proxyIframeSrc(this, value);
                return originalSrcDescriptor.set.call(this, proxied);
            }},
            configurable: true,
            enumerable: true
        }});
    }}
    
    // Intercept iframe.setAttribute for src
    const originalSetAttribute = Element.prototype.setAttribute;
    Element.prototype.setAttribute = function(name, value) {{
        if (this.tagName === 'IFRAME' && name.toLowerCase() === 'src') {{
            value = proxyIframeSrc(this, value);
        }}
        return originalSetAttribute.call(this, name, value);
    }};
    
    // Intercept document.createElement to catch new iframes
    const originalCreateElement = document.createElement.bind(document);
    document.createElement = function(tagName, options) {{
        const element = originalCreateElement(tagName, options);
        
        if (tagName.toLowerCase() === 'iframe') {{
            // Override src property on this specific iframe
            let _src = '';
            Object.defineProperty(element, 'src', {{
                get: function() {{ return _src; }},
                set: function(value) {{
                    _src = proxyIframeSrc(element, value);
                    element.setAttribute('src', _src);
                }},
                configurable: true,
                enumerable: true
            }});
        }}
        
        return element;
    }};
    
    // MutationObserver to catch iframes added via innerHTML or other means
    const iframeObserver = new MutationObserver(function(mutations) {{
        mutations.forEach(function(mutation) {{
            mutation.addedNodes.forEach(function(node) {{
                if (node.nodeType === Node.ELEMENT_NODE) {{
                    // Check if it's an iframe
                    if (node.tagName === 'IFRAME') {{
                        const currentSrc = node.getAttribute('src');
                        if (currentSrc && !currentSrc.includes('/__polyview/')) {{
                            const proxied = proxyIframeSrc(node, currentSrc);
                            if (proxied !== currentSrc) {{
                                originalSetAttribute.call(node, 'src', proxied);
                            }}
                        }}
                    }}
                    // Check for nested iframes
                    const nestedIframes = node.querySelectorAll ? node.querySelectorAll('iframe') : [];
                    nestedIframes.forEach(function(iframe) {{
                        const src = iframe.getAttribute('src');
                        if (src && !src.includes('/__polyview/')) {{
                            const proxied = proxyIframeSrc(iframe, src);
                            if (proxied !== src) {{
                                originalSetAttribute.call(iframe, 'src', proxied);
                            }}
                        }}
                    }});
                }}
            }});
        }});
    }});
    
    // Start observing
    iframeObserver.observe(document.documentElement || document.body || document, {{
        childList: true,
        subtree: true
    }});
    
    // Also proxy existing iframes on page load
    function proxyExistingIframes() {{
        document.querySelectorAll('iframe').forEach(function(iframe) {{
            const src = iframe.getAttribute('src');
            if (src && !src.includes('/__polyview/')) {{
                const proxied = proxyIframeSrc(iframe, src);
                if (proxied !== src) {{
                    originalSetAttribute.call(iframe, 'src', proxied);
                }}
            }}
        }});
    }}
    
    // Run on DOMContentLoaded and load
    if (document.readyState === 'loading') {{
        document.addEventListener('DOMContentLoaded', proxyExistingIframes);
    }} else {{
        proxyExistingIframes();
    }}
    window.addEventListener('load', proxyExistingIframes);
    
    // === IFRAME SECURITY FIX ===
    // Prevent child iframes from causing errors
    window.addEventListener('error', function(e) {{
        if (e.message && (e.message.includes('cross-origin') || e.message.includes('SecurityError'))) {{
            e.preventDefault();
            e.stopPropagation();
            return true;
        }}
    }}, true);
    
    // Suppress unhandled promise rejections from cross-origin issues
    window.addEventListener('unhandledrejection', function(e) {{
        if (e.reason && (String(e.reason).includes('cross-origin') || String(e.reason).includes('SecurityError'))) {{
            e.preventDefault();
            return true;
        }}
    }}, true);
    
    // === TITLE OBSERVER ===
    const titleObserver = new MutationObserver(function() {{
        reportTitle(document.title);
    }});
    
    if (document.querySelector('title')) {{
        titleObserver.observe(document.querySelector('title'), {{ 
            childList: true, 
            characterData: true, 
            subtree: true 
        }});
    }}
    
    // === INITIAL REPORTING ===
    reportNavigation(BASE_URL);
    
    window.addEventListener('load', function() {{
        reportLoaded();
        reportTitle(document.title);
    }});
    
    // Also report on DOMContentLoaded for faster feedback
    if (document.readyState === 'loading') {{
        document.addEventListener('DOMContentLoaded', function() {{
            reportTitle(document.title);
        }});
    }} else {{
        reportTitle(document.title);
    }}
    
}})();
</script>
"#, proxy_port, base_url)
}

// ============================================
// Public API for JavaScript
// ============================================

/// Get the JavaScript code for PolyView custom element
pub fn get_polyview_element_js() -> String {
    let port = get_proxy_port();
    format!(r#"
// PolyView Custom Element - "iframe2"
// Usage: <poly-view src="https://example.com"></poly-view>

class PolyView extends HTMLElement {{
    static get observedAttributes() {{
        return ['src'];
    }}
    
    constructor() {{
        super();
        this.attachShadow({{ mode: 'open' }});
        this._iframe = null;
        this._currentUrl = '';
        this._title = '';
        this._loading = false;
    }}
    
    connectedCallback() {{
        this.shadowRoot.innerHTML = `
            <style>
                :host {{
                    display: block;
                    width: 100%;
                    height: 100%;
                    position: relative;
                    contain: layout style;
                    overflow: hidden;
                }}
                :host([hidden]) {{
                    display: none !important;
                }}
                .iframe-container {{
                    position: absolute;
                    top: 0;
                    left: 0;
                    right: 0;
                    bottom: 0;
                    width: 100%;
                    height: 100%;
                }}
                iframe {{
                    width: 100%;
                    height: 100%;
                    border: none;
                    background: #1b2838;
                    display: block;
                }}
                .loading {{
                    position: absolute;
                    top: 0;
                    left: 0;
                    right: 0;
                    height: 3px;
                    background: linear-gradient(90deg, #1a9fff, #66c0f4, #1a9fff);
                    background-size: 200% 100%;
                    animation: shimmer 1.5s infinite linear;
                    opacity: 0;
                    transition: opacity 0.2s;
                    z-index: 10;
                }}
                .loading.active {{ opacity: 1; }}
                @keyframes shimmer {{
                    0% {{ background-position: 200% 0; }}
                    100% {{ background-position: -200% 0; }}
                }}
            </style>
            <div class="loading"></div>
            <div class="iframe-container">
                <iframe sandbox="allow-same-origin allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox allow-top-navigation allow-top-navigation-by-user-activation allow-modals allow-downloads allow-presentation"></iframe>
            </div>
        `;
        
        this._iframe = this.shadowRoot.querySelector('iframe');
        this._loadingBar = this.shadowRoot.querySelector('.loading');
        
        // Listen for messages from iframe
        window.addEventListener('message', this._handleMessage.bind(this));
        
        // Load initial src
        if (this.hasAttribute('src')) {{
            this.navigate(this.getAttribute('src'));
        }}
    }}
    
    disconnectedCallback() {{
        window.removeEventListener('message', this._handleMessage.bind(this));
    }}
    
    attributeChangedCallback(name, oldValue, newValue) {{
        if (name === 'src' && oldValue !== newValue && this._iframe) {{
            this.navigate(newValue);
        }}
    }}
    
    _handleMessage(event) {{
        if (event.source !== this._iframe?.contentWindow) return;
        
        const data = event.data;
        if (!data || !data.type?.startsWith('polyview:')) return;
        
        switch (data.type) {{
            case 'polyview:navigate':
                this._currentUrl = data.url;
                this._loading = true;
                this._loadingBar?.classList.add('active');
                this.dispatchEvent(new CustomEvent('navigate', {{ detail: {{ url: data.url }} }}));
                break;
                
            case 'polyview:title':
                this._title = data.title;
                this.dispatchEvent(new CustomEvent('titlechange', {{ detail: {{ title: data.title }} }}));
                break;
                
            case 'polyview:loaded':
                this._loading = false;
                this._loadingBar?.classList.remove('active');
                this._currentUrl = data.url;
                this._title = data.title;
                this.dispatchEvent(new CustomEvent('load', {{ detail: {{ url: data.url, title: data.title }} }}));
                break;
        }}
    }}
    
    // Public API
    navigate(url) {{
        if (!url) return;
        this._loading = true;
        this._loadingBar?.classList.add('active');
        this._currentUrl = url;
        
        const proxyUrl = `http://localhost:{}/__polyview/?url=${{encodeURIComponent(url)}}`;
        this._iframe.src = proxyUrl;
        
        this.dispatchEvent(new CustomEvent('navigate', {{ detail: {{ url }} }}));
    }}
    
    reload() {{
        if (this._currentUrl) {{
            this.navigate(this._currentUrl);
        }}
    }}
    
    goBack() {{
        this._iframe?.contentWindow?.history.back();
    }}
    
    goForward() {{
        this._iframe?.contentWindow?.history.forward();
    }}
    
    // Remove sandbox for full compatibility (use with caution)
    disableSandbox() {{
        if (this._iframe) {{
            this._iframe.removeAttribute('sandbox');
        }}
    }}
    
    // Re-enable sandbox with custom attributes
    enableSandbox(attributes = 'allow-same-origin allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox allow-top-navigation allow-modals') {{
        if (this._iframe) {{
            this._iframe.setAttribute('sandbox', attributes);
        }}
    }}
    
    // Execute JavaScript in the iframe context
    executeScript(code) {{
        try {{
            if (this._iframe?.contentWindow) {{
                return this._iframe.contentWindow.eval(code);
            }}
        }} catch(e) {{
            console.error('[PolyView] executeScript failed:', e);
        }}
        return null;
    }}
    
    // Post a message to the iframe
    postMessage(data, origin = '*') {{
        this._iframe?.contentWindow?.postMessage(data, origin);
    }}
    
    // Get the iframe element (for advanced usage)
    get iframe() {{ return this._iframe; }}
    
    // Get the content window (may be restricted by same-origin)
    get contentWindow() {{ return this._iframe?.contentWindow; }}
    
    // Get the content document (may be restricted by same-origin)  
    get contentDocument() {{
        try {{
            return this._iframe?.contentDocument;
        }} catch(e) {{
            return null;
        }}
    }}
    
    get src() {{ return this._currentUrl; }}
    set src(url) {{ this.navigate(url); }}
    
    get title() {{ return this._title; }}
    get loading() {{ return this._loading; }}
    
    // Static method to create a proxy URL
    static proxyUrl(url) {{
        return `http://localhost:{}/__polyview/?url=${{encodeURIComponent(url)}}`;
    }}
}}

customElements.define('poly-view', PolyView);

// Also expose as window.PolyView for easy access
window.PolyView = PolyView;

// iframe2 helper function - create a proxied iframe easily
window.createIframe2 = function(url, options = {{}}) {{
    const pv = document.createElement('poly-view');
    if (options.width) pv.style.width = options.width;
    if (options.height) pv.style.height = options.height;
    if (options.noSandbox) pv.disableSandbox();
    if (url) pv.src = url;
    return pv;
}};

// Export for module usage
if (typeof module !== 'undefined') {{
    module.exports = PolyView;
}}
"#, port, port)
}

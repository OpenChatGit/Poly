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
    
    // Build response - strip blocking headers
    let mut resp = tiny_http::Response::from_data(body)
        .with_status_code(status);
    
    // Set content type
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap()
    );
    
    // Allow all framing
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"X-Frame-Options"[..], &b"ALLOWALL"[..]).unwrap()
    );
    
    // Remove CSP that blocks framing
    resp = resp.with_header(
        tiny_http::Header::from_bytes(&b"Content-Security-Policy"[..], &b"frame-ancestors *"[..]).unwrap()
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

/// Rewrite HTML to proxy all URLs
fn rewrite_html(html: &str, base_url: &str) -> String {
    let proxy_port = PROXY_PORT.load(Ordering::Relaxed);
    let proxy_base = format!("http://localhost:{}/__polyview/?url=", proxy_port);
    
    let mut result = html.to_string();
    
    // Inject base tag for relative URLs that we can't catch
    let base_tag = format!(r#"<base href="{}">"#, base_url);
    if let Some(head_pos) = result.to_lowercase().find("<head") {
        if let Some(close_pos) = result[head_pos..].find('>') {
            let insert_pos = head_pos + close_pos + 1;
            result.insert_str(insert_pos, &base_tag);
        }
    }
    
    // Rewrite href attributes
    result = rewrite_attribute(&result, "href", base_url, &proxy_base);
    
    // Rewrite src attributes  
    result = rewrite_attribute(&result, "src", base_url, &proxy_base);
    
    // Rewrite action attributes (forms)
    result = rewrite_attribute(&result, "action", base_url, &proxy_base);
    
    // Inject PolyView client script
    let client_script = get_polyview_client_script(base_url, proxy_port);
    if let Some(pos) = result.to_lowercase().find("</head>") {
        result.insert_str(pos, &client_script);
    } else if let Some(pos) = result.to_lowercase().find("<body") {
        result.insert_str(pos, &client_script);
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
<script>
(function() {{
    // PolyView Client - Intercepts navigation and reports to parent
    const PROXY_BASE = 'http://localhost:{}/__polyview/?url=';
    const BASE_URL = '{}';
    
    // Report URL changes to parent
    function reportNavigation(url) {{
        if (window.parent && window.parent !== window) {{
            window.parent.postMessage({{
                type: 'polyview:navigate',
                url: url
            }}, '*');
        }}
    }}
    
    // Report title changes
    function reportTitle(title) {{
        if (window.parent && window.parent !== window) {{
            window.parent.postMessage({{
                type: 'polyview:title',
                title: title
            }}, '*');
        }}
    }}
    
    // Report load complete
    function reportLoaded() {{
        if (window.parent && window.parent !== window) {{
            window.parent.postMessage({{
                type: 'polyview:loaded',
                url: BASE_URL,
                title: document.title
            }}, '*');
        }}
    }}
    
    // Intercept link clicks
    document.addEventListener('click', function(e) {{
        const link = e.target.closest('a');
        if (link && link.href && !link.href.startsWith('javascript:')) {{
            // Let the proxy handle it
        }}
    }}, true);
    
    // Watch for title changes
    const titleObserver = new MutationObserver(function() {{
        reportTitle(document.title);
    }});
    
    if (document.querySelector('title')) {{
        titleObserver.observe(document.querySelector('title'), {{ childList: true, characterData: true, subtree: true }});
    }}
    
    // Report initial state
    window.addEventListener('load', function() {{
        reportLoaded();
        reportTitle(document.title);
    }});
    
    // Report navigation
    reportNavigation(BASE_URL);
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
                }}
                iframe {{
                    width: 100%;
                    height: 100%;
                    border: none;
                    background: #09090b;
                }}
                .loading {{
                    position: absolute;
                    top: 0;
                    left: 0;
                    right: 0;
                    height: 2px;
                    background: linear-gradient(90deg, #22d3ee, #a855f7, #22d3ee);
                    background-size: 200% 100%;
                    animation: shimmer 1.5s infinite linear;
                    opacity: 0;
                    transition: opacity 0.2s;
                }}
                .loading.active {{ opacity: 1; }}
                @keyframes shimmer {{
                    0% {{ background-position: 200% 0; }}
                    100% {{ background-position: -200% 0; }}
                }}
            </style>
            <div class="loading"></div>
            <iframe sandbox="allow-same-origin allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox"></iframe>
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
    
    get src() {{ return this._currentUrl; }}
    set src(url) {{ this.navigate(url); }}
    
    get title() {{ return this._title; }}
    get loading() {{ return this._loading; }}
}}

customElements.define('poly-view', PolyView);

// Export for module usage
if (typeof module !== 'undefined') {{
    module.exports = PolyView;
}}
"#, port)
}

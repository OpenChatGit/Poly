/// Handle PolyView proxy request - fetches URL and rewrites content to bypass iframe restrictions
/// This is "iframe2" - better than normal iframes because it bypasses ALL restrictions
pub(super) fn handle_polyview_proxy(target_url: &str, proxy_port: u16) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {


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

    // Get stored cookies for this domain (including shared Steam cookies)
    let cookie_header = poly::cookies::get_cookies_for_domain(&domain);

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

/// Store cookies from response (uses normalized domain for Steam cookie sharing)
fn store_polyview_cookies(response: &ureq::Response, domain: &str) {
    // Get all Set-Cookie headers
    for name in response.headers_names() {
        if name.eq_ignore_ascii_case("set-cookie") {
            if let Some(value) = response.header(&name) {
                // Extract just the cookie name=value part (before ;)
                let cookie = value.split(';').next().unwrap_or(value).to_string();
                let cookie_name = cookie.split('=').next().unwrap_or("");

                // Store cookie using the shared module
                poly::cookies::store_cookie(domain, &cookie);

                // Log important Steam session cookies
                if cookie_name == "steamLoginSecure" || cookie_name == "sessionid" || cookie_name == "steamRememberLogin" {
                    println!("[PolyView] Stored Steam cookie: {} for domain {}", cookie_name, domain);
                }
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
pub(super) fn handle_polyview_proxy_post(target_url: &str, proxy_port: u16, body: &[u8], content_type: &str) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {


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

    // Get stored cookies (including shared Steam cookies)
    let cookie_header = poly::cookies::get_cookies_for_domain(&domain);

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
/// OPTIMIZATION: Only proxy HTML pages and AJAX, let static resources load directly
fn rewrite_polyview_html(html: &str, base_url: &str, proxy_port: u16) -> String {
    let mut result = html.to_string();

    // Inject base tag for relative URLs - this makes static resources load directly!
    let base_tag = format!(r#"<base href="{}">"#, base_url);
    if let Some(head_pos) = result.to_lowercase().find("<head") {
        if let Some(close_pos) = result[head_pos..].find('>') {
            let insert_pos = head_pos + close_pos + 1;
            result.insert_str(insert_pos, &base_tag);
        }
    }

    // Inject PolyView client script - this is what makes iframe2 special
    // It intercepts navigation, form submissions, and reports state to parent
    let client_script = format!(r#"
<style>
/* Hide Steam's header/navigation - we have our own */
#global_header, .responsive_header, #global_action_menu,
header.responsive_header_content, .supernav_container,
#responsive_menu_logo, .header_installsteam_btn_content,
#store_header, #store_controls, #store_nav_area,
.responsive_page_menu_ctn, #footer, #footer_spacer, #footer_responsive_optin_spacer,
.responsive_page_template_content > .responsive_local_menu_tab,
.profile_header_actions, .profile_header_actions_ctn {{
    display: none !important;
}}
/* Adjust body to remove header space */
body {{
    padding-top: 0 !important;
    margin-top: 0 !important;
}}
.responsive_page_frame {{
    padding-top: 0 !important;
}}
.responsive_page_content {{
    padding-top: 0 !important;
}}
/* Make profile content full width */
.profile_page {{
    padding-top: 0 !important;
}}
</style>
<script>
(function() {{
    // PolyView Client - "iframe2" magic (optimized - only proxy navigation)
    const PROXY_BASE = 'http://localhost:{}/__polyview/?url=';
    const BASE_URL = '{}';
    const PROXY_PORT = {};

    // Report to parent window
    function reportToParent(type, data) {{
        if (window.parent && window.parent !== window) {{
            window.parent.postMessage({{ type: 'polyview:' + type, ...data }}, '*');
        }}
    }}

    // Check if URL is a page (needs proxy) vs static resource (load direct)
    function needsProxy(url) {{
        if (!url) return false;
        const lower = url.toLowerCase();
        // Static resources - load directly (faster!)
        const staticExts = ['.css', '.js', '.png', '.jpg', '.jpeg', '.gif', '.webp', '.svg', '.ico', '.woff', '.woff2', '.ttf', '.eot', '.mp3', '.mp4', '.webm', '.ogg'];
        for (const ext of staticExts) {{
            if (lower.includes(ext)) return false;
        }}
        // CDN domains - always direct
        if (lower.includes('steamstatic.com') || lower.includes('akamai') || lower.includes('cloudflare')) {{
            return false;
        }}
        return true;
    }}

    // Convert URL to proxy URL (only for pages)
    function toProxyUrl(url) {{
        if (!url || url.startsWith('javascript:') || url.startsWith('data:') ||
            url.startsWith('mailto:') || url.startsWith('tel:') || url.startsWith('#') ||
            url.includes('/__polyview/')) {{
            return url;
        }}
        // Make absolute
        try {{
            const absolute = new URL(url, BASE_URL).href;
            // Only proxy if it needs it (HTML pages)
            if (!needsProxy(absolute)) return absolute;
            return PROXY_BASE + encodeURIComponent(absolute);
        }} catch {{
            return url;
        }}
    }}

    // Intercept link clicks - only proxy page navigation
    document.addEventListener('click', function(e) {{
        const link = e.target.closest('a[href]');
        if (link) {{
            const href = link.getAttribute('href');
            if (href && !href.startsWith('javascript:') && !href.startsWith('#') &&
                !href.startsWith('mailto:') && !href.startsWith('tel:')) {{
                // Check if this is a page link that needs proxying
                try {{
                    const absolute = new URL(href, BASE_URL).href;
                    if (needsProxy(absolute)) {{
                        e.preventDefault();
                        window.location.href = PROXY_BASE + encodeURIComponent(absolute);
                    }}
                    // Otherwise let it navigate normally
                }} catch {{}}
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

    // Override fetch - only proxy API calls, not static resources
    const originalFetch = window.fetch;
    window.fetch = function(url, options) {{
        if (typeof url === 'string' && url.startsWith('http') && !url.includes('localhost')) {{
            // Only proxy if it's an API call (not static resource)
            if (needsProxy(url)) {{
                url = PROXY_BASE + encodeURIComponent(url);
            }}
        }}
        return originalFetch.call(window, url, options);
    }};

    // Override XMLHttpRequest - only proxy API calls
    const originalXHROpen = XMLHttpRequest.prototype.open;
    XMLHttpRequest.prototype.open = function(method, url, ...args) {{
        if (typeof url === 'string' && url.startsWith('http') && !url.includes('localhost')) {{
            if (needsProxy(url)) {{
                url = PROXY_BASE + encodeURIComponent(url);
            }}
        }}
        return originalXHROpen.call(this, method, url, ...args);
    }};

    console.log('[PolyView] iframe2 client loaded (optimized) for:', BASE_URL);
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

//! Ad Blocking Proxy Server
//! 
//! A local HTTP proxy that filters requests through the ad blocker.
//! This allows us to block ads at the network level without WebView API limitations.

use std::sync::Arc;
use std::net::SocketAddr;

#[cfg(feature = "native")]
use tiny_http::{Server, Response, Header};

/// Start the ad blocking proxy server
#[cfg(feature = "native")]
pub fn start_proxy(port: u16) -> Result<(), String> {
    let addr = format!("127.0.0.1:{}", port);
    let server = Server::http(&addr)
        .map_err(|e| format!("Failed to start proxy server: {}", e))?;
    
    println!("[AdBlockProxy] Started on {}", addr);
    
    // Run in background thread
    std::thread::spawn(move || {
        for request in server.incoming_requests() {
            let url = request.url().to_string();
            let method = request.method().as_str().to_string();
            
            // Check if URL should be blocked
            let blocked = crate::adblock::should_block(&url, "", "other");
            
            if blocked {
                println!("[AdBlockProxy] 🚫 Blocked: {}", url);
                // Return 403 Forbidden
                let response = Response::from_string("Blocked by ad blocker")
                    .with_status_code(403);
                let _ = request.respond(response);
                continue;
            }
            
            // Forward the request
            match forward_request(&method, &url, &request) {
                Ok(response) => {
                    let _ = request.respond(response);
                }
                Err(e) => {
                    eprintln!("[AdBlockProxy] Error forwarding request: {}", e);
                    let response = Response::from_string(format!("Proxy error: {}", e))
                        .with_status_code(502);
                    let _ = request.respond(response);
                }
            }
        }
    });
    
    Ok(())
}

#[cfg(feature = "native")]
fn forward_request(method: &str, url: &str, request: &tiny_http::Request) -> Result<Response<std::io::Cursor<Vec<u8>>>, String> {
    // Build the request
    let mut req = ureq::request(method, url);
    
    // Copy headers
    for header in request.headers() {
        req = req.set(header.field.as_str().as_str(), header.value.as_str());
    }
    
    // Send request
    let response = req.call()
        .map_err(|e| format!("Request failed: {}", e))?;
    
    // Read response body
    let mut body = Vec::new();
    response.into_reader()
        .read_to_end(&mut body)
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    // Build response
    let mut resp = Response::from_data(body);
    
    // Copy response headers
    for name in response.headers_names() {
        if let Some(value) = response.header(&name) {
            if let Ok(header) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
                resp.add_header(header);
            }
        }
    }
    
    Ok(resp)
}

#[cfg(not(feature = "native"))]
pub fn start_proxy(_port: u16) -> Result<(), String> {
    Err("Proxy not available in non-native builds".to_string())
}

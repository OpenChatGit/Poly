pub(super) fn dirs_path(name: &str) -> Option<String> {
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
pub(super) fn handle_http_request(fn_name: &str, args: &serde_json::Value) -> String {
    #[cfg(feature = "native")]
    {
        use reqwest::blocking::Client;
        use std::time::Duration;

        let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if url.is_empty() {
            return serde_json::json!({"error": "URL is required"}).to_string();
        }

        // Permission check - verify HTTP permission for this domain
        if let Err(e) = poly::permissions::checks::http(url) {
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
                    .unwrap_or(serde_json::Value::String(body));

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

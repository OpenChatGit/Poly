//! Cookie storage for Steam session management
//! 
//! Stores cookies from WebView/PolyView proxy for use in backend HTTP requests.

use std::collections::HashMap;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref COOKIES: Mutex<HashMap<String, Vec<String>>> = Mutex::new(HashMap::new());
}

/// Normalize domain for cookie sharing - Steam domains share cookies
pub fn normalize_domain(domain: &str) -> String {
    let domain_lower = domain.to_lowercase();
    
    // Steam domains should share cookies
    if domain_lower.ends_with("steampowered.com") 
        || domain_lower.ends_with("steamcommunity.com")
        || domain_lower.ends_with("steamgames.com")
        || domain_lower == "steampowered.com"
        || domain_lower == "steamcommunity.com"
        || domain_lower == "steamgames.com"
        || domain_lower.contains("store.steampowered")
        || domain_lower.contains("help.steampowered")
        || domain_lower.contains("partner.steamgames")
    {
        return "steam.shared".to_string();
    }
    
    domain.to_string()
}

/// Store a cookie for a domain
pub fn store_cookie(domain: &str, cookie: &str) {
    let normalized = normalize_domain(domain);
    let mut cookies = COOKIES.lock().unwrap();
    let entry = cookies.entry(normalized.clone()).or_insert_with(Vec::new);
    
    // Extract cookie name
    let cookie_name = cookie.split('=').next().unwrap_or("");
    
    // Update or add cookie (remove old one with same name)
    entry.retain(|c| !c.starts_with(&format!("{}=", cookie_name)));
    entry.push(cookie.to_string());
}

/// Get all cookies for a domain as a header string
pub fn get_cookies_for_domain(domain: &str) -> String {
    let cookies = COOKIES.lock().unwrap();
    let normalized = normalize_domain(domain);
    
    let mut all_cookies = Vec::new();
    
    // Get cookies for the normalized domain
    if let Some(domain_cookies) = cookies.get(&normalized) {
        all_cookies.extend(domain_cookies.clone());
    }
    
    // Also get cookies for the exact domain if different
    if normalized != domain {
        if let Some(domain_cookies) = cookies.get(domain) {
            for cookie in domain_cookies {
                if !all_cookies.contains(cookie) {
                    all_cookies.push(cookie.clone());
                }
            }
        }
    }
    
    all_cookies.join("; ")
}

/// Get Steam cookies specifically
pub fn get_steam_cookies() -> String {
    get_cookies_for_domain("steamcommunity.com")
}

/// Check if we have Steam login cookies
pub fn has_steam_login() -> bool {
    let cookies = get_steam_cookies();
    cookies.contains("steamLoginSecure")
}

/// Clear all cookies for a domain
pub fn clear_cookies(domain: &str) {
    let normalized = normalize_domain(domain);
    let mut cookies = COOKIES.lock().unwrap();
    cookies.remove(&normalized);
}

/// Clear all cookies
pub fn clear_all_cookies() {
    let mut cookies = COOKIES.lock().unwrap();
    cookies.clear();
}

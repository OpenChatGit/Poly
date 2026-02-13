//! Ad Blocking Module
//! 
//! Implements Adblock Plus filter syntax parsing and matching.
//! Compatible with EasyList, EasyPrivacy, and other ABP-style filter lists.
//! 
//! This is a simplified but functional implementation that handles the most
//! common filter types used by Brave and other ad blockers.

use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;

#[cfg(feature = "native")]
use regex::Regex;

/// Filter rule types
#[cfg(feature = "native")]
#[derive(Debug, Clone)]
enum FilterType {
    /// Block network requests
    Network {
        pattern: String,
        regex: Regex,
        options: FilterOptions,
    },
    /// Exception rule (whitelist)
    Exception {
        pattern: String,
        regex: Regex,
        options: FilterOptions,
    },
    /// Element hiding rule (cosmetic)
    ElementHiding {
        domains: Vec<String>,
        selector: String,
    },
}

/// Filter options (e.g., $script, $image, $third-party)
#[cfg(feature = "native")]
#[derive(Debug, Clone, Default)]
struct FilterOptions {
    /// Resource types to match
    types: Vec<String>,
    /// Domains to apply the filter to
    domains: Vec<String>,
    /// Domains to exclude
    exclude_domains: Vec<String>,
    /// Third-party only
    third_party: Option<bool>,
    /// Match case
    match_case: bool,
}

/// Global ad blocker with parsed rules
#[cfg(feature = "native")]
pub static AD_BLOCKER: Lazy<Arc<RwLock<Vec<FilterType>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(Vec::new()))
});

/// Statistics
#[cfg(feature = "native")]
pub static AD_BLOCK_STATS: Lazy<Arc<RwLock<(usize, usize)>>> = Lazy::new(|| {
    Arc::new(RwLock::new((0, 0))) // (blocked, total)
});

/// Initialize the ad blocker with filter lists
#[cfg(feature = "native")]
pub fn init_adblock(filter_lists: Vec<String>) -> Result<(), String> {
    let mut rules = Vec::new();
    let filter_count = filter_lists.len();
    let mut rule_count = 0;
    
    // Parse filter rules
    for filter_list in &filter_lists {
        for line in filter_list.lines() {
            let line = line.trim();
            
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
                continue;
            }
            
            // Parse the rule
            if let Some(rule) = parse_filter_rule(line) {
                rules.push(rule);
                rule_count += 1;
            }
        }
    }
    
    // Store in global state
    let mut blocker = AD_BLOCKER.write()
        .map_err(|_| "Failed to acquire ad blocker lock")?;
    *blocker = rules;
    
    println!("[AdBlock] Initialized with {} filter lists, {} rules", filter_count, rule_count);
    Ok(())
}

/// Parse a single filter rule
#[cfg(feature = "native")]
fn parse_filter_rule(line: &str) -> Option<FilterType> {
    // Element hiding rule: example.com##.ad-banner
    if line.contains("##") {
        let parts: Vec<&str> = line.split("##").collect();
        if parts.len() == 2 {
            let domains = if parts[0].is_empty() {
                vec![]
            } else {
                parts[0].split(',').map(|s| s.to_string()).collect()
            };
            return Some(FilterType::ElementHiding {
                domains,
                selector: parts[1].to_string(),
            });
        }
    }
    
    // Exception rule: @@pattern
    let is_exception = line.starts_with("@@");
    let pattern = if is_exception {
        &line[2..]
    } else {
        line
    };
    
    // Split pattern and options: pattern$option1,option2
    let (pattern, options_str) = if let Some(pos) = pattern.rfind('$') {
        (&pattern[..pos], Some(&pattern[pos + 1..]))
    } else {
        (pattern, None)
    };
    
    // Parse options
    let options = parse_filter_options(options_str);
    
    // Convert ABP pattern to regex
    let regex = pattern_to_regex(pattern)?;
    
    if is_exception {
        Some(FilterType::Exception {
            pattern: pattern.to_string(),
            regex,
            options,
        })
    } else {
        Some(FilterType::Network {
            pattern: pattern.to_string(),
            regex,
            options,
        })
    }
}

/// Parse filter options
#[cfg(feature = "native")]
fn parse_filter_options(options_str: Option<&str>) -> FilterOptions {
    let mut options = FilterOptions::default();
    
    if let Some(opts) = options_str {
        for opt in opts.split(',') {
            let opt = opt.trim();
            
            if opt == "third-party" || opt == "3p" {
                options.third_party = Some(true);
            } else if opt == "~third-party" || opt == "~3p" {
                options.third_party = Some(false);
            } else if opt == "match-case" {
                options.match_case = true;
            } else if opt.starts_with("domain=") {
                let domains_str = &opt[7..];
                for domain in domains_str.split('|') {
                    if domain.starts_with('~') {
                        options.exclude_domains.push(domain[1..].to_string());
                    } else {
                        options.domains.push(domain.to_string());
                    }
                }
            } else if !opt.starts_with('~') {
                // Resource type
                options.types.push(opt.to_string());
            }
        }
    }
    
    options
}

/// Convert ABP pattern to regex
#[cfg(feature = "native")]
fn pattern_to_regex(pattern: &str) -> Option<Regex> {
    let mut regex_pattern = String::new();
    let mut chars = pattern.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            // Separator (anything except letter, digit, or %-._)
            '^' => regex_pattern.push_str("[^a-zA-Z0-9%\\-._]"),
            // Wildcard
            '*' => regex_pattern.push_str(".*"),
            // Start anchor
            '|' => {
                if chars.peek() == Some(&'|') {
                    chars.next();
                    // Domain anchor ||example.com
                    regex_pattern.push_str("^https?://([^/]+\\.)?");
                } else {
                    // Start/end anchor
                    regex_pattern.push_str("^");
                }
            }
            // Regex special characters - escape them
            '.' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '$' | '\\' => {
                regex_pattern.push('\\');
                regex_pattern.push(ch);
            }
            // Normal character
            _ => regex_pattern.push(ch),
        }
    }
    
    // Create regex with case-insensitive flag by default
    Regex::new(&format!("(?i){}", regex_pattern)).ok()
}

/// Check if a URL should be blocked
#[cfg(feature = "native")]
pub fn should_block(url: &str, source_url: &str, resource_type: &str) -> bool {
    // Update stats
    if let Ok(mut stats) = AD_BLOCK_STATS.write() {
        stats.1 += 1; // Total requests
    }
    
    let blocker = match AD_BLOCKER.read() {
        Ok(b) => b,
        Err(_) => return false,
    };
    
    let mut blocked = false;
    
    // Extract domains for third-party check
    let source_domain = extract_domain(source_url);
    let request_domain = extract_domain(url);
    let is_third_party = source_domain != request_domain;
    
    // Check all rules
    for rule in blocker.iter() {
        match rule {
            FilterType::Network { regex, options, .. } => {
                if !regex.is_match(url) {
                    continue;
                }
                
                // Check options
                if !options_match(options, resource_type, &source_domain, is_third_party) {
                    continue;
                }
                
                blocked = true;
            }
            FilterType::Exception { regex, options, .. } => {
                if !regex.is_match(url) {
                    continue;
                }
                
                // Check options
                if !options_match(options, resource_type, &source_domain, is_third_party) {
                    continue;
                }
                
                // Exception rule - don't block
                return false;
            }
            FilterType::ElementHiding { .. } => {
                // Element hiding rules are handled separately (cosmetic filtering)
                continue;
            }
        }
    }
    
    if blocked {
        // Update stats
        if let Ok(mut stats) = AD_BLOCK_STATS.write() {
            stats.0 += 1; // Blocked requests
        }
    }
    
    blocked
}

/// Check if filter options match the request
#[cfg(feature = "native")]
fn options_match(options: &FilterOptions, resource_type: &str, source_domain: &str, is_third_party: bool) -> bool {
    // Check resource type
    if !options.types.is_empty() {
        let type_matches = options.types.iter().any(|t| {
            t.eq_ignore_ascii_case(resource_type) || t == "other"
        });
        if !type_matches {
            return false;
        }
    }
    
    // Check third-party
    if let Some(required_third_party) = options.third_party {
        if required_third_party != is_third_party {
            return false;
        }
    }
    
    // Check domain restrictions
    if !options.domains.is_empty() {
        let domain_matches = options.domains.iter().any(|d| {
            source_domain.ends_with(d) || source_domain == d
        });
        if !domain_matches {
            return false;
        }
    }
    
    // Check excluded domains
    if !options.exclude_domains.is_empty() {
        let domain_excluded = options.exclude_domains.iter().any(|d| {
            source_domain.ends_with(d) || source_domain == d
        });
        if domain_excluded {
            return false;
        }
    }
    
    true
}

/// Extract domain from URL
#[cfg(feature = "native")]
fn extract_domain(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            return host.to_string();
        }
    }
    String::new()
}

/// Download EasyList filter list
#[cfg(feature = "native")]
pub fn download_easylist() -> Result<String, String> {
    let url = "https://easylist.to/easylist/easylist.txt";
    
    let response = ureq::get(url)
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .map_err(|e| format!("Failed to download EasyList: {}", e))?;
    
    let body = response.into_string()
        .map_err(|e| format!("Failed to read EasyList: {}", e))?;
    
    Ok(body)
}

/// Download EasyPrivacy filter list
#[cfg(feature = "native")]
pub fn download_easyprivacy() -> Result<String, String> {
    let url = "https://easylist.to/easylist/easyprivacy.txt";
    
    let response = ureq::get(url)
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .map_err(|e| format!("Failed to download EasyPrivacy: {}", e))?;
    
    let body = response.into_string()
        .map_err(|e| format!("Failed to read EasyPrivacy: {}", e))?;
    
    Ok(body)
}

/// Download Pi-hole block list
#[cfg(feature = "native")]
pub fn download_pihole_blocklist() -> Result<String, String> {
    // Use Steven Black's unified hosts list (includes ads, malware, and tracking)
    let url = "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts";
    
    let response = ureq::get(url)
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .map_err(|e| format!("Failed to download Pi-hole list: {}", e))?;
    
    let body = response.into_string()
        .map_err(|e| format!("Failed to read Pi-hole list: {}", e))?;
    
    Ok(body)
}

/// Parse Pi-hole hosts file format and convert to domain list
#[cfg(feature = "native")]
pub fn parse_pihole_hosts(hosts_content: &str) -> Vec<String> {
    let mut domains = Vec::new();
    
    for line in hosts_content.lines() {
        let line = line.trim();
        
        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        // Parse hosts file format: "0.0.0.0 domain.com" or "127.0.0.1 domain.com"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let domain = parts[1];
            // Skip localhost entries
            if domain != "localhost" && domain != "localhost.localdomain" {
                domains.push(domain.to_string());
            }
        }
    }
    
    domains
}

/// Get statistics about blocked requests
#[cfg(feature = "native")]
pub fn get_stats() -> (usize, usize) {
    AD_BLOCK_STATS.read()
        .map(|stats| *stats)
        .unwrap_or((0, 0))
}

// Stubs for non-native builds
#[cfg(not(feature = "native"))]
pub fn init_adblock(_filter_lists: Vec<String>) -> Result<(), String> {
    Err("Ad blocking not available in non-native builds".to_string())
}

#[cfg(not(feature = "native"))]
pub fn should_block(_url: &str, _source_url: &str, _resource_type: &str) -> bool {
    false
}

#[cfg(not(feature = "native"))]
pub fn download_easylist() -> Result<String, String> {
    Err("Ad blocking not available in non-native builds".to_string())
}

#[cfg(not(feature = "native"))]
pub fn download_easyprivacy() -> Result<String, String> {
    Err("Ad blocking not available in non-native builds".to_string())
}

#[cfg(not(feature = "native"))]
pub fn download_pihole_blocklist() -> Result<String, String> {
    Err("Ad blocking not available in non-native builds".to_string())
}

#[cfg(not(feature = "native"))]
pub fn parse_pihole_hosts(_hosts_content: &str) -> Vec<String> {
    Vec::new()
}

#[cfg(not(feature = "native"))]
pub fn get_stats() -> (usize, usize) {
    (0, 0)
}

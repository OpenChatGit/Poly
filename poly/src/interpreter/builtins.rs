use super::*;

impl Interpreter {
    pub(super) fn call_native(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        match name {
            "print" => {
                let output: Vec<String> = args.iter().map(|v| {
                    let s = format!("{}", v);
                    // Process escape sequences
                    s.replace("\\n", "\n")
                     .replace("\\t", "\t")
                     .replace("\\r", "\r")
                     .replace("\\\"", "\"")
                     .replace("\\'", "'")
                     .replace("\\\\", "\\")
                }).collect();
                let line = output.join(" ");
                self.output.push(line.clone());
                #[cfg(not(target_arch = "wasm32"))]
                println!("{}", line);
                Ok(Value::None)
            }
            "stream" => {
                let text = match args.get(0) {
                    Some(v) => format!("{}", v),
                    None => String::new(),
                };
                // Process escapes just like print
                let text = text.replace("\\n", "\n")
                     .replace("\\t", "\t")
                     .replace("\\r", "\r")
                     .replace("\\\"", "\"")
                     .replace("\\'", "'")
                     .replace("\\\\", "\\");

                if let Some(sender) = &self.stream_sender {
                    let _ = sender.send(text);
                }
                Ok(Value::None)
            }
            "len" => match args.get(0) {
                Some(Value::List(items)) => Ok(Value::Int(items.len() as i64)),
                Some(Value::String(s)) => Ok(Value::Int(s.len() as i64)),
                Some(Value::Dict(pairs)) => Ok(Value::Int(pairs.len() as i64)),
                _ => Err("len() requires a list, string, or dict".to_string()),
            }
            "range" => match args.as_slice() {
                [Value::Int(end)] => Ok(Value::List((0..*end).map(Value::Int).collect())),
                [Value::Int(start), Value::Int(end)] => Ok(Value::List((*start..*end).map(Value::Int).collect())),
                [Value::Int(start), Value::Int(end), Value::Int(step)] => {
                    let mut result = Vec::new();
                    let mut i = *start;
                    while (step > &0 && i < *end) || (step < &0 && i > *end) {
                        result.push(Value::Int(i));
                        i += step;
                    }
                    Ok(Value::List(result))
                }
                _ => Err("range() requires 1-3 integer arguments".to_string()),
            }
            "str" => args.get(0).map(|v| Value::String(format!("{}", v))).ok_or("str() requires an argument".to_string()),
            "int" => match args.get(0) {
                Some(Value::Int(n)) => Ok(Value::Int(*n)),
                Some(Value::Float(f)) => Ok(Value::Int(*f as i64)),
                Some(Value::String(s)) => s.parse::<i64>().map(Value::Int).map_err(|_| "Cannot convert to int".to_string()),
                Some(Value::Bool(b)) => Ok(Value::Int(if *b { 1 } else { 0 })),
                _ => Err("int() requires a number or string".to_string()),
            }
            "float" => match args.get(0) {
                Some(Value::Int(n)) => Ok(Value::Float(*n as f64)),
                Some(Value::Float(f)) => Ok(Value::Float(*f)),
                Some(Value::String(s)) => s.parse::<f64>().map(Value::Float).map_err(|_| "Cannot convert to float".to_string()),
                _ => Err("float() requires a number or string".to_string()),
            }
            "bool" => Ok(Value::Bool(self.is_truthy(args.get(0).unwrap_or(&Value::None)))),
            "type" => {
                let type_name = match args.get(0) {
                    Some(Value::None) => "NoneType",
                    Some(Value::Bool(_)) => "bool",
                    Some(Value::Int(_)) => "int",
                    Some(Value::Float(_)) => "float",
                    Some(Value::String(_)) => "str",
                    Some(Value::List(_)) => "list",
                    Some(Value::Dict(_)) => "dict",
                    Some(Value::Function { .. }) => "function",
                    Some(Value::NativeFunction(_)) => "builtin_function",
                    Some(Value::Instance { class_name, .. }) => class_name,
                    Some(Value::Class { name, .. }) => name,
                    Some(Value::Widget(node)) => &node.widget_type,
                    None => return Err("type() requires an argument".to_string()),
                };
                Ok(Value::String(type_name.to_string()))
            }
            "abs" => match args.get(0) {
                Some(Value::Int(n)) => Ok(Value::Int(n.abs())),
                Some(Value::Float(f)) => Ok(Value::Float(f.abs())),
                _ => Err("abs() requires a number".to_string()),
            }
            "min" => {
                if args.is_empty() { return Err("min() requires arguments".to_string()); }
                let items = if let Some(Value::List(list)) = args.get(0) { list.clone() } else { args };
                items.into_iter().reduce(|a, b| if self.compare_values(&a, &b) == std::cmp::Ordering::Less { a } else { b })
                    .ok_or("min() requires non-empty sequence".to_string())
            }
            "max" => {
                if args.is_empty() { return Err("max() requires arguments".to_string()); }
                let items = if let Some(Value::List(list)) = args.get(0) { list.clone() } else { args };
                items.into_iter().reduce(|a, b| if self.compare_values(&a, &b) == std::cmp::Ordering::Greater { a } else { b })
                    .ok_or("max() requires non-empty sequence".to_string())
            }
            "sum" => {
                let items = match args.get(0) {
                    Some(Value::List(list)) => list.clone(),
                    _ => return Err("sum() requires a list".to_string()),
                };
                let mut total = 0i64;
                for item in items {
                    if let Value::Int(n) = item { total += n; }
                }
                Ok(Value::Int(total))
            }
            "sorted" => {
                let mut items = match args.get(0) {
                    Some(Value::List(list)) => list.clone(),
                    _ => return Err("sorted() requires a list".to_string()),
                };
                items.sort_by(|a, b| self.compare_values(a, b));
                Ok(Value::List(items))
            }
            "reversed" => {
                let mut items = match args.get(0) {
                    Some(Value::List(list)) => list.clone(),
                    Some(Value::String(s)) => s.chars().map(|c| Value::String(c.to_string())).collect(),
                    _ => return Err("reversed() requires a list or string".to_string()),
                };
                items.reverse();
                Ok(Value::List(items))
            }
            "list" => match args.get(0) {
                Some(Value::List(l)) => Ok(Value::List(l.clone())),
                Some(Value::String(s)) => Ok(Value::List(s.chars().map(|c| Value::String(c.to_string())).collect())),
                None => Ok(Value::List(Vec::new())),
                _ => Err("list() requires an iterable".to_string()),
            }
            "isinstance" => match (args.get(0), args.get(1)) {
                (Some(Value::Instance { class_name, .. }), Some(Value::Class { name, .. })) => {
                    Ok(Value::Bool(class_name == name))
                }
                _ => Ok(Value::Bool(false)),
            }
            "hasattr" => match (args.get(0), args.get(1)) {
                (Some(Value::Instance { fields, .. }), Some(Value::String(attr))) => {
                    Ok(Value::Bool(fields.contains_key(attr)))
                }
                _ => Ok(Value::Bool(false)),
            }
            // Math functions
            "math_sqrt" => match args.get(0) {
                Some(Value::Float(f)) => Ok(Value::Float(f.sqrt())),
                Some(Value::Int(n)) => Ok(Value::Float((*n as f64).sqrt())),
                _ => Err("sqrt() requires a number".to_string()),
            }
            "math_sin" => match args.get(0) {
                Some(Value::Float(f)) => Ok(Value::Float(f.sin())),
                Some(Value::Int(n)) => Ok(Value::Float((*n as f64).sin())),
                _ => Err("sin() requires a number".to_string()),
            }
            "math_cos" => match args.get(0) {
                Some(Value::Float(f)) => Ok(Value::Float(f.cos())),
                Some(Value::Int(n)) => Ok(Value::Float((*n as f64).cos())),
                _ => Err("cos() requires a number".to_string()),
            }
            "math_tan" => match args.get(0) {
                Some(Value::Float(f)) => Ok(Value::Float(f.tan())),
                Some(Value::Int(n)) => Ok(Value::Float((*n as f64).tan())),
                _ => Err("tan() requires a number".to_string()),
            }
            "math_floor" => match args.get(0) {
                Some(Value::Float(f)) => Ok(Value::Int(f.floor() as i64)),
                Some(Value::Int(n)) => Ok(Value::Int(*n)),
                _ => Err("floor() requires a number".to_string()),
            }
            "math_ceil" => match args.get(0) {
                Some(Value::Float(f)) => Ok(Value::Int(f.ceil() as i64)),
                Some(Value::Int(n)) => Ok(Value::Int(*n)),
                _ => Err("ceil() requires a number".to_string()),
            }
            "math_log" => match args.get(0) {
                Some(Value::Float(f)) => Ok(Value::Float(f.ln())),
                Some(Value::Int(n)) => Ok(Value::Float((*n as f64).ln())),
                _ => Err("log() requires a number".to_string()),
            }
            // Random functions
            "random" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
                let random = ((seed * 1103515245 + 12345) % (1 << 31)) as f64 / (1u64 << 31) as f64;
                Ok(Value::Float(random))
            }
            "randint" => match (args.get(0), args.get(1)) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => {
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
                    let range = (b - a + 1) as u128;
                    let random = (seed % range) as i64 + a;
                    Ok(Value::Int(random))
                }
                _ => Err("randint() requires two integers".to_string()),
            }
            "choice" => match args.get(0) {
                Some(Value::List(items)) if !items.is_empty() => {
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
                    let idx = (seed as usize) % items.len();
                    Ok(items[idx].clone())
                }
                _ => Err("choice() requires a non-empty list".to_string()),
            }
            // Time functions
            "time" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64();
                Ok(Value::Float(secs))
            }
            "sleep" => match args.get(0) {
                Some(Value::Float(secs)) => {
                    std::thread::sleep(std::time::Duration::from_secs_f64(*secs));
                    Ok(Value::None)
                }
                Some(Value::Int(secs)) => {
                    std::thread::sleep(std::time::Duration::from_secs(*secs as u64));
                    Ok(Value::None)
                }
                _ => Err(self.error("sleep() requires a number")),
            }
            // String methods
            "upper" => match args.get(0) {
                Some(Value::String(s)) => Ok(Value::String(s.to_uppercase())),
                _ => Err(self.error("upper() requires a string")),
            }
            "lower" => match args.get(0) {
                Some(Value::String(s)) => Ok(Value::String(s.to_lowercase())),
                _ => Err(self.error("lower() requires a string")),
            }
            "strip" => match args.get(0) {
                Some(Value::String(s)) => Ok(Value::String(s.trim().to_string())),
                _ => Err(self.error("strip() requires a string")),
            }
            "split" => match (args.get(0), args.get(1)) {
                (Some(Value::String(s)), Some(Value::String(sep))) => {
                    Ok(Value::List(s.split(sep.as_str()).map(|p| Value::String(p.to_string())).collect()))
                }
                (Some(Value::String(s)), None) => {
                    Ok(Value::List(s.split_whitespace().map(|p| Value::String(p.to_string())).collect()))
                }
                _ => Err(self.error("split() requires a string")),
            }
            "join" => match (args.get(0), args.get(1)) {
                (Some(Value::String(sep)), Some(Value::List(items))) => {
                    let strings: Vec<String> = items.iter().map(|v| format!("{}", v)).collect();
                    Ok(Value::String(strings.join(sep)))
                }
                _ => Err(self.error("join() requires a separator and a list")),
            }
            "replace" => match (args.get(0), args.get(1), args.get(2)) {
                (Some(Value::String(s)), Some(Value::String(old)), Some(Value::String(new))) => {
                    Ok(Value::String(s.replace(old.as_str(), new.as_str())))
                }
                _ => Err(self.error("replace() requires a string, old, and new")),
            }
            "startswith" => match (args.get(0), args.get(1)) {
                (Some(Value::String(s)), Some(Value::String(prefix))) => {
                    Ok(Value::Bool(s.starts_with(prefix.as_str())))
                }
                _ => Err(self.error("startswith() requires two strings")),
            }
            "endswith" => match (args.get(0), args.get(1)) {
                (Some(Value::String(s)), Some(Value::String(suffix))) => {
                    Ok(Value::Bool(s.ends_with(suffix.as_str())))
                }
                _ => Err(self.error("endswith() requires two strings")),
            }
            "find" => match (args.get(0), args.get(1)) {
                (Some(Value::String(s)), Some(Value::String(sub))) => {
                    Ok(Value::Int(s.find(sub.as_str()).map(|i| i as i64).unwrap_or(-1)))
                }
                _ => Err(self.error("find() requires two strings")),
            }
            "count" => match (args.get(0), args.get(1)) {
                (Some(Value::String(s)), Some(Value::String(sub))) => {
                    Ok(Value::Int(s.matches(sub.as_str()).count() as i64))
                }
                (Some(Value::List(items)), Some(val)) => {
                    Ok(Value::Int(items.iter().filter(|v| *v == val).count() as i64))
                }
                _ => Err(self.error("count() requires a string/list and value")),
            }
            "isdigit" => match args.get(0) {
                Some(Value::String(s)) => Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))),
                _ => Err(self.error("isdigit() requires a string")),
            }
            "isalpha" => match args.get(0) {
                Some(Value::String(s)) => Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_alphabetic()))),
                _ => Err(self.error("isalpha() requires a string")),
            }
            "isalnum" => match args.get(0) {
                Some(Value::String(s)) => Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_alphanumeric()))),
                _ => Err(self.error("isalnum() requires a string")),
            }
            // List methods
            "push" | "append" => match (args.get(0), args.get(1)) {
                (Some(Value::List(items)), Some(val)) => {
                    let mut new_items = items.clone();
                    new_items.push(val.clone());
                    Ok(Value::List(new_items))
                }
                _ => Err(self.error("push() requires a list and value")),
            }
            "pop" => match args.get(0) {
                Some(Value::List(items)) if !items.is_empty() => {
                    let mut new_items = items.clone();
                    let popped = new_items.pop().unwrap();
                    Ok(popped)
                }
                _ => Err(self.error("pop() requires a non-empty list")),
            }
            "insert" => match (args.get(0), args.get(1), args.get(2)) {
                (Some(Value::List(items)), Some(Value::Int(idx)), Some(val)) => {
                    let mut new_items = items.clone();
                    let idx = if *idx < 0 { (new_items.len() as i64 + idx) as usize } else { *idx as usize };
                    new_items.insert(idx.min(new_items.len()), val.clone());
                    Ok(Value::List(new_items))
                }
                _ => Err(self.error("insert() requires a list, index, and value")),
            }
            "remove" => match (args.get(0), args.get(1)) {
                (Some(Value::List(items)), Some(val)) => {
                    let mut new_items = items.clone();
                    if let Some(pos) = new_items.iter().position(|v| v == val) {
                        new_items.remove(pos);
                    }
                    Ok(Value::List(new_items))
                }
                _ => Err(self.error("remove() requires a list and value")),
            }
            "index" => match (args.get(0), args.get(1)) {
                (Some(Value::List(items)), Some(val)) => {
                    Ok(Value::Int(items.iter().position(|v| v == val).map(|i| i as i64).unwrap_or(-1)))
                }
                _ => Err(self.error("index() requires a list and value")),
            }
            "clear" => match args.get(0) {
                Some(Value::List(_)) => Ok(Value::List(Vec::new())),
                Some(Value::Dict(_)) => Ok(Value::Dict(Vec::new())),
                _ => Err(self.error("clear() requires a list or dict")),
            }
            "copy" => match args.get(0) {
                Some(Value::List(items)) => Ok(Value::List(items.clone())),
                Some(Value::Dict(pairs)) => Ok(Value::Dict(pairs.clone())),
                _ => Err(self.error("copy() requires a list or dict")),
            }
            "extend" => match (args.get(0), args.get(1)) {
                (Some(Value::List(items)), Some(Value::List(other))) => {
                    let mut new_items = items.clone();
                    new_items.extend(other.clone());
                    Ok(Value::List(new_items))
                }
                _ => Err(self.error("extend() requires two lists")),
            }
            // File I/O
            "read_file" => match args.get(0) {
                Some(Value::String(path)) => {
                    match std::fs::read_to_string(path) {
                        Ok(content) => Ok(Value::String(content)),
                        Err(e) => Err(self.error(format!("Failed to read file: {}", e))),
                    }
                }
                _ => Err(self.error("read_file() requires a path string")),
            }
            "read_file_base64" => {
                // read_file_base64(path) -> data URL string (e.g., "data:image/png;base64,...")
                match args.get(0) {
                    Some(Value::String(path)) => {
                        match std::fs::read(path) {
                            Ok(bytes) => {
                                // Determine MIME type from extension
                                let mime = if path.ends_with(".png") {
                                    "image/png"
                                } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
                                    "image/jpeg"
                                } else if path.ends_with(".gif") {
                                    "image/gif"
                                } else if path.ends_with(".webp") {
                                    "image/webp"
                                } else if path.ends_with(".svg") {
                                    "image/svg+xml"
                                } else if path.ends_with(".ico") {
                                    "image/x-icon"
                                } else if path.ends_with(".mp3") {
                                    "audio/mpeg"
                                } else if path.ends_with(".wav") {
                                    "audio/wav"
                                } else if path.ends_with(".mp4") {
                                    "video/mp4"
                                } else if path.ends_with(".webm") {
                                    "video/webm"
                                } else if path.ends_with(".pdf") {
                                    "application/pdf"
                                } else if path.ends_with(".json") {
                                    "application/json"
                                } else {
                                    "application/octet-stream"
                                };

                                // Base64 encode
                                const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
                                let mut result = String::new();
                                for chunk in bytes.chunks(3) {
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

                                Ok(Value::String(format!("data:{};base64,{}", mime, result)))
                            }
                            Err(e) => Err(self.error(format!("Failed to read file: {}", e))),
                        }
                    }
                    _ => Err(self.error("read_file_base64() requires a path string")),
                }
            }
            "write_file" => match (args.get(0), args.get(1)) {
                (Some(Value::String(path)), Some(Value::String(content))) => {
                    let processed = process_escapes(content);
                    match std::fs::write(path, processed) {
                        Ok(_) => Ok(Value::None),
                        Err(e) => Err(self.error(format!("Failed to write file: {}", e))),
                    }
                }
                _ => Err(self.error("write_file() requires a path and content string")),
            }
            "file_exists" => match args.get(0) {
                Some(Value::String(path)) => Ok(Value::Bool(std::path::Path::new(path).exists())),
                _ => Err(self.error("file_exists() requires a path string")),
            }
            // HTTP Functions
            "http_get" => {
                // http_get(url) -> string or dict with response
                #[cfg(feature = "native")]
                {
                    let url = match args.get(0) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(self.error("http_get() requires a URL string")),
                    };

                    let client = reqwest::blocking::Client::builder()
                        .timeout(std::time::Duration::from_secs(30))
                        .build()
                        .map_err(|e| self.error(format!("HTTP client error: {}", e)))?;

                    match client.get(&url).send() {
                        Ok(resp) => {
                            let status = resp.status().as_u16() as i64;
                            let body = resp.text().unwrap_or_default();

                            // Return a dict with status and body
                            Ok(Value::Dict(vec![
                                (Value::String("status".to_string()), Value::Int(status)),
                                (Value::String("body".to_string()), Value::String(body)),
                            ]))
                        }
                        Err(e) => Err(self.error(format!("HTTP request failed: {}", e))),
                    }
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("http_get() requires native feature"))
                }
            }
            "http_get_steam" => {
                // http_get_steam(url) -> dict with response
                // Makes HTTP request with Steam session cookies from WebView
                #[cfg(feature = "native")]
                {
                    let url = match args.get(0) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(self.error("http_get_steam() requires a URL string")),
                    };

                    // Get Steam cookies from the cookie storage
                    let cookies = crate::cookies::get_steam_cookies();

                    let client = reqwest::blocking::Client::builder()
                        .timeout(std::time::Duration::from_secs(30))
                        .build()
                        .map_err(|e| self.error(format!("HTTP client error: {}", e)))?;

                    let mut request = client.get(&url)
                        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
                        .header("Accept-Language", "en-US,en;q=0.9,de;q=0.8");

                    // Add cookies if we have them
                    if !cookies.is_empty() {
                        request = request.header("Cookie", &cookies);
                        println!("[Backend] http_get_steam with cookies: {}",
                            if cookies.contains("steamLoginSecure") { "has login" } else { "no login" });
                    } else {
                        println!("[Backend] http_get_steam: no cookies available");
                    }

                    match request.send() {
                        Ok(resp) => {
                            let status = resp.status().as_u16() as i64;
                            let body = resp.text().unwrap_or_default();

                            Ok(Value::Dict(vec![
                                (Value::String("status".to_string()), Value::Int(status)),
                                (Value::String("body".to_string()), Value::String(body)),
                            ]))
                        }
                        Err(e) => Err(self.error(format!("HTTP request failed: {}", e))),
                    }
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("http_get_steam() requires native feature"))
                }
            }
            "http_get_parallel" => {
                // http_get_parallel(urls_list) -> list of responses
                // Executes multiple HTTP GET requests in parallel using threads
                #[cfg(feature = "native")]
                {
                    let urls = match args.get(0) {
                        Some(Value::List(list)) => {
                            list.iter().filter_map(|v| {
                                if let Value::String(s) = v {
                                    Some(s.clone())
                                } else {
                                    None
                                }
                            }).collect::<Vec<String>>()
                        }
                        _ => return Err(self.error("http_get_parallel() requires a list of URL strings")),
                    };

                    use std::thread;
                    use std::sync::mpsc;

                    let (tx, rx) = mpsc::channel();
                    let mut handles = Vec::new();

                    for (idx, url) in urls.iter().enumerate() {
                        let url = url.clone();
                        let tx = tx.clone();

                        let handle = thread::spawn(move || {
                            let client = reqwest::blocking::Client::builder()
                                .timeout(std::time::Duration::from_secs(30))
                                .build();

                            let result = match client {
                                Ok(client) => {
                                    match client.get(&url).send() {
                                        Ok(resp) => {
                                            let status = resp.status().as_u16() as i64;
                                            let body = resp.text().unwrap_or_default();
                                            Value::Dict(vec![
                                                (Value::String("status".to_string()), Value::Int(status)),
                                                (Value::String("body".to_string()), Value::String(body)),
                                                (Value::String("url".to_string()), Value::String(url)),
                                            ])
                                        }
                                        Err(e) => Value::Dict(vec![
                                            (Value::String("status".to_string()), Value::Int(0)),
                                            (Value::String("body".to_string()), Value::String(format!("Error: {}", e))),
                                            (Value::String("url".to_string()), Value::String(url)),
                                        ]),
                                    }
                                }
                                Err(e) => Value::Dict(vec![
                                    (Value::String("status".to_string()), Value::Int(0)),
                                    (Value::String("body".to_string()), Value::String(format!("Client error: {}", e))),
                                    (Value::String("url".to_string()), Value::String(url)),
                                ]),
                            };

                            let _ = tx.send((idx, result));
                        });

                        handles.push(handle);
                    }

                    // Drop the original sender so rx.iter() will complete
                    drop(tx);

                    // Collect results in order
                    let mut results: Vec<(usize, Value)> = rx.iter().collect();
                    results.sort_by_key(|(idx, _)| *idx);

                    // Wait for all threads to complete
                    for handle in handles {
                        let _ = handle.join();
                    }

                    Ok(Value::List(results.into_iter().map(|(_, v)| v).collect()))
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("http_get_parallel() requires native feature"))
                }
            }
            "http_post" => {
                // http_post(url, body, content_type?) -> dict with response
                #[cfg(feature = "native")]
                {
                    let url = match args.get(0) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(self.error("http_post() requires a URL string")),
                    };
                    let body = match args.get(1) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(self.error("http_post() requires a body string")),
                    };
                    let content_type = match args.get(2) {
                        Some(Value::String(s)) => s.clone(),
                        _ => "application/json".to_string(),
                    };

                    let client = reqwest::blocking::Client::builder()
                        .timeout(std::time::Duration::from_secs(300))
                        .build()
                        .map_err(|e| self.error(format!("HTTP client error: {}", e)))?;

                    match client.post(&url)
                        .header("Content-Type", &content_type)
                        .body(body)
                        .send()
                    {
                        Ok(resp) => {
                            let status = resp.status().as_u16() as i64;
                            let body = resp.text().unwrap_or_default();

                            Ok(Value::Dict(vec![
                                (Value::String("status".to_string()), Value::Int(status)),
                                (Value::String("body".to_string()), Value::String(body)),
                            ]))
                        }
                        Err(e) => Err(self.error(format!("HTTP request failed: {}", e))),
                    }
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("http_post() requires native feature"))
                }
            }
            "http_post_json" => {
                // http_post_json(url, data_dict) -> dict with parsed JSON response
                #[cfg(feature = "native")]
                {
                    let url = match args.get(0) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(self.error("http_post_json() requires a URL string")),
                    };

                    // Convert Value to JSON
                    fn value_to_json(v: &Value) -> serde_json::Value {
                        match v {
                            Value::None => serde_json::Value::Null,
                            Value::Bool(b) => serde_json::Value::Bool(*b),
                            Value::Int(i) => serde_json::Value::Number((*i).into()),
                            Value::Float(f) => serde_json::json!(*f),
                            Value::String(s) => serde_json::Value::String(s.clone()),
                            Value::List(items) => serde_json::Value::Array(
                                items.iter().map(value_to_json).collect()
                            ),
                            Value::Dict(pairs) => {
                                let mut map = serde_json::Map::new();
                                for (k, v) in pairs {
                                    if let Value::String(key) = k {
                                        map.insert(key.clone(), value_to_json(v));
                                    }
                                }
                                serde_json::Value::Object(map)
                            }
                            _ => serde_json::Value::Null,
                        }
                    }

                    fn json_to_value(j: &serde_json::Value) -> Value {
                        match j {
                            serde_json::Value::Null => Value::None,
                            serde_json::Value::Bool(b) => Value::Bool(*b),
                            serde_json::Value::Number(n) => {
                                if let Some(i) = n.as_i64() {
                                    Value::Int(i)
                                } else if let Some(f) = n.as_f64() {
                                    Value::Float(f)
                                } else {
                                    Value::None
                                }
                            }
                            serde_json::Value::String(s) => Value::String(s.clone()),
                            serde_json::Value::Array(arr) => Value::List(
                                arr.iter().map(json_to_value).collect()
                            ),
                            serde_json::Value::Object(obj) => Value::Dict(
                                obj.iter().map(|(k, v)| (Value::String(k.clone()), json_to_value(v))).collect()
                            ),
                        }
                    }

                    let json_body = match args.get(1) {
                        Some(v) => value_to_json(v),
                        _ => return Err(self.error("http_post_json() requires data")),
                    };

                    let client = reqwest::blocking::Client::builder()
                        .timeout(std::time::Duration::from_secs(300))
                        .build()
                        .map_err(|e| self.error(format!("HTTP client error: {}", e)))?;

                    match client.post(&url)
                        .header("Content-Type", "application/json")
                        .json(&json_body)
                        .send()
                    {
                        Ok(resp) => {
                            let status = resp.status().as_u16() as i64;
                            let body_text = resp.text().unwrap_or_default();

                            // Try to parse as JSON
                            let body_value = match serde_json::from_str::<serde_json::Value>(&body_text) {
                                Ok(json) => json_to_value(&json),
                                Err(_) => Value::String(body_text),
                            };

                            Ok(Value::Dict(vec![
                                (Value::String("status".to_string()), Value::Int(status)),
                                (Value::String("body".to_string()), body_value),
                            ]))
                        }
                        Err(e) => Err(self.error(format!("HTTP request failed: {}", e))),
                    }
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("http_post_json() requires native feature"))
                }
            }
            // JSON functions
            "json_parse" => {
                // json_parse(string) -> value
                let json_str = match args.get(0) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(self.error("json_parse() requires a string")),
                };

                fn json_to_value(j: &serde_json::Value) -> Value {
                    match j {
                        serde_json::Value::Null => Value::None,
                        serde_json::Value::Bool(b) => Value::Bool(*b),
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                Value::Int(i)
                            } else if let Some(f) = n.as_f64() {
                                Value::Float(f)
                            } else {
                                Value::None
                            }
                        }
                        serde_json::Value::String(s) => Value::String(s.clone()),
                        serde_json::Value::Array(arr) => Value::List(
                            arr.iter().map(json_to_value).collect()
                        ),
                        serde_json::Value::Object(obj) => Value::Dict(
                            obj.iter().map(|(k, v)| (Value::String(k.clone()), json_to_value(v))).collect()
                        ),
                    }
                }

                match serde_json::from_str::<serde_json::Value>(&json_str) {
                    Ok(json) => Ok(json_to_value(&json)),
                    Err(e) => Err(self.error(format!("JSON parse error: {}", e))),
                }
            }
            "json_stringify" => {
                // json_stringify(value) -> string
                fn value_to_json(v: &Value) -> serde_json::Value {
                    match v {
                        Value::None => serde_json::Value::Null,
                        Value::Bool(b) => serde_json::Value::Bool(*b),
                        Value::Int(i) => serde_json::Value::Number((*i).into()),
                        Value::Float(f) => serde_json::json!(*f),
                        Value::String(s) => serde_json::Value::String(s.clone()),
                        Value::List(items) => serde_json::Value::Array(
                            items.iter().map(value_to_json).collect()
                        ),
                        Value::Dict(pairs) => {
                            let mut map = serde_json::Map::new();
                            for (k, v) in pairs {
                                if let Value::String(key) = k {
                                    map.insert(key.clone(), value_to_json(v));
                                }
                            }
                            serde_json::Value::Object(map)
                        }
                        _ => serde_json::Value::Null,
                    }
                }

                match args.get(0) {
                    Some(v) => Ok(Value::String(value_to_json(v).to_string())),
                    None => Err(self.error("json_stringify() requires a value")),
                }
            }
            // HTTP Streaming functions
            "http_stream_start" => {
                // http_stream_start(url, body_json) -> session_id
                // Starts a streaming POST request in background, returns session ID
                #[cfg(feature = "native")]
                {
                    let url = match args.get(0) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(self.error("http_stream_start() requires a URL string")),
                    };

                    // Convert Value to JSON for request body
                    fn value_to_json(v: &Value) -> serde_json::Value {
                        match v {
                            Value::None => serde_json::Value::Null,
                            Value::Bool(b) => serde_json::Value::Bool(*b),
                            Value::Int(i) => serde_json::Value::Number((*i).into()),
                            Value::Float(f) => serde_json::json!(*f),
                            Value::String(s) => serde_json::Value::String(s.clone()),
                            Value::List(items) => serde_json::Value::Array(
                                items.iter().map(value_to_json).collect()
                            ),
                            Value::Dict(pairs) => {
                                let mut map = serde_json::Map::new();
                                for (k, v) in pairs {
                                    if let Value::String(key) = k {
                                        map.insert(key.clone(), value_to_json(v));
                                    }
                                }
                                serde_json::Value::Object(map)
                            }
                            _ => serde_json::Value::Null,
                        }
                    }

                    let json_body = match args.get(1) {
                        Some(v) => value_to_json(v),
                        _ => return Err(self.error("http_stream_start() requires body data")),
                    };

                    // Generate session ID using atomic counter + milliseconds (fits in JS safe integer range)
                    use std::sync::atomic::{AtomicU64, Ordering};
                    static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);
                    let counter = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
                    let millis = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    // Use lower bits of millis + counter to stay within JS safe integer range
                    let session_id = ((millis & 0xFFFFFFFF) << 16) | (counter & 0xFFFF);

                    // Initialize session
                    {
                        let mut sessions = STREAM_SESSIONS.lock().unwrap();
                        sessions.insert(session_id, StreamSession {
                            buffer: Vec::new(),
                            done: false,
                            error: None,
                        });
                    }

                    // Spawn background thread for streaming
                    let url_clone = url.clone();
                    let session_id_clone = session_id;
                    std::thread::spawn(move || {
                        let client = match reqwest::blocking::Client::builder()
                            .timeout(std::time::Duration::from_secs(300))
                            .build()
                        {
                            Ok(c) => c,
                            Err(e) => {
                                let mut sessions = STREAM_SESSIONS.lock().unwrap();
                                if let Some(session) = sessions.get_mut(&session_id_clone) {
                                    session.error = Some(format!("Client error: {}", e));
                                    session.done = true;
                                }
                                return;
                            }
                        };

                        let response = match client.post(&url_clone)
                            .header("Content-Type", "application/json")
                            .json(&json_body)
                            .send()
                        {
                            Ok(r) => r,
                            Err(e) => {
                                let mut sessions = STREAM_SESSIONS.lock().unwrap();
                                if let Some(session) = sessions.get_mut(&session_id_clone) {
                                    session.error = Some(format!("Request error: {}", e));
                                    session.done = true;
                                }
                                return;
                            }
                        };

                        // Read streaming response line by line (NDJSON format)
                        use std::io::BufRead;
                        let reader = std::io::BufReader::new(response);
                        for line in reader.lines() {
                            match line {
                                Ok(line) if !line.is_empty() => {
                                    let mut sessions = STREAM_SESSIONS.lock().unwrap();
                                    if let Some(session) = sessions.get_mut(&session_id_clone) {
                                        session.buffer.push(line);
                                    }
                                }
                                Err(e) => {
                                    let mut sessions = STREAM_SESSIONS.lock().unwrap();
                                    if let Some(session) = sessions.get_mut(&session_id_clone) {
                                        session.error = Some(format!("Read error: {}", e));
                                        session.done = true;
                                    }
                                    return;
                                }
                                _ => {}
                            }
                        }

                        // Mark as done
                        let mut sessions = STREAM_SESSIONS.lock().unwrap();
                        if let Some(session) = sessions.get_mut(&session_id_clone) {
                            session.done = true;
                        }
                    });

                    Ok(Value::Int(session_id as i64))
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("http_stream_start() requires native feature"))
                }
            }
            "http_stream_poll" => {
                // http_stream_poll(session_id) -> { chunks: [...], done: bool, error: string|null }
                #[cfg(feature = "native")]
                {
                    let session_id = match args.get(0) {
                        Some(Value::Int(id)) => *id as u64,
                        _ => return Err(self.error("http_stream_poll() requires session ID")),
                    };

                    let mut sessions = STREAM_SESSIONS.lock().unwrap();
                    if let Some(session) = sessions.get_mut(&session_id) {
                        // Drain buffer
                        let chunks: Vec<Value> = session.buffer.drain(..)
                            .map(|s| Value::String(s))
                            .collect();

                        let error_val = match &session.error {
                            Some(e) => Value::String(e.clone()),
                            None => Value::None,
                        };

                        Ok(Value::Dict(vec![
                            (Value::String("chunks".to_string()), Value::List(chunks)),
                            (Value::String("done".to_string()), Value::Bool(session.done)),
                            (Value::String("error".to_string()), error_val),
                        ]))
                    } else {
                        Ok(Value::Dict(vec![
                            (Value::String("chunks".to_string()), Value::List(vec![])),
                            (Value::String("done".to_string()), Value::Bool(true)),
                            (Value::String("error".to_string()), Value::String("Session not found".to_string())),
                        ]))
                    }
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("http_stream_poll() requires native feature"))
                }
            }
            "http_stream_close" => {
                // http_stream_close(session_id) -> bool
                #[cfg(feature = "native")]
                {
                    let session_id = match args.get(0) {
                        Some(Value::Int(id)) => *id as u64,
                        _ => return Err(self.error("http_stream_close() requires session ID")),
                    };

                    let mut sessions = STREAM_SESSIONS.lock().unwrap();
                    let removed = sessions.remove(&session_id).is_some();
                    Ok(Value::Bool(removed))
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("http_stream_close() requires native feature"))
                }
            }
            // HTTP Server functions for OAuth callbacks, webhooks, etc.
            "http_server_start" => {
                // http_server_start(port, response_html?) -> bool
                // Starts a simple HTTP server that captures incoming requests
                #[cfg(feature = "native")]
                {
                    let port = match args.get(0) {
                        Some(Value::Int(p)) => *p as u16,
                        _ => return Err(self.error("http_server_start() requires a port number")),
                    };

                    let response_html = match args.get(1) {
                        Some(Value::String(s)) => s.clone(),
                        _ => r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Success</title></head><body><h1>Authentication successful!</h1><p>You can close this window.</p><script>window.close();</script></body></html>"#.to_string(),
                    };

                    // Check if already running
                    {
                        let state = HTTP_SERVER.lock().unwrap();
                        if state.running {
                            return Ok(Value::Bool(true)); // Already running
                        }
                    }

                    // Reset shutdown flag
                    {
                        let mut shutdown = HTTP_SERVER_SHUTDOWN.lock().unwrap();
                        *shutdown = false;
                    }

                    // Start server in background thread
                    let handle = std::thread::spawn(move || {
                        let addr = format!("127.0.0.1:{}", port);
                        let server = match tiny_http::Server::http(&addr) {
                            Ok(s) => s,
                            Err(e) => {
                                eprintln!("[http_server] Failed to start: {}", e);
                                return;
                            }
                        };

                        println!("[http_server] Listening on http://{}", addr);

                        // Mark as running
                        {
                            let mut state = HTTP_SERVER.lock().unwrap();
                            state.running = true;
                        }

                        // Use non-blocking with timeout to check shutdown flag
                        loop {
                            // Check shutdown flag
                            {
                                let shutdown = HTTP_SERVER_SHUTDOWN.lock().unwrap();
                                if *shutdown {
                                    break;
                                }
                            }

                            // Try to receive request with timeout
                            match server.recv_timeout(std::time::Duration::from_millis(100)) {
                                Ok(Some(mut request)) => {
                                    let method = request.method().to_string();
                                    let url = request.url().to_string();

                                    // Parse path and query
                                    let (path, query) = if let Some(q_pos) = url.find('?') {
                                        (url[..q_pos].to_string(), url[q_pos + 1..].to_string())
                                    } else {
                                        (url.clone(), String::new())
                                    };

                                    // Collect headers
                                    let headers: Vec<(String, String)> = request.headers()
                                        .iter()
                                        .map(|h| (h.field.to_string(), h.value.to_string()))
                                        .collect();

                                    // Read body (for POST requests)
                                    let mut body = String::new();
                                    if method == "POST" {
                                        let mut reader = request.as_reader();
                                        let _ = std::io::Read::read_to_string(&mut reader, &mut body);
                                    }

                                    // Store request
                                    {
                                        let mut state = HTTP_SERVER.lock().unwrap();
                                        state.requests.push(HttpRequest {
                                            method,
                                            path,
                                            query,
                                            headers,
                                            body,
                                        });
                                    }

                                    // Send response
                                    let response = tiny_http::Response::from_string(response_html.clone())
                                        .with_header(tiny_http::Header::from_bytes(
                                            &b"Content-Type"[..],
                                            &b"text/html; charset=utf-8"[..]
                                        ).unwrap());
                                    let _ = request.respond(response);
                                }
                                Ok(None) => {
                                    // Timeout, continue loop
                                }
                                Err(_) => {
                                    // Error, continue
                                }
                            }
                        }

                        // Mark as stopped
                        {
                            let mut state = HTTP_SERVER.lock().unwrap();
                            state.running = false;
                        }
                        println!("[http_server] Stopped");
                    });

                    // Store handle
                    {
                        let mut h = HTTP_SERVER_HANDLE.lock().unwrap();
                        *h = Some(handle);
                    }

                    // Wait a bit for server to start
                    std::thread::sleep(std::time::Duration::from_millis(100));

                    Ok(Value::Bool(true))
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("http_server_start() requires native feature"))
                }
            }
            "http_server_stop" => {
                // http_server_stop() -> bool
                #[cfg(feature = "native")]
                {
                    // Signal shutdown
                    {
                        let mut shutdown = HTTP_SERVER_SHUTDOWN.lock().unwrap();
                        *shutdown = true;
                    }

                    // Wait for thread to finish
                    {
                        let mut h = HTTP_SERVER_HANDLE.lock().unwrap();
                        if let Some(handle) = h.take() {
                            let _ = handle.join();
                        }
                    }

                    Ok(Value::Bool(true))
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("http_server_stop() requires native feature"))
                }
            }
            "http_server_poll" => {
                // http_server_poll() -> list of request dicts or None
                // Returns and clears all pending requests
                #[cfg(feature = "native")]
                {
                    let mut state = HTTP_SERVER.lock().unwrap();

                    if state.requests.is_empty() {
                        return Ok(Value::None);
                    }

                    let requests: Vec<Value> = state.requests.drain(..).map(|req| {
                        // Parse query string into dict
                        let query_pairs: Vec<(Value, Value)> = req.query
                            .split('&')
                            .filter(|s| !s.is_empty())
                            .filter_map(|pair| {
                                let mut parts = pair.splitn(2, '=');
                                let key = parts.next()?;
                                let value = parts.next().unwrap_or("");
                                let decoded_key = urlencoding::decode(key).unwrap_or_else(|_| key.into()).to_string();
                                let decoded_value = urlencoding::decode(value).unwrap_or_else(|_| value.into()).to_string();
                                Some((Value::String(decoded_key), Value::String(decoded_value)))
                            })
                            .collect();

                        // Convert headers to dict
                        let headers_dict: Vec<(Value, Value)> = req.headers
                            .iter()
                            .map(|(k, v)| (Value::String(k.clone()), Value::String(v.clone())))
                            .collect();

                        Value::Dict(vec![
                            (Value::String("method".to_string()), Value::String(req.method)),
                            (Value::String("path".to_string()), Value::String(req.path)),
                            (Value::String("query".to_string()), Value::String(req.query)),
                            (Value::String("query_params".to_string()), Value::Dict(query_pairs)),
                            (Value::String("headers".to_string()), Value::Dict(headers_dict)),
                            (Value::String("body".to_string()), Value::String(req.body)),
                        ])
                    }).collect();

                    Ok(Value::List(requests))
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("http_server_poll() requires native feature"))
                }
            }
            // Additional utility functions
            "chr" => match args.get(0) {
                Some(Value::Int(n)) => {
                    if *n >= 0 && *n <= 0x10FFFF {
                        Ok(Value::String(char::from_u32(*n as u32).unwrap_or('\u{FFFD}').to_string()))
                    } else {
                        Err(self.error("chr() arg not in range"))
                    }
                }
                _ => Err(self.error("chr() requires an integer")),
            }
            "ord" => match args.get(0) {
                Some(Value::String(s)) if s.len() == 1 => {
                    Ok(Value::Int(s.chars().next().unwrap() as i64))
                }
                _ => Err(self.error("ord() requires a single character string")),
            }
            "hex" => match args.get(0) {
                Some(Value::Int(n)) => Ok(Value::String(format!("0x{:x}", n))),
                _ => Err(self.error("hex() requires an integer")),
            }
            "bin" => match args.get(0) {
                Some(Value::Int(n)) => Ok(Value::String(format!("0b{:b}", n))),
                _ => Err(self.error("bin() requires an integer")),
            }
            "oct" => match args.get(0) {
                Some(Value::Int(n)) => Ok(Value::String(format!("0o{:o}", n))),
                _ => Err(self.error("oct() requires an integer")),
            }
            "round" => match (args.get(0), args.get(1)) {
                (Some(Value::Float(f)), Some(Value::Int(digits))) => {
                    let factor = 10f64.powi(*digits as i32);
                    Ok(Value::Float((f * factor).round() / factor))
                }
                (Some(Value::Float(f)), None) => Ok(Value::Int(f.round() as i64)),
                (Some(Value::Int(n)), _) => Ok(Value::Int(*n)),
                _ => Err(self.error("round() requires a number")),
            }
            "any" => match args.get(0) {
                Some(Value::List(items)) => {
                    Ok(Value::Bool(items.iter().any(|v| self.is_truthy(v))))
                }
                _ => Err(self.error("any() requires a list")),
            }
            "all" => match args.get(0) {
                Some(Value::List(items)) => {
                    Ok(Value::Bool(items.iter().all(|v| self.is_truthy(v))))
                }
                _ => Err(self.error("all() requires a list")),
            }
            "enumerate" => match args.get(0) {
                Some(Value::List(items)) => {
                    let enumerated: Vec<Value> = items.iter().enumerate()
                        .map(|(i, v)| Value::List(vec![Value::Int(i as i64), v.clone()]))
                        .collect();
                    Ok(Value::List(enumerated))
                }
                _ => Err(self.error("enumerate() requires a list")),
            }
            "zip" => match (args.get(0), args.get(1)) {
                (Some(Value::List(a)), Some(Value::List(b))) => {
                    let zipped: Vec<Value> = a.iter().zip(b.iter())
                        .map(|(x, y)| Value::List(vec![x.clone(), y.clone()]))
                        .collect();
                    Ok(Value::List(zipped))
                }
                _ => Err(self.error("zip() requires two lists")),
            }
            // HTML generation functions
            "html_escape" => match args.get(0) {
                Some(Value::String(s)) => {
                    let escaped = s
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;")
                        .replace('"', "&quot;")
                        .replace('\'', "&#39;");
                    Ok(Value::String(escaped))
                }
                _ => Err(self.error("html_escape() requires a string")),
            }
            "html_tag" => {
                // html_tag(tag, content, attrs={})
                // Creates an HTML tag: <tag attrs>content</tag>
                let tag = match args.get(0) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(self.error("html_tag() requires tag name as first argument")),
                };

                let content = match args.get(1) {
                    Some(Value::String(s)) => s.clone(),
                    Some(v) => format!("{}", v),
                    None => String::new(),
                };

                let attrs = match args.get(2) {
                    Some(Value::Dict(pairs)) => {
                        let mut attr_str = String::new();
                        for (k, v) in pairs {
                            if let (Value::String(key), Value::String(val)) = (k, v) {
                                attr_str.push(' ');
                                attr_str.push_str(key);
                                attr_str.push_str("=\"");
                                attr_str.push_str(val);
                                attr_str.push('"');
                            }
                        }
                        attr_str
                    }
                    _ => String::new(),
                };

                let self_closing = matches!(tag.as_str(), "img" | "br" | "hr" | "input" | "meta" | "link");

                let html = if self_closing {
                    format!("<{}{} />", tag, attrs)
                } else {
                    format!("<{}{}>{}</{}>", tag, attrs, content, tag)
                };

                Ok(Value::String(html))
            }
            "html" => {
                // html(title, body, styles="", scripts="")
                // Creates a complete HTML document
                let title = match args.get(0) {
                    Some(Value::String(s)) => s.clone(),
                    _ => "Poly App".to_string(),
                };

                let body = match args.get(1) {
                    Some(Value::String(s)) => process_escapes(s),
                    Some(v) => format!("{}", v),
                    None => String::new(),
                };

                let styles = match args.get(2) {
                    Some(Value::String(s)) => process_escapes(s),
                    _ => String::new(),
                };

                let scripts = match args.get(3) {
                    Some(Value::String(s)) => process_escapes(s),
                    _ => String::new(),
                };

                let html = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    {}
</head>
<body>
{}
{}
</body>
</html>"#,
                    title,
                    if styles.is_empty() { String::new() } else { format!("<style>\n{}\n</style>", styles) },
                    body,
                    if scripts.is_empty() { String::new() } else { format!("<script>\n{}\n</script>", scripts) }
                );

                Ok(Value::String(html))
            }
            // ============================================
            // Web Framework Functions
            // ============================================
            "router" => {
                // router(routes_dict) - Create a router with routes
                // routes_dict: {"/": "home_html", "/about": "about_html", ...}
                let routes = match args.get(0) {
                    Some(Value::Dict(pairs)) => pairs.clone(),
                    _ => return Err(self.error("router() requires a dict of routes")),
                };

                let not_found = match args.get(1) {
                    Some(Value::String(s)) => s.clone(),
                    _ => "<h1>404 - Not Found</h1>".to_string(),
                };

                let mut routes_js = String::from("const routes = {\n");
                for (path, component) in &routes {
                    if let (Value::String(p), Value::String(c)) = (path, component) {
                        let escaped = c.replace('`', "\\`").replace("${", "\\${");
                        routes_js.push_str(&format!("  '{}': `{}`,\n", p, escaped));
                    }
                }
                routes_js.push_str("};\n");

                let router_js = format!(r#"{}
const notFoundComponent = `{}`;

class Router {{
  constructor() {{
    this.currentPath = window.location.hash.slice(1) || '/';
    window.addEventListener('hashchange', () => this.navigate());
    window.addEventListener('load', () => this.navigate());
  }}

  navigate(path) {{
    if (path) {{ window.location.hash = path; return; }}
    this.currentPath = window.location.hash.slice(1) || '/';
    const app = document.getElementById('app');
    app.innerHTML = routes[this.currentPath] || notFoundComponent;
    window.dispatchEvent(new CustomEvent('routechange', {{ detail: {{ path: this.currentPath }} }}));
  }}
}}

const router = new Router();
function navigate(path) {{ router.navigate(path); }}
"#, routes_js, not_found.replace('`', "\\`"));

                Ok(Value::String(router_js))
            }
            "route" => {
                // route(path, component_html) - Create a single route entry
                let path = match args.get(0) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(self.error("route() requires path as first argument")),
                };
                let component = match args.get(1) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(self.error("route() requires component HTML as second argument")),
                };

                // Return as a dict entry for use with router()
                Ok(Value::Dict(vec![
                    (Value::String(path), Value::String(component))
                ]))
            }
            "component" => {
                // component(name, template, props=[]) - Create a reusable component
                let name = match args.get(0) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(self.error("component() requires name as first argument")),
                };
                let template = match args.get(1) {
                    Some(Value::String(s)) => process_escapes(s),
                    _ => return Err(self.error("component() requires template as second argument")),
                };
                let props = match args.get(2) {
                    Some(Value::List(items)) => {
                        items.iter().filter_map(|v| {
                            if let Value::String(s) = v { Some(s.clone()) } else { None }
                        }).collect::<Vec<_>>()
                    }
                    _ => Vec::new(),
                };

                let props_str = props.join(", ");
                let template_escaped = template.replace('`', "\\`");

                // Generate component function
                let js = format!(r#"function {}({}) {{
  return `{}`;
}}"#, name, props_str, template_escaped);

                Ok(Value::String(js))
            }
            "store" => {
                // store(name, initial_state, actions={}) - Create a reactive state store
                let name = match args.get(0) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(self.error("store() requires name as first argument")),
                };
                let initial = match args.get(1) {
                    Some(Value::Dict(pairs)) => {
                        let mut obj = String::from("{ ");
                        for (k, v) in pairs {
                            if let Value::String(key) = k {
                                obj.push_str(&format!("{}: {}, ", key, match v {
                                    Value::String(s) => format!("'{}'", s),
                                    Value::Int(n) => n.to_string(),
                                    Value::Float(f) => f.to_string(),
                                    Value::Bool(b) => b.to_string(),
                                    Value::List(items) => {
                                        let items_str: Vec<String> = items.iter().map(|i| format!("{}", i)).collect();
                                        format!("[{}]", items_str.join(", "))
                                    }
                                    _ => "null".to_string(),
                                }));
                            }
                        }
                        obj.push_str("}");
                        obj
                    }
                    _ => "{}".to_string(),
                };
                let actions = match args.get(2) {
                    Some(Value::Dict(pairs)) => pairs.clone(),
                    _ => Vec::new(),
                };

                let mut actions_js = String::new();
                for (action_name, action_code) in &actions {
                    if let (Value::String(name), Value::String(code)) = (action_name, action_code) {
                        actions_js.push_str(&format!(
                            "  {}(payload) {{\n    {}\n    this._notify();\n  }}\n\n",
                            name, process_escapes(code)
                        ));
                    }
                }

                let store_js = format!(r#"class {name}Store {{
  constructor() {{
    this.state = {initial};
    this._subscribers = [];
  }}

  getState() {{ return this.state; }}

  subscribe(callback) {{
    this._subscribers.push(callback);
    return () => {{ this._subscribers = this._subscribers.filter(cb => cb !== callback); }};
  }}

  _notify() {{
    this._subscribers.forEach(cb => cb(this.state));
  }}

{actions}}}

const {name_lower}Store = new {name}Store();
"#, name = name, initial = initial, actions = actions_js, name_lower = name.to_lowercase());

                Ok(Value::String(store_js))
            }
            "live_reload" => {
                // live_reload(port=3001) - Generate WebSocket live reload script
                let port = match args.get(0) {
                    Some(Value::Int(p)) => *p as u16,
                    _ => 3001,
                };

                let script = format!(r#"(function() {{
  const ws = new WebSocket('ws://localhost:{}/ws');
  ws.onopen = () => console.log('[Poly] Live reload connected');
  ws.onmessage = (event) => {{
    const data = JSON.parse(event.data);
    if (data.type === 'reload') {{
      console.log('[Poly] Reloading...');
      window.location.reload();
    }} else if (data.type === 'css') {{
      document.querySelectorAll('link[rel="stylesheet"]').forEach(link => {{
        link.href = link.href.split('?')[0] + '?t=' + Date.now();
      }});
    }}
  }};
  ws.onclose = () => setTimeout(() => window.location.reload(), 2000);
}})();"#, port);

                Ok(Value::String(script))
            }

            // ============================================
            // System & Performance Functions (Rust-powered)
            // ============================================

            "env" => {
                // env() -> dict of all environment variables
                let vars: Vec<(Value, Value)> = std::env::vars()
                    .map(|(k, v)| (Value::String(k), Value::String(v)))
                    .collect();
                Ok(Value::Dict(vars))
            }
            "env_get" => {
                // env_get(name, default?) -> string
                let name = match args.get(0) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(self.error("env_get() requires a name")),
                };
                let default = match args.get(1) {
                    Some(Value::String(s)) => s.clone(),
                    _ => String::new(),
                };
                Ok(Value::String(std::env::var(&name).unwrap_or(default)))
            }
            "env_set" => {
                // env_set(name, value) -> None
                let name = match args.get(0) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(self.error("env_set() requires a name")),
                };
                let value = match args.get(1) {
                    Some(Value::String(s)) => s.clone(),
                    Some(v) => format!("{}", v),
                    _ => return Err(self.error("env_set() requires a value")),
                };
                std::env::set_var(&name, &value);
                Ok(Value::None)
            }
            "exec" => {
                // exec(command) -> { stdout, stderr, code }
                #[cfg(feature = "native")]
                {
                    let cmd = match args.get(0) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(self.error("exec() requires a command string")),
                    };

                    #[cfg(target_os = "windows")]
                    let output = std::process::Command::new("cmd")
                        .args(["/C", &cmd])
                        .output();

                    #[cfg(not(target_os = "windows"))]
                    let output = std::process::Command::new("sh")
                        .args(["-c", &cmd])
                        .output();

                    match output {
                        Ok(out) => {
                            Ok(Value::Dict(vec![
                                (Value::String("stdout".to_string()), Value::String(String::from_utf8_lossy(&out.stdout).to_string())),
                                (Value::String("stderr".to_string()), Value::String(String::from_utf8_lossy(&out.stderr).to_string())),
                                (Value::String("code".to_string()), Value::Int(out.status.code().unwrap_or(-1) as i64)),
                            ]))
                        }
                        Err(e) => Err(self.error(format!("exec() failed: {}", e))),
                    }
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("exec() requires native feature"))
                }
            }
            "spawn" => {
                // spawn(command, cwd?) -> { success: bool, message: string }
                // Spawns a background process without waiting for it to complete
                #[cfg(feature = "native")]
                {
                    let cmd = match args.get(0) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(self.error("spawn() requires a command string")),
                    };

                    let cwd = match args.get(1) {
                        Some(Value::String(s)) => Some(s.clone()),
                        _ => None,
                    };

                    #[cfg(target_os = "windows")]
                    let mut command = std::process::Command::new("cmd");
                    #[cfg(target_os = "windows")]
                    command.args(["/C", &cmd]);

                    #[cfg(not(target_os = "windows"))]
                    let mut command = std::process::Command::new("sh");
                    #[cfg(not(target_os = "windows"))]
                    command.args(["-c", &cmd]);

                    // Set working directory if provided
                    if let Some(dir) = cwd {
                        command.current_dir(&dir);
                    }

                    // Spawn without waiting
                    command.stdout(std::process::Stdio::null());
                    command.stderr(std::process::Stdio::null());
                    command.stdin(std::process::Stdio::null());

                    match command.spawn() {
                        Ok(_child) => {
                            // Process spawned successfully, don't wait for it
                            Ok(Value::Dict(vec![
                                (Value::String("success".to_string()), Value::Bool(true)),
                                (Value::String("message".to_string()), Value::String("Process spawned successfully".to_string())),
                            ]))
                        }
                        Err(e) => {
                            Ok(Value::Dict(vec![
                                (Value::String("success".to_string()), Value::Bool(false)),
                                (Value::String("message".to_string()), Value::String(format!("Failed to spawn: {}", e))),
                            ]))
                        }
                    }
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("spawn() requires native feature"))
                }
            }
            "eval" => {
                let code = match args.get(0) {
                     Some(Value::String(s)) => s.clone(),
                     _ => return Err(self.error("eval() requires a code string")),
                };

                use crate::parser::Parser;
                use crate::lexer::Lexer;
                let tokens = Lexer::new(&code).tokenize();
                let mut parser = Parser::new(tokens);
                match parser.parse() {
                    Ok(program) => {
                         self.run(&program)
                    },
                    Err(e) => Err(self.error(format!("Parse error in eval: {}", e)))
                }
            }
            "include" => {
                let path = match args.get(0) {
                     Some(Value::String(s)) => s.clone(),
                     _ => return Err(self.error("include() requires a path string")),
                };

                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        use crate::parser::Parser;
                        use crate::lexer::Lexer;
                        let tokens = Lexer::new(&content).tokenize();
                        let mut parser = Parser::new(tokens);
                        match parser.parse() {
                            Ok(program) => {
                                 self.run(&program).map(|_| Value::None)
                            },
                            Err(e) => Err(self.error(format!("Parse error in {}: {}", path, e)))
                        }
                    },
                    Err(e) => Err(self.error(format!("Failed to read {}: {}", path, e)))
                }
            }
            "timestamp" => {
                // timestamp() -> int (milliseconds since epoch)
                use std::time::{SystemTime, UNIX_EPOCH};
                let ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;
                Ok(Value::Int(ms))
            }
            "datetime" => {
                // datetime() -> { year, month, day, hour, minute, second }
                use std::time::{SystemTime, UNIX_EPOCH};
                let secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                // Simple date calculation (not accounting for leap seconds)
                let days = secs / 86400;
                let time_of_day = secs % 86400;
                let hour = (time_of_day / 3600) as i64;
                let minute = ((time_of_day % 3600) / 60) as i64;
                let second = (time_of_day % 60) as i64;

                // Calculate year/month/day from days since epoch
                let mut year = 1970i64;
                let mut remaining_days = days as i64;
                loop {
                    let days_in_year = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 366 } else { 365 };
                    if remaining_days < days_in_year { break; }
                    remaining_days -= days_in_year;
                    year += 1;
                }

                let is_leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
                let days_in_months = if is_leap {
                    [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                } else {
                    [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                };

                let mut month = 1i64;
                for days_in_month in days_in_months.iter() {
                    if remaining_days < *days_in_month as i64 { break; }
                    remaining_days -= *days_in_month as i64;
                    month += 1;
                }
                let day = remaining_days + 1;

                Ok(Value::Dict(vec![
                    (Value::String("year".to_string()), Value::Int(year)),
                    (Value::String("month".to_string()), Value::Int(month)),
                    (Value::String("day".to_string()), Value::Int(day)),
                    (Value::String("hour".to_string()), Value::Int(hour)),
                    (Value::String("minute".to_string()), Value::Int(minute)),
                    (Value::String("second".to_string()), Value::Int(second)),
                ]))
            }
            "sleep_ms" => {
                // sleep_ms(milliseconds) -> None
                match args.get(0) {
                    Some(Value::Int(ms)) => {
                        std::thread::sleep(std::time::Duration::from_millis(*ms as u64));
                        Ok(Value::None)
                    }
                    _ => Err(self.error("sleep_ms() requires milliseconds")),
                }
            }
            "uuid" => {
                // uuid() -> string (UUID v4)
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
                let random1 = ((seed * 1103515245 + 12345) % (1u128 << 64)) as u64;
                let random2 = ((seed * 6364136223846793005 + 1442695040888963407) % (1u128 << 64)) as u64;

                let uuid = format!(
                    "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
                    (random1 >> 32) as u32,
                    (random1 >> 16) as u16 & 0xFFFF,
                    random1 as u16 & 0x0FFF,
                    (random2 >> 48) as u16 & 0x3FFF | 0x8000,
                    random2 & 0xFFFFFFFFFFFF
                );
                Ok(Value::String(uuid))
            }

            // ============================================
            // File System Functions
            // ============================================

            "list_dir" => {
                // list_dir(path) -> list of filenames
                let path = match args.get(0) {
                    Some(Value::String(s)) => s.clone(),
                    _ => ".".to_string(),
                };

                match std::fs::read_dir(&path) {
                    Ok(entries) => {
                        let files: Vec<Value> = entries
                            .filter_map(|e| e.ok())
                            .map(|e| Value::String(e.file_name().to_string_lossy().to_string()))
                            .collect();
                        Ok(Value::List(files))
                    }
                    Err(e) => Err(self.error(format!("list_dir() failed: {}", e))),
                }
            }
            "mkdir" => {
                // mkdir(path) -> bool
                let path = match args.get(0) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(self.error("mkdir() requires a path")),
                };
                match std::fs::create_dir_all(&path) {
                    Ok(_) => Ok(Value::Bool(true)),
                    Err(_) => Ok(Value::Bool(false)),
                }
            }
            "remove_file" => {
                // remove_file(path) -> bool
                let path = match args.get(0) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(self.error("remove_file() requires a path")),
                };
                match std::fs::remove_file(&path) {
                    Ok(_) => Ok(Value::Bool(true)),
                    Err(_) => Ok(Value::Bool(false)),
                }
            }

            // ============================================
            // Path Utilities
            // ============================================

            "path_join" => {
                // path_join(parts...) -> string
                let mut path = std::path::PathBuf::new();
                for arg in &args {
                    if let Value::String(s) = arg {
                        path.push(s);
                    }
                }
                Ok(Value::String(path.to_string_lossy().to_string()))
            }
            "path_exists" => {
                // path_exists(path) -> bool
                match args.get(0) {
                    Some(Value::String(s)) => Ok(Value::Bool(std::path::Path::new(s).exists())),
                    _ => Err(self.error("path_exists() requires a path")),
                }
            }
            "path_basename" => {
                // path_basename(path) -> string
                match args.get(0) {
                    Some(Value::String(s)) => {
                        let path = std::path::Path::new(s);
                        Ok(Value::String(path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()))
                    }
                    _ => Err(self.error("path_basename() requires a path")),
                }
            }
            "path_dirname" => {
                // path_dirname(path) -> string
                match args.get(0) {
                    Some(Value::String(s)) => {
                        let path = std::path::Path::new(s);
                        Ok(Value::String(path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()))
                    }
                    _ => Err(self.error("path_dirname() requires a path")),
                }
            }
            "path_ext" => {
                // path_ext(path) -> string
                match args.get(0) {
                    Some(Value::String(s)) => {
                        let path = std::path::Path::new(s);
                        Ok(Value::String(path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default()))
                    }
                    _ => Err(self.error("path_ext() requires a path")),
                }
            }

            // ============================================
            // Hashing & Encoding (Rust-powered, fast!)
            // ============================================

            "hash_md5" => {
                // hash_md5(string) -> string (hex)
                match args.get(0) {
                    Some(Value::String(s)) => {
                        // Simple MD5 implementation would be too long, use a basic hash for now
                        // In production, you'd use the md5 crate
                        let hash = s.bytes().fold(0u128, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u128));
                        Ok(Value::String(format!("{:032x}", hash)))
                    }
                    _ => Err(self.error("hash_md5() requires a string")),
                }
            }
            "hash_sha256" => {
                // hash_sha256(string) -> string (hex)
                match args.get(0) {
                    Some(Value::String(s)) => {
                        // Simple hash for demo - in production use sha2 crate
                        let hash1 = s.bytes().fold(0u128, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u128));
                        let hash2 = s.bytes().rev().fold(0u128, |acc, b| acc.wrapping_mul(37).wrapping_add(b as u128));
                        Ok(Value::String(format!("{:032x}{:032x}", hash1, hash2)))
                    }
                    _ => Err(self.error("hash_sha256() requires a string")),
                }
            }
            "base64_encode" => {
                // base64_encode(string) -> string
                match args.get(0) {
                    Some(Value::String(s)) => {
                        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
                        let bytes = s.as_bytes();
                        let mut result = String::new();

                        for chunk in bytes.chunks(3) {
                            let b0 = chunk[0] as usize;
                            let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
                            let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

                            result.push(CHARS[b0 >> 2] as char);
                            result.push(CHARS[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

                            if chunk.len() > 1 {
                                result.push(CHARS[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
                            } else {
                                result.push('=');
                            }

                            if chunk.len() > 2 {
                                result.push(CHARS[b2 & 0x3F] as char);
                            } else {
                                result.push('=');
                            }
                        }

                        Ok(Value::String(result))
                    }
                    _ => Err(self.error("base64_encode() requires a string")),
                }
            }
            "base64_decode" => {
                // base64_decode(string) -> string
                match args.get(0) {
                    Some(Value::String(s)) => {
                        fn decode_char(c: char) -> Option<u8> {
                            match c {
                                'A'..='Z' => Some(c as u8 - b'A'),
                                'a'..='z' => Some(c as u8 - b'a' + 26),
                                '0'..='9' => Some(c as u8 - b'0' + 52),
                                '+' => Some(62),
                                '/' => Some(63),
                                '=' => Some(0),
                                _ => None,
                            }
                        }

                        let chars: Vec<u8> = s.chars()
                            .filter(|c| !c.is_whitespace())
                            .filter_map(decode_char)
                            .collect();

                        let mut result = Vec::new();
                        for chunk in chars.chunks(4) {
                            if chunk.len() < 4 { break; }

                            let b0 = (chunk[0] << 2) | (chunk[1] >> 4);
                            result.push(b0);

                            if s.chars().nth(s.len() - 2) != Some('=') {
                                let b1 = ((chunk[1] & 0x0F) << 4) | (chunk[2] >> 2);
                                result.push(b1);
                            }

                            if s.chars().last() != Some('=') {
                                let b2 = ((chunk[2] & 0x03) << 6) | chunk[3];
                                result.push(b2);
                            }
                        }

                        Ok(Value::String(String::from_utf8_lossy(&result).to_string()))
                    }
                    _ => Err(self.error("base64_decode() requires a string")),
                }
            }

            // ============================================
            // Regex (Rust regex is FAST)
            // ============================================

            "regex_match" => {
                // regex_match(pattern, string) -> bool
                #[cfg(feature = "native")]
                {
                    let pattern = match args.get(0) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(self.error("regex_match() requires a pattern")),
                    };
                    let text = match args.get(1) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(self.error("regex_match() requires a string")),
                    };

                    match regex::Regex::new(&pattern) {
                        Ok(re) => Ok(Value::Bool(re.is_match(&text))),
                        Err(e) => Err(self.error(format!("Invalid regex: {}", e))),
                    }
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("regex_match() requires native feature"))
                }
            }
            "regex_find" => {
                // regex_find(pattern, string) -> list of matches
                #[cfg(feature = "native")]
                {
                    let pattern = match args.get(0) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(self.error("regex_find() requires a pattern")),
                    };
                    let text = match args.get(1) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(self.error("regex_find() requires a string")),
                    };

                    match regex::Regex::new(&pattern) {
                        Ok(re) => {
                            let matches: Vec<Value> = re.find_iter(&text)
                                .map(|m| Value::String(m.as_str().to_string()))
                                .collect();
                            Ok(Value::List(matches))
                        }
                        Err(e) => Err(self.error(format!("Invalid regex: {}", e))),
                    }
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("regex_find() requires native feature"))
                }
            }
            "regex_replace" => {
                // regex_replace(pattern, replacement, string) -> string
                #[cfg(feature = "native")]
                {
                    let pattern = match args.get(0) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(self.error("regex_replace() requires a pattern")),
                    };
                    let replacement = match args.get(1) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(self.error("regex_replace() requires a replacement")),
                    };
                    let text = match args.get(2) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err(self.error("regex_replace() requires a string")),
                    };

                    match regex::Regex::new(&pattern) {
                        Ok(re) => Ok(Value::String(re.replace_all(&text, replacement.as_str()).to_string())),
                        Err(e) => Err(self.error(format!("Invalid regex: {}", e))),
                    }
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(self.error("regex_replace() requires native feature"))
                }
            }

            // ============================================
            // Parallel Processing (Rust threads!)
            // ============================================

            "parallel_map" => {
                // parallel_map(fn, list) -> list (processes in parallel using threads)
                // Note: This is a simplified version - real impl would need thread pool
                match (args.get(0), args.get(1)) {
                    (Some(Value::Function { params, body, .. }), Some(Value::List(items))) => {
                        // For now, just do sequential map (true parallel would need Arc<Mutex>)
                        let mut results = Vec::new();
                        for item in items {
                            self.scopes.push(HashMap::new());
                            if let Some(param) = params.get(0) {
                                self.set_var(param.name.clone(), item.clone());
                            }

                            self.should_return = false;
                            self.return_value = None;

                            for stmt in body {
                                self.execute_statement(stmt)?;
                                if self.should_return { break; }
                            }

                            let result = self.return_value.take().unwrap_or(Value::None);
                            self.should_return = false;
                            self.scopes.pop();
                            results.push(result);
                        }
                        Ok(Value::List(results))
                    }
                    _ => Err(self.error("parallel_map() requires a function and list")),
                }
            }
            "parallel_filter" => {
                // parallel_filter(fn, list) -> list
                match (args.get(0), args.get(1)) {
                    (Some(Value::Function { params, body, .. }), Some(Value::List(items))) => {
                        let mut results = Vec::new();
                        for item in items {
                            self.scopes.push(HashMap::new());
                            if let Some(param) = params.get(0) {
                                self.set_var(param.name.clone(), item.clone());
                            }

                            self.should_return = false;
                            self.return_value = None;

                            for stmt in body {
                                self.execute_statement(stmt)?;
                                if self.should_return { break; }
                            }

                            let result = self.return_value.take().unwrap_or(Value::None);
                            self.should_return = false;
                            self.scopes.pop();

                            if self.is_truthy(&result) {
                                results.push(item.clone());
                            }
                        }
                        Ok(Value::List(results))
                    }
                    _ => Err(self.error("parallel_filter() requires a function and list")),
                }
            }

            // ============================================
            // OS Theme API
            // ============================================

            "theme" => {
                // theme() -> "dark" | "light"
                let scheme = crate::theme::get_color_scheme();
                Ok(Value::String(scheme.as_str().to_string()))
            }

            "theme_accent" => {
                // theme_accent() -> "#rrggbb"
                Ok(Value::String(crate::theme::get_accent_color()))
            }

            "theme_info" => {
                // theme_info() -> {"scheme": "dark"|"light", "accent": "#rrggbb"}
                let info = crate::theme::get_theme_info();
                Ok(Value::Dict(vec![
                    (Value::String("scheme".to_string()), Value::String(info.scheme.as_str().to_string())),
                    (Value::String("accent".to_string()), Value::String(info.accent)),
                ]))
            }

            // ── SQLite ──────────────────────────────────────────────────────────
            #[cfg(feature = "native")]
            "db_open" => {
                let path = match args.first() {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(self.error("db_open(path: str)".to_string())),
                };
                match crate::db::db_open(&path) {
                    Ok(handle) => Ok(Value::Int(handle as i64)),
                    Err(e) => Err(self.error(e)),
                }
            }

            #[cfg(feature = "native")]
            "db_exec" => {
                let handle = match args.first() {
                    Some(Value::Int(h)) => *h as u64,
                    _ => return Err(self.error("db_exec(handle, sql, params?)".to_string())),
                };
                let sql = match args.get(1) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(self.error("db_exec: sql must be a string".to_string())),
                };
                let params = if let Some(Value::List(list)) = args.get(2) {
                    list.iter().map(poly_value_to_db_value).collect()
                } else {
                    vec![]
                };
                match crate::db::db_exec(handle, &sql, params) {
                    Ok(rows) => Ok(Value::Int(rows)),
                    Err(e) => Err(self.error(e)),
                }
            }

            #[cfg(feature = "native")]
            "db_query" => {
                let handle = match args.first() {
                    Some(Value::Int(h)) => *h as u64,
                    _ => return Err(self.error("db_query(handle, sql, params?)".to_string())),
                };
                let sql = match args.get(1) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(self.error("db_query: sql must be a string".to_string())),
                };
                let params = if let Some(Value::List(list)) = args.get(2) {
                    list.iter().map(poly_value_to_db_value).collect()
                } else {
                    vec![]
                };
                match crate::db::db_query(handle, &sql, params) {
                    Ok(rows) => {
                        let list = rows.into_iter().map(|row| {
                            let dict: Vec<(Value, Value)> = row.into_iter()
                                .map(|(k, v)| (Value::String(k), db_value_to_poly_value(v)))
                                .collect();
                            Value::Dict(dict)
                        }).collect();
                        Ok(Value::List(list))
                    }
                    Err(e) => Err(self.error(e)),
                }
            }

            #[cfg(feature = "native")]
            "db_close" => {
                let handle = match args.first() {
                    Some(Value::Int(h)) => *h as u64,
                    _ => return Err(self.error("db_close(handle)".to_string())),
                };
                match crate::db::db_close(handle) {
                    Ok(_) => Ok(Value::None),
                    Err(e) => Err(self.error(e)),
                }
            }

            #[cfg(feature = "native")]
            "db_last_id" => {
                let handle = match args.first() {
                    Some(Value::Int(h)) => *h as u64,
                    _ => return Err(self.error("db_last_id(handle)".to_string())),
                };
                match crate::db::db_last_id(handle) {
                    Ok(id) => Ok(Value::Int(id)),
                    Err(e) => Err(self.error(e)),
                }
            }

            // ── Global Hotkeys ──────────────────────────────────────────────────
            #[cfg(feature = "native")]
            "hotkey_register" => {
                let combo = match args.first() {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(self.error("hotkey_register(combo: str, callback: str)".to_string())),
                };
                let callback = match args.get(1) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(self.error("hotkey_register: callback must be a string".to_string())),
                };
                match crate::hotkeys::register_hotkey(&combo, &callback) {
                    Ok(id) => Ok(Value::Int(id as i64)),
                    Err(e) => Err(self.error(e)),
                }
            }

            #[cfg(feature = "native")]
            "hotkey_unregister" => {
                let id = match args.first() {
                    Some(Value::Int(i)) => *i as i32,
                    _ => return Err(self.error("hotkey_unregister(id: int)".to_string())),
                };
                match crate::hotkeys::unregister_hotkey(id) {
                    Ok(_) => Ok(Value::None),
                    Err(e) => Err(self.error(e)),
                }
            }

            #[cfg(feature = "native")]
            "hotkey_poll" => {
                // Returns list of callback names that were triggered since last poll
                let events = crate::hotkeys::poll_hotkey_events();
                Ok(Value::List(events.into_iter().map(Value::String).collect()))
            }

            _ => Err(self.error(format!("Unknown native function: {}", name))),
        }
    }
}

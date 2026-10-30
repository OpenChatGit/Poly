/// Database connection storage
#[cfg(feature = "native")]
static DB_CONNECTIONS: once_cell::sync::Lazy<std::sync::Mutex<std::collections::HashMap<u64, rusqlite::Connection>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

#[cfg(feature = "native")]
static DB_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Handle database requests
pub(super) fn handle_db_request(fn_name: &str, args: &serde_json::Value) -> String {
    #[cfg(feature = "native")]
    {
        use std::sync::atomic::Ordering;

        // Permission check - verify database permission
        if let Err(e) = poly::permissions::checks::database() {
            return serde_json::json!({"error": e}).to_string();
        }

        match fn_name {
            "__poly_db_open" => {
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(":memory:");

                // Additional check for file path if not in-memory
                if path != ":memory:" {
                    if let Err(e) = poly::permissions::checks::fs_write(path) {
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

//! Built-in SQLite API for Poly
//! db_open, db_exec, db_query, db_close - no plugins needed

#[cfg(feature = "native")]
use rusqlite::{Connection, params_from_iter};
#[cfg(feature = "native")]
use std::collections::HashMap;
#[cfg(feature = "native")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "native")]
use once_cell::sync::Lazy;

/// Global connection pool: handle_id -> Connection
#[cfg(feature = "native")]
static DB_POOL: Lazy<Mutex<HashMap<u64, Arc<Mutex<Connection>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[cfg(feature = "native")]
static DB_ID_COUNTER: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);

/// Open a SQLite database. Returns a handle (Int).
/// Pass ":memory:" for an in-memory database.
#[cfg(feature = "native")]
pub fn db_open(path: &str) -> Result<u64, String> {
    let conn = Connection::open(path)
        .map_err(|e| format!("db_open failed: {}", e))?;

    // Enable WAL mode for better concurrent performance
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| format!("db_open pragma failed: {}", e))?;

    let id = DB_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    DB_POOL
        .lock()
        .map_err(|e| format!("db pool lock error: {}", e))?
        .insert(id, Arc::new(Mutex::new(conn)));

    Ok(id)
}

/// Execute a statement (INSERT, UPDATE, DELETE, CREATE, ...).
/// Optional params list for parameterized queries.
/// Returns number of rows affected.
#[cfg(feature = "native")]
pub fn db_exec(handle: u64, sql: &str, params: Vec<DbValue>) -> Result<i64, String> {
    let pool = DB_POOL.lock().map_err(|e| format!("db pool lock: {}", e))?;
    let conn_arc = pool
        .get(&handle)
        .ok_or_else(|| format!("db handle {} not found", handle))?
        .clone();
    drop(pool);

    let conn = conn_arc.lock().map_err(|e| format!("db conn lock: {}", e))?;
    let rusql_params: Vec<Box<dyn rusqlite::ToSql>> = params
        .into_iter()
        .map(|v| -> Box<dyn rusqlite::ToSql> {
            match v {
                DbValue::Null => Box::new(rusqlite::types::Null),
                DbValue::Int(i) => Box::new(i),
                DbValue::Float(f) => Box::new(f),
                DbValue::Text(s) => Box::new(s),
                DbValue::Blob(b) => Box::new(b),
            }
        })
        .collect();

    let rows = conn
        .execute(sql, params_from_iter(rusql_params.iter().map(|p| p.as_ref())))
        .map_err(|e| format!("db_exec error: {}", e))?;

    Ok(rows as i64)
}

/// Query rows. Returns a list of dicts (column_name -> value).
#[cfg(feature = "native")]
pub fn db_query(handle: u64, sql: &str, params: Vec<DbValue>) -> Result<Vec<HashMap<String, DbValue>>, String> {
    let pool = DB_POOL.lock().map_err(|e| format!("db pool lock: {}", e))?;
    let conn_arc = pool
        .get(&handle)
        .ok_or_else(|| format!("db handle {} not found", handle))?
        .clone();
    drop(pool);

    let conn = conn_arc.lock().map_err(|e| format!("db conn lock: {}", e))?;
    let rusql_params: Vec<Box<dyn rusqlite::ToSql>> = params
        .into_iter()
        .map(|v| -> Box<dyn rusqlite::ToSql> {
            match v {
                DbValue::Null => Box::new(rusqlite::types::Null),
                DbValue::Int(i) => Box::new(i),
                DbValue::Float(f) => Box::new(f),
                DbValue::Text(s) => Box::new(s),
                DbValue::Blob(b) => Box::new(b),
            }
        })
        .collect();

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("db_query prepare error: {}", e))?;

    let col_names: Vec<String> = stmt
        .column_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let rows = stmt
        .query_map(
            params_from_iter(rusql_params.iter().map(|p| p.as_ref())),
            |row| {
                let mut map = HashMap::new();
                for (i, name) in col_names.iter().enumerate() {
                    let val: rusqlite::types::Value = row.get(i)?;
                    let db_val = match val {
                        rusqlite::types::Value::Null => DbValue::Null,
                        rusqlite::types::Value::Integer(n) => DbValue::Int(n),
                        rusqlite::types::Value::Real(f) => DbValue::Float(f),
                        rusqlite::types::Value::Text(s) => DbValue::Text(s),
                        rusqlite::types::Value::Blob(b) => DbValue::Blob(b),
                    };
                    map.insert(name.clone(), db_val);
                }
                Ok(map)
            },
        )
        .map_err(|e| format!("db_query error: {}", e))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("db_query row error: {}", e))?);
    }
    Ok(result)
}

/// Close a database handle.
#[cfg(feature = "native")]
pub fn db_close(handle: u64) -> Result<(), String> {
    DB_POOL
        .lock()
        .map_err(|e| format!("db pool lock: {}", e))?
        .remove(&handle);
    Ok(())
}

/// Get the last inserted row ID for a handle.
#[cfg(feature = "native")]
pub fn db_last_id(handle: u64) -> Result<i64, String> {
    let pool = DB_POOL.lock().map_err(|e| format!("db pool lock: {}", e))?;
    let conn_arc = pool
        .get(&handle)
        .ok_or_else(|| format!("db handle {} not found", handle))?
        .clone();
    drop(pool);
    let conn = conn_arc.lock().map_err(|e| format!("db conn lock: {}", e))?;
    Ok(conn.last_insert_rowid())
}

/// A SQLite value type used for params and results
#[cfg(feature = "native")]
#[derive(Debug, Clone)]
pub enum DbValue {
    Null,
    Int(i64),
    Float(f64),
    Text(String),
    Blob(Vec<u8>),
}

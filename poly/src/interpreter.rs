use std::collections::HashMap;
use crate::ast::*;

mod builtins;
mod collection_methods;

// Global stream sessions for HTTP streaming
#[cfg(feature = "native")]
use std::sync::Mutex;
#[cfg(feature = "native")]
use once_cell::sync::Lazy;

#[cfg(feature = "native")]
struct StreamSession {
    buffer: Vec<String>,
    done: bool,
    error: Option<String>,
}

#[cfg(feature = "native")]
static STREAM_SESSIONS: Lazy<Mutex<HashMap<u64, StreamSession>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

// Global HTTP server state for OAuth callbacks etc.
#[cfg(feature = "native")]
struct HttpServerState {
    running: bool,
    requests: Vec<HttpRequest>,
}

#[cfg(feature = "native")]
#[derive(Clone)]
struct HttpRequest {
    method: String,
    path: String,
    query: String,
    headers: Vec<(String, String)>,
    body: String,
}

#[cfg(feature = "native")]
static HTTP_SERVER: Lazy<Mutex<HttpServerState>> = Lazy::new(|| {
    Mutex::new(HttpServerState {
        running: false,
        requests: Vec::new(),
    })
});

#[cfg(feature = "native")]
static HTTP_SERVER_HANDLE: Lazy<Mutex<Option<std::thread::JoinHandle<()>>>> = Lazy::new(|| {
    Mutex::new(None)
});

#[cfg(feature = "native")]
static HTTP_SERVER_SHUTDOWN: Lazy<Mutex<bool>> = Lazy::new(|| {
    Mutex::new(false)
});

/// Process escape sequences in a string
fn process_escapes(s: &str) -> String {
    // Order matters: process \\ last to avoid double-processing
    s.replace("\\n", "\n")
     .replace("\\t", "\t")
     .replace("\\r", "\r")
     .replace("\\\"", "\"")
     .replace("\\'", "'")
     .replace("\\\\", "\\")
}

/// Runtime error with location information
#[derive(Debug)]
pub struct RuntimeError {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(line) = self.line {
            write!(f, "Error at line {}: {}", line, self.message)
        } else {
            write!(f, "Error: {}", self.message)
        }
    }
}

impl RuntimeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), line: None, column: None }
    }
    
    pub fn at_line(message: impl Into<String>, line: usize) -> Self {
        Self { message: message.into(), line: Some(line), column: None }
    }
}

pub struct Interpreter {
    globals: HashMap<String, Value>,
    scopes: Vec<HashMap<String, Value>>,
    classes: HashMap<String, ClassDef>,
    output: Vec<String>,
    should_return: bool,
    return_value: Option<Value>,
    should_break: bool,
    should_continue: bool,
    current_line: Option<usize>,
    pub stream_sender: Option<std::sync::mpsc::Sender<String>>,
}

#[derive(Clone)]
struct ClassDef {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    parent: Option<String>,
    methods: HashMap<String, (Vec<Param>, Vec<Statement>)>,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interp = Self {
            globals: HashMap::new(),
            scopes: vec![HashMap::new()],
            classes: HashMap::new(),
            output: Vec::new(),
            should_return: false,
            return_value: None,
            should_break: false,
            should_continue: false,
            current_line: None,
            stream_sender: None,
        };
        interp.register_builtins();
        interp
    }

    pub fn set_stream_sender(&mut self, sender: Option<std::sync::mpsc::Sender<String>>) {
        self.stream_sender = sender;
    }

    fn register_builtins(&mut self) {
        let builtins = [
            "print", "len", "range", "str", "int", "float", "type",
            "input", "append", "abs", "min", "max", "sum", "sorted",
            "stream",
            "reversed", "enumerate", "zip", "map", "filter", "any", "all",
            "isinstance", "hasattr", "getattr", "setattr", "list", "dict",
            "set", "tuple", "bool", "chr", "ord", "hex", "bin", "oct",
            "round", "pow", "divmod", "slice", "iter", "next", "open",
            // String methods
            "upper", "lower", "strip", "split", "join", "replace", "startswith", "endswith",
            "find", "find_from", "count", "isdigit", "isalpha", "isalnum", "format",
            // HTML generation
            "html", "html_escape", "html_tag",
            // Web framework - Routing, Components, State
            "router", "route", "component", "store", "live_reload",
            // List methods  
            "push", "pop", "insert", "remove", "index", "clear", "copy", "extend",
            // File I/O
            "read_file", "read_file_base64", "write_file", "file_exists", "list_dir", "mkdir", "remove_file",
            // HTTP (low-level primitives - user implements their own logic)
            "http_get", "http_get_steam", "http_get_parallel", "http_post", "http_post_json",
            // HTTP Streaming (for SSE/chunked responses)
            "http_stream_start", "http_stream_poll", "http_stream_close",
            // HTTP Server (for OAuth callbacks, webhooks, etc.)
            "http_server_start", "http_server_stop", "http_server_poll",
            // JSON
            "json_parse", "json_stringify",
            // System & Performance (Rust-powered)
            "env", "env_get", "env_set", "exec", "spawn", "include",
            "hash_md5", "hash_sha256", "base64_encode", "base64_decode",
            "uuid", "timestamp", "datetime", "sleep_ms",
            // Parallel Processing (Rust threads)
            "parallel_map", "parallel_filter",
            // Crypto & Encoding
            "encrypt", "decrypt", "hmac",
            // Path utilities
            "path_join", "path_exists", "path_basename", "path_dirname", "path_ext",
            // Regex
            "regex_match", "regex_find", "regex_replace",
            // OS Theme
            "theme", "theme_accent", "theme_info",
            // SQLite
            "db_open", "db_exec", "db_query", "db_close", "db_last_id",
            // Global Hotkeys
            "hotkey_register", "hotkey_unregister", "hotkey_poll",
        ];
        for name in builtins {
            self.globals.insert(name.to_string(), Value::NativeFunction(name.to_string()));
        }
    }
    
    /// Create an error with current line info
    fn error(&self, message: impl Into<String>) -> String {
        if let Some(line) = self.current_line {
            format!("Error at line {}: {}", line, message.into())
        } else {
            format!("Error: {}", message.into())
        }
    }

    pub fn run(&mut self, program: &Program) -> Result<Value, String> {
        let mut result = Value::None;
        for stmt in &program.statements {
            result = self.execute_statement(stmt)?;
            if self.should_return {
                break;
            }
        }
        Ok(result)
    }

    pub fn get_output(&self) -> &[String] { &self.output }

    fn execute_statement(&mut self, stmt: &Statement) -> Result<Value, String> {
        if self.should_return || self.should_break || self.should_continue {
            return Ok(Value::None);
        }

        match stmt {
            Statement::Let { name, value, .. } => {
                let val = self.evaluate(value)?;
                self.set_var(name.clone(), val);
                Ok(Value::None)
            }
            Statement::Var { name, value, .. } => {
                let val = self.evaluate(value)?;
                self.set_var(name.clone(), val);
                Ok(Value::None)
            }
            Statement::Assign(name, expr) => {
                let value = self.evaluate(expr)?;
                self.set_var(name.clone(), value);
                Ok(Value::None)
            }
            Statement::IndexAssign(target, index, value) => {
                self.execute_index_assign(target, index, value)
            }
            Statement::AttrAssign(target, attr, value) => {
                self.execute_attr_assign(target, attr, value)
            }
            Statement::If { condition, then_body, elif_branches, else_body } => {
                self.execute_if(condition, then_body, elif_branches, else_body)
            }
            Statement::While { condition, body } => {
                self.execute_while(condition, body)
            }
            Statement::For { var, iter, body } => {
                self.execute_for(var, iter, body)
            }
            Statement::FnDef { name, params, return_type: _, body } => {
                let func = Value::Function {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                };
                self.globals.insert(name.clone(), func);
                Ok(Value::None)
            }
            Statement::Return(expr) => {
                self.return_value = match expr {
                    Some(e) => Some(self.evaluate(e)?),
                    None => Some(Value::None),
                };
                self.should_return = true;
                Ok(self.return_value.clone().unwrap_or(Value::None))
            }
            Statement::ClassDef { name, parent, methods } => {
                self.define_class(name, parent, methods)
            }
            Statement::StructDef { name, parent, fields, methods } => {
                // Register struct as a class with typed fields
                self.define_struct(name, parent, fields, methods)
            }
            Statement::Expr(expr) => self.evaluate(expr),
            Statement::Import(module) => self.import_module(module),
            Statement::FromImport(module, names) => self.from_import(module, names),
            Statement::Pass => Ok(Value::None),
            Statement::Break => { self.should_break = true; Ok(Value::None) }
            Statement::Continue => { self.should_continue = true; Ok(Value::None) }
            Statement::Try { try_body, except_body, .. } => {
                self.execute_try(try_body, except_body)
            }
            Statement::Raise(expr) => {
                let val = self.evaluate(expr)?;
                Err(format!("{}", val))
            }
            Statement::Assert(condition, message) => {
                let cond_val = self.evaluate(condition)?;
                if !self.is_truthy(&cond_val) {
                    let msg = match message {
                        Some(expr) => format!("{}", self.evaluate(expr)?),
                        None => "Assertion failed".to_string(),
                    };
                    return Err(self.error(msg));
                }
                Ok(Value::None)
            }
            Statement::Del(expr) => {
                match expr {
                    Expr::Identifier(name) => {
                        // Remove from current scope
                        if let Some(scope) = self.scopes.last_mut() {
                            scope.remove(name);
                        }
                        self.globals.remove(name);
                    }
                    Expr::Index(target, index) => {
                        if let Expr::Identifier(name) = target.as_ref() {
                            let idx = self.evaluate(index)?;
                            if let Some(Value::List(mut items)) = self.get_var(name) {
                                if let Value::Int(i) = idx {
                                    let i = if i < 0 { (items.len() as i64 + i) as usize } else { i as usize };
                                    if i < items.len() {
                                        items.remove(i);
                                        self.set_var(name.clone(), Value::List(items));
                                    }
                                }
                            } else if let Some(Value::Dict(mut pairs)) = self.get_var(name) {
                                pairs.retain(|(k, _)| k != &idx);
                                self.set_var(name.clone(), Value::Dict(pairs));
                            }
                        }
                    }
                    _ => return Err(self.error("Invalid del target")),
                }
                Ok(Value::None)
            }
            Statement::Global(names) => {
                // Mark variables as global (for now, just ensure they exist in globals)
                for name in names {
                    if !self.globals.contains_key(name) {
                        self.globals.insert(name.clone(), Value::None);
                    }
                }
                Ok(Value::None)
            }
            Statement::Match { subject, arms } => {
                self.execute_match(subject, arms)
            }
        }
    }
    
    /// Import a module
    fn import_module(&mut self, module: &str) -> Result<Value, String> {
        match module {
            "math" => {
                // Create math module as a dict
                let math_module = Value::Dict(vec![
                    (Value::String("pi".to_string()), Value::Float(std::f64::consts::PI)),
                    (Value::String("e".to_string()), Value::Float(std::f64::consts::E)),
                    (Value::String("tau".to_string()), Value::Float(std::f64::consts::TAU)),
                    (Value::String("inf".to_string()), Value::Float(f64::INFINITY)),
                ]);
                self.globals.insert("math".to_string(), math_module);
                
                // Register math functions
                self.globals.insert("math_sqrt".to_string(), Value::NativeFunction("math_sqrt".to_string()));
                self.globals.insert("math_sin".to_string(), Value::NativeFunction("math_sin".to_string()));
                self.globals.insert("math_cos".to_string(), Value::NativeFunction("math_cos".to_string()));
                self.globals.insert("math_tan".to_string(), Value::NativeFunction("math_tan".to_string()));
                self.globals.insert("math_floor".to_string(), Value::NativeFunction("math_floor".to_string()));
                self.globals.insert("math_ceil".to_string(), Value::NativeFunction("math_ceil".to_string()));
                self.globals.insert("math_log".to_string(), Value::NativeFunction("math_log".to_string()));
                Ok(Value::None)
            }
            "random" => {
                self.globals.insert("random".to_string(), Value::NativeFunction("random".to_string()));
                self.globals.insert("randint".to_string(), Value::NativeFunction("randint".to_string()));
                self.globals.insert("choice".to_string(), Value::NativeFunction("choice".to_string()));
                self.globals.insert("shuffle".to_string(), Value::NativeFunction("shuffle".to_string()));
                Ok(Value::None)
            }
            "time" => {
                self.globals.insert("time".to_string(), Value::NativeFunction("time".to_string()));
                self.globals.insert("sleep".to_string(), Value::NativeFunction("sleep".to_string()));
                Ok(Value::None)
            }
            "json" => {
                self.globals.insert("json_dumps".to_string(), Value::NativeFunction("json_dumps".to_string()));
                self.globals.insert("json_loads".to_string(), Value::NativeFunction("json_loads".to_string()));
                Ok(Value::None)
            }
            _ => {
                // Try to load from file
                let file_path = format!("{}.poly", module);
                if std::path::Path::new(&file_path).exists() {
                    let source = std::fs::read_to_string(&file_path)
                        .map_err(|e| format!("Failed to read module {}: {}", module, e))?;
                    
                    let lexer = crate::lexer::Lexer::new(&source);
                    let tokens = lexer.tokenize();
                    let mut parser = crate::parser::Parser::new(tokens);
                    let program = parser.parse()?;
                    
                    // Execute module in current scope
                    for stmt in &program.statements {
                        self.execute_statement(stmt)?;
                    }
                    Ok(Value::None)
                } else {
                    Err(format!("Module not found: {}", module))
                }
            }
        }
    }
    
    /// Import specific names from a module
    fn from_import(&mut self, module: &str, _names: &[String]) -> Result<Value, String> {
        // First import the module
        self.import_module(module)?;
        
        // For built-in modules, the functions are already available
        // For file modules, the definitions are already in scope
        Ok(Value::None)
    }

    fn execute_index_assign(&mut self, target: &Expr, index: &Expr, value: &Expr) -> Result<Value, String> {
        let idx = self.evaluate(index)?;
        let val = self.evaluate(value)?;
        
        if let Expr::Identifier(name) = target {
            if let Some(list) = self.get_var(name) {
                if let Value::List(mut items) = list {
                    if let Value::Int(i) = idx {
                        let i = if i < 0 { (items.len() as i64 + i) as usize } else { i as usize };
                        if i < items.len() {
                            items[i] = val;
                            self.set_var(name.clone(), Value::List(items));
                        }
                    }
                } else if let Value::Dict(mut pairs) = list {
                    let mut found = false;
                    for (k, v) in pairs.iter_mut() {
                        if *k == idx { *v = val.clone(); found = true; break; }
                    }
                    if !found { pairs.push((idx, val)); }
                    self.set_var(name.clone(), Value::Dict(pairs));
                }
            }
        }
        Ok(Value::None)
    }

    fn execute_attr_assign(&mut self, target: &Expr, attr: &str, value: &Expr) -> Result<Value, String> {
        let val = self.evaluate(value)?;
        
        if let Expr::Identifier(name) = target {
            if let Some(Value::Instance { class_name, mut fields }) = self.get_var(name) {
                fields.insert(attr.to_string(), val);
                self.set_var(name.clone(), Value::Instance { class_name, fields });
            }
        }
        Ok(Value::None)
    }

    fn execute_if(&mut self, condition: &Expr, then_body: &[Statement], 
                  elif_branches: &[(Expr, Vec<Statement>)], else_body: &Option<Vec<Statement>>) -> Result<Value, String> {
        let cond_val = self.evaluate(condition)?;
        if self.is_truthy(&cond_val) {
            for stmt in then_body {
                self.execute_statement(stmt)?;
                if self.should_return || self.should_break || self.should_continue { break; }
            }
        } else {
            let mut executed = false;
            for (elif_cond, elif_body) in elif_branches {
                let elif_val = self.evaluate(elif_cond)?;
                if self.is_truthy(&elif_val) {
                    for stmt in elif_body {
                        self.execute_statement(stmt)?;
                        if self.should_return || self.should_break || self.should_continue { break; }
                    }
                    executed = true;
                    break;
                }
            }
            if !executed {
                if let Some(else_stmts) = else_body {
                    for stmt in else_stmts {
                        self.execute_statement(stmt)?;
                        if self.should_return || self.should_break || self.should_continue { break; }
                    }
                }
            }
        }
        Ok(Value::None)
    }

    fn execute_while(&mut self, condition: &Expr, body: &[Statement]) -> Result<Value, String> {
        loop {
            let cond_val = self.evaluate(condition)?;
            if !self.is_truthy(&cond_val) { break; }
            
            for stmt in body {
                self.execute_statement(stmt)?;
                if self.should_return { return Ok(Value::None); }
                if self.should_break { self.should_break = false; return Ok(Value::None); }
                if self.should_continue { self.should_continue = false; break; }
            }
        }
        Ok(Value::None)
    }

    fn execute_for(&mut self, var: &str, iter: &Expr, body: &[Statement]) -> Result<Value, String> {
        let iterable = self.evaluate(iter)?;
        let items = match iterable {
            Value::List(items) => items,
            Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
            Value::Dict(pairs) => pairs.into_iter().map(|(k, _)| k).collect(),
            _ => return Err(self.error("Can only iterate over list, string, or dict")),
        };
        
        for item in items {
            self.set_var(var.to_string(), item);
            for stmt in body {
                self.execute_statement(stmt)?;
                if self.should_return { return Ok(Value::None); }
                if self.should_break { self.should_break = false; return Ok(Value::None); }
                if self.should_continue { self.should_continue = false; break; }
            }
        }
        Ok(Value::None)
    }

    fn execute_try(&mut self, try_body: &[Statement], except_body: &[Statement]) -> Result<Value, String> {
        for stmt in try_body {
            if let Err(_) = self.execute_statement(stmt) {
                for except_stmt in except_body {
                    self.execute_statement(except_stmt)?;
                }
                return Ok(Value::None);
            }
        }
        Ok(Value::None)
    }

    fn define_class(&mut self, name: &str, parent: &Option<String>, methods: &[Method]) -> Result<Value, String> {
        let mut method_map = HashMap::new();
        for m in methods {
            method_map.insert(m.name.clone(), (m.params.clone(), m.body.clone()));
        }
        
        self.classes.insert(name.to_string(), ClassDef {
            name: name.to_string(),
            parent: parent.clone(),
            methods: method_map,
        });
        
        self.globals.insert(name.to_string(), Value::Class {
            name: name.to_string(),
            parent: parent.clone(),
            methods: methods.to_vec(),
        });
        
        Ok(Value::None)
    }

    fn define_struct(
        &mut self,
        name: &str,
        parent: &Option<String>,
        fields: &[crate::ast::StructField],
        methods: &[Method],
    ) -> Result<Value, String> {
        // Register methods like a class
        let mut method_map = HashMap::new();
        for m in methods {
            method_map.insert(m.name.clone(), (m.params.clone(), m.body.clone()));
        }

        // Build a synthetic __init__ that accepts all fields as keyword args
        // and stores them on self
        let mut init_params = vec![crate::ast::Param {
            name: "self".to_string(),
            type_annotation: None,
            modifier: None,
            default: None,
        }];
        let mut init_body = Vec::new();

        for field in fields {
            init_params.push(crate::ast::Param {
                name: field.name.clone(),
                type_annotation: field.type_annotation.clone(),
                modifier: None,
                default: field.default.clone(),
            });
            // self.field_name = field_name
            init_body.push(Statement::AttrAssign(
                Expr::Identifier("self".to_string()),
                field.name.clone(),
                Expr::Identifier(field.name.clone()),
            ));
        }

        method_map.insert("__init__".to_string(), (init_params.clone(), init_body.clone()));

        self.classes.insert(name.to_string(), ClassDef {
            name: name.to_string(),
            parent: parent.clone(),
            methods: method_map,
        });

        // Also expose as a callable class value
        let mut all_methods = methods.to_vec();
        all_methods.push(Method {
            name: "__init__".to_string(),
            params: init_params,
            return_type: None,
            body: init_body,
        });

        self.globals.insert(name.to_string(), Value::Class {
            name: name.to_string(),
            parent: parent.clone(),
            methods: all_methods,
        });

        Ok(Value::None)
    }

    fn execute_match(&mut self, subject: &Expr, arms: &[crate::ast::MatchArm]) -> Result<Value, String> {
        let val = self.evaluate(subject)?;

        for arm in arms {
            if let Some(bindings) = self.match_pattern(&arm.pattern, &val)? {
                // Push a new scope with bindings
                self.scopes.push(HashMap::new());
                for (k, v) in bindings {
                    self.set_var(k, v);
                }
                for stmt in &arm.body {
                    self.execute_statement(stmt)?;
                    if self.should_return || self.should_break || self.should_continue { break; }
                }
                self.scopes.pop();
                return Ok(Value::None);
            }
        }
        Ok(Value::None)
    }

    /// Returns Some(bindings) if pattern matches, None otherwise
    fn match_pattern(
        &mut self,
        pattern: &crate::ast::MatchPattern,
        val: &Value,
    ) -> Result<Option<Vec<(String, Value)>>, String> {
        use crate::ast::MatchPattern;
        match pattern {
            MatchPattern::Wildcard => Ok(Some(vec![])),
            MatchPattern::Bind(name) => Ok(Some(vec![(name.clone(), val.clone())])),
            MatchPattern::Literal(expr) => {
                let lit = self.evaluate(expr)?;
                if lit == *val { Ok(Some(vec![])) } else { Ok(None) }
            }
            MatchPattern::Type(type_name) => {
                let tag = crate::ast::TypeTag::from_str(type_name);
                if tag.check(val) {
                    Ok(Some(vec![]))
                } else {
                    Ok(None)
                }
            }
            MatchPattern::Range(start_expr, end_expr) => {
                let start = self.evaluate(start_expr)?;
                let end = self.evaluate(end_expr)?;
                let matches = match (val, &start, &end) {
                    (Value::Int(v), Value::Int(s), Value::Int(e)) => v >= s && v < e,
                    (Value::Float(v), Value::Float(s), Value::Float(e)) => v >= s && v < e,
                    _ => false,
                };
                if matches { Ok(Some(vec![])) } else { Ok(None) }
            }
            MatchPattern::Or(patterns) => {
                for p in patterns {
                    if let Some(bindings) = self.match_pattern(p, val)? {
                        return Ok(Some(bindings));
                    }
                }
                Ok(None)
            }
            MatchPattern::Guard(inner, guard_expr) => {
                if let Some(bindings) = self.match_pattern(inner, val)? {
                    // Temporarily bind variables to evaluate guard
                    self.scopes.push(HashMap::new());
                    for (k, v) in &bindings {
                        self.set_var(k.clone(), v.clone());
                    }
                    let guard_val = self.evaluate(guard_expr)?;
                    self.scopes.pop();
                    if self.is_truthy(&guard_val) {
                        Ok(Some(bindings))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn evaluate(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::None => Ok(Value::None),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Int(n) => Ok(Value::Int(*n)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::FString(parts) => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        FStringPart::Literal(s) => result.push_str(s),
                        FStringPart::Expr(expr) => {
                            let val = self.evaluate(expr)?;
                            result.push_str(&format!("{}", val));
                        }
                    }
                }
                Ok(Value::String(result))
            }
            Expr::List(items) => {
                let values: Result<Vec<_>, _> = items.iter().map(|e| self.evaluate(e)).collect();
                Ok(Value::List(values?))
            }
            Expr::Dict(pairs) => {
                let mut result = Vec::new();
                for (k, v) in pairs {
                    result.push((self.evaluate(k)?, self.evaluate(v)?));
                }
                Ok(Value::Dict(result))
            }
            Expr::ListComp { expr, var, iter, condition } => {
                self.evaluate_list_comp(expr, var, iter, condition.as_deref())
            }
            Expr::Identifier(name) => {
                self.get_var(name).ok_or_else(|| self.error(format!("Undefined variable: {}", name)))
            }
            Expr::Index(target, index) => self.evaluate_index(target, index),
            Expr::Slice(target, start, end) => self.evaluate_slice(target, start, end),
            Expr::Attribute(target, attr) => self.evaluate_attribute(target, attr),
            Expr::BinaryOp(left, op, right) => {
                let left_val = self.evaluate(left)?;
                let right_val = self.evaluate(right)?;
                self.apply_binary_op(&left_val, op, &right_val)
            }
            Expr::UnaryOp(op, expr) => {
                let val = self.evaluate(expr)?;
                self.apply_unary_op(op, &val)
            }
            Expr::Ternary(cond, then_expr, else_expr) => {
                let cond_val = self.evaluate(cond)?;
                if self.is_truthy(&cond_val) {
                    self.evaluate(then_expr)
                } else {
                    self.evaluate(else_expr)
                }
            }
            Expr::Call(callee, args) => {
                // Check if this is a method call (obj.method())
                if let Expr::Attribute(target, method_name) = callee.as_ref() {
                    let target_val = self.evaluate(target)?;
                    
                    // Handle string method calls
                    if let Value::String(s) = &target_val {
                        let arg_values: Vec<Value> = args.iter().map(|a| self.evaluate(a)).collect::<Result<Vec<_>, _>>()?;
                        return self.call_string_method(s, method_name, arg_values);
                    }
                    
                    // Handle list method calls
                    if let Value::List(items) = &target_val {
                        let arg_values: Vec<Value> = args.iter().map(|a| self.evaluate(a)).collect::<Result<Vec<_>, _>>()?;
                        let result = self.call_list_method(items.clone(), method_name, arg_values)?;
                        
                        // For mutating methods, update the original variable
                        if matches!(method_name.as_str(), "append" | "push" | "insert" | "remove" | "pop" | "clear" | "extend" | "sort" | "reverse") {
                            if let Expr::Identifier(var_name) = target.as_ref() {
                                if let Value::List(new_items) = &result {
                                    self.set_var(var_name.clone(), Value::List(new_items.clone()));
                                }
                            }
                        }
                        return Ok(result);
                    }
                    
                    // Handle dict method calls
                    if let Value::Dict(pairs) = &target_val {
                        let arg_values: Vec<Value> = args.iter().map(|a| self.evaluate(a)).collect::<Result<Vec<_>, _>>()?;
                        return self.call_dict_method(pairs.clone(), method_name, arg_values);
                    }
                    
                    // Handle instance method calls
                    if let Value::Instance { class_name, fields: _ } = &target_val {
                        let method = self.classes.get(class_name)
                            .and_then(|c| c.methods.get(method_name))
                            .cloned();
                        
                        if let Some((params, body)) = method {
                            self.scopes.push(HashMap::new());
                            // Bind self
                            self.set_var("self".to_string(), target_val.clone());
                            // Bind other arguments (skip 'self' in params)
                            for (i, param) in params.iter().skip(1).enumerate() {
                                let value = if i < args.len() {
                                    self.evaluate(&args[i])?
                                } else if let Some(default_expr) = &param.default {
                                    self.evaluate(default_expr)?
                                } else {
                                    return Err(self.error(format!("Missing required argument: {}", param.name)));
                                };
                                self.set_var(param.name.clone(), value);
                            }
                            
                            self.should_return = false;
                            self.return_value = None;
                            
                            for stmt in &body {
                                self.execute_statement(stmt)?;
                                if self.should_return { break; }
                            }
                            
                            // Get potentially modified self and update the original variable
                            if let Some(modified_self) = self.get_var("self") {
                                // Update the original instance variable
                                if let Expr::Identifier(var_name) = target.as_ref() {
                                    self.scopes.pop();
                                    self.set_var(var_name.clone(), modified_self);
                                } else {
                                    self.scopes.pop();
                                }
                            } else {
                                self.scopes.pop();
                            }
                            
                            let result = self.return_value.take().unwrap_or(Value::None);
                            self.should_return = false;
                            return Ok(result);
                        }
                    }
                }
                
                let func = self.evaluate(callee)?;
                let arg_values: Result<Vec<_>, _> = args.iter().map(|a| self.evaluate(a)).collect();
                self.call_function(func, arg_values?)
            }
            Expr::Lambda(params, body) => {
                Ok(Value::Function {
                    name: "<lambda>".to_string(),
                    params: params.clone(),
                    body: vec![Statement::Return(Some(*body.clone()))],
                })
            }
            Expr::CallWithKwargs(callee, args, kwargs) => {
                // For now, treat kwargs as additional args or handle specially
                let func = self.evaluate(callee)?;
                let mut arg_values: Vec<Value> = args.iter().map(|a| self.evaluate(a)).collect::<Result<Vec<_>, _>>()?;
                
                // Store kwargs in a dict for functions that need them
                let mut kwargs_dict = Vec::new();
                for (key, val) in kwargs {
                    kwargs_dict.push((Value::String(key.clone()), self.evaluate(val)?));
                }
                
                // If function expects kwargs, pass them
                if !kwargs_dict.is_empty() {
                    arg_values.push(Value::Dict(kwargs_dict));
                }
                
                self.call_function(func, arg_values)
            }
            Expr::Widget { widget_type, props, children } => {
                // Evaluate widget and return as Widget value
                let mut eval_props = Vec::new();
                for (key, val) in props {
                    eval_props.push((key.clone(), self.evaluate(val)?));
                }
                
                let mut eval_children = Vec::new();
                for child in children {
                    if let Ok(Value::Widget(node)) = self.evaluate(child) {
                        eval_children.push(node);
                    } else if let Ok(val) = self.evaluate(child) {
                        // Convert non-widget to Text widget
                        eval_children.push(WidgetNode {
                            widget_type: "Text".to_string(),
                            props: vec![("text".to_string(), val)],
                            children: vec![],
                        });
                    }
                }
                
                Ok(Value::Widget(WidgetNode {
                    widget_type: widget_type.clone(),
                    props: eval_props,
                    children: eval_children,
                }))
            }
        }
    }

    fn evaluate_list_comp(&mut self, expr: &Expr, var: &str, iter: &Expr, condition: Option<&Expr>) -> Result<Value, String> {
        let iterable = self.evaluate(iter)?;
        let items = match iterable {
            Value::List(items) => items,
            _ => return Err(self.error("List comprehension requires iterable")),
        };
        
        let mut result = Vec::new();
        self.scopes.push(HashMap::new());
        
        for item in items {
            self.set_var(var.to_string(), item);
            
            let include = if let Some(cond) = condition {
                let cond_val = self.evaluate(cond)?;
                self.is_truthy(&cond_val)
            } else { true };
            
            if include {
                result.push(self.evaluate(expr)?);
            }
        }
        
        self.scopes.pop();
        Ok(Value::List(result))
    }

    fn evaluate_index(&mut self, target: &Expr, index: &Expr) -> Result<Value, String> {
        let target_val = self.evaluate(target)?;
        let index_val = self.evaluate(index)?;
        
        match (target_val, index_val) {
            (Value::List(items), Value::Int(i)) => {
                let i = if i < 0 { (items.len() as i64 + i) as usize } else { i as usize };
                items.get(i).cloned().ok_or_else(|| self.error("Index out of bounds"))
            }
            (Value::Dict(pairs), key) => {
                for (k, v) in pairs { if k == key { return Ok(v); } }
                Err(self.error("Key not found"))
            }
            (Value::String(s), Value::Int(i)) => {
                let i = if i < 0 { (s.len() as i64 + i) as usize } else { i as usize };
                s.chars().nth(i).map(|c| Value::String(c.to_string()))
                    .ok_or_else(|| self.error("Index out of bounds"))
            }
            _ => Err(self.error("Invalid index operation")),
        }
    }
    
    fn evaluate_slice(&mut self, target: &Expr, start: &Option<Box<Expr>>, end: &Option<Box<Expr>>) -> Result<Value, String> {
        let target_val = self.evaluate(target)?;
        
        let start_idx = match start {
            Some(expr) => {
                match self.evaluate(expr)? {
                    Value::Int(i) => Some(i),
                    _ => return Err(self.error("Slice index must be an integer")),
                }
            }
            None => None,
        };
        
        let end_idx = match end {
            Some(expr) => {
                match self.evaluate(expr)? {
                    Value::Int(i) => Some(i),
                    _ => return Err(self.error("Slice index must be an integer")),
                }
            }
            None => None,
        };
        
        match target_val {
            Value::List(items) => {
                let len = items.len() as i64;
                let start = start_idx.map(|i| if i < 0 { (len + i).max(0) } else { i.min(len) }).unwrap_or(0) as usize;
                let end = end_idx.map(|i| if i < 0 { (len + i).max(0) } else { i.min(len) }).unwrap_or(len) as usize;
                Ok(Value::List(items[start..end.min(items.len())].to_vec()))
            }
            Value::String(s) => {
                let len = s.len() as i64;
                let start = start_idx.map(|i| if i < 0 { (len + i).max(0) } else { i.min(len) }).unwrap_or(0) as usize;
                let end = end_idx.map(|i| if i < 0 { (len + i).max(0) } else { i.min(len) }).unwrap_or(len) as usize;
                Ok(Value::String(s.chars().skip(start).take(end.saturating_sub(start)).collect()))
            }
            _ => Err(self.error("Can only slice lists and strings")),
        }
    }

    fn evaluate_attribute(&mut self, target: &Expr, attr: &str) -> Result<Value, String> {
        let target_val = self.evaluate(target)?;
        
        match &target_val {
            Value::Instance { fields, class_name } => {
                if let Some(val) = fields.get(attr) {
                    return Ok(val.clone());
                }
                // Clone method info to avoid borrow issues
                let method = self.classes.get(class_name)
                    .and_then(|c| c.methods.get(attr))
                    .cloned();
                    
                if let Some((params, body)) = method {
                    return Ok(Value::Function {
                        name: attr.to_string(),
                        params,
                        body,
                    });
                }
                Err(self.error(format!("No attribute '{}' on instance", attr)))
            }
            Value::String(s) => {
                // String attributes/methods
                match attr {
                    "length" => Ok(Value::Int(s.len() as i64)),
                    _ => Ok(Value::NativeFunction(format!("str.{}", attr))),
                }
            }
            Value::List(items) => {
                // List attributes
                match attr {
                    "length" => Ok(Value::Int(items.len() as i64)),
                    _ => Ok(Value::NativeFunction(format!("list.{}", attr))),
                }
            }
            Value::Dict(pairs) => {
                // Dict attributes
                match attr {
                    "length" => Ok(Value::Int(pairs.len() as i64)),
                    "keys" => Ok(Value::NativeFunction("dict.keys".to_string())),
                    "values" => Ok(Value::NativeFunction("dict.values".to_string())),
                    "items" => Ok(Value::NativeFunction("dict.items".to_string())),
                    _ => Ok(Value::NativeFunction(format!("dict.{}", attr))),
                }
            }
            _ => Err(self.error(format!("No attribute '{}' on {:?}", attr, target_val))),
        }
    }

    fn call_function(&mut self, func: Value, args: Vec<Value>) -> Result<Value, String> {
        match func {
            Value::Function { params, body, name } => {
                self.scopes.push(HashMap::new());
                
                // Bind parameters with default value support
                let required_count = params.iter().filter(|p| p.default.is_none() && p.name != "self").count();
                if args.len() < required_count {
                    return Err(self.error(format!(
                        "{}() takes at least {} argument(s) but {} were given",
                        name, required_count, args.len()
                    )));
                }
                if args.len() > params.len() {
                    return Err(self.error(format!(
                        "{}() takes at most {} argument(s) but {} were given",
                        name, params.len(), args.len()
                    )));
                }
                
                for (i, param) in params.iter().enumerate() {
                    let value = if i < args.len() {
                        args[i].clone()
                    } else if let Some(default_expr) = &param.default {
                        self.evaluate(default_expr)?
                    } else {
                        return Err(self.error(format!("Missing required argument: {}", param.name)));
                    };

                    // Type-check if annotation is present
                    if let Some(ann) = &param.type_annotation {
                        let tag = crate::ast::TypeTag::from_str(ann);
                        if !tag.check(&value) {
                            let actual = crate::ast::TypeTag::of_value(&value);
                            return Err(self.error(format!(
                                "{}(): argument '{}' expected {} but got {}",
                                name, param.name, tag.name(), actual.name()
                            )));
                        }
                    }

                    self.set_var(param.name.clone(), value);
                }
                
                self.should_return = false;
                self.return_value = None;
                
                for stmt in &body {
                    self.execute_statement(stmt)?;
                    if self.should_return { break; }
                }
                
                self.scopes.pop();
                self.should_return = false;
                Ok(self.return_value.take().unwrap_or(Value::None))
            }
            Value::Class { name, .. } => {
                let mut instance = Value::Instance {
                    class_name: name.clone(),
                    fields: HashMap::new(),
                };
                
                // Clone the method info to avoid borrow issues
                let init_method = self.classes.get(&name)
                    .and_then(|c| c.methods.get("__init__").or(c.methods.get("init")))
                    .cloned();
                
                if let Some((params, body)) = init_method {
                    self.scopes.push(HashMap::new());
                    self.set_var("self".to_string(), instance.clone());
                    
                    // Bind parameters (skip 'self')
                    for (i, param) in params.iter().skip(1).enumerate() {
                        let value = if i < args.len() {
                            args[i].clone()
                        } else if let Some(default_expr) = &param.default {
                            self.evaluate(default_expr)?
                        } else {
                            return Err(self.error(format!("Missing required argument: {}", param.name)));
                        };
                        self.set_var(param.name.clone(), value);
                    }
                    
                    for stmt in &body { 
                        self.execute_statement(stmt)?; 
                    }
                    instance = self.get_var("self").unwrap_or(instance);
                    self.scopes.pop();
                }
                Ok(instance)
            }
            Value::NativeFunction(name) => self.call_native(&name, args),
            _ => Err(self.error("Not a function")),
        }
    }
    
    /// Call a string method
    fn call_string_method(&mut self, s: &str, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "upper" => Ok(Value::String(s.to_uppercase())),
            "lower" => Ok(Value::String(s.to_lowercase())),
            "strip" | "trim" => Ok(Value::String(s.trim().to_string())),
            "lstrip" => Ok(Value::String(s.trim_start().to_string())),
            "rstrip" => Ok(Value::String(s.trim_end().to_string())),
            "split" => {
                let sep = match args.get(0) {
                    Some(Value::String(sep)) => sep.as_str(),
                    _ => " ",
                };
                if sep == " " {
                    Ok(Value::List(s.split_whitespace().map(|p| Value::String(p.to_string())).collect()))
                } else {
                    Ok(Value::List(s.split(sep).map(|p| Value::String(p.to_string())).collect()))
                }
            }
            "join" => {
                match args.get(0) {
                    Some(Value::List(items)) => {
                        let strings: Vec<String> = items.iter().map(|v| format!("{}", v)).collect();
                        Ok(Value::String(strings.join(s)))
                    }
                    _ => Err(self.error("join() requires a list")),
                }
            }
            "replace" => {
                match (args.get(0), args.get(1)) {
                    (Some(Value::String(old)), Some(Value::String(new))) => {
                        Ok(Value::String(s.replace(old.as_str(), new.as_str())))
                    }
                    _ => Err(self.error("replace() requires two strings")),
                }
            }
            "startswith" => {
                match args.get(0) {
                    Some(Value::String(prefix)) => Ok(Value::Bool(s.starts_with(prefix.as_str()))),
                    _ => Err(self.error("startswith() requires a string")),
                }
            }
            "endswith" => {
                match args.get(0) {
                    Some(Value::String(suffix)) => Ok(Value::Bool(s.ends_with(suffix.as_str()))),
                    _ => Err(self.error("endswith() requires a string")),
                }
            }
            "find" => {
                match args.get(0) {
                    Some(Value::String(sub)) => {
                        Ok(Value::Int(s.find(sub.as_str()).map(|i| i as i64).unwrap_or(-1)))
                    }
                    _ => Err(self.error("find() requires a string")),
                }
            }
            "find_from" => {
                match (args.get(0), args.get(1)) {
                    (Some(Value::String(sub)), Some(Value::Int(start))) => {
                        let start_idx = (*start).max(0) as usize;
                        if start_idx >= s.len() {
                            Ok(Value::Int(-1))
                        } else {
                            // Ensure we're at a valid UTF-8 boundary
                            let safe_start = if s.is_char_boundary(start_idx) {
                                start_idx
                            } else {
                                // Find next valid boundary
                                (start_idx..s.len()).find(|&i| s.is_char_boundary(i)).unwrap_or(s.len())
                            };
                            if safe_start >= s.len() {
                                Ok(Value::Int(-1))
                            } else {
                                Ok(Value::Int(s[safe_start..].find(sub.as_str()).map(|i| (i + safe_start) as i64).unwrap_or(-1)))
                            }
                        }
                    }
                    _ => Err(self.error("find_from() requires a string and start index")),
                }
            }
            "rfind" => {
                match args.get(0) {
                    Some(Value::String(sub)) => {
                        Ok(Value::Int(s.rfind(sub.as_str()).map(|i| i as i64).unwrap_or(-1)))
                    }
                    _ => Err(self.error("rfind() requires a string")),
                }
            }
            "extract_numbers" => {
                // Extract all numbers from a string, returns list of ints
                let mut numbers = Vec::new();
                let mut current_num = String::new();
                
                for ch in s.chars() {
                    if ch.is_ascii_digit() {
                        current_num.push(ch);
                    } else if !current_num.is_empty() {
                        if let Ok(n) = current_num.parse::<i64>() {
                            numbers.push(Value::Int(n));
                        }
                        current_num.clear();
                    }
                }
                // Don't forget the last number
                if !current_num.is_empty() {
                    if let Ok(n) = current_num.parse::<i64>() {
                        numbers.push(Value::Int(n));
                    }
                }
                
                Ok(Value::List(numbers))
            }
            "extract_number" => {
                // Extract first number from a string, returns int or none
                let mut current_num = String::new();
                let mut found = false;
                
                for ch in s.chars() {
                    if ch.is_ascii_digit() {
                        current_num.push(ch);
                        found = true;
                    } else if found {
                        break;
                    }
                }
                
                if !current_num.is_empty() {
                    if let Ok(n) = current_num.parse::<i64>() {
                        return Ok(Value::Int(n));
                    }
                }
                Ok(Value::None)
            }
            "count" => {
                match args.get(0) {
                    Some(Value::String(sub)) => {
                        Ok(Value::Int(s.matches(sub.as_str()).count() as i64))
                    }
                    _ => Err(self.error("count() requires a string")),
                }
            }
            "isdigit" => Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))),
            "isalpha" => Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_alphabetic()))),
            "isalnum" => Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_alphanumeric()))),
            "isspace" => Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_whitespace()))),
            "isupper" => Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| !c.is_alphabetic() || c.is_uppercase()))),
            "islower" => Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| !c.is_alphabetic() || c.is_lowercase()))),
            "capitalize" => {
                let mut chars = s.chars();
                let result = match chars.next() {
                    Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
                    None => String::new(),
                };
                Ok(Value::String(result))
            }
            "title" => {
                let result = s.split_whitespace()
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
                            None => String::new(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                Ok(Value::String(result))
            }
            "center" => {
                match args.get(0) {
                    Some(Value::Int(width)) => {
                        let width = *width as usize;
                        let fill = match args.get(1) {
                            Some(Value::String(f)) if !f.is_empty() => f.chars().next().unwrap(),
                            _ => ' ',
                        };
                        if s.len() >= width {
                            Ok(Value::String(s.to_string()))
                        } else {
                            let padding = width - s.len();
                            let left = padding / 2;
                            let right = padding - left;
                            Ok(Value::String(format!("{}{}{}", fill.to_string().repeat(left), s, fill.to_string().repeat(right))))
                        }
                    }
                    _ => Err(self.error("center() requires a width")),
                }
            }
            "zfill" => {
                match args.get(0) {
                    Some(Value::Int(width)) => {
                        let width = *width as usize;
                        if s.len() >= width {
                            Ok(Value::String(s.to_string()))
                        } else {
                            Ok(Value::String(format!("{:0>width$}", s, width = width)))
                        }
                    }
                    _ => Err(self.error("zfill() requires a width")),
                }
            }
            "encode" => Ok(Value::List(s.bytes().map(|b| Value::Int(b as i64)).collect())),
            _ => Err(self.error(format!("Unknown string method: {}", method))),
        }
    }
    
    /// Call a list method
    fn apply_binary_op(&self, left: &Value, op: &BinOp, right: &Value) -> Result<Value, String> {
        match (left, op, right) {
            // Arithmetic
            (Value::Int(a), BinOp::Add, Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Int(a), BinOp::Sub, Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Int(a), BinOp::Mul, Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Int(a), BinOp::Div, Value::Int(b)) => Ok(Value::Float(*a as f64 / *b as f64)),
            (Value::Int(a), BinOp::FloorDiv, Value::Int(b)) => Ok(Value::Int(a / b)),
            (Value::Int(a), BinOp::Mod, Value::Int(b)) => Ok(Value::Int(a % b)),
            (Value::Int(a), BinOp::Pow, Value::Int(b)) => Ok(Value::Int(a.pow(*b as u32))),
            
            (Value::Float(a), BinOp::Add, Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Float(a), BinOp::Sub, Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Float(a), BinOp::Mul, Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Float(a), BinOp::Div, Value::Float(b)) => Ok(Value::Float(a / b)),
            (Value::Float(a), BinOp::Pow, Value::Float(b)) => Ok(Value::Float(a.powf(*b))),
            
            // Mixed int/float
            (Value::Int(a), BinOp::Add, Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), BinOp::Add, Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::Int(a), BinOp::Sub, Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), BinOp::Sub, Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
            (Value::Int(a), BinOp::Mul, Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), BinOp::Mul, Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
            (Value::Int(a), BinOp::Div, Value::Float(b)) => Ok(Value::Float(*a as f64 / b)),
            (Value::Float(a), BinOp::Div, Value::Int(b)) => Ok(Value::Float(a / *b as f64)),
            (Value::Int(a), BinOp::Pow, Value::Float(b)) => Ok(Value::Float((*a as f64).powf(*b))),
            (Value::Float(a), BinOp::Pow, Value::Int(b)) => Ok(Value::Float(a.powf(*b as f64))),
            
            // String operations
            (Value::String(a), BinOp::Add, Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (Value::String(a), BinOp::Mul, Value::Int(b)) => Ok(Value::String(a.repeat(*b as usize))),
            (Value::Int(a), BinOp::Mul, Value::String(b)) => Ok(Value::String(b.repeat(*a as usize))),
            
            // List operations
            (Value::List(a), BinOp::Add, Value::List(b)) => {
                let mut result = a.clone();
                result.extend(b.clone());
                Ok(Value::List(result))
            }
            (Value::List(a), BinOp::Mul, Value::Int(b)) => {
                let mut result = Vec::new();
                for _ in 0..*b { result.extend(a.clone()); }
                Ok(Value::List(result))
            }
            
            // Comparisons
            (Value::Int(a), BinOp::Eq, Value::Int(b)) => Ok(Value::Bool(a == b)),
            (Value::Int(a), BinOp::NotEq, Value::Int(b)) => Ok(Value::Bool(a != b)),
            (Value::Int(a), BinOp::Lt, Value::Int(b)) => Ok(Value::Bool(a < b)),
            (Value::Int(a), BinOp::Gt, Value::Int(b)) => Ok(Value::Bool(a > b)),
            (Value::Int(a), BinOp::LtEq, Value::Int(b)) => Ok(Value::Bool(a <= b)),
            (Value::Int(a), BinOp::GtEq, Value::Int(b)) => Ok(Value::Bool(a >= b)),
            
            (Value::Float(a), BinOp::Lt, Value::Float(b)) => Ok(Value::Bool(a < b)),
            (Value::Float(a), BinOp::Gt, Value::Float(b)) => Ok(Value::Bool(a > b)),
            
            (Value::String(a), BinOp::Eq, Value::String(b)) => Ok(Value::Bool(a == b)),
            (Value::String(a), BinOp::NotEq, Value::String(b)) => Ok(Value::Bool(a != b)),
            (Value::String(a), BinOp::Lt, Value::String(b)) => Ok(Value::Bool(a < b)),
            (Value::String(a), BinOp::Gt, Value::String(b)) => Ok(Value::Bool(a > b)),
            
            (Value::Bool(a), BinOp::Eq, Value::Bool(b)) => Ok(Value::Bool(a == b)),
            (Value::Bool(a), BinOp::NotEq, Value::Bool(b)) => Ok(Value::Bool(a != b)),
            
            (Value::None, BinOp::Eq, Value::None) => Ok(Value::Bool(true)),
            (Value::None, BinOp::NotEq, Value::None) => Ok(Value::Bool(false)),
            (_, BinOp::Eq, Value::None) | (Value::None, BinOp::Eq, _) => Ok(Value::Bool(false)),
            (_, BinOp::NotEq, Value::None) | (Value::None, BinOp::NotEq, _) => Ok(Value::Bool(true)),
            
            // Logical
            (Value::Bool(a), BinOp::And, Value::Bool(b)) => Ok(Value::Bool(*a && *b)),
            (Value::Bool(a), BinOp::Or, Value::Bool(b)) => Ok(Value::Bool(*a || *b)),
            
            // In operator
            (item, BinOp::In, Value::List(list)) => Ok(Value::Bool(list.contains(item))),
            (Value::String(s), BinOp::In, Value::String(haystack)) => Ok(Value::Bool(haystack.contains(s))),
            (key, BinOp::In, Value::Dict(pairs)) => {
                Ok(Value::Bool(pairs.iter().any(|(k, _)| k == key)))
            }
            
            // Is operator (identity comparison)
            (Value::None, BinOp::Is, Value::None) => Ok(Value::Bool(true)),
            (a, BinOp::Is, b) => Ok(Value::Bool(a == b)),
            
            // Bitwise operators
            (Value::Int(a), BinOp::BitAnd, Value::Int(b)) => Ok(Value::Int(a & b)),
            (Value::Int(a), BinOp::BitOr, Value::Int(b)) => Ok(Value::Int(a | b)),
            (Value::Int(a), BinOp::BitXor, Value::Int(b)) => Ok(Value::Int(a ^ b)),
            (Value::Int(a), BinOp::LShift, Value::Int(b)) => Ok(Value::Int(a << b)),
            (Value::Int(a), BinOp::RShift, Value::Int(b)) => Ok(Value::Int(a >> b)),
            
            _ => Err(format!("Invalid operation: {:?} {:?} {:?}", left, op, right)),
        }
    }

    fn apply_unary_op(&self, op: &UnaryOp, val: &Value) -> Result<Value, String> {
        match (op, val) {
            (UnaryOp::Neg, Value::Int(n)) => Ok(Value::Int(-n)),
            (UnaryOp::Neg, Value::Float(f)) => Ok(Value::Float(-f)),
            (UnaryOp::Not, val) => Ok(Value::Bool(!self.is_truthy(val))),
            (UnaryOp::BitNot, Value::Int(n)) => Ok(Value::Int(!n)),
            _ => Err(format!("Invalid unary operation: {:?} {:?}", op, val)),
        }
    }

    fn compare_values(&self, a: &Value, b: &Value) -> std::cmp::Ordering {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x.cmp(y),
            (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
            (Value::String(x), Value::String(y)) => x.cmp(y),
            _ => std::cmp::Ordering::Equal,
        }
    }

    fn is_truthy(&self, val: &Value) -> bool {
        match val {
            Value::None => false,
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::List(items) => !items.is_empty(),
            Value::Dict(pairs) => !pairs.is_empty(),
            _ => true,
        }
    }

    fn get_var(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(name) { return Some(val.clone()); }
        }
        self.globals.get(name).cloned()
    }

    fn set_var(&mut self, name: String, value: Value) {
        // If variable exists in globals (marked as global), update it there
        if self.globals.contains_key(&name) {
            self.globals.insert(name, value);
        } else if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self { Self::new() }
}

// ── DB value conversion helpers ─────────────────────────────────────────────
#[cfg(feature = "native")]
fn poly_value_to_db_value(v: &Value) -> crate::db::DbValue {
    match v {
        Value::None => crate::db::DbValue::Null,
        Value::Int(i) => crate::db::DbValue::Int(*i),
        Value::Float(f) => crate::db::DbValue::Float(*f),
        Value::String(s) => crate::db::DbValue::Text(s.clone()),
        Value::Bool(b) => crate::db::DbValue::Int(if *b { 1 } else { 0 }),
        _ => crate::db::DbValue::Null,
    }
}

#[cfg(feature = "native")]
fn db_value_to_poly_value(v: crate::db::DbValue) -> Value {
    match v {
        crate::db::DbValue::Null => Value::None,
        crate::db::DbValue::Int(i) => Value::Int(i),
        crate::db::DbValue::Float(f) => Value::Float(f),
        crate::db::DbValue::Text(s) => Value::String(s),
        crate::db::DbValue::Blob(b) => Value::String(format!("<blob {} bytes>", b.len())),
    }
}

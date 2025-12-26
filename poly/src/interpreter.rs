use std::collections::HashMap;
use crate::ast::*;

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
        };
        interp.register_builtins();
        interp
    }

    fn register_builtins(&mut self) {
        let builtins = [
            "print", "len", "range", "str", "int", "float", "type",
            "input", "append", "abs", "min", "max", "sum", "sorted",
            "reversed", "enumerate", "zip", "map", "filter", "any", "all",
            "isinstance", "hasattr", "getattr", "setattr", "list", "dict",
            "set", "tuple", "bool", "chr", "ord", "hex", "bin", "oct",
            "round", "pow", "divmod", "slice", "iter", "next", "open",
            // String methods
            "upper", "lower", "strip", "split", "join", "replace", "startswith", "endswith",
            "find", "count", "isdigit", "isalpha", "isalnum", "format",
            // HTML generation
            "html", "html_escape", "html_tag",
            // Web framework - Routing, Components, State
            "router", "route", "component", "store", "live_reload",
            // List methods  
            "push", "pop", "insert", "remove", "index", "clear", "copy", "extend",
            // File I/O
            "read_file", "write_file", "file_exists",
            // HTTP (low-level primitives - user implements their own logic)
            "http_get", "http_post", "http_post_json",
            // HTTP Streaming (for SSE/chunked responses)
            "http_stream_start", "http_stream_poll", "http_stream_close",
            // JSON
            "json_parse", "json_stringify",
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
            Statement::Let(name, expr) => {
                let value = self.evaluate(expr)?;
                self.set_var(name.clone(), value);
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
            Statement::FnDef { name, params, body } => {
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
            Value::List(_) | Value::String(_) | Value::Dict(_) => {
                Ok(Value::NativeFunction(format!("{}.{}", 
                    match &target_val { Value::List(_) => "list", Value::String(_) => "str", _ => "dict" }, 
                    attr)))
            }
            _ => Err(self.error(format!("No attribute '{}' on {:?}", attr, target_val))),
        }
    }

    fn call_function(&mut self, func: Value, args: Vec<Value>) -> Result<Value, String> {
        match func {
            Value::Function { params, body, name } => {
                self.scopes.push(HashMap::new());
                
                // Bind parameters with default value support
                let required_count = params.iter().filter(|p| p.default.is_none()).count();
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
                    .and_then(|c| c.methods.get("__init__"))
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

    fn call_native(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
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
            _ => Err(self.error(format!("Unknown native function: {}", name))),
        }
    }
    
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
            
            _ => Err(format!("Invalid operation: {:?} {:?} {:?}", left, op, right)),
        }
    }

    fn apply_unary_op(&self, op: &UnaryOp, val: &Value) -> Result<Value, String> {
        match (op, val) {
            (UnaryOp::Neg, Value::Int(n)) => Ok(Value::Int(-n)),
            (UnaryOp::Neg, Value::Float(f)) => Ok(Value::Float(-f)),
            (UnaryOp::Not, val) => Ok(Value::Bool(!self.is_truthy(val))),
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
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self { Self::new() }
}

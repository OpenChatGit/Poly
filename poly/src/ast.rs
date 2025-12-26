/// Abstract Syntax Tree for Poly

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<Value>),
    Dict(Vec<(Value, Value)>),
    Function {
        name: String,
        params: Vec<Param>,
        body: Vec<Statement>,
    },
    NativeFunction(String),
    Instance {
        class_name: String,
        fields: std::collections::HashMap<String, Value>,
    },
    Class {
        name: String,
        parent: Option<String>,
        methods: Vec<Method>,
    },
    // UI Widget value
    Widget(WidgetNode),
}

/// UI Widget node for declarative UI
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetNode {
    pub widget_type: String,
    pub props: Vec<(String, Value)>,
    pub children: Vec<WidgetNode>,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::None => write!(f, "none"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Dict(pairs) => {
                write!(f, "{{")?;
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Function { name, .. } => write!(f, "<fn {}>", name),
            Value::NativeFunction(name) => write!(f, "<native fn {}>", name),
            Value::Instance { class_name, .. } => write!(f, "<{} instance>", class_name),
            Value::Class { name, .. } => write!(f, "<class {}>", name),
            Value::Widget(node) => write!(f, "<Widget {}>", node.widget_type),
        }
    }
}

impl Value {
    /// Convert Value to valid JSON string
    pub fn to_json(&self) -> String {
        match self {
            Value::None => "null".to_string(),
            Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(fl) => {
                if fl.is_nan() { "null".to_string() }
                else if fl.is_infinite() { "null".to_string() }
                else { fl.to_string() }
            }
            Value::String(s) => {
                // Escape special characters for JSON
                let escaped = s
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t");
                format!("\"{}\"", escaped)
            }
            Value::List(items) => {
                let inner: Vec<String> = items.iter().map(|v| v.to_json()).collect();
                format!("[{}]", inner.join(","))
            }
            Value::Dict(pairs) => {
                let inner: Vec<String> = pairs.iter().map(|(k, v)| {
                    // Keys must be strings in JSON
                    let key_str = match k {
                        Value::String(s) => {
                            let escaped = s
                                .replace('\\', "\\\\")
                                .replace('"', "\\\"");
                            format!("\"{}\"", escaped)
                        }
                        _ => format!("\"{}\"", k),
                    };
                    format!("{}:{}", key_str, v.to_json())
                }).collect();
                format!("{{{}}}", inner.join(","))
            }
            Value::Function { name, .. } => format!("\"<fn {}>\"", name),
            Value::NativeFunction(name) => format!("\"<native fn {}>\"", name),
            Value::Instance { class_name, fields } => {
                let inner: Vec<String> = fields.iter().map(|(k, v)| {
                    format!("\"{}\":{}", k, v.to_json())
                }).collect();
                format!("{{\"__class__\":\"{}\",{}}}", class_name, inner.join(","))
            }
            Value::Class { name, .. } => format!("\"<class {}>\"", name),
            Value::Widget(node) => format!("\"<Widget {}>\"", node.widget_type),
        }
    }
}

/// Function parameter with optional default value
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Method {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Statement>,
}

/// Part of an f-string
#[derive(Debug, Clone, PartialEq)]
pub enum FStringPart {
    Literal(String),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // Literals
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    
    // F-String (interpolated): f"Hello {name}!"
    FString(Vec<FStringPart>),
    
    // Collections
    List(Vec<Expr>),
    Dict(Vec<(Expr, Expr)>),
    
    // List comprehension: [expr for var in iter if condition]
    ListComp {
        expr: Box<Expr>,
        var: String,
        iter: Box<Expr>,
        condition: Option<Box<Expr>>,
    },
    
    // Variables & Access
    Identifier(String),
    Index(Box<Expr>, Box<Expr>),        // list[0], dict["key"]
    Attribute(Box<Expr>, String),        // obj.attr
    
    // Operations
    BinaryOp(Box<Expr>, BinOp, Box<Expr>),
    UnaryOp(UnaryOp, Box<Expr>),
    
    // Ternary: value if condition else other
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    
    // Function call
    Call(Box<Expr>, Vec<Expr>),
    
    // Function call with keyword args: func(a, b, key=value)
    CallWithKwargs(Box<Expr>, Vec<Expr>, Vec<(String, Expr)>),
    
    // Lambda (anonymous function)
    Lambda(Vec<Param>, Box<Expr>),
    
    // UI Widget expression: Widget(props...): children...
    Widget {
        widget_type: String,
        props: Vec<(String, Expr)>,
        children: Vec<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, FloorDiv, Mod, Pow,
    Eq, NotEq, Lt, Gt, LtEq, GtEq,
    And, Or,
    In,  // 'in' operator for membership
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    // Variable declaration/assignment
    Let(String, Expr),
    Assign(String, Expr),
    IndexAssign(Expr, Expr, Expr),  // list[i] = value
    AttrAssign(Expr, String, Expr), // obj.attr = value
    
    // Control flow
    If {
        condition: Expr,
        then_body: Vec<Statement>,
        elif_branches: Vec<(Expr, Vec<Statement>)>,
        else_body: Option<Vec<Statement>>,
    },
    While {
        condition: Expr,
        body: Vec<Statement>,
    },
    For {
        var: String,
        iter: Expr,
        body: Vec<Statement>,
    },
    
    // Functions
    FnDef {
        name: String,
        params: Vec<Param>,
        body: Vec<Statement>,
    },
    Return(Option<Expr>),
    
    // Classes
    ClassDef {
        name: String,
        parent: Option<String>,
        methods: Vec<Method>,
    },
    
    // Expression statement
    Expr(Expr),
    
    // Import
    Import(String),
    FromImport(String, Vec<String>),
    
    // Control
    Pass,
    Break,
    Continue,
    
    // Exception handling
    Try {
        try_body: Vec<Statement>,
        exception_type: Option<String>,
        except_body: Vec<Statement>,
    },
    Raise(Expr),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

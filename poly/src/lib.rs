pub mod ast;
pub mod lexer;
pub mod parser;
pub mod interpreter;
pub mod web;
pub mod native;
pub mod updater;
pub mod tray;
pub mod ai;
pub mod clipboard;
pub mod window;
pub mod notification;

use lexer::Lexer;
use parser::Parser;
use interpreter::Interpreter;

pub use native::{NativeConfig, run_native_window, run_native_url, generate_native_bundle};
pub use updater::{UpdateConfig, UpdateInfo, check_github_updates, check_custom_updates, download_update, install_update};
pub use tray::{TrayConfig, TrayMenuItem, TrayEvent, TrayHandle, create_tray};
pub use ai::{AiProvider, ChatMessage, ChatRequest, ChatResponse, StreamEvent, MessageRole, check_ollama, list_ollama_models, chat as ai_chat};

/// Run Poly source code and return the output
pub fn run(source: &str) -> Result<Vec<String>, String> {
    let lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;
    
    let mut interpreter = Interpreter::new();
    interpreter.run(&program)?;
    
    Ok(interpreter.get_output().to_vec())
}

/// Run Poly source code and return the last value
pub fn eval(source: &str) -> Result<String, String> {
    let lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;
    
    let mut interpreter = Interpreter::new();
    let result = interpreter.run(&program)?;
    
    Ok(format!("{}", result))
}

/// Run Poly source code and return the last value as valid JSON
pub fn eval_json(source: &str) -> Result<String, String> {
    let lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;
    
    let mut interpreter = Interpreter::new();
    let result = interpreter.run(&program)?;
    
    Ok(result.to_json())
}

/// Create a new interpreter instance
pub fn create_interpreter() -> Interpreter {
    Interpreter::new()
}

/// Initialize interpreter with source code (parse and run definitions)
pub fn init_interpreter(interpreter: &mut Interpreter, source: &str) -> Result<(), String> {
    let lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;
    
    interpreter.run(&program)?;
    Ok(())
}

/// Call a function on an existing interpreter and return JSON result
pub fn call_function(interpreter: &mut Interpreter, fn_name: &str, args_json: &str) -> Result<String, String> {
    // Parse the call expression
    let call_source = format!("{}({})", fn_name, args_json);
    
    let lexer = Lexer::new(&call_source);
    let tokens = lexer.tokenize();
    
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;
    
    let result = interpreter.run(&program)?;
    Ok(result.to_json())
}

// WASM bindings for web interop
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn poly_run(source: &str) -> Result<String, JsValue> {
    match run(source) {
        Ok(output) => Ok(output.join("\n")),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn poly_eval(source: &str) -> Result<String, JsValue> {
    match eval(source) {
        Ok(result) => Ok(result),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        let output = run(r#"print("Hello, Poly!")"#).unwrap();
        assert_eq!(output, vec!["Hello, Poly!"]);
    }

    #[test]
    fn test_arithmetic() {
        let result = eval("2 + 3 * 4").unwrap();
        assert_eq!(result, "14");
    }

    #[test]
    fn test_variables() {
        let output = run(r#"
let x = 10
let y = 20
print(x + y)
"#).unwrap();
        assert_eq!(output, vec!["30"]);
    }

    #[test]
    fn test_function() {
        let output = run(r#"fn greet(name):
    return "Hello, " + name

let msg = greet("World")
print(msg)"#).unwrap();
        assert_eq!(output, vec!["Hello, World"]);
    }

    #[test]
    fn test_list() {
        let output = run(r#"
let nums = [1, 2, 3]
print(len(nums))
print(nums[0])
"#).unwrap();
        assert_eq!(output, vec!["3", "1"]);
    }

    #[test]
    fn test_conditional() {
        let output = run(r#"
let x = 10
if x > 5:
    print("big")
"#).unwrap();
        assert_eq!(output, vec!["big"]);
    }
}

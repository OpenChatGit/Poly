//! Web/WASM support for Poly UI

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use web_sys::{window, HtmlCanvasElement};

/// Initialize Poly UI for web
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn web_main() {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();
    
    web_sys::console::log_1(&"Poly UI initialized for web".into());
}

/// Create a canvas element for rendering
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn create_canvas(container_id: &str, width: u32, height: u32) -> Result<HtmlCanvasElement, JsValue> {
    let window = window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;
    
    let canvas = document.create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()?;
    
    canvas.set_width(width);
    canvas.set_height(height);
    canvas.set_id("poly-ui-canvas");
    
    if let Some(container) = document.get_element_by_id(container_id) {
        container.append_child(&canvas)?;
    }
    
    Ok(canvas)
}

/// Run Poly code and return output
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_poly(source: &str) -> Result<String, JsValue> {
    match ::poly::run(source) {
        Ok(output) => Ok(output.join("\n")),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}

/// Evaluate Poly expression
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn eval_poly(source: &str) -> Result<String, JsValue> {
    match ::poly::eval(source) {
        Ok(result) => Ok(result),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}

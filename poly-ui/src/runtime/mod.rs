//! Poly language runtime integration

/// Execute Poly code and get UI widgets
pub fn eval_poly_ui(source: &str) -> Result<String, String> {
    poly::eval(source)
}

/// Run Poly code
pub fn run_poly(source: &str) -> Result<Vec<String>, String> {
    poly::run(source)
}

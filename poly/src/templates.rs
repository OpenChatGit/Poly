/// Generate the starter page used by new Poly applications.
pub fn empty_app_html(name: &str) -> String {
    let name = escape_html(name);
    include_str!("../templates/empty_app.html")
        .replace("{{APP_NAME}}", &name)
        .replace("../assets/PolyNew.png", "./poly-logo.png")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Default Poly icon PNG embedded in the Poly binary.
pub const DEFAULT_ICON_PNG: &[u8] = include_bytes!("../assets/PolyNew.png");

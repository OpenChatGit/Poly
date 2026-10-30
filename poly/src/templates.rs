/// Generate the empty web shell used by new Poly applications.
pub fn empty_app_html(name: &str) -> String {
    let name = escape_html(name);
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{name}</title>
  <style>
    :root {{ color-scheme: dark; font-family: Inter, ui-sans-serif, system-ui, sans-serif; color: #edf4f7; background: #0d141a; }}
    * {{ box-sizing: border-box; }}
    body {{ margin: 0; min-width: 320px; min-height: 100vh; display: grid; place-items: center; background: radial-gradient(ellipse at top, #183344, #0d141a 65%); }}
    main {{ width: min(720px, calc(100vw - 48px)); padding: 48px; border: 1px solid #294252; border-radius: 24px; background: #111d25dd; box-shadow: 0 24px 90px #0008; text-align: center; }}
    .brand-logo {{ display: block; width: 160px; height: 160px; object-fit: contain; margin: 0 auto 28px; }}
    .eyebrow {{ color: #73d6e7; letter-spacing: .16em; font-size: 12px; font-weight: 700; }}
    h1 {{ font-size: clamp(36px, 6vw, 64px); line-height: 1.02; letter-spacing: -.05em; margin: 16px 0; }}
    p {{ color: #a4bac5; font-size: 17px; }}
  </style>
</head>
<body>
  <main>
    <img class="brand-logo" src="./poly-logo.png" alt="Poly logo">
    <p class="eyebrow">YOUR APP STARTS HERE</p>
    <h1>{name}</h1>
    <p>Built with Poly and web technologies.</p>
  </main>
</body>
</html>
"#,
        name = name
    )
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

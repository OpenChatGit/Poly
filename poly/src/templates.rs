// Welcome Screen Templates for new Poly projects
// Separated from main.rs to keep code organized

/// Generate the welcome screen HTML for a new project
pub fn welcome_html(name: &str) -> String {
    format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{name}</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <div class="app">
    {logo_svg}
    <h1>Welcome to {name}</h1>
  </div>
  <script src="app.js"></script>
</body>
</html>"##, name = name, logo_svg = POLY_LOGO_SVG)
}

/// Generate the welcome screen CSS
pub fn welcome_css() -> &'static str {
    r#"* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: #0a0a0a;
  color: #fff;
  min-height: 100vh;
}

.app {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  gap: 2rem;
}

.logo {
  width: 120px;
  height: auto;
}

h1 {
  font-size: 1.5rem;
  font-weight: 500;
  color: #e0e0e0;
}
"#
}

/// Generate the welcome screen JavaScript
pub fn welcome_js() -> &'static str {
    r#"// Your JavaScript goes here
"#
}

/// Generate the main.poly backend template
pub fn main_poly(name: &str) -> String {
    format!(r#"# {} - Backend Logic (optional)
# This file is for server-side logic, APIs, etc.
# Your frontend is in web/index.html, web/styles.css, web/app.js

print("Backend ready")
"#, name)
}

/// Poly Logo SVG
pub const POLY_LOGO_SVG: &str = r##"<svg viewBox="0 0 366.77 474.9" class="logo">
  <defs>
    <linearGradient id="a" x1="0" y1="237.45" x2="366.77" y2="237.45" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#1e6e95"/><stop offset="1" stop-color="#1971a2"/></linearGradient>
    <linearGradient id="b" x1="301.44" y1="213.46" x2="314.35" y2="168.43" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#1e76a4"/><stop offset="1" stop-color="#2689af"/></linearGradient>
    <linearGradient id="c" x1="280.28" y1="261.32" x2="366.77" y2="261.32" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#2696bb"/><stop offset="1" stop-color="#2689af"/></linearGradient>
    <linearGradient id="d" x1="319.27" y1="165" x2="324.98" y2="1.56" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#62c2d0"/><stop offset="1" stop-color="#6ec5ce"/></linearGradient>
    <linearGradient id="e" x1="269.17" y1="179.74" x2="281.73" y2=".1" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#1cb3c9"/><stop offset="1" stop-color="#52beca"/></linearGradient>
    <linearGradient id="f" x1="239.82" y1="144.42" x2="229.97" y2="3.52" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#288ca6"/><stop offset="1" stop-color="#3ebac5"/></linearGradient>
    <linearGradient id="g" x1="181.37" y1="140.88" x2="40.64" y2="-32.91" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#16a3bf"/><stop offset="1" stop-color="#1f9eb8"/></linearGradient>
    <linearGradient id="h" x1="-14.29" y1="13.32" x2="103.18" y2="139.29" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#5dc1d2"/><stop offset="1" stop-color="#19abc6"/></linearGradient>
    <linearGradient id="i" x1="33.68" y1="193.98" x2="13.15" y2="-1.38" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#2892b1"/><stop offset="1" stop-color="#19abc6"/></linearGradient>
    <linearGradient id="j" x1="118.66" y1="315.53" x2="94.56" y2="119.26" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#2786b9"/><stop offset="1" stop-color="#19abc6"/></linearGradient>
    <linearGradient id="k" x1="95.67" y1="412.92" x2="81.75" y2="147.37" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#174e99"/><stop offset="1" stop-color="#1e80ad"/></linearGradient>
    <linearGradient id="l" x1="43.67" y1="347.32" x2="29.91" y2="150.67" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#17428b"/><stop offset="1" stop-color="#176b9f"/></linearGradient>
    <linearGradient id="m" x1="96.99" y1="443.24" x2="27.66" y2="371.45" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#26326e"/><stop offset="1" stop-color="#1c3e7d"/></linearGradient>
    <linearGradient id="n" x1="31.03" y1="473.74" x2="35.55" y2="344.11" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#272f65"/><stop offset="1" stop-color="#24346b"/></linearGradient>
    <linearGradient id="o" x1="28.77" y1="408.67" x2="13.94" y2="196.55" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#273169"/><stop offset="1" stop-color="#114b8a"/></linearGradient>
    <linearGradient id="p" x1="129.22" y1="230.68" x2="212.86" y2="230.68" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#2180b6"/><stop offset="1" stop-color="#268db8"/></linearGradient>
    <linearGradient id="q" x1="186.81" y1="250.64" x2="280.28" y2="250.64" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#278cb9"/><stop offset="1" stop-color="#268ab7"/></linearGradient>
    <linearGradient id="r" x1="105.04" y1="293.82" x2="158.16" y2="230.51" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#213567"/><stop offset="1" stop-color="#1d5f95"/></linearGradient>
    <linearGradient id="s" x1="327.06" y1="209.69" x2="383.23" y2="94.53" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#3abacb"/><stop offset="1" stop-color="#54bfce"/></linearGradient>
    <linearGradient id="t" x1="190.32" y1="343.95" x2="224.87" y2="273.11" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#124b94"/><stop offset="1" stop-color="#1e619d"/></linearGradient>
    <linearGradient id="u" x1="0" y1="30.13" x2="280.28" y2="30.13" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#5dc1d2"/><stop offset="1" stop-color="#6cc5d1"/></linearGradient>
  </defs>
  <path fill="url(#a)" d="M328.43,266.8c12.78-12.58,25.56-25.16,38.33-37.74V86.49C337.94,57.66,309.11,28.83,280.28,0H0v410.68c21.4,21.4,42.81,42.81,64.21,64.21,21.71-21.25,43.43-42.5,65.14-63.74-.04-98.72-.09-197.43-.13-296.15,27.9.11,55.79.22,83.69.33l28.94,28.94v33.55c-3.08,3.08-6.16,6.16-9.24,9.23-6.59,6.58-13.17,13.17-19.76,19.75h-83.64c.04,35.8.09,71.6.13,107.4h150.93c16.05-15.8,32.1-31.61,48.15-47.41Z"/>
  <polygon fill="url(#b)" points="300.04 164.32 241.85 177.83 283.88 208.42 366.77 229.06 300.04 164.32"/>
  <polygon fill="url(#c)" points="280.28 314.21 283.88 208.42 366.77 229.06 280.28 314.21"/>
  <polygon fill="url(#d)" points="280.28 0 300.04 164.32 366.77 86.49 280.28 0"/>
  <polygon fill="url(#e)" points="241.85 144.28 280.28 0 300.04 164.32 241.85 177.83 241.85 144.28"/>
  <polygon fill="url(#f)" points="212.91 115.34 183.38 60.26 280.28 0 241.85 144.28 212.91 115.34"/>
  <polygon fill="url(#g)" points="212.91 115.34 129.22 115 0 0 183.38 60.26 212.91 115.34"/>
  <polygon fill="url(#h)" points="129.22 115 62.94 148.36 0 0 129.22 115"/>
  <polygon fill="url(#i)" points="62.94 148.36 0 197.52 0 0 62.94 148.36"/>
  <polygon fill="url(#j)" points="129.22 115 62.94 148.36 129.35 314.21 129.22 115"/>
  <polygon fill="url(#k)" points="62.94 148.36 53.34 346.65 129.35 411.15 129.35 314.21 62.94 148.36"/>
  <polygon fill="url(#l)" points="0 197.52 53.34 346.65 62.94 148.36 0 197.52"/>
  <polygon fill="url(#m)" points="64.21 474.9 53.34 346.65 129.35 411.15 64.21 474.9"/>
  <polygon fill="url(#n)" points="53.34 346.65 0 410.68 64.21 474.9 53.34 346.65"/>
  <polygon fill="url(#o)" points="0 410.68 53.34 346.65 0 197.52 0 410.68"/>
  <polygon fill="url(#p)" points="129.22 206.81 186.81 254.55 212.86 206.81 129.22 206.81"/>
  <polygon fill="url(#q)" points="280.28 314.21 186.81 254.55 212.86 206.81 232.62 187.06 280.28 314.21"/>
  <polygon fill="url(#r)" points="129.22 206.81 129.35 314.21 186.81 254.55 129.22 206.81"/>
  <polygon fill="url(#s)" points="300.04 164.32 366.77 229.06 366.77 86.49 300.04 164.32"/>
  <polygon fill="url(#t)" points="129.35 314.21 186.81 254.55 280.28 314.21 129.35 314.21"/>
  <polygon fill="url(#u)" points="280.28 0 183.38 60.26 0 0 280.28 0"/>
</svg>"##;

/// Default Poly icon PNG (embedded as bytes)
/// This is the Polybarsmall@2x.png icon
pub const DEFAULT_ICON_PNG: &[u8] = include_bytes!("../assets/Polybarsmall@2x.png");

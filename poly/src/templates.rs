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
  <div class="app" x-data="app()">
    <div class="container">
      <!-- Logo -->
      <div class="logo-wrapper">
        {logo_svg}
      </div>

      <!-- Welcome -->
      <h1>Welcome to <span class="gradient">{name}</span></h1>
      <p class="subtitle">Your Poly app is ready. Start building something amazing.</p>

      <!-- Features -->
      <div class="features">
        <div class="feature-card">
          <div class="feature-icon"><i data-lucide="zap"></i></div>
          <h3>Hot Reload</h3>
          <p>Changes appear instantly</p>
        </div>
        <div class="feature-card">
          <div class="feature-icon"><i data-lucide="cpu"></i></div>
          <h3>IPC Bridge</h3>
          <p>Call Poly from JavaScript</p>
        </div>
        <div class="feature-card">
          <div class="feature-icon"><i data-lucide="package"></i></div>
          <h3>Native Build</h3>
          <p>Desktop apps with WebView</p>
        </div>
      </div>

      <!-- Demo -->
      <div class="demo-section">
        <h2>Try it out</h2>
        <div class="demo-card">
          <div class="demo-row">
            <button class="btn btn-primary" @click="count++">
              <i data-lucide="plus"></i>
              Count: <span x-text="count"></span>
            </button>
            <button class="btn btn-secondary" @click="count = 0">Reset</button>
          </div>
          <div class="demo-row">
            <input type="text" x-model="name" placeholder="Enter your name..." class="input">
          </div>
          <p class="greeting" x-show="name.length > 0" x-transition>
            Hello, <span x-text="name"></span>! 👋
          </p>
        </div>
      </div>

      <!-- Quick Start -->
      <div class="quickstart">
        <h2>Quick Start</h2>
        <div class="code-block">
          <code>web/index.html</code> - Your app's HTML<br>
          <code>web/styles.css</code> - Your styles<br>
          <code>web/app.js</code> - Your JavaScript<br>
          <code>src/main.poly</code> - Backend logic (optional)
        </div>
      </div>

      <footer>Built with <span class="heart">♥</span> using Poly</footer>
    </div>
  </div>
  <script src="app.js"></script>
</body>
</html>"##, name = name, logo_svg = POLY_LOGO_SVG)
}

/// Generate the welcome screen CSS
pub fn welcome_css() -> &'static str {
    r#"* { margin: 0; padding: 0; box-sizing: border-box; }

:root {
  --bg: #0a0a0f;
  --surface: rgba(255, 255, 255, 0.03);
  --border: rgba(255, 255, 255, 0.08);
  --text: #fafafa;
  --text-muted: #888;
  --text-dim: #555;
  --accent: #5dc1d2;
  --accent-dark: #1e80ad;
}

body {
  font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  background: var(--bg);
  color: var(--text);
  min-height: 100vh;
  line-height: 1.6;
}

.app {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
}

.container { max-width: 720px; text-align: center; }

/* Logo */
.logo-wrapper { margin-bottom: 2rem; }

.logo {
  width: 80px;
  height: auto;
  filter: drop-shadow(0 20px 40px rgba(93, 193, 210, 0.2));
  animation: float 4s ease-in-out infinite;
}

@keyframes float {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-8px); }
}

/* Typography */
h1 { font-size: 2rem; font-weight: 600; margin-bottom: 0.5rem; }

.gradient {
  background: linear-gradient(135deg, var(--accent) 0%, var(--accent-dark) 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.subtitle { color: var(--text-muted); font-size: 1rem; margin-bottom: 3rem; }

h2 {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-dim);
  margin-bottom: 1rem;
}

/* Feature Cards */
.features {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1rem;
  margin-bottom: 3rem;
}

.feature-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 1.5rem 1rem;
  transition: all 0.2s;
}

.feature-card:hover {
  border-color: rgba(93, 193, 210, 0.3);
  transform: translateY(-2px);
}

.feature-icon {
  width: 40px;
  height: 40px;
  background: linear-gradient(135deg, var(--accent) 0%, var(--accent-dark) 100%);
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 0.75rem;
}

.feature-icon i, .feature-icon svg { width: 20px; height: 20px; color: white; }

.feature-card h3 { font-size: 0.9rem; font-weight: 600; margin-bottom: 0.25rem; }
.feature-card p { font-size: 0.8rem; color: var(--text-muted); }

/* Demo Section */
.demo-section { margin-bottom: 3rem; }

.demo-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 1.5rem;
}

.demo-row {
  display: flex;
  gap: 0.75rem;
  justify-content: center;
  margin-bottom: 1rem;
}

.demo-row:last-child { margin-bottom: 0; }

/* Buttons */
.btn {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.6rem 1.2rem;
  border: none;
  border-radius: 8px;
  font-size: 0.85rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn i, .btn svg { width: 16px; height: 16px; }

.btn-primary {
  background: linear-gradient(135deg, var(--accent) 0%, var(--accent-dark) 100%);
  color: white;
}

.btn-primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(93, 193, 210, 0.3);
}

.btn-secondary {
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text-muted);
}

.btn-secondary:hover {
  background: var(--surface);
  border-color: rgba(255, 255, 255, 0.2);
  color: var(--text);
}

/* Input */
.input {
  flex: 1;
  max-width: 250px;
  padding: 0.6rem 1rem;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text);
  font-size: 0.85rem;
  outline: none;
  transition: all 0.2s;
}

.input:focus {
  border-color: rgba(93, 193, 210, 0.5);
  background: rgba(255, 255, 255, 0.08);
}

.input::placeholder { color: var(--text-dim); }

.greeting { color: var(--accent); font-size: 1rem; margin-top: 0.5rem; }

/* Quick Start */
.quickstart { margin-bottom: 3rem; }

.code-block {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 1.25rem;
  text-align: left;
  font-size: 0.85rem;
  line-height: 1.8;
  color: var(--text-muted);
}

.code-block code {
  background: rgba(93, 193, 210, 0.15);
  color: var(--accent);
  padding: 0.15rem 0.4rem;
  border-radius: 4px;
  font-family: 'SF Mono', Monaco, 'Cascadia Code', monospace;
  font-size: 0.8rem;
}

/* Footer */
footer { color: var(--text-dim); font-size: 0.8rem; }
.heart { color: #ef4444; }

/* Responsive */
@media (max-width: 600px) {
  .features { grid-template-columns: 1fr; }
  h1 { font-size: 1.5rem; }
}
"#
}

/// Generate the welcome screen JavaScript
pub fn welcome_js() -> &'static str {
    r#"// App State
function app() {
  return {
    count: 0,
    name: '',
    
    // Example: Call Poly backend
    async callBackend() {
      try {
        const result = await poly.invoke('greet', { name: this.name || 'World' });
        console.log('Backend says:', result);
      } catch (e) {
        console.log('Backend not available');
      }
    }
  };
}

console.log('App loaded - Edit web/ files to customize');
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

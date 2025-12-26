# Poly

Build native desktop apps with web technologies. Like Electron/Tauri, but simpler and batteries-included.

## Features

- 🚀 **Hot Reload** - Changes appear instantly
- 🎨 **Alpine.js** - Reactive UI out of the box
- 🎯 **Lucide Icons** - 1000+ icons included
- 🌉 **IPC Bridge** - Call backend from JavaScript
- 📦 **Native Build** - Desktop apps with WebView

## Quick Start

```bash
# Create a new project
cargo run -p poly -- new my-app

# Start dev server
cd my-app
cargo run -p poly -- dev .

# Open http://localhost:3000
```

## Project Structure

```
my-app/
├── web/                 # Your frontend (edit directly!)
│   ├── index.html       # Main HTML
│   ├── styles.css       # Styles
│   └── app.js           # JavaScript + Alpine.js
├── src/
│   └── main.poly        # Backend logic (optional)
└── poly.toml            # Config
```

## Commands

```bash
poly new <name>          # Create new project
poly dev .               # Dev server (localhost:3000)
poly run . --native      # Native desktop window
poly build --target web  # Build for deployment
```

## Included Libraries

Poly automatically injects these in every project:

### Alpine.js
Lightweight reactive framework. Use `x-data`, `x-text`, `@click` etc:

```html
<div x-data="{ count: 0 }">
  <button @click="count++">Clicked <span x-text="count"></span></button>
</div>
```

### Lucide Icons
1000+ icons via `<i data-lucide="icon-name">`:

```html
<i data-lucide="home"></i>
<i data-lucide="settings"></i>
<i data-lucide="user"></i>
```

Browse icons: https://lucide.dev/icons

## IPC Bridge

Call Poly backend functions from JavaScript:

**src/main.poly:**
```python
fn get_users():
    return [
        {"id": 1, "name": "Alice"},
        {"id": 2, "name": "Bob"}
    ]

fn add_user(name):
    # Add to database...
    return {"success": true}
```

**web/app.js:**
```javascript
// Call backend function
const users = await poly.invoke('get_users');
console.log(users); // [{id: 1, name: "Alice"}, ...]

// With arguments
const result = await poly.invoke('add_user', { name: 'Charlie' });
```

## Full Example

**web/index.html:**
```html
<!DOCTYPE html>
<html>
<head>
  <title>My App</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <div x-data="app()">
    <h1><i data-lucide="rocket"></i> My App</h1>
    
    <button class="btn" @click="count++">
      Count: <span x-text="count"></span>
    </button>
    
    <input x-model="name" placeholder="Your name">
    <p x-show="name">Hello, <span x-text="name"></span>!</p>
  </div>
  <script src="app.js"></script>
</body>
</html>
```

**web/app.js:**
```javascript
function app() {
  return {
    count: 0,
    name: '',
    
    async loadData() {
      const data = await poly.invoke('get_data');
      console.log(data);
    }
  };
}
```

## Native Desktop App

```bash
# Build with native feature
cargo build -p poly --features native

# Run as desktop app
cargo run -p poly --features native -- run . --native
```

## Requirements

- Rust 1.70+
- For native: WebView2 (Windows) / WebKit (Linux/macOS)

## Why Poly?

| Feature | Electron | Tauri | Poly |
|---------|----------|-------|------|
| Bundle Size | ~150MB | ~3MB | ~3MB |
| Hot Reload | ❌ | ✅ | ✅ |
| Alpine.js | Manual | Manual | ✅ Built-in |
| Icons | Manual | Manual | ✅ Built-in |
| IPC | Complex | Complex | Simple |
| Setup | npm + node | Rust + npm | Rust only |

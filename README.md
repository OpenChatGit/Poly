# Poly

**Build native desktop apps with web technologies — lightweight, fast, and easy to use.**

Poly is a modern framework for building cross-platform desktop applications using HTML, CSS, and JavaScript. It provides native OS integration out-of-the-box with zero configuration.

> **Platform Support:** Currently focused on Windows. Linux and macOS support are planned — Poly is built with Rust and cross-platform libraries (wry/tao), so it will work on all platforms once tested.

## Features

- 🪶 **Lightweight** — ~7MB binary, fast startup
- 🌐 **Browser Mode** — Build browsers with custom UI and WebView API
- 🪟 **Multi-Window** — Create and manage multiple windows
- 📋 **Clipboard** — Read/write system clipboard
- 🎨 **Frameless Windows** — Optional frameless mode with window control API
- 🔒 **Single Instance** — Prevent multiple app instances
- 🖥️ **Native Builds** — Build standalone .exe without console window
- 📁 **Native Dialogs** — File open/save, folder picker
- 🔔 **Notifications** — Native OS notifications
- 🔗 **Deep Links** — Custom URL protocol handling (myapp://)
- 🔄 **Auto-Updater** — Built-in GitHub Releases support
- ✍️ **Code Signing** — Sign executables for Windows/macOS
- 📂 **File System** — Read/write files from JavaScript
- 🤖 **AI Integration** — Ollama, OpenAI, Anthropic built-in
- 📦 **Package Manager** — UV-style fast npm package downloads
- ⚡ **Hot Reload** — Instant updates during development

## Quick Start

```bash
# Install
cargo install --git https://github.com/OpenChatGit/Poly.git poly --features native

# Create project
poly new my-app
cd my-app

# Development (hot reload)
poly dev

# Run as native window
poly run --native

# Build standalone executable
poly build --release
# Output: dist/windows/my-app.exe (no console window!)
```

## CLI

| Command | Description |
|---------|-------------|
| `poly new <name>` | Create new project |
| `poly dev` | Dev server with hot reload |
| `poly run --native` | Run as native app |
| `poly build --release` | Production build (no console) |
| `poly build --target windows` | Build for Windows |
| `poly build --target macos` | Build for macOS |
| `poly build --target linux` | Build for Linux |
| `poly build --release --sign` | Build and sign executable |
| `poly build --installer` | Create installer |
| `poly build --ci` | Generate GitHub Actions |
| `poly add <package>` | Add npm package |
| `poly remove <package>` | Remove package |
| `poly install` | Install from lockfile |
| `poly browser <url>` | Browser mode with custom UI |

## Project Structure

```
my-app/
├── poly.toml        # Config
├── poly.lock        # Lockfile
├── packages/        # npm packages
├── web/
│   ├── index.html   # Your app
│   ├── app.js
│   └── styles.css
└── assets/
    └── icon.png     # App icon
```

## JavaScript API Overview

```javascript
// Dialogs
await poly.dialog.open();
await poly.dialog.save();
await poly.dialog.message('Title', 'Message');
await poly.dialog.confirm('Sure?', 'Delete file?');

// File System
await poly.fs.read('file.txt');
await poly.fs.write('file.txt', 'content');
await poly.fs.exists('file.txt');

// Clipboard
await poly.clipboard.read();
await poly.clipboard.write('text');

// Multi-Window
const win = await poly.windows.create({ title: 'New', width: 600, height: 400, html: '...' });
await poly.windows.close(win.id);
await poly.windows.list();

// Window Control (for custom titlebar in frameless mode)
polyWindow.minimize();
polyWindow.maximize();
polyWindow.close();
polyWindow.drag();
const isFrameless = await polyWindow.isFrameless();

// Notifications
await poly.notification.show('Title', 'Message body');
await poly.notification.showWithTimeout('Alert', 'Auto-dismiss', 5000);

// Deep Links (Custom URL Protocol)
await poly.deeplink.register('myapp', 'My App');  // Register myapp://
await poly.deeplink.unregister('myapp');          // Remove from registry
const link = await poly.deeplink.get();           // Get launch URL

// System Tray (configure in poly.toml)
poly.tray.onMenuClick((id) => console.log('Menu clicked:', id));

// Auto-Updater
await poly.updater.checkGithub('user/repo', '1.0.0');

// AI/LLM
await poly.ai.ollama('llama3', messages);
await poly.ai.openai('gpt-4', messages, apiKey);

// WebView API (for browser-like apps)
await poly.webview.create('content', { url: 'https://example.com', x: 0, y: 80, width: 1200, height: 720 });
await poly.webview.navigate('content', 'https://google.com');
await poly.webview.goBack('content');
await poly.webview.goForward('content');
```

📖 **Full API Documentation:** [docs/API.md](docs/API.md)

## Browser Mode

Build browser-like applications with a custom UI:

```bash
# Run browser with custom UI
poly browser https://google.com --ui-html browser-ui.html

# Custom window size
poly browser https://example.com --width 1400 --height 900 --ui-html ui.html
```

Your UI HTML communicates via IPC:
```javascript
// Navigate content area
window.ipc.postMessage('navigate:https://google.com');

// Window controls
window.ipc.postMessage('minimize');
window.ipc.postMessage('maximize');
window.ipc.postMessage('close');

// Receive events from content
window.onNavStart = (url) => { /* navigation started */ };
window.onLoadEnd = (url) => { /* page loaded */ };
window.onTitleChange = (title) => { /* title changed */ };
```

See [docs/API.md#browser-mode](docs/API.md#browser-mode) for complete documentation.

## Package Manager

```bash
poly add alpinejs        # Add package
poly add lodash -v 4.17  # Specific version
poly remove lodash       # Remove
poly install             # Install from lockfile
```

- UV-style parallel downloads (~4s for 65 packages)
- Full dependency resolution
- SHA256 integrity hashes in `poly.lock`

## Configuration

```toml
# poly.toml
[package]
name = "my-app"
version = "1.0.0"

[window]
width = 1024
height = 768
decorations = true  # false for frameless window (requires custom titlebar)
single_instance = true  # prevent multiple instances

[dev]
port = 3000  # custom port for dev server (0 = auto)

[tray]
enabled = true
close_to_tray = true

[dependencies]
alpinejs = "3.14.3"
```

## Comparison

| Feature | Poly | Electron | Tauri |
|---------|------|----------|-------|
| Binary Size | ~7MB | ~150MB | ~5MB |
| Memory | Low | High | Low |
| Package Manager | Built-in | npm | npm |
| Multi-Window | Built-in | Built-in | Built-in |
| Browser Mode | Built-in | Manual | Manual |
| Clipboard | Built-in | Built-in | Plugin |
| Notifications | Built-in | Built-in | Plugin |
| Deep Links | Built-in | Built-in | Plugin |
| System Tray | Built-in | Plugin | Plugin |
| Code Signing | Built-in | External | External |
| AI/LLM | Built-in | - | - |
| Setup | Simple | Complex | Medium |

## License

MIT

---

[Documentation](docs/API.md) · [GitHub](https://github.com/OpenChatGit/Poly)

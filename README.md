# 🔷 Poly

**Build native desktop apps with web technologies — simpler than Electron, faster than Tauri.**

Poly is a modern framework for building cross-platform desktop applications using HTML, CSS, and JavaScript. It provides native OS integration out-of-the-box with zero configuration.

## ✨ Features

- 🚀 **Lightweight** — Small binary size, fast startup
- 🎨 **Custom Titlebar** — Beautiful frameless windows by default
- 📁 **Native Dialogs** — File open/save, folder picker, message boxes
- 🔄 **Auto-Updater** — Built-in update system with GitHub Releases support
- 📂 **File System API** — Read/write files from JavaScript
- 🔥 **Hot Reload** — Instant updates during development
- 🌐 **Web Technologies** — Use HTML, CSS, JS, Alpine.js, any framework

## 🚀 Quick Start

```bash
# Install Poly
cargo install --git https://github.com/OpenChatGit/Poly.git poly --features native

# Create a new project
poly new my-app
cd my-app

# Start development server
poly dev

# Run as native app
poly run --native
```

## 📦 Installation

### From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/OpenChatGit/Poly.git
cd Poly

# Install globally with native features
cargo install --path poly --features native
```

### Requirements

- Rust 1.70+
- Windows 10+, macOS 10.15+, or Linux

## 🛠️ CLI Commands

| Command | Description |
|---------|-------------|
| `poly new <name>` | Create a new project |
| `poly dev` | Start dev server with hot reload |
| `poly run --native` | Run as native desktop app |
| `poly build` | Build for production |
| `poly update` | Check for updates |
| `poly --version` | Show version & check for updates |

## 📁 Project Structure

```
my-app/
├── poly.toml           # Project configuration
├── src/
│   └── main.poly       # Backend logic (optional)
├── web/
│   ├── index.html      # Main HTML file
│   ├── app.js          # JavaScript
│   └── styles.css      # Styles
└── assets/
    ├── icon.png        # App icon (taskbar/dock)
    └── icon.svg        # Titlebar icon (optional)
```

## 🎨 Custom Titlebar

Poly automatically provides a beautiful custom titlebar — no configuration needed!

- **Windows**: Minimize, Maximize, Close buttons on the right
- **macOS**: Traffic light buttons on the left
- **Custom Icon**: Add `assets/icon.svg` for your own titlebar icon

## 📡 JavaScript API

### Dialogs

```javascript
// Native file dialogs
const file = await poly.dialog.open({ 
  title: 'Select File',
  filters: [['Images', ['png', 'jpg']]]
});

const savePath = await poly.dialog.save({ defaultName: 'document.txt' });
const folder = await poly.dialog.folder();

// Custom in-app dialogs (styleable)
await poly.dialog.message('Success', 'File saved!', 'info');
const confirmed = await poly.dialog.confirm('Delete?', 'Are you sure?');

// Fully custom dialog
const result = await poly.dialog.custom({
  type: 'warning',
  title: 'Unsaved Changes',
  message: 'Do you want to save before closing?',
  buttons: [
    { text: 'Discard', value: 'discard' },
    { text: 'Cancel', value: 'cancel' },
    { text: 'Save', value: 'save', primary: true }
  ]
});
```

### File System

```javascript
// Read file
const content = await poly.fs.read('/path/to/file.txt');

// Write file
await poly.fs.write('/path/to/file.txt', 'Hello World');

// Check if exists
const exists = await poly.fs.exists('/path/to/file.txt');

// List directory
const files = await poly.fs.readDir('/path/to/folder');
```

### Auto-Updater

```javascript
// Check for updates
const info = await poly.updater.checkGithub('user/repo', '1.0.0');

if (info.update_available) {
  // Download and install
  const path = await poly.updater.download(info.download_url);
  await poly.updater.install(path);
}

// Or use the convenience method
await poly.updater.checkAndPrompt({
  repo: 'user/my-app',
  currentVersion: '1.0.0'
});
```

### IPC (Call Backend Functions)

```javascript
// Call a function defined in main.poly
const result = await poly.invoke('myFunction', { arg1: 'value' });
```

## ⚙️ Configuration

### poly.toml

```toml
[package]
name = "my-app"
version = "1.0.0"

[web]
dir = "web"

[window]
width = 1024
height = 768
resizable = true

[update]
enabled = true
source = "github"
github_repo = "user/my-app"
```

## 🔄 Auto-Updates

Poly has built-in auto-update support:

1. **CLI Updates**: Run `poly --version` or `poly update` to check for Poly updates
2. **App Updates**: Use `poly.updater` API in your app

### Setting up GitHub Releases

1. Create a release on GitHub
2. Upload your app binaries with platform-specific names:
   - `my-app-windows-x64.exe`
   - `my-app-macos-x64.dmg`
   - `my-app-linux-x64.AppImage`
3. Poly will automatically find the right download for each platform

## 🏗️ Architecture

```
Poly/
├── poly/                   # Core framework
│   ├── src/
│   │   ├── main.rs         # CLI & dev server
│   │   ├── lib.rs          # Library exports
│   │   ├── native.rs       # Native window (wry/tao)
│   │   ├── updater.rs      # Auto-updater
│   │   ├── lexer.rs        # Poly language lexer
│   │   ├── parser.rs       # Poly language parser
│   │   └── interpreter.rs  # Poly language runtime
│   └── Cargo.toml
├── poly-ui/                # UI framework (optional)
└── poly-vscode/            # VS Code extension
```

## 🆚 Comparison

| Feature | Poly | Electron | Tauri |
|---------|------|----------|-------|
| Binary Size | ~10MB | ~150MB | ~5MB |
| Memory Usage | Low | High | Low |
| Custom Titlebar | ✅ Default | Manual | Manual |
| Auto-Updater | ✅ Built-in | Plugin | Plugin |
| Native Dialogs | ✅ Built-in | Plugin | Built-in |
| Hot Reload | ✅ Built-in | Plugin | Built-in |
| Setup Complexity | Simple | Complex | Medium |

## 📄 License

MIT

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

---

Made with ❤️ by the Poly Team

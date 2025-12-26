# Poly

**Build native desktop apps with web technologies — simpler than Electron, faster than Tauri.**

Poly is a modern framework for building cross-platform desktop applications using HTML, CSS, and JavaScript. It provides native OS integration out-of-the-box with zero configuration.

## Features

- **Lightweight** — Small binary size (~10MB), fast startup
- **Custom Titlebar** — Beautiful frameless windows by default
- **System Tray** — Run apps in background with customizable tray menu
- **Native Dialogs** — File open/save, folder picker, message boxes
- **Auto-Updater** — Built-in update system with GitHub Releases support
- **File System API** — Read/write files from JavaScript
- **Hot Reload** — Instant updates during development
- **Web Technologies** — Use HTML, CSS, JS, Alpine.js, or any framework

## Quick Start

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

## Installation

### From Source

```bash
git clone https://github.com/OpenChatGit/Poly.git
cd Poly
cargo install --path poly --features native
```

### Requirements

- Rust 1.70+
- Windows 10+, macOS 10.15+, or Linux

## CLI Commands

| Command | Description |
|---------|-------------|
| `poly new <name>` | Create a new project |
| `poly dev` | Start dev server with hot reload |
| `poly run --native` | Run as native desktop app |
| `poly build` | Build for production |
| `poly update` | Check for updates |
| `poly --version` | Show version and check for updates |

## Project Structure

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

## Configuration

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

[tray]
enabled = true
tooltip = "My App"
minimize_to_tray = false
close_to_tray = true

# Custom tray menu
[[tray.menu]]
id = "show"
label = "Show Window"

[[tray.menu]]
id = "separator"

[[tray.menu]]
id = "quit"
label = "Exit"

[update]
enabled = true
source = "github"
github_repo = "user/my-app"
```

## JavaScript API

### Dialogs

```javascript
// Native file dialogs
const file = await poly.dialog.open({ 
  title: 'Select File',
  filters: [['Images', ['png', 'jpg']]]
});

const savePath = await poly.dialog.save({ defaultName: 'document.txt' });
const folder = await poly.dialog.folder();

// Custom in-app dialogs
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
const content = await poly.fs.read('/path/to/file.txt');
await poly.fs.write('/path/to/file.txt', 'Hello World');
const exists = await poly.fs.exists('/path/to/file.txt');
const files = await poly.fs.readDir('/path/to/folder');
```

### Window Control

```javascript
poly.window.minimize();  // Minimize (or hide to tray if configured)
poly.window.maximize();  // Toggle maximize
poly.window.close();     // Close (or hide to tray if configured)
poly.window.hide();      // Hide to tray
poly.window.show();      // Show and focus window
```

### Auto-Updater

```javascript
// Check for updates
const info = await poly.updater.checkGithub('user/repo', '1.0.0');

if (info.update_available) {
  const path = await poly.updater.download(info.download_url);
  await poly.updater.install(path);
}

// Or use the convenience method
await poly.updater.checkAndPrompt({
  repo: 'user/my-app',
  currentVersion: '1.0.0'
});
```

### IPC (Backend Functions)

```javascript
// Call a function defined in main.poly
const result = await poly.invoke('myFunction', { arg1: 'value' });
```

## System Tray

Enable system tray to run your app in the background:

```toml
[tray]
enabled = true
tooltip = "My App"
close_to_tray = true

[[tray.menu]]
id = "show"
label = "Show Window"

[[tray.menu]]
id = "separator"

[[tray.menu]]
id = "settings"
label = "Settings"

[[tray.menu]]
id = "quit"
label = "Exit"
```

Special menu IDs:
- `show` — Shows and focuses the window
- `quit` or `exit` — Exits the application
- `separator` — Adds a separator line
- Any other ID — Custom action

## Custom Titlebar

Poly automatically provides a custom titlebar with no configuration needed:

- **Windows**: Minimize, Maximize, Close buttons on the right
- **macOS**: Traffic light buttons on the left
- **Custom Icon**: Add `assets/icon.svg` for your own titlebar icon

## Auto-Updates

Poly has built-in auto-update support:

1. Run `poly --version` or `poly update` to check for Poly updates
2. Use `poly.updater` API in your app for app updates

### GitHub Releases Setup

1. Create a release on GitHub
2. Upload binaries with platform names:
   - `my-app-windows-x64.exe`
   - `my-app-macos-x64.dmg`
   - `my-app-linux-x64.AppImage`
3. Poly automatically finds the right download for each platform

## Comparison

| Feature | Poly | Electron | Tauri |
|---------|------|----------|-------|
| Binary Size | ~10MB | ~150MB | ~5MB |
| Memory Usage | Low | High | Low |
| Custom Titlebar | Built-in | Manual | Manual |
| System Tray | Built-in | Plugin | Plugin |
| Auto-Updater | Built-in | Plugin | Plugin |
| Native Dialogs | Built-in | Plugin | Built-in |
| Hot Reload | Built-in | Plugin | Built-in |
| Setup Complexity | Simple | Complex | Medium |

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

---

Made with care by the Poly Team

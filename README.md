# Poly

**Build native desktop apps with web technologies — lightweight, fast, and easy to use.**

Poly is a modern framework for building cross-platform desktop applications using HTML, CSS, and JavaScript. It provides native OS integration out-of-the-box with zero configuration.

> **Platform Support:** Currently focused on Windows development and testing. Linux and macOS support are planned — since Poly is built with Rust and cross-platform libraries (wry/tao), it will work on all platforms once the Windows version is mature and I have access to test environments for other operating systems.

## Features

- **Lightweight** — Small binary size (~10MB), fast startup
- **Custom Titlebar** — Beautiful frameless windows by default
- **System Tray** — Run apps in background with customizable tray menu
- **Native Dialogs** — File open/save, folder picker, message boxes
- **Auto-Updater** — Built-in update system with GitHub Releases support
- **File System API** — Read/write files from JavaScript
- **AI/LLM Integration** — Built-in support for Ollama, OpenAI, Anthropic
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
| `poly build --release` | Optimized release build |
| `poly build --installer` | Create installer/package |
| `poly build --ci` | Generate GitHub Actions workflow |
| `poly add <package>` | Add a JavaScript package (from npm) |
| `poly remove <package>` | Remove a package |
| `poly install` | Install packages from poly.lock |
| `poly update` | Check for updates |
| `poly --version` | Show version and check for updates |

## Package Management

Poly has a UV-style fast package manager that uses the npm registry directly. No npm or Node.js installation required!

```bash
# Add any npm package
poly add alpinejs
poly add express
poly add lodash

# Add specific version
poly add alpinejs -v 3.14.0

# Remove packages
poly remove express

# Install from lockfile (like npm ci)
poly install
```

### Features

- **UV-Style Speed** — Parallel downloads with thread pool (~4s for 65 packages)
- **Full Dependency Resolution** — Automatically resolves and installs all sub-dependencies
- **npm Registry** — Works with any package from npmjs.org
- **Lockfile** — `poly.lock` with SHA256 integrity hashes for reproducible builds
- **Zero Config** — No npm, yarn, or Node.js needed

### How It Works

1. Fetches package metadata from npm registry
2. Resolves full dependency tree recursively
3. Downloads all packages in parallel (UV-style)
4. Extracts to `packages/<name>/` folder
5. Creates `poly.lock` with integrity hashes
6. Updates `poly.toml` dependencies

### Using Packages in HTML

```html
<!-- In your web/index.html -->
<script src="/packages/alpinejs/dist/cdn.min.js" defer></script>
<script src="/packages/chart.js/dist/chart.umd.js"></script>
<script src="/packages/lodash/lodash.min.js"></script>
```

### poly.toml Dependencies

```toml
[package]
name = "my-app"
version = "1.0.0"

[dependencies]
alpinejs = "3.14.3"
express = "4.18.2"
lodash = "4.17.21"
```

### poly.lock

The lockfile ensures reproducible builds across machines:

```json
{
  "alpinejs": {
    "version": "3.14.3",
    "integrity": "sha256-...",
    "dependencies": []
  },
  "express": {
    "version": "4.18.2",
    "integrity": "sha256-...",
    "dependencies": ["body-parser", "cookie", ...]
  }
}
```

## Project Structure

```
my-app/
├── poly.toml           # Project configuration
├── poly.lock           # Lockfile with integrity hashes
├── packages/           # JavaScript dependencies (auto-managed)
│   ├── alpinejs/
│   ├── lodash/
│   └── ...             # + all sub-dependencies
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

## AI/LLM Integration

Poly has built-in support for AI/LLM APIs, making it easy to build chat applications:

### Ollama (Local)

```javascript
// Check if Ollama is running
const available = await poly.ai.checkOllama();

// List available models
const models = await poly.ai.listModels();

// Chat with Ollama
const response = await poly.ai.ollama('llama3', [
  { role: 'user', content: 'Hello!' }
]);

console.log(response.content);
console.log(response.thinking); // Reasoning content if available
```

### OpenAI

```javascript
const response = await poly.ai.openai('gpt-4', [
  { role: 'system', content: 'You are a helpful assistant.' },
  { role: 'user', content: 'Hello!' }
], 'sk-your-api-key');
```

### Anthropic (with Extended Thinking)

```javascript
const response = await poly.ai.anthropic('claude-3-5-sonnet-20241022', [
  { role: 'user', content: 'Solve this step by step...' }
], 'your-api-key', {
  enableThinking: true,
  thinkingBudget: 10000
});

console.log(response.thinking); // Claude's reasoning process
console.log(response.content);  // Final answer
```

### Custom OpenAI-Compatible APIs

```javascript
// LM Studio, LocalAI, etc.
const response = await poly.ai.custom('http://localhost:1234/v1', 'local-model', [
  { role: 'user', content: 'Hello!' }
]);
```

### Full Options

```javascript
const response = await poly.ai.chat({
  provider: 'ollama',        // 'ollama', 'openai', 'anthropic', 'custom'
  baseUrl: 'http://localhost:11434',
  model: 'llama3',
  messages: [
    { role: 'system', content: 'You are helpful.' },
    { role: 'user', content: 'Hello!' }
  ],
  temperature: 0.7,
  maxTokens: 4096,
  enableThinking: true,      // For reasoning models
  thinkingBudget: 10000      // Anthropic thinking budget
});
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

## Cross-Platform Builds

Poly makes it easy to build your app for Windows, macOS, and Linux.

### Build for Current Platform

```bash
# Development build
poly build

# Optimized release build
poly build --release

# Create installer/package (.zip on Windows, .app on macOS, .tar.gz on Linux)
poly build --release --installer
```

### Build for All Platforms (GitHub Actions)

Generate a CI workflow that builds for all platforms automatically:

```bash
poly build --ci
```

This creates `.github/workflows/build.yml` that:
- Builds for Windows, macOS, and Linux
- Creates release packages automatically
- Uploads to GitHub Releases when you push a tag

```bash
# After pushing to GitHub:
git tag v1.0.0
git push --tags
# GitHub Actions builds for all platforms!
```

### Build Output

```
dist/
├── windows/
│   ├── my-app.exe           # Windows executable
│   └── bundle/              # Bundled assets
├── macos/
│   └── my-app.app/          # macOS app bundle
└── linux/
    └── my-app               # Linux executable
```

## Comparison

| Feature | Poly | Electron | Tauri |
|---------|------|----------|-------|
| Binary Size | ~7MB | ~150MB | ~5MB |
| Memory Usage | Low | High | Low |
| Package Manager | UV-style fast | npm | npm/cargo |
| Cross-Platform Build | Built-in + CI | electron-builder | tauri-action |
| Custom Titlebar | Built-in | Manual | Manual |
| System Tray | Built-in | Plugin | Plugin |
| Auto-Updater | Built-in | Plugin | Plugin |
| Native Dialogs | Built-in | Plugin | Built-in |
| AI/LLM Integration | Built-in | None | None |
| Hot Reload | Built-in | Plugin | Built-in |
| Setup Complexity | Simple | Complex | Medium |

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

---

Made with care by the Poly Team

# Poly

![Version](https://img.shields.io/badge/version-0.4.1-blue)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey)
![Language](https://img.shields.io/badge/poly--lang-beta-orange)
![License](https://img.shields.io/badge/license-Custom-green)

Build native desktop apps with web technologies, using a powerful, typed, Python-like language.

Poly is a lightweight framework for creating cross-platform desktop applications using HTML, CSS, and JavaScript, now powered by a significantly enhanced Poly scripting language.

## New in 0.4.1: Frontend-Backend Communication

Call Poly functions directly from JavaScript with seamless type conversion!

```javascript
// Frontend (JavaScript)
const greeting = await poly.call('greet', 'Alice');
const sum = await poly.call('add_numbers', 42, 58);
```

```python
# Backend (Poly - src/main.poly)
fn greet(name: String) -> String:
    return "Hello, " + name + "!"

fn add_numbers(a: Int, b: Int) -> Int:
    return a + b
```

Perfect for building full-stack apps where you want the performance and type safety of Poly on the backend!

## 🛠️ Tooling & VSCode Setup

Poly 0.4.1 comes with a built-in VSCode extension installer. It supports **VSCode**, **Antigravity**, **Cursor**, and **VSCodium**.

To install syntax highlighting and language support:

```bash
poly install-vscode
```

Then simply restart your editor.

## Features

- Lightweight (~15MB binary)
- **Typed Scripting Language** (Python-like syntax)
- Multi-window support
- Native dialogs, clipboard, notifications
- System tray integration
- Deep links (custom URL protocols)
- Browser mode for building browsers
- Built-in package manager
- **Hot reload development**
- Auto-updater
- AI/LLM integration

## Quick Start

```bash
# Install Poly 0.4.0
cargo install --path poly --features native

# Create a new project
poly new my-app
cd my-app
poly dev
```

## Roadmap

- Linux support
- macOS support
- Plugin system
- More native APIs
- Further language optimizations

## Documentation

See [docs/API.md](docs/API.md) for the full API reference.

## License

[Poly License](LICENSE)

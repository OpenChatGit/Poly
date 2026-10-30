# Poly

![Version](https://img.shields.io/badge/version-0.4.8-blue)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey)
![Experimental](https://img.shields.io/badge/language%20and%20virtual%20drive-experimental-orange)
![License](https://img.shields.io/badge/license-Custom-green)

Build native desktop apps with web technologies.

Poly is a framework for creating desktop applications using HTML, CSS, and JavaScript.

Poly's custom scripting language and NCOS Virtual Drive are experimental and excluded from normal builds. To include them, build with `--features "native experimental-language experimental-virtual-drive"`.

> **Experimental:** `poly-ui` is an incomplete native UI prototype and is not part of Poly's default build. Its layout and renderer are still under development.

## Experimental: Poly Language Frontend-Backend Communication

This feature requires a build with `experimental-language` enabled.

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

### Streaming Support 🌊

Poly now supports true streaming from backend to frontend! Perfect for LLM token generation.

```javascript
// Frontend
await poly.stream('generate_text', 'Hello', (chunk) => {
    console.log("Received chunk:", chunk);
});
```

```python
// Backend
fn generate_text(prompt: String):
    stream("Processing...")
    sleep(500)
    stream("Done!")
```

## 🛠️ Tooling & VSCode Setup

Poly 0.4.1 comes with a built-in VSCode extension installer. It supports **VSCode**, **Antigravity**, **Cursor**, and **VSCodium**.

To install syntax highlighting and language support:

```bash
poly install-vscode
```

Then simply restart your editor.

## Features

- Lightweight (~15MB binary)
- React, Svelte, Vue, and Preact project templates
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
# Install Poly 0.4.8
cargo install --path poly --features native

# Create a new project
poly new my-app
cd my-app
poly dev
```

### Start with a web framework

Create a Vite project with React, Svelte, Vue, or Preact already configured. Poly runs `npm install` for you; add `--no-install` to create the files without downloading packages.

```bash
poly init --react
# or: poly init --svelte / --vue / --preact

poly dev          # Vite HMR plus the Poly backend
poly build        # build frontend and package the desktop app
poly run --native # launch the built app
```

You can also create a new folder directly with `poly new my-app --react`. These templates use Vite and require Node.js with npm installed.

## Roadmap

- Linux support
- macOS support
- Plugin system
- More native APIs
- Stabilize the experimental Poly language and NCOS Virtual Drive

## Documentation

See [docs/API.md](docs/API.md) for the full API reference.

## License

[Poly License](LICENSE)

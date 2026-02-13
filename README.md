# Poly

![Version](https://img.shields.io/badge/version-0.4.0-blue)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey)
![Language](https://img.shields.io/badge/poly--lang-beta-orange)
![License](https://img.shields.io/badge/license-Custom-green)

Build native desktop apps with web technologies, using a powerful, typed, Python-like language.

Poly is a lightweight framework for creating cross-platform desktop applications using HTML, CSS, and JavaScript, now powered by a significantly enhanced Poly scripting language.

## 🚀 New in 0.4.0: Mojo-Inspired Syntax

Poly 0.4.0 introduces major syntax improvements inspired by Mojo to bring safety and performance:

- **Strict Typing**: Optional type annotations for variables and functions.
- **Structs**: Define data structures with methods and efficient memory layout.
- **Ownership/Borrowing**: `inout`, `borrowed`, and `owned` keywords for memory management control.
- **VSCode Integration**: Auto-installing syntax highlighting extension.

### Example

```python
struct Point:
    fn init(inout self, x: Int, y: Int):
        self.x = x
        self.y = y
    
    fn distance(self) -> Float:
        return (self.x ** 2 + self.y ** 2) ** 0.5

fn main():
    var p: Point = Point(3, 4)
    let d: Float = p.distance()
    print(f"Distance: {d}")
```

## 🛠️ Tooling & VSCode Setup

Poly 0.4.0 comes with a built-in VSCode extension installer. It supports **VSCode**, **Antigravity**, **Cursor**, and **VSCodium**.

To install syntax highlighting and language support:

```bash
poly install-vscode
```

Then simply restart your editor.

## Features

- Lightweight (~13MB binary)
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

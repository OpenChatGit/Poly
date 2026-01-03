# Poly

![Version](https://img.shields.io/badge/version-0.3.2-blue)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey)
![Language](https://img.shields.io/badge/poly--lang-beta-orange)
![License](https://img.shields.io/badge/license-Custom-green)

Build native desktop apps with web technologies.

Poly is a lightweight framework for creating cross-platform desktop applications using HTML, CSS, and JavaScript. Native OS integration with zero configuration.

> **Note:** The Poly scripting language (.poly files) is currently in beta. The framework and JavaScript APIs are stable.

## Features

- Lightweight (~13MB binary)
- Multi-window support
- Native dialogs, clipboard, notifications
- System tray integration
- Deep links (custom URL protocols)
- Browser mode for building browsers
- Built-in package manager
- Hot reload development
- Auto-updater
- AI/LLM integration

## Quick Start

```bash
cargo install --git https://github.com/OpenChatGit/Poly.git poly --features native

poly new my-app
cd my-app
poly dev
```

## Roadmap

- Linux support
- macOS support
- Plugin system
- More native APIs

## Documentation

See [docs/API.md](docs/API.md) for the full API reference.

## License

[Poly License](LICENSE)

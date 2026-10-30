# Poly

![Version](https://img.shields.io/badge/version-0.4.9-blue)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey)
![Status](https://img.shields.io/badge/status-experimental-orange)
![License](https://img.shields.io/badge/license-custom-informational)

**Poly is an experimental, custom-built Rust framework and CLI for creating, developing, building, and running native desktop applications with HTML, CSS, and JavaScript frontends. It is far from production-ready: features are incomplete, APIs may change, and platform support is limited.** Review the current limitations before using it for anything important.

Poly combines a Rust application runtime with web frontends. The CLI can create Vite projects for React, Svelte, Vue, and Preact, run a development server, build an app, and launch it through the native runtime. Windows is the currently supported native platform.

## Get started

Install the CLI from this repository:

```sh
cargo install --git https://github.com/OpenChatGit/Poly poly-framework --features native
```

Create a starter project with a frontend framework:

```sh
poly init --react
# Also available: --svelte, --vue, --preact
cd <project-directory>
poly dev
```

The templates use Vite and require Node.js and npm. `poly init` creates a project without selecting a frontend framework. Add `--no-install` to skip installing npm dependencies during project creation.

Useful project commands:

```sh
poly dev          # Run the frontend and Poly development workflow
poly build        # Build the frontend and desktop application
poly run --native # Launch the native desktop app
```

## Current status and limitations

- Poly is experimental and not ready to be treated as a stable or production-proven framework.
- Native desktop support currently targets Windows. Linux and macOS support are future work.
- The custom Poly language and NCOS Virtual Drive are experimental features. They are excluded from default builds and are not recommended for application use.
- `poly-ui` is an incomplete prototype and is not part of the standard Poly build.
- Optional CEF support has additional runtime and packaging requirements. The default native backend uses Wry.
- APIs, project templates, and package metadata may change between releases.

The custom language and virtual drive can be enabled when building from source with `--features "native experimental-language experimental-virtual-drive"`. These features are experimental and may be incomplete.

## Features

- Vite starter projects for React, Svelte, Vue, and Preact
- Native desktop windows backed by Wry
- Development and build commands in the CLI
- Native dialogs, clipboard, notifications, and system tray integrations
- Multi-window support and custom URL protocols
- Optional CEF renderer backend
- Experimental language frontend and backend calls
- Experimental NCOS Virtual Drive

Feature availability depends on platform, build flags, and the maturity of each subsystem.

## Documentation

- [API reference](docs/API.md)
- [CEF backend notes](poly/docs/CEF.md)
- [Experimental language API notes](poly/docs/API.md)
- [Examples](poly/examples)
- [Crate package information](poly/README.md)

Documentation may describe experimental or incomplete interfaces. Check each section's status notes before relying on it.

## Build from source

```sh
git clone https://github.com/OpenChatGit/Poly.git
cd Poly
cargo build --release -p poly-framework --features native
```

The resulting CLI executable is named `poly`.

## License

Poly is distributed under the [Poly License](LICENSE). It is a custom license with restrictions on commercializing Poly itself. Applications built with Poly are explicitly allowed to be commercialized. Read the full license before using or redistributing the project.

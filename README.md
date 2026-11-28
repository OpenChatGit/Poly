# Poly

![Version](https://img.shields.io/badge/version-0.5.16-blue)
![Platforms](https://img.shields.io/badge/platforms-Windows%20x64%20%7C%20Linux%20x64%20%7C%20macOS-lightgrey)
![Status](https://img.shields.io/badge/status-experimental-orange)
![License](https://img.shields.io/badge/license-custom-informational)

**Poly is an experimental, custom-built Rust framework and CLI for creating, developing, building, and running native desktop applications with HTML, CSS, and JavaScript frontends. It is far from production-ready: features are incomplete, APIs may change, and platform support is limited.** Review the current limitations before using it for anything important.

Poly combines a Rust application runtime with web frontends. The CLI can create Vite projects for React, Svelte, Vue, and Preact, run a development server, build an app, and launch it through the native runtime. The default native runtime builds on Windows x64, Linux x64, Intel macOS, and Apple Silicon macOS; OS-specific APIs and dependencies vary by platform, and some features remain Windows-only or experimental.

## Get started

Install the native CLI from this repository:

```sh
cargo install --git https://github.com/OpenChatGit/Poly poly-framework
```

Starting with v0.5.16, GitHub Releases include prebuilt CLI downloads for Windows x64 (`.zip`), Linux x64 (`.tar.gz`), and macOS Intel and Apple Silicon (`.tar.gz` each). Extract the archive and add the executable to your `PATH`. Linux requires the system GTK/WebKitGTK runtime libraries; native app windows on Windows require the WebView2 Runtime. The macOS CLI is not signed or notarized, so Gatekeeper may warn when it is first run. The source `.crate` is also attached for users who prefer Cargo installation.

Create a starter project with a frontend framework:

```sh
poly init --react
# Also available: --svelte, --vue, --preact
poly reinit --svelte # Refresh generated starter files in an existing project
cd <project-directory>
poly dev
```

The templates use Vite and require Node.js and npm. `poly init` creates a project without selecting a frontend framework. Use `poly reinit [--react|--svelte|--vue|--preact]` to refresh generated starter files in an initialized project; custom files and installed `node_modules` are preserved. Add `--no-install` to skip installing npm dependencies during project creation.

Pull requests and pushes to `master` automatically run the cross-platform build and checks. When a new Poly package version is pushed to `master`, CI builds the CLI binaries and creates its GitHub Release after all platform builds and tests pass. A matching `v*` tag can also trigger a release; existing versions are skipped to avoid duplicates. Bump `poly/Cargo.toml` before the next release.

Useful project commands:

```sh
poly dev          # Run the browser frontend and Poly development workflow
poly dev --native # Open a desktop window with live frontend updates
poly build        # Build the frontend and desktop application
poly run --native # Launch the native desktop app
poly ai-info      # Print agent-oriented setup, API, and platform guidance
```

Add JavaScript libraries with `poly add <package>`. Poly records them in `package.json`, uses npm's lockfile, and configures Vite imports for plain HTML projects. Import modules normally from `web/src/main.js`. Use `poly dev --native` to make desktop changes live without rebuilding; `poly build` remains for packaging.

Project files in the top-level `assets/` folder can be imported from frontend modules with Vite (for example, `import logoUrl from '../../assets/logo.svg'`). Plain URLs such as `/assets/logo.svg` also work in Poly's native window in development and packaged builds.

## Current status and limitations

- Poly is experimental and not ready to be treated as a stable or production-proven framework.
- The default Wry native runtime builds for Windows x64, Linux x64, and macOS x64/ARM64. CI builds and tests each target. Feature coverage and system dependencies vary; the NCOS Virtual Drive and native app embedding remain Windows-only.
- The NCOS Virtual Drive is experimental and excluded from default builds.
- `poly-ui` is an incomplete prototype and is not part of the standard Poly build.
- Optional CEF support has additional runtime and packaging requirements. The default native backend uses Wry.
- APIs, project templates, and package metadata may change between releases.

The NCOS Virtual Drive can be enabled when building from source with `--features "native experimental-virtual-drive"`. This feature is experimental and may be incomplete.

## Features

- Vite starter projects for React, Svelte, Vue, and Preact
- Native desktop windows backed by Wry
- Development and build commands in the CLI
- Native dialogs, clipboard, notifications, and system tray integrations
- Multi-window support and custom URL protocols
- Optional CEF renderer backend
- Experimental NCOS Virtual Drive
- Rust sidecar plugins, including streamed native output

Feature availability depends on platform, build flags, and the maturity of each subsystem.

## Documentation

The [docs.rs reference](https://docs.rs/poly-framework/latest/poly/) combines the Rust API with the complete JavaScript API and developer guide, plus CEF setup notes.

- [API reference](https://github.com/OpenChatGit/Poly/blob/master/docs/API.md)
- [CEF backend notes](https://github.com/OpenChatGit/Poly/blob/master/poly/docs/CEF.md)
- [Examples](https://github.com/OpenChatGit/Poly/tree/master/poly/examples)
- [Crate package information](https://github.com/OpenChatGit/Poly/blob/master/poly/README.md)

Documentation may describe experimental or incomplete interfaces. Check each section's status notes before relying on it.

## Build from source

```sh
git clone https://github.com/OpenChatGit/Poly.git
cd Poly
cargo build --release -p poly-framework
```

The resulting CLI executable is named `poly` (`poly.exe` on Windows). For prebuilt downloads, use the platform archives attached to the [GitHub Releases](https://github.com/OpenChatGit/Poly/releases).

## License

Poly is distributed under the [Poly License](https://github.com/OpenChatGit/Poly/blob/master/LICENSE). It is a custom license with restrictions on commercializing Poly itself. Applications built with Poly are explicitly allowed to be commercialized. Read the full license before using or redistributing the project.

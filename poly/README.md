# Poly Framework

Poly is an experimental, custom-built Rust framework and command-line tool for creating, developing, building, and running native desktop applications with HTML, CSS, and JavaScript frontends. It is far from production-ready. Features are incomplete, APIs may change, and native support currently targets Windows.

The crate is named `poly-framework`; installing it provides the `poly` command:

```sh
cargo install poly-framework --features native
poly --help
```

The crate's default feature set is empty. Enable `native` for the standard native runtime. The custom language and NCOS Virtual Drive are experimental and require `experimental-language` and `experimental-virtual-drive` respectively. Optional CEF renderer support is enabled with `cef` and may require extra runtime and packaging setup.

Create a Vite starter project with a frontend framework:

```sh
poly init --react
# Also available: --svelte, --vue, --preact
```

Templates require Node.js and npm. Use `poly dev`, `poly build`, and `poly run --native` inside the generated project.

The [docs.rs reference](https://docs.rs/poly-framework/latest/poly/) combines the Rust API with the complete JavaScript API and developer guide, plus the experimental language and CEF guides. Read the [project README](https://github.com/OpenChatGit/Poly#readme) for setup and current limitations. Some interfaces are experimental.

Poly is distributed under the custom [Poly License](https://github.com/OpenChatGit/Poly/blob/master/LICENSE). Review it before using or redistributing the crate.

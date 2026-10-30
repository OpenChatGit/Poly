# CEF renderer support

Poly exposes CEF as an opt-in native capability so projects can keep using Wry
without packaging Chromium Embedded Framework binaries unless they need them.

## Features and use

- `cef` enables Poly's native feature set and re-exports the CEF Rust bindings
  as `poly::cef_api`.
- `cef-build-util` enables the Windows `cef-bundle` command, which packages the
  matching Poly executable, CEF runtime, and project web assets.
- `cef-osr` additionally enables CEF's accelerated off-screen-rendering API for
  shared GPU textures. NCOS's Windows browser currently uses an embedded native
  CEF child window instead, while other platforms keep the canvas OSR fallback.

For example, a Poly application can opt in with:

```toml
poly = { path = "../../poly", features = ["cef-osr"] }
```

## Managed surface contract

Poly owns browser creation, navigation, resize, visibility, focus, and shutdown.
On Windows, NCOS creates CEF as a native child of Poly's app window. Chromium
renders directly in that child window, avoiding the per-frame software readback
and canvas transfer used by OSR. The child follows the app window and is bounded
to the browser content rectangle. Other platforms use the windowless canvas path.

Build the CEF-enabled Poly binary with `cargo build -p poly --features cef`. The
binary re-enters `poly` for CEF renderer subprocesses, so the CEF runtime files
must be shipped alongside Poly. In NCOS, `poly.cef.create(...)` creates a
managed browser. Windows input is handled by the native CEF child; the canvas
and RGBA frame endpoint remain the fallback for windowless platforms.

On Windows, build Poly with the packaging utility enabled, then bundle a project
from the Poly workspace root:

```powershell
cargo run -p poly --features cef-build-util -- cef-bundle apps/ncos
```

For a release bundle, use `cargo run --release` and pass `--release` to the
command after `--`. By default, output goes to `apps/ncos/dist/cef`; pass
`-o <directory>` to choose another destination. CEF runtime files are placed
beside `poly.exe`, with the project under `bundle/` so Poly's bundled-app
startup path can find it.

The Windows native child path avoids the OSR software-frame copy. A portable
shared-texture compositor for platforms that require windowless CEF remains a
separate optimization; enabling `cef-osr` alone does not connect those textures
to a host renderer.

CEF starts with software rendering because this keeps its embedded child window
responsive on systems where Chromium cannot create a GPU shared context. To
compare the GPU path, set `POLY_CEF_ENABLE_GPU=1` before launching Poly. That
switch is opt-in until the GPU path is verified on the target driver.

The current renderer sets CEF's `no_sandbox` flag. The bundle command includes
CEF runtime files, but it does not enable the Windows sandbox. Current CEF
versions require a bootstrap executable plus a client DLL entry point for
sandboxed Windows apps; Poly still runs as an executable. Treat this as a
development integration until Poly's process entry point is ported to that
bootstrap model. See the [CEF sandbox setup guide](https://github.com/chromiumembedded/cef/blob/master/docs/sandbox_setup.md).

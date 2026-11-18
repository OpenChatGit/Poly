//! Native app plugins implemented as Rust sidecar executables.
//!
//! Sidecars use a small JSON-over-stdio protocol so plugins do not share an
//! unstable Rust ABI with the Poly runtime.

use serde_json::{json, Value};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static BUILD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn failure(message: impl Into<String>) -> String {
    json!({ "error": message.into() }).to_string()
}

fn valid_segment(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
}

fn safe_child(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if path.is_absolute() || path.components().any(|part| !matches!(part, std::path::Component::Normal(_))) {
        return Err("Plugin paths must stay inside the plugin folder.".into());
    }
    Ok(root.join(path))
}

fn sidecar_path(web_root: &Path, plugin_id: &str) -> Result<(PathBuf, PathBuf, Value), String> {
    if !valid_segment(plugin_id) {
        return Err("Invalid plugin ID.".into());
    }
    let plugins_dir = web_root.join("plugins").canonicalize().map_err(|_| "Plugin directory was not found.")?;
    let plugin_dir = plugins_dir.join(plugin_id).canonicalize().map_err(|_| "Plugin folder was not found.")?;
    if !plugin_dir.starts_with(&plugins_dir) {
        return Err("Plugin folder is outside the configured plugins directory.".into());
    }
    let manifest_path = plugin_dir.join("manifest.json");
    let manifest: Value = serde_json::from_slice(&std::fs::read(&manifest_path).map_err(|_| "Plugin manifest could not be read.")?)
        .map_err(|_| "Plugin manifest is not valid JSON.")?;
    if manifest.get("enabled").and_then(Value::as_bool) != Some(true) {
        return Err("Plugin is disabled.".into());
    }
    let native = manifest.get("native").ok_or("Plugin does not declare a native implementation.")?;
    if native.get("runtime").and_then(Value::as_str) != Some("rust-sidecar") {
        return Err("Unsupported native plugin runtime.".into());
    }
    let executable = native.get("executable").and_then(Value::as_str).ok_or("Native plugin must declare an executable path.")?;
    let executable_path = safe_child(&plugin_dir, executable)?;
    Ok((plugin_dir, executable_path, manifest))
}

fn compile_sidecar(web_root: &Path, plugin_dir: &Path, manifest: &Value) -> Result<PathBuf, String> {
    let native = manifest.get("native").ok_or("Missing native plugin configuration.")?;
    let crate_manifest = native.get("crate").and_then(Value::as_str).ok_or("Native plugin must declare its Cargo manifest.")?;
    let binary_name = native.get("binary").and_then(Value::as_str).ok_or("Native plugin must declare its Cargo binary name.")?;
    if !valid_segment(binary_name) {
        return Err("Invalid native plugin binary name.".into());
    }
    let crate_path = safe_child(plugin_dir, crate_manifest)?.canonicalize().map_err(|_| "Rust plugin Cargo manifest was not found.")?;
    if !crate_path.starts_with(plugin_dir) {
        return Err("Rust plugin crate is outside the plugin folder.".into());
    }

    let target_dir = web_root.parent().unwrap_or(web_root).join(".poly-plugin-target").join(
        plugin_dir.file_name().and_then(|name| name.to_str()).unwrap_or("plugin")
    );
    std::fs::create_dir_all(&target_dir).map_err(|error| format!("Could not prepare Rust plugin build directory: {error}"))?;
    let lock = BUILD_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().map_err(|_| "Rust plugin build lock is unavailable.")?;

    let output = Command::new("cargo")
        .args(["build", "--manifest-path"])
        .arg(&crate_path)
        .arg("--target-dir")
        .arg(&target_dir)
        .current_dir(plugin_dir)
        .output()
        .map_err(|error| format!("Could not start Cargo for the Rust plugin: {error}"))?;
    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Rust plugin compilation failed: {}", details.trim()));
    }

    let executable_name = if cfg!(windows) { format!("{binary_name}.exe") } else { binary_name.to_string() };
    let built = target_dir.join("debug").join(executable_name);
    if !built.is_file() {
        return Err(format!("Cargo completed but did not produce the declared binary at {}.", built.display()));
    }
    Ok(built)
}

/// Invoke an enabled Rust sidecar plugin using `{ method, args }` JSON on stdin.
pub fn invoke(web_root: &Path, plugin_id: &str, method: &str, args: &Value) -> String {
    if !valid_segment(method) {
        return failure("Invalid native plugin method.");
    }
    let (plugin_dir, executable_path, manifest) = match sidecar_path(web_root, plugin_id) {
        Ok(paths) => paths,
        Err(error) => return failure(error),
    };
    let executable_path = if executable_path.is_file() {
        match executable_path.canonicalize() {
            Ok(path) if path.starts_with(&plugin_dir) => path,
            Ok(_) => return failure("Native plugin executable is outside the plugin folder."),
            Err(error) => return failure(format!("Could not resolve Rust plugin executable: {error}")),
        }
    } else {
        match compile_sidecar(web_root, &plugin_dir, &manifest) {
            Ok(path) => path,
            Err(error) => return failure(error),
        }
    };

    let input = json!({ "method": method, "args": args });
    let mut child = match Command::new(&executable_path)
        .current_dir(&plugin_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return failure(format!("Could not start Rust plugin: {error}")),
    };
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(error) = writeln!(stdin, "{input}") {
            let _ = child.kill();
            return failure(format!("Could not send a request to the Rust plugin: {error}"));
        }
    }
    let timeout_ms = manifest.get("native").and_then(|native| native.get("timeoutMs"))
        .and_then(Value::as_u64).unwrap_or(30_000).clamp(100, 120_000);
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return failure(format!("Rust plugin timed out after {timeout_ms} ms."));
            }
            Err(error) => {
                let _ = child.kill();
                return failure(format!("Could not monitor Rust plugin: {error}"));
            }
        }
    }
    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(error) => return failure(format!("Could not read the Rust plugin response: {error}")),
    };
    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr);
        return failure(format!("Rust plugin exited unsuccessfully: {}", details.trim()));
    }
    let response: Value = match serde_json::from_slice(&output.stdout) {
        Ok(response) => response,
        Err(_) => return failure("Rust plugin must write one JSON response to stdout."),
    };
    if response.get("error").is_some() {
        response.to_string()
    } else if response.get("result").is_some() {
        response.to_string()
    } else {
        json!({ "result": response }).to_string()
    }
}

/// Start a native plugin request and expose its newline-delimited JSON output
/// as a reader so the app server can stream events to its webview.
pub fn invoke_stream(
    web_root: &Path,
    plugin_id: &str,
    method: &str,
    args: &Value,
) -> Result<Box<dyn Read + Send>, String> {
    if !valid_segment(method) {
        return Err("Invalid native plugin method.".into());
    }
    let (plugin_dir, executable_path, manifest) = sidecar_path(web_root, plugin_id)?;
    let executable_path = if executable_path.is_file() {
        match executable_path.canonicalize() {
            Ok(path) if path.starts_with(&plugin_dir) => path,
            Ok(_) => return Err("Native plugin executable is outside the plugin folder.".into()),
            Err(error) => return Err(format!("Could not resolve Rust plugin executable: {error}")),
        }
    } else {
        compile_sidecar(web_root, &plugin_dir, &manifest)?
    };

    let input = json!({ "method": method, "args": args });
    let mut child = Command::new(&executable_path)
        .current_dir(&plugin_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not start Rust plugin: {error}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(error) = writeln!(stdin, "{input}") {
            let _ = child.kill();
            return Err(format!("Could not send a request to the Rust plugin: {error}"));
        }
    }
    let stdout = child.stdout.take().ok_or("Rust plugin did not provide a stream.")?;
    let mut stderr = child.stderr.take().ok_or("Could not read Rust plugin diagnostics.")?;
    std::thread::spawn(move || {
        let _ = io::copy(&mut stderr, &mut io::sink());
        let _ = child.wait();
    });
    Ok(Box::new(stdout))
}

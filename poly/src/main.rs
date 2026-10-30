use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
#[cfg(feature = "experimental-language")]
use std::io::BufRead;
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;
use notify::{Watcher, RecursiveMode, Event, EventKind};

mod packages;
mod build;
#[cfg(feature = "experimental-virtual-drive")]
mod native_app;
mod polyview_proxy;
mod scaffold;
mod system_api;
mod dev_server;
mod app_build;
use app_build::{build_app, extract_toml_value};
mod app_runtime;
use app_runtime::{format_poly_diagnostic as render_poly_diagnostic, open_url_window, run_app_result, run_browser_mode, run_native_app_impl as run_native_app};
use dev_server::{find_entry_point, handle_ipc_invoke_stateful as invoke_ipc_stateful, handle_ipc_invoke_system_only as invoke_ipc_system_only, run_dev_server};
use system_api::handle_system_api;
use scaffold::{create_project, init_project, npm_command, selected_project_template};
use polyview_proxy::{handle_polyview_proxy, handle_polyview_proxy_post};

// ANSI color codes for terminal output
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "experimental-virtual-drive")]
const NCOS_VIRTUAL_DRIVE_API: &str = r#"ncos: {
  drive: {
    async list(path = 'C:/') { return poly.invoke('__poly_virtual_drive_list', { path }); },
    async readText(path) { return poly.invoke('__poly_virtual_drive_read_text', { path }); },
    async writeText(path, contents) { return poly.invoke('__poly_virtual_drive_write_text', { path, contents }); },
    async createDirectory(path) { return poly.invoke('__poly_virtual_drive_create_directory', { path }); },
    async delete(path) { return poly.invoke('__poly_virtual_drive_delete', { path }); },
    async prepareInstall(appId, versionId) { return poly.invoke('__poly_ncos_app_prepare_install', { appId, versionId }); },
    async verifyInstall(appId, versionId) { return poly.invoke('__poly_ncos_app_install_verify', { appId, versionId }); },
    forApp(appId) {
      const id = String(appId || '').trim().toLowerCase();
      if (!/^[a-z0-9][a-z0-9._-]{0,63}$/.test(id)) throw new TypeError('Invalid NCOS app id.');
      const base = `C:/System/Apps/${id}/Data`;
      const child = (path = '') => {
        const relative = String(path || '').replace(/\\/g, '/').replace(/^\/+/, '');
        if (relative.split('/').some(part => !part || part === '.' || part === '..' || part.includes(':'))) throw new TypeError('Invalid NCOS app data path.');
        return relative ? `${base}/${relative}` : base;
      };
      return Object.freeze({
        list(path = '') { return poly.ncos.drive.list(child(path)); },
        readText(path) { return poly.ncos.drive.readText(child(path)); },
        writeText(path, contents) { return poly.ncos.drive.writeText(child(path), contents); },
        createDirectory(path) { return poly.ncos.drive.createDirectory(child(path)); },
        delete(path) { return poly.ncos.drive.delete(child(path)); }
      });
    }
  }
},"#;

#[cfg(not(feature = "experimental-virtual-drive"))]
const NCOS_VIRTUAL_DRIVE_API: &str = "";

fn range_file_response(path: &Path, range_header: Option<&str>, content_type: &str) -> tiny_http::ResponseBox {
    use tiny_http::{Header, Response, StatusCode};

    let error_response = |status, message: &str| {
        Response::from_string(message.to_string()).with_status_code(status).boxed()
    };
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return error_response(404, "Media file not found"),
    };
    let total = match file.metadata() {
        Ok(metadata) => metadata.len(),
        Err(_) => return error_response(500, "Could not read wallpaper video metadata"),
    };

    let mut start = 0u64;
    let mut end = total.saturating_sub(1);
    let mut partial = false;
    if let Some(range) = range_header {
        let parsed = range.trim().strip_prefix("bytes=").and_then(|spec| spec.split_once('-'));
        let Some((start_text, end_text)) = parsed else {
            return error_response(416, "Invalid byte range");
        };
        if start_text.is_empty() {
            let Ok(suffix_length) = end_text.parse::<u64>() else { return error_response(416, "Invalid byte range"); };
            if suffix_length == 0 || total == 0 { return error_response(416, "Invalid byte range"); }
            start = total.saturating_sub(suffix_length);
        } else {
            let Ok(parsed_start) = start_text.parse::<u64>() else { return error_response(416, "Invalid byte range"); };
            start = parsed_start;
            if !end_text.is_empty() {
                let Ok(parsed_end) = end_text.parse::<u64>() else { return error_response(416, "Invalid byte range"); };
                end = parsed_end;
            }
        }
        if total == 0 || start >= total || start > end {
            return error_response(416, "Invalid byte range");
        }
        end = end.min(total - 1);
        partial = true;
    }

    let length = if total == 0 { 0 } else { end - start + 1 };
    if length > usize::MAX as u64 || file.seek(SeekFrom::Start(start)).is_err() {
        return error_response(500, "Could not seek media file");
    }
    let mut headers = vec![
        Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap(),
        Header::from_bytes(&b"Accept-Ranges"[..], &b"bytes"[..]).unwrap(),
        Header::from_bytes(&b"Content-Length"[..], length.to_string().as_bytes()).unwrap(),
    ];
    if partial {
        headers.push(Header::from_bytes(&b"Content-Range"[..], format!("bytes {}-{}/{}", start, end, total).as_bytes()).unwrap());
    }
    let reader: Box<dyn Read + Send> = Box::new(file.take(length));
    Response::new(StatusCode(if partial { 206 } else { 200 }), headers, reader, Some(length as usize), None).boxed()
}

fn wallpaper_video_info(url: &str, web_dir: &Path) -> Option<(std::path::PathBuf, String)> {
    let encoded_path = url.split('?').next()?.trim_start_matches('/');
    let decoded = urlencoding::decode(encoded_path).ok()?;
    let filename = decoded.strip_prefix("assets/wallpapers/")?;
    if filename.is_empty() || filename.contains('/') || filename.contains('\\') { return None; }
    let path = web_dir.join("assets").join("wallpapers").join(filename);
    if !path.is_file() { return None; }
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let content_type = match ext.as_str() {
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "wmv" => "video/x-ms-wmv",
        "flv" => "video/x-flv",
        "3gp" => "video/3gpp",
        "mpeg" | "mpg" => "video/mpeg",
        "ts" => "video/mp2t",
        _ => return None,
    };
    Some((path, content_type.to_string()))
}

fn virtual_drive_media_info(url: &str) -> Result<(std::path::PathBuf, String), String> {
    let query = url.split_once('?').map(|(_, query)| query).unwrap_or("");
    let mut virtual_path = None;
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else { continue; };
        let key = urlencoding::decode(key).map_err(|_| "Invalid virtual-drive media request.".to_string())?;
        if key == "path" {
            virtual_path = Some(urlencoding::decode(value)
                .map_err(|_| "Invalid virtual-drive media path encoding.".to_string())?.into_owned());
            break;
        }
    }
    let virtual_path = virtual_path.ok_or_else(|| "The virtual-drive media path is missing.".to_string())?;
    let path = poly::virtual_drive::host_file(&virtual_path)?;
    let extension = path.extension().and_then(|value| value.to_str()).unwrap_or("").to_ascii_lowercase();
    let content_type = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "apng" => "image/apng",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "wmv" => "video/x-ms-wmv",
        "flv" => "video/x-flv",
        "3gp" => "video/3gpp",
        "mpeg" | "mpg" => "video/mpeg",
        "ts" => "video/mp2t",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
        "opus" => "audio/opus",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "wma" => "audio/x-ms-wma",
        _ => return Err("This file type is not supported by Media Player.".to_string()),
    };
    Ok((path, content_type.to_string()))
}

#[allow(dead_code)]
const GITHUB_REPO: &str = "OpenChatGit/Poly";

#[derive(Parser)]
#[command(name = "poly")]
#[command(about = "Poly - Build native apps with web technologies", long_about = None)]
#[command(disable_version_flag = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Source file to execute (experimental language)
    #[cfg(feature = "experimental-language")]
    file: Option<String>,
    
    /// Run in REPL mode
    #[arg(short, long)]
    #[cfg(feature = "experimental-language")]
    repl: bool,
    
    /// Evaluate a single expression
    #[arg(short, long)]
    #[cfg(feature = "experimental-language")]
    eval: Option<String>,
    
    /// Show version and check for updates
    #[arg(short = 'V', long = "version")]
    version: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start development server with hot reload
    Dev {
        /// Project directory (default: current directory)
        #[arg(default_value = ".")]
        path: String,
        
        /// Port for dev server
        #[arg(short, long, default_value = "3000")]
        port: u16,
        
        /// Open browser automatically
        #[arg(long)]
        open: bool,
    },
    
    /// Run the application
    Run {
        /// Project directory or file
        #[arg(default_value = ".")]
        path: String,
        
        /// Run in release mode
        #[arg(long)]
        release: bool,
        
        /// Run as native window (requires --features native)
        #[arg(long)]
        native: bool,
        
        /// Browser mode: UI WebView + Content WebView (for browser-like apps)
        #[arg(long)]
        browser: bool,
        
        /// UI height in pixels for browser mode (default: 80)
        #[arg(long, default_value = "80")]
        ui_height: u32,
    },
    
    /// Build the application
    Build {
        /// Project directory
        #[arg(default_value = ".")]
        path: String,
        
        /// Target platform (windows, macos, linux, current)
        #[arg(short, long, default_value = "current")]
        target: String,
        
        /// Release build (optimized)
        #[arg(long)]
        release: bool,
        
        /// Create installer/package
        #[arg(long)]
        installer: bool,
        
        /// Sign the executable (requires certificate)
        #[arg(long)]
        sign: bool,
        
        /// Generate GitHub Actions CI workflow
        #[arg(long)]
        ci: bool,
    },

    /// Package the CEF runtime and a Poly project into a deployable folder
    #[cfg(feature = "cef-build-util")]
    CefBundle {
        /// Poly project directory to place inside the bundle
        #[arg(default_value = ".")]
        path: String,

        /// Output directory (defaults to <project>/dist/cef)
        #[arg(short, long)]
        output: Option<String>,

        /// Use the release Poly executable
        #[arg(long)]
        release: bool,
    },
    
    /// Create a new Poly project
    New {
        /// Project name
        name: String,
        
        /// Project template (app, lib, react, svelte, vue, preact)
        #[arg(short, long, default_value = "app")]
        template: String,
        #[arg(long, conflicts_with_all = ["svelte", "vue", "preact"])]
        react: bool,
        #[arg(long, conflicts_with_all = ["react", "vue", "preact"])]
        svelte: bool,
        #[arg(long, conflicts_with_all = ["react", "svelte", "preact"])]
        vue: bool,
        #[arg(long, conflicts_with_all = ["react", "svelte", "vue"])]
        preact: bool,
        /// Create files without running npm install
        #[arg(long)]
        no_install: bool,
    },
    
    /// Initialize Poly in existing directory
    Init {
        /// Project template (app, lib, react, svelte, vue, preact)
        #[arg(default_value = "app")]
        template: String,
        #[arg(long, conflicts_with_all = ["svelte", "vue", "preact"])]
        react: bool,
        #[arg(long, conflicts_with_all = ["react", "vue", "preact"])]
        svelte: bool,
        #[arg(long, conflicts_with_all = ["react", "svelte", "preact"])]
        vue: bool,
        #[arg(long, conflicts_with_all = ["react", "svelte", "vue"])]
        preact: bool,
        /// Create files without running npm install
        #[arg(long)]
        no_install: bool,
    },
    
    /// Check for updates
    Update,
    
    /// Add a JavaScript package from npm/CDN
    Add {
        /// Package name (e.g., alpinejs, chart.js, three)
        package: String,
        
        /// Specific version (default: latest)
        #[arg(short, long)]
        version: Option<String>,
    },
    
    /// Remove a JavaScript package
    Remove {
        /// Package name to remove
        package: String,
    },
    
    /// Install packages from poly.lock (reproducible builds)
    Install {
        /// Verify hashes without reinstalling
        #[arg(long)]
        verify: bool,
    },
    
    /// Show API information for AI assistants (JSON format)
    AiInfo {
        /// Output format
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    
    /// Open a URL in a new Poly WebView window (internal use)
    #[command(hide = true)]
    OpenUrl {
        /// URL to open
        url: String,
        
        /// Window title
        #[arg(long, default_value = "Poly Browser")]
        title: String,
        
        /// Window width
        #[arg(long, default_value = "1024")]
        width: u32,
        
        /// Window height
        #[arg(long, default_value = "768")]
        height: u32,
    },
    
    /// Open browser mode with separate UI and content WebViews (internal use)
    #[command(hide = true)]
    Browser {
        /// Start URL
        #[arg(default_value = "about:blank")]
        url: String,
        
        /// Window title
        #[arg(long, default_value = "Poly Browser")]
        title: String,
        
        /// Window width
        #[arg(long, default_value = "1024")]
        width: u32,
        
        /// Window height
        #[arg(long, default_value = "768")]
        height: u32,
        
        /// UI height (titlebar + nav bar)
        #[arg(long, default_value = "80")]
        ui_height: u32,
        
        /// Path to UI HTML file
        #[arg(long)]
        ui_html: Option<String>,
    },
}

/// Get all cookies for a domain, including shared Steam cookies
/// Generate AI-friendly API documentation
fn generate_ai_info(format: &str) {
    let info = serde_json::json!({
        "poly_version": VERSION,
        "description": "Poly language API reference for AI assistants",
        "frontend_apis": {
            "poly.call": {
                "description": "Call Poly backend functions from JavaScript",
                "syntax": "await poly.call(functionName, ...args)",
                "example": "const result = await poly.call('greet', 'Alice');"
            },
            "poly.fs": {
                "methods": ["read", "write", "exists", "readDir"],
                "example": "const content = await poly.fs.read('file.txt');"
            },
            "poly.dialog": {
                "methods": ["open", "openMultiple", "save", "folder", "message", "confirm", "custom"],
                "example": "const file = await poly.dialog.open({ filters: [{ name: 'Text', extensions: ['txt'] }] });"
            },
            "poly.http": {
                "methods": ["get", "post", "put", "patch", "delete", "request"],
                "example": "const response = await poly.http.get('https://api.example.com');"
            },
            "poly.db": {
                "methods": ["open", "close", "execute", "query", "queryOne"],
                "example": "const db = await poly.db.open('db.sqlite'); const rows = await poly.db.query(db, 'SELECT * FROM users');"
            },
            "poly.window": {
                "methods": ["setTitle", "getTitle", "center", "setSize", "getSize", "setPosition", "getPosition", "setMinSize", "setMaxSize", "setAlwaysOnTop", "setFullscreen", "isFullscreen", "isMaximized", "isMinimized"],
                "example": "await poly.window.setTitle('My App');"
            },
            "poly.clipboard": {
                "methods": ["read", "write", "clear"],
                "example": "await poly.clipboard.write('Hello');"
            },
            "poly.webview": {
                "methods": ["create", "navigate", "loadHtml", "goBack", "goForward", "reload", "stop", "setBounds", "getBounds", "eval", "destroy", "list", "get", "setVisible", "focus", "setZoom", "setMainBounds", "pollEvents", "respondToPermission"],
                "example": "await poly.webview.create('my-view', { url: 'https://example.com', width: 800, height: 600 });"
            },
            "poly.notification": {
                "methods": ["show", "showWithTimeout"],
                "example": "await poly.notification.show('Title', 'Message');"
            },
            "poly.shell": {
                "methods": ["open", "openPath", "openWith"],
                "example": "await poly.shell.open('https://example.com');"
            }
        },
        "backend_syntax": {
            "variables": {
                "var": "Mutable variable with optional type annotation",
                "let": "Immutable variable with optional type annotation",
                "example": "var x: Int = 10\nlet name: String = \"Poly\""
            },
            "functions": {
                "syntax": "fn name(param: Type) -> ReturnType:",
                "example": "fn add(a: Int, b: Int) -> Int:\n    return a + b"
            },
            "structs": {
                "syntax": "struct Name:\n    fn init(inout self, ...):",
                "example": "struct Point:\n    fn init(inout self, x: Int, y: Int):\n        self.x = x\n        self.y = y"
            },
            "parameter_modifiers": {
                "inout": "Pass by mutable reference",
                "borrowed": "Pass by immutable reference",
                "owned": "Transfer ownership",
                "example": "fn increment(inout x: Int):\n    x = x + 1"
            },
            "types": ["Int", "Float", "String", "Bool", "List", "Dict"],
            "control_flow": ["if/elif/else", "for", "while", "match"],
            "operators": ["+", "-", "*", "/", "**", "%", "==", "!=", "<", ">", "<=", ">=", "and", "or", "not"]
        },
        "builtin_functions": [
            "print", "len", "range", "str", "int", "float", "bool",
            "html_escape", "url_encode", "url_decode",
            "json_parse", "json_stringify",
            "sleep", "timestamp", "format_date"
        ],
        "cli_commands": [
            "poly run <file>",
            "poly dev [path]",
            "poly build [path]",
            "poly test [path]",
            "poly init <name>",
            "poly add <package>",
            "poly install",
            "poly ai-info [--format json|markdown]",
            "poly --version",
            "poly --help"
        ],
        "project_structure": {
            "poly.toml": "Project configuration file",
            "src/main.poly": "Default entry point for applications",
            "web/": "Frontend files (HTML, CSS, JS)",
            "web/index.html": "Main HTML file for GUI apps"
        },
        "dev_workflow": {
            "create_project": "poly init my-app",
            "run_dev_server": "poly dev (starts on http://localhost:3000)",
            "build_release": "poly build",
            "test": "poly test"
        }
    });

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&info).unwrap());
        },
        "markdown" | "md" => {
            println!("# Poly API Reference for AI Assistants");
            println!();
            println!("**Version:** {}", VERSION);
            println!();
            println!("## Frontend APIs (JavaScript)");
            println!();
            println!("### `poly.call(functionName, ...args)`");
            println!("Call Poly backend functions from JavaScript.");
            println!("```javascript");
            println!("const result = await poly.call('greet', 'Alice');");
            println!("```");
            println!();
            println!("### File System (`poly.fs`)");
            println!("- `read(path)` - Read file content");
            println!("- `write(path, content)` - Write to file");
            println!("- `exists(path)` - Check if file exists");
            println!("- `readDir(path)` - List directory contents");
            println!();
            println!("### Dialogs (`poly.dialog`)");
            println!("- `open(options)` - File open dialog");
            println!("- `save(options)` - File save dialog");
            println!("- `folder(options)` - Folder selection dialog");
            println!("- `message(title, message)` - Show message");
            println!("- `confirm(title, message)` - Confirmation dialog");
            println!();
            println!("### HTTP Requests (`poly.http`)");
            println!("- `get(url, options)` - GET request");
            println!("- `post(url, body, options)` - POST request");
            println!("- `put(url, body, options)` - PUT request");
            println!("- `delete(url, options)` - DELETE request");
            println!();
            println!("## Backend Syntax (Poly)");
            println!();
            println!("### Variables");
            println!("```python");
            println!("var x: Int = 10  # Mutable");
            println!("let name: String = \"Poly\"  # Immutable");
            println!("```");
            println!();
            println!("### Functions");
            println!("```python");
            println!("fn add(a: Int, b: Int) -> Int:");
            println!("    return a + b");
            println!("```");
            println!();
            println!("### Structs");
            println!("```python");
            println!("struct Point:");
            println!("    fn init(inout self, x: Int, y: Int):");
            println!("        self.x = x");
            println!("        self.y = y");
            println!("```");
            println!();
            println!("## CLI Commands");
            println!("- `poly run <file>` - Run a Poly script");
            println!("- `poly dev` - Start development server");
            println!("- `poly build` - Build for production");
            println!("- `poly init <name>` - Create new project");
            println!("- `poly ai-info` - Show this information");
        },
        _ => {
            eprintln!("Unknown format: {}. Use 'json' or 'markdown'", format);
            std::process::exit(1);
        }
    }
}

#[cfg(feature = "cef-build-util")]
fn bundle_cef_project(path: &str, output: Option<&str>, release: bool) -> Result<(), String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (path, output, release);
        return Err("The CEF bundle command currently supports Windows builds only.".into());
    }

    #[cfg(target_os = "windows")]
    {
        let project = Path::new(path).canonicalize().map_err(|error| format!("Could not resolve project path: {error}"))?;
        let manifest = project.join("poly.toml");
        if !manifest.is_file() {
            return Err(format!("Not a Poly project: {} is missing.", manifest.display()));
        }
        let output = output.map(Path::new).map(Path::to_path_buf)
            .unwrap_or_else(|| project.join("dist").join("cef"));
        fs::create_dir_all(&output).map_err(|error| format!("Could not create bundle directory: {error}"))?;

        let executable_dir = std::env::current_exe().map_err(|error| format!("Could not locate Poly executable: {error}"))?
            .parent().ok_or_else(|| "Poly executable has no parent directory.".to_string())?.to_path_buf();
        let executable = executable_dir.join("poly.exe");
        if !executable.is_file() {
            return Err(format!("CEF-enabled Poly executable was not found at {}. Build with `cargo build -p poly-framework --features cef-build-util{}` first.", executable.display(), if release { " --release" } else { "" }));
        }
        let expected_profile = if release { "release" } else { "debug" };
        if executable_dir.file_name().and_then(|name| name.to_str()) != Some(expected_profile) {
            return Err(format!("The running Poly binary is not from the {expected_profile} profile. Launch the matching build before bundling."));
        }

        cef::build_util::win::bundle(&output, &executable_dir, "poly")
            .map_err(|error| format!("Could not bundle CEF runtime: {error}"))?;

        let bundle = output.join("bundle");
        fs::create_dir_all(&bundle).map_err(|error| format!("Could not create app bundle directory: {error}"))?;
        for name in ["web", "assets", "packages"] {
            let source = project.join(name);
            if source.is_dir() { copy_directory_for_cef(&source, &bundle.join(name))?; }
        }
        fs::copy(&manifest, bundle.join("poly.toml")).map_err(|error| format!("Could not copy poly.toml: {error}"))?;
        let entry = project.join("src").join("main.poly");
        if entry.is_file() {
            let destination = bundle.join("src").join("main.poly");
            if let Some(parent) = destination.parent() { fs::create_dir_all(parent).map_err(|error| error.to_string())?; }
            fs::copy(&entry, destination).map_err(|error| format!("Could not copy Poly entry point: {error}"))?;
        }
        println!("CEF app bundle created at {}", output.display());
        println!("The Windows sandbox is not enabled by this package mode; CEF requires its bootstrap DLL entry-point integration for sandboxed builds.");
        Ok(())
    }
}

#[cfg(feature = "cef-build-util")]
fn copy_directory_for_cef(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| format!("Could not create {}: {error}", destination.display()))?;
    for entry in fs::read_dir(source).map_err(|error| format!("Could not read {}: {error}", source.display()))? {
        let entry = entry.map_err(|error| error.to_string())?;
        let target = destination.join(entry.file_name());
        if entry.file_type().map_err(|error| error.to_string())?.is_dir() {
            copy_directory_for_cef(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(|error| format!("Could not copy {}: {error}", target.display()))?;
        }
    }
    Ok(())
}

fn main() {
    // CEF 148+ requires API version selection before any other CEF entry
    // point in every process, including renderer/GPU subprocesses.
    #[cfg(feature = "cef")]
    let _cef_api_hash = cef::api_hash(cef::sys::CEF_API_VERSION, 0);

    // CEF launches Poly itself for renderer/GPU subprocesses. Route those
    // processes into CEF before Clap parses the CEF-owned --type argument.
    #[cfg(feature = "cef")]
    if std::env::args_os().any(|arg| arg.to_string_lossy().starts_with("--type=")) {
        let args = cef::args::Args::new();
        let exit_code = cef::execute_process(Some(args.as_main_args()), None, std::ptr::null_mut());
        if exit_code >= 0 { std::process::exit(exit_code); }
    }

    let cli = Cli::parse();
    
    // Handle version flag with update check
    if cli.version {
        print_version_with_update_check();
        return;
    }
    
    let result = match cli.command {
        Some(Commands::Dev { path, port, open }) => { run_dev_server(&path, port, open); Ok(()) },
        Some(Commands::Run { path, release, native, browser, ui_height }) => run_app_result(&path, release, native, browser, ui_height),
        Some(Commands::Build { path, target, release, installer, sign, ci }) => { 
            if ci {
                build::generate_ci_workflow(Path::new(&path))
            } else {
                build_app(&path, &target, release, installer, sign)
            }
        },
        #[cfg(feature = "cef-build-util")]
        Some(Commands::CefBundle { path, output, release }) => bundle_cef_project(&path, output.as_deref(), release),
        Some(Commands::New { name, template, react, svelte, vue, preact, no_install }) => {
            selected_project_template(&template, react, svelte, vue, preact)
                .and_then(|template| create_project(&name, &template, no_install))
        },
        Some(Commands::Init { template, react, svelte, vue, preact, no_install }) => {
            selected_project_template(&template, react, svelte, vue, preact)
                .and_then(|template| init_project(&template, no_install))
        },
        Some(Commands::Update) => { check_for_updates_interactive(); Ok(()) },
        Some(Commands::Add { package, version }) => packages::add_package(&package, version.as_deref()),
        Some(Commands::Remove { package }) => packages::remove_package(&package),
        Some(Commands::Install { verify }) => packages::install_packages(verify),
        Some(Commands::AiInfo { format }) => {
            generate_ai_info(&format);
            Ok(())
        },
        Some(Commands::OpenUrl { url, title, width, height }) => {
            open_url_window(&url, &title, width, height);
            Ok(())
        },
        Some(Commands::Browser { url, title, width, height, ui_height, ui_html }) => {
            run_browser_mode(&url, &title, width, height, ui_height, ui_html);
            Ok(())
        },
        None => {
            // Check if we're running as a bundled app (bundle folder or poly.toml next to exe)
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let bundle_dir = exe_dir.join("bundle");
                    let poly_toml = exe_dir.join("poly.toml");
                    let bundle_poly_toml = bundle_dir.join("poly.toml");
                    
                    // If bundle exists, run as native app
                    if bundle_dir.exists() && (bundle_poly_toml.exists() || poly_toml.exists()) {
                        // Hide console window on Windows when running as bundled app
                        #[cfg(all(target_os = "windows", feature = "native"))]
                        {
                            use windows::Win32::System::Console::{GetConsoleWindow, FreeConsole};
                            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
                            unsafe {
                                let console = GetConsoleWindow();
                                if !console.0.is_null() {
                                    let _ = ShowWindow(console, SW_HIDE);
                                    FreeConsole().ok();
                                }
                            }
                        }
                        
                        // Run native app from bundle (default: no browser mode, ui_height 80)
                        run_native_app(exe_dir, false, false, 80);
                        return;
                    }
                }
            }
            
            #[cfg(feature = "experimental-language")]
            {
                if let Some(expr) = cli.eval {
                    match poly::eval(&expr) {
                        Ok(result) => { println!("{}", result); Ok(()) },
                        Err(e) => Err(e),
                    }
                } else if let Some(file) = cli.file {
                    run_file_result(&file)
                } else if cli.repl || atty::is(atty::Stream::Stdin) {
                    run_repl();
                    Ok(())
                } else {
                    let stdin = io::stdin();
                    let source: String = stdin.lock().lines()
                        .filter_map(|l| l.ok())
                        .collect::<Vec<_>>()
                        .join("\n");
                    poly::run(&source).map(|_| ())
                }
            }
            #[cfg(not(feature = "experimental-language"))]
            {
                Err("No command was provided. Use `poly --help` to see available commands.".into())
            }
        }
    };
    
    if let Err(e) = result {
        eprintln!("\n  {}×{} {}{}Error{}", RED, RESET, BOLD, RED, RESET);
        for line in e.lines() {
            eprintln!("    {}", line);
        }
        std::process::exit(1);
    }
}

/// Print version and check for updates
fn print_version_with_update_check() {
    println!("poly {}", VERSION);
    println!();
    
    // Check for updates
    print!("  {}Checking for updates...{}", DIM, RESET);
    io::stdout().flush().ok();
    
    #[cfg(feature = "native")]
    {
        match poly::check_github_updates(GITHUB_REPO, VERSION) {
            Ok(info) => {
                // Clear the "Checking..." line
                print!("\r                                  \r");
                
                if info.update_available {
                    println!("  {}> New version available:{} {}", GREEN, RESET, info.latest_version);
                    println!();
                    
                    if let Some(notes) = &info.release_notes {
                        let short_notes: String = notes.lines().take(3).collect::<Vec<_>>().join("\n    ");
                        if !short_notes.is_empty() {
                            println!("  {}Release notes:{}", DIM, RESET);
                            println!("    {}", short_notes);
                            println!();
                        }
                    }
                    
                    print!("  {}Download and install now? [y/N]:{} ", CYAN, RESET);
                    io::stdout().flush().ok();
                    
                    let mut input = String::new();
                    if io::stdin().read_line(&mut input).is_ok() {
                        let input = input.trim().to_lowercase();
                        if input == "y" || input == "yes" {
                            println!();
                            download_and_install_update(&info);
                        }
                    }
                } else {
                    println!("  {}>{} You're on the latest version", GREEN, RESET);
                }
            }
            Err(_e) => {
                // Clear the "Checking..." line
                print!("\r                                  \r");
                println!("  {}(Could not check for updates){}", DIM, RESET);
            }
        }
    }
    
    #[cfg(not(feature = "native"))]
    {
        print!("\r                              \r");
        println!("{}(Update check requires native feature){}", DIM, RESET);
    }
}

/// Interactive update check command
fn check_for_updates_interactive() {
    println!();
    println!("  {}POLY{} v{}  {}update{}", CYAN, RESET, VERSION, DIM, RESET);
    println!();
    
    print!("  {}Checking for updates...{}", DIM, RESET);
    io::stdout().flush().ok();
    
    #[cfg(feature = "native")]
    {
        match poly::check_github_updates(GITHUB_REPO, VERSION) {
            Ok(info) => {
                print!("\r                                    \r");
                
                if info.update_available {
                    println!("  {}>{} New version available: {}{}{}", 
                        GREEN, RESET, BOLD, info.latest_version, RESET);
                    println!("  {}>{} Current version: {}", DIM, RESET, VERSION);
                    
                    if let Some(notes) = &info.release_notes {
                        println!();
                        println!("  {}Release notes:{}", DIM, RESET);
                        for line in notes.lines().take(10) {
                            println!("  {}", line);
                        }
                    }
                    
                    println!();
                    print!("  {}Download and install? [y/N]:{} ", CYAN, RESET);
                    io::stdout().flush().ok();
                    
                    let mut input = String::new();
                    if io::stdin().read_line(&mut input).is_ok() {
                        let input = input.trim().to_lowercase();
                        if input == "y" || input == "yes" {
                            println!();
                            download_and_install_update(&info);
                        } else {
                            println!();
                            println!("  {}Update cancelled.{}", DIM, RESET);
                        }
                    }
                } else {
                    println!("  {}>{} You're already on the latest version ({})", 
                        GREEN, RESET, VERSION);
                }
            }
            Err(e) => {
                print!("\r                                    \r");
                println!("  {}>{} {}Failed to check for updates:{} {}", RED, RESET, RED, RESET, e);
            }
        }
    }
    
    #[cfg(not(feature = "native"))]
    {
        print!("\r                                    \r");
        println!("  {}>{} Update check requires native feature", YELLOW, RESET);
        println!("  {}>{} Rebuild with: cargo install --path poly --features native", DIM, RESET);
    }
    
    println!();
}

/// Download and install update
#[cfg(feature = "native")]
fn download_and_install_update(info: &poly::UpdateInfo) {
    if let Some(url) = &info.download_url {
        print!("  {}Downloading update...{}", DIM, RESET);
        io::stdout().flush().ok();
        
        // Progress callback
        let progress = |downloaded: u64, total: u64| {
            if total > 0 {
                let percent = downloaded.saturating_mul(100).checked_div(total).unwrap_or(0);
                print!("\r  {}Downloading update... {}%{}", DIM, percent, RESET);
                io::stdout().flush().ok();
            }
        };
        
        match poly::download_update(url, Some(Box::new(progress))) {
            Ok(path) => {
                println!("\r  {}>{} Downloaded to: {}                    ", GREEN, RESET, path.display());
                print!("  {}Installing...{}", DIM, RESET);
                io::stdout().flush().ok();
                
                match poly::install_update(&path) {
                    Ok(_) => {
                        println!("\r  {}>{} Update installed successfully!          ", GREEN, RESET);
                        println!("  {}>{} Please restart Poly to use the new version.", DIM, RESET);
                    }
                    Err(e) => {
                        println!("\r  {}>{} {}Installation failed:{} {}          ", RED, RESET, RED, RESET, e);
                        println!("  {}>{} You can manually install from: {}", DIM, RESET, path.display());
                    }
                }
            }
            Err(e) => {
                println!("\r  {}>{} {}Download failed:{} {}          ", RED, RESET, RED, RESET, e);
            }
        }
    } else {
        println!("  {}>{} {}No download URL available for your platform{}", YELLOW, RESET, YELLOW, RESET);
        println!("  {}>{} Please download manually from GitHub", DIM, RESET);
    }
}

#[cfg(not(feature = "native"))]
#[allow(dead_code)]
fn download_and_install_update(_info: &poly::UpdateInfo) {
    println!("  {}>{} Update requires native feature", YELLOW, RESET);
}

#[cfg(feature = "experimental-language")]
fn run_file_result(file: &str) -> Result<(), String> {
    let source = fs::read_to_string(file)
        .map_err(|e| format!("Failed to read '{}': {}", file, e))?;
    poly::run(&source).map(|_| ())
}

#[cfg(feature = "experimental-language")]
fn run_repl() {
    println!();
    println!("  {}POLY{} v{}", CYAN, RESET, VERSION);
    println!("  {}Type 'exit' to quit{}", DIM, RESET);
    println!();
    
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    loop {
        print!("{}>{} ", CYAN, RESET);
        stdout.flush().unwrap();

        let mut input = String::new();
        if stdin.read_line(&mut input).is_err() {
            break;
        }
        
        let input = input.trim();
        if input.is_empty() { continue; }
        if input == "exit" || input == "quit" { break; }
        
        match poly::eval(input) {
            Ok(result) => {
                if result != "none" {
                    println!("{}", result);
                }
            }
            Err(_) => {
                if let Err(e) = poly::run(input) {
                    eprintln!("{}error{}: {}", RED, RESET, e);
                }
            }
        }
    }
}


/// Find a free port to use for the local server
fn find_free_port() -> Option<u16> {
    use std::net::TcpListener;
    
    // Try to bind to port 0 to get a random free port
    if let Ok(listener) = TcpListener::bind("127.0.0.1:0") {
        if let Ok(addr) = listener.local_addr() {
            return Some(addr.port());
        }
    }
    
    // Fallback: try common ports
    for port in [9473, 9474, 9475, 9476, 9477, 9478, 9479, 9480] {
        if TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok() {
            return Some(port);
        }
    }
    
    None
}

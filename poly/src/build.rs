//! Cross-Platform Build System for Poly
//! Builds native executables with bundled web assets

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

// ANSI colors
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
#[allow(dead_code)]
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

/// Build target platform
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
    Current,
}

impl Platform {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "windows" | "win" | "win64" => Some(Platform::Windows),
            "macos" | "mac" | "darwin" | "osx" => Some(Platform::MacOS),
            "linux" => Some(Platform::Linux),
            "current" | "native" => Some(Platform::Current),
            _ => None,
        }
    }
    
    pub fn current() -> Self {
        #[cfg(target_os = "windows")]
        return Platform::Windows;
        #[cfg(target_os = "macos")]
        return Platform::MacOS;
        #[cfg(target_os = "linux")]
        return Platform::Linux;
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Platform::Windows => "windows",
            Platform::MacOS => "macos",
            Platform::Linux => "linux",
            Platform::Current => Platform::current().name(),
        }
    }
    
    pub fn exe_extension(&self) -> &'static str {
        match self {
            Platform::Windows => ".exe",
            Platform::MacOS | Platform::Linux => "",
            Platform::Current => Platform::current().exe_extension(),
        }
    }
    
    #[allow(dead_code)]
    pub fn rust_target(&self) -> &'static str {
        match self {
            Platform::Windows => "x86_64-pc-windows-msvc",
            Platform::MacOS => "x86_64-apple-darwin",
            Platform::Linux => "x86_64-unknown-linux-gnu",
            Platform::Current => "",
        }
    }
}

/// Build configuration
#[allow(dead_code)]
pub struct BuildConfig {
    pub project_path: std::path::PathBuf,
    pub platform: Platform,
    pub release: bool,
    pub bundle: bool,
    pub installer: bool,
    /// Requested installer format; `None` uses the platform default.
    pub installer_format: Option<String>,
    pub sign: bool,
}

/// Signing configuration (from poly.toml or environment)
#[derive(Debug, Clone, Default)]
pub struct SigningConfig {
    // Windows
    pub windows_certificate: Option<String>,      // Path to .pfx file
    pub windows_certificate_password: Option<String>,
    pub windows_timestamp_url: Option<String>,
    
    // macOS
    pub macos_identity: Option<String>,           // Developer ID Application: Name
    /// `Developer ID Installer` identity, used by `productsign` for `.pkg`.
    /// Distinct from `macos_identity`; productsign rejects an Application cert.
    pub macos_installer_identity: Option<String>,
    pub macos_entitlements: Option<String>,       // Path to entitlements.plist
    pub macos_notarize_apple_id: Option<String>,
    pub macos_notarize_password: Option<String>,  // App-specific password
    pub macos_notarize_team_id: Option<String>,
}

/// Build result
#[allow(dead_code)]
pub struct BuildResult {
    pub output_path: std::path::PathBuf,
    pub size_bytes: u64,
}

/// Main build function
pub fn build(config: &BuildConfig) -> Result<BuildResult, String> {
    // Native runtime builds are supported only for the OS running Poly. Resolve
    // `current` from the host and reject explicit foreign targets before doing
    // frontend/plugin work, so e.g. a macOS runner never starts a Windows build.
    let host_platform = Platform::current();
    if config.platform != Platform::Current && config.platform != host_platform {
        return Err(format!(
            "Cannot build a {} app on {}. Run Poly on the target OS, or use `poly build --ci` to generate a per-OS GitHub Actions workflow.",
            config.platform.name(),
            host_platform.name(),
        ));
    }

    let project_path = &config.project_path;
    
    // Read project config
    let poly_toml = project_path.join("poly.toml");
    if !poly_toml.exists() {
        return Err("Not a Poly project. No poly.toml found.".to_string());
    }
    
    let toml_content = fs::read_to_string(&poly_toml)
        .map_err(|e| format!("Failed to read poly.toml: {}", e))?;
    
    let app_name = extract_toml_value(&toml_content, "name")
        .unwrap_or_else(|| "app".to_string());
    let app_version = extract_toml_value(&toml_content, "version")
        .unwrap_or_else(|| "1.0.0".to_string());

    if project_path.join("package.json").is_file() && crate::scaffold::has_vite_config(project_path) {
        run_frontend_build(project_path)?;
    }
    build_rust_plugins(project_path)?;
    
    println!();
    println!("  {}POLY{} {}build{}", CYAN, RESET, DIM, RESET);
    println!();
    println!("  {}>{} App:      {}{}{}", DIM, RESET, BOLD, app_name, RESET);
    println!("  {}>{} Version:  {}", DIM, RESET, app_version);
    println!("  {}>{} Platform: {}", DIM, RESET, config.platform.name());
    println!("  {}>{} Mode:     {}", DIM, RESET, if config.release { "release" } else { "debug" });
    if config.sign {
        println!("  {}>{} Signing:  enabled", DIM, RESET);
    }
    println!();
    
    let start = std::time::Instant::now();
    
    // Create dist directory
    let dist_dir = project_path.join("dist").join(config.platform.name());
    fs::create_dir_all(&dist_dir)
        .map_err(|error| format!("Could not create output directory {}: {error}", dist_dir.display()))?;
    
    // Step 1: Bundle web assets
    let bundle_progress = build_progress("Bundling web assets...");
    
    let bundle_dir = dist_dir.join("bundle");
    if let Err(error) = bundle_web_assets(project_path, &bundle_dir) {
        bundle_progress.abandon_with_message(format!("{}×{} Asset bundling failed", RED, RESET));
        return Err(error);
    }
    
    bundle_progress.finish_with_message(format!("{}✓{} Bundled web assets", GREEN, RESET));
    
    // Step 2: Create launcher/executable
    let executable_progress = build_progress("Compiling app runtime...");
    
    let exe_name = format!("{}{}", app_name, config.platform.exe_extension());
    let exe_path = dist_dir.join(&exe_name);
    
    if let Err(error) = create_launcher(project_path, &exe_path, &bundle_dir, config) {
        executable_progress.abandon_with_message(format!("{}×{} App runtime build failed", RED, RESET));
        return Err(error);
    }
    
    let size = fs::metadata(&exe_path)
        .map_err(|error| format!("Could not inspect built executable {}: {error}", exe_path.display()))?
        .len();
    executable_progress.finish_with_message(format!("{}✓{} Created {} ({:.1} MB)", GREEN, RESET, exe_name, size as f64 / 1_000_000.0));
    
    // Step 3: Sign the executable (optional).
    //
    // On macOS the signature must cover the assembled `.app`, because codesign
    // seals Resources/ — signing the bare binary here and then adding resources
    // would leave an invalid signature. macOS is signed in step 4 instead.
    let signing_config = load_signing_config(&config.project_path);
    let needs_app_bundle = config.platform == Platform::MacOS
        || (config.platform == Platform::Current && Platform::current() == Platform::MacOS);
    if config.sign && !needs_app_bundle {
        let signing_progress = build_progress("Signing executable...");
        match sign_executable(&exe_path, &signing_config) {
            Ok(()) => {
                signing_progress.finish_with_message(format!("{}✓{} Signed executable", GREEN, RESET));
            }
            Err(e) => {
                signing_progress.abandon_with_message(format!("{}×{} Signing failed", RED, RESET));
                return Err(format!("Could not sign {}: {}", exe_path.display(), e));
            }
        }
    }
    
    // Step 4: Assemble the launchable app. On macOS an `.app` is the runnable
    // artifact, so it is always produced; `--installer` then wraps it in a
    // distributable disk image.
    if needs_app_bundle {
        if Platform::current() != Platform::MacOS {
            return Err("macOS app bundles must be built on macOS.".to_string());
        }
        let app_progress = build_progress("Creating macOS app bundle...");
        let app_path = match create_macos_app_bundle(
            &dist_dir,
            project_path,
            &app_name,
            &app_version,
            Some(&dist_dir.join("bundle")),
        ) {
            Ok(app_path) => {
                app_progress.finish_with_message(format!("{}✓{} Created {}", GREEN, RESET,
                    app_path.file_name().unwrap_or_default().to_string_lossy()));
                app_path
            }
            Err(e) => {
                app_progress.abandon_with_message(format!("{}×{} App bundle creation failed", RED, RESET));
                return Err(format!("Could not create app bundle: {}", e));
            }
        };

        // Sign the finished bundle so Resources/ is covered by the seal. The
        // linker signs the bare binary before Resources/ exists, so an unsigned
        // build still needs re-signing ad-hoc to be valid. A Developer ID is used
        // when configured; otherwise the bundle is ad-hoc signed, which needs no
        // certificate and is what makes it launchable on Apple Silicon.
        {
            let sign_progress = build_progress("Signing app bundle...");
            let result = match &signing_config.macos_identity {
                Some(identity) if config.sign => {
                    let result = sign_macos(&app_path, &signing_config);
                    if result.is_ok() {
                        println!("  {}>{} Identity: {}", DIM, RESET, identity);
                    }
                    result
                }
                _ => adhoc_sign_macos(&app_path),
            };

            match result {
                Ok(()) => sign_progress.finish_with_message(format!(
                    "{}✓{} Signed app bundle{}",
                    GREEN,
                    RESET,
                    if config.sign && signing_config.macos_identity.is_some() {
                        " (Developer ID)"
                    } else {
                        " (ad-hoc)"
                    }
                )),
                Err(e) => {
                    sign_progress.abandon_with_message(format!("{}×{} Signing failed", RED, RESET));
                    return Err(format!("Could not sign {}: {}", app_path.display(), e));
                }
            }
        }

        if config.sign && signing_config.macos_identity.is_none() {
            println!(
                "  {}warning{}: --sign was requested but no macOS identity is configured; \
                 the app was ad-hoc signed. Set POLY_MACOS_IDENTITY or [signing.macos] \
                 identity to sign with a Developer ID.",
                YELLOW, RESET
            );
        }
    }

    // Step 5: Create a distributable installer (optional)
    if config.installer {
        let installer_progress = build_progress("Creating installer...");
        match create_installer(
            &dist_dir,
            project_path,
            &app_name,
            &app_version,
            config.platform,
            config.installer_format.as_deref(),
            config.sign,
        ) {
            Ok(installer_path) => {
                installer_progress.finish_with_message(format!("{}✓{} Created installer: {}", GREEN, RESET,
                    installer_path.file_name().unwrap_or_default().to_string_lossy()));
            }
            Err(e) => {
                installer_progress.abandon_with_message(format!("{}×{} Installer creation failed", RED, RESET));
                return Err(format!("Could not create installer: {}", e));
            }
        }
    }
    
    let elapsed = start.elapsed();
    println!();
    println!("  {}✓{} Build completed in {:.1}s", GREEN, RESET, elapsed.as_secs_f64());
    println!();
    println!("  {}Output:{} {}", DIM, RESET, dist_dir.display());
    
    Ok(BuildResult {
        output_path: exe_path,
        size_bytes: size,
    })
}

fn build_progress(message: &str) -> ProgressBar {
    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::with_template("  {spinner:.cyan} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    progress.set_message(message.to_string());
    progress.enable_steady_tick(Duration::from_millis(90));
    progress
}

/// Bundle web assets into a directory
fn bundle_web_assets(project_path: &Path, bundle_dir: &Path) -> Result<(), String> {
    // Clean and create bundle directory
    if bundle_dir.exists() {
        fs::remove_dir_all(bundle_dir)
            .map_err(|e| format!("Failed to replace old bundle {}: {}", bundle_dir.display(), e))?;
    }
    fs::create_dir_all(bundle_dir).map_err(|e| format!("Failed to create bundle dir: {}", e))?;
    
    // Copy web directory
    let config = poly::PolyConfig::load(project_path);
    let web_dir = project_path.join(&config.web.dir);
    if web_dir.exists() {
        let relative = Path::new(&config.web.dir);
        if relative.is_absolute() || relative.components().any(|part| !matches!(part, std::path::Component::Normal(_))) {
            return Err(format!("Configured web output directory '{}' must be a relative path without '..'.", config.web.dir));
        }
        copy_dir_recursive(&web_dir, &bundle_dir.join(relative))?;
    } else {
        return Err(format!("Configured web output directory '{}' does not exist.", web_dir.display()));
    }
    
    // Copy assets directory
    let assets_dir = project_path.join("assets");
    if assets_dir.exists() {
        copy_dir_recursive(&assets_dir, &bundle_dir.join("assets"))?;
    }
    
    // Copy packages directory (npm packages)
    let packages_dir = project_path.join("packages");
    if packages_dir.exists() {
        copy_dir_recursive(&packages_dir, &bundle_dir.join("packages"))?;
    }
    
    // Copy poly.toml
    let poly_toml = project_path.join("poly.toml");
    if poly_toml.exists() {
        fs::copy(&poly_toml, bundle_dir.join("poly.toml"))
            .map_err(|e| format!("Failed to copy {}: {}", poly_toml.display(), e))?;
    }
    

    
    Ok(())
}

fn build_rust_plugins(project_path: &Path) -> Result<(), String> {
    let project_config = poly::PolyConfig::load(project_path);
    let web_root = project_path.join(&project_config.web.dir);
    let plugins_dir = web_root.join("plugins");
    if !plugins_dir.is_dir() { return Ok(()); }

    for entry in fs::read_dir(&plugins_dir).map_err(|error| format!("Could not read plugins directory: {error}"))? {
        let entry = entry.map_err(|error| format!("Could not read plugin entry: {error}"))?;
        if !entry.path().is_dir() { continue; }
        let plugin_dir = entry.path();
        let manifest_path = plugin_dir.join("manifest.json");
        if !manifest_path.is_file() { continue; }
        let manifest: serde_json::Value = serde_json::from_slice(&fs::read(&manifest_path)
            .map_err(|error| format!("Could not read {}: {error}", manifest_path.display()))?)
            .map_err(|error| format!("Invalid plugin manifest {}: {error}", manifest_path.display()))?;
        if manifest.get("enabled").and_then(serde_json::Value::as_bool) != Some(true) { continue; }
        let Some(native) = manifest.get("native") else { continue; };
        if native.get("runtime").and_then(serde_json::Value::as_str) != Some("rust-sidecar") { continue; }

        let plugin_id = entry.file_name().to_string_lossy().to_string();
        if plugin_id.is_empty() || !plugin_id.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')) {
            return Err(format!("Invalid Rust plugin directory name: {plugin_id}"));
        }
        let crate_relative = native.get("crate").and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("Rust plugin '{plugin_id}' must declare native.crate."))?;
        let binary_name = native.get("binary").and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("Rust plugin '{plugin_id}' must declare native.binary."))?;
        let executable_relative = native.get("executable").and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("Rust plugin '{plugin_id}' must declare native.executable."))?;
        if !binary_name.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')) {
            return Err(format!("Invalid Rust binary name in plugin '{plugin_id}'."));
        }
        let safe_relative = |value: &str| {
            let path = Path::new(value);
            !path.is_absolute() && path.components().all(|part| matches!(part, std::path::Component::Normal(_)))
        };
        if !safe_relative(crate_relative) || !safe_relative(executable_relative) {
            return Err(format!("Rust plugin '{plugin_id}' paths must stay within its plugin directory."));
        }
        let crate_manifest = plugin_dir.join(crate_relative).canonicalize()
            .map_err(|error| format!("Rust crate for plugin '{plugin_id}' was not found: {error}"))?;
        let canonical_plugin_dir = plugin_dir.canonicalize().map_err(|error| format!("Could not resolve plugin directory: {error}"))?;
        if !crate_manifest.starts_with(&canonical_plugin_dir) {
            return Err(format!("Rust crate for plugin '{plugin_id}' is outside the plugin directory."));
        }

        println!("  {}>{} Building Rust plugin: {}", DIM, RESET, plugin_id);
        let target_dir = project_path.join("dist/native-plugins").join(&plugin_id);
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let output = Command::new(cargo)
            .args(["build", "--release", "--manifest-path"])
            .arg(&crate_manifest)
            .arg("--target-dir")
            .arg(&target_dir)
            .current_dir(&plugin_dir)
            .output()
            .map_err(|error| format!("Could not start Cargo for Rust plugin '{plugin_id}': {error}"))?;
        if !output.status.success() {
            let details = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Rust plugin '{plugin_id}' failed to compile: {}", details.trim()));
        }
        let executable_name = if cfg!(windows) { format!("{binary_name}.exe") } else { binary_name.to_string() };
        let built = target_dir.join("release").join(executable_name);
        if !built.is_file() {
            return Err(format!("Rust plugin '{plugin_id}' did not produce the binary '{}'", built.display()));
        }
        let staged_path = plugin_dir.join(executable_relative);
        if let Some(parent) = staged_path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("Could not create Rust plugin output folder: {error}"))?;
        }
        fs::copy(&built, &staged_path).map_err(|error| format!("Could not stage Rust plugin '{plugin_id}': {error}"))?;
    }
    Ok(())
}

fn run_frontend_build(project_path: &Path) -> Result<(), String> {
    let progress = build_progress("Building frontend with Vite...");
    let output = npm_command()
        .args(["run", "build"])
        .current_dir(project_path)
        .output()
        .map_err(|error| {
            progress.abandon_with_message(format!("{}×{} Frontend build could not start", RED, RESET));
            format!("Could not start npm run build: {error}")
        })?;
    print_cargo_output(&output.stdout, &output.stderr);
    if !output.status.success() {
        progress.abandon_with_message(format!("{}×{} Frontend build failed", RED, RESET));
        let status = output.status.code().map(|code| format!("exit code {code}"))
            .unwrap_or_else(|| "terminated by a signal".to_string());
        return Err(format!("npm run build failed ({status})."));
    }
    progress.finish_with_message(format!("{}✓{} Frontend build completed", GREEN, RESET));
    Ok(())
}

pub fn build_frontend(project_path: &Path) -> Result<(), String> {
    if project_path.join("package.json").is_file() && crate::scaffold::has_vite_config(project_path) {
        run_frontend_build(project_path)?;
    }
    Ok(())
}

fn npm_command() -> Command {
    if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/C", "npm"]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        command
    } else {
        Command::new("npm")
    }
}

/// Create the launcher executable
fn create_launcher(
    _project_path: &Path,
    exe_path: &Path,
    _bundle_dir: &Path,
    config: &BuildConfig,
) -> Result<(), String> {
    // For current platform, build a GUI version of poly (no console window)
    if config.platform == Platform::Current || config.platform == Platform::current() {
        // Build poly with gui feature to hide console on Windows
        let cargo_path = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        
        // Find the poly crate directory (relative to current exe or workspace)
        let poly_crate = find_poly_crate()?;
        
        let mut cmd = Command::new(&cargo_path);
        cmd.current_dir(&poly_crate);
        cmd.arg("build")
            .arg("-p").arg("poly-framework")
            .arg("--features").arg("native,gui")
            .arg("--quiet");
        
        if config.release {
            cmd.arg("--release");
        }
        
        let output = cmd.output()
            .map_err(|e| format!("Could not start Cargo build: {}", e))?;
        
        if !output.status.success() {
            print_cargo_output(&output.stdout, &output.stderr);
            let status = output.status.code()
                .map(|code| format!("exit code {code}"))
                .unwrap_or_else(|| "terminated by a signal".to_string());
            return Err(format!("Cargo could not compile the Poly app runtime ({status}). See the compiler output above."));
        }

        if !output.stdout.is_empty() || !output.stderr.is_empty() {
            print_cargo_output(&output.stdout, &output.stderr);
        }

        let target_dir = std::env::var_os("CARGO_TARGET_DIR")
            .map(std::path::PathBuf::from)
            .or_else(|| poly_crate.parent().map(|workspace| workspace.join("target")))
            .unwrap_or_else(|| poly_crate.join("target"));
        let profile = if config.release { "release" } else { "debug" };
        let built_exe = target_dir.join(profile).join(format!("poly{}", Platform::current().exe_extension()));
        if !built_exe.is_file() {
            return Err(format!("Cargo succeeded, but the app runtime executable was not found at {}.", built_exe.display()));
        }
        fs::copy(&built_exe, exe_path)
            .map_err(|e| format!("Could not copy the compiled app runtime: {}", e))?;
        
        Ok(())
    } else {
        // Cross-compilation would require cargo cross or similar
        Err(format!(
            "Cross-compilation to {} requires building on that platform or using GitHub Actions.\n\
             Run 'poly build --ci' to generate a GitHub Actions workflow.",
            config.platform.name()
        ))
    }
}

fn print_cargo_output(stdout: &[u8], stderr: &[u8]) {
    if !stdout.is_empty() {
        let _ = io::stdout().write_all(stdout);
        if !stdout.ends_with(b"\n") { println!(); }
    }
    if !stderr.is_empty() {
        let _ = io::stderr().write_all(stderr);
        if !stderr.ends_with(b"\n") { eprintln!(); }
    }
}

/// Find the poly crate directory
fn find_poly_crate() -> Result<std::path::PathBuf, String> {
    // Try relative to current exe
    if let Ok(exe_path) = std::env::current_exe() {
        // Check if we're in target/release or target/debug
        if let Some(target_dir) = exe_path.parent() {
            if let Some(parent) = target_dir.parent() {
                // We might be in workspace/target/release
                let poly_crate = parent.parent().map(|p| p.join("poly"));
                if let Some(ref p) = poly_crate {
                    if p.join("Cargo.toml").exists() {
                        return Ok(p.clone());
                    }
                }
                // Or workspace root
                if let Some(workspace) = parent.parent() {
                    let poly_crate = workspace.join("poly");
                    if poly_crate.join("Cargo.toml").exists() {
                        return Ok(poly_crate);
                    }
                }
            }
        }
    }
    
    // Try current directory
    let cwd = std::env::current_dir().map_err(|e| format!("Failed to get cwd: {}", e))?;
    let poly_path = cwd.join("poly");
    for candidate in [poly_path.join("poly"), poly_path] {
        let manifest = candidate.join("Cargo.toml");
        if manifest.is_file()
            && fs::read_to_string(manifest)
                .unwrap_or_default()
                .contains("name = \"poly-framework\"")
        {
            return Ok(candidate);
        }
    }
    
    // Check if we're in the poly crate itself
    if cwd.join("Cargo.toml").exists() {
        let content = fs::read_to_string(cwd.join("Cargo.toml")).unwrap_or_default();
        if content.contains("name = \"poly-framework\"") {
            return Ok(cwd);
        }
    }
    
    Err("Could not find poly crate. Make sure you're in the Poly workspace.".to_string())
}

/// Create the platform's distributable package.
///
/// Installs are a separate concern from building: the macOS `.app` is already a
/// runnable artifact produced by the build, so `--installer` only wraps it in a
/// distributable. On macOS that is a `.dmg` by default, or a `.pkg` when
/// requested with `--installer-format pkg`.
fn create_installer(
    dist_dir: &Path,
    project_path: &Path,
    app_name: &str,
    app_version: &str,
    platform: Platform,
    format: Option<&str>,
    sign: bool,
) -> Result<std::path::PathBuf, String> {
    if platform != Platform::Current && platform != Platform::current() {
        return Err(
            "Cross-platform installers must be built on the target platform.".to_string(),
        );
    }

    match Platform::current() {
        Platform::Windows => {
            if let Some(format) = format.filter(|format| !format.trim().is_empty()) {
                return Err(format!(
                    "Unsupported installer format '{format}' for Windows. Omit --installer-format to build the default installer."
                ));
            }
            create_windows_installer(dist_dir, app_name, app_version)
        }
        Platform::MacOS => {
            let format = InstallerFormat::parse(format, Platform::MacOS)?;
            let app_bundle = dist_dir.parent().unwrap_or(dist_dir).join(format!("{}.app", app_name));
            let package = match format {
                InstallerFormat::Dmg => create_macos_dmg(dist_dir, app_name, app_version),
                InstallerFormat::Pkg => {
                    create_macos_pkg(dist_dir, app_name, app_version, &app_bundle)
                }
            }?;

            // A `.pkg` is signed with productsign and an Installer certificate.
            // Without one the package is left unsigned; the postinstall script
            // still ad-hoc signs the installed app so it launches locally.
            if sign && format == InstallerFormat::Pkg {
                let signing_config = load_signing_config(project_path);
                match &signing_config.macos_installer_identity {
                    Some(identity) => {
                        sign_macos_pkg(&package, &signing_config)?;
                        println!("  {}>{} Package identity: {}", DIM, RESET, identity);
                    }
                    None => println!(
                        "  {}warning{}: --sign was requested but no Developer ID Installer \
                         identity is configured; the .pkg is unsigned. Set \
                         POLY_MACOS_INSTALLER_IDENTITY or [signing.macos] installer_identity.",
                        YELLOW, RESET
                    ),
                }
            }

            Ok(package)
        }
        Platform::Linux => {
            if let Some(format) = format.filter(|format| !format.trim().is_empty()) {
                return Err(format!(
                    "Unsupported installer format '{format}' for Linux. Omit --installer-format to build the default package."
                ));
            }
            create_linux_appimage(dist_dir, app_name, app_version)
        }
        Platform::Current => unreachable!(),
    }
}

// ============================================
// Code Signing
// ============================================

/// Load signing configuration from poly.toml and environment variables
pub fn load_signing_config(project_path: &Path) -> SigningConfig {
    let mut config = SigningConfig::default();
    
    // Try to load from poly.toml
    let poly_toml = project_path.join("poly.toml");
    if poly_toml.exists() {
        if let Ok(content) = fs::read_to_string(&poly_toml) {
            let mut _in_signing_section = false;
            let mut in_windows_section = false;
            let mut in_macos_section = false;
            
            for line in content.lines() {
                let line = line.trim();
                
                if line == "[signing]" {
                    _in_signing_section = true;
                    in_windows_section = false;
                    in_macos_section = false;
                } else if line == "[signing.windows]" {
                    _in_signing_section = false;
                    in_windows_section = true;
                    in_macos_section = false;
                } else if line == "[signing.macos]" {
                    _in_signing_section = false;
                    in_windows_section = false;
                    in_macos_section = true;
                } else if line.starts_with('[') {
                    _in_signing_section = false;
                    in_windows_section = false;
                    in_macos_section = false;
                } else if in_windows_section {
                    if let Some(val) = extract_toml_value_line(line, "certificate") {
                        config.windows_certificate = Some(val);
                    } else if let Some(val) = extract_toml_value_line(line, "timestamp_url") {
                        config.windows_timestamp_url = Some(val);
                    }
                } else if in_macos_section {
                    if let Some(val) = extract_toml_value_line(line, "identity") {
                        config.macos_identity = Some(val);
                    } else if let Some(val) = extract_toml_value_line(line, "installer_identity") {
                        config.macos_installer_identity = Some(val);
                    } else if let Some(val) = extract_toml_value_line(line, "entitlements") {
                        config.macos_entitlements = Some(val);
                    } else if let Some(val) = extract_toml_value_line(line, "team_id") {
                        config.macos_notarize_team_id = Some(val);
                    }
                }
            }
        }
    }
    
    // Override with environment variables (for CI/CD)
    if let Ok(val) = std::env::var("POLY_WINDOWS_CERTIFICATE") {
        config.windows_certificate = Some(val);
    }
    if let Ok(val) = std::env::var("POLY_WINDOWS_CERTIFICATE_PASSWORD") {
        config.windows_certificate_password = Some(val);
    }
    if let Ok(val) = std::env::var("POLY_WINDOWS_TIMESTAMP_URL") {
        config.windows_timestamp_url = Some(val);
    }
    if let Ok(val) = std::env::var("POLY_MACOS_IDENTITY") {
        config.macos_identity = Some(val);
    }
    if let Ok(val) = std::env::var("POLY_MACOS_INSTALLER_IDENTITY") {
        config.macos_installer_identity = Some(val);
    }
    if let Ok(val) = std::env::var("POLY_MACOS_ENTITLEMENTS") {
        config.macos_entitlements = Some(val);
    }
    if let Ok(val) = std::env::var("POLY_MACOS_APPLE_ID") {
        config.macos_notarize_apple_id = Some(val);
    }
    if let Ok(val) = std::env::var("POLY_MACOS_APP_PASSWORD") {
        config.macos_notarize_password = Some(val);
    }
    if let Ok(val) = std::env::var("POLY_MACOS_TEAM_ID") {
        config.macos_notarize_team_id = Some(val);
    }
    
    config
}

fn extract_toml_value_line(line: &str, key: &str) -> Option<String> {
    if line.starts_with(&format!("{} =", key)) || line.starts_with(&format!("{}=", key)) {
        if let Some(value) = line.split('=').nth(1) {
            return Some(value.trim().trim_matches('"').to_string());
        }
    }
    None
}

/// Sign an executable for the current platform
pub fn sign_executable(exe_path: &Path, signing_config: &SigningConfig) -> Result<(), String> {
    match Platform::current() {
        Platform::Windows => sign_windows(exe_path, signing_config),
        Platform::MacOS => sign_macos(exe_path, signing_config),
        Platform::Linux => {
            Err("Linux executable signing is not implemented. Disable signing or provide a platform signing tool.".to_string())
        }
        Platform::Current => unreachable!(),
    }
}

/// Apply an ad-hoc signature to a macOS app bundle.
///
/// Apple Silicon requires every Mach-O to carry at least an ad-hoc signature, and
/// the signature must cover the assembled bundle. The binary is signed by the
/// linker *before* `Resources/` is populated, so the bundle needs re-signing or
/// `codesign --verify` reports the resources are unsealed. Ad-hoc signing needs
/// no certificate and no Apple account.
fn adhoc_sign_macos(app_path: &Path) -> Result<(), String> {
    let output = Command::new("codesign")
        .arg("--force")
        .arg("--sign").arg("-")
        .arg(app_path)
        .output()
        .map_err(|error| format!("Failed to run codesign: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "codesign failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Sign Windows executable using signtool
fn sign_windows(exe_path: &Path, config: &SigningConfig) -> Result<(), String> {
    let cert_path = config.windows_certificate.as_ref()
        .ok_or("No Windows certificate configured. Set POLY_WINDOWS_CERTIFICATE or [signing.windows] certificate in poly.toml")?;
    
    let password = config.windows_certificate_password.as_ref()
        .ok_or("No certificate password. Set POLY_WINDOWS_CERTIFICATE_PASSWORD environment variable")?;
    
    // Find signtool.exe
    let signtool = find_signtool()
        .ok_or("signtool.exe not found. Install Windows SDK or Visual Studio.")?;
    
    let mut cmd = Command::new(&signtool);
    cmd.arg("sign")
        .arg("/f").arg(cert_path)
        .arg("/p").arg(password)
        .arg("/fd").arg("SHA256");
    
    // Add timestamp if configured
    if let Some(ref ts_url) = config.windows_timestamp_url {
        cmd.arg("/tr").arg(ts_url)
            .arg("/td").arg("SHA256");
    } else {
        // Default timestamp server
        cmd.arg("/tr").arg("http://timestamp.digicert.com")
            .arg("/td").arg("SHA256");
    }
    
    cmd.arg(exe_path);
    
    let output = cmd.output()
        .map_err(|e| format!("Failed to run signtool: {}", e))?;
    
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("signtool failed: {}", String::from_utf8_lossy(&output.stderr)))
    }
}

/// Find signtool.exe on Windows
fn find_signtool() -> Option<std::path::PathBuf> {
    // Common locations for signtool
    let program_files = std::env::var("ProgramFiles(x86)")
        .or_else(|_| std::env::var("ProgramFiles"))
        .unwrap_or_else(|_| "C:\\Program Files (x86)".to_string());
    
    let sdk_paths = [
        format!("{}\\Windows Kits\\10\\bin\\10.0.22621.0\\x64\\signtool.exe", program_files),
        format!("{}\\Windows Kits\\10\\bin\\10.0.22000.0\\x64\\signtool.exe", program_files),
        format!("{}\\Windows Kits\\10\\bin\\10.0.19041.0\\x64\\signtool.exe", program_files),
        format!("{}\\Windows Kits\\10\\bin\\x64\\signtool.exe", program_files),
        format!("{}\\Windows Kits\\8.1\\bin\\x64\\signtool.exe", program_files),
    ];
    
    for path in &sdk_paths {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    
    // Try PATH
    if Command::new("signtool").arg("/?").output().is_ok() {
        return Some(std::path::PathBuf::from("signtool"));
    }
    
    None
}

/// Sign macOS executable/app bundle using codesign
/// Sign a macOS app bundle with a Developer ID Application identity.
///
/// The bundle is signed as a whole so `Resources/` and `Info.plist` are sealed.
/// `--deep` is intentionally avoided: Apple deprecated it, and the bundle's only
/// nested code is the main executable.
fn sign_macos(app_path: &Path, config: &SigningConfig) -> Result<(), String> {
    let identity = config.macos_identity.as_ref()
        .ok_or("No macOS signing identity configured. Set POLY_MACOS_IDENTITY or [signing.macos] identity in poly.toml")?;

    let mut cmd = Command::new("codesign");
    cmd.arg("--force")
        .arg("--options").arg("runtime")  // Hardened runtime for notarization
        .arg("--timestamp")
        .arg("--sign").arg(identity);

    // Add entitlements if configured
    if let Some(ref entitlements) = config.macos_entitlements {
        cmd.arg("--entitlements").arg(entitlements);
    }

    cmd.arg(app_path);

    let output = cmd.output()
        .map_err(|e| format!("Failed to run codesign: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!("codesign failed: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

/// Sign a `.pkg` with a `Developer ID Installer` identity.
///
/// `productsign` is the only way to sign a flat package, and it needs an
/// Installer certificate — an Application certificate is rejected. Callers must
/// only reach this when such an identity is configured.
fn sign_macos_pkg(pkg_path: &Path, config: &SigningConfig) -> Result<(), String> {
    let identity = config.macos_installer_identity.as_ref()
        .ok_or("No macOS installer identity configured. Set POLY_MACOS_INSTALLER_IDENTITY or [signing.macos] installer_identity in poly.toml")?;

    let signed_path = pkg_path.with_extension("signed.pkg");
    let _ = fs::remove_file(&signed_path);

    let output = Command::new("productsign")
        .arg("--sign").arg(identity)
        .arg(pkg_path)
        .arg(&signed_path)
        .output()
        .map_err(|e| format!("Failed to run productsign: {e}"))?;

    if !output.status.success() {
        let _ = fs::remove_file(&signed_path);
        return Err(format!(
            "productsign failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    fs::rename(&signed_path, pkg_path)
        .map_err(|e| format!("Could not replace the unsigned package: {e}"))
}

/// Notarize macOS app bundle with Apple
#[allow(dead_code)]
pub fn notarize_macos(app_path: &Path, config: &SigningConfig) -> Result<(), String> {
    let apple_id = config.macos_notarize_apple_id.as_ref()
        .ok_or("No Apple ID for notarization. Set POLY_MACOS_APPLE_ID")?;
    let password = config.macos_notarize_password.as_ref()
        .ok_or("No app-specific password for notarization. Set POLY_MACOS_APP_PASSWORD")?;
    let team_id = config.macos_notarize_team_id.as_ref()
        .ok_or("No Team ID for notarization. Set POLY_MACOS_TEAM_ID")?;
    
    // Create a zip for notarization
    let zip_path = app_path.with_extension("zip");
    let status = Command::new("ditto")
        .args(["-c", "-k", "--keepParent"])
        .arg(app_path)
        .arg(&zip_path)
        .status()
        .map_err(|e| format!("Failed to create zip: {}", e))?;
    
    if !status.success() {
        return Err("Failed to create zip for notarization".to_string());
    }
    
    // Submit for notarization
    let output = Command::new("xcrun")
        .args(["notarytool", "submit"])
        .arg(&zip_path)
        .arg("--apple-id").arg(apple_id)
        .arg("--password").arg(password)
        .arg("--team-id").arg(team_id)
        .arg("--wait")
        .output()
        .map_err(|e| format!("Failed to run notarytool: {}", e))?;
    
    // Clean up zip
    fs::remove_file(&zip_path).ok();
    
    if output.status.success() {
        // Staple the notarization ticket
        let staple_status = Command::new("xcrun")
            .args(["stapler", "staple"])
            .arg(app_path)
            .status()
            .map_err(|e| format!("Failed to staple: {}", e))?;
        
        if staple_status.success() {
            Ok(())
        } else {
            Err("Failed to staple notarization ticket".to_string())
        }
    } else {
        Err(format!("Notarization failed: {}", String::from_utf8_lossy(&output.stderr)))
    }
}

/// Create Windows installer (portable zip for now)
#[allow(unused_variables)]
fn create_windows_installer(
    dist_dir: &Path,
    app_name: &str,
    app_version: &str,
) -> Result<std::path::PathBuf, String> {
    let zip_name = format!("{}-{}-windows-x64.zip", app_name, app_version);
    let zip_path = dist_dir.parent().unwrap_or(dist_dir).join(&zip_name);
    
    // Create zip file
    #[cfg(feature = "native")]
    {
        let file = fs::File::create(&zip_path)
            .map_err(|e| format!("Failed to create zip: {}", e))?;
        let mut zip = zip::ZipWriter::new(file);
        
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        
        // Add all files from dist_dir
        add_dir_to_zip(&mut zip, dist_dir, "", &options)?;
        
        zip.finish().map_err(|e| format!("Failed to finish zip: {}", e))?;
        
        Ok(zip_path)
    }
    
    #[cfg(not(feature = "native"))]
    {
        Err("Installer creation requires native feature".to_string())
    }
}

#[cfg(feature = "native")]
fn add_dir_to_zip<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    dir: &Path,
    prefix: &str,
    options: &zip::write::FileOptions,
) -> Result<(), String> {
    use std::io::Read;
    
    for entry in fs::read_dir(dir).map_err(|e| format!("Read dir error: {}", e))? {
        let entry = entry.map_err(|e| format!("Entry error: {}", e))?;
        let path = entry.path();
        let name = format!(
            "{}{}",
            if prefix.is_empty() { "".to_string() } else { format!("{}/", prefix) },
            entry.file_name().to_string_lossy()
        );
        
        if path.is_dir() {
            zip.add_directory(&name, *options).ok();
            add_dir_to_zip(zip, &path, &name, options)?;
        } else {
            zip.start_file(&name, *options)
                .map_err(|e| format!("Zip start file error: {}", e))?;
            let mut file = fs::File::open(&path)
                .map_err(|e| format!("Open file error: {}", e))?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .map_err(|e| format!("Read file error: {}", e))?;
            zip.write_all(&buffer)
                .map_err(|e| format!("Write zip error: {}", e))?;
        }
    }
    Ok(())
}

/// Path to an app's `bundle/` data directory given the directory holding the
/// executable.
///
/// A macOS `.app` launches `Contents/MacOS/<exe>`, whose data bundle lives two
/// levels up in `Contents/Resources/bundle`. Every other layout keeps the
/// bundle beside the executable.
pub fn bundle_source_dir(exe_dir: &Path) -> std::path::PathBuf {
    let macos_bundle = bundle_project_dir(exe_dir).map(|resources| resources.join("bundle"));
    macos_bundle.unwrap_or_else(|| exe_dir.join("bundle"))
}

/// Directory that `run_native_app` should treat as the app root for a macOS
/// `.app`, or `None` when the executable is not inside one.
pub fn bundle_project_dir(exe_dir: &Path) -> Option<std::path::PathBuf> {
    // `exe_dir` is Contents/MacOS, so its parent is Contents.
    let contents = exe_dir.parent()?;
    let resources = contents.join("Resources");
    let is_app = contents.file_name()? == "Contents" && resources.join("bundle").is_dir();
    is_app.then_some(resources)
}

/// Build a macOS `.icns` icon from a source image.
///
#[cfg(feature = "native")]
/// The artwork is composited onto `background` (when given) and inset inside a
/// rounded "squircle" the way macOS app icons are drawn, so a transparent logo
/// still yields a solid tile instead of a floating cut-out. PNG payloads are
/// written directly because that is the format `iconutil` produces and
/// `NSImage` accepts.
fn create_macos_icns(
    source: &Path,
    output: &Path,
    background: Option<(u8, u8, u8, u8)>,
) -> Result<(), String> {
    let artwork = image::ImageReader::open(source)
        .map_err(|error| format!("Could not open icon {}: {error}", source.display()))?
        .decode()
        .map_err(|error| format!("Could not decode icon {}: {error}", source.display()))?;

    // Draw at the largest size the iconset needs, then downscale once per slot.
    const CANVAS: u32 = 1024;
    let base = compose_icon_tile(&artwork, CANVAS, background);

    // (pixel size, element type) for the standard iconset:
    // 16@1x 16@2x 32@1x 32@2x 128@1x 128@2x 256@1x 256@2x 512@1x 512@2x
    const ICON_ELEMENTS: [(u32, &str); 10] = [
        (16, "icp4"),
        (32, "ic11"),
        (32, "icp5"),
        (64, "ic12"),
        (128, "icp6"),
        (256, "ic13"),
        (256, "icp7"),
        (512, "ic14"),
        (512, "ic08"),
        (1024, "ic09"),
    ];

    let mut payloads: Vec<(&str, Vec<u8>)> = Vec::new();
    for (size, element) in ICON_ELEMENTS {
        let resized = image::imageops::resize(
            &base,
            size,
            size,
            image::imageops::FilterType::Lanczos3,
        );
        let mut png_bytes: Vec<u8> = Vec::new();
        resized
            .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
            .map_err(|error| format!("Could not encode the {size}px icon: {error}"))?;
        payloads.push((element, png_bytes));
    }

    // An .icns file is: magic "icns", total length, then typed PNG chunks.
    // Chunk order is not significant, so only the payloads are assembled here
    // and the sizes are written as big-endian lengths.
    let mut body: Vec<u8> = Vec::new();
    for (element, png_bytes) in &payloads {
        let chunk_len = (png_bytes.len() + 8) as u32;
        body.extend_from_slice(element.as_bytes());
        body.extend_from_slice(&chunk_len.to_be_bytes());
        body.extend_from_slice(png_bytes);
    }

    let total_len = (body.len() + 8) as u32;
    let mut icns: Vec<u8> = Vec::with_capacity(total_len as usize);
    icns.extend_from_slice(b"icns");
    icns.extend_from_slice(&total_len.to_be_bytes());
    icns.extend_from_slice(&body);

    fs::write(output, icns)
        .map_err(|error| format!("Could not write {}: {error}", output.display()))
}

#[cfg(feature = "native")]
/// Compose artwork into a square app-icon tile.
///
/// macOS icons are drawn on a rounded tile that occupies roughly 80% of the
/// canvas. The artwork is fitted inside that tile on `background`, which lets a
/// transparent or irregular logo sit on a solid, correctly proportioned icon.
/// Fitting preserves the artwork's aspect ratio, so the motif is positioned
/// from its own scaled dimensions to keep it centred on non-square images.
fn compose_icon_tile(
    artwork: &image::DynamicImage,
    canvas: u32,
    background: Option<(u8, u8, u8, u8)>,
) -> image::RgbaImage {
    use image::imageops::FilterType;
    use image::Rgba;

    let mut tile = image::RgbaImage::new(canvas, canvas);

    // Solid tile: fill a rounded square with padding, like other macOS apps.
    let inner_extent = match background {
        Some((r, g, b, a)) => {
            let inset = (canvas as f32 * 0.10).round() as u32;
            let extent = canvas - inset * 2;
            let radius = (extent as f32 * 0.225).round();

            for y in 0..canvas {
                for x in 0..canvas {
                    let inside_bounds = x >= inset && x < inset + extent && y >= inset && y < inset + extent;
                    if !inside_bounds {
                        continue;
                    }
                    // Rounded corners: skip pixels outside the corner arc.
                    let cx = x as f32 + 0.5;
                    let cy = y as f32 + 0.5;
                    let nearest_x = cx.clamp(inset as f32 + radius, (inset + extent) as f32 - radius);
                    let nearest_y = cy.clamp(inset as f32 + radius, (inset + extent) as f32 - radius);
                    let distance = ((cx - nearest_x).powi(2) + (cy - nearest_y).powi(2)).sqrt();
                    if distance <= radius {
                        tile.put_pixel(x, y, Rgba([r, g, b, a]));
                    }
                }
            }
            extent
        }
        None => canvas,
    };

    // Fit the artwork inside the tile, then centre it using the size it
    // actually scaled to rather than the box that was requested.
    let art_box = (inner_extent as f32 * if background.is_some() { 0.78 } else { 0.80 }).round() as u32;
    let scaled = artwork.resize(art_box, art_box, FilterType::Lanczos3).to_rgba8();
    let (scaled_w, scaled_h) = scaled.dimensions();
    let offset_x = (canvas.saturating_sub(scaled_w)) / 2;
    let offset_y = (canvas.saturating_sub(scaled_h)) / 2;
    image::imageops::overlay(&mut tile, &scaled, offset_x as i64, offset_y as i64);

    tile
}

/// Create the app's macOS icon and return its `.icns` path, if a source icon exists.
///
/// Without the `native` feature the `image` crate is unavailable, so no icon is
/// generated and the app bundle falls back to the system default.
#[cfg(feature = "native")]
fn create_macos_app_icon(
    project_path: &Path,
    app_name: &str,
    icon_source: Option<&str>,
    icon_background: Option<&str>,
) -> Option<std::path::PathBuf> {
    let source = icon_source
        .map(|path| project_path.join(path))
        .or_else(|| Some(project_path.join("assets/icon.png")))
        .filter(|path| path.is_file());

    let source = match source {
        Some(path) => path,
        None => {
            eprintln!("  {}warning{}: No app icon found; the macOS app will use the default icon.", YELLOW, RESET);
            return None;
        }
    };

    let background = icon_background.and_then(|value| {
        poly::config::parse_color(value).or_else(|| {
            eprintln!(
                "  {}warning{}: Could not parse [build] icon_background '{}'; using a transparent icon.",
                YELLOW, RESET, value
            );
            None
        })
    });

    let icns_path = std::env::temp_dir()
        .join(format!("poly-{}-{}.icns", slugify(app_name), std::process::id()));
    match create_macos_icns(&source, &icns_path, background) {
        Ok(()) => Some(icns_path),
        Err(error) => {
            eprintln!("  {}warning{}: Could not build the macOS app icon: {}", YELLOW, RESET, error);
            let _ = fs::remove_file(&icns_path);
            None
        }
    }
}

/// Without the `native` feature the image decoder is unavailable, so the macOS
/// bundle is built without a custom icon rather than failing the build.
#[cfg(not(feature = "native"))]
fn create_macos_app_icon(
    _project_path: &Path,
    _app_name: &str,
    _icon_source: Option<&str>,
    _icon_background: Option<&str>,
) -> Option<std::path::PathBuf> {
    eprintln!(
        "  {}warning{}: App icons need the `native` feature; the macOS app will use the default icon.",
        YELLOW, RESET
    );
    None
}

/// Lowercase, ASCII-only identifier fragment for paths and bundle identifiers.
fn slugify(value: &str) -> String {
    let slug: String = value
        .to_lowercase()
        .chars()
        .map(|character| if character.is_ascii_alphanumeric() { character } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() { "app".to_string() } else { slug }
}

/// Create macOS .app bundle
fn create_macos_app_bundle(
    dist_dir: &Path,
    project_path: &Path,
    app_name: &str,
    app_version: &str,
    bundle_dir: Option<&Path>,
) -> Result<std::path::PathBuf, String> {
    let app_bundle = dist_dir.parent().unwrap_or(dist_dir).join(format!("{}.app", app_name));
    let contents = app_bundle.join("Contents");
    let macos = contents.join("MacOS");
    let resources = contents.join("Resources");

    fs::create_dir_all(&macos)
        .map_err(|error| format!("Could not create {}: {error}", macos.display()))?;
    fs::create_dir_all(&resources)
        .map_err(|error| format!("Could not create {}: {error}", resources.display()))?;

    // Copy executable
    let exe_name = app_name.to_string();
    let src_exe = dist_dir.join(&exe_name);
    if !src_exe.exists() {
        return Err(format!("Expected a built executable at {}.", src_exe.display()));
    }
    fs::copy(&src_exe, macos.join(&exe_name))
        .map_err(|error| format!("Could not copy {} into the app bundle: {error}", src_exe.display()))?;

    // `fs::copy` preserves file contents but not the executable bit, and an app
    // whose main binary is not executable will not launch.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let executable = macos.join(&exe_name);
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("Could not mark {} as executable: {error}", executable.display()))?;
    }

    // Stage the app data at Contents/Resources/bundle, matching the location
    // `bundle_source_dir` reads at runtime.
    let staged_bundle = resources.join("bundle");
    if staged_bundle.exists() {
        fs::remove_dir_all(&staged_bundle)
            .map_err(|error| format!("Could not replace {}: {error}", staged_bundle.display()))?;
    }
    if let Some(bundle_dir) = bundle_dir.filter(|dir| dir.is_dir()) {
        copy_dir_recursive(bundle_dir, &staged_bundle)
            .map_err(|error| format!("Could not copy the app data bundle: {error}"))?;
    }

    // macOS takes the Dock/Finder icon from CFBundleIconFile inside the bundle.
    // `icon_path` supplies the artwork and `icon_background` supplies the tile
    // colour behind it, so the two can differ.
    let build_settings = fs::read_to_string(project_path.join("poly.toml"))
        .map(|content| {
            (
                extract_toml_value(&content, "icon_path"),
                extract_toml_value(&content, "icon_background"),
            )
        })
        .unwrap_or((None, None));
    let (icon_source, icon_background) = build_settings;
    let icon_entry = create_macos_app_icon(
        project_path,
        app_name,
        icon_source.as_deref(),
        icon_background.as_deref(),
    )
        .and_then(|temporary| {
            let target = resources.join("icon.icns");
            match fs::copy(&temporary, &target) {
                Ok(_) => {
                    let _ = fs::remove_file(&temporary);
                    Some("icon.icns".to_string())
                }
                Err(error) => {
                    let _ = fs::remove_file(&temporary);
                    eprintln!("  {}warning{}: Could not copy the app icon: {}", YELLOW, RESET, error);
                    None
                }
            }
        });

    // A bundle identifier is reverse-DNS and may not contain spaces, so derive
    // it from the app name rather than embedding the display text verbatim.
    let bundle_id = format!("com.poly.{}", slugify(app_name));
    let icon_plist = match &icon_entry {
        Some(entry) => format!("\n    <key>CFBundleIconFile</key>\n    <string>{entry}</string>"),
        None => String::new(),
    };

    // Create Info.plist
    let info_plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>{exe_name}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_id}</string>
    <key>CFBundleName</key>
    <string>{app_name}</string>
    <key>CFBundleDisplayName</key>
    <string>{app_name}</string>
    <key>CFBundleVersion</key>
    <string>{app_version}</string>
    <key>CFBundleShortVersionString</key>
    <string>{app_version}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>{icon_plist}
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>"#);

    fs::write(contents.join("Info.plist"), info_plist)
        .map_err(|error| format!("Could not create the app's Info.plist: {error}"))?;

    Ok(app_bundle)
}

/// Packaging format requested for `--installer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallerFormat {
    Dmg,
    Pkg,
}

impl InstallerFormat {
    /// Resolve a `--installer-format` value, or pick the platform default.
    fn parse(value: Option<&str>, platform: Platform) -> Result<Self, String> {
        let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
            return match platform {
                Platform::MacOS => Ok(Self::Dmg),
                _ => Ok(Self::Dmg),
            };
        };

        match value.to_ascii_lowercase().as_str() {
            "dmg" | "disk-image" | "diskimage" => Ok(Self::Dmg),
            "pkg" | "installer-pkg" | "productbuild" => Ok(Self::Pkg),
            "exe" | "appimage" | "tar.gz" | "tarball" => Err(format!(
                "'{value}' is not a macOS installer format. Use 'dmg' or 'pkg'."
            )),
            other => Err(format!("Unknown installer format '{other}'. Use 'dmg' or 'pkg'.")),
        }
    }
}

/// Wrap the assembled `.app` in a `.dmg` disk image.
///
/// `hdiutil -srcfolder` copies the folder's *contents*, so the app is staged
/// under a temporary directory to keep `X.app` at the root of the mounted
/// volume. The image also gets an `/Applications` symlink so it supports the
/// usual drag-to-install gesture.
fn create_macos_dmg(
    dist_dir: &Path,
    app_name: &str,
    app_version: &str,
) -> Result<std::path::PathBuf, String> {
    let app_bundle = dist_dir.parent().unwrap_or(dist_dir).join(format!("{}.app", app_name));
    if !app_bundle.is_dir() {
        return Err(format!(
            "Expected the app bundle at {} before packaging. Build the app first.",
            app_bundle.display()
        ));
    }

    let dmg_name = format!("{}-{}-macos.dmg", slugify(app_name), app_version);
    let dmg_path = dist_dir.parent().unwrap_or(dist_dir).join(&dmg_name);
    let _ = fs::remove_file(&dmg_path);

    // Stage the volume contents: the app plus an Applications shortcut.
    let staging = std::env::temp_dir().join(format!(
        "poly-dmg-{}-{}",
        slugify(app_name),
        std::process::id()
    ));
    if staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    fs::create_dir_all(&staging)
        .map_err(|error| format!("Could not create the disk image staging folder: {error}"))?;

    let staged_app = staging.join(format!("{}.app", app_name));
    let staged = copy_dir_recursive(&app_bundle, &staged_app)
        .map_err(|error| format!("Could not stage the app for the disk image: {error}"));
    if let Err(error) = staged {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    // Disk images are only created on macOS, where the Unix symlink API is
    // available. Keep this macOS-only operation out of Windows builds.
    #[cfg(unix)]
    let _ = std::os::unix::fs::symlink("/Applications", staging.join("Applications"));

    let output = Command::new("hdiutil")
        .arg("create")
        .arg("-volname").arg(app_name)
        .arg("-srcfolder").arg(&staging)
        .arg("-ov")
        .arg("-format").arg("UDZO")
        .arg(&dmg_path)
        .output();

    let _ = fs::remove_dir_all(&staging);

    let output = output.map_err(|error| format!("Could not run hdiutil: {error}"))?;
    if output.status.success() {
        Ok(dmg_path)
    } else {
        let _ = fs::remove_file(&dmg_path);
        Err(format!("hdiutil failed: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

/// Wrap the assembled `.app` in a `.pkg` installer package.
///
/// `pkgbuild` installs the app into `/Applications`. Signing is applied by the
/// caller when `--sign` is passed, because a product signature needs the same
/// identity used for the app itself.
fn create_macos_pkg(
    dist_dir: &Path,
    app_name: &str,
    app_version: &str,
    app_bundle: &Path,
) -> Result<std::path::PathBuf, String> {
    if !app_bundle.is_dir() {
        return Err(format!(
            "Expected the app bundle at {} before packaging. Build the app first.",
            app_bundle.display()
        ));
    }

    let pkg_name = format!("{}-{}-macos.pkg", slugify(app_name), app_version);
    let pkg_path = dist_dir.parent().unwrap_or(dist_dir).join(&pkg_name);
    let _ = fs::remove_file(&pkg_path);

    // pkgbuild requires a component root whose layout mirrors the install
    // destination, so stage the app under Applications/ before building.
    let staging = std::env::temp_dir().join(format!(
        "poly-pkg-{}-{}",
        slugify(app_name),
        std::process::id()
    ));
    if staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    let staged_app = staging.join("Applications").join(format!("{}.app", app_name));
    if let Err(error) = copy_dir_recursive(app_bundle, &staged_app) {
        let _ = fs::remove_dir_all(&staging);
        return Err(format!("Could not stage the app for the installer: {error}"));
    }

    // `pkgbuild` only records what the archive contains; it cannot guarantee the
    // main binary keeps its executable bit on the target, and an unsigned app
    // copied to another Mac may need an ad-hoc signature to launch. A postinstall
    // script repairs both on the machine the package is installed on.
    let scripts_dir = staging.join("scripts");
    if let Err(error) = write_pkg_postinstall(&scripts_dir, app_name) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    let output = Command::new("pkgbuild")
        .arg("--root").arg(&staging)
        .arg("--scripts").arg(&scripts_dir)
        .arg("--identifier").arg(format!("com.poly.{}", slugify(app_name)))
        .arg("--version").arg(app_version)
        .arg("--install-location").arg("/")
        .arg(&pkg_path)
        .output();

    let _ = fs::remove_dir_all(&staging);

    let output = output.map_err(|error| format!("Could not run pkgbuild: {error}"))?;
    if output.status.success() {
        Ok(pkg_path)
    } else {
        let _ = fs::remove_file(&pkg_path);
        Err(format!("pkgbuild failed: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

/// Write the `.pkg` postinstall script that repairs the installed app.
///
/// `pkgbuild --scripts` needs a directory containing an executable `postinstall`.
/// The script runs as root on the installing machine, so it can:
///   1. restore the executable bit on the app's main binary, and
///   2. ad-hoc sign the bundle when no Developer ID signature is present,
///      which is what lets an unsigned build launch without being blocked.
///
/// It never overwrites an existing valid signature: `codesign --verify` runs
/// first, and a signed app is left untouched so Developer ID signatures survive.
fn write_pkg_postinstall(scripts_dir: &Path, app_name: &str) -> Result<(), String> {
    fs::create_dir_all(scripts_dir)
        .map_err(|error| format!("Could not create the installer scripts folder: {error}"))?;

    // The bundle name is interpolated into a shell script, so keep it to a form
    // that cannot break out of the quoting below.
    let bundle_name = format!("{}.app", app_name).replace('"', "").replace('$', "").replace('`', "");
    let executable_name = app_name.replace('"', "").replace('$', "").replace('`', "");

    let script = format!(
        r#"#!/bin/sh
# Poly installer postinstall.
# Repairs the installed app so it runs from this machine.
set -u

APP="/Applications/{bundle_name}"
EXECUTABLE="$APP/Contents/MacOS/{executable_name}"

if [ ! -d "$APP" ]; then
  echo "poly: expected the app at $APP" >&2
  exit 0
fi

# 1. The main binary must be executable.
if [ -f "$EXECUTABLE" ]; then
  chmod 755 "$EXECUTABLE" || echo "poly: could not mark $EXECUTABLE executable" >&2
else
  echo "poly: missing app executable at $EXECUTABLE" >&2
fi

# 2. Ad-hoc sign only when the bundle is not already validly signed.
#    A Developer ID signed build passes --verify and is left as-is.
if ! /usr/bin/codesign --verify "$APP" >/dev/null 2>&1; then
  if ! /usr/bin/codesign --force --deep --sign - "$APP" >/dev/null 2>&1; then
    echo "poly: ad-hoc signing failed; the app may be blocked by Gatekeeper" >&2
  fi
fi

exit 0
"#,
        bundle_name = bundle_name,
        executable_name = executable_name,
    );

    let postinstall = scripts_dir.join("postinstall");
    fs::write(&postinstall, script)
        .map_err(|error| format!("Could not write the installer postinstall script: {error}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&postinstall, fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("Could not mark the postinstall script executable: {error}"))?;
    }

    Ok(())
}

/// Create Linux AppImage (simplified - creates tarball for now)
fn create_linux_appimage(
    dist_dir: &Path,
    app_name: &str,
    app_version: &str,
) -> Result<std::path::PathBuf, String> {
    // For now, create a tarball. Full AppImage requires appimagetool
    let tar_name = format!("{}-{}-linux-x64.tar.gz", app_name, app_version);
    let tar_path = dist_dir.parent().unwrap_or(dist_dir).join(&tar_name);
    
    // Use tar command if available
    let status = Command::new("tar")
        .args(["-czf", &tar_path.to_string_lossy(), "-C", &dist_dir.to_string_lossy(), "."])
        .status();
    
    match status {
        Ok(s) if s.success() => Ok(tar_path),
        _ => Err("tar command not available. Install tar to create Linux packages.".to_string()),
    }
}

/// Generate GitHub Actions workflow for cross-platform builds
pub fn generate_ci_workflow(project_path: &Path) -> Result<(), String> {
    let workflows_dir = project_path.join(".github/workflows");
    fs::create_dir_all(&workflows_dir)
        .map_err(|e| format!("Failed to create workflows dir: {}", e))?;
    
    let workflow = r#"name: Build

on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            name: windows-x64
            ext: .exe
          - os: macos-latest
            target: x86_64-apple-darwin
            name: macos-x64
            ext: ""
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            name: linux-x64
            ext: ""
    
    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: ${{ matrix.target }}
      
      - name: Install Poly
        run: cargo install --git https://github.com/OpenChatGit/Poly.git poly-framework --features native
      
      - name: Build
        run: poly build --release
      
      - name: Package
        shell: bash
        run: |
          cd dist/${{ matrix.name }}
          if [ "${{ matrix.os }}" = "windows-latest" ]; then
            7z a -tzip ../../${{ github.event.repository.name }}-${{ matrix.name }}.zip .
          else
            tar -czf ../../${{ github.event.repository.name }}-${{ matrix.name }}.tar.gz .
          fi
      
      - name: Upload Artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.name }}
          path: |
            *.zip
            *.tar.gz

  release:
    needs: build
    runs-on: ubuntu-latest
    if: startsWith(github.ref, 'refs/tags/')
    
    steps:
      - name: Download Artifacts
        uses: actions/download-artifact@v4
      
      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            windows-x64/*.zip
            macos-x64/*.tar.gz
            linux-x64/*.tar.gz
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
"#;
    
    fs::write(workflows_dir.join("build.yml"), workflow)
        .map_err(|e| format!("Failed to write workflow: {}", e))?;
    
    println!("  {}✓{} Created .github/workflows/build.yml", GREEN, RESET);
    println!();
    println!("  {}To build for all platforms:{}", DIM, RESET);
    println!("  1. Push to GitHub");
    println!("  2. Create a tag: git tag v1.0.0 && git push --tags");
    println!("  3. GitHub Actions will build for Windows, macOS, and Linux");
    
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("Create dir error: {}", e))?;
    
    for entry in fs::read_dir(src).map_err(|e| format!("Read dir error: {}", e))? {
        let entry = entry.map_err(|e| format!("Entry error: {}", e))?;
        let dest = dst.join(entry.file_name());
        
        if entry.path().is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), &dest)
                .map_err(|e| format!("Copy error: {}", e))?;
        }
    }
    Ok(())
}

fn extract_toml_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(&format!("{} =", key)) || line.starts_with(&format!("{}=", key)) {
            if let Some(value) = line.split('=').nth(1) {
                return Some(value.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

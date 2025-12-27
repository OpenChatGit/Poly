//! Cross-Platform Build System for Poly
//! Builds native executables with bundled web assets

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

// ANSI colors
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
#[allow(dead_code)]
const RED: &str = "\x1b[31m";
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
    fs::create_dir_all(&dist_dir).ok();
    
    // Step 1: Bundle web assets
    print!("  {}Bundling assets...{}", DIM, RESET);
    io::stdout().flush().ok();
    
    let bundle_dir = dist_dir.join("bundle");
    bundle_web_assets(project_path, &bundle_dir)?;
    
    println!("\r  {}✓{} Bundled assets          ", GREEN, RESET);
    
    // Step 2: Create launcher/executable
    print!("  {}Creating executable...{}", DIM, RESET);
    io::stdout().flush().ok();
    
    let exe_name = format!("{}{}", app_name, config.platform.exe_extension());
    let exe_path = dist_dir.join(&exe_name);
    
    create_launcher(project_path, &exe_path, &bundle_dir, config)?;
    
    let size = fs::metadata(&exe_path).map(|m| m.len()).unwrap_or(0);
    println!("\r  {}✓{} Created {} ({:.1} MB)          ", GREEN, RESET, exe_name, size as f64 / 1_000_000.0);
    
    // Step 3: Sign executable (optional)
    if config.sign {
        print!("  {}Signing executable...{}", DIM, RESET);
        io::stdout().flush().ok();
        
        let signing_config = load_signing_config(&config.project_path);
        match sign_executable(&exe_path, &signing_config) {
            Ok(()) => {
                println!("\r  {}✓{} Signed executable          ", GREEN, RESET);
            }
            Err(e) => {
                println!("\r  {}!{} Signing failed: {}          ", YELLOW, RESET, e);
            }
        }
    }
    
    // Step 4: Create installer (optional)
    if config.installer {
        print!("  {}Creating installer...{}", DIM, RESET);
        io::stdout().flush().ok();
        
        match create_installer(&dist_dir, &app_name, &app_version, config.platform) {
            Ok(installer_path) => {
                println!("\r  {}✓{} Created installer: {}          ", GREEN, RESET, 
                    installer_path.file_name().unwrap_or_default().to_string_lossy());
            }
            Err(e) => {
                println!("\r  {}!{} Installer skipped: {}          ", YELLOW, RESET, e);
            }
        }
    }
    
    let elapsed = start.elapsed();
    println!();
    println!("  {}done{} in {:.1}s", GREEN, RESET, elapsed.as_secs_f64());
    println!();
    println!("  {}Output:{} {}", DIM, RESET, dist_dir.display());
    
    Ok(BuildResult {
        output_path: exe_path,
        size_bytes: size,
    })
}

/// Bundle web assets into a directory
fn bundle_web_assets(project_path: &Path, bundle_dir: &Path) -> Result<(), String> {
    // Clean and create bundle directory
    if bundle_dir.exists() {
        fs::remove_dir_all(bundle_dir).ok();
    }
    fs::create_dir_all(bundle_dir).map_err(|e| format!("Failed to create bundle dir: {}", e))?;
    
    // Copy web directory
    let web_dir = project_path.join("web");
    if web_dir.exists() {
        copy_dir_recursive(&web_dir, &bundle_dir.join("web"))?;
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
        fs::copy(&poly_toml, bundle_dir.join("poly.toml")).ok();
    }
    
    // Copy main.poly if exists
    let main_poly = project_path.join("src/main.poly");
    if main_poly.exists() {
        fs::create_dir_all(bundle_dir.join("src")).ok();
        fs::copy(&main_poly, bundle_dir.join("src/main.poly")).ok();
    }
    
    Ok(())
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
            .arg("-p").arg("poly")
            .arg("--features").arg("native,gui");  // Add gui feature for no-console
        
        if config.release {
            cmd.arg("--release");
        }
        
        let output = cmd.output()
            .map_err(|e| format!("Failed to run cargo build: {}", e))?;
        
        if !output.status.success() {
            // If gui build fails, fall back to copying current exe
            let poly_exe = std::env::current_exe()
                .map_err(|e| format!("Failed to find poly executable: {}", e))?;
            fs::copy(&poly_exe, exe_path)
                .map_err(|e| format!("Failed to copy executable: {}", e))?;
        } else {
            // Copy the built GUI executable
            let target_dir = poly_crate.join("target");
            let profile = if config.release { "release" } else { "debug" };
            let built_exe = target_dir.join(profile).join(format!("poly{}", Platform::current().exe_extension()));
            
            if built_exe.exists() {
                fs::copy(&built_exe, exe_path)
                    .map_err(|e| format!("Failed to copy built executable: {}", e))?;
            } else {
                // Fallback: copy current exe
                let poly_exe = std::env::current_exe()
                    .map_err(|e| format!("Failed to find poly executable: {}", e))?;
                fs::copy(&poly_exe, exe_path)
                    .map_err(|e| format!("Failed to copy executable: {}", e))?;
            }
        }
        
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
    let poly_crate = cwd.join("poly");
    if poly_crate.join("Cargo.toml").exists() {
        return Ok(poly_crate);
    }
    
    // Check if we're in the poly crate itself
    if cwd.join("Cargo.toml").exists() {
        let content = fs::read_to_string(cwd.join("Cargo.toml")).unwrap_or_default();
        if content.contains("name = \"poly\"") {
            return Ok(cwd);
        }
    }
    
    Err("Could not find poly crate. Make sure you're in the Poly workspace.".to_string())
}

/// Create platform-specific installer
fn create_installer(
    dist_dir: &Path,
    app_name: &str,
    app_version: &str,
    _platform: Platform,
) -> Result<std::path::PathBuf, String> {
    match Platform::current() {
        Platform::Windows => create_windows_installer(dist_dir, app_name, app_version),
        Platform::MacOS => create_macos_app_bundle(dist_dir, app_name, app_version),
        Platform::Linux => create_linux_appimage(dist_dir, app_name, app_version),
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
            let mut in_signing_section = false;
            let mut in_windows_section = false;
            let mut in_macos_section = false;
            
            for line in content.lines() {
                let line = line.trim();
                
                if line == "[signing]" {
                    in_signing_section = true;
                    in_windows_section = false;
                    in_macos_section = false;
                } else if line == "[signing.windows]" {
                    in_signing_section = false;
                    in_windows_section = true;
                    in_macos_section = false;
                } else if line == "[signing.macos]" {
                    in_signing_section = false;
                    in_windows_section = false;
                    in_macos_section = true;
                } else if line.starts_with('[') {
                    in_signing_section = false;
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
            // Linux doesn't have a standard code signing mechanism
            // GPG signing could be added later
            println!("  {}!{} Linux code signing not implemented (GPG signing planned)", YELLOW, RESET);
            Ok(())
        }
        Platform::Current => unreachable!(),
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
fn sign_macos(exe_path: &Path, config: &SigningConfig) -> Result<(), String> {
    let identity = config.macos_identity.as_ref()
        .ok_or("No macOS signing identity configured. Set POLY_MACOS_IDENTITY or [signing.macos] identity in poly.toml")?;
    
    let mut cmd = Command::new("codesign");
    cmd.arg("--force")
        .arg("--options").arg("runtime")  // Hardened runtime for notarization
        .arg("--sign").arg(identity);
    
    // Add entitlements if configured
    if let Some(ref entitlements) = config.macos_entitlements {
        cmd.arg("--entitlements").arg(entitlements);
    }
    
    cmd.arg(exe_path);
    
    let output = cmd.output()
        .map_err(|e| format!("Failed to run codesign: {}", e))?;
    
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("codesign failed: {}", String::from_utf8_lossy(&output.stderr)))
    }
}

/// Notarize macOS app bundle with Apple
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
    }
    
    #[cfg(not(feature = "native"))]
    {
        return Err("Installer creation requires native feature".to_string());
    }
    
    Ok(zip_path)
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

/// Create macOS .app bundle
fn create_macos_app_bundle(
    dist_dir: &Path,
    app_name: &str,
    app_version: &str,
) -> Result<std::path::PathBuf, String> {
    let app_bundle = dist_dir.parent().unwrap_or(dist_dir).join(format!("{}.app", app_name));
    let contents = app_bundle.join("Contents");
    let macos = contents.join("MacOS");
    let resources = contents.join("Resources");
    
    fs::create_dir_all(&macos).ok();
    fs::create_dir_all(&resources).ok();
    
    // Copy executable
    let exe_name = format!("{}", app_name);
    let src_exe = dist_dir.join(&exe_name);
    if src_exe.exists() {
        fs::copy(&src_exe, macos.join(&exe_name)).ok();
    }
    
    // Copy bundle to Resources
    let bundle_dir = dist_dir.join("bundle");
    if bundle_dir.exists() {
        copy_dir_recursive(&bundle_dir, &resources.join("bundle")).ok();
    }
    
    // Create Info.plist
    let info_plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>{}</string>
    <key>CFBundleIdentifier</key>
    <string>com.poly.{}</string>
    <key>CFBundleName</key>
    <string>{}</string>
    <key>CFBundleVersion</key>
    <string>{}</string>
    <key>CFBundleShortVersionString</key>
    <string>{}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>"#, app_name, app_name.to_lowercase(), app_name, app_version, app_version);
    
    fs::write(contents.join("Info.plist"), info_plist).ok();
    
    Ok(app_bundle)
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
        run: cargo install --git https://github.com/OpenChatGit/Poly.git poly --features native
      
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
            fs::copy(&entry.path(), &dest)
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

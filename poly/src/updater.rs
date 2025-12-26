//! Auto-Updater for Poly Applications
//! 
//! Supports:
//! - GitHub Releases
//! - Custom update URLs
//! - Automatic background checks
//! - One-click updates

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Update configuration from poly.toml
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateConfig {
    /// Enable auto-update checks
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// Update source: "github" or custom URL
    #[serde(default)]
    pub source: UpdateSource,
    
    /// GitHub owner/repo (e.g., "user/my-app")
    #[serde(default)]
    pub github_repo: Option<String>,
    
    /// Custom update manifest URL
    #[serde(default)]
    pub update_url: Option<String>,
    
    /// Check interval in hours (default: 24)
    #[serde(default = "default_interval")]
    pub check_interval_hours: u32,
    
    /// Show update dialog automatically
    #[serde(default = "default_true")]
    pub auto_prompt: bool,
}

fn default_enabled() -> bool { false }
fn default_interval() -> u32 { 24 }
fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UpdateSource {
    #[default]
    Github,
    Custom,
}

/// Update manifest (returned from update server)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub platforms: Platforms,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Platforms {
    #[serde(rename = "windows-x86_64")]
    pub windows: Option<PlatformAsset>,
    #[serde(rename = "darwin-x86_64")]
    pub macos_intel: Option<PlatformAsset>,
    #[serde(rename = "darwin-aarch64")]
    pub macos_arm: Option<PlatformAsset>,
    #[serde(rename = "linux-x86_64")]
    pub linux: Option<PlatformAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformAsset {
    pub url: String,
    pub signature: Option<String>,
}

/// Update check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub download_url: Option<String>,
    pub release_notes: Option<String>,
    pub pub_date: Option<String>,
}

/// GitHub Release API response
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    published_at: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}


/// Check for updates from GitHub releases
#[cfg(feature = "native")]
pub fn check_github_updates(repo: &str, current_version: &str) -> Result<UpdateInfo, String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    
    let client = reqwest::blocking::Client::builder()
        .user_agent("Poly-Updater/1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    let response = client.get(&url)
        .send()
        .map_err(|e| format!("Failed to fetch releases: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("GitHub API error: {}", response.status()));
    }
    
    let release: GitHubRelease = response.json()
        .map_err(|e| format!("Failed to parse release: {}", e))?;
    
    // Parse versions (remove 'v' prefix if present)
    let latest = release.tag_name.trim_start_matches('v');
    let current = current_version.trim_start_matches('v');
    
    let update_available = is_newer_version(latest, current);
    
    // Find download URL for current platform
    let download_url = find_platform_asset(&release.assets);
    
    Ok(UpdateInfo {
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        update_available,
        download_url,
        release_notes: release.body,
        pub_date: release.published_at,
    })
}

/// Check for updates from custom URL
#[cfg(feature = "native")]
pub fn check_custom_updates(update_url: &str, current_version: &str) -> Result<UpdateInfo, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Poly-Updater/1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    let response = client.get(update_url)
        .send()
        .map_err(|e| format!("Failed to fetch update manifest: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Update server error: {}", response.status()));
    }
    
    let manifest: UpdateManifest = response.json()
        .map_err(|e| format!("Failed to parse manifest: {}", e))?;
    
    let latest = manifest.version.trim_start_matches('v');
    let current = current_version.trim_start_matches('v');
    let update_available = is_newer_version(latest, current);
    
    // Get platform-specific download URL
    let download_url = get_platform_url(&manifest.platforms);
    
    Ok(UpdateInfo {
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        update_available,
        download_url,
        release_notes: manifest.notes,
        pub_date: manifest.pub_date,
    })
}

/// Compare semantic versions
#[cfg(feature = "native")]
fn is_newer_version(latest: &str, current: &str) -> bool {
    use semver::Version;
    
    let latest_ver = Version::parse(latest).ok();
    let current_ver = Version::parse(current).ok();
    
    match (latest_ver, current_ver) {
        (Some(l), Some(c)) => l > c,
        _ => latest != current, // Fallback to string comparison
    }
}

/// Find the appropriate asset for the current platform
#[cfg(feature = "native")]
fn find_platform_asset(assets: &[GitHubAsset]) -> Option<String> {
    let platform_patterns = get_platform_patterns();
    
    for asset in assets {
        let name_lower = asset.name.to_lowercase();
        for pattern in &platform_patterns {
            if name_lower.contains(pattern) {
                return Some(asset.browser_download_url.clone());
            }
        }
    }
    None
}

/// Get platform-specific URL from manifest
#[cfg(feature = "native")]
fn get_platform_url(platforms: &Platforms) -> Option<String> {
    #[cfg(target_os = "windows")]
    { platforms.windows.as_ref().map(|p| p.url.clone()) }
    
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    { platforms.macos_intel.as_ref().map(|p| p.url.clone()) }
    
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    { platforms.macos_arm.as_ref().map(|p| p.url.clone()) }
    
    #[cfg(target_os = "linux")]
    { platforms.linux.as_ref().map(|p| p.url.clone()) }
}

/// Get platform-specific filename patterns
#[cfg(feature = "native")]
fn get_platform_patterns() -> Vec<&'static str> {
    #[cfg(target_os = "windows")]
    { vec!["windows", "win64", "win-x64", ".exe", ".msi"] }
    
    #[cfg(target_os = "macos")]
    { vec!["macos", "darwin", "osx", ".dmg", ".app"] }
    
    #[cfg(target_os = "linux")]
    { vec!["linux", "appimage", ".deb", ".rpm"] }
}

/// Download update to temp directory
#[cfg(feature = "native")]
pub fn download_update(url: &str, progress_callback: Option<Box<dyn Fn(u64, u64)>>) -> Result<std::path::PathBuf, String> {
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let client = reqwest::blocking::Client::builder()
        .user_agent("Poly-Updater/1.0")
        .timeout(std::time::Duration::from_secs(300)) // 5 min timeout
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    let mut response = client.get(url)
        .send()
        .map_err(|e| format!("Failed to download: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Download failed: {}", response.status()));
    }
    
    let total_size = response.content_length().unwrap_or(0);
    
    // Get filename from URL and add timestamp for uniqueness
    let filename = url.split('/').last().unwrap_or("update");
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let temp_dir = std::env::temp_dir();
    let download_path = temp_dir.join(format!("poly_update_{}_{}", timestamp, filename));
    
    // Remove old update files
    if let Ok(entries) = std::fs::read_dir(&temp_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("poly_update_") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    
    let mut file = std::fs::File::create(&download_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    
    let mut downloaded: u64 = 0;
    let mut buffer = [0u8; 8192];
    
    loop {
        use std::io::Read;
        let bytes_read = response.read(&mut buffer)
            .map_err(|e| format!("Download error: {}", e))?;
        
        if bytes_read == 0 { break; }
        
        file.write_all(&buffer[..bytes_read])
            .map_err(|e| format!("Write error: {}", e))?;
        
        downloaded += bytes_read as u64;
        
        if let Some(ref cb) = progress_callback {
            cb(downloaded, total_size);
        }
    }
    
    Ok(download_path)
}

/// Install update (platform-specific)
#[cfg(all(feature = "native", target_os = "windows"))]
pub fn install_update(update_path: &Path) -> Result<(), String> {
    use std::process::Command;
    
    let ext = update_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    
    match ext {
        "msi" => {
            // Run MSI installer and exit current app
            Command::new("msiexec")
                .args(["/i", &update_path.to_string_lossy()])
                .spawn()
                .map_err(|e| format!("Failed to run installer: {}", e))?;
            
            std::process::exit(0);
        }
        "exe" => {
            // For exe files, we need to replace the current executable
            // This is tricky because the current exe is locked
            // We'll create a batch script to do the replacement after we exit
            
            let current_exe = std::env::current_exe()
                .map_err(|e| format!("Failed to get current exe path: {}", e))?;
            
            let batch_path = std::env::temp_dir().join("poly_update.bat");
            let batch_content = format!(
                r#"@echo off
echo Updating Poly...
timeout /t 2 /nobreak >nul
copy /Y "{}" "{}"
if errorlevel 1 (
    echo Update failed! Please manually copy the file.
    echo From: {}
    echo To: {}
    pause
) else (
    echo Update complete!
    del "{}"
    start "" "{}"
)
del "%~f0"
"#,
                update_path.display(),
                current_exe.display(),
                update_path.display(),
                current_exe.display(),
                update_path.display(),
                current_exe.display()
            );
            
            std::fs::write(&batch_path, batch_content)
                .map_err(|e| format!("Failed to create update script: {}", e))?;
            
            // Run the batch script and exit
            Command::new("cmd")
                .args(["/C", "start", "", &batch_path.to_string_lossy()])
                .spawn()
                .map_err(|e| format!("Failed to run update script: {}", e))?;
            
            println!("  Update will complete after Poly exits...");
            std::process::exit(0);
        }
        "zip" => {
            // Extract and replace
            extract_and_replace(update_path)?;
            Ok(())
        }
        _ => Err(format!("Unknown update format: {}", ext))
    }
}

#[cfg(all(feature = "native", target_os = "macos"))]
pub fn install_update(update_path: &Path) -> Result<(), String> {
    use std::process::Command;
    
    let ext = update_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    
    match ext {
        "dmg" => {
            // Open DMG
            Command::new("open")
                .arg(update_path)
                .spawn()
                .map_err(|e| format!("Failed to open DMG: {}", e))?;
            Ok(())
        }
        "zip" => {
            extract_and_replace(update_path)?;
            Ok(())
        }
        _ => Err(format!("Unknown update format: {}", ext))
    }
}

#[cfg(all(feature = "native", target_os = "linux"))]
pub fn install_update(update_path: &Path) -> Result<(), String> {
    use std::process::Command;
    
    let ext = update_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    
    match ext {
        "AppImage" => {
            // Make executable and run
            Command::new("chmod")
                .args(["+x", &update_path.to_string_lossy()])
                .status()
                .map_err(|e| format!("Failed to chmod: {}", e))?;
            
            Command::new(update_path)
                .spawn()
                .map_err(|e| format!("Failed to run AppImage: {}", e))?;
            
            std::process::exit(0);
        }
        "deb" => {
            // Install with dpkg
            Command::new("pkexec")
                .args(["dpkg", "-i", &update_path.to_string_lossy()])
                .status()
                .map_err(|e| format!("Failed to install deb: {}", e))?;
            Ok(())
        }
        _ => Err(format!("Unknown update format: {}", ext))
    }
}

#[cfg(feature = "native")]
fn extract_and_replace(zip_path: &Path) -> Result<(), String> {
    use std::io::Read;
    
    let file = std::fs::File::open(zip_path)
        .map_err(|e| format!("Failed to open zip: {}", e))?;
    
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip: {}", e))?;
    
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?;
    
    let exe_dir = exe_path.parent()
        .ok_or("Failed to get exe directory")?;
    
    // Extract to temp, then replace
    let temp_extract = std::env::temp_dir().join("poly_update_extract");
    std::fs::create_dir_all(&temp_extract).ok();
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?;
        
        let outpath = temp_extract.join(file.name());
        
        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath).ok();
        } else {
            if let Some(p) = outpath.parent() {
                std::fs::create_dir_all(p).ok();
            }
            let mut outfile = std::fs::File::create(&outpath)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to extract: {}", e))?;
        }
    }
    
    // TODO: Replace current exe with extracted one
    // This is tricky on Windows due to file locking
    
    Ok(())
}

// Stubs for non-native builds
#[cfg(not(feature = "native"))]
pub fn check_github_updates(_repo: &str, _current_version: &str) -> Result<UpdateInfo, String> {
    Err("Native feature not enabled".to_string())
}

#[cfg(not(feature = "native"))]
pub fn check_custom_updates(_update_url: &str, _current_version: &str) -> Result<UpdateInfo, String> {
    Err("Native feature not enabled".to_string())
}

#[cfg(not(feature = "native"))]
pub fn download_update(_url: &str, _progress_callback: Option<Box<dyn Fn(u64, u64)>>) -> Result<std::path::PathBuf, String> {
    Err("Native feature not enabled".to_string())
}

#[cfg(not(feature = "native"))]
pub fn install_update(_update_path: &Path) -> Result<(), String> {
    Err("Native feature not enabled".to_string())
}

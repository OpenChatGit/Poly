//! Package Management for Poly
//! Fast npm-based package manager (UV-style)

use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[cfg(feature = "native")]
use std::io::{self, Write};
#[cfg(feature = "native")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "native")]
use std::thread;

// ANSI colors
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
#[allow(dead_code)]
const YELLOW: &str = "\x1b[33m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

/// NPM package metadata
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct NpmPackage {
    pub name: String,
    pub version: String,
    pub dist: NpmDist,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct NpmDist {
    pub tarball: String,
    #[serde(default)]
    pub integrity: Option<String>,
}

/// Lockfile entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LockEntry {
    pub version: String,
    pub integrity: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// Shared HTTP client for connection pooling
#[cfg(feature = "native")]
fn get_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("poly/1.0")
        .pool_max_idle_per_host(10)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap()
}

/// Fetch package info from npm registry
#[cfg(feature = "native")]
pub fn fetch_npm_package(client: &reqwest::blocking::Client, name: &str, version: Option<&str>) -> Result<NpmPackage, String> {
    let url = match version {
        Some(v) => format!("https://registry.npmjs.org/{}/{}", name, v),
        None => format!("https://registry.npmjs.org/{}/latest", name),
    };
    
    let response = client.get(&url)
        .send()
        .map_err(|e| format!("Failed to fetch: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Package '{}' not found", name));
    }
    
    response.json::<NpmPackage>()
        .map_err(|e| format!("Parse error: {}", e))
}

#[cfg(not(feature = "native"))]
#[allow(dead_code)]
pub fn fetch_npm_package(_client: &(), _name: &str, _version: Option<&str>) -> Result<NpmPackage, String> {
    Err("Requires native feature".to_string())
}

/// Download and extract npm tarball
#[cfg(feature = "native")]
pub fn download_and_extract(client: &reqwest::blocking::Client, pkg: &NpmPackage, dest: &Path) -> Result<String, String> {
    use flate2::read::GzDecoder;
    use tar::Archive;
    use sha2::{Sha256, Digest};
    
    let response = client.get(&pkg.dist.tarball)
        .send()
        .map_err(|e| format!("Download failed: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Download failed: {}", response.status()));
    }
    
    let bytes = response.bytes()
        .map_err(|e| format!("Read failed: {}", e))?;
    
    // Calculate hash
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = format!("sha256-{}", base64_encode(&hasher.finalize()));
    
    // Create directory
    fs::create_dir_all(dest).ok();
    
    // Extract tarball
    let decoder = GzDecoder::new(&bytes[..]);
    let mut archive = Archive::new(decoder);
    
    for entry in archive.entries().map_err(|e| format!("Tar error: {}", e))? {
        let mut entry = entry.map_err(|e| format!("Entry error: {}", e))?;
        let path = entry.path().map_err(|e| format!("Path error: {}", e))?;
        
        // Strip "package/" prefix
        let path_str = path.to_string_lossy();
        let rel = path_str.strip_prefix("package/").unwrap_or(&path_str);
        if rel.is_empty() { continue; }
        
        let dest_path = dest.join(rel);
        
        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(&dest_path).ok();
        } else {
            if let Some(p) = dest_path.parent() {
                fs::create_dir_all(p).ok();
            }
            entry.unpack(&dest_path).ok();
        }
    }
    
    Ok(hash)
}

#[cfg(not(feature = "native"))]
#[allow(dead_code)]
pub fn download_and_extract(_client: &(), _pkg: &NpmPackage, _dest: &Path) -> Result<String, String> {
    Err("Requires native feature".to_string())
}

#[allow(dead_code)]
fn base64_encode(data: &[u8]) -> String {
    const ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        result.push(ALPHA[b0 >> 2] as char);
        result.push(ALPHA[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        result.push(if chunk.len() > 1 { ALPHA[((b1 & 0x0f) << 2) | (b2 >> 6)] as char } else { '=' });
        result.push(if chunk.len() > 2 { ALPHA[b2 & 0x3f] as char } else { '=' });
    }
    result
}

pub fn read_lockfile() -> HashMap<String, LockEntry> {
    let path = Path::new("poly.lock");
    if !path.exists() { return HashMap::new(); }
    fs::read_to_string(path).ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

pub fn write_lockfile(entries: &HashMap<String, LockEntry>) -> Result<(), String> {
    let content = serde_json::to_string_pretty(entries)
        .map_err(|e| format!("JSON error: {}", e))?;
    fs::write("poly.lock", content).map_err(|e| format!("Write error: {}", e))
}


/// poly add <package> - add an npm dependency for frontend imports
#[cfg(feature = "native")]
pub fn add_package(package: &str, version: Option<&str>) -> Result<(), String> {
    println!();
    println!("  {}POLY{} {}add{}", CYAN, RESET, DIM, RESET);
    println!();
    
    if !Path::new("poly.toml").exists() {
        return Err("Not a Poly project. Run 'poly init' first.".to_string());
    }
    
    let needs_vite = crate::scaffold::enable_vite_imports(Path::new("."))?;
    if needs_vite {
        run_npm(&["install", "--save-dev", "vite"])?;
    }

    let spec = version.map(|version| format!("{package}@{version}")).unwrap_or_else(|| package.to_string());
    run_npm(&["install", "--save", &spec])
}

#[cfg(feature = "native")]
fn run_npm(args: &[&str]) -> Result<(), String> {
    let status = crate::scaffold::npm_command()
        .args(args)
        .status()
        .map_err(|error| format!("Could not start npm: {error}"))?;
    if !status.success() {
        return Err(format!("npm {} failed with {status}.", args.join(" ")));
    }
    Ok(())
}

#[cfg(not(feature = "native"))]
pub fn add_package(_package: &str, _version: Option<&str>) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

/// poly remove <package>
pub fn remove_package(package: &str) -> Result<(), String> {
    println!();
    println!("  {}POLY{} {}remove{}", CYAN, RESET, DIM, RESET);
    println!();
    
    if Path::new("package.json").is_file() {
        #[cfg(feature = "native")]
        return run_npm(&["uninstall", package]);
        #[cfg(not(feature = "native"))]
        return Err("Removing npm packages requires the native feature.".to_string());
    }

    let pkg_dir = Path::new("packages").join(package);
    
    if !pkg_dir.exists() {
        return Err(format!("Package '{}' not installed", package));
    }
    
    fs::remove_dir_all(&pkg_dir).map_err(|e| format!("Remove failed: {}", e))?;
    
    println!("  {}Removed{} {}{}{}", GREEN, RESET, BOLD, package, RESET);
    
    remove_from_poly_toml(package)?;
    
    let mut lockfile = read_lockfile();
    lockfile.remove(package);
    write_lockfile(&lockfile)?;
    
    println!();
    Ok(())
}

/// poly install - install from lockfile
#[cfg(feature = "native")]
pub fn install_packages(verify_only: bool) -> Result<(), String> {
    use std::time::Instant;
    
    println!();
    println!("  {}POLY{} {}install{}", CYAN, RESET, DIM, RESET);
    println!();
    
    if !Path::new("poly.toml").exists() {
        return Err("Not a Poly project.".to_string());
    }

    if Path::new("package.json").is_file() {
        return run_npm(if verify_only { &["ls", "--depth=0"] } else { &["install"] });
    }
    
    let lockfile = read_lockfile();
    
    if lockfile.is_empty() {
        println!("  {}No packages in poly.lock{}", DIM, RESET);
        println!();
        return Ok(());
    }
    
    if verify_only {
        for (name, entry) in &lockfile {
            let pkg_dir = Path::new("packages").join(name);
            let status = if pkg_dir.exists() { "✓" } else { "missing" };
            let color = if pkg_dir.exists() { GREEN } else { YELLOW };
            println!("  {}{}@{}{} {}", color, name, entry.version, RESET, status);
        }
        println!();
        return Ok(());
    }
    
    let start = Instant::now();
    
    // Find packages that need installation
    let to_install: Vec<_> = lockfile.iter()
        .filter(|(name, _)| !Path::new("packages").join(name).exists())
        .collect();
    
    if to_install.is_empty() {
        println!("  {}All packages up to date{}", DIM, RESET);
        println!();
        return Ok(());
    }
    
    println!("  {}Installing {} packages...{}", DIM, to_install.len(), RESET);
    
    let counter: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let total = to_install.len();
    let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    
    let handles: Vec<_> = to_install.into_iter().map(|(name, entry)| {
        let name = name.clone();
        let version = entry.version.clone();
        let counter = Arc::clone(&counter);
        let errors = Arc::clone(&errors);
        
        thread::spawn(move || {
            let client = get_client();
            let pkg_dir = Path::new("packages").join(&name);
            
            match fetch_npm_package(&client, &name, Some(&version)) {
                Ok(pkg) => {
                    if let Err(e) = download_and_extract(&client, &pkg, &pkg_dir) {
                        errors.lock().unwrap().push(format!("{}: {}", name, e));
                    }
                }
                Err(e) => {
                    errors.lock().unwrap().push(format!("{}: {}", name, e));
                }
            }
            
            let mut c = counter.lock().unwrap();
            *c += 1;
            print!("\r  {}Installing... {}/{}{}", DIM, *c, total, RESET);
            io::stdout().flush().ok();
        })
    }).collect();
    
    for handle in handles {
        handle.join().ok();
    }
    
    let errs = errors.lock().unwrap();
    if !errs.is_empty() {
        println!();
        for e in errs.iter() {
            println!("  {}Error:{} {}", YELLOW, RESET, e);
        }
    }
    
    let elapsed = start.elapsed();
    println!("\r  {}Installed {} packages in {:.2}s{}          ", GREEN, total, elapsed.as_secs_f64(), RESET);
    println!();
    
    Ok(())
}

#[cfg(not(feature = "native"))]
pub fn install_packages(_verify_only: bool) -> Result<(), String> {
    Err("Requires native feature".to_string())
}

#[allow(dead_code)]
fn update_poly_toml(package: &str, version: &str) -> Result<(), String> {
    let path = Path::new("poly.toml");
    let content = fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
    
    let key = if package.contains('.') || package.contains('-') || package.contains('@') {
        format!("\"{}\"", package)
    } else {
        package.to_string()
    };
    let line = format!("{} = \"{}\"", key, version);
    
    let new_content = if content.contains("[dependencies]") {
        let exists = content.lines().any(|l| {
            let t = l.trim();
            t.starts_with(&format!("{} =", package)) || t.starts_with(&format!("\"{}\" =", package))
        });
        
        if exists {
            content.lines().map(|l| {
                let t = l.trim();
                if t.starts_with(&format!("{} =", package)) || t.starts_with(&format!("\"{}\" =", package)) {
                    line.clone()
                } else {
                    l.to_string()
                }
            }).collect::<Vec<_>>().join("\n")
        } else {
            content.replace("[dependencies]", &format!("[dependencies]\n{}", line))
        }
    } else {
        format!("{}\n\n[dependencies]\n{}", content.trim_end(), line)
    };
    
    fs::write(path, new_content).map_err(|e| format!("Write error: {}", e))
}

fn remove_from_poly_toml(package: &str) -> Result<(), String> {
    let path = Path::new("poly.toml");
    let content = fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
    
    let new_content: String = content.lines()
        .filter(|l| {
            let t = l.trim();
            !t.starts_with(&format!("{} =", package)) && !t.starts_with(&format!("\"{}\" =", package))
        })
        .collect::<Vec<_>>()
        .join("\n");
    
    fs::write(path, new_content).map_err(|e| format!("Write error: {}", e))
}

pub fn add_to_gitignore(entry: &str) -> Result<(), String> {
    let path = Path::new(".gitignore");
    let content = fs::read_to_string(path).unwrap_or_default();
    
    if !content.contains(entry) {
        let new = if content.is_empty() {
            format!("{}\n", entry)
        } else if content.ends_with('\n') {
            format!("{}{}\n", content, entry)
        } else {
            format!("{}\n{}\n", content, entry)
        };
        fs::write(path, new).map_err(|e| format!("Write error: {}", e))?;
    }
    Ok(())
}

// VSCode Extension Auto-Installation for Poly
// This module allows the poly executable to automatically install 
// the VSCode extension for syntax highlighting

use std::path::{Path, PathBuf};
use std::fs;

const EXTENSION_FILES: &[(&str, &[u8])] = &[
    ("package.json", include_bytes!("../../poly-vscode/package.json")),
    ("language-configuration.json", include_bytes!("../../poly-vscode/language-configuration.json")),
    ("syntaxes/poly.tmLanguage.json", include_bytes!("../../poly-vscode/syntaxes/poly.tmLanguage.json")),
];

/// Get potential VSCode-compatible extensions directories
fn get_potential_extension_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    
    #[cfg(target_os = "windows")]
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        let base = PathBuf::from(userprofile);
        dirs.push(base.join(".vscode").join("extensions"));
        dirs.push(base.join(".antigravity").join("extensions"));
        dirs.push(base.join(".cursor").join("extensions"));
        dirs.push(base.join(".vscodium").join("extensions"));
    }
    
    #[cfg(not(target_os = "windows"))]
    if let Ok(home) = std::env::var("HOME") {
        let base = PathBuf::from(home);
        dirs.push(base.join(".vscode").join("extensions"));
        dirs.push(base.join(".antigravity").join("extensions"));
        dirs.push(base.join(".cursor").join("extensions"));
        dirs.push(base.join(".vscodium").join("extensions"));
    }
    
    dirs
}

/// Install the VSCode extension
pub fn install_vscode_extension() -> Result<(), String> {
    println!("🔧 Installing Poly VSCode Extension...");
    
    let potential_dirs = get_potential_extension_dirs();
    let mut installed_count = 0;
    
    for extensions_dir in potential_dirs {
        // Only install if the parent config directory exists (e.g. .vscode, .antigravity)
        // OR if the extensions directory itself already exists
        let parent_exists = extensions_dir.parent().map(|p| p.exists()).unwrap_or(false);
        let dir_exists = extensions_dir.exists();
        
        if parent_exists || dir_exists {
            match install_to_dir(&extensions_dir) {
                Ok(_) => {
                    println!("  ✅ Installed to: {}", extensions_dir.display());
                    installed_count += 1;
                },
                Err(e) => println!("  ❌ Failed to install to {}: {}", extensions_dir.display(), e),
            }
        }
    }
    
    if installed_count > 0 {
        println!("\n✅ Poly VSCode Extension installed successfully in {} location(s)!", installed_count);
        println!("\n📝 Next steps:");
        println!("   1. Restart your Editor (VSCode/Antigravity/Cursor)");
        println!("   2. Open a .poly file");
        println!("   3. Enjoy syntax highlighting! 🎨");
        Ok(())
    } else {
        Err("No suitable VSCode/Antigravity installation found. Please ensure you have an editor installed.".to_string())
    }
}

fn install_to_dir(extensions_dir: &Path) -> Result<(), String> {
    let extension_dir = extensions_dir.join("poly-lang-0.1.0");
    
    // Create extension directory
    fs::create_dir_all(&extension_dir)
        .map_err(|e| format!("Failed to create extension directory: {}", e))?;
    
    // Extract embedded files
    for (relative_path, content) in EXTENSION_FILES {
        let file_path = extension_dir.join(relative_path);
        
        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
        }
        
        // Write file
        fs::write(&file_path, content)
            .map_err(|e| format!("Failed to write {}: {}", file_path.display(), e))?;
    }
    
    Ok(())
}

/// Check if extension is installed in ANY location
pub fn is_extension_installed() -> bool {
    let potential_dirs = get_potential_extension_dirs();
    for dir in potential_dirs {
        if dir.join("poly-lang-0.1.0").exists() {
            return true;
        }
    }
    false
}

/// Uninstall the VSCode extension from ALL locations
pub fn uninstall_vscode_extension() -> Result<(), String> {
    println!("🗑️  Uninstalling Poly VSCode Extension...");
    
    let potential_dirs = get_potential_extension_dirs();
    let mut uninstalled_count = 0;
    
    for extensions_dir in potential_dirs {
        let extension_dir = extensions_dir.join("poly-lang-0.1.0");
        if extension_dir.exists() {
            match fs::remove_dir_all(&extension_dir) {
                Ok(_) => {
                    println!("  ✅ Uninstalled from: {}", extensions_dir.display());
                    uninstalled_count += 1;
                },
                Err(e) => println!("  ❌ Failed to uninstall from {}: {}", extensions_dir.display(), e),
            }
        }
    }
    
    if uninstalled_count > 0 {
        println!("✅ Extension uninstalled successfully from {} location(s)!", uninstalled_count);
        Ok(())
    } else {
        Err("Extension was not found in any standard location.".to_string())
    }
}

use super::*;

pub(super) fn build_app(path: &str, target: &str, release: bool, installer: bool, sign: bool) -> Result<(), String> {
    let project_path = Path::new(path);

    // Check if it's the old-style target (web, native, all)
    match target {
        "web" | "wasm" => {
            println!();
            println!("  {}POLY{} {}build{}", CYAN, RESET, DIM, RESET);
            println!();
            build_web(project_path)?;
            println!();
            return Ok(());
        }
        "all" => {
            // Build both web and native
            println!();
            println!("  {}POLY{} {}build{}", CYAN, RESET, DIM, RESET);
            println!();
            build_web(project_path)?;
            // Fall through to native build
        }
        _ => {}
    }

    // Use new cross-platform build system
    let platform = build::Platform::from_str(target).unwrap_or(build::Platform::Current);

    let config = build::BuildConfig {
        project_path: project_path.to_path_buf(),
        platform,
        release,
        bundle: true,
        installer,
        sign,
    };

    build::build(&config).map(|_| ())
}

fn build_web(project_path: &Path) -> Result<(), String> {
    build::build_frontend(project_path)?;
    let dist = project_path.join("dist/web");
    fs::create_dir_all(&dist)
        .map_err(|error| format!("Could not create output directory {}: {error}", dist.display()))?;

    let config = poly::PolyConfig::load(project_path);
    let web_dir = project_path.join(&config.web.dir);
    if web_dir.exists() && web_dir != dist {
        copy_dir_recursive(&web_dir, &dist)
            .map_err(|error| format!("Could not bundle web assets: {error}"))?;
    }

    let index = dist.join("index.html");
    if !index.exists() {
        let name = project_path.file_name().and_then(|n| n.to_str()).unwrap_or("App");
        fs::write(&index, format!(r#"<!DOCTYPE html><html><head><meta charset="UTF-8"><title>{}</title></head><body><div id="app"></div></body></html>"#, name))
            .map_err(|error| format!("Could not create {}: {error}", index.display()))?;
    }

    println!("  {}>{} dist/web/index.html", GREEN, RESET);
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dest = dst.join(entry.file_name());
        if entry.path().is_dir() { copy_dir_recursive(&entry.path(), &dest)?; }
        else { fs::copy(entry.path(), &dest)?; }
    }
    Ok(())
}

pub(super) fn extract_toml_value(content: &str, key: &str) -> Option<String> {
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

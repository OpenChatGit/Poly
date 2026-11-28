use super::*;

pub(super) fn selected_project_template(
    template: &str,
    react: bool,
    svelte: bool,
    vue: bool,
    preact: bool,
) -> Result<String, String> {
    let selected = if react { "react" }
        else if svelte { "svelte" }
        else if vue { "vue" }
        else if preact { "preact" }
        else { template };
    match selected {
        "app" | "react" | "svelte" | "vue" | "preact" => Ok(selected.to_string()),
        other => Err(format!("Unknown project template '{other}'. Choose app, react, svelte, vue, or preact.")),
    }
}

pub(super) fn create_project(name: &str, template: &str, no_install: bool) -> Result<(), String> {
    println!("\n  {}POLY{} v{}\n", CYAN, RESET, VERSION);
    let project_path = Path::new(name);
    if project_path.exists() {
        return Err(format!("Directory '{}' already exists.", name));
    }
    fs::create_dir_all(project_path).map_err(|error| format!("Could not create project folder: {error}"))?;
    initialize_project_files(project_path, name, template, no_install, false)?;
    println!("\n  {}✓{} Created {} project in {}", GREEN, RESET, template, name);
    print_next_steps(name, template);
    Ok(())
}

pub(super) fn init_project(template: &str, no_install: bool) -> Result<(), String> {
    println!("\n  {}POLY{} v{}\n", CYAN, RESET, VERSION);
    let cwd = std::env::current_dir().map_err(|error| format!("Could not read current directory: {error}"))?;
    let name = cwd.file_name().and_then(|part| part.to_str()).unwrap_or("app");
    initialize_project_files(&cwd, name, template, no_install, false)?;
    println!("\n  {}✓{} Initialized {} project", GREEN, RESET, template);
    print_next_steps(".", template);
    Ok(())
}

pub(super) fn reinit_project(template: &str, no_install: bool, yes: bool) -> Result<(), String> {
    println!("\n  {}POLY{} v{}\n", CYAN, RESET, VERSION);
    let project_path = std::env::current_dir().map_err(|error| format!("Could not read current directory: {error}"))?;
    if !project_path.join("poly.toml").is_file() {
        return Err("Not an initialized Poly project. Run `poly init` first.".into());
    }

    let previous_template = read_scaffold_template(&project_path).unwrap_or_else(|| detect_scaffold_template(&project_path));
    if !yes {
        println!("  This will refresh Poly's generated {previous_template} starter files for the {template} template.");
        println!("  Your poly.toml settings, custom files, package.json fields, and node_modules will be preserved where possible.");
        print!("  Continue? [y/N] ");
        io::stdout().flush().map_err(|error| format!("Could not write confirmation prompt: {error}"))?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer).map_err(|error| format!("Could not read confirmation: {error}"))?;
        if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
            println!("  Reinit cancelled.");
            return Ok(());
        }
    }

    refresh_generated_scaffold(&project_path, &previous_template, template, no_install)?;
    println!("\n  {}✓{} Reinitialized {} project", GREEN, RESET, template);
    print_next_steps(".", template);
    Ok(())
}

fn read_scaffold_template(project_path: &Path) -> Option<String> {
    let marker = fs::read_to_string(project_path.join(".poly/scaffold.json")).ok()?;
    serde_json::from_str::<serde_json::Value>(&marker).ok()?
        .get("template")?.as_str().map(str::to_owned)
}

fn detect_scaffold_template(project_path: &Path) -> String {
    let source = project_path.join("web/src");
    for (template, file) in [("svelte", "App.svelte"), ("vue", "App.vue"), ("react", "App.jsx"), ("preact", "App.jsx")] {
        if source.join(file).is_file() {
            if template == "react" && project_path.join("package.json").is_file() {
                let manifest = fs::read_to_string(project_path.join("package.json")).unwrap_or_default();
                if manifest.contains("@preact/preset-vite") { return "preact".into(); }
            }
            return template.into();
        }
    }
    "app".into()
}

fn refresh_generated_scaffold(project_path: &Path, old_template: &str, template: &str, no_install: bool) -> Result<(), String> {
    let vite_config_path = project_path.join("vite.config.js");
    let replace_vite_config = if matches!(old_template, "react" | "preact" | "vue" | "svelte") && vite_config_path.is_file() {
        let config = fs::read_to_string(&vite_config_path).map_err(|error| format!("Could not read {}: {error}", vite_config_path.display()))?;
        let is_poly_config = config.contains("root: 'web'")
            && config.contains("process.env.POLY_BACKEND_URL")
            && config.contains("build: { outDir: '../dist/web'");
        if !is_poly_config {
            return Err(format!("{} appears to contain custom settings. Poly left it untouched; update its framework plugin manually before reinitializing.", vite_config_path.display()));
        }
        true
    } else {
        false
    };
    let index_path = project_path.join("web/index.html");
    if matches!(template, "react" | "preact" | "vue" | "svelte")
        && index_path.is_file()
        && !is_generated_index(&index_path)
    {
        return Err(format!("{} contains a custom page. Poly preserved it; move it aside before reinitializing with a framework template.", index_path.display()));
    }

    let source_dir = project_path.join("web/src");
    // The fixed filenames below are Poly's generated starter entry/component files.
    // Preserve additional source files and directories owned by the project.
    for extension in ["js", "jsx", "ts", "tsx", "vue", "svelte"] {
        remove_generated_file(&source_dir.join(format!("main.{extension}")))?;
        remove_generated_file(&source_dir.join(format!("App.{extension}")))?;
    }
    for relative in ["web/src/style.css", "web/src/starter.js", "web/src/poly.js", "web/framework-logo.svg"] {
        remove_generated_file(&project_path.join(relative))?;
    }
    remove_generated_index_if_poly(project_path)?;

    // Replace the generated Vite config when changing an existing Poly framework
    // template. Keep unrelated/custom Vite configs intact and let scaffold_frontend
    // report a conflict if it cannot safely own them.
    if replace_vite_config {
        remove_generated_file(&vite_config_path)?;
    }

    let web_dir = if matches!(template, "react" | "preact" | "vue" | "svelte") { "dist/web" } else { "web" };
    configure_web_dir(&project_path.join("poly.toml"), web_dir)?;
    update_scaffold_manifest(project_path, old_template, template)?;
    initialize_project_files(project_path, project_path.file_name().and_then(|name| name.to_str()).unwrap_or("app"), template, no_install, true)?;
    let marker = serde_json::json!({ "template": template, "version": VERSION });
    let marker_path = project_path.join(".poly/scaffold.json");
    fs::create_dir_all(marker_path.parent().unwrap()).map_err(|error| format!("Could not create .poly/: {error}"))?;
    fs::write(marker_path, serde_json::to_string_pretty(&marker).unwrap() + "\n")
        .map_err(|error| format!("Could not update Poly scaffold metadata: {error}"))?;
    Ok(())
}

fn remove_generated_file(path: &Path) -> Result<(), String> {
    if path.is_file() {
        fs::remove_file(path).map_err(|error| format!("Could not replace generated file {}: {error}", path.display()))?;
    }
    Ok(())
}

fn remove_generated_index_if_poly(project_path: &Path) -> Result<(), String> {
    let path = project_path.join("web/index.html");
    if is_generated_index(&path) { remove_generated_file(&path)?; }
    Ok(())
}

fn is_generated_index(path: &Path) -> bool {
    let Ok(html) = fs::read_to_string(path) else { return false; };
    (html.contains("src=\"/src/starter.js\"") && html.contains("<title>Poly "))
        || (html.contains("class=\"brand-logo\"") && html.contains("Built with Poly and web technologies."))
}

fn update_scaffold_manifest(project_path: &Path, old_template: &str, template: &str) -> Result<(), String> {
    let manifest_path = project_path.join("package.json");
    if !manifest_path.is_file() {
        // Let the normal scaffold create the correctly named initial manifest.
        return Ok(());
    }
    if !matches!(old_template, "react" | "preact" | "vue" | "svelte") && !matches!(template, "react" | "preact" | "vue" | "svelte") {
        return Ok(());
    }
    let contents = fs::read_to_string(&manifest_path).map_err(|error| format!("Could not read package.json: {error}"))?;
    let mut manifest = serde_json::from_str::<serde_json::Value>(&contents).map_err(|error| format!("Could not parse package.json: {error}"))?;
    let object = manifest.as_object_mut().ok_or("package.json must contain a JSON object.")?;
    let generated_packages: &[&str] = match old_template {
        "react" => &["react", "react-dom", "vite", "@vitejs/plugin-react"],
        "preact" => &["preact", "vite", "@preact/preset-vite"],
        "vue" => &["vue", "vite", "@vitejs/plugin-vue"],
        "svelte" => &["svelte", "vite", "@sveltejs/vite-plugin-svelte"],
        _ => &[],
    };
    for field in ["dependencies", "devDependencies"] {
        if let Some(dependencies) = object.get_mut(field).and_then(serde_json::Value::as_object_mut) {
            for package in generated_packages {
                if dependencies.get(*package).and_then(serde_json::Value::as_str) == Some("latest") {
                    dependencies.remove(*package);
                }
            }
        }
    }
    if matches!(template, "react" | "preact" | "vue" | "svelte") {
        let packages: &[(&str, &str, &str)] = match template {
            "react" => &[("dependencies", "react", "latest"), ("dependencies", "react-dom", "latest"), ("devDependencies", "@vitejs/plugin-react", "latest")],
            "preact" => &[("dependencies", "preact", "latest"), ("devDependencies", "@preact/preset-vite", "latest")],
            "vue" => &[("dependencies", "vue", "latest"), ("devDependencies", "@vitejs/plugin-vue", "latest")],
            _ => &[("dependencies", "svelte", "latest"), ("devDependencies", "@sveltejs/vite-plugin-svelte", "latest")],
        };
        for (field, name, version) in packages {
            let dependencies = object.entry((*field).to_string()).or_insert_with(|| serde_json::json!({}));
            let dependencies = dependencies.as_object_mut().ok_or_else(|| format!("package.json `{field}` must be an object."))?;
            dependencies.entry(*name).or_insert_with(|| serde_json::Value::String((*version).into()));
        }
        let dependencies = object.entry("devDependencies").or_insert_with(|| serde_json::json!({}));
        let dependencies = dependencies.as_object_mut().ok_or("package.json `devDependencies` must be an object.")?;
        dependencies.entry("vite").or_insert_with(|| serde_json::Value::String("latest".into()));
        object.entry("scripts").or_insert_with(|| serde_json::json!({}));
        let scripts = object.get_mut("scripts").and_then(serde_json::Value::as_object_mut).ok_or("package.json `scripts` must be an object.")?;
        scripts.entry("dev").or_insert(serde_json::Value::String("vite".into()));
        scripts.entry("build").or_insert(serde_json::Value::String("vite build".into()));
        scripts.entry("preview").or_insert(serde_json::Value::String("vite preview".into()));
    }
    fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap() + "\n")
        .map_err(|error| format!("Could not update package.json: {error}"))
}

fn initialize_project_files(project_path: &Path, name: &str, template: &str, no_install: bool, reinitializing: bool) -> Result<(), String> {
    let is_frontend = matches!(template, "react" | "svelte" | "vue" | "preact");
    for directory in ["src", "web", "assets"] {
        fs::create_dir_all(project_path.join(directory))
            .map_err(|error| format!("Could not create {directory}/: {error}"))?;
    }
    let web_dir = if is_frontend { "dist/web" } else { "web" };
    let poly_toml = project_path.join("poly.toml");
    if !poly_toml.exists() {
        fs::write(&poly_toml, format!(r#"[package]
name = "{name}"
version = "0.1.0"

[web]
dir = "{web_dir}"

[window]
title = "{name}"
width = 1440
height = 900
resizable = true
decorations = true
icon_path = "assets/icon.png"

"#)).map_err(|error| format!("Could not create poly.toml: {error}"))?;
    } else if is_frontend {
        configure_web_dir(&poly_toml, "dist/web")?;
    }
    let gitignore = project_path.join(".gitignore");
    if !gitignore.exists() {
        fs::write(&gitignore, "/dist\n/target\n/packages\n/node_modules\n")
            .map_err(|error| format!("Could not create .gitignore: {error}"))?;
    } else {
        packages::add_to_gitignore("packages/").ok();
        if is_frontend {
            append_gitignore_entry(project_path, "/node_modules")?;
        }
    }
    let icon_path = project_path.join("assets/icon.png");
    if !icon_path.exists() {
        fs::write(&icon_path, poly::templates::DEFAULT_ICON_PNG)
            .map_err(|error| format!("Could not write app icon: {error}"))?;
    }
    let web_logo_path = project_path.join("web/poly-logo.png");
    if !web_logo_path.exists() {
        fs::write(&web_logo_path, poly::templates::DEFAULT_ICON_PNG)
            .map_err(|error| format!("Could not write the Poly logo: {error}"))?;
    }

    match template {
        "app" => write_if_missing(project_path, "web/index.html", &poly::templates::empty_app_html(name))?,
        "react" | "svelte" | "vue" | "preact" => {
            scaffold_frontend(project_path, template, name, reinitializing)?;
            if !no_install {
                install_frontend_dependencies(project_path)?;
            }
        }
        _ => return Err(format!("Unknown project template '{template}'.")),
    }
    let marker_path = project_path.join(".poly/scaffold.json");
    fs::create_dir_all(marker_path.parent().unwrap()).map_err(|error| format!("Could not create .poly/: {error}"))?;
    let marker = serde_json::json!({ "template": template, "version": VERSION });
    write_if_missing(project_path, ".poly/scaffold.json", &(serde_json::to_string_pretty(&marker).unwrap() + "\n"))?;
    Ok(())
}

fn configure_web_dir(poly_toml: &Path, web_dir: &str) -> Result<(), String> {
    let content = fs::read_to_string(poly_toml).map_err(|error| format!("Could not read poly.toml: {error}"))?;
    let mut lines = Vec::new();
    let mut in_web = false;
    let mut found_web = false;
    let mut found_dir = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if in_web && !found_dir {
                lines.push(format!("dir = \"{web_dir}\""));
                found_dir = true;
            }
            in_web = trimmed == "[web]";
            found_web |= in_web;
        }
        if in_web && trimmed.starts_with("dir") && trimmed.contains('=') {
            lines.push(format!("dir = \"{web_dir}\""));
            found_dir = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if in_web && !found_dir { lines.push(format!("dir = \"{web_dir}\"")); }
    if !found_web { lines.extend(["".into(), "[web]".into(), format!("dir = \"{web_dir}\"")]); }
    let mut updated = lines.join("\n");
    updated.push('\n');
    fs::write(poly_toml, updated).map_err(|error| format!("Could not update web output path: {error}"))
}

/// Prepare a plain Poly HTML project to use npm packages through Vite imports.
#[cfg(feature = "native")]
pub(super) fn enable_vite_imports(project_path: &Path) -> Result<bool, String> {
    let poly_toml = project_path.join("poly.toml");
    if !poly_toml.is_file() {
        return Err("Not a Poly project. Run `poly init` first.".to_string());
    }

    let config = poly::PolyConfig::load(project_path);
    let configured_web_dir = Path::new(&config.web.dir);
    let web_dir = if project_path.join("web/index.html").is_file() {
        Path::new("web").to_path_buf()
    } else if project_path.join(configured_web_dir).join("index.html").is_file() {
        if configured_web_dir == Path::new("dist/web") {
            return Err("The configured `dist/web` directory is build output. Restore the editable `web/index.html` source before adding npm imports.".to_string());
        }
        configured_web_dir.to_path_buf()
    } else {
        return Err(format!(
            "Could not find index.html in `web/` or configured web directory `{}`.",
            config.web.dir
        ));
    };
    let index_path = project_path.join(&web_dir).join("index.html");

    let manifest_path = project_path.join("package.json");
    let mut manifest = if manifest_path.is_file() {
        let contents = fs::read_to_string(&manifest_path)
            .map_err(|error| format!("Could not read package.json: {error}"))?;
        serde_json::from_str::<serde_json::Value>(&contents)
            .map_err(|error| format!("Could not parse package.json: {error}"))?
    } else {
        serde_json::json!({})
    };
    let object = manifest.as_object_mut()
        .ok_or_else(|| "package.json must contain a JSON object.".to_string())?;
    if !object.contains_key("name") {
        let name = project_path.file_name().and_then(|name| name.to_str()).unwrap_or("poly-app");
        let normalized = name.to_ascii_lowercase().chars()
            .map(|character| if character.is_ascii_alphanumeric() || matches!(character, '-' | '.') { character } else { '-' })
            .collect::<String>().trim_matches('-').to_string();
        object.insert("name".into(), serde_json::Value::String(if normalized.is_empty() { "poly-app".into() } else { normalized }));
    }
    object.entry("private").or_insert(serde_json::Value::Bool(true));
    object.entry("type").or_insert(serde_json::Value::String("module".into()));
    let scripts = object.entry("scripts").or_insert_with(|| serde_json::json!({}));
    let scripts = scripts.as_object_mut()
        .ok_or_else(|| "package.json `scripts` must be a JSON object.".to_string())?;
    scripts.entry("dev").or_insert(serde_json::Value::String("vite".into()));
    scripts.entry("build").or_insert(serde_json::Value::String("vite build".into()));
    let has_vite = object.get("dependencies").and_then(serde_json::Value::as_object).is_some_and(|dependencies| dependencies.contains_key("vite"))
        || object.get("devDependencies").and_then(serde_json::Value::as_object).is_some_and(|dependencies| dependencies.contains_key("vite"));
    let dev_dependencies = object.entry("devDependencies").or_insert_with(|| serde_json::json!({}));
    if !dev_dependencies.is_object() {
        return Err("package.json `devDependencies` must be a JSON object.".to_string());
    }
    fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap() + "\n")
        .map_err(|error| format!("Could not update package.json: {error}"))?;

    if !has_vite_config(project_path) {
        let web_root = web_dir.to_string_lossy().replace('\\', "/");
        let dist_dir = relative_project_path(&web_dir, Path::new("dist/web"));
        let web_root_json = serde_json::to_string(&web_root).unwrap();
        let dist_dir_json = serde_json::to_string(&dist_dir).unwrap();
        let contents = format!(r#"import {{ defineConfig }} from 'vite'

export default defineConfig({{
  root: {web_root_json},
  server: {{
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    proxy: {{
      '/__poly_invoke': {{ target: process.env.POLY_BACKEND_URL || 'http://127.0.0.1:3000' }},
      '/__poly_invoke_stream': {{ target: process.env.POLY_BACKEND_URL || 'http://127.0.0.1:3000' }},
      '/assets': {{ target: process.env.POLY_BACKEND_URL || 'http://127.0.0.1:3000' }}
    }}
  }},
  build: {{ outDir: {dist_dir_json}, emptyOutDir: true }}
}})
"#);
        let vite_config = project_path.join("vite.config.mjs");
        fs::write(&vite_config, contents)
            .map_err(|error| format!("Could not create vite.config.mjs: {error}"))?;
    }

    let source_dir = project_path.join(&web_dir).join("src");
    fs::create_dir_all(&source_dir).map_err(|error| format!("Could not create frontend source directory: {error}"))?;
    let entry_path = source_dir.join("main.js");
    if !entry_path.exists() {
        fs::write(&entry_path, "// Import npm packages and app modules from this entry point.\n")
            .map_err(|error| format!("Could not create {}: {error}", entry_path.display()))?;
    }

    let gitignore = project_path.join(".gitignore");
    if !gitignore.exists() {
        fs::write(&gitignore, "/dist\n/node_modules\n/packages\n")
            .map_err(|error| format!("Could not create .gitignore: {error}"))?;
    } else {
        append_gitignore_entry(project_path, "/dist")?;
        append_gitignore_entry(project_path, "/node_modules")?;
    }

    let html = fs::read_to_string(&index_path)
        .map_err(|error| format!("Could not read {}: {error}", index_path.display()))?;
    let lower_html = html.to_ascii_lowercase();
    let has_external_module = lower_html.contains("type=\"module\"") && lower_html.contains("src=");
    if !has_external_module {
        let script = "  <script type=\"module\" src=\"/src/main.js\"></script>\n";
        let updated_html = if let Some(index) = lower_html.rfind("</body>") {
            format!("{}{}{}", &html[..index], script, &html[index..])
        } else {
            format!("{}\n{}", html, script)
        };
        fs::write(&index_path, updated_html)
            .map_err(|error| format!("Could not add the Vite entry point to {}: {error}", index_path.display()))?;
    }

    configure_web_dir(&poly_toml, "dist/web")?;
    Ok(!has_vite)
}

pub(super) fn has_vite_config(project_path: &Path) -> bool {
    ["vite.config.js", "vite.config.mjs", "vite.config.ts", "vite.config.mts", "vite.config.cjs", "vite.config.cts"]
        .iter()
        .any(|name| project_path.join(name).is_file())
}

#[cfg(all(test, feature = "native"))]
mod vite_import_tests {
    use super::enable_vite_imports;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn generated_vite_config_proxies_project_assets() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let project_dir = std::env::temp_dir().join(format!(
            "poly-vite-assets-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(project_dir.join("web")).unwrap();
        fs::write(
            project_dir.join("poly.toml"),
            "[package]\nname = \"Test App\"\nversion = \"0.1.0\"\n\n[web]\ndir = \"web\"\n",
        )
        .unwrap();
        fs::write(project_dir.join("web/index.html"), "<!doctype html><html><body></body></html>")
            .unwrap();

        assert!(enable_vite_imports(&project_dir).unwrap());
        let config = fs::read_to_string(project_dir.join("vite.config.mjs")).unwrap();
        assert!(config.contains("'/assets': { target: process.env.POLY_BACKEND_URL"));
        let html = fs::read_to_string(project_dir.join("web/index.html")).unwrap();
        assert!(html.contains("src=\"/src/main.js\""));

        fs::remove_dir_all(project_dir).unwrap();
    }
}

#[cfg(feature = "native")]
fn relative_project_path(from: &Path, to: &Path) -> String {
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from.iter().zip(&to).take_while(|(left, right)| left == right).count();
    let mut parts = vec!["..".to_string(); from.len().saturating_sub(common)];
    parts.extend(to.iter().skip(common).map(|part| part.as_os_str().to_string_lossy().into_owned()));
    parts.join("/")
}

fn append_gitignore_entry(project_path: &Path, entry: &str) -> Result<(), String> {
    let path = project_path.join(".gitignore");
    let content = fs::read_to_string(&path).map_err(|error| format!("Could not read .gitignore: {error}"))?;
    if !content.lines().any(|line| line.trim().trim_end_matches('/').trim_start_matches('/') == entry.trim().trim_end_matches('/').trim_start_matches('/')) {
        let separator = if content.is_empty() || content.ends_with('\n') { "" } else { "\n" };
        fs::write(path, format!("{content}{separator}{entry}\n"))
            .map_err(|error| format!("Could not update .gitignore: {error}"))?;
    }
    Ok(())
}

fn write_if_missing(project_path: &Path, relative: &str, contents: &str) -> Result<(), String> {
    let path = project_path.join(relative);
    if path.exists() { return Ok(()); }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
    }
    fs::write(&path, contents).map_err(|error| format!("Could not create {}: {error}", path.display()))
}

fn scaffold_frontend(project_path: &Path, framework: &str, project_name: &str, reinitializing: bool) -> Result<(), String> {
    if !reinitializing && project_path.join("package.json").is_file() && !has_vite_config(project_path) {
        return Err("package.json already exists without a Vite config. Poly left it untouched; merge the generated framework setup manually or initialize an empty folder.".to_string());
    }
    let (plugin, dependencies, main_source, component_source_template) = match framework {
        "react" => ("@vitejs/plugin-react", "\"react\": \"latest\",\n    \"react-dom\": \"latest\"", "import React from 'react'\nimport { createRoot } from 'react-dom/client'\nimport App from './App.jsx'\nimport './style.css'\n\ncreateRoot(document.getElementById('root')).render(<React.StrictMode><App /></React.StrictMode>)\n", "import { useState } from 'react'\nimport polyLogo from '../poly-logo.png'\n\nexport default function App() {\n  const [count, setCount] = useState(0)\n  return <main><img className=\"brand-logo\" src={polyLogo} alt=\"Poly logo\" /><p className=\"eyebrow\">POLY × REACT</p><h1>Your desktop app starts here.</h1><p>React, Vite, and Poly are ready.</p><button onClick={() => setCount(count + 1)}>Clicked {count} times</button></main>\n}\n"),
        "preact" => ("@preact/preset-vite", "\"preact\": \"latest\"", "import { render } from 'preact'\nimport App from './App.jsx'\nimport './style.css'\n\nrender(<App />, document.getElementById('root'))\n", "import { useState } from 'preact/hooks'\nimport polyLogo from '../poly-logo.png'\n\nexport default function App() {\n  const [count, setCount] = useState(0)\n  return <main><img className=\"brand-logo\" src={polyLogo} alt=\"Poly logo\" /><p className=\"eyebrow\">POLY × PREACT</p><h1>Your desktop app starts here.</h1><p>Preact, Vite, and Poly are ready.</p><button onClick={() => setCount(count + 1)}>Clicked {count} times</button></main>\n}\n"),
        "vue" => ("@vitejs/plugin-vue", "\"vue\": \"latest\"", "import { createApp } from 'vue'\nimport App from './App.vue'\nimport './style.css'\n\ncreateApp(App).mount('#root')\n", "<script setup>\nimport { ref } from 'vue'\nimport polyLogo from '../poly-logo.png'\nconst count = ref(0)\n</script>\n\n<template>\n  <main><img class=\"brand-logo\" :src=\"polyLogo\" alt=\"Poly logo\" /><p class=\"eyebrow\">POLY × VUE</p><h1>Your desktop app starts here.</h1><p>Vue, Vite, and Poly are ready.</p><button @click=\"count++\">Clicked {{ count }} times</button></main>\n</template>\n"),
        "svelte" => ("@sveltejs/vite-plugin-svelte", "\"svelte\": \"latest\"", "import { mount } from 'svelte'\nimport App from './App.svelte'\nimport './style.css'\n\nmount(App, { target: document.getElementById('root') })\n", "<script>\n  import polyLogo from '../poly-logo.png'\n  let count = $state(0)\n</script>\n\n<main><img class=\"brand-logo\" src={polyLogo} alt=\"Poly logo\" /><p class=\"eyebrow\">POLY × SVELTE</p><h1>Your desktop app starts here.</h1><p>Svelte, Vite, and Poly are ready.</p><button onclick={() => count++}>Clicked {count} times</button></main>\n"),
        _ => return Err(format!("Unsupported frontend framework '{framework}'.")),
    };
    let partner_logo_pair = match framework {
        "react" => r#"<div className="brand-pair"><img className="brand-logo poly-logo" src={polyLogo} alt="Poly logo" /><span className="brand-divider" aria-hidden="true"></span><img className="framework-logo" src={frameworkLogo} alt="React logo" /></div>"#,
        "preact" => r#"<div className="brand-pair"><img className="brand-logo poly-logo" src={polyLogo} alt="Poly logo" /><span className="brand-divider" aria-hidden="true"></span><img className="framework-logo" src={frameworkLogo} alt="Preact logo" /></div>"#,
        "vue" => r#"<div class="brand-pair"><img class="brand-logo poly-logo" :src="polyLogo" alt="Poly logo" /><span class="brand-divider" aria-hidden="true"></span><img class="framework-logo" :src="frameworkLogo" alt="Vue logo" /></div>"#,
        "svelte" => r#"<div class="brand-pair"><img class="brand-logo poly-logo" src={polyLogo} alt="Poly logo" /><span class="brand-divider" aria-hidden="true"></span><img class="framework-logo" src={frameworkLogo} alt="Svelte logo" /></div>"#,
        _ => unreachable!(),
    };
    let framework_eyebrow = match framework {
        "react" => "<p className=\"eyebrow\">POLY × REACT</p>",
        "preact" => "<p className=\"eyebrow\">POLY × PREACT</p>",
        "vue" => "<p class=\"eyebrow\">POLY × VUE</p>",
        "svelte" => "<p class=\"eyebrow\">POLY × SVELTE</p>",
        _ => unreachable!(),
    };
    let component_source = component_source_template
        .replace(
            "import polyLogo from '../poly-logo.png'",
            "import polyLogo from '../poly-logo.png'\nimport frameworkLogo from '../framework-logo.svg'",
        )
        .replace(
            "<img className=\"brand-logo\" src={polyLogo} alt=\"Poly logo\" />",
            partner_logo_pair,
        )
        .replace(
            "<img class=\"brand-logo\" :src=\"polyLogo\" alt=\"Poly logo\" />",
            partner_logo_pair,
        )
        .replace(
            "<img class=\"brand-logo\" src={polyLogo} alt=\"Poly logo\" />",
            partner_logo_pair,
        )
        .replace(framework_eyebrow, "");
    let plugin_import = match framework {
        "react" => "import react from '@vitejs/plugin-react'\n",
        "preact" => "import preact from '@preact/preset-vite'\n",
        "vue" => "import vue from '@vitejs/plugin-vue'\n",
        _ => "import { svelte } from '@sveltejs/vite-plugin-svelte'\n",
    };
    let plugin_call = match framework { "react" => "react()", "preact" => "preact()", "vue" => "vue()", _ => "svelte()" };
    let component_extension = match framework {
        "vue" => "vue",
        "svelte" => "svelte",
        _ => "jsx",
    };
    let main_extension = if matches!(framework, "vue" | "svelte") { "js" } else { "jsx" };
    let npm_name = project_name.to_ascii_lowercase().chars()
        .map(|character| if character.is_ascii_alphanumeric() || matches!(character, '-' | '.') { character } else { '-' })
        .collect::<String>()
        .trim_matches('-').to_string();
    let npm_name = if npm_name.is_empty() { format!("poly-{framework}-app") } else { npm_name };
    let manifest = format!(r#"{{
  "name": "{npm_name}",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {{ "dev": "vite", "build": "vite build", "preview": "vite preview" }},
  "dependencies": {{ {dependencies} }},
  "devDependencies": {{ "{plugin}": "latest", "vite": "latest" }}
}}
"#);
    let vite_config = format!("{plugin_import}import {{ defineConfig }} from 'vite'\n\nexport default defineConfig({{\n  root: 'web',\n  plugins: [{plugin_call}],\n  server: {{\n    host: '127.0.0.1',\n    port: 5173,\n    strictPort: true,\n    proxy: {{\n      '/__poly_invoke': {{ target: process.env.POLY_BACKEND_URL || 'http://127.0.0.1:3000' }},\n      '/__poly_invoke_stream': {{ target: process.env.POLY_BACKEND_URL || 'http://127.0.0.1:3000' }},\n      '/assets': {{ target: process.env.POLY_BACKEND_URL || 'http://127.0.0.1:3000' }}\n    }}\n  }},\n  build: {{ outDir: '../dist/web', emptyOutDir: true }}\n}})\n");
    let html = format!("<!doctype html>\n<html lang=\"en\"><head><meta charset=\"UTF-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"><title>Poly {framework}</title></head><body><div id=\"root\"></div><script type=\"module\" src=\"/src/starter.js\"></script><script type=\"module\" src=\"/src/main.{main_extension}\"></script></body></html>\n");
    let poly_api = r#"export async function invoke(fn, args = {}) {
  const response = await fetch('/__poly_invoke', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ fn, args })
  });
  const payload = await response.json();
  if (!response.ok || payload.error) throw new Error(payload.error || `Poly API failed: ${response.status}`);
  return Object.prototype.hasOwnProperty.call(payload, 'result') ? payload.result : payload;
}

export function sendNativeMessage(message) {
  if (window.chrome?.webview?.postMessage) return window.chrome.webview.postMessage(message);
  if (window.webkit?.messageHandlers?.ipc?.postMessage) return window.webkit.messageHandlers.ipc.postMessage(message);
  throw new Error('Native window controls are only available in a Poly desktop window.');
}

export function getCurrentWindow() {
  return Object.freeze({
    minimize: () => sendNativeMessage('minimize'),
    maximize: () => sendNativeMessage('maximize'),
    restore: () => sendNativeMessage('restore'),
    close: () => sendNativeMessage('close'),
    startDragging: () => sendNativeMessage('drag')
  });
}
"#;
    let css = r#":root { color-scheme: dark; font-family: Inter, ui-sans-serif, system-ui, sans-serif; color: #edf4f7; background: #1d1f21; font-synthesis: none; }
* { box-sizing: border-box; }
body { margin: 0; min-width: 320px; min-height: 100vh; display: grid; place-items: center; background: #1d1f21; }
main { width: min(720px, calc(100vw - 48px)); padding: 48px; text-align: center; }
.brand-pair { display: flex; align-items: center; justify-content: center; gap: 28px; margin: 0 auto 28px; }
.brand-logo { display: block; width: 112px; height: 112px; object-fit: contain; transform: perspective(700px) rotateX(0deg) rotateY(0deg); transform-style: preserve-3d; filter: drop-shadow(0 14px 22px #0008); transition: transform 110ms ease-out; will-change: transform; }
.brand-divider { width: 1px; height: 52px; flex: 0 0 1px; background: #687177; opacity: .65; }
.framework-logo { display: block; width: 88px; height: 88px; object-fit: contain; filter: drop-shadow(0 14px 22px #0008); }
h1 { font-size: clamp(36px, 6vw, 64px); line-height: 1.02; letter-spacing: -.05em; margin: 16px 0; }
main > p { color: #a4bac5; font-size: 17px; }
button { margin-top: 20px; border: 0; border-radius: 10px; padding: 12px 18px; color: #08202a; background: #73d6e7; font-weight: 700; cursor: pointer; }
"#;
    let starter_script = r#"const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
document.addEventListener('pointermove', (event) => {
  const logo = document.querySelector('.poly-logo');
  if (!logo) return;
  const maxTilt = reduceMotion.matches ? 2 : 9;
  const x = (event.clientX / window.innerWidth) * 2 - 1;
  const y = (event.clientY / window.innerHeight) * 2 - 1;
  logo.style.transform = `perspective(700px) rotateX(${-y * maxTilt}deg) rotateY(${x * maxTilt}deg)`;
}, { passive: true });
document.addEventListener('pointerleave', () => {
  const logo = document.querySelector('.poly-logo');
  if (logo) logo.style.transform = 'perspective(700px) rotateX(0deg) rotateY(0deg)';
});
"#;
    let logo_path = project_path.join("web/poly-logo.png");
    if !logo_path.exists() {
        fs::write(&logo_path, poly::templates::DEFAULT_ICON_PNG)
            .map_err(|error| format!("Could not write the Poly logo: {error}"))?;
    }
    let partner_logo = match framework {
        "react" => include_str!("../assets/framework-logos/react.svg"),
        "preact" => include_str!("../assets/framework-logos/preact.svg"),
        "vue" => include_str!("../assets/framework-logos/vue.svg"),
        "svelte" => include_str!("../assets/framework-logos/svelte.svg"),
        _ => unreachable!(),
    };
    let partner_logo_path = project_path.join("web/framework-logo.svg");
    if !partner_logo_path.exists() {
        fs::write(&partner_logo_path, partner_logo)
            .map_err(|error| format!("Could not write the {framework} logo: {error}"))?;
    }
    for (file, contents) in [
        ("package.json", manifest.as_str()),
        ("vite.config.js", vite_config.as_str()),
        ("web/src/poly.js", poly_api),
        ("web/src/starter.js", starter_script),
        ("web/src/main.jsx", main_source),
        ("web/src/style.css", css),
        ("web/src/App.jsx", component_source.as_str()),
    ] {
        if file == "vite.config.js" && has_vite_config(project_path) {
            continue;
        }
        let actual_file = if matches!(framework, "vue" | "svelte") && file == "web/src/main.jsx" {
            format!("web/src/main.{main_extension}")
        } else if matches!(framework, "vue" | "svelte") && file == "web/src/App.jsx" {
            format!("web/src/App.{component_extension}")
        } else { file.to_string() };
        write_if_missing(project_path, &actual_file, contents)?;
    }
    let index = project_path.join("web/index.html");
    if index.exists() {
        let current = fs::read_to_string(&index).map_err(|error| format!("Could not read {}: {error}", index.display()))?;
        let default_html = poly::templates::empty_app_html(project_name);
        if current.trim() != default_html.trim() && current.trim() != html.trim() {
            return Err(format!("{} already contains a custom page. Poly left it untouched; move it aside before applying this framework template.", index.display()));
        }
    }
    fs::write(&index, html).map_err(|error| format!("Could not write {}: {error}", index.display()))?;
    Ok(())
}

pub(super) fn npm_command() -> std::process::Command {
    if cfg!(windows) {
        let mut command = std::process::Command::new("cmd");
        command.args(["/C", "npm"]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        command
    } else {
        std::process::Command::new("npm")
    }
}

/// Run npm as a visible child process so Vite startup diagnostics reach the
/// terminal that launched `poly dev`.
pub(super) fn npm_dev_command() -> std::process::Command {
    if cfg!(windows) {
        #[cfg(windows)]
        use std::os::windows::process::CommandExt;

        let mut command = std::process::Command::new("cmd");
        command.args(["/C", "npm"]);
        // Keep Ctrl+C in Poly so it can terminate npm and the full Vite
        // process tree deliberately, while still inheriting visible output.
        #[cfg(windows)]
        command.creation_flags(0x00000200); // CREATE_NEW_PROCESS_GROUP
        command.stdout(std::process::Stdio::inherit());
        command.stderr(std::process::Stdio::inherit());
        command
    } else {
        let mut command = std::process::Command::new("npm");
        command.stdout(std::process::Stdio::inherit());
        command.stderr(std::process::Stdio::inherit());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        command
    }
}

fn install_frontend_dependencies(project_path: &Path) -> Result<(), String> {
    println!("\n  {}Installing frontend dependencies with npm...{}", DIM, RESET);
    let status = npm_command()
        .arg("install")
        .current_dir(project_path)
        .status()
        .map_err(|error| format!("Could not start npm install: {error}. The project files were created; run npm install in {}.", project_path.display()))?;
    if !status.success() {
        return Err(format!("npm install failed with {status}. The project files were created; run npm install in {} to retry.", project_path.display()));
    }
    Ok(())
}

fn print_next_steps(path: &str, template: &str) {
    println!("\n  {}Next steps:{}", DIM, RESET);
    println!("    cd {path}");
    if matches!(template, "react" | "svelte" | "vue" | "preact") {
        println!("    poly dev          {}# Vite HMR + Poly APIs{}", DIM, RESET);
        println!("    poly build        {}# frontend + desktop app{}", DIM, RESET);
        println!("    poly run --native {}# launch the built app{}", DIM, RESET);
    } else {
        println!("    poly dev");
    }
    println!();
}

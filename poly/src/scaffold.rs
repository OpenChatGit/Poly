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
    initialize_project_files(project_path, name, template, no_install)?;
    println!("\n  {}✓{} Created {} project in {}", GREEN, RESET, template, name);
    print_next_steps(name, template);
    Ok(())
}

pub(super) fn init_project(template: &str, no_install: bool) -> Result<(), String> {
    println!("\n  {}POLY{} v{}\n", CYAN, RESET, VERSION);
    let cwd = std::env::current_dir().map_err(|error| format!("Could not read current directory: {error}"))?;
    let name = cwd.file_name().and_then(|part| part.to_str()).unwrap_or("app");
    initialize_project_files(&cwd, name, template, no_install)?;
    println!("\n  {}✓{} Initialized {} project", GREEN, RESET, template);
    print_next_steps(".", template);
    Ok(())
}

fn initialize_project_files(project_path: &Path, name: &str, template: &str, no_install: bool) -> Result<(), String> {
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

[dev]
# Auto-inject libraries (default: false - you control your dependencies)
# inject_alpine = true
# inject_lucide = true

[dependencies]
"#)).map_err(|error| format!("Could not create poly.toml: {error}"))?;
    } else if is_frontend {
        configure_frontend_web_dir(&poly_toml)?;
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
            scaffold_frontend(project_path, template, name)?;
            if !no_install {
                install_frontend_dependencies(project_path)?;
            }
        }
        _ => return Err(format!("Unknown project template '{template}'.")),
    }
    Ok(())
}

fn configure_frontend_web_dir(poly_toml: &Path) -> Result<(), String> {
    let content = fs::read_to_string(poly_toml).map_err(|error| format!("Could not read poly.toml: {error}"))?;
    let mut lines = Vec::new();
    let mut in_web = false;
    let mut found_web = false;
    let mut found_dir = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if in_web && !found_dir {
                lines.push("dir = \"dist/web\"".to_string());
                found_dir = true;
            }
            in_web = trimmed == "[web]";
            found_web |= in_web;
        }
        if in_web && trimmed.starts_with("dir") && trimmed.contains('=') {
            lines.push("dir = \"dist/web\"".to_string());
            found_dir = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if in_web && !found_dir { lines.push("dir = \"dist/web\"".to_string()); }
    if !found_web { lines.extend(["".into(), "[web]".into(), "dir = \"dist/web\"".into()]); }
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
      '/__poly_invoke_stream': {{ target: process.env.POLY_BACKEND_URL || 'http://127.0.0.1:3000' }}
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

    configure_frontend_web_dir(&poly_toml)?;
    Ok(!has_vite)
}

pub(super) fn has_vite_config(project_path: &Path) -> bool {
    ["vite.config.js", "vite.config.mjs", "vite.config.ts", "vite.config.mts", "vite.config.cjs", "vite.config.cts"]
        .iter()
        .any(|name| project_path.join(name).is_file())
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

fn scaffold_frontend(project_path: &Path, framework: &str, project_name: &str) -> Result<(), String> {
    if project_path.join("package.json").is_file() && !has_vite_config(project_path) {
        return Err("package.json already exists without a Vite config. Poly left it untouched; merge the generated framework setup manually or initialize an empty folder.".to_string());
    }
    let (plugin, dependencies, main_source, component_source) = match framework {
        "react" => ("@vitejs/plugin-react", "\"react\": \"latest\",\n    \"react-dom\": \"latest\"", "import React from 'react'\nimport { createRoot } from 'react-dom/client'\nimport App from './App.jsx'\nimport './style.css'\n\ncreateRoot(document.getElementById('root')).render(<React.StrictMode><App /></React.StrictMode>)\n", "import { useState } from 'react'\nimport polyLogo from '../poly-logo.png'\n\nexport default function App() {\n  const [count, setCount] = useState(0)\n  return <main><img className=\"brand-logo\" src={polyLogo} alt=\"Poly logo\" /><p className=\"eyebrow\">POLY × REACT</p><h1>Your desktop app starts here.</h1><p>React, Vite, and Poly are ready.</p><button onClick={() => setCount(count + 1)}>Clicked {count} times</button></main>\n}\n"),
        "preact" => ("@preact/preset-vite", "\"preact\": \"latest\"", "import { render } from 'preact'\nimport App from './App.jsx'\nimport './style.css'\n\nrender(<App />, document.getElementById('root'))\n", "import { useState } from 'preact/hooks'\nimport polyLogo from '../poly-logo.png'\n\nexport default function App() {\n  const [count, setCount] = useState(0)\n  return <main><img className=\"brand-logo\" src={polyLogo} alt=\"Poly logo\" /><p className=\"eyebrow\">POLY × PREACT</p><h1>Your desktop app starts here.</h1><p>Preact, Vite, and Poly are ready.</p><button onClick={() => setCount(count + 1)}>Clicked {count} times</button></main>\n}\n"),
        "vue" => ("@vitejs/plugin-vue", "\"vue\": \"latest\"", "import { createApp } from 'vue'\nimport App from './App.vue'\nimport './style.css'\n\ncreateApp(App).mount('#root')\n", "<script setup>\nimport { ref } from 'vue'\nimport polyLogo from '../poly-logo.png'\nconst count = ref(0)\n</script>\n\n<template>\n  <main><img class=\"brand-logo\" :src=\"polyLogo\" alt=\"Poly logo\" /><p class=\"eyebrow\">POLY × VUE</p><h1>Your desktop app starts here.</h1><p>Vue, Vite, and Poly are ready.</p><button @click=\"count++\">Clicked {{ count }} times</button></main>\n</template>\n"),
        "svelte" => ("@sveltejs/vite-plugin-svelte", "\"svelte\": \"latest\"", "import { mount } from 'svelte'\nimport App from './App.svelte'\nimport './style.css'\n\nmount(App, { target: document.getElementById('root') })\n", "<script>\n  import polyLogo from '../poly-logo.png'\n  let count = $state(0)\n</script>\n\n<main><img class=\"brand-logo\" src={polyLogo} alt=\"Poly logo\" /><p class=\"eyebrow\">POLY × SVELTE</p><h1>Your desktop app starts here.</h1><p>Svelte, Vite, and Poly are ready.</p><button onclick={() => count++}>Clicked {count} times</button></main>\n"),
        _ => return Err(format!("Unsupported frontend framework '{framework}'.")),
    };
    let plugin_import = match framework {
        "react" => "import react from '@vitejs/plugin-react'\n",
        "preact" => "import preact from '@preact/preset-vite'\n",
        "vue" => "import vue from '@vitejs/plugin-vue'\n",
        _ => "import { svelte } from '@sveltejs/vite-plugin-svelte'\n",
    };
    let plugin_call = match framework { "react" => "react()", "preact" => "preact()", "vue" => "vue()", _ => "svelte()" };
    let extension = if framework == "vue" { "vue" } else if framework == "svelte" { "svelte" } else { "jsx" };
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
    let vite_config = format!("{plugin_import}import {{ defineConfig }} from 'vite'\n\nexport default defineConfig({{\n  root: 'web',\n  plugins: [{plugin_call}],\n  server: {{\n    host: '127.0.0.1',\n    port: 5173,\n    strictPort: true,\n    proxy: {{\n      '/__poly_invoke': {{ target: process.env.POLY_BACKEND_URL || 'http://127.0.0.1:3000' }},\n      '/__poly_invoke_stream': {{ target: process.env.POLY_BACKEND_URL || 'http://127.0.0.1:3000' }}\n    }}\n  }},\n  build: {{ outDir: '../dist/web', emptyOutDir: true }}\n}})\n");
    let html = format!("<!doctype html>\n<html lang=\"en\"><head><meta charset=\"UTF-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"><title>Poly {framework}</title></head><body><div id=\"root\"></div><script type=\"module\" src=\"/poly-dev.js\"></script><script type=\"module\" src=\"/src/main.{extension}\"></script></body></html>\n");
    let dev_bridge = r#"if (import.meta.env.DEV) {
  const poly = window.poly || {}
  poly.invoke ??= async (fn, args = {}) => {
    const response = await fetch('/__poly_invoke', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ fn, args }) })
    const result = await response.json()
    if (result.error) throw new Error(result.error)
    return Object.prototype.hasOwnProperty.call(result, 'result') ? result.result : result
  }
  window.poly = poly
}
"#;
    let css = ":root { font-family: Inter, ui-sans-serif, system-ui, sans-serif; color: #edf4f7; background: #0d141a; font-synthesis: none; }\n* { box-sizing: border-box; } body { margin: 0; min-width: 320px; min-height: 100vh; display: grid; place-items: center; background: radial-gradient(ellipse at top, #183344, #0d141a 65%); } main { width: min(720px, calc(100vw - 48px)); padding: 48px; border: 1px solid #294252; border-radius: 24px; background: #111d25dd; box-shadow: 0 24px 90px #0008; text-align: center; } .brand-logo { display: block; width: 160px; height: 160px; object-fit: contain; margin: 0 auto 28px; } .eyebrow { color: #73d6e7; letter-spacing: .16em; font-size: 12px; font-weight: 700; } h1 { font-size: clamp(36px, 6vw, 64px); line-height: 1.02; letter-spacing: -.05em; margin: 16px 0; } main > p:not(.eyebrow) { color: #a4bac5; font-size: 17px; } button { margin-top: 20px; border: 0; border-radius: 10px; padding: 12px 18px; color: #08202a; background: #73d6e7; font-weight: 700; cursor: pointer; }\n";
    let logo_path = project_path.join("web/poly-logo.png");
    if !logo_path.exists() {
        fs::write(&logo_path, poly::templates::DEFAULT_ICON_PNG)
            .map_err(|error| format!("Could not write the Poly logo: {error}"))?;
    }
    for (file, contents) in [
        ("package.json", manifest.as_str()),
        ("vite.config.js", vite_config.as_str()),
        ("web/poly-dev.js", dev_bridge),
        ("web/src/main.jsx", main_source),
        ("web/src/style.css", css),
        ("web/src/App.jsx", component_source),
    ] {
        if file == "vite.config.js" && has_vite_config(project_path) {
            continue;
        }
        let actual_file = if (framework == "vue" || framework == "svelte") && file == "web/src/App.jsx" {
            format!("web/src/App.{extension}")
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
        command
    } else {
        std::process::Command::new("npm")
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

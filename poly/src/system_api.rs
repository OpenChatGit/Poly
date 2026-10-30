use super::*;

mod database;
mod http;
use self::database::handle_db_request as database_handle_request;
use self::http::{dirs_path as common_dirs_path, handle_http_request as http_handle_request};

/// Handle built-in system APIs
#[allow(unused_variables)]
pub(super) fn handle_system_api(fn_name: &str, args: &serde_json::Value) -> String {
    match fn_name {
        "__poly_cef_available" => serde_json::json!({"result": poly::cef_osr::is_initialized()}).to_string(),
        "__poly_cef_favicon" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::cef_osr::favicon(id) {
                Ok(url) => serde_json::json!({"result": url}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        "__poly_cef_metadata" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::cef_osr::metadata(id) {
                Ok((address, title, favicon)) => serde_json::json!({"result": {"address": address, "title": title, "favicon": favicon}}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        "__poly_cef_create" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("about:blank");
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(800) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(600) as u32;
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let windowed = args.get("windowed").and_then(|v| v.as_bool()).unwrap_or(false);
            match poly::cef_osr::create(id, url, width, height, x, y, windowed) {
                Ok(windowed) => serde_json::json!({"result": {"id": id, "created": true, "windowed": windowed}}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        "__poly_cef_navigate" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("about:blank");
            match poly::cef_osr::navigate(id, url) {
                Ok(()) => serde_json::json!({"result": true}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        "__poly_cef_history" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let direction = args.get("direction").and_then(|v| v.as_str()).unwrap_or("back");
            match poly::cef_osr::history(id, direction) {
                Ok(()) => serde_json::json!({"result": true}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        "__poly_cef_reload" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::cef_osr::reload(id) {
                Ok(()) => serde_json::json!({"result": true}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        "__poly_cef_resize" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
            match poly::cef_osr::resize(id, width, height) {
                Ok(()) => serde_json::json!({"result": true}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        "__poly_cef_bounds" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
            match poly::cef_osr::set_bounds(id, x, y, width, height) {
                Ok(()) => serde_json::json!({"result": true}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        "__poly_cef_visible" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let visible = args.get("visible").and_then(|v| v.as_bool()).unwrap_or(false);
            match poly::cef_osr::set_visible(id, visible) {
                Ok(()) => serde_json::json!({"result": true}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        "__poly_cef_mouse_move" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let leave = args.get("leave").and_then(|v| v.as_bool()).unwrap_or(false);
            match poly::cef_osr::mouse_move(id, x, y, leave) {
                Ok(()) => serde_json::json!({"result": true}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        "__poly_cef_mouse_button" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let down = args.get("down").and_then(|v| v.as_bool()).unwrap_or(false);
            match poly::cef_osr::mouse_button(id, x, y, down) {
                Ok(()) => serde_json::json!({"result": true}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        "__poly_cef_mouse_wheel" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let delta_x = args.get("deltaX").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let delta_y = args.get("deltaY").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            match poly::cef_osr::mouse_wheel(id, x, y, delta_x, delta_y) {
                Ok(()) => serde_json::json!({"result": true}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        "__poly_cef_key" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let key_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("down");
            let key_code = args.get("keyCode").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let character = args.get("character").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
            let modifiers = args.get("modifiers").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            match poly::cef_osr::key(id, key_type, key_code, character, modifiers) {
                Ok(()) => serde_json::json!({"result": true}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        "__poly_cef_destroy" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::cef_osr::destroy(id) {
                Ok(()) => serde_json::json!({"result": true}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_virtual_drive_list" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("C:/");
            match poly::virtual_drive::list_directory(path) {
                Ok(result) => serde_json::json!({"result": result}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_virtual_drive_read_text" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            match poly::virtual_drive::read_text(path) {
                Ok(result) => serde_json::json!({"result": result}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_virtual_drive_write_text" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let contents = args.get("contents").and_then(|v| v.as_str()).unwrap_or("");
            match poly::virtual_drive::write_text(path, contents) {
                Ok(result) => serde_json::json!({"result": result}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_virtual_drive_create_directory" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            match poly::virtual_drive::create_directory(path) {
                Ok(result) => serde_json::json!({"result": result}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_virtual_drive_delete" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            match poly::virtual_drive::delete(path) {
                Ok(result) => serde_json::json!({"result": result}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_ncos_app_prepare_install" => {
            let app_id = args.get("appId").and_then(|value| value.as_str()).unwrap_or("");
            let version_id = args.get("versionId").and_then(|value| value.as_str());
            let result = match version_id {
                Some(version_id) => poly::virtual_drive::prepare_app_install_version(app_id, version_id),
                None => poly::virtual_drive::prepare_app_install(app_id),
            };
            match result {
                Ok(result) => serde_json::json!({"result": result}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_ncos_app_stage_installer" => {
            let app_id = args.get("appId").and_then(|value| value.as_str()).unwrap_or("");
            let source = args.get("sourcePath").and_then(|value| value.as_str()).unwrap_or("");
            let version_id = args.get("versionId").and_then(|value| value.as_str());
            match poly::virtual_drive::stage_app_installer_for(app_id, source, version_id) {
                Ok(result) => serde_json::json!({"result": result}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_ncos_app_install_verify" => {
            let app_id = args.get("appId").and_then(|value| value.as_str()).unwrap_or("");
            let version_id = args.get("versionId").and_then(|value| value.as_str()).unwrap_or("");
            match poly::virtual_drive::verify_app_install_version(app_id, version_id) {
                Ok(result) => serde_json::json!({"result": result}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_ncos_app_install_rollback" => {
            let app_id = args.get("appId").and_then(|value| value.as_str()).unwrap_or("");
            let version_id = args.get("versionId").and_then(|value| value.as_str()).unwrap_or("");
            let installer = args.get("installerPath").and_then(|value| value.as_str()).unwrap_or("");
            match poly::virtual_drive::rollback_app_install_version(app_id, version_id, installer) {
                Ok(result) => serde_json::json!({"result": result}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_ncos_app_install_commit" => {
            let app_id = args.get("appId").and_then(|value| value.as_str()).unwrap_or("");
            let version_id = args.get("versionId").and_then(|value| value.as_str()).unwrap_or("");
            match poly::virtual_drive::commit_app_install_version(app_id, version_id) {
                Ok(result) => serde_json::json!({"result": result}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_ncos_app_uninstall" => {
            let app_id = args.get("appId").and_then(|value| value.as_str()).unwrap_or("");
            match poly::virtual_drive::uninstall_app(app_id) {
                Ok(result) => serde_json::json!({"result": result}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        // File Dialogs
        "__poly_dialog_open" => {
            let title = args.get("title").and_then(|v| v.as_str());
            let filters = parse_filters(args.get("filters"));

            #[cfg(feature = "native")]
            {
                let result = poly::native::dialog_open_file(title, filters);
                serde_json::json!({"result": result}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": null}).to_string()
            }
        }
        "__poly_dialog_open_multiple" => {
            let title = args.get("title").and_then(|v| v.as_str());
            let filters = parse_filters(args.get("filters"));

            #[cfg(feature = "native")]
            {
                let result = poly::native::dialog_open_files(title, filters);
                serde_json::json!({"result": result}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": []}).to_string()
            }
        }
        "__poly_dialog_save" => {
            let title = args.get("title").and_then(|v| v.as_str());
            let default_name = args.get("defaultName").and_then(|v| v.as_str());
            let filters = parse_filters(args.get("filters"));

            #[cfg(feature = "native")]
            {
                let result = poly::native::dialog_save_file(title, default_name, filters);
                serde_json::json!({"result": result}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": null}).to_string()
            }
        }
        "__poly_dialog_folder" => {
            let title = args.get("title").and_then(|v| v.as_str());

            #[cfg(feature = "native")]
            {
                let result = poly::native::dialog_pick_folder(title);
                serde_json::json!({"result": result}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": null}).to_string()
            }
        }
        "__poly_dialog_message" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Message");
            let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
            let level = args.get("level").and_then(|v| v.as_str()).unwrap_or("info");

            #[cfg(feature = "native")]
            {
                let msg_level = match level {
                    "warning" => poly::native::MessageLevel::Warning,
                    "error" => poly::native::MessageLevel::Error,
                    _ => poly::native::MessageLevel::Info,
                };
                poly::native::dialog_message(title, message, msg_level);
                serde_json::json!({"result": true}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": false}).to_string()
            }
        }
        "__poly_dialog_confirm" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Confirm");
            let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("");

            #[cfg(feature = "native")]
            {
                let result = poly::native::dialog_confirm(title, message);
                serde_json::json!({"result": result}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": false}).to_string()
            }
        }
        // File System
        "__poly_fs_read" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if let Err(error) = poly::permissions::checks::fs_read(path) {
                return serde_json::json!({"error": error}).to_string();
            }
            match fs::read_to_string(path) {
                Ok(content) => serde_json::json!({"result": content}).to_string(),
                Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
            }
        }
        "__poly_fs_write" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            if let Err(error) = poly::permissions::checks::fs_write(path) {
                return serde_json::json!({"error": error}).to_string();
            }
            match fs::write(path, content) {
                Ok(_) => serde_json::json!({"result": true}).to_string(),
                Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
            }
        }
        "__poly_fs_exists" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if let Err(error) = poly::permissions::checks::fs_read(path) {
                return serde_json::json!({"error": error}).to_string();
            }
            let exists = Path::new(path).exists();
            serde_json::json!({"result": exists}).to_string()
        }
        "__poly_fs_read_dir" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            if let Err(error) = poly::permissions::checks::fs_read(path) {
                return serde_json::json!({"error": error}).to_string();
            }
            match fs::read_dir(path) {
                Ok(entries) => {
                    let files: Vec<serde_json::Value> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| {
                            let path = e.path();
                            serde_json::json!({
                                "name": e.file_name().to_string_lossy(),
                                "path": path.to_string_lossy(),
                                "isDir": path.is_dir(),
                            })
                        })
                        .collect();
                    serde_json::json!({"result": files}).to_string()
                }
                Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
            }
        }
        // Auto-Updater APIs
        "__poly_updater_check_github" => {
            let repo = args.get("repo").and_then(|v| v.as_str()).unwrap_or("");
            let current_version = args.get("currentVersion").and_then(|v| v.as_str()).unwrap_or("0.0.0");

            #[cfg(feature = "native")]
            {
                match poly::check_github_updates(repo, current_version) {
                    Ok(info) => serde_json::json!({"result": info}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_updater_check_url" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let current_version = args.get("currentVersion").and_then(|v| v.as_str()).unwrap_or("0.0.0");

            #[cfg(feature = "native")]
            {
                match poly::check_custom_updates(url, current_version) {
                    Ok(info) => serde_json::json!({"result": info}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_updater_download" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");

            #[cfg(feature = "native")]
            {
                match poly::download_update(url, None) {
                    Ok(path) => serde_json::json!({"result": path.to_string_lossy()}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_updater_install" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");

            #[cfg(feature = "native")]
            {
                match poly::install_update(Path::new(path)) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        // Clipboard APIs
        "__poly_clipboard_read" => {
            #[cfg(feature = "native")]
            {
                match poly::clipboard::read_text() {
                    Ok(text) => serde_json::json!({"result": text}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_clipboard_write" => {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");

            #[cfg(feature = "native")]
            {
                match poly::clipboard::write_text(text) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_clipboard_clear" => {
            #[cfg(feature = "native")]
            {
                match poly::clipboard::clear() {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        // Multi-Window APIs
        "__poly_window_create" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("New Window");
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(800) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(600) as u32;
            let url = args.get("url").and_then(|v| v.as_str());
            let html = args.get("html").and_then(|v| v.as_str());

            #[cfg(feature = "native")]
            {
                let mut config = poly::window::WindowConfig::new(title)
                    .with_size(width, height);

                if let Some(u) = url {
                    config = config.with_url(u);
                }
                if let Some(h) = html {
                    config = config.with_html(h);
                }

                match poly::window::create_window(config) {
                    Ok(handle) => serde_json::json!({"result": {"id": handle.id}}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_count" => {
            #[cfg(feature = "native")]
            {
                let count = poly::window::window_count();
                serde_json::json!({"result": count}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": 0}).to_string()
            }
        }
        "__poly_window_close" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);

            #[cfg(feature = "native")]
            {
                match poly::window::close_window(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_close_all" => {
            #[cfg(feature = "native")]
            {
                match poly::window::close_all_windows() {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_list" => {
            #[cfg(feature = "native")]
            {
                let ids = poly::window::list_windows();
                serde_json::json!({"result": ids}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": []}).to_string()
            }
        }
        "__poly_window_minimize" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            #[cfg(feature = "native")]
            {
                match poly::window::minimize_window(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_maximize" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            #[cfg(feature = "native")]
            {
                match poly::window::maximize_window(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_restore" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            #[cfg(feature = "native")]
            {
                match poly::window::restore_window(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_show" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            #[cfg(feature = "native")]
            {
                match poly::window::show_window(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_hide" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            #[cfg(feature = "native")]
            {
                match poly::window::hide_window(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_focus" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            #[cfg(feature = "native")]
            {
                match poly::window::focus_window(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_set_title" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("");
            #[cfg(feature = "native")]
            {
                match poly::window::set_window_title(id, title) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_get_size" => {
            #[cfg(feature = "native")]
            {
                let (width, height) = poly::native::get_main_window_size();
                serde_json::json!({"result": {"width": width, "height": height}}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_set_size" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(800) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(600) as u32;
            #[cfg(feature = "native")]
            {
                match poly::window::set_window_size(id, width, height) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_set_position" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            #[cfg(feature = "native")]
            {
                match poly::window::set_window_position(id, x, y) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_set_always_on_top" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let value = args.get("value").and_then(|v| v.as_bool()).unwrap_or(false);
            #[cfg(feature = "native")]
            {
                match poly::window::set_window_always_on_top(id, value) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_set_fullscreen" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let value = args.get("value").and_then(|v| v.as_bool()).unwrap_or(false);
            #[cfg(feature = "native")]
            {
                match poly::window::set_window_fullscreen(id, value) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_navigate" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            #[cfg(feature = "native")]
            {
                match poly::window::navigate_window(id, url) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_load_html" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let html = args.get("html").and_then(|v| v.as_str()).unwrap_or("");
            #[cfg(feature = "native")]
            {
                match poly::window::load_window_html(id, html) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_eval" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let script = args.get("script").and_then(|v| v.as_str()).unwrap_or("");
            #[cfg(feature = "native")]
            {
                match poly::window::eval_window_script(id, script) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_get_state" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            #[cfg(feature = "native")]
            {
                match poly::window::get_window_state(id) {
                    Ok(state) => serde_json::json!({
                        "result": {
                            "id": state.id,
                            "title": state.title,
                            "width": state.width,
                            "height": state.height,
                            "x": state.x,
                            "y": state.y,
                            "isVisible": state.is_visible,
                            "isMinimized": state.is_minimized,
                            "isMaximized": state.is_maximized,
                            "isFullscreen": state.is_fullscreen,
                            "isFocused": state.is_focused
                        }
                    }).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_window_list_states" => {
            #[cfg(feature = "native")]
            {
                let states: Vec<_> = poly::window::list_window_states()
                    .into_iter()
                    .map(|s| serde_json::json!({
                        "id": s.id,
                        "title": s.title,
                        "width": s.width,
                        "height": s.height,
                        "x": s.x,
                        "y": s.y,
                        "isVisible": s.is_visible,
                        "isMinimized": s.is_minimized,
                        "isMaximized": s.is_maximized,
                        "isFullscreen": s.is_fullscreen,
                        "isFocused": s.is_focused
                    }))
                    .collect();
                serde_json::json!({"result": states}).to_string()
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"result": []}).to_string()
            }
        }
        // Notification APIs
        "__poly_notification_show" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Notification");
            let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
            let icon = args.get("icon").and_then(|v| v.as_str());

            #[cfg(feature = "native")]
            {
                match poly::notification::show(title, body, icon) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_notification_show_timeout" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Notification");
            let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
            let timeout = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(5000) as u32;

            #[cfg(feature = "native")]
            {
                match poly::notification::show_with_timeout(title, body, timeout) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        // Deep Link APIs
        "__poly_deeplink_register" => {
            let protocol = args.get("protocol").and_then(|v| v.as_str()).unwrap_or("");
            let app_name = args.get("appName").and_then(|v| v.as_str()).unwrap_or("Poly App");

            if protocol.is_empty() {
                return serde_json::json!({"error": "Protocol name required"}).to_string();
            }

            match poly::deeplink::register_protocol(protocol, app_name) {
                Ok(_) => serde_json::json!({"result": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_deeplink_unregister" => {
            let protocol = args.get("protocol").and_then(|v| v.as_str()).unwrap_or("");

            if protocol.is_empty() {
                return serde_json::json!({"error": "Protocol name required"}).to_string();
            }

            match poly::deeplink::unregister_protocol(protocol) {
                Ok(_) => serde_json::json!({"result": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_deeplink_is_registered" => {
            let protocol = args.get("protocol").and_then(|v| v.as_str()).unwrap_or("");
            let registered = poly::deeplink::is_protocol_registered(protocol);
            serde_json::json!({"result": registered}).to_string()
        }
        "__poly_deeplink_get" => {
            let link = poly::deeplink::get_deep_link();
            serde_json::json!({"result": link}).to_string()
        }
        "__poly_deeplink_has" => {
            let has = poly::deeplink::has_deep_link();
            serde_json::json!({"result": has}).to_string()
        }
        "__poly_tray_is_enabled" => {
            // This is determined by poly.toml config, return false in dev mode
            // In native mode, the actual tray state is managed by the window
            serde_json::json!({"result": false}).to_string()
        }
        // Shell APIs
        "__poly_shell_open" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            eprintln!("[DEBUG] shell.open URL: {}", url);
            #[cfg(target_os = "windows")]
            {
                // Use PowerShell Start-Process which handles URLs with special chars correctly
                use std::os::windows::process::CommandExt;
                let cmd = format!("Start-Process '{}'", url.replace("'", "''"));
                eprintln!("[DEBUG] PowerShell command: {}", cmd);
                let _ = std::process::Command::new("powershell")
                    .args(["-NoProfile", "-Command", &cmd])
                    .creation_flags(0x08000000) // CREATE_NO_WINDOW
                    .spawn();
            }
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open")
                    .arg(url)
                    .spawn();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("xdg-open")
                    .arg(url)
                    .spawn();
            }
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_shell_open_path" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("explorer")
                    .arg(path)
                    .spawn();
            }
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open")
                    .arg(path)
                    .spawn();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("xdg-open")
                    .arg(path)
                    .spawn();
            }
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_shell_open_with" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let app = args.get("app").and_then(|v| v.as_str()).unwrap_or("");
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new(app)
                    .arg(path)
                    .spawn();
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = std::process::Command::new(app)
                    .arg(path)
                    .spawn();
            }
            serde_json::json!({"result": true}).to_string()
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_native_app_launch" => {
            let app_id = args.get("appId").and_then(|value| value.as_str()).unwrap_or("");
            let executable = args.get("executable").and_then(|value| value.as_str()).unwrap_or("");
            let app_args = args.get("args").and_then(|value| value.as_array()).map(|items| {
                items.iter().filter_map(|item| item.as_str().map(str::to_owned)).collect::<Vec<_>>()
            }).unwrap_or_default();
            let capture_output = args.get("captureOutput").and_then(|value| value.as_bool()).unwrap_or(false);
            match native_app::launch(app_id, executable, &app_args, capture_output) {
                Ok(result) => result.to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_native_app_attach" => {
            let app_id = args.get("appId").and_then(|value| value.as_str()).unwrap_or("");
            let chrome = native_app::NativeWindowChrome::from_str(args.get("chrome").and_then(|value| value.as_str()).unwrap_or("ncos"));
            let bounds = native_app::NativeAppBounds {
                x: args.get("x").and_then(|value| value.as_i64()).unwrap_or(0).clamp(i32::MIN as i64, i32::MAX as i64) as i32,
                y: args.get("y").and_then(|value| value.as_i64()).unwrap_or(0).clamp(i32::MIN as i64, i32::MAX as i64) as i32,
                width: args.get("width").and_then(|value| value.as_i64()).unwrap_or(0).clamp(0, i32::MAX as i64) as i32,
                height: args.get("height").and_then(|value| value.as_i64()).unwrap_or(0).clamp(0, i32::MAX as i64) as i32,
            };
            match native_app::attach(app_id, bounds, chrome) {
                Ok(result) => result.to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_native_app_set_bounds" => {
            let app_id = args.get("appId").and_then(|value| value.as_str()).unwrap_or("");
            let bounds = native_app::NativeAppBounds {
                x: args.get("x").and_then(|value| value.as_i64()).unwrap_or(0).clamp(i32::MIN as i64, i32::MAX as i64) as i32,
                y: args.get("y").and_then(|value| value.as_i64()).unwrap_or(0).clamp(i32::MIN as i64, i32::MAX as i64) as i32,
                width: args.get("width").and_then(|value| value.as_i64()).unwrap_or(0).clamp(0, i32::MAX as i64) as i32,
                height: args.get("height").and_then(|value| value.as_i64()).unwrap_or(0).clamp(0, i32::MAX as i64) as i32,
            };
            match native_app::set_bounds(app_id, bounds) {
                Ok(updated) => serde_json::json!({"updated": updated}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_native_app_refresh" => {
            let app_id = args.get("appId").and_then(|value| value.as_str()).unwrap_or("");
            match native_app::refresh(app_id) {
                Ok(attached) => serde_json::json!({"attachedDialogs": attached}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_native_app_set_visible" => {
            let app_id = args.get("appId").and_then(|value| value.as_str()).unwrap_or("");
            let visible = args.get("visible").and_then(|value| value.as_bool()).unwrap_or(false);
            match native_app::set_visible(app_id, visible) {
                Ok(updated) => serde_json::json!({"updated": updated}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_native_app_is_running" => {
            let app_id = args.get("appId").and_then(|value| value.as_str()).unwrap_or("");
            match native_app::is_running(app_id) {
                Ok(running) => serde_json::json!({"running": running, "exitCode": native_app::last_exit_code(app_id)}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(feature = "experimental-virtual-drive")]
        "__poly_native_app_close" => {
            let app_id = args.get("appId").and_then(|value| value.as_str()).unwrap_or("");
            match native_app::close(app_id) {
                Ok(closed) => serde_json::json!({"closed": closed}).to_string(),
                Err(error) => serde_json::json!({"error": error}).to_string(),
            }
        }
        #[cfg(not(feature = "experimental-virtual-drive"))]
        "__poly_virtual_drive_list"
        | "__poly_virtual_drive_read_text"
        | "__poly_virtual_drive_write_text"
        | "__poly_virtual_drive_create_directory"
        | "__poly_virtual_drive_delete"
        | "__poly_ncos_app_prepare_install"
        | "__poly_ncos_app_stage_installer"
        | "__poly_ncos_app_install_verify"
        | "__poly_ncos_app_install_rollback"
        | "__poly_ncos_app_install_commit"
        | "__poly_ncos_app_uninstall"
        | "__poly_native_app_launch"
        | "__poly_native_app_attach"
        | "__poly_native_app_set_bounds"
        | "__poly_native_app_refresh"
        | "__poly_native_app_set_visible"
        | "__poly_native_app_is_running"
        | "__poly_native_app_close" => serde_json::json!({"error": "The NCOS Virtual Drive is experimental. Rebuild with --features experimental-virtual-drive."}).to_string(),
        // App APIs
        "__poly_app_get_version" => {
            serde_json::json!({"result": VERSION}).to_string()
        }
        "__poly_app_get_name" => {
            let name = std::env::current_exe()
                .ok()
                .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
                .unwrap_or_else(|| "Poly App".to_string());
            serde_json::json!({"result": name}).to_string()
        }
        "__poly_app_get_path" => {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("data");
            let path = match name {
                "exe" => std::env::current_exe().ok().map(|p| p.to_string_lossy().to_string()),
                "data" | "appData" => common_dirs_path("data"),
                "config" => common_dirs_path("config"),
                "cache" => common_dirs_path("cache"),
                "temp" => Some(std::env::temp_dir().to_string_lossy().to_string()),
                "home" => common_dirs_path("home"),
                "desktop" => common_dirs_path("desktop"),
                "documents" => common_dirs_path("documents"),
                "downloads" => common_dirs_path("downloads"),
                _ => None,
            };
            match path {
                Some(p) => serde_json::json!({"result": p}).to_string(),
                None => serde_json::json!({"result": null}).to_string(),
            }
        }
        "__poly_app_exit" => {
            let code = args.get("code").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            std::process::exit(code);
        }
        "__poly_app_relaunch" => {
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::process::Command::new(exe)
                    .args(std::env::args().skip(1))
                    .spawn();
            }
            std::process::exit(0);
        }
        // OS APIs
        "__poly_os_platform" => {
            let platform = if cfg!(target_os = "windows") { "windows" }
                else if cfg!(target_os = "macos") { "macos" }
                else if cfg!(target_os = "linux") { "linux" }
                else { "unknown" };
            serde_json::json!({"result": platform}).to_string()
        }
        "__poly_os_arch" => {
            let arch = if cfg!(target_arch = "x86_64") { "x64" }
                else if cfg!(target_arch = "aarch64") { "arm64" }
                else if cfg!(target_arch = "x86") { "x86" }
                else { "unknown" };
            serde_json::json!({"result": arch}).to_string()
        }
        "__poly_os_version" => {
            serde_json::json!({"result": std::env::consts::OS}).to_string()
        }
        "__poly_os_hostname" => {
            #[cfg(target_os = "windows")]
            let hostname = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".to_string());
            #[cfg(not(target_os = "windows"))]
            let hostname = std::env::var("HOSTNAME")
                .or_else(|_| std::env::var("HOST"))
                .unwrap_or_else(|_| "unknown".to_string());
            serde_json::json!({"result": hostname}).to_string()
        }
        "__poly_os_homedir" => {
            let home = common_dirs_path("home").unwrap_or_default();
            serde_json::json!({"result": home}).to_string()
        }
        "__poly_os_tempdir" => {
            let temp = std::env::temp_dir().to_string_lossy().to_string();
            serde_json::json!({"result": temp}).to_string()
        }
        // HTTP API
        "__poly_http_get" | "__poly_http_post" | "__poly_http_put" | "__poly_http_patch" | "__poly_http_delete" | "__poly_http_request" => {
            http_handle_request(fn_name, args)
        }
        // AI/LLM API
        "__poly_ai_check_ollama" => {
            #[cfg(feature = "native")]
            {
                match poly::ai::check_ollama() {
                    Ok(available) => serde_json::json!({"result": available}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_ai_list_models" => {
            #[cfg(feature = "native")]
            {
                match poly::ai::list_ollama_models() {
                    Ok(models) => serde_json::json!({"result": models}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_ai_ollama" => {
            #[cfg(feature = "native")]
            {
                let model = args.get("model").and_then(|v| v.as_str()).unwrap_or("llama3");
                let messages = args.get("messages").and_then(|v| v.as_array());
                let temperature = args.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
                let max_tokens = args.get("maxTokens").and_then(|v| v.as_u64()).map(|v| v as u32);
                let think = args.get("think").and_then(|v| v.as_bool()).unwrap_or(false);
                let tools_json = args.get("tools").and_then(|v| v.as_array());
                let tool_choice = args.get("toolChoice").and_then(|v| v.as_str()).unwrap_or("auto");

                let chat_messages: Vec<poly::ai::ChatMessage> = messages
                    .map(|arr| arr.iter().filter_map(|m| {
                        let role = m.get("role")?.as_str()?;
                        let content = m.get("content")?.as_str()?;
                        Some(poly::ai::ChatMessage {
                            role: match role {
                                "system" => poly::ai::MessageRole::System,
                                "assistant" => poly::ai::MessageRole::Assistant,
                                _ => poly::ai::MessageRole::User,
                            },
                            content: content.to_string(),
                        })
                    }).collect())
                    .unwrap_or_default();

                // Parse tools from JSON
                let tools: Vec<poly::ai::Tool> = tools_json
                    .map(|arr| arr.iter().filter_map(|t| {
                        serde_json::from_value(t.clone()).ok()
                    }).collect())
                    .unwrap_or_default();

                let request = poly::ai::ChatRequest {
                    provider: poly::ai::AiProvider::Ollama,
                    base_url: None,
                    api_key: None,
                    model: model.to_string(),
                    messages: chat_messages,
                    temperature,
                    max_tokens,
                    stream: false,
                    enable_thinking: think,
                    thinking_budget: None,
                    tools,
                    tool_choice: tool_choice.to_string(),
                };

                match poly::ai::chat(&request) {
                    Ok(resp) => serde_json::json!({
                        "result": {
                            "content": resp.content,
                            "thinking": resp.thinking,
                            "model": resp.model,
                            "usage": resp.usage,
                            "toolCalls": resp.tool_calls,
                            "finishReason": resp.finish_reason
                        }
                    }).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_ai_openai" => {
            #[cfg(feature = "native")]
            {
                let model = args.get("model").and_then(|v| v.as_str()).unwrap_or("gpt-4");
                let messages = args.get("messages").and_then(|v| v.as_array());
                let api_key = args.get("apiKey").and_then(|v| v.as_str());
                let temperature = args.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
                let max_tokens = args.get("maxTokens").and_then(|v| v.as_u64()).map(|v| v as u32);
                let tools_json = args.get("tools").and_then(|v| v.as_array());
                let tool_choice = args.get("toolChoice").and_then(|v| v.as_str()).unwrap_or("auto");

                let chat_messages: Vec<poly::ai::ChatMessage> = messages
                    .map(|arr| arr.iter().filter_map(|m| {
                        let role = m.get("role")?.as_str()?;
                        let content = m.get("content")?.as_str()?;
                        Some(poly::ai::ChatMessage {
                            role: match role {
                                "system" => poly::ai::MessageRole::System,
                                "assistant" => poly::ai::MessageRole::Assistant,
                                _ => poly::ai::MessageRole::User,
                            },
                            content: content.to_string(),
                        })
                    }).collect())
                    .unwrap_or_default();

                // Parse tools from JSON
                let tools: Vec<poly::ai::Tool> = tools_json
                    .map(|arr| arr.iter().filter_map(|t| {
                        serde_json::from_value(t.clone()).ok()
                    }).collect())
                    .unwrap_or_default();

                let request = poly::ai::ChatRequest {
                    provider: poly::ai::AiProvider::OpenAI,
                    base_url: None,
                    api_key: api_key.map(|s| s.to_string()),
                    model: model.to_string(),
                    messages: chat_messages,
                    temperature,
                    max_tokens,
                    stream: false,
                    enable_thinking: false,
                    thinking_budget: None,
                    tools,
                    tool_choice: tool_choice.to_string(),
                };

                match poly::ai::chat(&request) {
                    Ok(resp) => serde_json::json!({
                        "result": {
                            "content": resp.content,
                            "thinking": resp.thinking,
                            "model": resp.model,
                            "usage": resp.usage,
                            "toolCalls": resp.tool_calls,
                            "finishReason": resp.finish_reason
                        }
                    }).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_ai_anthropic" => {
            #[cfg(feature = "native")]
            {
                let model = args.get("model").and_then(|v| v.as_str()).unwrap_or("claude-3-5-sonnet-20241022");
                let messages = args.get("messages").and_then(|v| v.as_array());
                let api_key = args.get("apiKey").and_then(|v| v.as_str());
                let temperature = args.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
                let max_tokens = args.get("maxTokens").and_then(|v| v.as_u64()).map(|v| v as u32);
                let enable_thinking = args.get("enableThinking").and_then(|v| v.as_bool()).unwrap_or(false);
                let thinking_budget = args.get("thinkingBudget").and_then(|v| v.as_u64()).map(|v| v as u32);
                let tools_json = args.get("tools").and_then(|v| v.as_array());
                let tool_choice = args.get("toolChoice").and_then(|v| v.as_str()).unwrap_or("auto");

                let chat_messages: Vec<poly::ai::ChatMessage> = messages
                    .map(|arr| arr.iter().filter_map(|m| {
                        let role = m.get("role")?.as_str()?;
                        let content = m.get("content")?.as_str()?;
                        Some(poly::ai::ChatMessage {
                            role: match role {
                                "system" => poly::ai::MessageRole::System,
                                "assistant" => poly::ai::MessageRole::Assistant,
                                _ => poly::ai::MessageRole::User,
                            },
                            content: content.to_string(),
                        })
                    }).collect())
                    .unwrap_or_default();

                // Parse tools from JSON
                let tools: Vec<poly::ai::Tool> = tools_json
                    .map(|arr| arr.iter().filter_map(|t| {
                        serde_json::from_value(t.clone()).ok()
                    }).collect())
                    .unwrap_or_default();

                let request = poly::ai::ChatRequest {
                    provider: poly::ai::AiProvider::Anthropic,
                    base_url: None,
                    api_key: api_key.map(|s| s.to_string()),
                    model: model.to_string(),
                    messages: chat_messages,
                    temperature,
                    max_tokens,
                    stream: false,
                    enable_thinking,
                    thinking_budget,
                    tools,
                    tool_choice: tool_choice.to_string(),
                };

                match poly::ai::chat(&request) {
                    Ok(resp) => serde_json::json!({
                        "result": {
                            "content": resp.content,
                            "thinking": resp.thinking,
                            "model": resp.model,
                            "usage": resp.usage,
                            "toolCalls": resp.tool_calls,
                            "finishReason": resp.finish_reason
                        }
                    }).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_ai_custom" => {
            #[cfg(feature = "native")]
            {
                let base_url = args.get("baseUrl").and_then(|v| v.as_str()).unwrap_or("http://localhost:8080");
                let model = args.get("model").and_then(|v| v.as_str()).unwrap_or("default");
                let messages = args.get("messages").and_then(|v| v.as_array());
                let api_key = args.get("apiKey").and_then(|v| v.as_str());
                let temperature = args.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
                let max_tokens = args.get("maxTokens").and_then(|v| v.as_u64()).map(|v| v as u32);
                let tools_json = args.get("tools").and_then(|v| v.as_array());
                let tool_choice = args.get("toolChoice").and_then(|v| v.as_str()).unwrap_or("auto");

                let chat_messages: Vec<poly::ai::ChatMessage> = messages
                    .map(|arr| arr.iter().filter_map(|m| {
                        let role = m.get("role")?.as_str()?;
                        let content = m.get("content")?.as_str()?;
                        Some(poly::ai::ChatMessage {
                            role: match role {
                                "system" => poly::ai::MessageRole::System,
                                "assistant" => poly::ai::MessageRole::Assistant,
                                _ => poly::ai::MessageRole::User,
                            },
                            content: content.to_string(),
                        })
                    }).collect())
                    .unwrap_or_default();

                // Parse tools from JSON
                let tools: Vec<poly::ai::Tool> = tools_json
                    .map(|arr| arr.iter().filter_map(|t| {
                        serde_json::from_value(t.clone()).ok()
                    }).collect())
                    .unwrap_or_default();

                let request = poly::ai::ChatRequest {
                    provider: poly::ai::AiProvider::Custom,
                    base_url: Some(base_url.to_string()),
                    api_key: api_key.map(|s| s.to_string()),
                    model: model.to_string(),
                    messages: chat_messages,
                    temperature,
                    max_tokens,
                    stream: false,
                    enable_thinking: false,
                    thinking_budget: None,
                    tools,
                    tool_choice: tool_choice.to_string(),
                };

                match poly::ai::chat(&request) {
                    Ok(resp) => serde_json::json!({
                        "result": {
                            "content": resp.content,
                            "thinking": resp.thinking,
                            "model": resp.model,
                            "usage": resp.usage,
                            "toolCalls": resp.tool_calls,
                            "finishReason": resp.finish_reason
                        }
                    }).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_ai_chat" => {
            #[cfg(feature = "native")]
            {
                let provider_str = args.get("provider").and_then(|v| v.as_str()).unwrap_or("ollama");
                let model = args.get("model").and_then(|v| v.as_str()).unwrap_or("llama3");
                let messages = args.get("messages").and_then(|v| v.as_array());
                let base_url = args.get("baseUrl").and_then(|v| v.as_str());
                let api_key = args.get("apiKey").and_then(|v| v.as_str());
                let temperature = args.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
                let max_tokens = args.get("maxTokens").and_then(|v| v.as_u64()).map(|v| v as u32);
                let enable_thinking = args.get("enableThinking").and_then(|v| v.as_bool()).unwrap_or(false);
                let thinking_budget = args.get("thinkingBudget").and_then(|v| v.as_u64()).map(|v| v as u32);
                let tools_json = args.get("tools").and_then(|v| v.as_array());
                let tool_choice = args.get("toolChoice").and_then(|v| v.as_str()).unwrap_or("auto");

                let provider = match provider_str {
                    "openai" => poly::ai::AiProvider::OpenAI,
                    "anthropic" => poly::ai::AiProvider::Anthropic,
                    "custom" => poly::ai::AiProvider::Custom,
                    _ => poly::ai::AiProvider::Ollama,
                };

                let chat_messages: Vec<poly::ai::ChatMessage> = messages
                    .map(|arr| arr.iter().filter_map(|m| {
                        let role = m.get("role")?.as_str()?;
                        let content = m.get("content")?.as_str()?;
                        Some(poly::ai::ChatMessage {
                            role: match role {
                                "system" => poly::ai::MessageRole::System,
                                "assistant" => poly::ai::MessageRole::Assistant,
                                _ => poly::ai::MessageRole::User,
                            },
                            content: content.to_string(),
                        })
                    }).collect())
                    .unwrap_or_default();

                // Parse tools from JSON
                let tools: Vec<poly::ai::Tool> = tools_json
                    .map(|arr| arr.iter().filter_map(|t| {
                        serde_json::from_value(t.clone()).ok()
                    }).collect())
                    .unwrap_or_default();

                let request = poly::ai::ChatRequest {
                    provider,
                    base_url: base_url.map(|s| s.to_string()),
                    api_key: api_key.map(|s| s.to_string()),
                    model: model.to_string(),
                    messages: chat_messages,
                    temperature,
                    max_tokens,
                    stream: false,
                    enable_thinking,
                    thinking_budget,
                    tools,
                    tool_choice: tool_choice.to_string(),
                };

                match poly::ai::chat(&request) {
                    Ok(resp) => serde_json::json!({
                        "result": {
                            "content": resp.content,
                            "thinking": resp.thinking,
                            "model": resp.model,
                            "usage": resp.usage,
                            "provider": resp.provider,
                            "toolCalls": resp.tool_calls,
                            "finishReason": resp.finish_reason
                        }
                    }).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        // AI Streaming API
        "__poly_ai_stream_start" => {
            #[cfg(feature = "native")]
            {
                let model = args.get("model").and_then(|v| v.as_str()).unwrap_or("llama3");
                let messages = args.get("messages").and_then(|v| v.as_array());
                let temperature = args.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
                let think = args.get("think").and_then(|v| v.as_bool()).unwrap_or(false);

                let chat_messages: Vec<poly::ai::ChatMessage> = messages
                    .map(|arr| arr.iter().filter_map(|m| {
                        let role = m.get("role")?.as_str()?;
                        let content = m.get("content")?.as_str()?;
                        Some(poly::ai::ChatMessage {
                            role: match role {
                                "system" => poly::ai::MessageRole::System,
                                "assistant" => poly::ai::MessageRole::Assistant,
                                _ => poly::ai::MessageRole::User,
                            },
                            content: content.to_string(),
                        })
                    }).collect())
                    .unwrap_or_default();

                match poly::ai::stream_start_ollama(model, chat_messages, temperature, think) {
                    Ok(stream_id) => serde_json::json!({"result": {"streamId": stream_id}}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "Native feature not enabled"}).to_string()
            }
        }
        "__poly_ai_stream_poll" => {
            let stream_id = args.get("streamId").and_then(|v| v.as_str()).unwrap_or("");
            match poly::ai::stream_poll(stream_id) {
                Ok((chunks, done)) => serde_json::json!({
                    "result": {
                        "chunks": chunks,
                        "done": done
                    }
                }).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_ai_stream_cancel" => {
            let stream_id = args.get("streamId").and_then(|v| v.as_str()).unwrap_or("");
            match poly::ai::stream_cancel(stream_id) {
                Ok(_) => serde_json::json!({"result": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_ai_stream_list" => {
            let streams = poly::ai::stream_list();
            serde_json::json!({"result": streams}).to_string()
        }
        // Database API
        "__poly_db_open" | "__poly_db_close" | "__poly_db_execute" | "__poly_db_query" | "__poly_db_query_one" => {
            database_handle_request(fn_name, args)
        }
        // Browser API - Tab Management
        "__poly_browser_create_tab" => {
            let url = args.get("url").and_then(|v| v.as_str());
            let id = poly::browser::create_tab(url);
            serde_json::json!({"result": id}).to_string()
        }
        "__poly_browser_close_tab" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let success = poly::browser::close_tab(id);
            serde_json::json!({"result": success}).to_string()
        }
        "__poly_browser_get_tab" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            match poly::browser::get_tab(id) {
                Some(tab) => serde_json::json!({
                    "result": {
                        "id": tab.id,
                        "url": tab.url,
                        "title": tab.title,
                        "canGoBack": tab.can_go_back,
                        "canGoForward": tab.can_go_forward,
                        "isLoading": tab.is_loading
                    }
                }).to_string(),
                None => serde_json::json!({"result": null}).to_string(),
            }
        }
        "__poly_browser_list_tabs" => {
            let tabs: Vec<serde_json::Value> = poly::browser::list_tabs().iter().map(|tab| {
                serde_json::json!({
                    "id": tab.id,
                    "url": tab.url,
                    "title": tab.title,
                    "canGoBack": tab.can_go_back,
                    "canGoForward": tab.can_go_forward,
                    "isLoading": tab.is_loading
                })
            }).collect();
            serde_json::json!({"result": tabs}).to_string()
        }
        "__poly_browser_navigate" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            match poly::browser::navigate(id, url) {
                Ok(_) => serde_json::json!({"result": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_browser_back" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            match poly::browser::go_back(id) {
                Ok(url) => serde_json::json!({"result": url}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_browser_forward" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            match poly::browser::go_forward(id) {
                Ok(url) => serde_json::json!({"result": url}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_browser_set_title" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("");
            poly::browser::set_tab_title(id, title);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_browser_set_loading" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let loading = args.get("loading").and_then(|v| v.as_bool()).unwrap_or(false);
            poly::browser::set_tab_loading(id, loading);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_browser_get_history" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let history = poly::browser::get_history(id);
            serde_json::json!({"result": history}).to_string()
        }
        "__poly_browser_clear_history" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            poly::browser::clear_history(id);
            serde_json::json!({"result": true}).to_string()
        }
        // Browser API - Fetch URL content (bypasses iframe restrictions)
        "__poly_browser_fetch" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");

            // Check permissions - extract domain from URL
            if poly::permissions_enabled() {
                let domain = url.split("://")
                    .nth(1)
                    .and_then(|s| s.split('/').next())
                    .unwrap_or(url);
                let perm = poly::Permission::HttpConnect(poly::permissions::DomainScope::Domain(domain.to_string()));
                if let Err(e) = poly::check_permission(&perm) {
                    return serde_json::json!({"error": e}).to_string();
                }
            }

            match reqwest::blocking::get(url) {
                Ok(response) => {
                    let status = response.status().as_u16();
                    let headers: std::collections::HashMap<String, String> = response.headers()
                        .iter()
                        .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
                        .collect();

                    match response.text() {
                        Ok(body) => serde_json::json!({
                            "result": {
                                "status": status,
                                "headers": headers,
                                "body": body
                            }
                        }).to_string(),
                        Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
                    }
                }
                Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
            }
        }
        // Browser API - Open real WebView window
        "__poly_browser_open_window" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("about:blank");
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Browser");
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(1024) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(768) as u32;

            #[cfg(feature = "native")]
            {
                match poly::browser::open_webview(url, title, width, height) {
                    Ok(id) => serde_json::json!({"result": id}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "WebView requires native feature"}).to_string()
            }
        }
        "__poly_browser_window_navigate" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");

            #[cfg(feature = "native")]
            {
                match poly::browser::webview_navigate(id, url) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "WebView requires native feature"}).to_string()
            }
        }
        "__poly_browser_window_close" => {
            let id = args.get("id").and_then(|v| v.as_u64()).unwrap_or(0);

            #[cfg(feature = "native")]
            {
                match poly::browser::webview_close(id) {
                    Ok(_) => serde_json::json!({"result": true}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            }
            #[cfg(not(feature = "native"))]
            {
                serde_json::json!({"error": "WebView requires native feature"}).to_string()
            }
        }
        // Titlebar API
        "__poly_titlebar_set" => {
            let config = poly::titlebar::TitlebarConfig {
                enabled: args.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
                height: args.get("height").and_then(|v| v.as_u64()).unwrap_or(40) as u32,
                html: args.get("html").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                css: args.get("css").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                js: args.get("js").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                background: args.get("background").and_then(|v| v.as_str()).unwrap_or("#1a1a1f").to_string(),
            };
            poly::titlebar::set_titlebar(config);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_titlebar_get" => {
            let config = poly::titlebar::get_titlebar();
            serde_json::json!({
                "result": {
                    "enabled": config.enabled,
                    "height": config.height,
                    "html": config.html,
                    "css": config.css,
                    "js": config.js,
                    "background": config.background
                }
            }).to_string()
        }
        "__poly_titlebar_set_enabled" => {
            let enabled = args.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
            poly::titlebar::set_enabled(enabled);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_titlebar_set_height" => {
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(40) as u32;
            poly::titlebar::set_height(height);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_titlebar_set_html" => {
            let html = args.get("html").and_then(|v| v.as_str()).unwrap_or("");
            poly::titlebar::set_html(html);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_titlebar_set_css" => {
            let css = args.get("css").and_then(|v| v.as_str()).unwrap_or("");
            poly::titlebar::set_css(css);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_titlebar_set_js" => {
            let js = args.get("js").and_then(|v| v.as_str()).unwrap_or("");
            poly::titlebar::set_js(js);
            serde_json::json!({"result": true}).to_string()
        }
        "__poly_titlebar_navigate" => {
            // This will be handled specially - it navigates the WebView but injects titlebar
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            // Store the URL for the WebView to navigate to
            // The actual navigation happens in the native code
            serde_json::json!({"result": url, "action": "navigate"}).to_string()
        }
        // WebView API - Multi-WebView management
        "__poly_webview_create" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("about:blank");
            let html = args.get("html").and_then(|v| v.as_str()).map(|s| s.to_string());
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(800) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(600) as u32;
            let visible = args.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
            let transparent = args.get("transparent").and_then(|v| v.as_bool()).unwrap_or(false);
            let devtools = args.get("devtools").and_then(|v| v.as_bool()).unwrap_or(false);
            let user_agent = args.get("userAgent").and_then(|v| v.as_str()).map(|s| s.to_string());
            let zoom_level = args.get("zoomLevel").and_then(|v| v.as_f64()).unwrap_or(1.0);
            let autoplay = args.get("autoplay").and_then(|v| v.as_bool()).unwrap_or(true);
            let adblock_enabled = args.get("adblockEnabled").and_then(|v| v.as_bool()).unwrap_or(true);

            // Parse backgroundColor - defaults to Steam dark
            let background_color = if let Some(bg) = args.get("backgroundColor").and_then(|v| v.as_array()) {
                let r = bg.first().and_then(|v| v.as_u64()).unwrap_or(23) as u8;
                let g = bg.get(1).and_then(|v| v.as_u64()).unwrap_or(26) as u8;
                let b = bg.get(2).and_then(|v| v.as_u64()).unwrap_or(33) as u8;
                let a = bg.get(3).and_then(|v| v.as_u64()).unwrap_or(255) as u8;
                (r, g, b, a)
            } else {
                (23, 26, 33, 255)  // Steam dark color
            };

            // Parse initScripts array - scripts that run before page loads
            let init_scripts: Vec<String> = args.get("initScripts")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect())
                .unwrap_or_default();

            let config = poly::webview::WebViewConfig {
                id: id.to_string(),
                url: url.to_string(),
                html,
                bounds: poly::webview::WebViewBounds { x, y, width, height },
                visible,
                transparent,
                devtools,
                user_agent,
                zoom_level,
                autoplay,
                adblock_enabled,
                init_scripts,
                background_color,
            };

            match poly::webview::create(config) {
                Ok(_) => serde_json::json!({"success": true, "id": id}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_navigate" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("about:blank");
            match poly::webview::navigate(id, url) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_load_html" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let html = args.get("html").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::load_html(id, html) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_go_back" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::go_back(id) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_go_forward" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::go_forward(id) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_reload" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::reload(id) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_stop" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::stop(id) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_set_bounds" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(800) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(600) as u32;
            match poly::webview::set_bounds(id, poly::webview::WebViewBounds { x, y, width, height }) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_set_interaction_active" => {
            let active = args.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
            poly::webview_native::set_native_surface_interaction_active(active);
            serde_json::json!({"success": true}).to_string()
        }
        "__poly_webview_get_bounds" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::get_bounds(id) {
                Ok(b) => serde_json::json!({"x": b.x, "y": b.y, "width": b.width, "height": b.height}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_eval" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let script = args.get("script").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::eval(id, script) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_destroy" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::destroy(id) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_list" => {
            let list: Vec<_> = poly::webview::list().iter().map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "url": s.url,
                    "title": s.title,
                    "visible": s.visible,
                    "isLoading": s.is_loading,
                    "canGoBack": s.can_go_back,
                    "canGoForward": s.can_go_forward,
                    "zoomLevel": s.zoom_level,
                    "bounds": { "x": s.bounds.x, "y": s.bounds.y, "width": s.bounds.width, "height": s.bounds.height }
                })
            }).collect();
            serde_json::json!(list).to_string()
        }
        "__poly_webview_get" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::get(id) {
                Some(s) => serde_json::json!({
                    "id": s.id,
                    "url": s.url,
                    "title": s.title,
                    "visible": s.visible,
                    "isLoading": s.is_loading,
                    "canGoBack": s.can_go_back,
                    "canGoForward": s.can_go_forward,
                    "zoomLevel": s.zoom_level,
                    "bounds": { "x": s.bounds.x, "y": s.bounds.y, "width": s.bounds.width, "height": s.bounds.height }
                }).to_string(),
                None => serde_json::json!({"error": format!("WebView '{}' not found", id)}).to_string(),
            }
        }
        "__poly_webview_set_zoom" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let level = args.get("level").and_then(|v| v.as_f64()).unwrap_or(1.0);
            match poly::webview::set_zoom(id, level) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_poll_events" => {
            let events = poly::webview::take_events();
            let json_events: Vec<_> = events.iter().map(|e| {
                match e {
                    poly::webview::WebViewEvent::NavigationStarted { id, url } =>
                        serde_json::json!({"type": "navigate", "id": id, "url": url}),
                    poly::webview::WebViewEvent::NavigationFinished { id, url } =>
                        serde_json::json!({"type": "navigateFinish", "id": id, "url": url}),
                    poly::webview::WebViewEvent::TitleChanged { id, title } =>
                        serde_json::json!({"type": "titleChange", "id": id, "title": title}),
                    poly::webview::WebViewEvent::LoadStarted { id } =>
                        serde_json::json!({"type": "loadStart", "id": id}),
                    poly::webview::WebViewEvent::LoadFinished { id } =>
                        serde_json::json!({"type": "loadFinish", "id": id}),
                    poly::webview::WebViewEvent::NewWindowRequested { id, url, target } =>
                        serde_json::json!({"type": "newWindow", "id": id, "url": url, "target": target}),
                    poly::webview::WebViewEvent::DownloadRequested { id, url, filename } =>
                        serde_json::json!({"type": "download", "id": id, "url": url, "filename": filename}),
                    poly::webview::WebViewEvent::Closed { id } =>
                        serde_json::json!({"type": "close", "id": id}),
                    poly::webview::WebViewEvent::Error { id, error } =>
                        serde_json::json!({"type": "error", "id": id, "error": error}),
                    poly::webview::WebViewEvent::FaviconChanged { id, url } =>
                        serde_json::json!({"type": "favicon", "id": id, "url": url}),
                    poly::webview::WebViewEvent::HistoryChanged { id, can_go_back, can_go_forward } =>
                        serde_json::json!({"type": "historyChange", "id": id, "canGoBack": can_go_back, "canGoForward": can_go_forward}),
                    poly::webview::WebViewEvent::FullscreenRequested { id, enter } =>
                        serde_json::json!({"type": "fullscreen", "id": id, "enter": enter}),
                    poly::webview::WebViewEvent::PermissionRequested { id, permission, origin } =>
                        serde_json::json!({"type": "permission", "id": id, "permission": permission, "origin": origin}),
                }
            }).collect();
            // Wrap in {"result": ...} format for poly.invoke()
            serde_json::json!({"result": json_events}).to_string()
        }
        "__poly_webview_respond_permission" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let permission = args.get("permission").and_then(|v| v.as_str()).unwrap_or("");
            let granted = args.get("granted").and_then(|v| v.as_bool()).unwrap_or(false);
            match poly::webview::respond_to_permission(id, permission, granted) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_set_visible" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let visible = args.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
            match poly::webview::set_visible(id, visible) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_focus" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match poly::webview::focus(id) {
                Ok(_) => serde_json::json!({"success": true}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_webview_set_main_bounds" => {
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(800) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(600) as u32;
            poly::webview::set_main_bounds(poly::webview::WebViewBounds { x, y, width, height });
            serde_json::json!({"success": true}).to_string()
        }
        // Ad Blocking API
        "__poly_adblock_init" => {
            let filter_lists = args.get("filterLists")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            match poly::init_adblock(filter_lists) {
                Ok(_) => serde_json::json!({"result": {"success": true}}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_adblock_should_block" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let source_url = args.get("sourceUrl").and_then(|v| v.as_str()).unwrap_or("");
            let resource_type = args.get("resourceType").and_then(|v| v.as_str()).unwrap_or("other");

            let blocked = poly::should_block(url, source_url, resource_type);
            serde_json::json!({"result": {"blocked": blocked}}).to_string()
        }
        "__poly_adblock_download_easylist" => {
            match poly::download_easylist() {
                Ok(content) => serde_json::json!({"result": {"content": content}}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_adblock_download_easyprivacy" => {
            match poly::download_easyprivacy() {
                Ok(content) => serde_json::json!({"result": {"content": content}}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_adblock_download_pihole" => {
            match poly::download_pihole_blocklist() {
                Ok(content) => serde_json::json!({"result": {"content": content}}).to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        }
        "__poly_adblock_parse_pihole" => {
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let domains = poly::parse_pihole_hosts(content);
            serde_json::json!({"result": {"domains": domains}}).to_string()
        }
        "__poly_adblock_get_stats" => {
            let (blocked, total) = poly::adblock_stats();
            serde_json::json!({"result": {"blocked": blocked, "total": total}}).to_string()
        }
        // MultiView API - Create windows with multiple WebViews
        "__poly_multiview_create" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Poly MultiView").to_string();
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(1024) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(768) as u32;
            let decorations = args.get("decorations").and_then(|v| v.as_bool()).unwrap_or(false);
            let resizable = args.get("resizable").and_then(|v| v.as_bool()).unwrap_or(true);
            let icon_path = args.get("icon").and_then(|v| v.as_str()).map(|s| s.to_string());

            let views_arr = args.get("views").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let views: Vec<poly::multiview::ViewConfig> = views_arr.iter().map(|v| {
                poly::multiview::ViewConfig {
                    id: v.get("id").and_then(|x| x.as_str()).unwrap_or("view").to_string(),
                    url: v.get("url").and_then(|x| x.as_str()).unwrap_or("about:blank").to_string(),
                    html: v.get("html").and_then(|x| x.as_str()).map(|s| s.to_string()),
                    x: v.get("x").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
                    y: v.get("y").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
                    width: v.get("width").and_then(|x| x.as_u64()).unwrap_or(800) as u32,
                    height: v.get("height").and_then(|x| x.as_u64()).unwrap_or(600) as u32,
                    transparent: v.get("transparent").and_then(|x| x.as_bool()).unwrap_or(false),
                    devtools: v.get("devtools").and_then(|x| x.as_bool()).unwrap_or(false),
                }
            }).collect();

            let config = poly::multiview::MultiViewWindowConfig {
                title,
                width,
                height,
                decorations,
                resizable,
                views,
                icon_path,
            };

            let window_id = poly::multiview::create_window(config.clone());

            // In native mode, actually create the window
            #[cfg(feature = "native")]
            {
                if let Err(e) = poly::multiview_native::create_multiview_window(window_id, config) {
                    return serde_json::json!({"error": e}).to_string();
                }
            }

            serde_json::json!({"id": window_id}).to_string()
        }
        "__poly_multiview_navigate" => {
            let window_id = args.get("windowId").and_then(|v| v.as_u64()).unwrap_or(0);
            let view_id = args.get("viewId").and_then(|v| v.as_str()).unwrap_or("");
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");

            poly::multiview::navigate(window_id, view_id, url);

            #[cfg(feature = "native")]
            poly::multiview_native::queue_navigation(window_id, view_id, url);

            serde_json::json!({"success": true}).to_string()
        }
        "__poly_multiview_post_message" => {
            let window_id = args.get("windowId").and_then(|v| v.as_u64()).unwrap_or(0);
            let view_id = args.get("viewId").and_then(|v| v.as_str()).unwrap_or("");
            let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("{}");

            poly::multiview::post_message(window_id, view_id, message);
            serde_json::json!({"success": true}).to_string()
        }
        "__poly_multiview_set_bounds" => {
            let window_id = args.get("windowId").and_then(|v| v.as_u64()).unwrap_or(0);
            let view_id = args.get("viewId").and_then(|v| v.as_str()).unwrap_or("");
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(800) as u32;
            let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(600) as u32;

            poly::multiview::set_view_bounds(window_id, view_id, x, y, width, height);
            serde_json::json!({"success": true}).to_string()
        }
        "__poly_multiview_close" => {
            let window_id = args.get("windowId").and_then(|v| v.as_u64()).unwrap_or(0);
            poly::multiview::close_window(window_id);
            serde_json::json!({"success": true}).to_string()
        }
        "__poly_multiview_list" => {
            let windows = poly::multiview::list_windows();
            let list: Vec<serde_json::Value> = windows.iter().map(|w| {
                serde_json::json!({
                    "id": w.id,
                    "title": w.title,
                    "views": w.views
                })
            }).collect();
            serde_json::json!({"windows": list}).to_string()
        }
        "__poly_multiview_get" => {
            let window_id = args.get("windowId").and_then(|v| v.as_u64()).unwrap_or(0);
            match poly::multiview::get_window(window_id) {
                Some(w) => serde_json::json!({
                    "id": w.id,
                    "title": w.title,
                    "views": w.views
                }).to_string(),
                None => serde_json::json!({"error": "Window not found"}).to_string(),
            }
        }
        _ => serde_json::json!({"error": format!("Unknown system API: {}", fn_name)}).to_string(),
    }
}

/// Parse filter array from JSON: [["Images", ["png", "jpg"]], ["All", ["*"]]]
fn parse_filters(_value: Option<&serde_json::Value>) -> Option<Vec<(&str, &[&str])>> {
    // For simplicity, we don't parse complex filters in this version
    // Users can pass simple filters or none
    None
}

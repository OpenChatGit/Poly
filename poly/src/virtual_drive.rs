//! NCOS virtual C: drive shared by Poly-hosted NCOS apps.

use serde_json::{json, Value};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, PathBuf};
use std::time::UNIX_EPOCH;

const MAX_TEXT_FILE_BYTES: usize = 5 * 1024 * 1024;
pub const MAX_IMPORTED_FILE_BYTES: u64 = 1024 * 1024 * 1024;

/// Platform-neutral logical layout for an installed NCOS app.
/// Each OS backend maps these logical folders to its native app-profile locations.
#[derive(Debug, Clone)]
pub struct NcosAppLayout {
    pub id: String,
    pub virtual_root: String,
    pub virtual_program: String,
    pub virtual_data: String,
    pub program: PathBuf,
    pub data: PathBuf,
    pub profile: PathBuf,
    pub roaming: PathBuf,
    pub local: PathBuf,
    pub program_data: PathBuf,
    pub temp: PathBuf,
}

pub fn validate_app_id(app_id: &str) -> Result<(), String> {
    let valid = !app_id.is_empty()
        && app_id.len() <= 64
        && app_id.as_bytes()[0].is_ascii_alphanumeric()
        && app_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if valid {
        Ok(())
    } else {
        Err("The NCOS app ID is invalid.".to_string())
    }
}

fn copy_package_contents(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), String> {
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        let target = destination.join(entry.file_name());
        if file_type.is_symlink() {
            return Err("App packages cannot contain symbolic links.".to_string());
        }
        if file_type.is_dir() {
            fs::create_dir_all(&target).map_err(|error| error.to_string())?;
            copy_package_contents(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), target).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

/// Install a bundled system app under `C:/System/Apps/<id>`.
pub fn install_bundled_app(
    app_id: &str,
    source: &std::path::Path,
    version: &str,
) -> Result<bool, String> {
    validate_app_id(app_id)?;
    if version.trim().is_empty() || version.len() > 64 {
        return Err("The NCOS app package version is invalid.".to_string());
    }
    let source = source
        .canonicalize()
        .map_err(|error| format!("Bundled app package is unavailable: {error}"))?;
    if !source.is_dir() || !source.join("index.html").is_file() {
        return Err("Bundled app package has no index.html entry point.".to_string());
    }

    let app_root = resolve_path(&format!("C:/System/Apps/{app_id}"), false)?;
    fs::create_dir_all(&app_root).map_err(|error| error.to_string())?;
    let package = app_root.join("Package");
    if package.join("index.html").is_file()
        && fs::read_to_string(package.join(".ncos-version"))
            .ok()
            .as_deref()
            == Some(version)
    {
        return Ok(false);
    }

    let staging = app_root.join("Package.installing");
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
    if let Err(error) = copy_package_contents(&source, &staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    fs::write(staging.join(".ncos-version"), version).map_err(|error| error.to_string())?;
    if package.exists() {
        fs::remove_dir_all(&package).map_err(|error| error.to_string())?;
    }
    fs::rename(&staging, &package).map_err(|error| error.to_string())?;
    Ok(true)
}

/// Resolve a file from an installed app package while retaining virtual-drive path checks.
pub fn installed_app_file(app_id: &str, relative_path: &str) -> Result<PathBuf, String> {
    validate_app_id(app_id)?;
    let relative = relative_path.trim().replace('\\', "/");
    let relative = relative.trim_start_matches('/');
    if !relative.is_empty()
        && relative
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == ".." || part.contains(':'))
    {
        return Err("The app package path is invalid.".to_string());
    }
    let relative = if relative.is_empty() {
        "index.html"
    } else {
        relative
    };
    let path = resolve_path(
        &format!("C:/System/Apps/{app_id}/Package/{relative}"),
        false,
    )?;
    if !path.is_file() {
        return Err("App package file does not exist.".to_string());
    }
    Ok(path)
}

fn drive_root() -> Result<PathBuf, String> {
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .ok_or_else(|| "Windows local application data folder is unavailable.".to_string())?;
    let root = local.join("ProjectInternet").join("NCOS").join("Drive");
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    for folder in [
        "Documents",
        "Downloads",
        "System/Apps",
        "System/Cache",
        "System/Config",
        "System/Drivers",
        "System/Logs",
        "System/Resources",
        "System/Temp",
    ] {
        fs::create_dir_all(root.join(folder)).map_err(|error| error.to_string())?;
    }
    migrate_legacy_app_folders(&root);
    root.canonicalize().map_err(|error| error.to_string())
}

fn migrate_legacy_app_folders(root: &std::path::Path) {
    let legacy_apps = root.join("Apps");
    let system_apps = root.join("System").join("Apps");
    if fs::symlink_metadata(&system_apps).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return;
    }
    let Ok(metadata) = fs::symlink_metadata(&legacy_apps) else {
        return;
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return;
    }
    let Ok(entries) = fs::read_dir(&legacy_apps) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        let Some(app_id) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if validate_app_id(&app_id).is_err() {
            continue;
        }
        let destination = system_apps.join(&app_id);
        if fs::create_dir_all(&destination).is_err() {
            continue;
        }
        let Ok(contents) = fs::read_dir(entry.path()) else {
            continue;
        };
        for child in contents.flatten() {
            let Ok(child_type) = child.file_type() else {
                continue;
            };
            if child_type.is_symlink() {
                continue;
            }
            let name = child.file_name();
            let mut target = destination.join(&name);
            if fs::symlink_metadata(&target).is_ok() {
                let legacy = destination.join("Legacy");
                if fs::symlink_metadata(&legacy)
                    .is_ok_and(|metadata| metadata.file_type().is_symlink())
                    || fs::create_dir_all(&legacy).is_err()
                {
                    continue;
                }
                target = legacy.join(&name);
                let base_name = name.to_string_lossy().into_owned();
                let mut suffix = 1usize;
                while fs::symlink_metadata(&target).is_ok() {
                    target = legacy.join(format!("{base_name}.legacy-{suffix}"));
                    suffix += 1;
                }
            }
            let _ = fs::rename(child.path(), target);
        }
        if fs::read_dir(entry.path())
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false)
        {
            let _ = fs::remove_dir(entry.path());
        }
    }
    if fs::read_dir(&legacy_apps)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false)
    {
        let _ = fs::remove_dir(legacy_apps);
    }
}

fn resolve_path(virtual_path: &str, allow_root: bool) -> Result<PathBuf, String> {
    let root = drive_root()?;
    let mut normalized = virtual_path.trim().replace('\\', "/");
    if normalized.eq_ignore_ascii_case("C:") {
        normalized.clear();
    } else if normalized
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("C:/"))
    {
        normalized.drain(..3);
    }
    normalized = normalized.trim_matches('/').to_string();
    if normalized.is_empty() {
        return if allow_root {
            Ok(root)
        } else {
            Err("A file or directory path is required.".to_string())
        };
    }

    let parts: Vec<&str> = normalized.split('/').collect();
    if parts
        .iter()
        .any(|part| part.is_empty() || *part == "." || *part == ".." || part.contains(':'))
    {
        return Err("The virtual path is invalid.".to_string());
    }

    let mut path = root.clone();
    for part in parts {
        path.push(part);
        if let Ok(metadata) = fs::symlink_metadata(&path) {
            if metadata.file_type().is_symlink() {
                return Err("Virtual-drive links are not supported.".to_string());
            }
        }
    }
    if !path.starts_with(&root)
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err("The path is outside the NCOS virtual drive.".to_string());
    }
    Ok(path)
}

fn is_system_path(virtual_path: &str) -> bool {
    let normalized = virtual_path.trim().replace('\\', "/");
    let relative = if normalized.eq_ignore_ascii_case("C:") {
        ""
    } else if normalized
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("C:/"))
    {
        &normalized[3..]
    } else {
        normalized.trim_start_matches('/')
    };
    relative.eq_ignore_ascii_case("System")
        || relative
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("System/"))
}

fn is_app_data_path(virtual_path: &str) -> bool {
    let normalized = virtual_path.trim().replace('\\', "/");
    let relative = if normalized
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("C:/"))
    {
        &normalized[3..]
    } else {
        normalized.trim_start_matches('/')
    };
    let mut parts = relative.split('/');
    parts
        .next()
        .is_some_and(|part| part.eq_ignore_ascii_case("System"))
        && parts
            .next()
            .is_some_and(|part| part.eq_ignore_ascii_case("Apps"))
        && parts
            .next()
            .is_some_and(|app_id| validate_app_id(app_id).is_ok())
        && parts
            .next()
            .is_some_and(|part| part.eq_ignore_ascii_case("Data"))
}

fn is_managed_system_path(virtual_path: &str) -> bool {
    is_system_path(virtual_path) && !is_app_data_path(virtual_path)
}

/// Resolve a virtual-drive file to its host path, rejecting links and paths outside C:.
pub fn host_file(virtual_path: &str) -> Result<PathBuf, String> {
    let path = resolve_path(virtual_path, false)?;
    if !path.is_file() {
        return Err("File does not exist on the NCOS virtual drive.".to_string());
    }
    let root = drive_root()?;
    let canonical = path.canonicalize().map_err(|error| error.to_string())?;
    if !canonical.starts_with(&root) {
        return Err("The file is outside the NCOS virtual drive.".to_string());
    }
    Ok(canonical)
}

/// Resolve an existing virtual-drive file or directory to its backing host path.
pub fn host_path(virtual_path: &str) -> Result<PathBuf, String> {
    let path = resolve_path(virtual_path, false)?;
    let canonical = path.canonicalize().map_err(|error| error.to_string())?;
    let root = drive_root()?;
    if !canonical.starts_with(&root) {
        return Err("The path is outside the NCOS virtual drive.".to_string());
    }
    Ok(canonical)
}

/// Create and return the standard install/profile folders for an NCOS app.
pub fn app_layout(app_id: &str) -> Result<NcosAppLayout, String> {
    validate_app_id(app_id)?;
    let program = resolve_path(&format!("C:/System/Apps/{app_id}/Program"), true)?;
    let data = resolve_path(&format!("C:/System/Apps/{app_id}/Data"), true)?;
    fs::create_dir_all(&program)
        .map_err(|error| format!("Could not prepare NCOS app program folder: {error}"))?;
    fs::create_dir_all(&data)
        .map_err(|error| format!("Could not prepare NCOS app data folder: {error}"))?;
    let profile = data.join("Profile");
    fs::create_dir_all(&profile)
        .map_err(|error| format!("Could not prepare NCOS app profile folder: {error}"))?;
    for (legacy, current) in [
        (
            data.join("Roaming"),
            profile.join("AppData").join("Roaming"),
        ),
        (data.join("Local"), profile.join("AppData").join("Local")),
    ] {
        if !current.exists()
            && fs::symlink_metadata(&legacy)
                .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
        {
            if let Some(parent) = current.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!("Could not migrate NCOS app profile folder: {error}")
                })?;
            }
            fs::rename(&legacy, &current)
                .map_err(|error| format!("Could not migrate NCOS app profile folder: {error}"))?;
        }
    }
    let profile_desktop = profile.join("Desktop");
    let profile_documents = profile.join("Documents");
    let profile_downloads = profile.join("Downloads");
    let profile_pictures = profile.join("Pictures");
    let profile_music = profile.join("Music");
    let profile_videos = profile.join("Videos");
    let roaming = profile.join("AppData").join("Roaming");
    let local = profile.join("AppData").join("Local");
    let program_data = data.join("ProgramData");
    let temp = data.join("Temp");
    for folder in [
        &profile,
        &roaming,
        &local,
        &program_data,
        &temp,
        &profile_desktop,
        &profile_documents,
        &profile_downloads,
        &profile_pictures,
        &profile_music,
        &profile_videos,
    ] {
        fs::create_dir_all(folder)
            .map_err(|error| format!("Could not prepare NCOS app data folder: {error}"))?;
    }
    let root = drive_root()?;
    let program = program.canonicalize().map_err(|error| error.to_string())?;
    let data = data.canonicalize().map_err(|error| error.to_string())?;
    if !program.starts_with(&root) || !data.starts_with(&root) {
        return Err("Native app folders resolved outside the NCOS virtual drive.".to_string());
    }
    Ok(NcosAppLayout {
        id: app_id.to_string(),
        virtual_root: format!("C:/System/Apps/{app_id}"),
        virtual_program: format!("C:/System/Apps/{app_id}/Program"),
        virtual_data: format!("C:/System/Apps/{app_id}/Data"),
        program,
        data: data.clone(),
        profile: data.join("Profile"),
        roaming: data.join("Profile").join("AppData").join("Roaming"),
        local: data.join("Profile").join("AppData").join("Local"),
        program_data: data.join("ProgramData"),
        temp: data.join("Temp"),
    })
}

/// Prepare an app's persistent NCOS install location for an installer workflow.
/// The host paths are supplied because conventional Windows installers expect physical paths.
pub fn prepare_app_install(app_id: &str) -> Result<Value, String> {
    prepare_app_install_internal(app_id, None)
}

/// Prepare a separate program version folder for an install or repair attempt.
pub fn prepare_app_install_version(app_id: &str, version_id: &str) -> Result<Value, String> {
    validate_app_id(version_id)?;
    prepare_app_install_internal(app_id, Some(version_id))
}

fn prepare_app_install_internal(app_id: &str, version_id: Option<&str>) -> Result<Value, String> {
    let layout = app_layout(app_id)?;
    let downloads = drive_root()?.join("Downloads");
    let install_path = version_id
        .map(|version| layout.program.join("Versions").join(version))
        .unwrap_or_else(|| layout.program.clone());
    if version_id.is_some() && install_path.exists() {
        return Err("This NCOS install staging folder already exists.".to_string());
    }
    fs::create_dir_all(&install_path)
        .map_err(|error| format!("Could not create NCOS install folder: {error}"))?;
    let root = drive_root()?;
    let program_root = layout.program.canonicalize().map_err(|error| error.to_string())?;
    let install_path = install_path.canonicalize().map_err(|error| error.to_string())?;
    if !install_path.starts_with(&program_root) || !install_path.starts_with(&root) {
        return Err("The NCOS install folder resolved outside its app program directory.".to_string());
    }
    let install_virtual_path = version_id
        .map(|version| format!("{}/Versions/{version}", layout.virtual_program))
        .unwrap_or_else(|| layout.virtual_program.clone());
    Ok(ok(Some(json!({
        "appId": layout.id,
        "virtual": {
            "root": layout.virtual_root,
            "program": layout.virtual_program,
            "data": layout.virtual_data,
        },
        "installPath": install_path.to_string_lossy(),
        "installVirtualPath": install_virtual_path,
        "installVersion": version_id,
        "profilePaths": {
            "user": layout.profile.to_string_lossy(),
            "roaming": layout.roaming.to_string_lossy(),
            "local": layout.local.to_string_lossy(),
            "programData": layout.program_data.to_string_lossy(),
            "temp": layout.temp.to_string_lossy(),
            "downloads": downloads.to_string_lossy(),
        },
    }))))
}

/// Copy a user-selected EXE from the NCOS drive into an app's Program folder for setup.
pub fn stage_app_installer(app_id: &str, source_virtual_path: &str) -> Result<Value, String> {
    stage_app_installer_for(app_id, source_virtual_path, None)
}

/// Copy a setup EXE into a transaction-specific staging directory.
pub fn stage_app_installer_for(
    app_id: &str,
    source_virtual_path: &str,
    version_id: Option<&str>,
) -> Result<Value, String> {
    validate_app_id(app_id)?;
    if let Some(version_id) = version_id {
        validate_app_id(version_id)?;
    }
    let source = host_file(source_virtual_path)?;
    if !source
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        return Err(
            "Only Windows EXE installers can be staged in this installer flow.".to_string(),
        );
    }
    let metadata = fs::metadata(&source).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_IMPORTED_FILE_BYTES {
        return Err("Installer EXE exceeds the 1 GiB NCOS file limit.".to_string());
    }
    let layout = app_layout(app_id)?;
    let file_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Installer filename is not valid Unicode.".to_string())?;
    let installer_dir = version_id
        .map(|version| layout.program.join("Installer").join(version))
        .unwrap_or_else(|| layout.program.join("Installer"));
    fs::create_dir_all(&installer_dir).map_err(|error| error.to_string())?;
    let target = installer_dir.join(file_name);
    if target.exists() {
        return Err("An installer with that filename is already staged for this app.".to_string());
    }
    let mut input = fs::File::open(&source).map_err(|error| error.to_string())?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(|error| error.to_string())?;
    if let Err(error) = std::io::copy(&mut input, &mut output) {
        drop(output);
        let _ = fs::remove_file(&target);
        return Err(error.to_string());
    }
    let root = drive_root()?;
    let target = target.canonicalize().map_err(|error| error.to_string())?;
    if !target.starts_with(&layout.program) || !target.starts_with(&root) {
        let _ = fs::remove_file(&target);
        return Err("Staged installer resolved outside its NCOS app folder.".to_string());
    }
    let virtual_path = match version_id {
        Some(version) => format!(
            "{}/Installer/{}/{}",
            layout.virtual_program,
            version,
            file_name.replace('\\', "/")
        ),
        None => format!(
            "{}/Installer/{}",
            layout.virtual_program,
            file_name.replace('\\', "/")
        ),
    };
    Ok(ok(Some(json!({
        "appId": app_id,
        "virtualPath": virtual_path,
        "hostPath": target.to_string_lossy(),
        "fileName": file_name,
        "size": metadata.len(),
    }))))
}

/// Roll back only one failed install attempt, preserving any previously installed app.
pub fn rollback_app_install_version(
    app_id: &str,
    version_id: &str,
    installer_virtual_path: &str,
) -> Result<Value, String> {
    validate_app_id(app_id)?;
    validate_app_id(version_id)?;
    let layout = app_layout(app_id)?;
    let versions_root = layout.program.join("Versions");
    let version_path = versions_root.join(version_id);
    if version_path.exists() {
        let metadata = fs::symlink_metadata(&version_path).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err("Refusing to roll back a non-directory NCOS app version path.".to_string());
        }
        let root = drive_root()?;
        let canonical_versions = versions_root.canonicalize().map_err(|error| error.to_string())?;
        let canonical_version = version_path.canonicalize().map_err(|error| error.to_string())?;
        if !canonical_version.starts_with(&canonical_versions) || !canonical_version.starts_with(&root) {
            return Err("Refusing to roll back a path outside the NCOS app version folder.".to_string());
        }
        fs::remove_dir_all(canonical_version)
            .map_err(|error| format!("Could not remove failed NCOS app version: {error}"))?;
    }
    remove_staged_installer(app_id, version_id, installer_virtual_path)?;
    Ok(json!({ "ok": true, "appId": app_id, "version": version_id, "rolledBack": true }))
}

/// Verify a staged app version has at least one non-empty executable before it
/// can be registered as installed or replace an older version.
fn is_valid_pe_executable(path: &std::path::Path) -> Result<bool, String> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let file_size = file.metadata().map_err(|error| error.to_string())?.len();
    if file_size < 64 {
        return Ok(false);
    }
    let mut dos_header = [0u8; 64];
    file.read_exact(&mut dos_header).map_err(|error| error.to_string())?;
    if &dos_header[..2] != b"MZ" {
        return Ok(false);
    }
    let pe_offset = u32::from_le_bytes(dos_header[60..64].try_into().unwrap()) as u64;
    if pe_offset.checked_add(4).map_or(true, |end| end > file_size) {
        return Ok(false);
    }
    file.seek(SeekFrom::Start(pe_offset)).map_err(|error| error.to_string())?;
    let mut signature = [0u8; 4];
    file.read_exact(&mut signature).map_err(|error| error.to_string())?;
    Ok(&signature == b"PE\0\0")
}

pub fn verify_app_install_version(app_id: &str, version_id: &str) -> Result<Value, String> {
    validate_app_id(app_id)?;
    validate_app_id(version_id)?;
    let layout = app_layout(app_id)?;
    let versions_root = layout.program.join("Versions");
    let current_version = versions_root.join(version_id);
    if !current_version.is_dir() {
        return Err("The NCOS app version folder is missing.".to_string());
    }
    let current_metadata = fs::symlink_metadata(&current_version).map_err(|error| error.to_string())?;
    if current_metadata.file_type().is_symlink() || !current_metadata.is_dir() {
        return Err("The NCOS app version path must be a regular directory.".to_string());
    }

    let drive = drive_root()?;
    let canonical_program = layout.program.canonicalize().map_err(|error| error.to_string())?;
    let canonical_versions = versions_root.canonicalize().map_err(|error| error.to_string())?;
    let canonical_version = current_version.canonicalize().map_err(|error| error.to_string())?;
    if !canonical_versions.starts_with(&canonical_program)
        || !canonical_version.starts_with(&canonical_versions)
        || !canonical_version.starts_with(&drive)
    {
        return Err("The NCOS app version resolved outside its program folder.".to_string());
    }

    let mut pending = vec![canonical_version.clone()];
    let mut executables = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            if kind.is_symlink() {
                return Err("NCOS app version folders cannot contain links.".to_string());
            }
            let path = entry.path();
            if kind.is_dir() {
                let canonical = path.canonicalize().map_err(|error| error.to_string())?;
                if !canonical.starts_with(&canonical_version) || !canonical.starts_with(&drive) {
                    return Err("An NCOS app subfolder resolved outside its version folder.".to_string());
                }
                pending.push(canonical);
                continue;
            }
            if !kind.is_file()
                || !path.extension().and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
            {
                continue;
            }
            let metadata = entry.metadata().map_err(|error| error.to_string())?;
            if metadata.len() == 0 || !is_valid_pe_executable(&path)? {
                continue;
            }
            if app_id.eq_ignore_ascii_case("firefox")
                && !path.file_name().and_then(|name| name.to_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case("firefox.exe"))
            {
                continue;
            }
            let relative = path.strip_prefix(&canonical_version)
                .map_err(|error| error.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            executables.push(json!({
                "path": format!("{}/Versions/{version_id}/{relative}", layout.virtual_program),
                "size": metadata.len(),
            }));
        }
    }
    executables.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
    if executables.is_empty() {
        let expected = if app_id.eq_ignore_ascii_case("firefox") { "firefox.exe" } else { "a non-empty .exe" };
        return Err(format!("The NCOS app version does not contain {expected}."));
    }
    Ok(ok(Some(json!({ "appId": app_id, "version": version_id, "executables": executables }))))
}

/// Commit a verified install version and remove superseded program files only after success.
pub fn commit_app_install_version(app_id: &str, version_id: &str) -> Result<Value, String> {
    verify_app_install_version(app_id, version_id)?;
    validate_app_id(app_id)?;
    validate_app_id(version_id)?;
    let layout = app_layout(app_id)?;
    let versions_root = layout.program.join("Versions");
    let current_version = versions_root.join(version_id);
    if !current_version.is_dir() {
        return Err("The verified NCOS app version folder is missing.".to_string());
    }
    let root = drive_root()?;
    let canonical_program = layout.program.canonicalize().map_err(|error| error.to_string())?;
    let canonical_versions = versions_root.canonicalize().map_err(|error| error.to_string())?;
    let canonical_current = current_version.canonicalize().map_err(|error| error.to_string())?;
    if !canonical_versions.starts_with(&canonical_program)
        || !canonical_current.starts_with(&canonical_versions)
        || !canonical_current.starts_with(&root)
    {
        return Err("Refusing to commit an NCOS app version outside its program folder.".to_string());
    }
    let program_entries = fs::read_dir(&canonical_program)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let mut superseded_paths = Vec::new();
    for entry in program_entries {
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() {
            return Err("NCOS app program folders cannot contain links during commit.".to_string());
        }
        let path = entry.path();
        if entry.file_name().to_string_lossy().eq_ignore_ascii_case("Versions") {
            let versions = fs::read_dir(&canonical_versions)
                .map_err(|error| error.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            for version in versions {
                let version_type = version.file_type().map_err(|error| error.to_string())?;
                if version_type.is_symlink() {
                    return Err("NCOS app version folders cannot contain links during commit.".to_string());
                }
                if version.file_name().to_string_lossy() == version_id {
                    if !version_type.is_dir() {
                        return Err("The committed NCOS app version is not a regular directory.".to_string());
                    }
                    continue;
                }
                superseded_paths.push((version.path(), version_type.is_dir()));
            }
        } else {
            superseded_paths.push((path, file_type.is_dir()));
        }
    }
    for (path, is_directory) in superseded_paths {
        if is_directory {
            fs::remove_dir_all(&path)
                .map_err(|error| format!("Could not remove superseded NCOS app folder: {error}"))?;
        } else {
            fs::remove_file(&path)
                .map_err(|error| format!("Could not remove superseded NCOS app file: {error}"))?;
        }
    }
    Ok(json!({ "ok": true, "appId": app_id, "version": version_id, "committed": true }))
}

fn remove_staged_installer(app_id: &str, version_id: &str, installer_virtual_path: &str) -> Result<(), String> {
    if !installer_virtual_path.trim().is_empty() {
        let expected_prefix = format!("C:/System/Apps/{app_id}/Program/Installer/{version_id}/");
        if !installer_virtual_path.to_ascii_lowercase().starts_with(&expected_prefix.to_ascii_lowercase()) {
            return Err("The staged installer path does not match its NCOS install transaction.".to_string());
        }
        let installer = resolve_path(installer_virtual_path, false)?;
        if installer.is_file() {
            fs::remove_file(&installer).map_err(|error| error.to_string())?;
        }
    }

    let layout = app_layout(app_id)?;
    let installer_root = layout.program.join("Installer");
    let transaction_dir = installer_root.join(version_id);
    if transaction_dir.exists() {
        let root = drive_root()?;
        let canonical_root = installer_root.canonicalize().map_err(|error| error.to_string())?;
        let canonical_transaction = transaction_dir.canonicalize().map_err(|error| error.to_string())?;
        if !canonical_transaction.starts_with(&canonical_root) || !canonical_transaction.starts_with(&root) {
            return Err("Refusing to remove installer staging outside its NCOS transaction folder.".to_string());
        }
        let metadata = fs::symlink_metadata(&transaction_dir).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err("The NCOS installer transaction path is not a regular directory.".to_string());
        }
        fs::remove_dir_all(canonical_transaction)
            .map_err(|error| format!("Could not remove NCOS installer transaction files: {error}"))?;
    }
    Ok(())
}

/// Remove an app's complete NCOS-owned folder after its native process has been closed.
pub fn uninstall_app(app_id: &str) -> Result<Value, String> {
    validate_app_id(app_id)?;
    let virtual_root = format!("C:/System/Apps/{app_id}");
    let app_root = resolve_path(&virtual_root, false)?;
    if !app_root.is_dir() {
        return Err("The NCOS app folder does not exist.".to_string());
    }
    let root = drive_root()?;
    let canonical = app_root.canonicalize().map_err(|error| error.to_string())?;
    let system_apps = root
        .join("System")
        .join("Apps")
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !canonical.starts_with(&system_apps) || canonical == system_apps {
        return Err("Refusing to remove a path outside the NCOS app store.".to_string());
    }
    remove_app_tree(&canonical)
        .map_err(|error| format!("Could not remove NCOS app files: {error}"))?;
    Ok(json!({"ok": true, "appId": app_id, "removed": virtual_root}))
}

#[cfg(all(windows, feature = "native"))]
fn request_locking_app_shutdown(path: &std::path::Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::System::RestartManager::{
        RmEndSession, RmRegisterResources, RmShutdown, RmStartSession,
    };

    let mut session = 0u32;
    let mut key = [0u16; 33];
    let start = unsafe { RmStartSession(&mut session, 0, PWSTR(key.as_mut_ptr())) };
    if start.0 != 0 {
        return false;
    }
    let path_wide = path.as_os_str().encode_wide().chain(Some(0)).collect::<Vec<_>>();
    let resource = [PCWSTR(path_wide.as_ptr())];
    let registered = unsafe { RmRegisterResources(session, Some(&resource), None, None) };
    let closed = registered.0 == 0 && unsafe { RmShutdown(session, 0, None) }.0 == 0;
    let _ = unsafe { RmEndSession(session) };
    closed
}

#[cfg(not(all(windows, feature = "native")))]
fn request_locking_app_shutdown(_path: &std::path::Path) -> bool {
    false
}

fn remove_app_tree(path: &std::path::Path) -> std::io::Result<()> {
    fn remove_entry(path: &std::path::Path) -> std::io::Result<()> {
        let metadata = fs::symlink_metadata(path)?;
        #[cfg(windows)]
        let is_reparse_point = {
            use std::os::windows::fs::MetadataExt;
            metadata.file_type().is_symlink() || (metadata.file_attributes() & 0x400) != 0
        };
        #[cfg(not(windows))]
        let is_reparse_point = metadata.file_type().is_symlink();

        // Windows installers can create junctions (for example INetCache\Content.IE5).
        // Remove the link itself and never recurse into its target.
        if is_reparse_point {
            let result = if metadata.is_dir() || path.is_dir() {
                fs::remove_dir(path)
            } else {
                remove_locked_file(path)
            };
            return result.map_err(|error| path_error(path, error));
        }

        if metadata.is_dir() {
            for entry in fs::read_dir(path).map_err(|error| path_error(path, error))? {
                remove_entry(&entry.map_err(|error| path_error(path, error))?.path())?;
            }
            clear_readonly(path, &metadata)?;
            fs::remove_dir(path).map_err(|error| path_error(path, error))
        } else {
            clear_readonly(path, &metadata)?;
            remove_locked_file(path).map_err(|error| path_error(path, error))
        }
    }

    fn remove_locked_file(path: &std::path::Path) -> std::io::Result<()> {
        let mut shutdown_requested = false;
        let mut last_error = None;
        for attempt in 0..31 {
            match fs::remove_file(path) {
                Ok(()) => return Ok(()),
                Err(error) if is_file_lock_error(&error) => {
                    if !shutdown_requested {
                        let _ = request_locking_app_shutdown(path);
                        shutdown_requested = true;
                    }
                    last_error = Some(error);
                    if attempt < 30 {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
                Err(error) => return Err(error),
            }
        }
        Err(last_error.expect("file removal retry records a lock error"))
    }

    fn is_file_lock_error(error: &std::io::Error) -> bool {
        matches!(error.raw_os_error(), Some(5 | 32 | 33))
            || error.kind() == std::io::ErrorKind::PermissionDenied
    }

    fn clear_readonly(path: &std::path::Path, metadata: &fs::Metadata) -> std::io::Result<()> {
        #[cfg(windows)]
        {
            let mut permissions = metadata.permissions();
            if permissions.readonly() {
                permissions.set_readonly(false);
                fs::set_permissions(path, permissions).map_err(|error| path_error(path, error))?;
            }
        }
        #[cfg(not(windows))]
        let _ = (path, metadata);
        Ok(())
    }

    fn path_error(path: &std::path::Path, error: std::io::Error) -> std::io::Error {
        std::io::Error::new(error.kind(), format!("{}: {error}", path.display()))
    }

    let mut last_error = None;
    for attempt in 0..4 {
        match remove_entry(path) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied && attempt < 3 => {
                last_error = Some(error);
                std::thread::sleep(std::time::Duration::from_millis(100 * (attempt + 1)));
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error.expect("retry loop records a permission error"))
}

/// Prepare an app-specific install directory and return both its NCOS path and host path.
/// Installers must support selecting this directory; this does not redirect arbitrary OS writes.
/// Copy one imported file into a virtual-drive directory without allowing path traversal or overwrite.
pub fn import_file<R: Read>(
    virtual_directory: &str,
    file_name: &str,
    input: R,
) -> Result<Value, String> {
    if is_managed_system_path(virtual_directory) {
        return Err("The NCOS System folder is managed by the operating system.".to_string());
    }
    if file_name.is_empty()
        || file_name.len() > 255
        || file_name == "."
        || file_name == ".."
        || file_name.chars().any(|character| {
            character == '/'
                || character == char::from(92)
                || character == ':'
                || character == '\0'
                || character.is_control()
        })
    {
        return Err("The imported file name is invalid.".to_string());
    }
    let directory = resolve_path(virtual_directory, true)?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    if !directory.is_dir() {
        return Err("Import destination is not a directory.".to_string());
    }
    let root = drive_root()?;
    let canonical_dir = directory
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !canonical_dir.starts_with(&root) {
        return Err("Import destination is outside the NCOS virtual drive.".to_string());
    }
    let target = directory.join(file_name);
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                "A file with that name already exists.".to_string()
            } else {
                error.to_string()
            }
        })?;
    let copied = std::io::copy(&mut input.take(MAX_IMPORTED_FILE_BYTES + 1), &mut output);
    match copied {
        Ok(size) if size <= MAX_IMPORTED_FILE_BYTES => {
            if let Err(error) = output.flush() {
                drop(output);
                let _ = fs::remove_file(&target);
                return Err(error.to_string());
            }
            Ok(
                json!({"ok": true, "data": {"name": file_name, "path": format!("C:/{}", target.strip_prefix(&root).map_err(|error| error.to_string())?.to_string_lossy().replace('\\', "/")), "size": size}}),
            )
        }
        Ok(_) => {
            drop(output);
            let _ = fs::remove_file(target);
            Err("Imported files must be 1 GiB or smaller.".to_string())
        }
        Err(error) => {
            drop(output);
            let _ = fs::remove_file(target);
            Err(error.to_string())
        }
    }
}

fn ok(data: Option<Value>) -> Value {
    match data {
        Some(data) => json!({ "ok": true, "data": data }),
        None => json!({ "ok": true }),
    }
}

pub fn list_directory(virtual_path: &str) -> Result<Value, String> {
    let root = drive_root()?;
    let path = resolve_path(virtual_path, true)?;
    if !path.is_dir() {
        return Err("Directory does not exist.".to_string());
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let metadata = entry.metadata().map_err(|error| error.to_string())?;
        let relative = entry
            .path()
            .strip_prefix(&root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let is_directory = metadata.is_dir();
        let modified_at = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64);
        entries.push(json!({
            "name": entry.file_name().to_string_lossy(),
            "path": format!("C:/{}", relative),
            "isDirectory": is_directory,
            "size": if is_directory { 0 } else { metadata.len() },
            "modifiedAt": modified_at,
        }));
    }
    let normalized_directory = virtual_path
        .trim()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase();
    let directory_priority = |entry: &Value| -> usize {
        let name = entry["name"].as_str().unwrap_or("");
        if normalized_directory.is_empty() || normalized_directory.eq_ignore_ascii_case("c:") {
            ["System", "Documents", "Downloads"]
                .iter()
                .position(|candidate| name.eq_ignore_ascii_case(candidate))
                .unwrap_or(100)
        } else if normalized_directory == "c:/system" {
            [
                "Apps",
                "Config",
                "Drivers",
                "Resources",
                "Logs",
                "Cache",
                "Temp",
            ]
            .iter()
            .position(|candidate| name.eq_ignore_ascii_case(candidate))
            .unwrap_or(100)
        } else {
            100
        }
    };
    entries.sort_by(|left, right| {
        let left_dir = left["isDirectory"].as_bool().unwrap_or(false);
        let right_dir = right["isDirectory"].as_bool().unwrap_or(false);
        right_dir
            .cmp(&left_dir)
            .then_with(|| directory_priority(left).cmp(&directory_priority(right)))
            .then_with(|| {
                left["name"]
                    .as_str()
                    .unwrap_or("")
                    .to_lowercase()
                    .cmp(&right["name"].as_str().unwrap_or("").to_lowercase())
            })
    });
    Ok(ok(Some(Value::Array(entries))))
}

pub fn read_text(virtual_path: &str) -> Result<Value, String> {
    let path = resolve_path(virtual_path, false)?;
    let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_TEXT_FILE_BYTES as u64 {
        return Err("File exceeds the 5 MiB text-file limit.".to_string());
    }
    let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
    Ok(ok(Some(Value::String(contents))))
}

pub fn write_text(virtual_path: &str, contents: &str) -> Result<Value, String> {
    if is_managed_system_path(virtual_path) {
        return Err("The NCOS System folder is managed by the operating system.".to_string());
    }
    let path = resolve_path(virtual_path, false)?;
    if contents.len() > MAX_TEXT_FILE_BYTES {
        return Err("File exceeds the 5 MiB text-file limit.".to_string());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "The virtual path is invalid.".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    fs::write(path, contents).map_err(|error| error.to_string())?;
    Ok(ok(None))
}

pub fn create_directory(virtual_path: &str) -> Result<Value, String> {
    if is_managed_system_path(virtual_path) {
        return Err("The NCOS System folder is managed by the operating system.".to_string());
    }
    let path = resolve_path(virtual_path, false)?;
    fs::create_dir_all(path).map_err(|error| error.to_string())?;
    Ok(ok(None))
}

pub fn delete(virtual_path: &str) -> Result<Value, String> {
    let path = resolve_path(virtual_path, false)?;
    let relative = path
        .strip_prefix(drive_root()?)
        .map_err(|error| error.to_string())?
        .to_string_lossy()
        .replace('\\', "/");
    let protected_system = is_system_path(&relative);
    let removable_system_package = relative
        .to_ascii_lowercase()
        .strip_prefix("system/apps/")
        .and_then(|tail| tail.strip_suffix("/package"))
        .is_some_and(|app_id| validate_app_id(app_id).is_ok());
    let removable_app_data = is_app_data_path(&relative);
    if relative.eq_ignore_ascii_case("Apps")
        || (protected_system && !removable_system_package && !removable_app_data)
    {
        return Err("This virtual-drive system directory cannot be removed.".to_string());
    }
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|error| error.to_string())?;
    } else if path.is_file() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    } else {
        return Err("Path does not exist.".to_string());
    }
    Ok(ok(None))
}

//! Launch and track native desktop applications exposed through NCOS.

use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

static CHILDREN: Lazy<Mutex<HashMap<String, Child>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static ATTACHED_WINDOWS: Lazy<Mutex<HashMap<String, AttachedAppWindows>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static EXIT_CODES: Lazy<Mutex<HashMap<String, Option<i32>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Copy)]
pub struct NativeAppBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy)]
struct AttachedAuxWindow {
    hwnd: isize,
    offset_x: i32,
    offset_y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Clone)]
struct AttachedAppWindows {
    primary: isize,
    auxiliary: Vec<AttachedAuxWindow>,
    bounds: NativeAppBounds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeWindowChrome {
    Ncos,
    App,
}

impl NativeWindowChrome {
    pub fn from_str(value: &str) -> Self {
        if value.eq_ignore_ascii_case("app") {
            Self::App
        } else {
            Self::Ncos
        }
    }
}

fn validate_id(id: &str) -> Result<(), String> {
    if !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        Ok(())
    } else {
        Err("Invalid native app ID.".into())
    }
}

pub fn launch(
    id: &str,
    executable: &str,
    args: &[String],
    capture_output: bool,
) -> Result<Value, String> {
    validate_id(id)?;
    if args.len() > 32
        || args
            .iter()
            .any(|arg| arg.len() > 4096 || arg.contains('\0'))
    {
        return Err("Native app arguments are invalid.".into());
    }
    let virtual_executable = executable.replace('\\', "/");
    let app_program_prefix = format!("C:/System/Apps/{id}/Program/");
    if !virtual_executable
        .to_ascii_lowercase()
        .starts_with(&app_program_prefix.to_ascii_lowercase())
    {
        return Err(
            "Native apps must be launched from their NCOS C:/System/Apps/<app-id>/Program folder."
                .into(),
        );
    }
    let path = poly::virtual_drive::host_file(&virtual_executable)?;
    if !path.is_absolute() || !path.is_file() {
        return Err("The native app executable was not found in the NCOS virtual drive.".into());
    }
    let path = path
        .canonicalize()
        .map_err(|error| format!("Could not resolve app executable: {error}"))?;
    let mut children = CHILDREN
        .lock()
        .map_err(|_| "Native app process registry is unavailable.")?;
    if let Some(child) = children.get_mut(id) {
        if child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_none()
        {
            return Ok(json!({ "started": true, "alreadyRunning": true, "pid": child.id() }));
        }
        children.remove(id);
    }
    let layout = poly::virtual_drive::app_layout(id)?;
    let mut command = Command::new(path);
    let resolved_args = args
        .iter()
        .map(|argument| {
            argument
                .strip_prefix("ncos-drive:")
                .map(poly::virtual_drive::host_path)
                .transpose()
                .map(|resolved| {
                    resolved
                        .map(|path| path.to_string_lossy().into_owned())
                        .unwrap_or_else(|| argument.clone())
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    command.args(&resolved_args).current_dir(&layout.program);
    // Give native apps conventional per-user writable locations inside their NCOS app data.
    // This keeps apps that honor these environment variables from scattering their profile
    // and temporary files into the host user's AppData tree.
    command
        .env("APPDATA", &layout.roaming)
        .env("LOCALAPPDATA", &layout.local)
        .env("USERPROFILE", &layout.profile)
        .env("HOME", &layout.profile)
        .env("PROGRAMDATA", &layout.program_data)
        .env("TEMP", &layout.temp)
        .env("TMP", &layout.temp)
        .env("ProgramFiles", &layout.program)
        .env("ProgramFiles(x86)", &layout.program)
        .env("ProgramW6432", &layout.program)
        .env("NCOS_INSTALL_DIR", &layout.program);
    if capture_output {
        let log_path = layout.temp.join("installer-output.log");
        let log = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&log_path)
            .map_err(|error| format!("Could not create installer diagnostic log: {error}"))?;
        let log_copy = log
            .try_clone()
            .map_err(|error| format!("Could not open installer diagnostic log: {error}"))?;
        command
            .stdout(Stdio::from(log_copy))
            .stderr(Stdio::from(log));
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let child = command
        .spawn()
        .map_err(|error| format!("Could not start native app: {error}"))?;
    let pid = child.id();
    if let Ok(mut exits) = EXIT_CODES.lock() {
        exits.remove(id);
    }
    children.insert(id.to_string(), child);
    drop(children);
    // Setup programs often show a normal Windows top-level window before the
    // JS window manager gets its first bounds-sync frame. Catch and hide that
    // first window here; attach() will adopt the hidden HWND into NCOS.
    #[cfg(target_os = "windows")]
    if capture_output {
        if let Err(error) = suppress_initial_window(id, pid) {
            log_window_diagnostic(id, &format!("initial window suppression failed; root PID={pid}; error={error}"));
        }
    }
    Ok(json!({ "started": true, "alreadyRunning": false, "pid": pid }))
}

pub fn is_running(id: &str) -> Result<bool, String> {
    validate_id(id)?;
    let mut children = CHILDREN
        .lock()
        .map_err(|_| "Native app process registry is unavailable.")?;
    let Some(child) = children.get_mut(id) else {
        return Ok(false);
    };
    if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
        #[cfg(target_os = "windows")]
        if !process_tree(child.id()).is_empty() {
            // Bootstrap installers often hand the UI to a child process and then exit.
            return Ok(true);
        }
        if let Ok(mut exits) = EXIT_CODES.lock() {
            exits.insert(id.to_string(), status.code());
        }
        children.remove(id);
        if let Ok(mut windows) = ATTACHED_WINDOWS.lock() {
            windows.remove(id);
        }
        Ok(false)
    } else {
        Ok(true)
    }
}

pub fn last_exit_code(id: &str) -> Option<i32> {
    EXIT_CODES.lock().ok()?.get(id).copied().flatten()
}

pub fn close(id: &str) -> Result<bool, String> {
    validate_id(id)?;
    let mut children = CHILDREN
        .lock()
        .map_err(|_| "Native app process registry is unavailable.")?;
    let root_pid = children.get(id).map(Child::id);
    #[cfg(target_os = "windows")]
    if let Some(root_pid) = root_pid {
        // Hide hosted windows before terminating their process so Windows
        // cannot expose a detached frame while the Poly host is closing.
        hide_attached_windows(id);
        terminate_process_tree(root_pid)?;
    }
    #[cfg(target_os = "windows")]
    let orphaned_processes_stopped = terminate_app_directory_processes(id)?;
    let child = children.remove(id);
    #[cfg(target_os = "windows")]
    if child.is_none() && !orphaned_processes_stopped {
        return Ok(false);
    }
    #[cfg(not(target_os = "windows"))]
    if child.is_none() {
        return Ok(false);
    }
    #[cfg(target_os = "windows")]
    drop(child);
    if let Ok(mut windows) = ATTACHED_WINDOWS.lock() {
        windows.remove(id);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let Some(child) = child else { return Ok(false) };
        let mut child = child;
        if child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_none()
        {
            child
                .kill()
                .map_err(|error| format!("Could not close native app: {error}"))?;
        }
        let _ = child.wait();
    }
    #[cfg(target_os = "windows")]
    return Ok(root_pid.is_some() || orphaned_processes_stopped);
    #[cfg(not(target_os = "windows"))]
    Ok(true)
}

/// Stop every app process owned by this Poly runtime during application exit.
pub fn shutdown_all() {
    let ids = CHILDREN
        .lock()
        .map(|children| children.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    for id in ids {
        if let Err(error) = close(&id) {
            eprintln!("[Poly] Could not stop native app '{id}' during shutdown: {error}");
        }
    }
}

#[cfg(target_os = "windows")]
fn hide_attached_windows(id: &str) {
    let handles = ATTACHED_WINDOWS
        .lock()
        .ok()
        .and_then(|windows| windows.get(id).cloned())
        .map(|state| {
            std::iter::once(state.primary)
                .chain(state.auxiliary.iter().map(|window| window.hwnd))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for handle in handles {
        let _ = unsafe {
            windows::Win32::UI::WindowsAndMessaging::ShowWindow(
                windows::Win32::Foundation::HWND(handle as *mut _),
                windows::Win32::UI::WindowsAndMessaging::SW_HIDE,
            )
        };
    }
}

#[cfg(not(target_os = "windows"))]
fn hide_attached_windows(_id: &str) {}

/// Reparent an app's existing top-level HWND into the Poly shell while preserving its window styles.
pub fn attach(
    id: &str,
    bounds: NativeAppBounds,
    chrome: NativeWindowChrome,
) -> Result<Value, String> {
    validate_id(id)?;
    if bounds.width < 1 || bounds.height < 1 {
        return Err("Native app bounds must have a positive size.".into());
    }
    let host = poly::native::native_host_window()
        .ok_or_else(|| "The Poly native host window is unavailable.".to_string())?;
    let pid = {
        let mut children = CHILDREN
            .lock()
            .map_err(|_| "Native app process registry is unavailable.")?;
        let Some(child) = children.get_mut(id) else {
            let code = last_exit_code(id)
                .map(|code| format!(" (exit code {code})"))
                .unwrap_or_default();
            return Err(format!("Native app process is not running{code}."));
        };
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            #[cfg(target_os = "windows")]
            if process_tree(child.id()).is_empty() {
                if let Ok(mut exits) = EXIT_CODES.lock() {
                    exits.insert(id.to_string(), status.code());
                }
                children.remove(id);
                let code = status
                    .code()
                    .map(|code| format!(" with exit code {code}"))
                    .unwrap_or_default();
                return Err(format!(
                    "Native app exited before its window was attached{code}."
                ));
            }
            #[cfg(not(target_os = "windows"))]
            {
                if let Ok(mut exits) = EXIT_CODES.lock() {
                    exits.insert(id.to_string(), status.code());
                }
                children.remove(id);
                return Err("Native app exited before its window was attached.".into());
            }
        }
        child.id()
    };

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::{GetLastError, SetLastError, HWND, WIN32_ERROR};
        use windows::Win32::UI::WindowsAndMessaging::{
            GetParent, GetWindowLongPtrW, GetWindowThreadProcessId, IsWindow, SetParent,
            SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, GWL_STYLE, HWND_TOP,
            SWP_FRAMECHANGED, SWP_NOACTIVATE, SW_HIDE, SW_SHOW, WS_CAPTION, WS_CHILD,
            WS_EX_APPWINDOW, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
        };
        let process_ids = process_tree(pid);
        let existing_windows = {
            let windows = ATTACHED_WINDOWS
                .lock()
                .map_err(|_| "Native app window registry is unavailable.")?;
            windows.get(id).cloned()
        };
        if let Some(existing) = existing_windows {
            let raw_hwnd = existing.primary;
            let hwnd = HWND(raw_hwnd as *mut _);
            let mut owner_pid = 0;
            if unsafe { IsWindow(hwnd).as_bool() } {
                unsafe {
                    GetWindowThreadProcessId(hwnd, Some(&mut owner_pid));
                }
            }
            let parent_is_host =
                unsafe { GetParent(hwnd) }.is_ok_and(|parent| parent.0 == host as *mut _);
            if unsafe { IsWindow(hwnd).as_bool() }
                && process_ids.contains(&owner_pid)
                && parent_is_host
            {
                set_bounds(id, bounds)?;
                let auxiliary_attached =
                    refresh_auxiliary_windows(id, &process_ids, HWND(host as *mut _))?;
                return Ok(
                    json!({ "attached": true, "alreadyAttached": true, "pid": pid, "hwnd": raw_hwnd, "auxiliaryWindows": auxiliary_attached }),
                );
            }
            if let Ok(mut windows) = ATTACHED_WINDOWS.lock() {
                windows.remove(id);
            }
        }
        let hwnd = wait_for_main_window(pid, true).map_err(|error| {
            log_window_diagnostic(id, &format!("window search failed; root PID={pid}; process tree={process_ids:?}; host HWND={host}; error={error}"));
            error
        })?;
        let primary_origin = window_rect(hwnd)
            .map_err(|error| format!("Could not read native app window bounds: {error}"))?;
        let initial_auxiliary = find_owned_windows(&process_ids, hwnd)?;
        log_window_diagnostic(id, &format!("selected HWND={:?}; root PID={pid}; process tree={process_ids:?}; host HWND={host}", hwnd.0));
        // Keep the installer from remaining visible as a separate desktop
        // window while we switch it from a top-level HWND to an NCOS child.
        let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
        let extended_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
        let chrome_bits = match chrome {
            NativeWindowChrome::App => 0,
            NativeWindowChrome::Ncos => {
                WS_CAPTION.0 | WS_THICKFRAME.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0
            }
        };
        let child_style = ((style as u32 & !WS_POPUP.0 & !chrome_bits) | WS_CHILD.0) as isize;
        unsafe {
            SetWindowLongPtrW(hwnd, GWL_STYLE, child_style);
            // A former top-level Chromium window must not keep its taskbar button
            // after it becomes a child of the Poly host window.
            SetWindowLongPtrW(
                hwnd,
                GWL_EXSTYLE,
                (extended_style as u32 & !WS_EX_APPWINDOW.0) as isize,
            );
        }
        // SetParent legitimately returns a null previous-parent HWND for an
        // ordinary top-level window. The windows-rs wrapper interprets NULL as
        // failure, so clear stale thread error state first and only treat it as
        // an error if SetParent sets a fresh Win32 error.
        unsafe {
            SetLastError(WIN32_ERROR(0));
        }
        if let Err(error) = unsafe { SetParent(hwnd, HWND(host as *mut _)) } {
            let last_error = unsafe { GetLastError() };
            if last_error.0 != 0 {
                unsafe {
                    SetWindowLongPtrW(hwnd, GWL_STYLE, style);
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, extended_style);
                }
                log_window_diagnostic(id, &format!("SetParent failed for HWND={:?}; host HWND={host}; error={error}; last_error={}", hwnd.0, last_error.0));
                return Err(format!(
                    "Windows could not attach the native app window: {error}"
                ));
            }
        }
        let actual_parent = unsafe { GetParent(hwnd) }
            .map_err(|error| format!("Could not verify the embedded app window parent: {error}"))?;
        if actual_parent.0 != host as *mut _ {
            unsafe {
                SetWindowLongPtrW(hwnd, GWL_STYLE, style);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, extended_style);
            }
            log_window_diagnostic(
                id,
                &format!(
                    "parent verification failed for HWND={:?}; expected={host}; actual={:?}",
                    hwnd.0, actual_parent.0
                ),
            );
            return Err("Windows did not parent the app window to the NCOS shell.".into());
        }
        unsafe {
            SetWindowPos(
                hwnd,
                HWND_TOP,
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height,
                SWP_FRAMECHANGED | SWP_NOACTIVATE,
            )
            .map_err(|error| format!("Could not position native app window: {error}"))?;
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
        ATTACHED_WINDOWS
            .lock()
            .map_err(|_| "Native app window registry is unavailable.")?
            .insert(
                id.to_string(),
                AttachedAppWindows {
                    primary: hwnd.0 as isize,
                    auxiliary: Vec::new(),
                    bounds,
                },
            );
        let attached_auxiliary = attach_auxiliary_candidates(
            id,
            HWND(host as *mut _),
            hwnd,
            primary_origin,
            bounds,
            initial_auxiliary,
        );
        let auxiliary_count = attached_auxiliary.len();
        if let Ok(mut windows) = ATTACHED_WINDOWS.lock() {
            if let Some(state) = windows.get_mut(id) {
                state.auxiliary.extend(attached_auxiliary);
            }
        }
        log_window_diagnostic(id, &format!("attached HWND={:?} to host HWND={host}; bounds={bounds:?}; auxiliary windows={auxiliary_count}", hwnd.0));
        Ok(
            json!({ "attached": true, "pid": pid, "hwnd": hwnd.0 as isize, "auxiliaryWindows": auxiliary_count }),
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (pid, host, bounds, chrome);
        Err("Native HWND embedding is currently available on Windows only.".into())
    }
}

pub fn set_bounds(id: &str, bounds: NativeAppBounds) -> Result<bool, String> {
    validate_id(id)?;
    if bounds.width < 1 || bounds.height < 1 {
        return Err("Native app bounds must have a positive size.".into());
    }
    #[cfg(target_os = "windows")]
    {
        let root_pid = {
            let mut children = CHILDREN
                .lock()
                .map_err(|_| "Native app process registry is unavailable.")?;
            children
                .get_mut(id)
                .ok_or_else(|| "Native app process is not running.".to_string())?
                .id()
        };
        let process_ids = process_tree(root_pid);
        let mut windows = ATTACHED_WINDOWS
            .lock()
            .map_err(|_| "Native app window registry is unavailable.")?;
        let state = windows
            .get_mut(id)
            .ok_or_else(|| "Native app window is not attached.".to_string())?;
        state.bounds = bounds;
        let primary = state.primary;
        let auxiliary = state.auxiliary.clone();
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowThreadProcessId, IsWindow, SetWindowPos, HWND_TOP, SWP_FRAMECHANGED,
            SWP_NOACTIVATE, SWP_NOZORDER,
        };
        let position_flags = SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOZORDER;
        unsafe {
            SetWindowPos(
                HWND(primary as *mut _),
                HWND_TOP,
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height,
                position_flags,
            )
        }
        .map_err(|error| format!("Could not update native app bounds: {error}"))?;
        for child in auxiliary {
            let hwnd = HWND(child.hwnd as *mut _);
            if !unsafe { IsWindow(hwnd).as_bool() } {
                continue;
            }
            let mut owner_pid = 0;
            unsafe {
                GetWindowThreadProcessId(hwnd, Some(&mut owner_pid));
            }
            if !process_ids.contains(&owner_pid) {
                continue;
            }
            unsafe {
                SetWindowPos(
                    hwnd,
                    HWND_TOP,
                    bounds.x + child.offset_x,
                    bounds.y + child.offset_y,
                    child.width,
                    child.height,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                )
            }
            .map_err(|error| format!("Could not update native app dialog bounds: {error}"))?;
        }
        Ok(true)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = bounds;
        Err("Native HWND embedding is currently available on Windows only.".into())
    }
}

pub fn set_visible(id: &str, visible: bool) -> Result<bool, String> {
    validate_id(id)?;
    #[cfg(target_os = "windows")]
    {
        let root_pid = {
            let mut children = CHILDREN
                .lock()
                .map_err(|_| "Native app process registry is unavailable.")?;
            children
                .get_mut(id)
                .ok_or_else(|| "Native app process is not running.".to_string())?
                .id()
        };
        let process_ids = process_tree(root_pid);
        let windows = ATTACHED_WINDOWS
            .lock()
            .map_err(|_| "Native app window registry is unavailable.")?;
        let state = windows
            .get(id)
            .ok_or_else(|| "Native app window is not attached.".to_string())?;
        let mut handles = Vec::with_capacity(1 + state.auxiliary.len());
        handles.push(state.primary);
        handles.extend(state.auxiliary.iter().map(|window| window.hwnd));
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowThreadProcessId, IsWindow, ShowWindow, SW_HIDE, SW_SHOW,
        };
        for handle in handles {
            let hwnd = HWND(handle as *mut _);
            if unsafe { IsWindow(hwnd).as_bool() } {
                let mut owner_pid = 0;
                unsafe {
                    GetWindowThreadProcessId(hwnd, Some(&mut owner_pid));
                }
                if !process_ids.contains(&owner_pid) {
                    continue;
                }
                unsafe {
                    let _ = ShowWindow(hwnd, if visible { SW_SHOW } else { SW_HIDE });
                }
            }
        }
        Ok(true)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = visible;
        Err("Native HWND embedding is currently available on Windows only.".into())
    }
}

/// Attach newly created owned dialogs for an already hosted native app.
pub fn refresh(id: &str) -> Result<usize, String> {
    validate_id(id)?;
    #[cfg(target_os = "windows")]
    {
        let pid = {
            let mut children = CHILDREN
                .lock()
                .map_err(|_| "Native app process registry is unavailable.")?;
            let child = children
                .get_mut(id)
                .ok_or_else(|| "Native app process is not running.".to_string())?;
            child.id()
        };
        let host = poly::native::native_host_window()
            .ok_or_else(|| "The Poly native host window is unavailable.".to_string())?;
        let process_ids = process_tree(pid);
        refresh_auxiliary_windows(
            id,
            &process_ids,
            windows::Win32::Foundation::HWND(host as *mut _),
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Native HWND embedding is currently available on Windows only.".into())
    }
}

#[cfg(target_os = "windows")]
fn refresh_auxiliary_windows(
    id: &str,
    process_ids: &[u32],
    host: windows::Win32::Foundation::HWND,
) -> Result<usize, String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GetParent, GetWindowThreadProcessId, IsWindow};
    let state = {
        let mut windows = ATTACHED_WINDOWS
            .lock()
            .map_err(|_| "Native app window registry is unavailable.")?;
        let current = windows
            .get_mut(id)
            .ok_or_else(|| "Native app window is not attached.".to_string())?;
        current.auxiliary.retain(|item| {
            let hwnd = HWND(item.hwnd as *mut _);
            if !unsafe { IsWindow(hwnd).as_bool() } {
                return false;
            }
            let mut owner_pid = 0;
            unsafe {
                GetWindowThreadProcessId(hwnd, Some(&mut owner_pid));
            }
            process_ids.contains(&owner_pid)
                && unsafe { GetParent(hwnd) }.is_ok_and(|parent| parent.0 == host.0)
        });
        current.clone()
    };
    let primary = HWND(state.primary as *mut _);
    if !unsafe { IsWindow(primary).as_bool() } {
        return Err("The native app's main window is no longer available.".into());
    }
    let origin = window_rect(primary)?;
    let candidates = find_owned_windows(process_ids, primary)?;
    let existing = state
        .auxiliary
        .iter()
        .map(|item| item.hwnd)
        .collect::<Vec<_>>();
    let newly_attached =
        attach_auxiliary_candidates(id, host, primary, origin, state.bounds, candidates)
            .into_iter()
            .filter(|item| !existing.contains(&item.hwnd))
            .collect::<Vec<_>>();
    let count = newly_attached.len();
    if count > 0 {
        let mut windows = ATTACHED_WINDOWS
            .lock()
            .map_err(|_| "Native app window registry is unavailable.")?;
        if let Some(current) = windows.get_mut(id) {
            for item in newly_attached {
                if !current
                    .auxiliary
                    .iter()
                    .any(|existing| existing.hwnd == item.hwnd)
                {
                    current.auxiliary.push(item);
                }
            }
        }
        log_window_diagnostic(
            id,
            &format!("attached {count} newly opened owned app dialog(s)"),
        );
    }
    Ok(count)
}

#[cfg(target_os = "windows")]
fn window_rect(
    hwnd: windows::Win32::Foundation::HWND,
) -> Result<windows::Win32::Foundation::RECT, String> {
    let mut rect = windows::Win32::Foundation::RECT::default();
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect) }
        .map_err(|error| format!("Could not read native window bounds: {error}"))?;
    Ok(rect)
}

#[cfg(target_os = "windows")]
fn attach_auxiliary_candidates(
    app_id: &str,
    host: windows::Win32::Foundation::HWND,
    _primary: windows::Win32::Foundation::HWND,
    origin: windows::Win32::Foundation::RECT,
    bounds: NativeAppBounds,
    candidates: Vec<(
        windows::Win32::Foundation::HWND,
        windows::Win32::Foundation::RECT,
    )>,
) -> Vec<AttachedAuxWindow> {
    use windows::Win32::Foundation::{GetLastError, SetLastError, HWND, WIN32_ERROR};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetParent, GetWindowLongPtrW, IsWindow, SetParent, SetWindowLongPtrW, SetWindowPos,
        ShowWindow, GWL_STYLE, HWND_TOP, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE,
        SW_SHOW, WS_CHILD, WS_POPUP,
    };
    let mut attached = Vec::new();
    for (hwnd, rect) in candidates {
        if !unsafe { IsWindow(hwnd).as_bool() } {
            continue;
        }
        let already_hosted = unsafe { GetParent(hwnd) }.is_ok_and(|parent| parent.0 == host.0);
        if already_hosted {
            continue;
        }
        let width = (rect.right - rect.left).max(1);
        let height = (rect.bottom - rect.top).max(1);
        let offset_x = rect.left - origin.left;
        let offset_y = rect.top - origin.top;
        let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
        let child_style = ((style as u32 & !WS_POPUP.0) | WS_CHILD.0) as isize;
        unsafe {
            SetWindowLongPtrW(hwnd, GWL_STYLE, child_style);
        }
        unsafe {
            SetLastError(WIN32_ERROR(0));
        }
        if let Err(error) = unsafe { SetParent(hwnd, HWND(host.0)) } {
            let last_error = unsafe { GetLastError() };
            if last_error.0 != 0 {
                unsafe {
                    SetWindowLongPtrW(hwnd, GWL_STYLE, style);
                    let _ = ShowWindow(hwnd, SW_SHOW);
                }
                log_window_diagnostic(
                    app_id,
                    &format!(
                        "could not attach owned dialog HWND={:?}: {error}; last_error={}",
                        hwnd.0, last_error.0
                    ),
                );
                continue;
            }
        }
        let actual_parent = unsafe { GetParent(hwnd) };
        if !actual_parent.is_ok_and(|parent| parent.0 == host.0) {
            unsafe {
                SetWindowLongPtrW(hwnd, GWL_STYLE, style);
                let _ = ShowWindow(hwnd, SW_SHOW);
            }
            log_window_diagnostic(
                app_id,
                &format!(
                    "owned dialog HWND={:?} did not attach to the Poly host",
                    hwnd.0
                ),
            );
            continue;
        }
        if let Err(error) = unsafe {
            SetWindowPos(
                hwnd,
                HWND_TOP,
                bounds.x + offset_x,
                bounds.y + offset_y,
                width,
                height,
                SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            )
        } {
            log_window_diagnostic(
                app_id,
                &format!("could not position owned dialog HWND={:?}: {error}", hwnd.0),
            );
            continue;
        }
        let _ = unsafe { ShowWindow(hwnd, SW_SHOW) };
        attached.push(AttachedAuxWindow {
            hwnd: hwnd.0 as isize,
            offset_x,
            offset_y,
            width,
            height,
        });
    }
    attached
}

#[cfg(target_os = "windows")]
fn wait_for_main_window(
    pid: u32,
    visible_only: bool,
) -> Result<windows::Win32::Foundation::HWND, String> {
    use std::time::{Duration, Instant};
    use windows::Win32::UI::WindowsAndMessaging::IsWindow;
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut previous = None;
    let mut stable_since = Instant::now();
    while Instant::now() < deadline {
        let process_ids = process_tree(pid);
        if let Some(hwnd) = find_main_window(&process_ids, visible_only)? {
            let current = hwnd.0 as isize;
            if previous == Some(current)
                && stable_since.elapsed() >= Duration::from_millis(300)
                && unsafe { IsWindow(hwnd).as_bool() }
            {
                return Ok(hwnd);
            }
            if previous != Some(current) {
                previous = Some(current);
                stable_since = Instant::now();
            }
        } else {
            previous = None;
            stable_since = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    Err("The app did not expose a visible top-level window within 8 seconds.".into())
}

#[cfg(target_os = "windows")]
fn suppress_initial_window(app_id: &str, pid: u32) -> Result<(), String> {
    use std::time::{Duration, Instant};
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        let process_ids = process_tree(pid);
        if let Some(hwnd) = find_main_window(&process_ids, true)? {
            let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
            log_window_diagnostic(
                app_id,
                &format!("suppressed initial top-level window HWND={:?}; root PID={pid}; process tree={process_ids:?}", hwnd.0),
            );
            return Ok(());
        }
        if process_ids.is_empty() {
            return Err("The installer process exited before creating a window.".into());
        }
        std::thread::sleep(Duration::from_millis(12));
    }
    // The NCOS attach path may already have reparented the window while this
    // best-effort watcher was polling, or the installer may legitimately have
    // no GUI. In either case the normal attach/status path owns the final result.
    Ok(())
}

#[cfg(target_os = "windows")]
fn process_tree(root_pid: u32) -> Vec<u32> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    let snapshot = match unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) } {
        Ok(snapshot) => snapshot,
        Err(_) => return Vec::new(),
    };
    let mut processes = Vec::new();
    let mut entry = PROCESSENTRY32W::default();
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    unsafe {
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                processes.push((entry.th32ProcessID, entry.th32ParentProcessID));
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }
    let mut tree = vec![root_pid];
    loop {
        let previous_len = tree.len();
        for (pid, parent_pid) in &processes {
            if tree.contains(parent_pid) && !tree.contains(pid) {
                tree.push(*pid);
            }
        }
        if tree.len() == previous_len {
            break;
        }
    }
    tree.retain(|pid| processes.iter().any(|(candidate, _)| candidate == pid));
    tree
}

#[cfg(target_os = "windows")]
fn terminate_process_tree(root_pid: u32) -> Result<(), String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    let mut process_ids = process_tree(root_pid);
    if !process_ids.contains(&root_pid) {
        process_ids.push(root_pid);
    }
    for pid in process_ids.into_iter().rev() {
        if let Ok(process) = unsafe { OpenProcess(PROCESS_TERMINATE, false, pid) } {
            unsafe {
                let _ = TerminateProcess(process, 1);
                let _ = CloseHandle(process);
            }
        }
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while std::time::Instant::now() < deadline {
        if process_tree(root_pid).is_empty() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    if !process_tree(root_pid).is_empty() {
        return Err("The native app process is still shutting down; its NCOS files are still in use.".into());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn terminate_app_directory_processes(app_id: &str) -> Result<bool, String> {
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    };

    let layout = poly::virtual_drive::app_layout(app_id)?;
    let program_root = layout.program.canonicalize().map_err(|error| error.to_string())?;
    let root_prefix = format!("{}\\", program_root.to_string_lossy()).to_ascii_lowercase();
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
        .map_err(|error| format!("Could not inspect NCOS app processes: {error}"))?;
    let mut pids = Vec::new();
    let mut entry = PROCESSENTRY32W::default();
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    unsafe {
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                if let Ok(process) = OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE,
                    false,
                    entry.th32ProcessID,
                ) {
                    let mut image = vec![0u16; 32768];
                    let mut length = image.len() as u32;
                    let path = QueryFullProcessImageNameW(
                        process,
                        PROCESS_NAME_WIN32,
                        windows::core::PWSTR(image.as_mut_ptr()),
                        &mut length,
                    )
                    .ok()
                    .map(|_| std::path::PathBuf::from(std::ffi::OsString::from_wide(&image[..length as usize])));
                    if path.is_some_and(|path| {
                        path.to_string_lossy()
                            .to_ascii_lowercase()
                            .starts_with(&root_prefix)
                    }) {
                        pids.push(entry.th32ProcessID);
                    }
                    let _ = CloseHandle(process);
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }
    let found = !pids.is_empty();
    for pid in pids.into_iter().rev() {
        terminate_process_tree(pid)?;
    }
    Ok(found)
}

#[cfg(target_os = "windows")]
fn find_main_window(
    process_ids: &[u32],
    visible_only: bool,
) -> Result<Option<windows::Win32::Foundation::HWND>, String> {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindow, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible, GW_OWNER,
    };
    struct Search<'a> {
        process_ids: &'a [u32],
        visible_only: bool,
        found: Option<(HWND, i64)>,
    }
    unsafe extern "system" fn visit(hwnd: HWND, state: LPARAM) -> BOOL {
        let search = unsafe { &mut *(state.0 as *mut Search<'_>) };
        let mut owner_pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut owner_pid));
        }
        if search.process_ids.contains(&owner_pid)
            && (!search.visible_only || unsafe { IsWindowVisible(hwnd).as_bool() })
        {
            let has_owner = unsafe {
                GetWindow(hwnd, GW_OWNER)
                    .map(|owner| !owner.0.is_null())
                    .unwrap_or(false)
            };
            if !has_owner {
                let mut rect = windows::Win32::Foundation::RECT::default();
                if unsafe { GetWindowRect(hwnd, &mut rect) }.is_ok() {
                    let area = i64::from((rect.right - rect.left).max(0))
                        * i64::from((rect.bottom - rect.top).max(0));
                    if area > 0 && search
                        .found
                        .is_none_or(|(_, largest_area)| area > largest_area)
                    {
                        search.found = Some((hwnd, area));
                    }
                }
            }
        }
        BOOL(1)
    }
    let mut search = Search {
        process_ids,
        visible_only,
        found: None,
    };
    let enumeration =
        unsafe { EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize)) };
    enumeration.map_err(|error| format!("Could not enumerate native app windows: {error}"))?;
    Ok(search.found.map(|(hwnd, _)| hwnd))
}

#[cfg(target_os = "windows")]
fn find_owned_windows(
    process_ids: &[u32],
    primary: windows::Win32::Foundation::HWND,
) -> Result<
    Vec<(
        windows::Win32::Foundation::HWND,
        windows::Win32::Foundation::RECT,
    )>,
    String,
> {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindow, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible, GW_OWNER,
    };
    struct Search<'a> {
        process_ids: &'a [u32],
        primary: HWND,
        found: Vec<(HWND, RECT)>,
    }
    unsafe extern "system" fn visit(hwnd: HWND, state: LPARAM) -> BOOL {
        let search = unsafe { &mut *(state.0 as *mut Search<'_>) };
        if hwnd.0 == search.primary.0 || !unsafe { IsWindowVisible(hwnd).as_bool() } {
            return BOOL(1);
        }
        let Ok(owner) = (unsafe { GetWindow(hwnd, GW_OWNER) }) else {
            return BOOL(1);
        };
        if owner.0.is_null() {
            return BOOL(1);
        }
        let mut window_pid = 0;
        let mut owner_pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut window_pid));
            GetWindowThreadProcessId(owner, Some(&mut owner_pid));
        }
        if !search.process_ids.contains(&window_pid) || !search.process_ids.contains(&owner_pid) {
            return BOOL(1);
        }
        let mut rect = RECT::default();
        if unsafe { GetWindowRect(hwnd, &mut rect) }.is_ok()
            && rect.right > rect.left
            && rect.bottom > rect.top
        {
            search.found.push((hwnd, rect));
        }
        BOOL(1)
    }
    let mut search = Search {
        process_ids,
        primary,
        found: Vec::new(),
    };
    unsafe { EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize)) }
        .map_err(|error| format!("Could not enumerate native app dialogs: {error}"))?;
    Ok(search.found)
}

fn log_window_diagnostic(app_id: &str, message: &str) {
    let Some(local) = std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from) else {
        return;
    };
    let log_dir = local
        .join("ProjectInternet")
        .join("NCOS")
        .join("Drive")
        .join("System")
        .join("Logs");
    if std::fs::create_dir_all(&log_dir).is_err() {
        return;
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("native-app-windows.log"))
    else {
        return;
    };
    let _ = writeln!(file, "[{}] [{app_id}] {message}", chrono_free_timestamp());
}

fn chrono_free_timestamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

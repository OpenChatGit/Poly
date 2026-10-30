//! Poly-managed CEF windowless browser surfaces.
//!
//! CEF owns its browser UI thread. Poly initializes CEF on the application
//! thread and sends browser commands through a dedicated actor thread.

#[cfg(feature = "cef")]
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
#[cfg(feature = "cef")]
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug)]
pub struct CefFrame {
    /// Full CEF surface dimensions.
    pub width: u32,
    pub height: u32,
    pub sequence: u64,
    /// Updated rectangle in surface coordinates.
    pub dirty_x: u32,
    pub dirty_y: u32,
    pub dirty_width: u32,
    pub dirty_height: u32,
    /// RGBA8 pixels for the updated rectangle, suitable for ImageData/Canvas 2D.
    pub rgba: Arc<[u8]>,
}

#[cfg(feature = "cef")]
mod enabled {
    use super::*;
    use cef::{ImplBrowser, ImplBrowserHost, ImplFrame, ImplRenderHandler, *};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{self, Sender};
    use std::sync::Condvar;
    use std::thread;
    use std::time::{Duration, Instant};

    #[derive(Default)]
    struct FrameState {
        width: u32,
        height: u32,
        sequence: u64,
        dirty_x: u32,
        dirty_y: u32,
        dirty_width: u32,
        dirty_height: u32,
        rgba: Arc<[u8]>,
        full_rgba: Vec<u8>,
    }

    #[derive(Clone, Default)]
    struct RenderState {
        size: Arc<Mutex<(i32, i32)>>,
        frame: Arc<Mutex<FrameState>>,
    }

    #[derive(Default)]
    struct PageInfo {
        address: String,
        title: String,
        favicon: Option<String>,
    }

    wrap_display_handler! {
        struct DisplayHandlerBuilder { info: Arc<Mutex<PageInfo>> }
        impl DisplayHandler {
            fn on_address_change(
                &self,
                _browser: Option<&mut Browser>,
                frame: Option<&mut Frame>,
                url: Option<&CefString>,
            ) {
                if frame.as_ref().is_some_and(|frame| frame.is_main() == 0) { return; }
                if let Some(url) = url {
                    let url = url.to_string();
                    let mut info = self.info.lock().unwrap_or_else(|error| error.into_inner());
                    if info.address != url {
                        info.title.clear();
                        info.favicon = None;
                    }
                    info.address = url;
                }
            }

            fn on_title_change(&self, _browser: Option<&mut Browser>, title: Option<&CefString>) {
                if let Some(title) = title {
                    self.info.lock().unwrap_or_else(|error| error.into_inner()).title = title.to_string();
                }
            }

            fn on_favicon_urlchange(
                &self,
                _browser: Option<&mut Browser>,
                icon_urls: Option<&mut CefStringList>,
            ) {
                let candidate = icon_urls.as_ref()
                    .map(|urls| (*urls).clone().into_iter().find(|url| {
                        let lower = url.to_ascii_lowercase();
                        lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("data:image/")
                    }))
                    .flatten();
                if let Some(url) = candidate {
                    self.info.lock().unwrap_or_else(|error| error.into_inner()).favicon = Some(url);
                }
            }
        }
    }

    impl DisplayHandlerBuilder {
        fn build(info: Arc<Mutex<PageInfo>>) -> DisplayHandler {
            Self::new(info)
        }
    }

    wrap_render_handler! {
        struct RenderHandlerBuilder { state: RenderState }
        impl RenderHandler {
            fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
                if let Some(rect) = rect {
                    let (width, height) = *self.state.size.lock().unwrap_or_else(|e| e.into_inner());
                    rect.x = 0;
                    rect.y = 0;
                    rect.width = width.max(1);
                    rect.height = height.max(1);
                }
            }

            fn on_paint(
                &self,
                _browser: Option<&mut Browser>,
                element_type: PaintElementType,
                dirty_rects: Option<&[Rect]>,
                buffer: *const u8,
                width: ::std::os::raw::c_int,
                height: ::std::os::raw::c_int,
            ) {
                if element_type != PaintElementType::default() || buffer.is_null() || width <= 0 || height <= 0 {
                    return;
                }
                let byte_count = (width as usize).checked_mul(height as usize).and_then(|n| n.checked_mul(4));
                let Some(byte_count) = byte_count else { return; };
                if (width as u64) * (height as u64) > MAX_SURFACE_PIXELS { return; }
                // CEF owns the BGRA buffer for this callback only. Convert only
                // the dirty rectangle, rather than copying the entire surface.
                let bgra = unsafe { std::slice::from_raw_parts(buffer, byte_count) };
                let mut frame = self.state.frame.lock().unwrap_or_else(|e| e.into_inner());
                let width = width as u32;
                let height = height as u32;
                let full_surface = frame.sequence == 0 || frame.width != width || frame.height != height;
                let mut left = width;
                let mut top = height;
                let mut right = 0u32;
                let mut bottom = 0u32;
                if !full_surface {
                    for rect in dirty_rects.unwrap_or_default() {
                        let x0 = rect.x.max(0).min(width as i32) as u32;
                        let y0 = rect.y.max(0).min(height as i32) as u32;
                        let x1 = rect.x.saturating_add(rect.width).max(0).min(width as i32) as u32;
                        let y1 = rect.y.saturating_add(rect.height).max(0).min(height as i32) as u32;
                        if x1 <= x0 || y1 <= y0 { continue; }
                        left = left.min(x0);
                        top = top.min(y0);
                        right = right.max(x1);
                        bottom = bottom.max(y1);
                    }
                }
                if full_surface || right <= left || bottom <= top {
                    left = 0;
                    top = 0;
                    right = width;
                    bottom = height;
                }
                let dirty_width = right - left;
                let dirty_height = bottom - top;
                let patch_len = (dirty_width as usize).checked_mul(dirty_height as usize).and_then(|n| n.checked_mul(4));
                let Some(patch_len) = patch_len else { return; };
                let mut rgba = vec![0; patch_len];
                frame.full_rgba.resize(byte_count, 0);
                for row in 0..dirty_height as usize {
                    for column in 0..dirty_width as usize {
                        let source_index = (((top as usize + row) * width as usize) + left as usize + column) * 4;
                        let target_index = (row * dirty_width as usize + column) * 4;
                        let surface_index = (((top as usize + row) * width as usize) + left as usize + column) * 4;
                        let red = bgra[source_index + 2];
                        let green = bgra[source_index + 1];
                        let blue = bgra[source_index];
                        let alpha = bgra[source_index + 3];
                        rgba[target_index] = red;
                        rgba[target_index + 1] = green;
                        rgba[target_index + 2] = blue;
                        rgba[target_index + 3] = alpha;
                        frame.full_rgba[surface_index] = red;
                        frame.full_rgba[surface_index + 1] = green;
                        frame.full_rgba[surface_index + 2] = blue;
                        frame.full_rgba[surface_index + 3] = alpha;
                    }
                }
                frame.width = width;
                frame.height = height;
                frame.sequence = frame.sequence.wrapping_add(1);
                frame.dirty_x = left;
                frame.dirty_y = top;
                frame.dirty_width = dirty_width;
                frame.dirty_height = dirty_height;
                frame.rgba = Arc::from(rgba);
                drop(frame);
                if let Some((generation, signal)) = FRAME_SIGNAL.get() {
                    if let Ok(mut generation) = generation.lock() {
                        *generation = generation.wrapping_add(1);
                        signal.notify_all();
                    }
                }
            }
        }
    }

    wrap_client! {
        struct ClientBuilder { render_handler: RenderHandler, life_span_handler: LifeSpanHandler, display_handler: DisplayHandler }
        impl Client {
            fn render_handler(&self) -> Option<RenderHandler> {
                Some(self.render_handler.clone())
            }

            fn life_span_handler(&self) -> Option<LifeSpanHandler> {
                Some(self.life_span_handler.clone())
            }

            fn display_handler(&self) -> Option<DisplayHandler> {
                Some(self.display_handler.clone())
            }
        }
    }

    wrap_life_span_handler! {
        struct LifeSpanHandlerBuilder {
            id: String,
            ready: mpsc::SyncSender<Result<(), String>>,
            sender: Sender<Command>,
        }

        impl LifeSpanHandler {
            fn on_after_created(&self, browser: Option<&mut Browser>) {
                let Some(browser) = browser else {
                    let _ = self.ready.send(Err("CEF created a browser without returning its handle.".into()));
                    return;
                };
                let _ = self.sender.send(Command::Created {
                    id: self.id.clone(), browser: browser.clone(), ready: self.ready.clone(),
                });
            }
        }
    }

    impl RenderHandlerBuilder {
        fn build(state: RenderState) -> RenderHandler {
            Self::new(state)
        }
    }

    impl ClientBuilder {
        fn build(
            render_handler: RenderHandler,
            life_span_handler: LifeSpanHandler,
            display_handler: DisplayHandler,
        ) -> Client {
            Self::new(render_handler, life_span_handler, display_handler)
        }
    }

    enum Command {
        Create {
            id: String,
            url: String,
            size: Arc<Mutex<(i32, i32)>>,
            frame: Arc<Mutex<FrameState>>,
            info: Arc<Mutex<PageInfo>>,
            windowed: bool,
            parent_window: Option<isize>,
            x: i32,
            y: i32,
            ready: mpsc::SyncSender<Result<(), String>>,
        },
        Created {
            id: String,
            browser: Browser,
            ready: mpsc::SyncSender<Result<(), String>>,
        },
        Navigate {
            id: String,
            url: String,
        },
        History {
            id: String,
            direction: String,
        },
        Reload {
            id: String,
        },
        Resize {
            id: String,
        },
        Bounds {
            id: String,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
        },
        Visible {
            id: String,
            visible: bool,
        },
        MouseMove {
            id: String,
            x: i32,
            y: i32,
            leave: bool,
        },
        MouseButton {
            id: String,
            x: i32,
            y: i32,
            down: bool,
        },
        MouseWheel {
            id: String,
            x: i32,
            y: i32,
            delta_x: i32,
            delta_y: i32,
        },
        Key {
            id: String,
            key_type: String,
            key_code: i32,
            character: u16,
            modifiers: u32,
        },
        Destroy {
            id: String,
        },
        Shutdown {
            done: mpsc::SyncSender<()>,
        },
    }

    struct Host {
        sender: Sender<Command>,
        frames: Mutex<HashMap<String, Arc<Mutex<FrameState>>>>,
        sizes: Mutex<HashMap<String, Arc<Mutex<(i32, i32)>>>>,
        page_info: Mutex<HashMap<String, Arc<Mutex<PageInfo>>>>,
    }

    static HOST: OnceLock<Result<Host, String>> = OnceLock::new();
    static CEF_INITIALIZED: OnceLock<Result<(), String>> = OnceLock::new();
    static CEF_CONTEXT_READY: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());
    static CEF_SHUTDOWN: AtomicBool = AtomicBool::new(false);
    static CEF_WAS_INITIALIZED: AtomicBool = AtomicBool::new(false);

    const MAX_SURFACE_PIXELS: u64 = 16_777_216;
    static FRAME_SIGNAL: OnceLock<(Mutex<u64>, Condvar)> = OnceLock::new();

    #[cfg(all(feature = "native", target_os = "windows"))]
    fn set_child_bounds(
        handle: cef::sys::cef_window_handle_t,
        (x, y, width, height): (i32, i32, i32, i32),
    ) {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOP, SWP_NOACTIVATE};
        if handle.0.is_null() || width <= 0 || height <= 0 {
            return;
        }
        let _ = unsafe {
            SetWindowPos(
                HWND(handle.0.cast()),
                HWND_TOP,
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE,
            )
        };
    }

    #[cfg(all(feature = "native", target_os = "windows"))]
    fn set_child_visible(handle: cef::sys::cef_window_handle_t, visible: bool) {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOWNA};
        if !handle.0.is_null() {
            unsafe {
                let _ = ShowWindow(
                    HWND(handle.0.cast()),
                    if visible { SW_SHOWNA } else { SW_HIDE },
                );
            }
        }
    }

    #[cfg(not(all(feature = "native", target_os = "windows")))]
    fn set_child_bounds(_: cef::sys::cef_window_handle_t, _: (i32, i32, i32, i32)) {}

    #[cfg(not(all(feature = "native", target_os = "windows")))]
    fn set_child_visible(_: cef::sys::cef_window_handle_t, _: bool) {}

    wrap_browser_process_handler! {
        struct BrowserProcessHandlerBuilder {}
        impl BrowserProcessHandler {
            fn on_context_initialized(&self) {
                let (ready, wake) = &CEF_CONTEXT_READY;
                *ready.lock().unwrap_or_else(|error| error.into_inner()) = true;
                wake.notify_all();
            }
        }
    }

    impl BrowserProcessHandlerBuilder {
        fn build() -> BrowserProcessHandler {
            Self::new()
        }
    }

    wrap_app! {
        struct CefAppBuilder { handler: BrowserProcessHandler }
        impl App {
            fn on_before_command_line_processing(
                &self,
                process_type: Option<&CefString>,
                command_line: Option<&mut CommandLine>,
            ) {
                let browser_process = process_type.map_or(true, |process| process.to_string().is_empty());
                let gpu_enabled = std::env::var("POLY_CEF_ENABLE_GPU")
                    .map(|value| matches!(value.trim(), "1" | "true" | "yes"))
                    .unwrap_or(false);
                if browser_process && !gpu_enabled {
                    if let Some(command_line) = command_line {
                        command_line.append_switch(Some(&CefString::from("disable-gpu")));
                    }
                }
            }

            fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
                Some(self.handler.clone())
            }
        }
    }

    impl CefAppBuilder {
        fn build() -> App {
            Self::new(BrowserProcessHandlerBuilder::build())
        }
    }

    fn checked_size(width: u32, height: u32) -> Result<(i32, i32), String> {
        let width = width.max(1);
        let height = height.max(1);
        if u64::from(width) * u64::from(height) > MAX_SURFACE_PIXELS {
            return Err("CEF surface exceeds Poly's 16-megapixel limit.".into());
        }
        Ok((width as i32, height as i32))
    }

    /// Initialize CEF on Poly's main application thread before starting the native UI.
    pub fn initialize() -> Result<(), String> {
        let result = CEF_INITIALIZED.get_or_init(|| {
            // Pin the CEF C API version before initialize. This is also needed
            // when Poly is consumed as a library rather than launched through
            // the `poly` binary's entry point.
            let _api_hash = cef::api_hash(cef::sys::CEF_API_VERSION, 0);
            let args = cef::args::Args::new();
            let subprocess_path = std::env::current_exe().unwrap_or_default().to_string_lossy().into_owned();
            let settings = Settings {
                no_sandbox: 1,
                browser_subprocess_path: CefString::from(subprocess_path.as_str()),
                windowless_rendering_enabled: 1,
                multi_threaded_message_loop: 1,
                ..Default::default()
            };
            let mut app = CefAppBuilder::build();
            let initialized = cef::initialize(Some(args.as_main_args()), Some(&settings), Some(&mut app), std::ptr::null_mut());
            if initialized == 1 {
                CEF_WAS_INITIALIZED.store(true, Ordering::Release);
                // CEF accepts browser creation only once its shared request
                // context is ready. With a multi-threaded loop this callback
                // arrives asynchronously on CEF's UI thread.
                let (ready, wake) = &CEF_CONTEXT_READY;
                let ready = ready.lock().unwrap_or_else(|error| error.into_inner());
                let (ready, timeout) = wake.wait_timeout_while(ready, Duration::from_secs(20), |ready| !*ready)
                    .unwrap_or_else(|error| error.into_inner());
                if *ready {
                    Ok(())
                } else if timeout.timed_out() {
                    Err("CEF initialized but its browser context did not become ready within 20 seconds.".into())
                } else {
                    Err("CEF browser context did not become ready.".into())
                }
            } else {
                Err(format!("CEF initialization failed ({initialized}); check that the CEF runtime files are beside Poly."))
            }
        });
        result.as_ref().map_err(Clone::clone).copied()
    }

    pub fn is_initialized() -> bool {
        matches!(CEF_INITIALIZED.get(), Some(Ok(())))
    }

    /// Close every browser surface and release CEF on the application thread.
    /// Call this from the native event loop's LoopDestroyed callback.
    pub fn shutdown() {
        if CEF_SHUTDOWN.swap(true, Ordering::AcqRel) {
            return;
        }
        if let Some(Ok(host)) = HOST.get() {
            let (done_tx, done_rx) = mpsc::sync_channel(1);
            if host
                .sender
                .send(Command::Shutdown { done: done_tx })
                .is_ok()
            {
                // The actor acknowledges after issuing close requests and dropping
                // its browser references. CEF performs final child-process teardown
                // in cef::shutdown() below.
                let _ = done_rx.recv_timeout(Duration::from_secs(5));
            }
        }
        if CEF_WAS_INITIALIZED.swap(false, Ordering::AcqRel) {
            cef::shutdown();
        }
    }

    fn host() -> Result<&'static Host, String> {
        initialize()?;
        let result = HOST.get_or_init(|| {
            let (sender, receiver) = mpsc::channel();
            let thread_sender = sender.clone();
            thread::Builder::new()
                .name("poly-cef-commands".into())
                .spawn(move || cef_thread(receiver, thread_sender))
                .map_err(|error| format!("Could not start the CEF command thread: {error}"))?;
            Ok(Host {
                sender,
                frames: Mutex::new(HashMap::new()),
                sizes: Mutex::new(HashMap::new()),
                page_info: Mutex::new(HashMap::new()),
            })
        });
        result.as_ref().map_err(Clone::clone)
    }

    pub fn create(
        id: &str,
        url: &str,
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        prefer_native_window: bool,
    ) -> Result<bool, String> {
        let host = host()?;
        let id = valid_id(id)?;
        let size = Arc::new(Mutex::new(checked_size(width, height)?));
        let frame = Arc::new(Mutex::new(FrameState::default()));
        let info = Arc::new(Mutex::new(PageInfo {
            address: url.to_string(),
            ..Default::default()
        }));
        #[cfg(all(feature = "native", target_os = "windows"))]
        let parent_window = prefer_native_window
            .then(crate::native::native_host_window)
            .flatten();
        #[cfg(all(feature = "native", target_os = "windows"))]
        let windowed = parent_window.is_some();
        #[cfg(not(all(feature = "native", target_os = "windows")))]
        let windowed = false;
        #[cfg(not(all(feature = "native", target_os = "windows")))]
        let parent_window = None;
        let mut frames = host
            .frames
            .lock()
            .map_err(|_| "CEF frame registry is unavailable")?;
        if frames.contains_key(&id) {
            return Ok(windowed);
        }
        frames.insert(id.clone(), frame.clone());
        drop(frames);
        host.sizes
            .lock()
            .map_err(|_| "CEF surface size registry is unavailable")?
            .insert(id.clone(), size.clone());
        host.page_info
            .lock()
            .map_err(|_| "CEF page metadata registry is unavailable")?
            .insert(id.clone(), info.clone());
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        if let Err(error) = host.sender.send(Command::Create {
            id: id.clone(),
            url: url.to_string(),
            size,
            frame,
            info,
            windowed,
            parent_window,
            x,
            y,
            ready: ready_tx,
        }) {
            if let Ok(mut frames) = host.frames.lock() {
                frames.remove(&id);
            }
            if let Ok(mut sizes) = host.sizes.lock() {
                sizes.remove(&id);
            }
            if let Ok(mut metadata) = host.page_info.lock() {
                metadata.remove(&id);
            }
            return Err(format!("CEF runtime is not accepting surfaces: {error}"));
        }
        match ready_rx.recv_timeout(Duration::from_secs(20)) {
            Ok(Ok(())) => Ok(windowed),
            Ok(Err(error)) => {
                if let Ok(mut frames) = host.frames.lock() {
                    frames.remove(&id);
                }
                if let Ok(mut sizes) = host.sizes.lock() {
                    sizes.remove(&id);
                }
                if let Ok(mut metadata) = host.page_info.lock() {
                    metadata.remove(&id);
                }
                Err(error)
            }
            Err(error) => {
                let _ = host.sender.send(Command::Destroy { id: id.clone() });
                if let Ok(mut frames) = host.frames.lock() {
                    frames.remove(&id);
                }
                if let Ok(mut sizes) = host.sizes.lock() {
                    sizes.remove(&id);
                }
                if let Ok(mut metadata) = host.page_info.lock() {
                    metadata.remove(&id);
                }
                Err(format!("CEF surface creation did not complete: {error}"))
            }
        }
    }

    pub fn set_bounds(id: &str, x: i32, y: i32, width: u32, height: u32) -> Result<(), String> {
        let (width, height) = checked_size(width, height)?;
        let host = host()?;
        let id = valid_id(id)?;
        if let Some(size) = host
            .sizes
            .lock()
            .map_err(|_| "CEF surface size registry is unavailable")?
            .get(&id)
            .cloned()
        {
            *size.lock().map_err(|_| "CEF surface size is unavailable")? = (width, height);
        }
        host.sender
            .send(Command::Bounds {
                id,
                x,
                y,
                width,
                height,
            })
            .map_err(|error| error.to_string())
    }

    pub fn navigate(id: &str, url: &str) -> Result<(), String> {
        host()?
            .sender
            .send(Command::Navigate {
                id: valid_id(id)?,
                url: url.to_string(),
            })
            .map_err(|error| error.to_string())
    }

    pub fn history(id: &str, direction: &str) -> Result<(), String> {
        if !matches!(direction, "back" | "forward") {
            return Err("Invalid CEF history direction.".into());
        }
        host()?
            .sender
            .send(Command::History {
                id: valid_id(id)?,
                direction: direction.into(),
            })
            .map_err(|error| error.to_string())
    }

    pub fn reload(id: &str) -> Result<(), String> {
        host()?
            .sender
            .send(Command::Reload { id: valid_id(id)? })
            .map_err(|error| error.to_string())
    }

    pub fn resize(id: &str, width: u32, height: u32) -> Result<(), String> {
        let host = host()?;
        let id = valid_id(id)?;
        let size_px = checked_size(width, height)?;
        let size = host
            .sizes
            .lock()
            .map_err(|_| "CEF surface size registry is unavailable")?
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("CEF surface '{id}' does not exist"))?;
        *size.lock().map_err(|_| "CEF surface size is unavailable")? = size_px;
        host.sender
            .send(Command::Resize { id })
            .map_err(|error| error.to_string())
    }

    pub fn set_visible(id: &str, visible: bool) -> Result<(), String> {
        host()?
            .sender
            .send(Command::Visible {
                id: valid_id(id)?,
                visible,
            })
            .map_err(|error| error.to_string())
    }

    pub fn mouse_move(id: &str, x: i32, y: i32, leave: bool) -> Result<(), String> {
        host()?
            .sender
            .send(Command::MouseMove {
                id: valid_id(id)?,
                x,
                y,
                leave,
            })
            .map_err(|error| error.to_string())
    }

    pub fn mouse_button(id: &str, x: i32, y: i32, down: bool) -> Result<(), String> {
        host()?
            .sender
            .send(Command::MouseButton {
                id: valid_id(id)?,
                x,
                y,
                down,
            })
            .map_err(|error| error.to_string())
    }

    pub fn mouse_wheel(id: &str, x: i32, y: i32, delta_x: i32, delta_y: i32) -> Result<(), String> {
        host()?
            .sender
            .send(Command::MouseWheel {
                id: valid_id(id)?,
                x,
                y,
                delta_x,
                delta_y,
            })
            .map_err(|error| error.to_string())
    }

    pub fn key(
        id: &str,
        key_type: &str,
        key_code: i32,
        character: u16,
        modifiers: u32,
    ) -> Result<(), String> {
        if !matches!(key_type, "down" | "up" | "char") {
            return Err("Invalid CEF key event type.".into());
        }
        host()?
            .sender
            .send(Command::Key {
                id: valid_id(id)?,
                key_type: key_type.into(),
                key_code,
                character,
                modifiers,
            })
            .map_err(|error| error.to_string())
    }

    pub fn destroy(id: &str) -> Result<(), String> {
        let host = host()?;
        let id = valid_id(id)?;
        host.sender
            .send(Command::Destroy { id: id.clone() })
            .map_err(|error| error.to_string())?;
        if let Ok(mut frames) = host.frames.lock() {
            frames.remove(&id);
        }
        if let Ok(mut sizes) = host.sizes.lock() {
            sizes.remove(&id);
        }
        if let Ok(mut metadata) = host.page_info.lock() {
            metadata.remove(&id);
        }
        Ok(())
    }

    pub fn read_frame(id: &str) -> Result<CefFrame, String> {
        read_frame_after(id, 0)?.ok_or_else(|| "CEF frame is not ready.".into())
    }

    pub fn read_frame_after(id: &str, sequence: u64) -> Result<Option<CefFrame>, String> {
        let host = host()?;
        let id = valid_id(id)?;
        let frames = host
            .frames
            .lock()
            .map_err(|_| "CEF frame registry is unavailable")?;
        let frame = frames
            .get(&id)
            .ok_or_else(|| format!("CEF surface '{id}' does not exist"))?
            .lock()
            .map_err(|_| "CEF frame is unavailable")?;
        if frame.sequence <= sequence {
            return Ok(None);
        }
        let missed_updates = frame.sequence > sequence.saturating_add(1);
        Ok(Some(CefFrame {
            width: frame.width,
            height: frame.height,
            sequence: frame.sequence,
            dirty_x: if missed_updates { 0 } else { frame.dirty_x },
            dirty_y: if missed_updates { 0 } else { frame.dirty_y },
            dirty_width: if missed_updates {
                frame.width
            } else {
                frame.dirty_width
            },
            dirty_height: if missed_updates {
                frame.height
            } else {
                frame.dirty_height
            },
            rgba: if missed_updates {
                Arc::from(frame.full_rgba.clone())
            } else {
                Arc::clone(&frame.rgba)
            },
        }))
    }

    pub fn read_frame_after_wait(
        id: &str,
        sequence: u64,
        timeout_ms: u64,
    ) -> Result<Option<CefFrame>, String> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms.min(250));
        let (generation, signal) = FRAME_SIGNAL.get_or_init(|| (Mutex::new(0), Condvar::new()));
        let mut generation = generation.lock().unwrap_or_else(|error| error.into_inner());
        loop {
            if let Some(frame) = read_frame_after(id, sequence)? {
                return Ok(Some(frame));
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            let (next_generation, timeout) = signal
                .wait_timeout(generation, remaining)
                .unwrap_or_else(|error| error.into_inner());
            generation = next_generation;
            if timeout.timed_out() {
                return read_frame_after(id, sequence);
            }
        }
    }

    pub fn favicon(id: &str) -> Result<Option<String>, String> {
        Ok(metadata(id)?.2)
    }

    pub fn metadata(id: &str) -> Result<(String, String, Option<String>), String> {
        let host = host()?;
        let id = valid_id(id)?;
        let entries = host
            .page_info
            .lock()
            .map_err(|_| "CEF page metadata registry is unavailable")?;
        let info = entries
            .get(&id)
            .ok_or_else(|| format!("CEF surface '{id}' does not exist"))?;
        let info = info
            .lock()
            .map_err(|_| "CEF page metadata is unavailable")?;
        Ok((
            info.address.clone(),
            info.title.clone(),
            info.favicon.clone(),
        ))
    }

    fn valid_id(id: &str) -> Result<String, String> {
        let id = id.trim();
        if id.is_empty()
            || id.len() > 96
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
        {
            return Err("Invalid CEF surface ID.".into());
        }
        Ok(id.to_string())
    }

    fn cef_thread(receiver: mpsc::Receiver<Command>, sender: Sender<Command>) {
        let mut browsers: HashMap<String, Browser> = HashMap::new();
        let mut pending_creations: HashSet<String> = HashSet::new();
        let mut visible_browsers: HashSet<String> = HashSet::new();
        let mut windowed_browsers: HashSet<String> = HashSet::new();
        let mut browser_bounds: HashMap<String, (i32, i32, i32, i32)> = HashMap::new();
        'actor: loop {
            while let Ok(command) = receiver.try_recv() {
                match command {
                    Command::Create {
                        id,
                        url,
                        size,
                        frame,
                        info,
                        windowed,
                        parent_window,
                        x,
                        y,
                        ready,
                    } => {
                        if browsers.contains_key(&id) || pending_creations.contains(&id) {
                            let _ = ready.send(Err(format!("CEF surface '{id}' already exists.")));
                            continue;
                        }
                        let initial_size = *size.lock().unwrap_or_else(|error| error.into_inner());
                        let handler = RenderHandlerBuilder::build(RenderState { size, frame });
                        let life_span_handler =
                            LifeSpanHandlerBuilder::new(id.clone(), ready.clone(), sender.clone());
                        let display_handler = DisplayHandlerBuilder::build(info);
                        let mut client =
                            ClientBuilder::build(handler, life_span_handler, display_handler);
                        pending_creations.insert(id.clone());
                        // CEF documents CreateBrowser as callable from any
                        // browser-process thread. It copies these parameters
                        // internally and creates the native/browser UI on its
                        // own UI thread. Avoid wrapping a Rust Task here: the
                        // generated Task bridge reports version -1 with CEF 156.
                        #[cfg(all(feature = "native", target_os = "windows"))]
                        let window_info = if windowed {
                            use windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE;
                            let parent = cef::sys::HWND(
                                parent_window.unwrap_or_default() as *mut cef::sys::HWND__
                            );
                            let (width, height) = initial_size;
                            let bounds = Rect {
                                x,
                                y,
                                width,
                                height,
                            };
                            let mut info = WindowInfo::default().set_as_child(parent, &bounds);
                            info.style &= !WS_VISIBLE.0;
                            windowed_browsers.insert(id.clone());
                            browser_bounds.insert(id.clone(), (x, y, width, height));
                            info
                        } else {
                            WindowInfo {
                                windowless_rendering_enabled: 1,
                                ..Default::default()
                            }
                        };
                        #[cfg(not(all(feature = "native", target_os = "windows")))]
                        let window_info = WindowInfo {
                            windowless_rendering_enabled: 1,
                            ..Default::default()
                        };
                        // CEF's OSR default is 30 FPS. Let the embedded browser
                        // render up to a normal desktop refresh rate when the PC
                        // can keep up; CEF will naturally produce fewer frames
                        // when the page or machine cannot.
                        let browser_settings = BrowserSettings {
                            windowless_frame_rate: 60,
                            ..Default::default()
                        };
                        let cef_url = CefString::from(url.as_str());
                        if cef::browser_host_create_browser(
                            Some(&window_info),
                            Some(&mut client),
                            Some(&cef_url),
                            Some(&browser_settings),
                            None,
                            None,
                        ) == 0
                        {
                            pending_creations.remove(&id);
                            let _ = ready
                                .send(Err("CEF rejected the windowless browser request.".into()));
                        }
                    }
                    Command::Created { id, browser, ready } => {
                        pending_creations.remove(&id);
                        if !windowed_browsers.contains(&id) {
                            visible_browsers.insert(id.clone());
                        }
                        browsers.insert(id, browser);
                        let _ = ready.send(Ok(()));
                    }
                    Command::Navigate { id, url } => {
                        if let Some(frame) = browsers
                            .get_mut(&id)
                            .and_then(|browser| browser.main_frame())
                        {
                            frame.load_url(Some(&CefString::from(url.as_str())));
                        }
                    }
                    Command::History { id, direction } => {
                        if let Some(browser) = browsers.get_mut(&id) {
                            if direction == "back" {
                                browser.go_back();
                            } else {
                                browser.go_forward();
                            }
                        }
                    }
                    Command::Reload { id } => {
                        if let Some(browser) = browsers.get_mut(&id) {
                            browser.reload();
                        }
                    }
                    Command::Resize { id } => {
                        if let Some(browser) = browsers.get_mut(&id) {
                            if windowed_browsers.contains(&id) {
                                if let (Some(host), Some(bounds)) =
                                    (browser.host(), browser_bounds.get(&id).copied())
                                {
                                    set_child_bounds(host.window_handle(), bounds);
                                }
                            } else if let Some(host) = browser.host() {
                                host.was_resized();
                            }
                        }
                    }
                    Command::Bounds {
                        id,
                        x,
                        y,
                        width,
                        height,
                    } => {
                        browser_bounds.insert(id.clone(), (x, y, width, height));
                        if windowed_browsers.contains(&id) {
                            if let Some(host) =
                                browsers.get_mut(&id).and_then(|browser| browser.host())
                            {
                                set_child_bounds(host.window_handle(), (x, y, width, height));
                            }
                        }
                    }
                    Command::Visible { id, visible } => {
                        if let Some(host) = browsers.get_mut(&id).and_then(|browser| browser.host())
                        {
                            if windowed_browsers.contains(&id) {
                                set_child_visible(host.window_handle(), visible);
                            } else {
                                host.was_hidden((!visible) as _);
                            }
                        }
                        if browsers.contains_key(&id) {
                            if visible && !windowed_browsers.contains(&id) {
                                visible_browsers.insert(id);
                            } else {
                                visible_browsers.remove(&id);
                            }
                        }
                    }
                    Command::MouseMove { id, x, y, leave } => {
                        if let Some(host) = browsers.get_mut(&id).and_then(|browser| browser.host())
                        {
                            let event = MouseEvent {
                                x,
                                y,
                                ..Default::default()
                            };
                            host.send_mouse_move_event(Some(&event), leave as _);
                        }
                    }
                    Command::MouseButton { id, x, y, down } => {
                        if let Some(host) = browsers.get_mut(&id).and_then(|browser| browser.host())
                        {
                            let event = MouseEvent {
                                x,
                                y,
                                ..Default::default()
                            };
                            host.send_mouse_click_event(
                                Some(&event),
                                MouseButtonType::LEFT,
                                (!down) as _,
                                1,
                            );
                        }
                    }
                    Command::MouseWheel {
                        id,
                        x,
                        y,
                        delta_x,
                        delta_y,
                    } => {
                        if let Some(host) = browsers.get_mut(&id).and_then(|browser| browser.host())
                        {
                            let event = MouseEvent {
                                x,
                                y,
                                ..Default::default()
                            };
                            host.send_mouse_wheel_event(Some(&event), delta_x, delta_y);
                        }
                    }
                    Command::Key {
                        id,
                        key_type,
                        key_code,
                        character,
                        modifiers,
                    } => {
                        if let Some(host) = browsers.get_mut(&id).and_then(|browser| browser.host())
                        {
                            let event = KeyEvent {
                                size: std::mem::size_of::<KeyEvent>(),
                                type_: match key_type.as_str() {
                                    "up" => KeyEventType::KEYUP,
                                    "char" => KeyEventType::CHAR,
                                    _ => KeyEventType::RAWKEYDOWN,
                                },
                                modifiers,
                                windows_key_code: key_code,
                                native_key_code: key_code,
                                is_system_key: 0,
                                character,
                                unmodified_character: character,
                                focus_on_editable_field: 0,
                            };
                            host.send_key_event(Some(&event));
                        }
                    }
                    Command::Destroy { id } => {
                        visible_browsers.remove(&id);
                        windowed_browsers.remove(&id);
                        browser_bounds.remove(&id);
                        pending_creations.remove(&id);
                        if let Some(browser) = browsers.remove(&id) {
                            if let Some(host) = browser.host() {
                                host.close_browser(true as _);
                            }
                        }
                    }
                    Command::Shutdown { done } => {
                        for (_, browser) in browsers.drain() {
                            if let Some(host) = browser.host() {
                                host.close_browser(true as _);
                            }
                        }
                        pending_creations.clear();
                        visible_browsers.clear();
                        windowed_browsers.clear();
                        browser_bounds.clear();
                        let _ = done.send(());
                        break 'actor;
                    }
                }
            }
            let idle_delay = if browsers.is_empty() {
                50
            } else if visible_browsers.is_empty() {
                25
            } else {
                8
            };
            thread::sleep(Duration::from_millis(idle_delay));
        }
    }
}

#[cfg(feature = "cef")]
pub use enabled::{
    create, destroy, favicon, history, initialize, is_initialized, key, metadata, mouse_button,
    mouse_move, mouse_wheel, navigate, read_frame, read_frame_after, read_frame_after_wait, reload,
    resize, set_bounds, set_visible, shutdown,
};

#[cfg(not(feature = "cef"))]
mod disabled {
    use super::*;
    fn disabled() -> Result<(), String> {
        Err("Poly was built without the `cef` feature.".into())
    }
    pub fn create(
        _: &str,
        _: &str,
        _: u32,
        _: u32,
        _: i32,
        _: i32,
        _: bool,
    ) -> Result<bool, String> {
        disabled()?;
        Ok(false)
    }
    pub fn navigate(_: &str, _: &str) -> Result<(), String> {
        disabled()
    }
    pub fn history(_: &str, _: &str) -> Result<(), String> {
        disabled()
    }
    pub fn reload(_: &str) -> Result<(), String> {
        disabled()
    }
    pub fn resize(_: &str, _: u32, _: u32) -> Result<(), String> {
        disabled()
    }
    pub fn set_bounds(_: &str, _: i32, _: i32, _: u32, _: u32) -> Result<(), String> {
        disabled()
    }
    pub fn set_visible(_: &str, _: bool) -> Result<(), String> {
        disabled()
    }
    pub fn mouse_move(_: &str, _: i32, _: i32, _: bool) -> Result<(), String> {
        disabled()
    }
    pub fn mouse_button(_: &str, _: i32, _: i32, _: bool) -> Result<(), String> {
        disabled()
    }
    pub fn mouse_wheel(_: &str, _: i32, _: i32, _: i32, _: i32) -> Result<(), String> {
        disabled()
    }
    pub fn key(_: &str, _: &str, _: i32, _: u16, _: u32) -> Result<(), String> {
        disabled()
    }
    pub fn destroy(_: &str) -> Result<(), String> {
        disabled()
    }
    pub fn initialize() -> Result<(), String> {
        disabled()
    }
    pub fn shutdown() {}
    pub fn is_initialized() -> bool {
        false
    }
    pub fn read_frame(_: &str) -> Result<CefFrame, String> {
        Err("Poly was built without the `cef` feature.".into())
    }
    pub fn read_frame_after(_: &str, _: u64) -> Result<Option<CefFrame>, String> {
        Err("Poly was built without the `cef` feature.".into())
    }
    pub fn read_frame_after_wait(_: &str, _: u64, _: u64) -> Result<Option<CefFrame>, String> {
        Err("Poly was built without the `cef` feature.".into())
    }
    pub fn favicon(_: &str) -> Result<Option<String>, String> {
        Err("Poly was built without the `cef` feature.".into())
    }
    pub fn metadata(_: &str) -> Result<(String, String, Option<String>), String> {
        Err("Poly was built without the `cef` feature.".into())
    }
}

#[cfg(not(feature = "cef"))]
pub use disabled::{
    create, destroy, favicon, history, initialize, is_initialized, key, metadata, mouse_button,
    mouse_move, mouse_wheel, navigate, read_frame, read_frame_after, read_frame_after_wait, reload,
    resize, set_bounds, set_visible, shutdown,
};

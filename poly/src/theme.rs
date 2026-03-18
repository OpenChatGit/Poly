//! OS Theme Detection for Poly
//! Detects dark/light mode and accent color from the operating system.

/// The current OS color scheme
#[derive(Debug, Clone, PartialEq)]
pub enum ColorScheme {
    Dark,
    Light,
}

impl ColorScheme {
    pub fn as_str(&self) -> &'static str {
        match self {
            ColorScheme::Dark => "dark",
            ColorScheme::Light => "light",
        }
    }
}

/// Detect the current OS color scheme (dark or light mode)
pub fn get_color_scheme() -> ColorScheme {
    #[cfg(all(target_os = "windows", feature = "native"))]
    {
        if let Some(scheme) = detect_windows_theme() {
            return scheme;
        }
    }
    // Fallback: light
    ColorScheme::Light
}

/// Get the OS accent color as a hex string (e.g. "#0078d4")
pub fn get_accent_color() -> String {
    #[cfg(all(target_os = "windows", feature = "native"))]
    {
        if let Some(color) = detect_windows_accent() {
            return color;
        }
    }
    // Fallback: Poly cyan
    "#5dc1d2".to_string()
}

/// Returns a dict-like structure with all theme info
pub fn get_theme_info() -> ThemeInfo {
    ThemeInfo {
        scheme: get_color_scheme(),
        accent: get_accent_color(),
    }
}

/// Generate a JavaScript snippet that injects OS theme as CSS variables.
/// Inject this into your WebView init script so every page gets theme-aware CSS.
///
/// Provides:
///   --poly-scheme:  "dark" | "light"
///   --poly-accent:  e.g. "#0078d4"
///   --poly-bg:      main background color
///   --poly-fg:      main foreground color
///   --poly-surface: card/panel background
///   --poly-border:  border color
pub fn get_theme_inject_js() -> String {
    let info = get_theme_info();
    let scheme = info.scheme.as_str();
    let accent = info.accent;

    let (bg, fg, surface, border) = if info.scheme == ColorScheme::Dark {
        ("#1a1a1f", "#f0f0f0", "#2a2a2f", "#3a3a3f")
    } else {
        ("#f5f5f5", "#1a1a1a", "#ffffff", "#e0e0e0")
    };

    format!(r#"(function() {{
  const scheme = "{scheme}";
  const accent = "{accent}";
  const root = document.documentElement;
  root.setAttribute("data-poly-theme", scheme);
  root.style.setProperty("--poly-scheme", scheme);
  root.style.setProperty("--poly-accent", accent);
  root.style.setProperty("--poly-bg", "{bg}");
  root.style.setProperty("--poly-fg", "{fg}");
  root.style.setProperty("--poly-surface", "{surface}");
  root.style.setProperty("--poly-border", "{border}");
  // Dispatch event so apps can react
  window.dispatchEvent(new CustomEvent("polytheme", {{
    detail: {{ scheme, accent }}
  }}));
}})();"#,
        scheme = scheme,
        accent = accent,
        bg = bg,
        fg = fg,
        surface = surface,
        border = border,
    )
}

#[derive(Debug, Clone)]
pub struct ThemeInfo {
    pub scheme: ColorScheme,
    pub accent: String,
}

// ============================================
// Windows Implementation
// ============================================

#[cfg(all(target_os = "windows", feature = "native"))]
fn detect_windows_theme() -> Option<ColorScheme> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    // Read from registry:
    // HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize
    // AppsUseLightTheme = 0 -> dark, 1 -> light
    let key_path: Vec<u16> = OsStr::new(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"
    )
    .encode_wide()
    .chain(std::iter::once(0))
    .collect();

    let value_name: Vec<u16> = OsStr::new("AppsUseLightTheme")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        use windows::Win32::System::Registry::{
            RegOpenKeyExW, RegQueryValueExW, RegCloseKey,
            HKEY_CURRENT_USER, KEY_READ, REG_VALUE_TYPE,
        };
        use windows::core::PCWSTR;

        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(key_path.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        );

        if result.is_err() {
            return None;
        }

        let mut data: u32 = 0;
        let mut data_size: u32 = std::mem::size_of::<u32>() as u32;
        let mut reg_type = REG_VALUE_TYPE::default();

        let query_result = RegQueryValueExW(
            hkey,
            PCWSTR(value_name.as_ptr()),
            None,
            Some(&mut reg_type),
            Some(&mut data as *mut u32 as *mut u8),
            Some(&mut data_size),
        );

        RegCloseKey(hkey).ok();

        if query_result.is_err() {
            return None;
        }

        // 0 = dark mode, 1 = light mode
        if data == 0 {
            Some(ColorScheme::Dark)
        } else {
            Some(ColorScheme::Light)
        }
    }
}

#[cfg(all(target_os = "windows", feature = "native"))]
fn detect_windows_accent() -> Option<String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    // Try AccentColor first (user-set), then ColorizationColor (DWM actual color)
    for (key_str, value_str) in &[
        ("Software\\Microsoft\\Windows\\DWM", "AccentColor"),
        ("Software\\Microsoft\\Windows\\DWM", "ColorizationColor"),
    ] {
        let key_path: Vec<u16> = OsStr::new(key_str)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let value_name: Vec<u16> = OsStr::new(value_str)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            use windows::Win32::System::Registry::{
                RegOpenKeyExW, RegQueryValueExW, RegCloseKey,
                HKEY_CURRENT_USER, KEY_READ, REG_VALUE_TYPE,
            };
            use windows::core::PCWSTR;

            let mut hkey = windows::Win32::System::Registry::HKEY::default();
            let result = RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(key_path.as_ptr()),
                0,
                KEY_READ,
                &mut hkey,
            );

            if result.is_err() {
                continue;
            }

            let mut data: u32 = 0;
            let mut data_size: u32 = std::mem::size_of::<u32>() as u32;
            let mut reg_type = REG_VALUE_TYPE::default();

            let query_result = RegQueryValueExW(
                hkey,
                PCWSTR(value_name.as_ptr()),
                None,
                Some(&mut reg_type),
                Some(&mut data as *mut u32 as *mut u8),
                Some(&mut data_size),
            );

            RegCloseKey(hkey).ok();

            if query_result.is_err() {
                continue;
            }

            // Windows stores as ABGR (alpha, blue, green, red)
            let _a = ((data >> 24) & 0xFF) as u8;
            let b = ((data >> 16) & 0xFF) as u8;
            let g = ((data >> 8) & 0xFF) as u8;
            let r = (data & 0xFF) as u8;

            // Skip black (#000000) - means "automatic", try next key
            if r == 0 && g == 0 && b == 0 {
                continue;
            }

            return Some(format!("#{:02x}{:02x}{:02x}", r, g, b));
        }
    }

    // Final fallback: Windows default blue
    Some("#0078d4".to_string())
}

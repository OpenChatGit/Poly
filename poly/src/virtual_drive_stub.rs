//! Compatibility errors for the experimental Virtual Drive when it is disabled.

use std::path::PathBuf;

pub const MAX_IMPORTED_FILE_BYTES: u64 = 1024 * 1024 * 1024;

fn unavailable() -> String {
    "Poly's Virtual Drive is experimental. Rebuild with --features experimental-virtual-drive.".into()
}

pub fn host_file(_path: &str) -> Result<PathBuf, String> {
    Err(unavailable())
}

pub fn import_file<R: std::io::Read>(
    _directory: &str,
    _name: &str,
    _reader: R,
) -> Result<serde_json::Value, String> {
    Err(unavailable())
}

pub fn validate_app_id(_app_id: &str) -> Result<(), String> {
    Err(unavailable())
}

pub fn install_bundled_app(
    _app_id: &str,
    _source: &std::path::Path,
    _version: &str,
) -> Result<serde_json::Value, String> {
    Err(unavailable())
}

pub fn installed_app_file(_app_id: &str, _relative: &str) -> Result<PathBuf, String> {
    Err(unavailable())
}

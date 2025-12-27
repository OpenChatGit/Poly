fn main() {
    // On Windows, when building with the "gui" feature, set the subsystem to "windows"
    // This prevents the console window from appearing
    #[cfg(target_os = "windows")]
    {
        if std::env::var("CARGO_FEATURE_GUI").is_ok() {
            // For MSVC linker: set Windows subsystem to hide console
            println!("cargo:rustc-link-arg-bin=poly=/SUBSYSTEM:WINDOWS");
            println!("cargo:rustc-link-arg-bin=poly=/ENTRY:mainCRTStartup");
        }
    }
}

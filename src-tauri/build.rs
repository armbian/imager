fn main() {
    // On Windows, embed the manifest to request admin privileges at startup
    #[cfg(windows)]
    {
        let mut windows = tauri_build::WindowsAttributes::new();
        windows = windows.app_manifest(include_str!("app.manifest"));
        tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
            .expect("failed to run build script");
    }

    #[cfg(not(windows))]
    tauri_build::build();
}

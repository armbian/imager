fn main() {
    // Extract Tauri version from Cargo.toml and expose it as a compile-time env var
    println!("cargo:rustc-env=TAURI_VERSION={}", tauri_version());

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

/// Extract Tauri version from Cargo.toml dependencies
fn tauri_version() -> String {
    let cargo_toml = std::path::PathBuf::from("Cargo.toml");

    if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
        // Look for tauri dependency version
        for line in content.lines() {
            if line.contains("tauri =") || line.contains("tauri =") {
                // Extract version from line like: tauri = { version = "2.x", ... }
                if let Some(start) = line.find("version = \"") {
                    let after_version = &line[start + 11..];
                    if let Some(end) = after_version.find('"') {
                        return after_version[..end].to_string();
                    }
                }
            }
        }
    }

    // Fallback to unknown if parsing fails
    "unknown".to_string()
}

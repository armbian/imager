//! Settings persistence (theme, language, cache, etc.) via the Tauri Store plugin.

use crate::log_info;

const MODULE: &str = "commands::settings";

/// System information structure
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SystemInfo {
    pub platform: String,
    pub arch: String,
}

/// Get the real system platform and architecture
#[tauri::command]
pub fn get_system_info() -> SystemInfo {
    let platform = std::env::consts::OS.to_string();
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "ARM64",
        "x86" => "x86",
        "arm" => "ARM",
        _ => std::env::consts::ARCH,
    }
    .to_string();

    SystemInfo { platform, arch }
}

/// Get the Tauri framework version (compile-time constant from build.rs)
#[tauri::command]
pub fn get_tauri_version() -> String {
    env!("TAURI_VERSION").to_string()
}

/// Read only the last N lines of a file, to avoid loading large logs into memory
fn read_last_lines(path: &std::path::PathBuf, lines: usize) -> Result<String, String> {
    use std::io::{BufRead, BufReader};

    let file = std::fs::File::open(path).map_err(|e| format!("Failed to open log file: {}", e))?;

    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    let start = if all_lines.len() > lines {
        all_lines.len() - lines
    } else {
        0
    };

    Ok(all_lines[start..].join("\n"))
}

/// Get the latest log contents (last 10k lines when the file exceeds 5MB)
#[tauri::command]
pub fn get_logs() -> Result<String, String> {
    use crate::config::paste::{MAX_LOG_LINES, MAX_LOG_SIZE};
    use crate::logging;
    use std::fs::Metadata;

    match logging::get_current_log_path() {
        Some(log_path) => {
            if !log_path.exists() {
                return Ok("No log file found".to_string());
            }

            let metadata: Metadata = std::fs::metadata(&log_path)
                .map_err(|e| format!("Failed to read log file metadata: {}", e))?;

            if metadata.len() > MAX_LOG_SIZE {
                log_info!(
                    MODULE,
                    "Log file is large ({} bytes), reading last {} lines",
                    metadata.len(),
                    MAX_LOG_LINES
                );
                return read_last_lines(&log_path, MAX_LOG_LINES);
            }

            std::fs::read_to_string(&log_path)
                .map_err(|e| format!("Failed to read log file: {}", e))
        }
        None => Ok("No log file available".to_string()),
    }
}

// Cache Settings

/// Total size of all cached images in bytes.
#[tauri::command]
pub fn get_cache_size() -> Result<u64, String> {
    crate::cache::calculate_cache_size()
}

/// Cache size split into flashable images and assets (photos + API JSON).
#[tauri::command]
pub fn get_cache_breakdown() -> Result<crate::cache::CacheBreakdown, String> {
    crate::cache::calculate_cache_breakdown()
}

/// Remove all files from the image cache directory.
#[tauri::command]
pub fn clear_cache() -> Result<(), String> {
    crate::cache::clear_cache()
}

// Cache Manager

/// List cached images with metadata (filename, size, last used, parsed board)
#[tauri::command]
pub fn list_cached_images() -> Result<Vec<crate::cache::CachedImageInfo>, String> {
    crate::cache::list_cached_images()
}

/// Delete one cached image by filename, returning the new total cache size in bytes
#[tauri::command]
pub fn delete_cached_image(filename: String) -> Result<u64, String> {
    crate::cache::delete_cached_image(&filename)
}

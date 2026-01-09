//! Settings persistence commands using Tauri Store plugin
//!
//! Manages user preferences like theme and language using the Tauri Store plugin.

use crate::log_info;
use tauri_plugin_store::StoreExt;

const MODULE: &str = "commands::settings";
const SETTINGS_STORE: &str = "settings.json";
const MAX_LOG_SIZE: u64 = 5 * 1024 * 1024; // 5MB
const MAX_LOG_LINES: usize = 10_000;

/// Default values for settings
fn default_theme() -> String {
    "auto".to_string()
}

fn default_language() -> String {
    "auto".to_string()
}

fn default_show_motd() -> bool {
    true
}

fn default_show_updater_modal() -> bool {
    true
}

fn default_developer_mode() -> bool {
    false
}

fn default_cache_enabled() -> bool {
    true
}

fn default_cache_max_size() -> u64 {
    crate::cache::DEFAULT_MAX_SIZE
}

/// Get the current theme preference
#[tauri::command]
pub fn get_theme(app: tauri::AppHandle) -> String {
    match app.store(SETTINGS_STORE) {
        Ok(store) => match store.get("theme") {
            Some(value) => value.as_str().unwrap_or("auto").to_string(),
            None => {
                log_info!(MODULE, "Theme not found in store, using default");
                default_theme()
            }
        },
        Err(e) => {
            log_info!(MODULE, "Error loading store, using default theme: {}", e);
            default_theme()
        }
    }
}

/// Set the theme preference
#[tauri::command]
pub fn set_theme(theme: String, app: tauri::AppHandle) -> Result<(), String> {
    log_info!(MODULE, "Setting theme to: {}", theme);

    match app.store(SETTINGS_STORE) {
        Ok(store) => {
            store.set("theme", theme);
            Ok(())
        }
        Err(e) => Err(format!("Failed to access store: {}", e)),
    }
}

/// Get the current language preference
#[tauri::command]
pub fn get_language(app: tauri::AppHandle) -> String {
    match app.store(SETTINGS_STORE) {
        Ok(store) => match store.get("language") {
            Some(value) => value.as_str().unwrap_or("auto").to_string(),
            None => {
                log_info!(MODULE, "Language not found in store, using default");
                default_language()
            }
        },
        Err(e) => {
            log_info!(MODULE, "Error loading store, using default language: {}", e);
            default_language()
        }
    }
}

/// Set the language preference
#[tauri::command]
pub fn set_language(language: String, app: tauri::AppHandle) -> Result<(), String> {
    log_info!(MODULE, "Setting language to: {}", language);

    match app.store(SETTINGS_STORE) {
        Ok(store) => {
            store.set("language", language);
            Ok(())
        }
        Err(e) => Err(format!("Failed to access store: {}", e)),
    }
}

/// Get the MOTD visibility preference
#[tauri::command]
pub fn get_show_motd(app: tauri::AppHandle) -> bool {
    match app.store(SETTINGS_STORE) {
        Ok(store) => match store.get("show_motd") {
            Some(value) => value.as_bool().unwrap_or(true),
            None => {
                log_info!(MODULE, "show_motd not found in store, using default");
                default_show_motd()
            }
        },
        Err(e) => {
            log_info!(
                MODULE,
                "Error loading store, using default show_motd: {}",
                e
            );
            default_show_motd()
        }
    }
}

/// Set the MOTD visibility preference
#[tauri::command]
pub fn set_show_motd(show: bool, app: tauri::AppHandle) -> Result<(), String> {
    log_info!(MODULE, "Setting show_motd to: {}", show);

    match app.store(SETTINGS_STORE) {
        Ok(store) => {
            store.set("show_motd", show);
            Ok(())
        }
        Err(e) => Err(format!("Failed to access store: {}", e)),
    }
}

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

/// Get the Tauri version
///
/// Returns the Tauri framework version as a compile-time constant.
/// The version is extracted from Cargo.toml during build time via build.rs.
#[tauri::command]
pub fn get_tauri_version() -> String {
    env!("TAURI_VERSION").to_string()
}

/// Get the updater modal visibility preference
#[tauri::command]
pub fn get_show_updater_modal(app: tauri::AppHandle) -> bool {
    match app.store(SETTINGS_STORE) {
        Ok(store) => match store.get("show_updater_modal") {
            Some(value) => value.as_bool().unwrap_or(true),
            None => {
                log_info!(
                    MODULE,
                    "show_updater_modal not found in store, using default"
                );
                default_show_updater_modal()
            }
        },
        Err(e) => {
            log_info!(
                MODULE,
                "Error loading store, using default show_updater_modal: {}",
                e
            );
            default_show_updater_modal()
        }
    }
}

/// Set the updater modal visibility preference
#[tauri::command]
pub fn set_show_updater_modal(show: bool, app: tauri::AppHandle) -> Result<(), String> {
    log_info!(MODULE, "Setting show_updater_modal to: {}", show);

    match app.store(SETTINGS_STORE) {
        Ok(store) => {
            store.set("show_updater_modal", show);
            Ok(())
        }
        Err(e) => Err(format!("Failed to access store: {}", e)),
    }
}

/// Get the developer mode preference
#[tauri::command]
pub fn get_developer_mode(app: tauri::AppHandle) -> bool {
    match app.store(SETTINGS_STORE) {
        Ok(store) => match store.get("developer_mode") {
            Some(value) => value.as_bool().unwrap_or_else(default_developer_mode),
            None => {
                log_info!(MODULE, "developer_mode not found in store, using default");
                default_developer_mode()
            }
        },
        Err(e) => {
            log_info!(
                MODULE,
                "Error loading store, using default developer_mode: {}",
                e
            );
            default_developer_mode()
        }
    }
}

/// Set the developer mode preference
#[tauri::command]
pub fn set_developer_mode(enabled: bool, app: tauri::AppHandle) -> Result<(), String> {
    log_info!(MODULE, "Setting developer_mode to: {}", enabled);

    // Update the log level based on developer mode
    crate::logging::set_log_level(enabled);

    match app.store(SETTINGS_STORE) {
        Ok(store) => {
            store.set("developer_mode", enabled);
            Ok(())
        }
        Err(e) => Err(format!("Failed to access store: {}", e)),
    }
}

/// Read only the last N lines from a file to avoid loading large files into memory
///
/// This function is optimized for large log files by reading line-by-line
/// and only keeping the last N lines in memory.
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

/// Get the latest log file contents
///
/// For large log files (>5MB), only the last 10,000 lines are returned
/// to avoid memory issues. This prevents the application from consuming
/// excessive memory when viewing logs.
#[tauri::command]
pub fn get_logs() -> Result<String, String> {
    use crate::logging;
    use std::fs::Metadata;

    match logging::get_current_log_path() {
        Some(log_path) => {
            if !log_path.exists() {
                return Ok("No log file found".to_string());
            }

            // Get file metadata to check size
            let metadata: Metadata = std::fs::metadata(&log_path)
                .map_err(|e| format!("Failed to read log file metadata: {}", e))?;

            // For large files, use optimized line reader
            if metadata.len() > MAX_LOG_SIZE {
                log_info!(
                    MODULE,
                    "Log file is large ({} bytes), reading last {} lines",
                    metadata.len(),
                    MAX_LOG_LINES
                );
                return read_last_lines(&log_path, MAX_LOG_LINES);
            }

            // For small files, read entire contents
            std::fs::read_to_string(&log_path)
                .map_err(|e| format!("Failed to read log file: {}", e))
        }
        None => Ok("No log file available".to_string()),
    }
}

// ============================================================================
// Cache Settings
// ============================================================================

/// Get the cache enabled preference
///
/// Returns whether image caching is enabled (default: true).
#[tauri::command]
pub fn get_cache_enabled(app: tauri::AppHandle) -> bool {
    match app.store(SETTINGS_STORE) {
        Ok(store) => match store.get("cache_enabled") {
            Some(value) => value.as_bool().unwrap_or_else(default_cache_enabled),
            None => {
                log_info!(MODULE, "cache_enabled not found in store, using default");
                default_cache_enabled()
            }
        },
        Err(e) => {
            log_info!(
                MODULE,
                "Error loading store, using default cache_enabled: {}",
                e
            );
            default_cache_enabled()
        }
    }
}

/// Set the cache enabled preference
#[tauri::command]
pub fn set_cache_enabled(enabled: bool, app: tauri::AppHandle) -> Result<(), String> {
    log_info!(MODULE, "Setting cache_enabled to: {}", enabled);

    match app.store(SETTINGS_STORE) {
        Ok(store) => {
            store.set("cache_enabled", enabled);
            Ok(())
        }
        Err(e) => Err(format!("Failed to access store: {}", e)),
    }
}

/// Get the maximum cache size in bytes
///
/// Returns the configured maximum cache size (default: 20 GB).
#[tauri::command]
pub fn get_cache_max_size(app: tauri::AppHandle) -> u64 {
    match app.store(SETTINGS_STORE) {
        Ok(store) => match store.get("cache_max_size") {
            Some(value) => value.as_u64().unwrap_or_else(default_cache_max_size),
            None => {
                log_info!(MODULE, "cache_max_size not found in store, using default");
                default_cache_max_size()
            }
        },
        Err(e) => {
            log_info!(
                MODULE,
                "Error loading store, using default cache_max_size: {}",
                e
            );
            default_cache_max_size()
        }
    }
}

/// Set the maximum cache size in bytes
///
/// The size is validated to be between 1 GB and 500 GB.
#[tauri::command]
pub fn set_cache_max_size(size: u64, app: tauri::AppHandle) -> Result<(), String> {
    use crate::config::cache::{MAX_SIZE, MIN_SIZE};

    // Validate cache size bounds
    if size < MIN_SIZE {
        return Err(format!(
            "Cache size too small: {} bytes (minimum: {} bytes / 1 GB)",
            size, MIN_SIZE
        ));
    }

    if size > MAX_SIZE {
        return Err(format!(
            "Cache size too large: {} bytes (maximum: {} bytes / 100 GB)",
            size, MAX_SIZE
        ));
    }

    log_info!(MODULE, "Setting cache_max_size to: {} bytes", size);

    match app.store(SETTINGS_STORE) {
        Ok(store) => {
            store.set("cache_max_size", size);

            // Trigger eviction if needed
            if let Err(e) = crate::cache::evict_to_size(size) {
                log_info!(MODULE, "Failed to evict cache after size change: {}", e);
            }

            Ok(())
        }
        Err(e) => Err(format!("Failed to access store: {}", e)),
    }
}

/// Get the current cache size in bytes
///
/// Calculates and returns the total size of all cached images.
#[tauri::command]
pub fn get_cache_size() -> Result<u64, String> {
    crate::cache::calculate_cache_size()
}

/// Clear all cached images
///
/// Removes all files from the image cache directory.
#[tauri::command]
pub fn clear_cache() -> Result<(), String> {
    crate::cache::clear_cache()
}

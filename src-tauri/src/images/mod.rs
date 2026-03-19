//! Image management module
//!
//! Handles fetching, parsing, and filtering Armbian image data.
//! Caches the API response on disk for offline use.

mod filters;
mod models;

// Re-export types and functions
pub use filters::{extract_images, filter_images_for_board, get_unique_boards};
pub use models::{BoardInfo, ImageInfo};
// ArmbianImage is used internally by filters module

use crate::config;
use crate::utils::get_cache_dir;
use crate::{log_error, log_info, log_warn};

/// Path to the locally cached API response
pub(crate) fn get_api_cache_path() -> std::path::PathBuf {
    get_cache_dir(config::app::NAME)
        .join("assets")
        .join("api-images.json")
}

/// Fetch the all-images.json from Armbian, with local disk cache fallback.
///
/// On success: saves the response to disk for offline use.
/// On failure: loads the last saved response from disk.
/// If both fail: returns an error.
pub async fn fetch_all_images() -> Result<serde_json::Value, String> {
    log_info!(
        "images",
        "Fetching all images from {}",
        config::urls::ALL_IMAGES
    );

    // Try fetching from the API
    match fetch_from_api().await {
        Ok(json) => {
            // Save to disk for offline use (non-blocking, best-effort)
            save_api_cache(&json);
            Ok(json)
        }
        Err(e) => {
            log_warn!("images", "API fetch failed, trying local cache: {}", e);
            load_api_cache().await
        }
    }
}

/// Fetch the all-images.json directly from the remote Armbian API
///
/// Returns the parsed JSON on success, or an error string on network/parse failure.
async fn fetch_from_api() -> Result<serde_json::Value, String> {
    let response = reqwest::get(config::urls::ALL_IMAGES).await.map_err(|e| {
        log_error!("images", "Failed to fetch images: {}", e);
        format!("Failed to fetch images: {}", e)
    })?;

    let json: serde_json::Value = response.json().await.map_err(|e| {
        log_error!("images", "Failed to parse JSON response: {}", e);
        format!("Failed to parse JSON: {}", e)
    })?;

    log_info!("images", "Successfully fetched images data from API");
    Ok(json)
}

/// Save API response to disk for offline use
///
/// Serializes the JSON and writes it on a blocking thread to avoid
/// stalling the Tokio async runtime with synchronous file I/O.
/// Uses a temp file + rename pattern for atomic writes to prevent
/// partial reads if the app crashes mid-write.
fn save_api_cache(json: &serde_json::Value) {
    let path = get_api_cache_path();
    match serde_json::to_string(json) {
        Ok(data) => {
            std::thread::spawn(move || {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                // Write to temp file first, then rename for atomicity
                let tmp_path = path.with_extension("json.tmp");
                if let Err(e) = std::fs::write(&tmp_path, &data) {
                    log_warn!("images", "Failed to write API cache temp file: {}", e);
                    return;
                }
                if let Err(e) = std::fs::rename(&tmp_path, &path) {
                    log_warn!("images", "Failed to rename API cache file: {}", e);
                    let _ = std::fs::remove_file(&tmp_path);
                } else {
                    log_info!("images", "Saved API cache to {}", path.display());
                }
            });
        }
        Err(e) => {
            log_warn!("images", "Failed to serialize API cache: {}", e);
        }
    }
}

/// Load API response from disk cache
///
/// Uses async I/O to avoid blocking the Tokio runtime.
async fn load_api_cache() -> Result<serde_json::Value, String> {
    let path = get_api_cache_path();
    if !path.exists() {
        return Err("No cached API data available (first launch while offline)".to_string());
    }

    let data = tokio::fs::read_to_string(&path).await.map_err(|e| {
        log_error!("images", "Failed to read API cache: {}", e);
        format!("Failed to read cached data: {}", e)
    })?;

    let json: serde_json::Value = serde_json::from_str(&data).map_err(|e| {
        log_error!("images", "Failed to parse API cache: {}", e);
        format!("Failed to parse cached data: {}", e)
    })?;

    log_info!(
        "images",
        "Loaded API data from local cache ({})",
        path.display()
    );
    Ok(json)
}

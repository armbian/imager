//! Core operations module
//!
//! Handles download and flash operations.

use std::path::PathBuf;
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;

use crate::config;
use crate::download::download_image as do_download;
use crate::flash::{flash_image as do_flash, request_authorization};
use crate::utils::get_cache_dir;
use crate::{log_debug, log_error, log_info};

use super::state::AppState;

/// Request write authorization before starting the flash process
/// This shows the authorization dialog (Touch ID on macOS) BEFORE downloading
/// On Linux, if not root, this triggers pkexec to elevate and restart the app
/// Returns true if authorized, false if user cancelled
#[tauri::command]
pub async fn request_write_authorization(device_path: String) -> Result<bool, String> {
    log_info!(
        "operations",
        "Requesting write authorization for device: {}",
        device_path
    );
    let result = request_authorization(&device_path);
    match &result {
        Ok(authorized) => {
            if *authorized {
                log_info!("operations", "Authorization granted for {}", device_path);
            } else {
                log_info!(
                    "operations",
                    "Authorization denied/cancelled for {}",
                    device_path
                );
            }
        }
        Err(e) => {
            log_error!(
                "operations",
                "Authorization failed for {}: {}",
                device_path,
                e
            );
        }
    }
    result
}

/// Start downloading an image
#[tauri::command]
pub async fn download_image(
    file_url: String,
    file_url_sha: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    log_info!("operations", "Starting download: {}", file_url);
    log_debug!(
        "operations",
        "Download directory: {:?}",
        get_cache_dir(config::app::NAME).join("images")
    );
    if let Some(ref sha) = file_url_sha {
        log_info!("operations", "SHA URL: {}", sha);
    } else {
        log_info!("operations", "No SHA URL provided");
        log_debug!("operations", "SHA verification will be skipped");
    }
    let download_dir = get_cache_dir(config::app::NAME).join("images");

    let download_state = state.download_state.clone();
    let result = do_download(
        &file_url,
        file_url_sha.as_deref(),
        &download_dir,
        download_state,
    )
    .await;

    match &result {
        Ok(path) => {
            log_info!("operations", "Download completed: {}", path.display());
            Ok(path.to_string_lossy().to_string())
        }
        Err(e) => {
            log_error!("operations", "Download failed: {}", e);
            Err(e.clone())
        }
    }
}

/// Start flashing an image to a device
#[tauri::command]
pub async fn flash_image(
    image_path: String,
    device_path: String,
    verify: bool,
    state: State<'_, AppState>,
    _app: AppHandle,
) -> Result<(), String> {
    log_info!(
        "operations",
        "Starting flash: {} -> {} (verify: {})",
        image_path,
        device_path,
        verify
    );
    log_debug!(
        "operations",
        "Image path exists: {}",
        std::path::Path::new(&image_path).exists()
    );
    log_debug!(
        "operations",
        "Device path exists: {}",
        std::path::Path::new(&device_path).exists()
    );
    log_debug!("operations", "Verification enabled: {}", verify);

    let path = PathBuf::from(&image_path);
    let flash_state = state.flash_state.clone();

    let result = do_flash(&path, &device_path, flash_state, verify).await;

    match &result {
        Ok(_) => {
            log_info!("operations", "Flash completed successfully");
        }
        Err(e) => {
            log_error!("operations", "Flash failed: {}", e);
        }
    }

    result
}

/// Force delete a cached image regardless of cache settings
///
/// Used when an image repeatedly fails to flash, suggesting the cached
/// file may be corrupted. Bypasses the cache_enabled check.
#[tauri::command]
pub async fn force_delete_cached_image(image_path: String) -> Result<(), String> {
    log_info!("operations", "Force delete cached image: {}", image_path);

    let path = PathBuf::from(&image_path);

    // Safety check: only delete files in our cache directory
    // Use canonicalize() to resolve symlinks and prevent path traversal attacks
    let cache_dir = get_cache_dir(config::app::NAME)
        .canonicalize()
        .map_err(|e| format!("Failed to resolve cache directory: {}", e))?;

    let canonical_path = path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve image path: {}", e))?;

    if !canonical_path.starts_with(&cache_dir) {
        log_error!(
            "operations",
            "Attempted to force delete file outside cache: {} (resolved: {})",
            image_path,
            canonical_path.display()
        );
        return Err("Cannot delete files outside cache directory".to_string());
    }

    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| {
            log_error!(
                "operations",
                "Failed to force delete image {}: {}",
                image_path,
                e
            );
            format!("Failed to delete image: {}", e)
        })?;
        log_info!("operations", "Force deleted cached image: {}", image_path);
    } else {
        log_debug!("operations", "Image already deleted: {}", image_path);
    }

    Ok(())
}

/// Delete a downloaded image file
///
/// If image caching is enabled, the file is kept for future use.
/// If caching is disabled, the file is deleted.
#[tauri::command]
pub async fn delete_downloaded_image(image_path: String, app: AppHandle) -> Result<(), String> {
    log_info!("operations", "Delete request for image: {}", image_path);

    // Check if cache is enabled
    let cache_enabled = match app.store("settings.json") {
        Ok(store) => store
            .get("cache_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        Err(_) => true, // Default to cache enabled
    };

    if cache_enabled {
        log_info!("operations", "Cache enabled, keeping image: {}", image_path);
        return Ok(());
    }

    let path = PathBuf::from(&image_path);

    // Safety check: only delete files in our cache directory
    // Use canonicalize() to resolve symlinks and prevent path traversal attacks
    let cache_dir = match get_cache_dir(config::app::NAME).canonicalize() {
        Ok(dir) => dir,
        Err(e) => {
            log_debug!(
                "operations",
                "Cache directory doesn't exist yet, skipping delete: {}",
                e
            );
            return Ok(());
        }
    };

    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            log_debug!(
                "operations",
                "Image path doesn't exist or can't be resolved, skipping delete: {}",
                e
            );
            return Ok(());
        }
    };

    if !canonical_path.starts_with(&cache_dir) {
        log_error!(
            "operations",
            "Attempted to delete file outside cache: {} (resolved: {})",
            image_path,
            canonical_path.display()
        );
        return Err("Cannot delete files outside cache directory".to_string());
    }

    if canonical_path.exists() {
        std::fs::remove_file(&canonical_path).map_err(|e| {
            log_error!("operations", "Failed to delete image {}: {}", image_path, e);
            format!("Failed to delete image: {}", e)
        })?;
        log_info!("operations", "Deleted image: {}", image_path);
    }

    Ok(())
}

/// Continue a download that failed due to SHA unavailable
/// Uses the already downloaded file without re-downloading
#[tauri::command]
pub async fn continue_download_without_sha(state: State<'_, AppState>) -> Result<String, String> {
    log_info!("operations", "Continuing download without SHA verification");

    let download_dir = get_cache_dir(config::app::NAME).join("images");
    let download_state = state.download_state.clone();

    let result = crate::download::continue_without_sha(download_state, &download_dir).await;

    match &result {
        Ok(path) => {
            log_info!("operations", "Continue completed: {}", path.display());
            Ok(path.to_string_lossy().to_string())
        }
        Err(e) => {
            log_error!("operations", "Continue failed: {}", e);
            Err(e.clone())
        }
    }
}

/// Clean up a failed download (delete temp file)
/// Called when user cancels after SHA unavailable error
#[tauri::command]
pub async fn cleanup_failed_download(state: State<'_, AppState>) -> Result<(), String> {
    log_info!("operations", "Cleaning up failed download");
    crate::download::cleanup_pending_download(state.download_state.clone()).await;
    Ok(())
}

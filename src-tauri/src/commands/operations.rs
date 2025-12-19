//! Core operations module
//!
//! Handles download and flash operations.

use std::path::PathBuf;
use tauri::State;

use crate::config;
use crate::download::download_image as do_download;
use crate::flash::{flash_image as do_flash, request_authorization};
use crate::utils::get_cache_dir;
use crate::{log_error, log_info};

use super::state::AppState;

/// Request write authorization before starting the flash process
/// This shows the authorization dialog (Touch ID on macOS) BEFORE downloading
/// On Linux, if not root, this triggers pkexec to elevate and restart the app
/// Returns true if authorized, false if user cancelled
#[tauri::command]
pub async fn request_write_authorization(device_path: String) -> Result<bool, String> {
    log_info!("operations", "Requesting write authorization for device: {}", device_path);
    let result = request_authorization(&device_path);
    match &result {
        Ok(authorized) => {
            if *authorized {
                log_info!("operations", "Authorization granted for {}", device_path);
            } else {
                log_info!("operations", "Authorization denied/cancelled for {}", device_path);
            }
        }
        Err(e) => {
            log_error!("operations", "Authorization failed for {}: {}", device_path, e);
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
    if let Some(ref sha) = file_url_sha {
        log_info!("operations", "SHA URL: {}", sha);
    } else {
        log_info!("operations", "No SHA URL provided");
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
) -> Result<(), String> {
    log_info!("operations", "Starting flash: {} -> {} (verify: {})", image_path, device_path, verify);
    let path = PathBuf::from(&image_path);
    let flash_state = state.flash_state.clone();

    let result = do_flash(&path, &device_path, flash_state, verify).await;
    if let Err(ref e) = result {
        log_error!("operations", "Flash failed: {}", e);
    } else {
        log_info!("operations", "Flash completed successfully");
    }
    result
}

/// Delete a downloaded image file
#[tauri::command]
pub async fn delete_downloaded_image(image_path: String) -> Result<(), String> {
    log_info!("operations", "Deleting downloaded image: {}", image_path);
    let path = PathBuf::from(&image_path);

    // Safety check: only delete files in our cache directory
    let cache_dir = get_cache_dir(config::app::NAME);

    if !path.starts_with(&cache_dir) {
        log_error!("operations", "Attempted to delete file outside cache: {}", image_path);
        return Err("Cannot delete files outside cache directory".to_string());
    }

    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| {
            log_error!("operations", "Failed to delete image {}: {}", image_path, e);
            format!("Failed to delete image: {}", e)
        })?;
        log_info!("operations", "Deleted image: {}", image_path);
    }

    Ok(())
}

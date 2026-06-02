//! Download and flash operations.

use std::path::PathBuf;
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;

use armbian_write_conf::WriteConfError;

use crate::autoconfig::AutoconfigConfig;
use crate::download::download_image as do_download;
use crate::flash::{flash_image as do_flash, request_authorization};
use crate::utils::{app_cache_dir, images_dir, validate_cache_path};
use crate::{log_debug, log_error, log_info, log_warn};

use super::state::AppState;

/// Request write authorization before download (Touch ID on macOS, pkexec re-launch
/// on Linux when not root). Returns false if the user cancels.
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
    sha_url: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    log_info!("operations", "Starting download: {}", file_url);
    log_debug!("operations", "Download directory: {:?}", images_dir());
    if let Some(ref sha) = sha_url {
        log_debug!("operations", "SHA URL: {}", sha);
    } else {
        log_debug!(
            "operations",
            "No SHA URL provided; verification will be skipped"
        );
    }
    let download_dir = images_dir();

    let download_state = state.download_state.clone();
    let result = do_download(&file_url, sha_url.as_deref(), &download_dir, download_state).await;

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

/// Start flashing an image to a device. With `autoconfig` Some, injects the Armbian first-boot preset
/// into a per-flash copy (original never mutated) and flashes that; None flashes the original directly.
#[tauri::command]
pub async fn flash_image(
    image_path: String,
    device_path: String,
    verify: bool,
    autoconfig: Option<AutoconfigConfig>,
    state: State<'_, AppState>,
    _app: AppHandle,
) -> Result<(), String> {
    log_info!(
        "operations",
        "Starting flash: {} -> {} (verify: {}, autoconfig: {})",
        image_path,
        device_path,
        verify,
        autoconfig.is_some()
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

    // Reset shared progress before auth/copy/unmount: frontend polls on invoke, so without an early
    // reset it reads the previous flash's stale state (is_verifying=true, verified=100%) and latches onto it.
    flash_state.reset();

    // With a profile selected, flash a temp copy with the preset injected so the
    // shared cached/decompressed image stays pristine.
    let (flash_path, temp_copy) = match autoconfig {
        Some(config) => {
            let copy = prepare_autoconfig_copy(&path, &config)?;
            (copy.clone(), Some(copy))
        }
        None => (path, None),
    };

    let result = do_flash(&flash_path, &device_path, flash_state, verify).await;

    // Always remove the temp copy, regardless of flash outcome.
    if let Some(copy) = temp_copy {
        if let Err(e) = std::fs::remove_file(&copy) {
            log_warn!(
                "operations",
                "Failed to remove autoconfig temp copy {}: {}",
                copy.display(),
                e
            );
        }
    }

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

/// Copy the decompressed image to a per-flash temp file and inject the autoconfig preset into the copy.
/// Aborts (deleting the copy) if the image has no writable ext4 rootfs, since a profile was requested.
fn prepare_autoconfig_copy(
    source: &std::path::Path,
    config: &AutoconfigConfig,
) -> Result<PathBuf, String> {
    let temp_dir = app_cache_dir().join("autoconfig-temp");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create autoconfig temp directory: {}", e))?;

    // Unique per-flash name to avoid collisions across concurrent/repeat flashes.
    let stem = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "image.img".to_string());
    let unique = format!(
        "{}.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
        stem
    );
    let copy_path = temp_dir.join(unique);

    log_info!(
        "operations",
        "Copying image for autoconfig injection: {} -> {}",
        source.display(),
        copy_path.display()
    );
    std::fs::copy(source, &copy_path)
        .map_err(|e| format!("Failed to copy image for autoconfig: {}", e))?;

    if let Err(e) = crate::autoconfig::inject_into_image(&copy_path, config) {
        // Clean up the half-prepared copy before bubbling up.
        let _ = std::fs::remove_file(&copy_path);
        let message = match e {
            WriteConfError::UnsupportedImage(_) | WriteConfError::NoExt4Rootfs(_) => format!(
                "This image does not have a writable ext4 root filesystem, so the selected autoconfig profile cannot be applied: {}",
                e
            ),
            other => format!("Failed to apply autoconfig profile: {}", other),
        };
        return Err(message);
    }

    Ok(copy_path)
}

/// Force-delete a cached image (bypasses cache_enabled), for when a file looks corrupted
#[tauri::command]
pub async fn force_delete_cached_image(image_path: String) -> Result<(), String> {
    log_info!("operations", "Force delete cached image: {}", image_path);

    let path = PathBuf::from(&image_path);

    // Refuse to delete anything outside our cache directory.
    if let Err(e) = validate_cache_path(&path) {
        log_error!(
            "operations",
            "Attempted to force delete file outside cache: {}: {}",
            image_path,
            e
        );
        return Err(e);
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

/// Delete a downloaded image, unless caching is enabled (then it is kept for reuse)
#[tauri::command]
pub async fn delete_downloaded_image(image_path: String, app: AppHandle) -> Result<(), String> {
    log_info!("operations", "Delete request for image: {}", image_path);

    let cache_enabled = match app.store("settings.json") {
        Ok(store) => store
            .get("cache_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        Err(_) => true, // Default to cache enabled.
    };

    if cache_enabled {
        log_info!("operations", "Cache enabled, keeping image: {}", image_path);
        return Ok(());
    }

    let path = PathBuf::from(&image_path);

    // Refuse to delete anything outside our cache directory.
    let canonical_path = match validate_cache_path(&path) {
        Ok(p) => p,
        Err(e) => {
            // Nothing to delete if the path or cache dir is already gone.
            if !path.exists() || !app_cache_dir().exists() {
                log_debug!(
                    "operations",
                    "Path or cache directory doesn't exist, skipping delete: {}",
                    e
                );
                return Ok(());
            }
            log_error!(
                "operations",
                "Attempted to delete file outside cache: {}: {}",
                image_path,
                e
            );
            return Err(e);
        }
    };

    if canonical_path.exists() {
        std::fs::remove_file(&canonical_path).map_err(|e| {
            log_error!("operations", "Failed to delete image {}: {}", image_path, e);
            format!("Failed to delete image: {}", e)
        })?;
        log_info!("operations", "Deleted image: {}", image_path);
    }

    Ok(())
}

/// Finish a download that stalled on an unavailable SHA, reusing the downloaded file.
#[tauri::command]
pub async fn continue_download_without_sha(state: State<'_, AppState>) -> Result<String, String> {
    log_info!("operations", "Continuing download without SHA verification");

    let download_dir = images_dir();
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

/// Delete the temp file from a failed download (user cancelled after SHA-unavailable).
#[tauri::command]
pub async fn cleanup_failed_download(state: State<'_, AppState>) -> Result<(), String> {
    log_info!("operations", "Cleaning up failed download");
    crate::download::cleanup_pending_download(state.download_state.clone()).await;
    Ok(())
}

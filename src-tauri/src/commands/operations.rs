//! Core operations module
//!
//! Handles download and flash operations.

use std::path::PathBuf;
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;

use crate::config;
use crate::download::download_image as do_download;
use crate::flash::{flash_image as do_flash, request_authorization};
use crate::utils::{get_cache_dir, validate_cache_path};
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
    let canonical_path = match validate_cache_path(&path) {
        Ok(p) => p,
        Err(e) => {
            // If path or cache dir doesn't exist, skip silently
            if !path.exists() || !get_cache_dir(config::app::NAME).exists() {
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

/// Inject the autoconfig payload dynamically onto the newly flashed Linux filesystem
#[tauri::command]
pub async fn inject_autoconfig(device_path: String, payload: String) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        use std::fs::{self, File};
        use std::io::Write;
        use std::thread;
        use std::time::Duration;

        log_info!("operations", "Injecting armbian-firstlogin.conf to block device: {}", device_path);

        // 1. Force the kernel to re-read the partition table
        log_info!("operations", "Running partprobe to refresh partitions on {}", device_path);
        let _ = Command::new("partprobe").arg(&device_path).status();
        
        // Brief sleep to allow dev handler to surface the partition block node
        thread::sleep(Duration::from_secs(3));

        // 2. Identify the boot partition
        // On SD cards/eMMC (mmcblkX) or NVMe (nvmeXn1), partition 1 usually adds 'p1'.
        // On USB/SATA (sdX), partition 1 usually adds '1'.
        let part_suffix = if device_path.contains("mmcblk") || device_path.contains("nvme") {
            "p1"
        } else {
            "1"
        };
        let part_path = format!("{}{}", device_path, part_suffix);
        log_info!("operations", "Targeting partition: {}", part_path);

        // 3. Setup temporary mount directory
        let mount_dir = "/tmp/armbian-config-mount";
        let _ = fs::create_dir_all(mount_dir);

        // 4. Mount partition
        log_info!("operations", "Mounting {} to {}", part_path, mount_dir);
        let mount_status = Command::new("mount")
            .arg(&part_path)
            .arg(mount_dir)
            .status()
            .map_err(|e| format!("Failed to execute mount: {}", e))?;

        if !mount_status.success() {
            log_error!("operations", "Failed to mount partition {}", part_path);
            return Err("Failed to mount target ext4 partition".to_string());
        }

        // 5. Determine target path and write configuration
        // Armbian traditionally checks /boot/armbian-firstlogin.conf, or fallback to /armbian-firstlogin.conf
        let boot_dir = format!("{}/boot", mount_dir);
        let target_file_path = if std::path::Path::new(&boot_dir).exists() {
            format!("{}/armbian-firstlogin.conf", boot_dir)
        } else {
            format!("{}/armbian-firstlogin.conf", mount_dir)
        };

        log_info!("operations", "Writing configuration payload to {}", target_file_path);
        
        let write_result = (|| -> std::io::Result<()> {
            let mut file = File::create(&target_file_path)?;
            file.write_all(payload.as_bytes())?;
            file.sync_all()?;
            Ok(())
        })();

        if let Err(e) = write_result {
            log_error!("operations", "Failed to write payload: {}", e);
        } else {
            log_info!("operations", "Successfully wrote autoconfig payload.");
        }

        // 6. Cleanup: Unmount directory
        log_info!("operations", "Unmounting {}", mount_dir);
        let umount_status = Command::new("umount")
            .arg(mount_dir)
            .status()
            .map_err(|e| format!("Failed to execute umount: {}", e))?;

        if !umount_status.success() {
            log_error!("operations", "Warning: Failed to gracefully unmount {}", mount_dir);
        }

        let _ = fs::remove_dir(mount_dir); // Attempt simple cleanup

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        // On non-Linux, this should not be called dynamically as the frontend catches it.
        Err("Autoconfig dynamic injection is only supported on Linux native hosts.".to_string())
    }
}

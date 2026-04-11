//! QDL operations module
//!
//! Tauri command handlers for QDL (Qualcomm EDL) device detection and flashing.

use std::path::PathBuf;
use tauri::State;

use crate::qdl;
use crate::qdl::QdlDevice;
use crate::utils::get_cache_dir;
use crate::{config, log_error, log_info};

use super::state::AppState;

/// Detect QDL (Qualcomm EDL) devices connected via USB
///
/// Scans for USB devices with Qualcomm EDL vendor/product IDs (05c6:9008).
/// Returns an empty list if no devices are found.
#[tauri::command]
pub async fn get_qdl_devices() -> Result<Vec<QdlDevice>, String> {
    qdl::detect::get_qdl_devices()
}

/// Flash a QDL image (TAR archive) to a device in EDL mode
///
/// Pipeline: Extract TAR → Connect USB → Sahara → Firehose → Reset
///
/// # Arguments
/// * `tar_path` - Path to the downloaded TAR archive containing flash files
/// * `serial` - Optional USB serial number to target a specific device
#[tauri::command]
pub async fn flash_qdl_image(
    tar_path: String,
    serial: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log_info!("qdl_operations", "Starting QDL flash: {}", tar_path);

    let flash_state = state.flash_state.clone();
    flash_state.reset();

    // Extract TAR archive to temp directory
    let tar_path = PathBuf::from(&tar_path);
    let extract_dir = get_cache_dir(config::app::NAME).join("qdl-temp");

    let flash_dir = qdl::extract::extract_qdl_archive(&tar_path, &extract_dir).map_err(|e| {
        log_error!("qdl_operations", "TAR extraction failed: {}", e);
        e
    })?;

    log_info!(
        "qdl_operations",
        "Extracted flash files to: {}",
        flash_dir.display()
    );

    // Run QDL flash in a blocking task (qdlrs API is synchronous)
    let flash_dir_clone = flash_dir.clone();
    let result = tokio::task::spawn_blocking(move || {
        qdl::flash::qdl_flash(&flash_dir_clone, serial, flash_state)
    })
    .await
    .map_err(|e| {
        let msg = e.to_string();
        // Return tagged error codes so the frontend can map them to i18n keys
        if msg.contains("Error sending data") || msg.contains("Error receiving data") {
            "[QDL_DISCONNECTED]".to_string()
        } else if msg.contains("cancelled") || msg.contains("Interrupted") {
            "[QDL_CANCELLED]".to_string()
        } else {
            format!("[QDL_ERROR] {}", msg)
        }
    })?;

    // Clean up extraction directory
    qdl::extract::cleanup_extraction(&extract_dir);

    match &result {
        Ok(()) => {
            log_info!("qdl_operations", "QDL flash completed successfully");
        }
        Err(e) => {
            log_error!("qdl_operations", "QDL flash failed: {}", e);
        }
    }

    result
}

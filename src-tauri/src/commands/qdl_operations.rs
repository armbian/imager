//! Tauri command handlers for QDL (Qualcomm EDL) device detection and flashing.

use std::path::PathBuf;
use tauri::State;

use crate::qdl;
use crate::qdl::QdlDevice;
use crate::utils::qdl_temp_dir;
use crate::{log_error, log_info, log_warn};

use super::state::AppState;

/// Detect connected USB devices in Qualcomm EDL mode (VID:PID 05c6:9008); empty list if none.
#[tauri::command]
pub async fn get_qdl_devices() -> Result<Vec<QdlDevice>, String> {
    qdl::detect::get_qdl_devices()
}

/// Flash a QDL image (TAR archive) to a device in EDL mode. Pipeline: Extract TAR
/// -> Connect USB -> Sahara -> Firehose -> Reset. `serial` optionally targets one device.
#[tauri::command]
pub async fn flash_qdl_image(
    tar_path: String,
    serial: Option<String>,
    autoconfig: Option<crate::autoconfig::AutoconfigConfig>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log_info!("qdl_operations", "Starting QDL flash: {}", tar_path);

    let flash_state = state.flash_state.clone();
    flash_state.reset();

    let tar_path = PathBuf::from(&tar_path);
    let extract_dir = qdl_temp_dir();

    let flash_dir = qdl::extract::extract_qdl_archive(&tar_path, &extract_dir).map_err(|e| {
        log_error!("qdl_operations", "TAR extraction failed: {}", e);
        e
    })?;

    log_info!(
        "qdl_operations",
        "Extracted flash files to: {}",
        flash_dir.display()
    );

    // qdlrs is synchronous, so run the flash off the async runtime.
    let flash_dir_clone = flash_dir.clone();
    let result = tokio::task::spawn_blocking(move || {
        qdl::flash::qdl_flash(&flash_dir_clone, serial, autoconfig, flash_state)
    })
    .await
    .map_err(|e| {
        let msg = e.to_string();
        // Tag error codes so the frontend can map them to i18n keys.
        if msg.contains("Error sending data") || msg.contains("Error receiving data") {
            "[QDL_DISCONNECTED]".to_string()
        } else if msg.contains("cancelled") || msg.contains("Interrupted") {
            "[QDL_CANCELLED]".to_string()
        } else {
            format!("[QDL_ERROR] {}", msg)
        }
    })?;

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

/// Flash a UFS image (a downloaded + decompressed `.img`) to a device in EDL mode via a
/// single raw Firehose write. The loader is resolved from `soc`, then from `board_slug`,
/// since the Armbian API reports `soc` as null for these boards.
#[tauri::command]
pub async fn flash_qdl_ufs_image(
    image_path: String,
    soc: String,
    board_slug: String,
    serial: Option<String>,
    autoconfig: Option<crate::autoconfig::AutoconfigConfig>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log_info!("qdl_operations", "Starting QDL UFS flash: {}", image_path);

    let flash_state = state.flash_state.clone();
    flash_state.reset();

    // Resolve the board's QDL facts from the API (bundled fallback), then fetch the
    // loader and provisioning descriptor from the API blob proxy with digest checks.
    let resolved = qdl::registry::resolve(&board_slug).await.ok_or_else(|| {
        let msg = format!("No QDL metadata for board '{board_slug}' (SoC hint '{soc}')");
        log_error!("qdl_operations", "{}", msg);
        format!("[QDL_ERROR] {msg}")
    })?;

    if resolved.storage != qdl::QdlStorage::Ufs {
        return Err(format!(
            "[QDL_ERROR] Board '{board_slug}' is not a UFS QDL target"
        ));
    }

    let loader_path = qdl::loader::ensure_loader(&resolved).await.map_err(|e| {
        log_error!("qdl_operations", "Firehose loader unavailable: {}", e);
        e
    })?;

    // Provisioning descriptor for setting up a brand-new (unprovisioned) module.
    let provision = qdl::provision::ensure_provision_xml(&resolved).await;
    if let qdl::provision::ProvisionSource::Unavailable(reason) = &provision {
        log_warn!("qdl_operations", "Provision XML unavailable: {}", reason);
    }

    let image_path = PathBuf::from(&image_path);
    // qdlrs is synchronous, so run the flash off the async runtime.
    let result = tokio::task::spawn_blocking(move || {
        qdl::flash::qdl_flash_ufs(
            &image_path,
            &loader_path,
            serial,
            autoconfig,
            provision,
            flash_state,
        )
    })
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("Error sending data") || msg.contains("Error receiving data") {
            "[QDL_DISCONNECTED]".to_string()
        } else if msg.contains("cancelled") || msg.contains("Interrupted") {
            "[QDL_CANCELLED]".to_string()
        } else {
            format!("[QDL_ERROR] {}", msg)
        }
    })?;

    match &result {
        Ok(()) => log_info!("qdl_operations", "QDL UFS flash completed successfully"),
        Err(e) => log_error!("qdl_operations", "QDL UFS flash failed: {}", e),
    }
    result
}

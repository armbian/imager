// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Tauri command handlers for QDL (Qualcomm EDL) device detection and flashing.

use std::path::PathBuf;
use tauri::State;

use crate::autoconfig::{prepare_flash_copy, PrepError};
use crate::flash::reject_simulated;
use crate::qdl;
use crate::qdl::QdlDevice;
use crate::utils::{autoconfig_temp_dir, qdl_temp_dir};
use crate::{log_error, log_info, log_warn};

use super::state::AppState;

/// Tag error codes so the frontend can map them to i18n keys.
fn tag_join_error(e: tokio::task::JoinError) -> String {
    let msg = e.to_string();
    if msg.contains("Error sending data") || msg.contains("Error receiving data") {
        qdl::TAG_QDL_DISCONNECTED.to_string()
    } else if msg.contains("cancelled") || msg.contains("Interrupted") {
        qdl::TAG_QDL_CANCELLED.to_string()
    } else {
        format!("{} {}", qdl::TAG_QDL_ERROR, msg)
    }
}

/// Detect connected USB devices in Qualcomm EDL mode (VID:PID 05c6:9008); empty list if none.
#[tauri::command]
pub async fn get_qdl_devices() -> Result<Vec<QdlDevice>, String> {
    #[cfg(debug_assertions)]
    if let Some(devices) = super::dev_scenarios::qdl_devices_override() {
        return Ok(devices);
    }
    qdl::detect::get_qdl_devices()
}

/// Fail fast before extraction or downloads; the flash re-resolves right before connecting.
fn check_edl_target(device_path: &str) -> Result<(), String> {
    reject_simulated(device_path)
        .and_then(|_| qdl::detect::resolve_edl_serial(device_path))
        .map(|_| ())
        .map_err(|e| {
            log_error!(
                "qdl_operations",
                "Refusing EDL target {:?}: {}",
                device_path,
                e
            );
            e
        })
}

/// Flash a QDL image (TAR archive) to the EDL device at `device_path` (`qdl://...` from
/// `get_qdl_devices`). Pipeline: Extract TAR -> Connect USB -> Sahara -> Firehose -> Reset.
#[tauri::command]
pub async fn flash_qdl_image(
    tar_path: String,
    device_path: String,
    autoconfig: Option<crate::autoconfig::AutoconfigConfig>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log_info!(
        "qdl_operations",
        "Starting QDL flash: {} -> {}",
        tar_path,
        device_path
    );

    let flash_state = state.flash_state.clone();
    flash_state.reset();

    #[cfg(debug_assertions)]
    if let Some(result) = super::dev_scenarios::intercept_qdl_flash(
        crate::dev_scenarios::flash_sim::QdlKind::Tar,
        std::path::Path::new(&tar_path),
        &device_path,
        flash_state.clone(),
    )
    .await
    {
        return result;
    }

    check_edl_target(&device_path)?;

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
    let extract_root = qdl::extract::extraction_dir(&extract_dir);
    let result = tokio::task::spawn_blocking(move || {
        qdl::flash::qdl_flash(
            &flash_dir_clone,
            &extract_root,
            &device_path,
            autoconfig,
            flash_state,
        )
    })
    .await
    .map_err(tag_join_error)?;

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

/// Flash a UFS image (a downloaded + decompressed `.img`) to the EDL device at `device_path`
/// via a single raw Firehose write. The loader is resolved from `soc`, then from `board_slug`,
/// since the Armbian API reports `soc` as null for these boards.
#[tauri::command]
pub async fn flash_qdl_ufs_image(
    image_path: String,
    soc: String,
    board_slug: String,
    device_path: String,
    autoconfig: Option<crate::autoconfig::AutoconfigConfig>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log_info!(
        "qdl_operations",
        "Starting QDL UFS flash: {} -> {}",
        image_path,
        device_path
    );

    let flash_state = state.flash_state.clone();
    flash_state.reset();

    let image_path = PathBuf::from(&image_path);

    #[cfg(debug_assertions)]
    if let Some(result) = super::dev_scenarios::intercept_qdl_flash(
        crate::dev_scenarios::flash_sim::QdlKind::Ufs,
        std::path::Path::new(&image_path),
        &device_path,
        flash_state.clone(),
    )
    .await
    {
        return result;
    }

    check_edl_target(&device_path)?;

    // Resolve the board's QDL facts from the API (bundled fallback), then fetch the
    // loader and provisioning descriptor from the API blob proxy with digest checks.
    let resolved = qdl::registry::resolve(&board_slug).await.ok_or_else(|| {
        let msg = format!("No QDL metadata for board '{board_slug}' (SoC hint '{soc}')");
        log_error!("qdl_operations", "{}", msg);
        format!("{} {msg}", qdl::TAG_QDL_ERROR)
    })?;

    if resolved.storage != qdl::QdlStorage::Ufs {
        return Err(format!(
            "{} Board '{board_slug}' is not a UFS QDL target",
            qdl::TAG_QDL_ERROR
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

    // Inject into a per-flash copy before connecting, so the cached image stays pristine.
    let working_copy = match autoconfig {
        Some(cfg) => Some(
            // Catalog images always carry the script, so the marker probe only refuses custom images built without it.
            prepare_flash_copy(
                image_path.clone(),
                autoconfig_temp_dir(),
                cfg,
                true,
                flash_state.clone(),
            )
            .await
            .map_err(|e| match e {
                PrepError::Cancelled => qdl::QDL_CANCELLED_ERROR.to_string(),
                PrepError::Failed(message) => {
                    format!("{} {message}", qdl::TAG_QDL_AUTOCONFIG_FAILED)
                }
            })?,
        ),
        None => None,
    };
    qdl::flash::check_cancelled(&flash_state)?;
    let write_path = working_copy
        .as_ref()
        .map_or(image_path, |copy| copy.path().to_path_buf());

    // qdlrs is synchronous, so run the flash off the async runtime.
    let result = tokio::task::spawn_blocking(move || {
        qdl::flash::qdl_flash_ufs(
            &write_path,
            &loader_path,
            &device_path,
            provision,
            flash_state,
        )
    })
    .await
    .map_err(tag_join_error)?;

    drop(working_copy);

    match &result {
        Ok(()) => log_info!("qdl_operations", "QDL UFS flash completed successfully"),
        Err(e) => log_error!("qdl_operations", "QDL UFS flash failed: {}", e),
    }
    result
}

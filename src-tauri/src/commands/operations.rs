// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

//! Download and flash operations.

use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;

use crate::autoconfig::{prepare_flash_copy, AutoconfigConfig, PrepError};
use crate::config;
use crate::devices::target::TAG_NOT_FOUND;
use crate::devices::{get_block_devices, select_flash_target, FlashTarget, TargetRefusal};
use crate::download::download_image as do_download;
use crate::flash::{
    check_capacity, discard_saved_authorization, flash_image as do_flash, reject_simulated,
    request_authorization, FlashState,
};
use crate::utils::{
    app_cache_dir, autoconfig_temp_dir, image_size, images_dir, validate_cache_path,
};
use crate::{log_debug, log_error, log_info, log_warn};

use super::state::AppState;

/// The system-disk unlock; anything but an explicit `true` keeps system disks refused.
pub(crate) fn allow_system_devices(app: &AppHandle) -> bool {
    allow_system_from(
        app.store(config::app::SETTINGS_STORE)
            .map(|store| store.get(config::app::SETTING_ALLOW_SYSTEM_DEVICES)),
    )
}

fn allow_system_from<E: std::fmt::Display>(setting: Result<Option<serde_json::Value>, E>) -> bool {
    match setting {
        Ok(value) => value.and_then(|v| v.as_bool()).unwrap_or(false),
        Err(e) => {
            log_warn!(
                "operations",
                "Settings store unavailable, system devices stay blocked: {}",
                e
            );
            false
        }
    }
}

/// Validate a frontend device path against a fresh scan; only a missing device is rescanned.
async fn resolve_flash_target(
    app: &AppHandle,
    device_path: &str,
    expected_size: u64,
) -> Result<FlashTarget, String> {
    let refuse = |e: String| {
        log_error!("operations", "Refusing target {:?}: {}", device_path, e);
        e
    };

    reject_simulated(device_path).map_err(refuse)?;

    let allow_system = allow_system_devices(app);
    log_info!(
        "operations",
        "Validating target {:?} ({} bytes), allow_system_devices={}",
        device_path,
        expected_size,
        allow_system
    );

    let mut rescans = 0;
    loop {
        let devices = get_block_devices()
            .map_err(|e| refuse(format!("{TAG_NOT_FOUND} device scan failed: {e}")))?;
        match select_flash_target(device_path, expected_size, &devices, allow_system) {
            Ok(target) => return Ok(target),
            Err(TargetRefusal::NotFound(_)) if rescans < config::flash::TARGET_RESCAN_RETRIES => {
                rescans += 1;
                log_debug!(
                    "operations",
                    "Target {:?} not in scan, rescan {}/{}",
                    device_path,
                    rescans,
                    config::flash::TARGET_RESCAN_RETRIES
                );
                tokio::time::sleep(std::time::Duration::from_millis(
                    config::flash::TARGET_RESCAN_INTERVAL_MS,
                ))
                .await;
            }
            Err(refusal) => return Err(refuse(refusal.into_message())),
        }
    }
}

/// Request write authorization before download (Touch ID on macOS, pkexec re-launch
/// on Linux when not root). Returns false if the user cancels.
#[tauri::command]
pub async fn request_write_authorization(
    device_path: String,
    expected_size: u64,
    app: AppHandle,
) -> Result<bool, String> {
    log_info!(
        "operations",
        "Requesting write authorization for device: {}",
        device_path
    );
    #[cfg(debug_assertions)]
    if let Some(result) =
        super::dev_scenarios::intercept_authorization(&app, &device_path, expected_size).await
    {
        return result;
    }

    let target = resolve_flash_target(&app, &device_path, expected_size).await?;
    let result = request_authorization(&target);
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
    expected_size: u64,
    verify: bool,
    autoconfig: Option<AutoconfigConfig>,
    state: State<'_, AppState>,
    app: AppHandle,
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

    #[cfg(debug_assertions)]
    if let Some(result) = super::dev_scenarios::intercept_flash(
        &app,
        &path,
        &device_path,
        expected_size,
        verify,
        autoconfig.as_ref(),
        flash_state.clone(),
    )
    .await
    {
        return result;
    }

    let result = prepare_and_flash(
        &app,
        path,
        &device_path,
        expected_size,
        verify,
        autoconfig,
        flash_state,
    )
    .await;
    // The writer consumes the authorization; any earlier exit leaves it saved.
    discard_saved_authorization();

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

async fn prepare_and_flash(
    app: &AppHandle,
    path: PathBuf,
    device_path: &str,
    expected_size: u64,
    verify: bool,
    autoconfig: Option<AutoconfigConfig>,
    flash_state: Arc<FlashState>,
) -> Result<(), String> {
    let target = resolve_flash_target(app, device_path, expected_size).await?;

    let image_size = image_size(&path)?;
    if let Err(e) = check_capacity(image_size, target.size()) {
        log_error!("operations", "Refusing target {:?}: {}", device_path, e);
        return Err(e);
    }

    // The preset goes into a per-flash copy (removed on drop) so the cached image stays pristine.
    let working_copy = match autoconfig {
        Some(profile) => Some(
            // Catalog images always carry the script, so the marker probe only refuses custom images built without it.
            prepare_flash_copy(
                path.clone(),
                autoconfig_temp_dir(),
                profile,
                true,
                flash_state.clone(),
            )
            .await
            .map_err(PrepError::into_flash_error)?,
        ),
        None => None,
    };
    flash_state.ensure_not_cancelled()?;
    let flash_path = working_copy
        .as_ref()
        .map_or(path, |copy| copy.path().to_path_buf());

    let result = do_flash(&flash_path, &target, flash_state, verify).await;
    drop(working_copy);
    result
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

    let cache_enabled = match app.store(config::app::SETTINGS_STORE) {
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

#[cfg(test)]
mod tests {
    use super::allow_system_from;
    use serde_json::{json, Value};

    fn setting(value: Option<Value>) -> Result<Option<Value>, String> {
        Ok(value)
    }

    #[test]
    fn allow_system_only_on_explicit_true() {
        assert!(!allow_system_from::<String>(Err(
            "store unavailable".to_string()
        )));
        assert!(!allow_system_from(setting(None)));
        assert!(!allow_system_from(setting(Some(json!("true")))));
        assert!(!allow_system_from(setting(Some(json!(1)))));
        assert!(!allow_system_from(setting(Some(Value::Null))));
        assert!(!allow_system_from(setting(Some(json!(false)))));
        assert!(allow_system_from(setting(Some(json!(true)))));
    }
}

// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Tauri command handlers for QDL (Qualcomm EDL) device detection and flashing.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::State;

use crate::autoconfig::{prepare_flash_copy, PrepError};
use crate::flash::{reject_simulated, FlashSession, FlashState, SessionRefusal};
use crate::qdl;
use crate::qdl::outcome::{is_disconnect_panic, settle};
use crate::qdl::QdlDevice;
use crate::utils::{autoconfig_temp_dir, ensure_regular_nonempty, qdl_temp_dir};
use crate::{log_error, log_info, log_warn};

use super::state::AppState;

/// Tag a panicked flash task so the frontend can map it to an i18n key; a cancel is decided by `settle`.
fn tag_join_error(e: tokio::task::JoinError) -> String {
    let msg = e.to_string();
    if is_disconnect_panic(&msg) {
        qdl::TAG_QDL_DISCONNECTED.to_string()
    } else {
        format!("{} {}", qdl::TAG_QDL_ERROR, msg)
    }
}

fn settle_join_error(e: tokio::task::JoinError, flash_state: &FlashState) -> Result<(), String> {
    settle(
        Err(tag_join_error(e)),
        flash_state.is_cancelled.load(Ordering::SeqCst),
    )
}

// qdlrs is synchronous, so the flash runs off the async runtime.
async fn run_flash_task<C, F>(
    session: FlashSession,
    cleanup: C,
    flash_state: Arc<FlashState>,
    flash: F,
) -> Result<(), String>
where
    C: Send + 'static,
    F: FnOnce(Arc<FlashState>) -> Result<(), String> + Send + 'static,
{
    let join_state = flash_state.clone();
    tokio::task::spawn_blocking(move || {
        let _session = session;
        // Dropped before the session, panics included: the next flash may reuse what it holds.
        let _cleanup = cleanup;
        let result = flash(flash_state.clone());
        // Read while the session is held: a Retry clears the flag once the slot frees.
        settle(result, flash_state.is_cancelled.load(Ordering::SeqCst))
    })
    .await
    .unwrap_or_else(|e| settle_join_error(e, &join_state))
}

struct ExtractionCleanup(PathBuf);

impl Drop for ExtractionCleanup {
    fn drop(&mut self) {
        qdl::extract::cleanup_extraction(&self.0);
    }
}

/// One flash at a time, as for block devices; a cancel maps to the QDL cancel error.
async fn begin_qdl_session(flash_state: &FlashState) -> Result<FlashSession, String> {
    flash_state.begin_session().await.map_err(|refusal| {
        let message = match refusal {
            SessionRefusal::Cancelled(_) => qdl::QDL_CANCELLED_ERROR.to_string(),
            SessionRefusal::Busy(_) => refusal.message(),
        };
        log_error!("qdl_operations", "QDL flash not started: {}", message);
        message
    })
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
    let session = begin_qdl_session(&flash_state).await?;

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
    ensure_regular_nonempty(&tar_path)?;
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

    let extract_root = qdl::extract::extraction_dir(&extract_dir);
    // The next flash extracts into the same directory.
    let extraction = ExtractionCleanup(extract_dir);
    let result = run_flash_task(session, extraction, flash_state, move |state| {
        qdl::flash::qdl_flash(&flash_dir, &extract_root, &device_path, autoconfig, state)
    })
    .await;

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
    let session = begin_qdl_session(&flash_state).await?;

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
    ensure_regular_nonempty(&image_path)?;

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

    let result = run_flash_task(session, (), flash_state, move |state| {
        qdl::flash::qdl_flash_ufs(&write_path, &loader_path, &device_path, provision, state)
    })
    .await;

    drop(working_copy);

    match &result {
        Ok(()) => log_info!("qdl_operations", "QDL UFS flash completed successfully"),
        Err(e) => log_error!("qdl_operations", "QDL UFS flash failed: {}", e),
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn the_extraction_is_gone_before_the_next_flash_starts() {
        let dir = tempfile::tempdir().unwrap();
        let extract_dir = dir.path().join("qdl-extract");
        std::fs::create_dir_all(extract_dir.join("flash")).unwrap();
        let state = Arc::new(FlashState::new());
        let session = state.begin_session().await.unwrap();
        let next = {
            let state = state.clone();
            let extract_dir = extract_dir.clone();
            tokio::spawn(async move {
                let next = state.begin_session().await;
                (next.is_ok(), extract_dir.exists())
            })
        };

        let held = extract_dir.clone();
        tokio::task::spawn_blocking(move || {
            let _session = session;
            let _extraction = ExtractionCleanup(held);
            std::thread::sleep(Duration::from_millis(300));
        })
        .await
        .unwrap();

        assert_eq!(next.await.unwrap(), (true, false));
    }

    const RAW_WRITE_ERROR: &str = "Failed to write UFS image: [QDL_DISCONNECTED] Error sending data for partition system: mock";

    #[tokio::test(flavor = "multi_thread")]
    async fn the_flash_task_settles_on_the_flag_read_inside_the_session() {
        let state = Arc::new(FlashState::new());
        let session = state.begin_session().await.unwrap();
        // A Retry waiting for the slot clears the flag as soon as the session is released.
        let retry = {
            let state = state.clone();
            tokio::spawn(async move { state.begin_session().await.map(drop).is_ok() })
        };
        let result = run_flash_task(session, (), state.clone(), |state| {
            state.is_cancelled.store(true, Ordering::SeqCst);
            Err(RAW_WRITE_ERROR.to_string())
        })
        .await;
        assert_eq!(result, Err(qdl::QDL_CANCELLED_ERROR.to_string()));
        assert!(retry.await.unwrap());

        let state = Arc::new(FlashState::new());
        let session = state.begin_session().await.unwrap();
        let result = run_flash_task(session, (), state, |_| Err(RAW_WRITE_ERROR.to_string())).await;
        assert_eq!(result, Err(RAW_WRITE_ERROR.to_string()));

        let state = Arc::new(FlashState::new());
        let session = state.begin_session().await.unwrap();
        assert_eq!(run_flash_task(session, (), state, |_| Ok(())).await, Ok(()));
    }

    #[tokio::test]
    async fn a_panicking_flash_task_goes_through_the_same_settle() {
        let state = Arc::new(FlashState::new());
        let session = state.begin_session().await.unwrap();
        let result = run_flash_task(session, (), state.clone(), |_| {
            expect_zlp();
            Ok(())
        })
        .await;
        assert_eq!(result, Err(qdl::TAG_QDL_DISCONNECTED.to_string()));

        let session = state.begin_session().await.unwrap();
        let result = run_flash_task(session, (), state.clone(), |state| {
            state.is_cancelled.store(true, Ordering::SeqCst);
            unwrap_interrupted();
            Ok(())
        })
        .await;
        assert_eq!(result, Err(qdl::QDL_CANCELLED_ERROR.to_string()));
    }

    #[tokio::test]
    async fn the_flash_task_drops_the_extraction_before_the_session() {
        let dir = tempfile::tempdir().unwrap();
        let extract_dir = dir.path().join("qdl-extract");
        std::fs::create_dir_all(extract_dir.join("flash")).unwrap();
        let state = Arc::new(FlashState::new());
        let session = state.begin_session().await.unwrap();
        let next = {
            let state = state.clone();
            let extract_dir = extract_dir.clone();
            tokio::spawn(async move {
                let next = state.begin_session().await;
                (next.is_ok(), extract_dir.exists())
            })
        };

        let cleanup = ExtractionCleanup(extract_dir.clone());
        let result = run_flash_task(session, cleanup, state, |_| {
            std::thread::sleep(Duration::from_millis(300));
            Ok(())
        })
        .await;

        assert_eq!(result, Ok(()));
        assert_eq!(next.await.unwrap(), (true, false));
    }

    #[tokio::test]
    async fn only_panic_text_maps_the_receive_fragment_to_a_disconnect() {
        let state = FlashState::new();
        let receive = panicked(|| {
            usb_transfer(std::io::ErrorKind::TimedOut, "timed out").expect("Error receiving data");
        })
        .await;
        assert_eq!(
            settle_join_error(receive, &state),
            Err(qdl::TAG_QDL_DISCONNECTED.to_string())
        );
    }

    async fn panicked(task: impl FnOnce() + Send + 'static) -> tokio::task::JoinError {
        tokio::task::spawn_blocking(task).await.unwrap_err()
    }

    fn usb_transfer(kind: std::io::ErrorKind, text: &str) -> std::io::Result<usize> {
        Err(std::io::Error::new(kind, text))
    }

    fn unwrap_interrupted() {
        usb_transfer(std::io::ErrorKind::Interrupted, "transfer was cancelled").unwrap();
    }

    fn expect_zlp() {
        usb_transfer(std::io::ErrorKind::ConnectionAborted, "device disconnected")
            .expect("Error sending ZLP");
    }

    #[tokio::test]
    async fn a_panicked_flash_task_is_tagged_and_only_the_flag_means_cancel() {
        let state = FlashState::new();

        let interrupted = settle_join_error(panicked(unwrap_interrupted).await, &state);
        let err = interrupted.unwrap_err();
        assert!(err.starts_with(qdl::TAG_QDL_ERROR), "{err}");
        assert!(err.contains("Interrupted"), "{err}");

        assert_eq!(
            settle_join_error(panicked(expect_zlp).await, &state),
            Err(qdl::TAG_QDL_DISCONNECTED.to_string())
        );

        state
            .is_cancelled
            .store(true, std::sync::atomic::Ordering::SeqCst);
        for task in [unwrap_interrupted as fn(), expect_zlp] {
            assert_eq!(
                settle_join_error(panicked(task).await, &state),
                Err(qdl::QDL_CANCELLED_ERROR.to_string())
            );
        }
    }
}

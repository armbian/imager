//! Progress tracking module
//!
//! Handles download and flash progress reporting.

use serde::{Deserialize, Serialize};
use tauri::State;

use super::state::AppState;

/// Download progress information
#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub is_verifying_sha: bool,
    pub is_decompressing: bool,
    pub progress_percent: f64,
    pub error: Option<String>,
}

/// Flash progress information
#[derive(Debug, Serialize, Deserialize)]
pub struct FlashProgress {
    pub total_bytes: u64,
    pub written_bytes: u64,
    pub verified_bytes: u64,
    pub is_verifying: bool,
    pub progress_percent: f64,
    pub error: Option<String>,
}

/// Get current download progress
#[tauri::command]
pub async fn get_download_progress(state: State<'_, AppState>) -> Result<DownloadProgress, String> {
    let ds = &state.download_state;

    let total = ds.total_bytes.load(std::sync::atomic::Ordering::SeqCst);
    let downloaded = ds.downloaded_bytes.load(std::sync::atomic::Ordering::SeqCst);
    let is_verifying_sha = ds.is_verifying_sha.load(std::sync::atomic::Ordering::SeqCst);
    let is_decompressing = ds.is_decompressing.load(std::sync::atomic::Ordering::SeqCst);

    let progress = if total > 0 {
        (downloaded as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let error = ds.error.lock().await.clone();

    Ok(DownloadProgress {
        total_bytes: total,
        downloaded_bytes: downloaded,
        is_verifying_sha,
        is_decompressing,
        progress_percent: progress,
        error,
    })
}

/// Get current flash progress
#[tauri::command]
pub async fn get_flash_progress(state: State<'_, AppState>) -> Result<FlashProgress, String> {
    let fs = &state.flash_state;

    let total = fs.total_bytes.load(std::sync::atomic::Ordering::SeqCst);
    let written = fs.written_bytes.load(std::sync::atomic::Ordering::SeqCst);
    let verified = fs.verified_bytes.load(std::sync::atomic::Ordering::SeqCst);
    let is_verifying = fs.is_verifying.load(std::sync::atomic::Ordering::SeqCst);

    let progress = if is_verifying {
        if total > 0 {
            (verified as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    } else if total > 0 {
        (written as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let error = fs.error.lock().await.clone();

    Ok(FlashProgress {
        total_bytes: total,
        written_bytes: written,
        verified_bytes: verified,
        is_verifying,
        progress_percent: progress,
        error,
    })
}

/// Cancel current operation
#[tauri::command]
pub async fn cancel_operation(state: State<'_, AppState>) -> Result<(), String> {
    state
        .download_state
        .is_cancelled
        .store(true, std::sync::atomic::Ordering::SeqCst);
    state
        .flash_state
        .is_cancelled
        .store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

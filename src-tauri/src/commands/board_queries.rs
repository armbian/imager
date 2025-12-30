//! Board and image queries module
//!
//! Handles fetching and filtering board/image data.

use std::collections::HashSet;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use tauri::State;

use crate::devices::{get_block_devices as devices_get_block_devices, BlockDevice};
use crate::images::{
    extract_images, fetch_all_images, filter_images_for_board, get_unique_boards, BoardInfo,
    ImageInfo,
};
use crate::{log_debug, log_error, log_info};

use super::state::AppState;

/// Track previously seen device paths to detect changes
static PREV_DEVICE_PATHS: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

/// Get list of available boards
#[tauri::command]
pub async fn get_boards(state: State<'_, AppState>) -> Result<Vec<BoardInfo>, String> {
    log_info!("board_queries", "Fetching boards list");

    // Fetch images if not cached
    let mut json_guard = state.images_json.lock().await;
    if json_guard.is_none() {
        log_info!("board_queries", "Cache miss - fetching from API");
        let json = fetch_all_images().await.map_err(|e| {
            log_error!("board_queries", "Failed to fetch boards: {}", e);
            e
        })?;
        *json_guard = Some(json);
    }

    let json = json_guard.as_ref().unwrap();
    let images = extract_images(json);
    let boards = get_unique_boards(&images);
    log_info!("board_queries", "Found {} boards", boards.len());
    Ok(boards)
}

/// Get images available for a specific board
#[tauri::command]
pub async fn get_images_for_board(
    board_slug: String,
    preapp_filter: Option<String>,
    kernel_filter: Option<String>,
    variant_filter: Option<String>,
    stable_only: bool,
    state: State<'_, AppState>,
) -> Result<Vec<ImageInfo>, String> {
    log_info!(
        "board_queries",
        "Getting images for board: {} (stable_only: {})",
        board_slug,
        stable_only
    );
    log_debug!(
        "board_queries",
        "Filters - preapp: {:?}, kernel: {:?}, variant: {:?}",
        preapp_filter,
        kernel_filter,
        variant_filter
    );

    let json_guard = state.images_json.lock().await;
    let json = json_guard.as_ref().ok_or_else(|| {
        log_error!(
            "board_queries",
            "Images not loaded when requesting board: {}",
            board_slug
        );
        "Images not loaded. Call get_boards first.".to_string()
    })?;

    let images = extract_images(json);
    log_debug!("board_queries", "Total images available: {}", images.len());
    let filtered = filter_images_for_board(
        &images,
        &board_slug,
        preapp_filter.as_deref(),
        kernel_filter.as_deref(),
        variant_filter.as_deref(),
        stable_only,
    );
    log_debug!(
        "board_queries",
        "Filtered down to {} images for board {}",
        filtered.len(),
        board_slug
    );
    log_info!(
        "board_queries",
        "Found {} images for board {}",
        filtered.len(),
        board_slug
    );
    Ok(filtered)
}

/// Get available block devices
#[tauri::command]
pub async fn get_block_devices() -> Result<Vec<BlockDevice>, String> {
    let devices = devices_get_block_devices().map_err(|e| {
        log_error!("board_queries", "Failed to get block devices: {}", e);
        e
    })?;

    // Only log when device list changes
    let current_paths: HashSet<String> = devices.iter().map(|d| d.path.clone()).collect();
    let mut prev_paths = PREV_DEVICE_PATHS.lock().unwrap();

    if *prev_paths != current_paths {
        // Find added and removed devices
        let added: Vec<_> = current_paths.difference(&prev_paths).collect();
        let removed: Vec<_> = prev_paths.difference(&current_paths).collect();

        if !added.is_empty() {
            log_info!("board_queries", "Device(s) added: {:?}", added);
        }
        if !removed.is_empty() {
            log_info!("board_queries", "Device(s) removed: {:?}", removed);
        }
        if added.is_empty() && removed.is_empty() {
            // First scan
            log_info!("board_queries", "Found {} block devices", devices.len());
        }

        *prev_paths = current_paths;
    }

    Ok(devices)
}

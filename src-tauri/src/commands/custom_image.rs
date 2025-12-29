//! Custom image handling module
//!
//! Handles selection and processing of user-provided custom images.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

use crate::config;
use crate::decompress::{decompress_local_file, needs_decompression};
use crate::images::{extract_images, fetch_all_images, get_unique_boards, BoardInfo};
use crate::utils::{get_cache_dir, normalize_slug};
use crate::{log_error, log_info};

use super::state::AppState;

/// Custom image info returned when user selects a local file
#[derive(Debug, Serialize, Deserialize)]
pub struct CustomImageInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
}

/// Check if a custom image needs decompression
#[tauri::command]
pub async fn check_needs_decompression(image_path: String) -> Result<bool, String> {
    let path = PathBuf::from(&image_path);
    let needs = needs_decompression(&path);
    log_info!(
        "custom_image",
        "Check decompression for {}: {}",
        image_path,
        needs
    );
    Ok(needs)
}

/// Decompress a custom image file
/// Returns the path to the decompressed file
#[tauri::command]
pub async fn decompress_custom_image(
    image_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    log_info!("custom_image", "Starting decompression: {}", image_path);
    let path = PathBuf::from(&image_path);
    let download_state = state.download_state.clone();

    // Reset state for progress tracking
    download_state.reset();

    // Run decompression in a blocking task
    let result = tokio::task::spawn_blocking(move || decompress_local_file(&path, &download_state))
        .await
        .map_err(|e| {
            log_error!("custom_image", "Decompression task failed: {}", e);
            format!("Task failed: {}", e)
        })?;

    match &result {
        Ok(path) => {
            log_info!(
                "custom_image",
                "Decompression completed: {}",
                path.display()
            );
        }
        Err(e) => {
            log_error!("custom_image", "Decompression failed: {}", e);
        }
    }

    result.map(|p| p.to_string_lossy().to_string())
}

/// Select a custom image file using native file picker
#[tauri::command]
pub async fn select_custom_image(window: tauri::Window) -> Result<Option<CustomImageInfo>, String> {
    use tauri_plugin_dialog::DialogExt;

    log_info!("custom_image", "Opening file picker dialog");

    let file_path = window
        .dialog()
        .file()
        .add_filter(
            "Disk Images",
            &["img", "iso", "raw", "xz", "gz", "bz2", "zst"],
        )
        .add_filter("All Files", &["*"])
        .set_title("Select Disk Image")
        .blocking_pick_file();

    match file_path {
        Some(file_path) => {
            let path_buf = file_path.as_path().ok_or_else(|| {
                log_error!("custom_image", "Invalid path: not a valid file path");
                "Invalid path: not a valid file path".to_string()
            })?;
            let metadata = std::fs::metadata(path_buf).map_err(|e| {
                log_error!(
                    "custom_image",
                    "Failed to read file info for {:?}: {}",
                    path_buf,
                    e
                );
                format!("Failed to read file info: {}", e)
            })?;

            let name = path_buf
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            log_info!(
                "custom_image",
                "Selected custom image: {} ({} bytes)",
                name,
                metadata.len()
            );

            Ok(Some(CustomImageInfo {
                path: path_buf.to_string_lossy().to_string(),
                name,
                size: metadata.len(),
            }))
        }
        None => {
            log_info!("custom_image", "File picker cancelled by user");
            Ok(None)
        }
    }
}

/// Delete a decompressed custom image file
#[tauri::command]
pub async fn delete_decompressed_custom_image(image_path: String) -> Result<(), String> {
    log_info!(
        "custom_image",
        "Deleting decompressed custom image: {}",
        image_path
    );
    let path = PathBuf::from(&image_path);

    // Safety check: only delete files in our custom-decompress directory
    let custom_decompress_dir = get_cache_dir(config::app::NAME).join("custom-decompress");

    if !path.starts_with(&custom_decompress_dir) {
        log_error!(
            "custom_image",
            "Attempted to delete file outside custom-decompress cache: {}",
            image_path
        );
        return Err("Cannot delete files outside custom-decompress directory".to_string());
    }

    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| {
            log_error!(
                "custom_image",
                "Failed to delete decompressed image {}: {}",
                image_path,
                e
            );
            format!("Failed to delete decompressed image: {}", e)
        })?;
        log_info!("custom_image", "Deleted decompressed image: {}", image_path);
    }

    // Try to remove empty parent directory (ignore errors)
    let _ = std::fs::remove_dir(&custom_decompress_dir);

    Ok(())
}

/// Detect board information from custom image filename
/// Parses Armbian naming convention: Armbian_VERSION_BOARD_DISTRO_VENDOR_KERNEL_FLAVOR.img.xz
#[tauri::command]
pub async fn detect_board_from_filename(
    filename: String,
    state: State<'_, AppState>,
) -> Result<Option<BoardInfo>, String> {
    log_info!(
        "custom_image",
        "=== Starting board detection from filename: {} ===",
        filename
    );

    // 1. Extract filename from path (remove directory)
    let path = PathBuf::from(&filename);
    let filename_only = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?;

    // 2. Remove extension(s) - handle .img.xz, .img.gz, .img.zst, .img.bz2, .img
    let stem = filename_only
        .strip_suffix(".xz")
        .or_else(|| filename_only.strip_suffix(".gz"))
        .or_else(|| filename_only.strip_suffix(".zst"))
        .or_else(|| filename_only.strip_suffix(".bz2"))
        .or_else(|| filename_only.strip_suffix(".img"))
        .unwrap_or(filename_only);

    // 3. Parse Armbian naming pattern: Armbian_VERSION_BOARD_DISTRO_VENDOR_KERNEL_FLAVOR
    let parts: Vec<&str> = stem.split('_').collect();

    // 4. Validate Armbian format (at least 4 parts, starts with "Armbian")
    if parts.len() < 4 || !parts[0].eq_ignore_ascii_case("Armbian") {
        log_info!(
            "custom_image",
            "Not an Armbian image or invalid format: {}",
            filename_only
        );
        return Ok(None);
    }

    // 5. Extract board name (index 2)
    let board_name = parts[2];
    log_info!(
        "custom_image",
        "Extracted board name from filename: {}",
        board_name
    );

    // 6. Normalize board name to slug format
    let normalized_slug = normalize_slug(board_name);
    log_info!("custom_image", "Normalized board slug: {}", normalized_slug);

    // 7. Ensure board data is loaded (auto-load if not cached)
    // Use compare-and-swap pattern to prevent race conditions
    log_info!("custom_image", "Checking if board data is cached...");
    {
        let needs_loading = {
            let json_guard = state.images_json.lock().await;
            json_guard.is_none()
        };

        if needs_loading {
            log_info!(
                "custom_image",
                "Board data not cached, fetching from API..."
            );
            let json = fetch_all_images().await.map_err(|e| {
                log_error!("custom_image", "Failed to fetch board data: {}", e);
                format!("Failed to fetch board data: {}", e)
            })?;

            // Cache the fetched data
            let mut json_guard = state.images_json.lock().await;
            // Double-check: another thread might have loaded it while we were fetching
            if json_guard.is_none() {
                *json_guard = Some(json);
                log_info!("custom_image", "Board data cached successfully");
            } else {
                log_info!(
                    "custom_image",
                    "Board data was already cached by another thread"
                );
            }
        }
    }

    // 8. Get cached boards data (now guaranteed to be loaded)
    // Extract boards in a scoped block to release lock early
    let matching_board = {
        log_info!("custom_image", "Accessing cached board data...");
        let json_guard = state.images_json.lock().await;
        let json = json_guard.as_ref().ok_or("Images not loaded")?;

        log_info!("custom_image", "Loaded images JSON, extracting boards...");
        let images = extract_images(json);
        log_info!("custom_image", "Extracted {} images", images.len());
        let boards = get_unique_boards(&images);
        log_info!(
            "custom_image",
            "Found {} unique boards in database",
            boards.len()
        );
        // Lock released here

        // 9. Find matching board by slug
        boards
            .iter()
            .find(|board| board.slug == normalized_slug)
            .cloned()
    }; // matching_board is now owned, lock is released

    if let Some(ref board) = matching_board {
        log_info!(
            "custom_image",
            "Detected board: {} (slug: {})",
            board.name,
            board.slug
        );
    } else {
        log_info!(
            "custom_image",
            "Board not found in database: {}",
            normalized_slug
        );
    }

    log_info!("custom_image", "Board detection completed successfully");
    Ok(matching_board)
}

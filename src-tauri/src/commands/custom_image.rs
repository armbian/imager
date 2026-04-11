//! Custom image handling module
//!
//! Handles selection and processing of user-provided custom images.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

use crate::config;
use crate::decompress::{decompress_local_file, needs_decompression};
use crate::images::{extract_images, fetch_all_images, get_unique_boards, BoardInfo};
use crate::qdl::extract::open_tar_reader;
use crate::utils::{get_cache_dir, normalize_slug, parse_armbian_filename};
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
            &["img", "iso", "raw", "xz", "gz", "bz2", "zst", "tar"],
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

/// Check if a custom image file is a QDL (Qualcomm EDL) archive
///
/// Inspects the archive contents for a valid QDL structure:
/// - rawprogram0.xml (partition programming instructions)
/// - prog_firehose_ddr.elf (Sahara firehose programmer)
///
/// Supports plain .tar and compressed archives (.tar.xz, .tar.gz, .tar.bz2, .tar.zst).
/// Files that are not TAR archives (e.g. .img) return false without error.
#[tauri::command]
pub async fn check_is_qdl_image(image_path: String) -> Result<bool, String> {
    let path = PathBuf::from(&image_path);
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    // For plain .tar files, full content inspection is fast
    if filename.ends_with(".tar") {
        log_info!(
            "custom_image",
            "Checking plain TAR for QDL structure: {}",
            image_path
        );
        let reader = match open_tar_reader(&path) {
            Ok(r) => r,
            Err(_) => return Ok(false),
        };
        let has_qdl = check_tar_for_qdl(reader);
        log_info!("custom_image", "Archive has QDL structure: {}", has_qdl);
        return Ok(has_qdl);
    }

    // For compressed TAR archives (.tar.xz, .tar.gz, etc.), full decompression
    // is too slow. Instead, verify it's actually a valid TAR by decompressing
    // only the first entry header (~512 bytes). In the Armbian ecosystem, a
    // compressed .tar is always a QDL flash archive (block-device images use
    // .img.xz). The actual QDL file structure is validated after full extraction
    // in the flash pipeline (extract.rs::validate_required_files).
    if filename.contains(".tar.") {
        log_info!(
            "custom_image",
            "Checking compressed TAR validity: {}",
            image_path
        );
        let reader = match open_tar_reader(&path) {
            Ok(r) => r,
            Err(_) => return Ok(false),
        };
        let has_qdl = quick_check_qdl_structure(reader);
        log_info!("custom_image", "QDL structure detected: {}", has_qdl);
        return Ok(has_qdl);
    }

    // Not a TAR archive
    Ok(false)
}

/// Scan a TAR archive reader for QDL-required files
fn check_tar_for_qdl<R: std::io::Read>(reader: R) -> bool {
    let mut archive = tar::Archive::new(reader);
    let mut has_rawprogram = false;
    let mut has_firehose = false;

    let entries = match archive.entries() {
        Ok(e) => e,
        Err(_) => return false,
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let filename = entry
            .path()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));

        if let Some(name) = filename {
            if crate::qdl::extract::REQUIRED_FILES.contains(&name.as_str()) {
                has_rawprogram = true;
            }
            if name == crate::qdl::extract::FIREHOSE_ELF {
                has_firehose = true;
            }
            if has_rawprogram && has_firehose {
                return true;
            }
        }
    }

    false
}

/// Quick check for QDL structure in a compressed TAR by reading only the first
/// few entry headers. Looks for a "flash/" directory or paths containing "/flash/"
/// which indicate QDL archive layout. Only decompresses headers of the first
/// entries (before the large rootfs blob), so it's fast (~1 second max).
///
/// The full QDL file validation (rawprogram0.xml, prog_firehose_ddr.elf)
/// happens after extraction in the flash pipeline.
fn quick_check_qdl_structure<R: std::io::Read>(reader: R) -> bool {
    let mut archive = tar::Archive::new(reader);
    let entries = match archive.entries() {
        Ok(e) => e,
        Err(_) => return false,
    };

    // Check the first entries — in QDL archives, directory entries and small
    // files come before the large rootfs. We stop after finding a "flash/"
    // directory or after hitting a large file entry (to avoid decompressing it).
    const MAX_SMALL_ENTRIES: usize = 10;
    const LARGE_FILE_THRESHOLD: u64 = 100 * 1024 * 1024; // 100MB

    for (idx, entry) in entries.enumerate() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => return false,
        };

        // Check entry path for QDL indicators (header only, no data decompression)
        if let Ok(path) = entry.path() {
            let path_str = path.to_string_lossy();
            // Direct match: flash/ directory or files inside it
            if path_str.ends_with("flash/") || path_str.contains("/flash/") {
                return true;
            }
            // Armbian QDL archives contain partition images with these names
            if path_str.contains("disk-sdcard.img") {
                return true;
            }
        }

        // Stop before the iterator advances past large file data (which would
        // require decompressing it). We've already read this entry's header.
        if entry.size() > LARGE_FILE_THRESHOLD || idx >= MAX_SMALL_ENTRIES {
            break;
        }
    }

    false
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

    // 2. Parse Armbian filename using shared utility
    let parsed = match parse_armbian_filename(filename_only) {
        Some(info) => info,
        None => {
            log_info!(
                "custom_image",
                "Not an Armbian image or invalid format: {}",
                filename_only
            );
            return Ok(None);
        }
    };

    log_info!(
        "custom_image",
        "Extracted board slug from filename: {}",
        parsed.board_slug
    );

    // 3. Normalize board slug for matching against API data
    let normalized_slug = normalize_slug(&parsed.board_slug);
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

//! Selection and processing of user-provided custom images.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

use crate::decompress::{decompress_local_file, needs_decompression};
use crate::images::{fetch_boards, map_board, BoardInfo};
use crate::qdl::extract::open_tar_reader;
use crate::utils::{custom_decompress_dir, normalize_slug, parse_armbian_filename};
use crate::{log_debug, log_error, log_info};

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
    log_debug!(
        "custom_image",
        "Check decompression for {}: {}",
        image_path,
        needs
    );
    Ok(needs)
}

/// Decompress a custom image file, returning the decompressed path.
#[tauri::command]
pub async fn decompress_custom_image(
    image_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    log_info!("custom_image", "Starting decompression: {}", image_path);
    let path = PathBuf::from(&image_path);
    let download_state = state.download_state.clone();

    download_state.reset();

    // Decompression is CPU-bound, so run it off the async runtime.
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

    // Refuse to delete anything outside the custom-decompress directory.
    let custom_dir = custom_decompress_dir();

    if !path.starts_with(&custom_dir) {
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

    // Best-effort removal of the now-empty directory.
    let _ = std::fs::remove_dir(&custom_dir);

    Ok(())
}

/// Check whether a custom image is a QDL archive (TAR containing rawprogram0.xml
/// and prog_firehose_ddr.elf). Non-TAR files like .img return false, not an error.
#[tauri::command]
pub async fn check_is_qdl_image(image_path: String) -> Result<bool, String> {
    let path = PathBuf::from(&image_path);
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Plain .tar: cheap to inspect every entry.
    if filename.ends_with(".tar") {
        log_debug!(
            "custom_image",
            "Checking plain TAR for QDL structure: {}",
            image_path
        );
        let reader = match open_tar_reader(&path) {
            Ok(r) => r,
            Err(_) => return Ok(false),
        };
        let has_qdl = check_tar_for_qdl(reader);
        log_debug!("custom_image", "Archive has QDL structure: {}", has_qdl);
        return Ok(has_qdl);
    }

    // Compressed TAR: QDL requires BOTH rawprogram0.xml and prog_firehose_ddr.elf, so scan entries
    // rather than sniff a flash/ dir or disk-sdcard.img name (a generic .tar.gz rootfs is NOT QDL).
    if filename.contains(".tar.") {
        log_debug!(
            "custom_image",
            "Checking compressed TAR for QDL structure: {}",
            image_path
        );
        let reader = match open_tar_reader(&path) {
            Ok(r) => r,
            Err(_) => return Ok(false),
        };
        let has_qdl = check_tar_for_qdl(reader);
        log_debug!("custom_image", "QDL structure detected: {}", has_qdl);
        return Ok(has_qdl);
    }

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

/// Detect the board for a custom image by parsing its Armbian filename and matching the API list.
#[tauri::command]
pub async fn detect_board_from_filename(
    filename: String,
    state: State<'_, AppState>,
) -> Result<Option<BoardInfo>, String> {
    log_debug!(
        "custom_image",
        "Starting board detection from filename: {}",
        filename
    );

    let path = PathBuf::from(&filename);
    let filename_only = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?;

    let parsed = match parse_armbian_filename(filename_only) {
        Some(info) => info,
        None => {
            log_debug!(
                "custom_image",
                "Not an Armbian image or invalid format: {}",
                filename_only
            );
            return Ok(None);
        }
    };

    log_debug!(
        "custom_image",
        "Extracted board slug from filename: {}",
        parsed.board_slug
    );

    let normalized_slug = normalize_slug(&parsed.board_slug);
    log_debug!("custom_image", "Normalized board slug: {}", normalized_slug);

    // Auto-load board data on first use; double-checked to avoid a redundant fetch.
    {
        let needs_loading = {
            let boards_guard = state.boards.lock().await;
            boards_guard.is_none()
        };

        if needs_loading {
            log_debug!("custom_image", "Board data not cached, fetching from API");
            let api_boards = fetch_boards().await.map_err(|e| {
                log_error!("custom_image", "Failed to fetch board data: {}", e);
                format!("Failed to fetch board data: {}", e)
            })?;

            let mut boards_guard = state.boards.lock().await;
            if boards_guard.is_none() {
                *boards_guard = Some(api_boards);
            }
        }
    }

    let matching_board = {
        let boards_guard = state.boards.lock().await;
        let api_boards = boards_guard.as_ref().ok_or("Boards not loaded")?;

        let boards: Vec<BoardInfo> = api_boards.iter().map(map_board).collect();
        log_debug!("custom_image", "Found {} boards in database", boards.len());

        boards
            .into_iter()
            .find(|board| board.slug == normalized_slug)
    };

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

    Ok(matching_board)
}

#[cfg(test)]
mod tests {
    use super::check_tar_for_qdl;

    /// Build an in-memory uncompressed tar from `(path, contents)` entries.
    fn build_tar(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (path, contents) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, *contents).unwrap();
        }
        builder.into_inner().unwrap()
    }

    #[test]
    fn flash_dir_only_is_not_qdl() {
        // A tar carrying a flash/ dir and a disk-sdcard.img name but neither of
        // the two required QDL files must NOT be detected as QDL.
        let tar = build_tar(&[
            ("arduino-images/flash/some-other.xml", b"<data/>"),
            ("arduino-images/disk-sdcard.img.root", b"rootfs-bytes"),
        ]);
        assert!(!check_tar_for_qdl(&tar[..]));
    }

    #[test]
    fn both_required_files_present_is_qdl() {
        // rawprogram0.xml + prog_firehose_ddr.elf together => QDL.
        let tar = build_tar(&[
            ("arduino-images/flash/rawprogram0.xml", b"<data/>"),
            ("arduino-images/flash/prog_firehose_ddr.elf", b"\x7fELF"),
        ]);
        assert!(check_tar_for_qdl(&tar[..]));
    }

    #[test]
    fn only_rawprogram_is_not_qdl() {
        // rawprogram0.xml alone (no firehose elf) must NOT be QDL.
        let tar = build_tar(&[("flash/rawprogram0.xml", b"<data/>")]);
        assert!(!check_tar_for_qdl(&tar[..]));
    }
}

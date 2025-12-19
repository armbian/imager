//! Image filtering and extraction
//!
//! Functions for parsing and filtering image data.

use std::collections::HashMap;

use crate::config;
use crate::utils::normalize_slug;

use super::models::{ArmbianImage, BoardInfo, ImageInfo};

/// Check if file extension is a valid image file
fn is_valid_image_extension(ext: &str) -> bool {
    let ext_lower = ext.to_lowercase();
    ext_lower.starts_with("img")
        && !ext_lower.contains("asc")
        && !ext_lower.contains("torrent")
        && !ext_lower.contains("sha")
}

/// Extract all image objects from the nested JSON structure
pub fn extract_images(json: &serde_json::Value) -> Vec<ArmbianImage> {
    let mut images = Vec::new();
    extract_images_recursive(json, &mut images);
    images
}

fn extract_images_recursive(value: &serde_json::Value, images: &mut Vec<ArmbianImage>) {
    match value {
        serde_json::Value::Object(map) => {
            if map.contains_key("board_slug") {
                if let Ok(img) = serde_json::from_value::<ArmbianImage>(value.clone()) {
                    if let Some(ref ext) = img.file_extension {
                        if is_valid_image_extension(ext) {
                            let kernel = img.kernel_branch.as_deref().unwrap_or("");
                            if kernel != "cloud" {
                                images.push(img);
                            }
                        }
                    }
                }
            }
            for (_, v) in map {
                extract_images_recursive(v, images);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                extract_images_recursive(v, images);
            }
        }
        _ => {}
    }
}

/// Board data accumulated from images
struct BoardData {
    original_slug: String,
    board_name: Option<String>,
    count: usize,
    has_promoted: bool,
}

/// Get unique board list from images
pub fn get_unique_boards(images: &[ArmbianImage]) -> Vec<BoardInfo> {
    let mut board_map: HashMap<String, BoardData> = HashMap::new();

    for img in images {
        if let Some(ref slug) = img.board_slug {
            let normalized = normalize_slug(slug);
            let entry = board_map.entry(normalized.clone()).or_insert(BoardData {
                original_slug: slug.clone(),
                board_name: img.board_name.clone(),
                count: 0,
                has_promoted: false,
            });
            entry.count += 1;
            if img.promoted.as_deref() == Some("true") {
                entry.has_promoted = true;
            }
        }
    }

    let mut boards: Vec<BoardInfo> = board_map
        .into_iter()
        .map(|(slug, data)| {
            // Use board_name from API, fallback to slug if missing
            let name = data.board_name.unwrap_or(data.original_slug);

            BoardInfo {
                slug,
                name,
                image_count: data.count,
                has_promoted: data.has_promoted,
            }
        })
        .collect();

    boards.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    boards
}

/// Filter images for a specific board
pub fn filter_images_for_board(
    images: &[ArmbianImage],
    board_slug: &str,
    preapp_filter: Option<&str>,
    kernel_filter: Option<&str>,
    variant_filter: Option<&str>,
    stable_only: bool,
) -> Vec<ImageInfo> {
    let normalized_board = normalize_slug(board_slug);

    let mut filtered: Vec<ImageInfo> = images
        .iter()
        .filter(|img| {
            let img_slug = img.board_slug.as_deref().unwrap_or("");
            if normalize_slug(img_slug) != normalized_board {
                return false;
            }

            if let Some(filter) = preapp_filter {
                let preapp = img.preinstalled_application.as_deref().unwrap_or("");
                if filter == config::images::EMPTY_FILTER {
                    if !preapp.is_empty() {
                        return false;
                    }
                } else if preapp != filter {
                    return false;
                }
            }

            if stable_only {
                let repo = img.download_repository.as_deref().unwrap_or("");
                if repo != config::images::STABLE_REPO {
                    return false;
                }
            }

            if let Some(filter) = kernel_filter {
                let kernel = img.kernel_branch.as_deref().unwrap_or("");
                if kernel != filter {
                    return false;
                }
            }

            if let Some(filter) = variant_filter {
                let variant = img.image_variant.as_deref().unwrap_or("");
                if variant != filter {
                    return false;
                }
            }

            true
        })
        .map(|img| ImageInfo {
            armbian_version: img.armbian_version.clone().unwrap_or_default(),
            distro_release: img.distro_release.clone().unwrap_or_default(),
            kernel_branch: img.kernel_branch.clone().unwrap_or_default(),
            image_variant: img.image_variant.clone().unwrap_or_default(),
            preinstalled_application: img.preinstalled_application.clone().unwrap_or_default(),
            promoted: img.promoted.as_deref() == Some("true"),
            file_url: img.file_url.clone().unwrap_or_default(),
            file_url_sha: img.file_url_sha.clone(),
            file_size: img.file_size.as_ref().and_then(|s| s.parse().ok()).unwrap_or(0),
            download_repository: img.download_repository.clone().unwrap_or_default(),
        })
        .collect();

    filtered.sort_by(|a, b| match (a.promoted, b.promoted) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => b.armbian_version.cmp(&a.armbian_version),
    });

    filtered
}

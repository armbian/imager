//! Image filtering and extraction
//!
//! Functions for parsing and filtering image data.

use std::collections::HashMap;

use crate::config;
use crate::log_info;
use crate::utils::normalize_slug;

use super::models::{ArmbianImage, BoardInfo, ImageInfo};

/// Capitalize vendor ID for display (e.g., "rockchip" -> "Rockchip", "intel-amd" -> "Intel-Amd")
fn capitalize_vendor(vendor: &str) -> String {
    if vendor == "other" {
        return "Other".to_string();
    }
    vendor
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join("-")
}

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
    vendor: Option<String>,
    vendor_name: Option<String>,
    vendor_logo: Option<String>,
    count: usize,
    /// board_support: "conf" without platinum
    has_standard_support: bool,
    /// board_support: "csc"
    has_community_support: bool,
    /// board_support: "eos" (end of support)
    has_eos_support: bool,
    /// board_support: "tvb" (TV Box - experimental)
    has_tvb_support: bool,
    /// board_support: "wip" (Work In Progress)
    has_wip_support: bool,
    /// board_support: "conf" with platinum: "true" and valid date
    platinum_support_until: Option<String>,
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
                vendor: img.board_vendor.clone(),
                vendor_name: img.company_name.clone(),
                vendor_logo: img.company_logo.clone(),
                count: 0,
                has_standard_support: false,
                has_community_support: false,
                has_eos_support: false,
                has_tvb_support: false,
                has_wip_support: false,
                platinum_support_until: None,
            });
            entry.count += 1;

            // Use board_support field to determine support level
            match img.board_support.as_deref() {
                Some("conf") => {
                    // conf can be either Platinum (if platinum=true) or Standard
                    if img.platinum_support.as_deref() == Some("true") {
                        if let Some(ref until) = img.platinum_support_until {
                            entry.platinum_support_until = Some(until.clone());
                        }
                    } else {
                        entry.has_standard_support = true;
                    }
                }
                Some("csc") => entry.has_community_support = true,
                Some("eos") => entry.has_eos_support = true,
                Some("tvb") => entry.has_tvb_support = true,
                Some("wip") => entry.has_wip_support = true,
                _ => {}
            }
        }
    }

    let today = chrono::Utc::now().date_naive();

    let mut boards: Vec<BoardInfo> = board_map
        .into_iter()
        .map(|(slug, data)| {
            let name = data.board_name.unwrap_or(data.original_slug);

            let has_platinum_support = data.platinum_support_until
                .as_ref()
                .and_then(|until| chrono::NaiveDate::parse_from_str(until, "%Y-%m-%d").ok())
                .map(|exp_date| exp_date >= today)
                .unwrap_or(false);

            let has_logo = data.vendor_logo.as_ref().map(|l| !l.is_empty()).unwrap_or(false);
            let (vendor_id, vendor_display, vendor_logo) = if has_logo {
                let id = data.vendor.unwrap_or_else(|| "other".to_string());
                let display = data.vendor_name
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| capitalize_vendor(&id));
                (id, display, data.vendor_logo)
            } else {
                ("other".to_string(), "Other".to_string(), None)
            };

            // Community is shown only if no standard or platinum support
            let has_community_support = data.has_community_support
                && !data.has_standard_support
                && !has_platinum_support;

            // EOS is shown only if no other support level
            let has_eos_support = data.has_eos_support
                && !data.has_standard_support
                && !has_platinum_support
                && !has_community_support;

            // TVB is shown only if no other support level
            let has_tvb_support = data.has_tvb_support
                && !data.has_standard_support
                && !has_platinum_support
                && !has_community_support
                && !has_eos_support;

            // WIP is shown only if no other support level
            let has_wip_support = data.has_wip_support
                && !data.has_standard_support
                && !has_platinum_support
                && !has_community_support
                && !has_eos_support
                && !has_tvb_support;

            BoardInfo {
                slug,
                name,
                vendor: vendor_id,
                vendor_name: vendor_display,
                vendor_logo,
                image_count: data.count,
                has_standard_support: data.has_standard_support,
                has_community_support,
                has_platinum_support,
                has_eos_support,
                has_tvb_support,
                has_wip_support,
            }
        })
        .collect();

    boards.sort_by(|a, b| {
        // Platinum first, then Standard, then others
        match (a.has_platinum_support, b.has_platinum_support) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => match (a.has_standard_support, b.has_standard_support) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            },
        }
    });

    // Log board statistics by support level
    let platinum_count = boards.iter().filter(|b| b.has_platinum_support).count();
    let standard_count = boards.iter().filter(|b| b.has_standard_support).count();
    let community_count = boards.iter().filter(|b| b.has_community_support).count();
    let eos_count = boards.iter().filter(|b| b.has_eos_support).count();
    let tvb_count = boards.iter().filter(|b| b.has_tvb_support).count();
    let wip_count = boards.iter().filter(|b| b.has_wip_support).count();

    log_info!(
        "images",
        "Loaded {} boards: {} Platinum, {} Standard, {} Community, {} EOS, {} TVB, {} WIP",
        boards.len(),
        platinum_count,
        standard_count,
        community_count,
        eos_count,
        tvb_count,
        wip_count
    );
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

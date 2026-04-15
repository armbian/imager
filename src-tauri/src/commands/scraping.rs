//! Asset caching module
//!
//! Serves board images and vendor logos from the local picture cache
//! as base64 data URIs, downloading from the Armbian API on first access.

use crate::config;
use crate::picture_cache;

/// Get a board image from local cache as a data URI, downloading if needed
///
/// Returns a `data:image/png;base64,...` string ready for `<img src>`,
/// or `None` if the image is unavailable (offline and not cached).
///
/// # Arguments
/// * `board_slug` - Board identifier used to construct the image URL
#[tauri::command]
pub async fn get_cached_board_image(board_slug: String) -> Result<Option<String>, String> {
    let url = format!(
        "{}{}/{}.png",
        config::urls::BOARD_IMAGES_BASE,
        config::urls::BOARD_IMAGE_SIZE,
        board_slug
    );

    let path = picture_cache::get_asset("boards", &board_slug, &url).await;
    match path {
        Some(p) => Ok(picture_cache::read_as_data_uri(&p).await),
        None => Ok(None),
    }
}

/// Get a vendor logo from local cache as a data URI, downloading if needed
///
/// Returns a `data:image/png;base64,...` string ready for `<img src>`,
/// or `None` if the logo is unavailable (offline and not cached).
///
/// # Arguments
/// * `vendor_slug` - Vendor identifier used to construct the logo URL
#[tauri::command]
pub async fn get_cached_vendor_logo(vendor_slug: String) -> Result<Option<String>, String> {
    let url = format!("{}{}.png", config::urls::VENDOR_IMAGES_BASE, vendor_slug);

    let path = picture_cache::get_asset("vendors", &vendor_slug, &url).await;
    match path {
        Some(p) => Ok(picture_cache::read_as_data_uri(&p).await),
        None => Ok(None),
    }
}

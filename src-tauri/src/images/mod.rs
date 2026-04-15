//! Image management module
//!
//! Handles fetching board, image, and vendor data from the Armbian REST API.
//! Caches API responses on disk for offline use.

mod filters;
mod models;

// Re-export types and mapper functions
pub use filters::{map_board, map_images};
pub use models::{ApiBoardSummary, ApiImage, ApiVendor, BoardInfo, ImageInfo};

use models::ApiResponse;

use crate::config;
use crate::utils::get_cache_dir;
use crate::{log_debug, log_error, log_warn};

use once_cell::sync::Lazy;
use reqwest::header::{HeaderMap, HeaderValue};
use std::path::PathBuf;

/// Shared HTTP client for JSON API endpoints (short timeout, X-Armbian-Client header)
///
/// Uses SHORT_TIMEOUT_SECS (10s) since metadata responses are small. For large
/// downloads (images) use a dedicated client with longer timeouts.
static API_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    let mut headers = HeaderMap::new();
    headers.insert(
        config::http::CLIENT_HEADER_NAME,
        HeaderValue::from_static(config::http::CLIENT_HEADER_VALUE),
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .user_agent(config::app::USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(
            config::http::CONNECT_TIMEOUT_SECS,
        ))
        .timeout(std::time::Duration::from_secs(
            config::http::SHORT_TIMEOUT_SECS,
        ))
        .build()
        .expect("Failed to create API HTTP client")
});

/// Safety cap on pagination to avoid runaway loops if the API returns
/// inconsistent `meta.total` values.
const MAX_PAGES: u32 = 50;

/// Get the path for a named cache file inside the assets directory
fn get_cache_path(name: &str) -> PathBuf {
    get_cache_dir(config::app::NAME)
        .join("assets")
        .join(format!("{}.json", name))
}

/// Save data to a cache file atomically (temp file + rename).
///
/// Uses `tokio::task::spawn_blocking` for consistency with the async runtime,
/// and a unique tmp suffix to avoid collisions between concurrent writers.
fn save_cache(name: &str, data: &str) {
    let path = get_cache_path(name);
    let data = data.to_string();
    tokio::task::spawn_blocking(move || {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Unique tmp filename per writer (nanosecond timestamp + pid) to avoid
        // races if two tasks save the same cache file concurrently.
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let tmp_path = path.with_extension(format!("json.{}.{}.tmp", pid, nanos));
        if let Err(e) = std::fs::write(&tmp_path, &data) {
            log_warn!("images", "Failed to write cache temp file: {}", e);
            return;
        }
        if let Err(e) = std::fs::rename(&tmp_path, &path) {
            log_warn!("images", "Failed to rename cache file: {}", e);
            let _ = std::fs::remove_file(&tmp_path);
        } else {
            log_debug!("images", "Saved cache to {}", path.display());
        }
    });
}

/// Load data from a cache file
async fn load_cache(name: &str) -> Result<String, String> {
    let path = get_cache_path(name);
    if !path.exists() {
        return Err(format!(
            "No cached {} data available (first launch while offline)",
            name
        ));
    }

    let data = tokio::fs::read_to_string(&path).await.map_err(|e| {
        log_error!("images", "Failed to read {} cache: {}", name, e);
        format!("Failed to read cached data: {}", e)
    })?;

    log_debug!(
        "images",
        "Loaded {} data from local cache ({})",
        name,
        path.display()
    );
    Ok(data)
}

/// Delete old API cache file from pre-migration format
pub fn cleanup_legacy_cache() {
    let legacy_path = get_cache_dir(config::app::NAME)
        .join("assets")
        .join("api-images.json");
    if legacy_path.exists() {
        match std::fs::remove_file(&legacy_path) {
            Ok(_) => log_debug!(
                "images",
                "Removed legacy api-images.json cache: {}",
                legacy_path.display()
            ),
            Err(e) => log_warn!("images", "Failed to remove legacy cache: {}", e),
        }
    }
}

/// Fetch all boards from the Armbian REST API with pagination support.
///
/// Requests up to 500 boards per page and fetches additional pages if needed.
/// On success, saves the response to disk for offline use.
/// On failure, falls back to the local disk cache.
pub async fn fetch_boards() -> Result<Vec<ApiBoardSummary>, String> {
    log_debug!(
        "images",
        "Fetching boards from {}/boards",
        config::urls::API_BASE
    );

    match fetch_boards_from_api().await {
        Ok(boards) => {
            // Save to disk cache
            if let Ok(json) = serde_json::to_string(&boards) {
                save_cache("api-boards", &json);
            }
            Ok(boards)
        }
        Err(e) => {
            log_warn!("images", "API fetch failed, trying local cache: {}", e);
            let data = load_cache("api-boards").await?;
            serde_json::from_str(&data).map_err(|e| {
                log_error!("images", "Failed to parse boards cache: {}", e);
                format!("Failed to parse cached boards: {}", e)
            })
        }
    }
}

/// Fetch boards directly from the remote API, handling pagination.
///
/// Safety guards:
/// - Breaks early if the API returns an empty page (prevents infinite loops
///   when `meta.total` is inconsistent with actual data length).
/// - Enforces a hard `MAX_PAGES` cap as a final safety net.
async fn fetch_boards_from_api() -> Result<Vec<ApiBoardSummary>, String> {
    let url_base = format!("{}/boards", config::urls::API_BASE);
    let mut all_boards = Vec::new();
    let mut page: u32 = 1;
    let limit: u32 = 500;

    while page <= MAX_PAGES {
        let response: ApiResponse<Vec<ApiBoardSummary>> = API_CLIENT
            .get(&url_base)
            .query(&[("limit", limit.to_string()), ("page", page.to_string())])
            .send()
            .await
            .map_err(|e| format!("Failed to fetch boards: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Boards API returned error: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse boards response: {}", e))?;

        // Guard against runaway loops: stop immediately on empty page
        if response.data.is_empty() {
            break;
        }

        let total = response.meta.total.unwrap_or(0);
        all_boards.extend(response.data);

        // Stop when we've fetched enough to cover the advertised total
        if (page * limit) >= total {
            break;
        }
        page += 1;
    }

    if page > MAX_PAGES {
        log_warn!(
            "images",
            "Boards pagination reached MAX_PAGES cap ({}); stopping early",
            MAX_PAGES
        );
    }

    // Breakdown of support tiers for diagnostic purposes
    let mut platinum = 0u32;
    let mut standard = 0u32;
    let mut community = 0u32;
    let mut eos = 0u32;
    let mut other = 0u32;
    for b in &all_boards {
        match b.support_tier.as_str() {
            "platinum" => platinum += 1,
            "standard" => standard += 1,
            "community" => community += 1,
            "eos" => eos += 1,
            _ => other += 1,
        }
    }

    log_debug!(
        "images",
        "Successfully fetched {} boards from API (platinum: {}, standard: {}, community: {}, eos: {}, other: {})",
        all_boards.len(),
        platinum,
        standard,
        community,
        eos,
        other
    );
    Ok(all_boards)
}

/// Fetch images for a specific board from the Armbian REST API.
///
/// Supports optional query parameter filters passed directly to the API.
/// On success, saves the response to disk for offline use.
/// On failure, falls back to the local disk cache.
pub async fn fetch_images_for_board(
    slug: &str,
    variant: Option<&str>,
    distribution: Option<&str>,
    branch: Option<&str>,
    promoted: Option<bool>,
) -> Result<Vec<ApiImage>, String> {
    log_debug!("images", "Fetching images for board: {} from API", slug);

    let cache_name = format!("api-images-{}", slug);

    match fetch_images_from_api(slug, variant, distribution, branch, promoted).await {
        Ok(images) => {
            if let Ok(json) = serde_json::to_string(&images) {
                save_cache(&cache_name, &json);
            }
            Ok(images)
        }
        Err(e) => {
            log_warn!(
                "images",
                "API fetch for board {} failed, trying cache: {}",
                slug,
                e
            );
            let data = load_cache(&cache_name).await?;
            serde_json::from_str(&data).map_err(|e| {
                log_error!("images", "Failed to parse images cache for {}: {}", slug, e);
                format!("Failed to parse cached images: {}", e)
            })
        }
    }
}

/// Fetch images for a board directly from the remote API
async fn fetch_images_from_api(
    slug: &str,
    variant: Option<&str>,
    distribution: Option<&str>,
    branch: Option<&str>,
    promoted: Option<bool>,
) -> Result<Vec<ApiImage>, String> {
    let url = format!("{}/boards/{}/images", config::urls::API_BASE, slug);

    // Build query parameters using reqwest's encoder (handles URL-encoding)
    let mut params: Vec<(&str, String)> = Vec::new();
    if let Some(v) = variant {
        params.push(("variant", v.to_string()));
    }
    if let Some(d) = distribution {
        params.push(("distribution", d.to_string()));
    }
    if let Some(b) = branch {
        params.push(("branch", b.to_string()));
    }
    if let Some(p) = promoted {
        params.push(("promoted", p.to_string()));
    }

    let response: ApiResponse<Vec<ApiImage>> = API_CLIENT
        .get(&url)
        .query(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch images for {}: {}", slug, e))?
        .error_for_status()
        .map_err(|e| format!("Images API returned error for {}: {}", slug, e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse images response for {}: {}", slug, e))?;

    log_debug!(
        "images",
        "Successfully fetched {} images for board {}",
        response.data.len(),
        slug
    );
    Ok(response.data)
}

/// Fetch all vendors from the Armbian REST API.
///
/// On success, saves the response to disk for offline use.
/// On failure, falls back to the local disk cache.
pub async fn fetch_vendors() -> Result<Vec<ApiVendor>, String> {
    log_debug!(
        "images",
        "Fetching vendors from {}/vendors",
        config::urls::API_BASE
    );

    match fetch_vendors_from_api().await {
        Ok(vendors) => {
            if let Ok(json) = serde_json::to_string(&vendors) {
                save_cache("api-vendors", &json);
            }
            Ok(vendors)
        }
        Err(e) => {
            log_warn!("images", "Vendors API fetch failed, trying cache: {}", e);
            let data = load_cache("api-vendors").await?;
            serde_json::from_str(&data).map_err(|e| {
                log_error!("images", "Failed to parse vendors cache: {}", e);
                format!("Failed to parse cached vendors: {}", e)
            })
        }
    }
}

/// Fetch vendors directly from the remote API
async fn fetch_vendors_from_api() -> Result<Vec<ApiVendor>, String> {
    let url = format!("{}/vendors", config::urls::API_BASE);

    let response: ApiResponse<Vec<ApiVendor>> = API_CLIENT
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch vendors: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Vendors API returned error: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse vendors response: {}", e))?;

    log_debug!(
        "images",
        "Successfully fetched {} vendors from API",
        response.data.len()
    );
    Ok(response.data)
}

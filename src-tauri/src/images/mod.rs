//! Image management module
//!
//! Handles fetching, parsing, and filtering Armbian image data.

mod filters;
mod models;

// Re-export types and functions
pub use filters::{extract_images, filter_images_for_board, get_unique_boards};
pub use models::{BoardInfo, ImageInfo};
// ArmbianImage is used internally by filters module

use crate::config;
use crate::{log_debug, log_error, log_info, log_warn};
use std::collections::HashMap;
use std::sync::RwLock;

/// GitHub releases API URL for Armbian OS releases
const GITHUB_RELEASES_API: &str = "https://api.github.com/repos/armbian/os/releases/latest";

/// Cached SHA256 digests from GitHub releases (filename -> sha256 hash)
static DIGEST_CACHE: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);

/// Fetch the all-images.json from Armbian
pub async fn fetch_all_images() -> Result<serde_json::Value, String> {
    log_info!(
        "images",
        "Fetching all images from {}",
        config::urls::ALL_IMAGES
    );

    let response = reqwest::get(config::urls::ALL_IMAGES).await.map_err(|e| {
        log_error!("images", "Failed to fetch images: {}", e);
        format!("Failed to fetch images: {}", e)
    })?;

    let json: serde_json::Value = response.json().await.map_err(|e| {
        log_error!("images", "Failed to parse JSON response: {}", e);
        format!("Failed to parse JSON: {}", e)
    })?;

    log_info!("images", "Successfully fetched images data");
    Ok(json)
}

/// Fetch SHA256 digests from GitHub releases API
/// Returns a map of filename -> sha256 hash
pub async fn fetch_github_digests() -> Result<HashMap<String, String>, String> {
    // Check cache first
    {
        let cache = DIGEST_CACHE.read().map_err(|e| format!("Cache lock error: {}", e))?;
        if let Some(ref digests) = *cache {
            log_debug!("images", "Using cached GitHub digests ({} entries)", digests.len());
            return Ok(digests.clone());
        }
    }

    log_info!("images", "Fetching GitHub release digests from {}", GITHUB_RELEASES_API);

    let client = reqwest::Client::builder()
        .user_agent(config::app::USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(GITHUB_RELEASES_API)
        .send()
        .await
        .map_err(|e| {
            log_error!("images", "Failed to fetch GitHub releases: {}", e);
            format!("Failed to fetch GitHub releases: {}", e)
        })?;

    if !response.status().is_success() {
        return Err(format!(
            "GitHub API request failed with status: {}",
            response.status()
        ));
    }

    let release: serde_json::Value = response.json().await.map_err(|e| {
        log_error!("images", "Failed to parse GitHub releases JSON: {}", e);
        format!("Failed to parse GitHub releases: {}", e)
    })?;

    let mut digests = HashMap::new();

    // Parse assets array
    if let Some(assets) = release.get("assets").and_then(|a| a.as_array()) {
        for asset in assets {
            // Get the filename from "name" field
            let name = match asset.get("name").and_then(|n| n.as_str()) {
                Some(n) => n,
                None => continue,
            };

            // Get the digest from "digest" field (format: "sha256:...")
            if let Some(digest) = asset.get("digest").and_then(|d| d.as_str()) {
                // Extract just the hash part (remove "sha256:" prefix)
                let hash = if let Some(stripped) = digest.strip_prefix("sha256:") {
                    stripped.to_lowercase()
                } else {
                    // If no prefix, use the whole string
                    digest.to_lowercase()
                };

                // Validate it looks like a SHA256 hash (64 hex chars)
                if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
                    digests.insert(name.to_string(), hash);
                } else {
                    log_warn!("images", "Invalid digest format for {}: {}", name, digest);
                }
            }
        }
    }

    log_info!("images", "Loaded {} digests from GitHub releases", digests.len());

    // Cache the result
    {
        let mut cache = DIGEST_CACHE.write().map_err(|e| format!("Cache lock error: {}", e))?;
        *cache = Some(digests.clone());
    }

    Ok(digests)
}

/// Clear the GitHub digests cache (useful for refresh)
#[allow(dead_code)]
pub fn clear_digest_cache() {
    if let Ok(mut cache) = DIGEST_CACHE.write() {
        *cache = None;
        log_debug!("images", "Cleared GitHub digests cache");
    }
}

/// Look up SHA256 digest for a filename
/// Fetches from GitHub API if not cached
pub async fn get_digest_for_file(filename: &str) -> Option<String> {
    match fetch_github_digests().await {
        Ok(digests) => {
            // Try exact match first
            if let Some(hash) = digests.get(filename) {
                return Some(hash.clone());
            }
            
            // Try without path (just the filename)
            let base_filename = filename.rsplit('/').next().unwrap_or(filename);
            if let Some(hash) = digests.get(base_filename) {
                return Some(hash.clone());
            }
            
            log_debug!("images", "No digest found for filename: {}", filename);
            None
        }
        Err(e) => {
            log_warn!("images", "Failed to fetch digests: {}", e);
            None
        }
    }
}

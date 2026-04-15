//! Picture cache module
//!
//! Local disk cache for board images and vendor logos with ETag-based
//! conditional refresh. Cache-first strategy: always serve from local
//! cache immediately, refresh stale entries in the background.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use base64::Engine;

use crate::config;
use crate::utils::get_cache_dir;
use crate::{log_debug, log_info, log_warn};

const MODULE: &str = "picture_cache";

/// Staleness threshold: 24 hours in seconds
const STALE_THRESHOLD_SECS: u64 = 24 * 60 * 60;

/// Maximum concurrent background refresh requests
const MAX_CONCURRENT_REFRESHES: usize = 5;

/// Metadata for a single cached asset
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AssetEntry {
    /// ETag header from the server
    etag: Option<String>,
    /// Last-Modified header from the server
    last_modified: Option<String>,
    /// Unix timestamp of last freshness check
    last_checked: u64,
    /// Original remote URL (for background refresh)
    #[serde(default)]
    url: Option<String>,
}

/// Root metadata structure persisted as meta.json
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct AssetsMeta {
    entries: HashMap<String, AssetEntry>,
}

/// Global metadata state protected by async mutex
static META: once_cell::sync::Lazy<Mutex<Option<AssetsMeta>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));

/// Shared HTTP client for asset downloads (10s timeout)
static HTTP_CLIENT: once_cell::sync::Lazy<reqwest::Client> = once_cell::sync::Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client")
});

/// Get the assets cache base directory
fn get_assets_dir() -> PathBuf {
    get_cache_dir(config::app::NAME).join("assets")
}

/// Get the path to meta.json
fn get_meta_path() -> PathBuf {
    get_assets_dir().join("meta.json")
}

/// Get current unix timestamp
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Initialize metadata from disk if not yet loaded (helper for use under lock)
///
/// Uses `get_or_insert_with` to avoid redundant disk reads once
/// the in-memory cache is populated.
fn init_meta_from_disk(guard: &mut Option<AssetsMeta>) -> &mut AssetsMeta {
    guard.get_or_insert_with(|| {
        let meta_path = get_meta_path();
        if meta_path.exists() {
            match std::fs::read_to_string(&meta_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
                    log_warn!(MODULE, "Corrupted meta.json, starting fresh: {}", e);
                    AssetsMeta::default()
                }),
                Err(e) => {
                    log_warn!(MODULE, "Failed to read meta.json: {}", e);
                    AssetsMeta::default()
                }
            }
        } else {
            AssetsMeta::default()
        }
    })
}

/// Persist metadata to disk (helper for use under lock)
///
/// Uses synchronous I/O because the caller already holds the async mutex
/// and the metadata file is small. This keeps the critical section atomic.
fn persist_meta_to_disk(meta: &AssetsMeta) {
    let meta_path = get_meta_path();
    if let Some(parent) = meta_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(meta) {
        if let Err(e) = std::fs::write(&meta_path, json) {
            log_warn!(MODULE, "Failed to write meta.json: {}", e);
        }
    }
}

/// Load metadata from disk, initializing if needed
async fn load_meta() -> AssetsMeta {
    let mut guard = META.lock().await;
    init_meta_from_disk(&mut guard).clone()
}

/// Atomically update a single entry in metadata and persist to disk
///
/// Holds the mutex for the entire read-modify-write cycle to prevent
/// concurrent tasks from overwriting each other's changes.
async fn update_entry(key: &str, entry: AssetEntry) {
    let mut guard = META.lock().await;
    let meta = init_meta_from_disk(&mut guard);
    meta.entries.insert(key.to_string(), entry);
    persist_meta_to_disk(meta);
}

/// Get a cached asset, downloading if needed
///
/// Cache-first strategy:
/// - If file exists locally, return path immediately (spawn background
///   refresh if stale, i.e. not checked in over 24 hours)
/// - If file is missing, download synchronously and return path
/// - If download fails, return `None`
///
/// # Arguments
/// * `kind` - Asset category directory: `"boards"` or `"vendors"`
/// * `key` - Asset identifier (board slug or vendor id)
/// * `remote_url` - Full URL to download from
///
/// # Returns
/// Local file path on success, or `None` if the asset could not be obtained
pub async fn get_asset(kind: &str, key: &str, remote_url: &str) -> Option<PathBuf> {
    // Reject keys with path traversal characters
    if key.contains('/') || key.contains('\\') || key.contains("..") {
        log_warn!(MODULE, "Rejected invalid asset key: {}", key);
        return None;
    }

    let assets_dir = get_assets_dir();
    let asset_dir = assets_dir.join(kind);
    let file_path = asset_dir.join(format!("{}.png", key));
    let meta_key = format!("{}/{}", kind, key);

    // Cache hit: return path immediately, spawn background refresh if stale
    if file_path.exists() {
        let meta = load_meta().await;
        let entry = meta.entries.get(&meta_key);

        let is_stale = entry
            .map(|e| now_secs().saturating_sub(e.last_checked) > STALE_THRESHOLD_SECS)
            .unwrap_or(true);

        if is_stale {
            let url = remote_url.to_string();
            let key_owned = meta_key.clone();
            let path = file_path.clone();
            let etag = entry.and_then(|e| e.etag.clone());
            let last_mod = entry.and_then(|e| e.last_modified.clone());

            tokio::spawn(async move {
                refresh_asset(
                    &key_owned,
                    &url,
                    &path,
                    etag.as_deref(),
                    last_mod.as_deref(),
                )
                .await;
            });
        }

        return Some(file_path);
    }

    // Cache miss: download and return path (or None on failure)
    if let Err(e) = tokio::fs::create_dir_all(&asset_dir).await {
        log_warn!(
            MODULE,
            "Failed to create asset directory {}: {}",
            asset_dir.display(),
            e
        );
        return None;
    }

    download_asset(&meta_key, remote_url, &file_path).await
}

/// Download an asset for the first time
async fn download_asset(meta_key: &str, url: &str, file_path: &Path) -> Option<PathBuf> {
    log_debug!(
        MODULE,
        "Downloading asset: {} -> {}",
        url,
        file_path.display()
    );

    let response = match HTTP_CLIENT.get(url).send().await {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            log_debug!(MODULE, "Asset download returned {}: {}", r.status(), url);
            // Record the check so we don't retry immediately
            update_entry(
                meta_key,
                AssetEntry {
                    etag: None,
                    last_modified: None,
                    last_checked: now_secs(),
                    url: Some(url.to_string()),
                },
            )
            .await;
            return None;
        }
        Err(e) => {
            log_debug!(MODULE, "Asset download failed: {} ({})", url, e);
            return None;
        }
    };

    // Extract cache headers
    let etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let last_modified = response
        .headers()
        .get("last-modified")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Read body
    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            log_warn!(MODULE, "Failed to read asset body: {}", e);
            return None;
        }
    };

    // Write to disk
    if let Err(e) = tokio::fs::write(file_path, &bytes).await {
        log_warn!(
            MODULE,
            "Failed to write asset to {}: {}",
            file_path.display(),
            e
        );
        return None;
    }

    // Update metadata
    update_entry(
        meta_key,
        AssetEntry {
            etag,
            last_modified,
            last_checked: now_secs(),
            url: Some(url.to_string()),
        },
    )
    .await;

    log_debug!(MODULE, "Cached asset: {}", file_path.display());
    Some(file_path.to_path_buf())
}

/// Refresh a stale cached asset using conditional request
async fn refresh_asset(
    meta_key: &str,
    url: &str,
    file_path: &Path,
    etag: Option<&str>,
    last_modified: Option<&str>,
) {
    let mut request = HTTP_CLIENT.get(url);

    // Add conditional headers
    if let Some(etag) = etag {
        request = request.header("If-None-Match", etag);
    }
    if let Some(last_modified) = last_modified {
        request = request.header("If-Modified-Since", last_modified);
    }

    match request.send().await {
        Ok(response) => {
            if response.status() == reqwest::StatusCode::NOT_MODIFIED {
                // Asset unchanged -- just update last_checked
                log_debug!(MODULE, "Asset unchanged (304): {}", meta_key);
                update_entry(
                    meta_key,
                    AssetEntry {
                        etag: etag.map(|s| s.to_string()),
                        last_modified: last_modified.map(|s| s.to_string()),
                        last_checked: now_secs(),
                        url: Some(url.to_string()),
                    },
                )
                .await;
            } else if response.status().is_success() {
                // Asset updated -- save new version
                let new_etag = response
                    .headers()
                    .get("etag")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                let new_last_modified = response
                    .headers()
                    .get("last-modified")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                if let Ok(bytes) = response.bytes().await {
                    if let Err(e) = tokio::fs::write(file_path, &bytes).await {
                        log_warn!(
                            MODULE,
                            "Failed to update asset {}: {}",
                            file_path.display(),
                            e
                        );
                        return;
                    }
                    log_info!(MODULE, "Updated cached asset: {}", meta_key);
                }

                update_entry(
                    meta_key,
                    AssetEntry {
                        etag: new_etag,
                        last_modified: new_last_modified,
                        last_checked: now_secs(),
                        url: Some(url.to_string()),
                    },
                )
                .await;
            } else {
                // Error response -- just update last_checked to avoid retrying
                log_debug!(
                    MODULE,
                    "Asset refresh returned {}: {}",
                    response.status(),
                    meta_key
                );
                update_entry(
                    meta_key,
                    AssetEntry {
                        etag: etag.map(|s| s.to_string()),
                        last_modified: last_modified.map(|s| s.to_string()),
                        last_checked: now_secs(),
                        url: Some(url.to_string()),
                    },
                )
                .await;
            }
        }
        Err(e) => {
            // Network error -- keep stale cache, don't update last_checked
            log_debug!(MODULE, "Asset refresh failed for {}: {}", meta_key, e);
        }
    }
}

/// Read a cached image file and return it as a data URI (base64-encoded)
///
/// Returns `Some("data:image/png;base64,...")` if the file can be read,
/// or `None` on any error. Works on all platforms without protocol issues.
pub async fn read_as_data_uri(path: &Path) -> Option<String> {
    match tokio::fs::read(path).await {
        Ok(bytes) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            Some(format!("data:image/png;base64,{}", b64))
        }
        Err(e) => {
            log_warn!(
                MODULE,
                "Failed to read cached asset {}: {}",
                path.display(),
                e
            );
            None
        }
    }
}

/// Pre-populate the asset cache by downloading all board images and vendor logos
///
/// Fetches board and vendor lists from the Armbian REST API (with disk cache fallback),
/// then downloads images and logos using a semaphore to limit concurrency.
/// Intended to be called once at app startup in the background.
pub async fn prepopulate_assets() {
    log_info!(MODULE, "Pre-populating asset cache...");

    // Fetch boards and vendors from API (with disk cache fallback)
    let boards = match crate::images::fetch_boards().await {
        Ok(b) => b,
        Err(e) => {
            log_warn!(MODULE, "Cannot fetch boards for prepopulate: {}", e);
            return;
        }
    };

    let vendors = match crate::images::fetch_vendors().await {
        Ok(v) => v,
        Err(e) => {
            log_warn!(MODULE, "Cannot fetch vendors for prepopulate: {}", e);
            Vec::new() // Continue with board images even if vendors fail
        }
    };

    let total = boards.len() + vendors.len();
    log_debug!(
        MODULE,
        "Pre-populating {} board images + {} vendor logos ({} total)",
        boards.len(),
        vendors.len(),
        total
    );

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_REFRESHES));
    let mut handles = Vec::new();

    // Download board images
    for board in &boards {
        let sem = semaphore.clone();
        let slug = board.slug.clone();
        handles.push(tokio::spawn(async move {
            let _permit = match sem.acquire().await {
                Ok(p) => p,
                Err(_) => return,
            };
            let url = format!(
                "{}{}/{}.png",
                config::urls::BOARD_IMAGES_BASE,
                config::urls::BOARD_IMAGE_SIZE,
                slug
            );
            get_asset("boards", &slug, &url).await;
        }));
    }

    // Download vendor logos
    for vendor in &vendors {
        let sem = semaphore.clone();
        let slug = vendor.slug.clone();
        handles.push(tokio::spawn(async move {
            let _permit = match sem.acquire().await {
                Ok(p) => p,
                Err(_) => return,
            };
            let url = format!("{}{}.png", config::urls::VENDOR_IMAGES_BASE, slug);
            get_asset("vendors", &slug, &url).await;
        }));
    }

    let mut completed = 0;
    for handle in handles {
        if handle.await.is_ok() {
            completed += 1;
        }
    }

    log_info!(
        MODULE,
        "Pre-populate complete: {}/{} assets processed",
        completed,
        total
    );
}

/// Refresh all stale assets in the background
///
/// Iterates all entries in meta.json and sends conditional requests
/// for entries that haven't been checked in over 24 hours.
/// Uses a semaphore to limit concurrent requests.
pub async fn refresh_stale_assets() {
    let meta = load_meta().await;

    if meta.entries.is_empty() {
        log_debug!(MODULE, "No cached assets to refresh");
        return;
    }

    let now = now_secs();
    let stale_entries: Vec<(String, AssetEntry)> = meta
        .entries
        .iter()
        .filter(|(_, entry)| now.saturating_sub(entry.last_checked) > STALE_THRESHOLD_SECS)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if stale_entries.is_empty() {
        log_debug!(MODULE, "All {} cached assets are fresh", meta.entries.len());
        return;
    }

    log_debug!(
        MODULE,
        "Refreshing {} stale assets (of {} total)",
        stale_entries.len(),
        meta.entries.len()
    );

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_REFRESHES));
    let mut handles = Vec::new();

    for (key, entry) in stale_entries {
        let sem = semaphore.clone();
        let assets_dir = get_assets_dir();

        handles.push(tokio::spawn(async move {
            let _permit = match sem.acquire().await {
                Ok(p) => p,
                Err(_) => return,
            };

            // Reconstruct file path and URL from key
            let parts: Vec<&str> = key.splitn(2, '/').collect();
            if parts.len() != 2 {
                return;
            }
            let kind = parts[0];
            let asset_key = parts[1];
            let file_path = assets_dir.join(kind).join(format!("{}.png", asset_key));

            // Only refresh if file still exists on disk
            if !file_path.exists() {
                return;
            }

            // Reconstruct URL from kind and asset key
            let url = match kind {
                "boards" => format!(
                    "{}{}/{}.png",
                    config::urls::BOARD_IMAGES_BASE,
                    config::urls::BOARD_IMAGE_SIZE,
                    asset_key
                ),
                "vendors" => format!("{}{}.png", config::urls::VENDOR_IMAGES_BASE, asset_key),
                _ => match &entry.url {
                    Some(u) => u.clone(),
                    None => return,
                },
            };

            refresh_asset(
                &key,
                &url,
                &file_path,
                entry.etag.as_deref(),
                entry.last_modified.as_deref(),
            )
            .await;
        }));
    }

    // Wait for all refreshes to complete
    let mut processed = 0;
    for handle in handles {
        if handle.await.is_ok() {
            processed += 1;
        }
    }

    log_debug!(
        MODULE,
        "Background refresh complete: {} assets processed",
        processed
    );
}

//! Download module
//!
//! Handles downloading Armbian images from the web.

use futures_util::StreamExt;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config;
use crate::decompress::decompress_with_rust_xz;
use crate::utils::{bytes_to_mb, ProgressTracker};
use crate::{log_debug, log_error, log_info, log_warn};

const MODULE: &str = "download";

/// Download progress state
pub struct DownloadState {
    pub total_bytes: AtomicU64,
    pub downloaded_bytes: AtomicU64,
    pub is_verifying_sha: AtomicBool,
    pub is_decompressing: AtomicBool,
    pub is_cancelled: AtomicBool,
    pub error: Mutex<Option<String>>,
    pub output_path: Mutex<Option<PathBuf>>,
}

impl DownloadState {
    pub fn new() -> Self {
        Self {
            total_bytes: AtomicU64::new(0),
            downloaded_bytes: AtomicU64::new(0),
            is_verifying_sha: AtomicBool::new(false),
            is_decompressing: AtomicBool::new(false),
            is_cancelled: AtomicBool::new(false),
            error: Mutex::new(None),
            output_path: Mutex::new(None),
        }
    }

    pub fn reset(&self) {
        self.total_bytes.store(0, Ordering::SeqCst);
        self.downloaded_bytes.store(0, Ordering::SeqCst);
        self.is_verifying_sha.store(false, Ordering::SeqCst);
        self.is_decompressing.store(false, Ordering::SeqCst);
        self.is_cancelled.store(false, Ordering::SeqCst);
    }
}

impl Default for DownloadState {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract filename from URL
fn extract_filename(url: &str) -> Result<&str, String> {
    log_debug!(MODULE, "Extracting filename from URL: {}", url);
    let url_path = url.split('?').next().unwrap_or(url);
    let filename = url_path
        .split('/')
        .next_back()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Invalid URL: no filename".to_string())?;
    log_debug!(MODULE, "Extracted filename: {}", filename);
    Ok(filename)
}

/// Check if URL is from GitHub (uses digest from releases API)
fn is_github_url(url: &str) -> bool {
    url.contains("github.com")
}

/// Fetch expected SHA256 from a .sha URL (for dl.armbian.com)
async fn fetch_sha_from_url(client: &Client, sha_url: &str) -> Result<String, String> {
    log_debug!(MODULE, "Fetching SHA256 from: {}", sha_url);

    let response = client
        .get(sha_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch SHA: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "SHA fetch failed with status: {}",
            response.status()
        ));
    }

    let content = response
        .text()
        .await
        .map_err(|e| format!("Failed to read SHA response: {}", e))?;

    // Parse SHA file format: "hash *filename" or "hash  filename"
    let hash = content
        .split_whitespace()
        .next()
        .ok_or("Invalid SHA file format")?
        .to_lowercase();

    // Validate it looks like a SHA256 hash (64 hex chars)
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("Invalid SHA256 hash format: {}", hash));
    }

    log_debug!(MODULE, "Expected SHA256: {}", hash);
    Ok(hash)
}

/// Fetch expected SHA256 from GitHub releases API based on filename
async fn fetch_sha_from_github(filename: &str) -> Result<String, String> {
    log_debug!(MODULE, "Looking up SHA256 digest for: {}", filename);

    // Use the GitHub releases API to get the digest
    match crate::images::get_digest_for_file(filename).await {
        Some(hash) => {
            log_debug!(MODULE, "Found SHA256 digest: {}", hash);
            Ok(hash)
        }
        None => {
            Err(format!("No SHA256 digest found for file: {}", filename))
        }
    }
}

/// Calculate SHA256 of a file
fn calculate_file_sha256(path: &Path, state: &Arc<DownloadState>) -> Result<String, String> {
    log_debug!(MODULE, "Calculating SHA256 of: {}", path.display());
    log_debug!(
        MODULE,
        "File size: {:?} bytes",
        path.metadata().ok().map(|m| m.len())
    );

    let mut file = File::open(path).map_err(|e| format!("Failed to open file for SHA: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    let mut bytes_processed = 0u64;

    loop {
        // Check for cancellation
        if state.is_cancelled.load(Ordering::SeqCst) {
            log_info!(MODULE, "SHA256 calculation cancelled by user");
            return Err("SHA256 verification cancelled".to_string());
        }

        let bytes_read = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read file for SHA: {}", e))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
        bytes_processed += bytes_read as u64;

        // Log progress every 10MB in debug mode
        if bytes_processed % (10 * 1024 * 1024) == 0 {
            log_debug!(
                MODULE,
                "SHA256 calculation progress: {} MB",
                bytes_processed / (1024 * 1024)
            );
        }
    }

    let result = hasher.finalize();
    let hash = format!("{:x}", result);
    log_debug!(MODULE, "Calculated SHA256: {}", hash);
    Ok(hash)
}

/// Verify file SHA256 against expected value
/// For GitHub URLs: uses digest from releases API
/// For other URLs: uses the provided sha_url to download .sha file
async fn verify_sha256(
    client: &Client,
    file_path: &Path,
    filename: &str,
    url: &str,
    sha_url: Option<&str>,
    state: &Arc<DownloadState>,
) -> Result<(), String> {
    // Check cancellation before fetching
    if state.is_cancelled.load(Ordering::SeqCst) {
        return Err("SHA256 verification cancelled".to_string());
    }

    // Get expected SHA based on source
    let expected = if is_github_url(url) {
        // GitHub: use releases API digest
        fetch_sha_from_github(filename).await?
    } else if let Some(sha_url) = sha_url {
        // Other sources: download .sha file
        fetch_sha_from_url(client, sha_url).await?
    } else {
        return Err("No SHA verification method available".to_string());
    };

    // Check cancellation after fetching
    if state.is_cancelled.load(Ordering::SeqCst) {
        return Err("SHA256 verification cancelled".to_string());
    }

    let actual = calculate_file_sha256(file_path, state)?;

    if expected == actual {
        log_info!(MODULE, "SHA256 verification PASSED");
        Ok(())
    } else {
        log_error!(
            MODULE,
            "SHA256 verification FAILED! Expected: {}, Got: {}",
            expected,
            actual
        );
        Err(format!(
            "SHA256 mismatch: expected {}, got {}",
            expected, actual
        ))
    }
}

/// Download and decompress an Armbian image
/// For GitHub URLs: verifies using digest from releases API
/// For other URLs: verifies using provided sha_url
pub async fn download_image(
    url: &str,
    sha_url: Option<&str>,
    output_dir: &PathBuf,
    state: Arc<DownloadState>,
) -> Result<PathBuf, String> {
    state.reset();

    let filename = extract_filename(url)?;

    // Determine output filename (remove .xz if present)
    let output_filename = filename.trim_end_matches(".xz");
    let output_path = output_dir.join(output_filename);

    log_info!(MODULE, "Download requested: {}", url);
    log_debug!(MODULE, "Output path: {}", output_path.display());

    // Check if image is already in cache (also updates mtime for LRU)
    if let Some(cached_path) = crate::cache::get_cached_image(output_filename) {
        log_info!(MODULE, "Using cached image: {}", cached_path.display());
        *state.output_path.lock().await = Some(cached_path.clone());
        return Ok(cached_path);
    }

    // Create output directory if needed
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let client = Client::builder()
        .user_agent(config::app::USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Start download
    log_info!(MODULE, "Starting download...");
    let response = client.get(url).send().await.map_err(|e| {
        log_error!(MODULE, "Failed to start download: {}", e);
        format!("Failed to start download: {}", e)
    })?;

    if !response.status().is_success() {
        log_error!(MODULE, "Download failed with status: {}", response.status());
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    // Get content length
    let total_size = response.content_length().unwrap_or(0);
    state.total_bytes.store(total_size, Ordering::SeqCst);

    log_info!(
        MODULE,
        "Download size: {} bytes ({:.2} MB)",
        total_size,
        bytes_to_mb(total_size)
    );

    // Create temp file for compressed data
    let temp_path = output_dir.join(format!("{}.downloading", filename));
    let mut temp_file =
        File::create(&temp_path).map_err(|e| format!("Failed to create temp file: {}", e))?;

    // Download with progress tracking
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut tracker = ProgressTracker::new(
        "Download",
        MODULE,
        total_size,
        config::logging::DOWNLOAD_LOG_INTERVAL_MB,
    );

    while let Some(chunk) = stream.next().await {
        if state.is_cancelled.load(Ordering::SeqCst) {
            log_info!(MODULE, "Download cancelled by user");
            drop(temp_file);
            let _ = std::fs::remove_file(&temp_path);
            return Err("Download cancelled".to_string());
        }

        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        temp_file
            .write_all(&chunk)
            .map_err(|e| format!("Failed to write chunk: {}", e))?;

        downloaded += chunk.len() as u64;
        state.downloaded_bytes.store(downloaded, Ordering::SeqCst);
        tracker.update(chunk.len() as u64);
    }

    drop(temp_file);
    tracker.finish();

    // Verify SHA256 based on download source
    let can_verify = is_github_url(url) || sha_url.is_some();
    if can_verify {
        state.is_verifying_sha.store(true, Ordering::SeqCst);
        if is_github_url(url) {
            log_info!(MODULE, "Verifying SHA256 (from GitHub releases)...");
        } else {
            log_info!(MODULE, "Verifying SHA256 (from .sha file)...");
        }
        match verify_sha256(&client, &temp_path, filename, url, sha_url, &state).await {
            Ok(()) => {
                log_info!(MODULE, "SHA256 verification successful");
            }
            Err(e) => {
                log_error!(MODULE, "SHA256 verification failed: {}", e);
                state.is_verifying_sha.store(false, Ordering::SeqCst);
                let _ = std::fs::remove_file(&temp_path);
                // Check if it was a cancellation
                if state.is_cancelled.load(Ordering::SeqCst) {
                    return Err("Download cancelled".to_string());
                }
                return Err(format!("SHA256 verification failed: {}", e));
            }
        }
        state.is_verifying_sha.store(false, Ordering::SeqCst);
    } else {
        log_warn!(MODULE, "No SHA URL provided, skipping verification");
    }

    // Decompress if needed
    if filename.ends_with(".xz") {
        state.is_decompressing.store(true, Ordering::SeqCst);
        log_info!(
            MODULE,
            "Starting decompression with Rust lzma-rust2 (multi-threaded)..."
        );

        // Use Rust lzma-rust2 library (multi-threaded) on all platforms
        decompress_with_rust_xz(&temp_path, &output_path, &state)?;
        log_info!(MODULE, "Decompression complete");

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_path);
    } else {
        // No decompression needed, just rename
        std::fs::rename(&temp_path, &output_path)
            .map_err(|e| format!("Failed to move file: {}", e))?;
    }

    log_info!(MODULE, "Image ready: {}", output_path.display());
    *state.output_path.lock().await = Some(output_path.clone());
    Ok(output_path)
}

/// Download an Armbian image without decompression
/// For GitHub URLs: verifies using digest from releases API
/// For other URLs: verifies using provided sha_url
/// Returns the path to the compressed file (keeps .xz extension)
pub async fn download_image_raw(
    url: &str,
    sha_url: Option<&str>,
    output_dir: &PathBuf,
    state: Arc<DownloadState>,
) -> Result<PathBuf, String> {
    state.reset();

    let filename = extract_filename(url)?;
    let output_path = output_dir.join(filename);

    log_info!(MODULE, "Download (raw/compressed) requested: {}", url);
    log_debug!(MODULE, "Output path: {}", output_path.display());

    // Check if compressed image is already in cache
    if let Some(cached_path) = crate::cache::get_cached_image(filename) {
        log_info!(MODULE, "Using cached image: {}", cached_path.display());
        *state.output_path.lock().await = Some(cached_path.clone());
        return Ok(cached_path);
    }

    // Create output directory if needed
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let client = Client::builder()
        .user_agent(config::app::USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Start download
    log_info!(MODULE, "Starting download...");
    let response = client.get(url).send().await.map_err(|e| {
        log_error!(MODULE, "Failed to start download: {}", e);
        format!("Failed to start download: {}", e)
    })?;

    if !response.status().is_success() {
        log_error!(MODULE, "Download failed with status: {}", response.status());
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    // Get content length
    let total_size = response.content_length().unwrap_or(0);
    state.total_bytes.store(total_size, Ordering::SeqCst);

    log_info!(
        MODULE,
        "Download size: {} bytes ({:.2} MB)",
        total_size,
        bytes_to_mb(total_size)
    );

    // Create temp file
    let temp_path = output_dir.join(format!("{}.downloading", filename));
    let mut temp_file =
        File::create(&temp_path).map_err(|e| format!("Failed to create temp file: {}", e))?;

    // Download with progress tracking
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut tracker = ProgressTracker::new(
        "Download",
        MODULE,
        total_size,
        config::logging::DOWNLOAD_LOG_INTERVAL_MB,
    );

    while let Some(chunk) = stream.next().await {
        if state.is_cancelled.load(Ordering::SeqCst) {
            log_info!(MODULE, "Download cancelled by user");
            drop(temp_file);
            let _ = std::fs::remove_file(&temp_path);
            return Err("Download cancelled".to_string());
        }

        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        temp_file
            .write_all(&chunk)
            .map_err(|e| format!("Failed to write chunk: {}", e))?;

        downloaded += chunk.len() as u64;
        state.downloaded_bytes.store(downloaded, Ordering::SeqCst);
        tracker.update(chunk.len() as u64);
    }

    drop(temp_file);
    tracker.finish();

    // Verify SHA256 based on download source
    let can_verify = is_github_url(url) || sha_url.is_some();
    if can_verify {
        state.is_verifying_sha.store(true, Ordering::SeqCst);
        if is_github_url(url) {
            log_info!(MODULE, "Verifying SHA256 (from GitHub releases)...");
        } else {
            log_info!(MODULE, "Verifying SHA256 (from .sha file)...");
        }
        match verify_sha256(&client, &temp_path, filename, url, sha_url, &state).await {
            Ok(()) => {
                log_info!(MODULE, "SHA256 verification successful");
            }
            Err(e) => {
                log_error!(MODULE, "SHA256 verification failed: {}", e);
                state.is_verifying_sha.store(false, Ordering::SeqCst);
                let _ = std::fs::remove_file(&temp_path);
                if state.is_cancelled.load(Ordering::SeqCst) {
                    return Err("Download cancelled".to_string());
                }
                return Err(format!("SHA256 verification failed: {}", e));
            }
        }
        state.is_verifying_sha.store(false, Ordering::SeqCst);
    } else {
        log_warn!(MODULE, "No SHA URL provided, skipping verification");
    }

    // Move temp file to final location (no decompression)
    std::fs::rename(&temp_path, &output_path)
        .map_err(|e| format!("Failed to move file: {}", e))?;

    log_info!(MODULE, "Image ready (compressed): {}", output_path.display());
    *state.output_path.lock().await = Some(output_path.clone());
    Ok(output_path)
}

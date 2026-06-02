//! Downloading Armbian images from the web.

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
use crate::utils::{bytes_to_mb, validate_cache_path, ProgressTracker};
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
    /// Temp file kept when SHA is unavailable, so the user can decide to proceed.
    pub temp_path: Mutex<Option<PathBuf>>,
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
            temp_path: Mutex::new(None),
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

/// Fetch the expected SHA256. Errors are prefixed [SHA_UNAVAILABLE] to
/// distinguish a fetch failure from a genuine mismatch.
async fn fetch_expected_sha(client: &Client, sha_url: &str) -> Result<String, String> {
    log_debug!(MODULE, "Fetching SHA256 from: {}", sha_url);

    let response = client
        .get(sha_url)
        .send()
        .await
        .map_err(|e| format!("[SHA_UNAVAILABLE] Failed to fetch SHA: {}", e))?;

    // Post-redirect URL reveals which mirror served the SHA.
    let final_sha_url = response.url().to_string();
    if final_sha_url != sha_url {
        log_debug!(MODULE, "SHA redirected to mirror: {}", final_sha_url);
    }

    if !response.status().is_success() {
        return Err(format!(
            "[SHA_UNAVAILABLE] SHA fetch failed with status: {}",
            response.status()
        ));
    }

    let content = response
        .text()
        .await
        .map_err(|e| format!("[SHA_UNAVAILABLE] Failed to read SHA response: {}", e))?;

    // SHA file format is "hash *filename" or "hash  filename".
    let hash = content
        .split_whitespace()
        .next()
        .ok_or("[SHA_UNAVAILABLE] Invalid SHA file format")?
        .to_lowercase();

    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!(
            "[SHA_UNAVAILABLE] Invalid SHA256 hash format: {}",
            hash
        ));
    }

    log_debug!(MODULE, "Expected SHA256: {}", hash);
    Ok(hash)
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
    let mut buffer = [0u8; config::logging::SHA_BUFFER_SIZE];
    let mut bytes_processed = 0u64;

    loop {
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
async fn verify_sha256(
    client: &Client,
    file_path: &Path,
    sha_url: &str,
    state: &Arc<DownloadState>,
) -> Result<(), String> {
    if state.is_cancelled.load(Ordering::SeqCst) {
        return Err("SHA256 verification cancelled".to_string());
    }

    let expected = fetch_expected_sha(client, sha_url).await?;

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

/// Download and decompress an Armbian image; when sha_url is given, verifies the compressed file first.
pub async fn download_image(
    url: &str,
    sha_url: Option<&str>,
    output_dir: &PathBuf,
    state: Arc<DownloadState>,
) -> Result<PathBuf, String> {
    state.reset();
    // Clear any stale temp_path left by a previous failed download.
    *state.temp_path.lock().await = None;

    let filename = extract_filename(url)?;

    let output_filename = filename.trim_end_matches(".xz");
    let output_path = output_dir.join(output_filename);

    log_info!(MODULE, "Download requested: {}", url);
    log_debug!(MODULE, "Output path: {}", output_path.display());

    if let Some(cached_path) = crate::cache::get_cached_image(output_filename) {
        log_info!(MODULE, "Using cached image: {}", cached_path.display());
        *state.output_path.lock().await = Some(cached_path.clone());
        return Ok(cached_path);
    }

    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let client = Client::builder()
        .user_agent(config::app::USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    log_info!(MODULE, "Starting download...");
    let response = client.get(url).send().await.map_err(|e| {
        log_error!(MODULE, "Failed to start download: {}", e);
        format!("Failed to start download: {}", e)
    })?;

    // Post-redirect URL reveals which mirror is being used.
    let final_url = response.url().to_string();
    if final_url != url {
        log_debug!(MODULE, "Redirected to mirror: {}", final_url);
    }

    if !response.status().is_success() {
        log_error!(MODULE, "Download failed with status: {}", response.status());
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);
    state.total_bytes.store(total_size, Ordering::SeqCst);

    log_info!(
        MODULE,
        "Download size: {} bytes ({:.2} MB)",
        total_size,
        bytes_to_mb(total_size)
    );

    let temp_path = output_dir.join(format!("{}{}", filename, config::images::DOWNLOAD_SUFFIX));
    let mut temp_file =
        File::create(&temp_path).map_err(|e| format!("Failed to create temp file: {}", e))?;

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

        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                drop(temp_file);
                let _ = std::fs::remove_file(&temp_path);
                return Err(format!("Download error: {}", e));
            }
        };
        if let Err(e) = temp_file.write_all(&chunk) {
            drop(temp_file);
            let _ = std::fs::remove_file(&temp_path);
            return Err(format!("Failed to write chunk: {}", e));
        }

        downloaded += chunk.len() as u64;
        state.downloaded_bytes.store(downloaded, Ordering::SeqCst);
        tracker.update(chunk.len() as u64);
    }

    drop(temp_file);
    tracker.finish();

    if let Some(sha_url) = sha_url {
        state.is_verifying_sha.store(true, Ordering::SeqCst);
        log_info!(MODULE, "Verifying SHA256...");
        match verify_sha256(&client, &temp_path, sha_url, &state).await {
            Ok(()) => {
                log_info!(MODULE, "SHA256 verification successful");
            }
            Err(e) => {
                log_error!(MODULE, "SHA256 verification failed: {}", e);
                state.is_verifying_sha.store(false, Ordering::SeqCst);

                if state.is_cancelled.load(Ordering::SeqCst) {
                    let _ = std::fs::remove_file(&temp_path);
                    return Err("Download cancelled".to_string());
                }

                // SHA unavailable: keep the file so the user can choose to proceed.
                if e.contains("[SHA_UNAVAILABLE]") {
                    log_info!(
                        MODULE,
                        "SHA unavailable, keeping temp file for user decision: {}",
                        temp_path.display()
                    );
                    *state.temp_path.lock().await = Some(temp_path.clone());
                    return Err(format!("SHA256 verification failed: {}", e));
                }

                // Genuine mismatch means a corrupted image: delete it.
                let _ = std::fs::remove_file(&temp_path);
                return Err(format!("SHA256 verification failed: {}", e));
            }
        }
        state.is_verifying_sha.store(false, Ordering::SeqCst);
    } else {
        log_warn!(MODULE, "No SHA URL provided, skipping verification");
    }

    if filename.ends_with(".xz") {
        state.is_decompressing.store(true, Ordering::SeqCst);
        log_info!(
            MODULE,
            "Starting decompression with Rust lzma-rust2 (multi-threaded)..."
        );

        if let Err(e) = decompress_with_rust_xz(&temp_path, &output_path, &state) {
            let _ = std::fs::remove_file(&temp_path);
            let _ = std::fs::remove_file(&output_path);
            return Err(e);
        }
        log_info!(MODULE, "Decompression complete");

        let _ = std::fs::remove_file(&temp_path);
    } else {
        if let Err(e) = std::fs::rename(&temp_path, &output_path) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(format!("Failed to move file: {}", e));
        }
    }

    log_info!(MODULE, "Image ready: {}", output_path.display());
    *state.output_path.lock().await = Some(output_path.clone());
    Ok(output_path)
}

/// Finish a download without SHA verification, reusing the already-downloaded
/// temp file. Called when the user proceeds after a SHA-unavailable error.
pub async fn continue_without_sha(
    state: Arc<DownloadState>,
    output_dir: &Path,
) -> Result<PathBuf, String> {
    let temp_path = state
        .temp_path
        .lock()
        .await
        .take()
        .ok_or("No pending download to continue")?;

    // Defense in depth: ensure temp_path is still inside the cache directory.
    if let Err(e) = validate_cache_path(&temp_path) {
        log_error!(
            MODULE,
            "Security: temp_path {} is outside cache directory: {}",
            temp_path.display(),
            e
        );
        return Err("Invalid temp file location".to_string());
    }

    log_info!(
        MODULE,
        "Continuing without SHA verification: {}",
        temp_path.display()
    );

    let filename = temp_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid temp path")?;

    // temp_path is "<name>.downloading"; strip that, then the .xz if present.
    let original_filename = filename.trim_end_matches(config::images::DOWNLOAD_SUFFIX);
    let output_filename = original_filename.trim_end_matches(".xz");
    let output_path = output_dir.join(output_filename);

    log_info!(MODULE, "Output path: {}", output_path.display());

    if original_filename.ends_with(".xz") {
        state.is_decompressing.store(true, Ordering::SeqCst);
        log_info!(
            MODULE,
            "Starting decompression with Rust lzma-rust2 (multi-threaded)..."
        );

        if let Err(e) = decompress_with_rust_xz(&temp_path, &output_path, &state) {
            let _ = std::fs::remove_file(&temp_path);
            let _ = std::fs::remove_file(&output_path);
            return Err(e);
        }

        state.is_decompressing.store(false, Ordering::SeqCst);
        log_info!(MODULE, "Decompression complete");

        let _ = std::fs::remove_file(&temp_path);
    } else {
        if let Err(e) = std::fs::rename(&temp_path, &output_path) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(format!("Failed to move file: {}", e));
        }
    }

    log_info!(MODULE, "Image ready: {}", output_path.display());
    *state.output_path.lock().await = Some(output_path.clone());
    Ok(output_path)
}

/// Remove the temp file left by a failed download (user cancelled after SHA-unavailable).
pub async fn cleanup_pending_download(state: Arc<DownloadState>) {
    if let Some(temp_path) = state.temp_path.lock().await.take() {
        log_info!(
            MODULE,
            "Cleaning up pending download: {}",
            temp_path.display()
        );
        let _ = std::fs::remove_file(&temp_path);
    }
}

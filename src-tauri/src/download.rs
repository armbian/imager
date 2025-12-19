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
use crate::decompress::{decompress_with_rust_xz, decompress_with_system_xz};
use crate::{log_error, log_info, log_warn};

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
    let url_path = url.split('?').next().unwrap_or(url);
    url_path
        .split('/')
        .last()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Invalid URL: no filename".to_string())
}

/// Fetch expected SHA256 from URL
async fn fetch_expected_sha(client: &Client, sha_url: &str) -> Result<String, String> {
    log_info!(MODULE, "Fetching SHA256 from: {}", sha_url);

    let response = client
        .get(sha_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch SHA: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("SHA fetch failed with status: {}", response.status()));
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

    log_info!(MODULE, "Expected SHA256: {}", hash);
    Ok(hash)
}

/// Calculate SHA256 of a file
fn calculate_file_sha256(path: &Path, state: &Arc<DownloadState>) -> Result<String, String> {
    log_info!(MODULE, "Calculating SHA256 of: {}", path.display());

    let mut file = File::open(path).map_err(|e| format!("Failed to open file for SHA: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

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
    }

    let result = hasher.finalize();
    let hash = format!("{:x}", result);
    log_info!(MODULE, "Calculated SHA256: {}", hash);
    Ok(hash)
}

/// Verify file SHA256 against expected value
async fn verify_sha256(
    client: &Client,
    file_path: &Path,
    sha_url: &str,
    state: &Arc<DownloadState>,
) -> Result<(), String> {
    // Check cancellation before fetching
    if state.is_cancelled.load(Ordering::SeqCst) {
        return Err("SHA256 verification cancelled".to_string());
    }

    let expected = fetch_expected_sha(client, sha_url).await?;

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
/// If sha_url is provided, verifies the downloaded compressed file before decompression
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
    log_info!(MODULE, "Output path: {}", output_path.display());

    // Check if already exists
    if output_path.exists() {
        log_info!(MODULE, "Image already exists, skipping download");
        *state.output_path.lock().await = Some(output_path.clone());
        return Ok(output_path);
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
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| {
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
        total_size as f64 / 1024.0 / 1024.0
    );

    // Create temp file for compressed data
    let temp_path = output_dir.join(format!("{}.downloading", filename));
    let mut temp_file =
        File::create(&temp_path).map_err(|e| format!("Failed to create temp file: {}", e))?;

    // Download with progress
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

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
    }

    drop(temp_file);
    log_info!(MODULE, "Download complete: {} bytes", downloaded);

    // Verify SHA256 if URL provided
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
        log_info!(MODULE, "Starting decompression...");

        // Try system xz first, fall back to Rust library
        if let Err(e) = decompress_with_system_xz(&temp_path, &output_path, &state) {
            // Check if it was cancelled
            if state.is_cancelled.load(Ordering::SeqCst) {
                let _ = std::fs::remove_file(&temp_path);
                return Err("Decompression cancelled".to_string());
            }
            log_warn!(
                MODULE,
                "System xz failed: {}, falling back to Rust library (slower)",
                e
            );
            decompress_with_rust_xz(&temp_path, &output_path, &state)?;
            log_info!(MODULE, "Rust fallback decompression complete");
        }

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

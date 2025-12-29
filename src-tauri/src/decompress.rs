//! Decompression module
//!
//! Handles decompressing compressed image files (XZ, GZ, BZ2, ZST)
//! using Rust native libraries with multi-threading support.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use lzma_rust2::XzReaderMt;
use zstd::stream::read::Decoder as ZstdDecoder;

use crate::config;
use crate::download::DownloadState;
use crate::log_info;
use crate::utils::get_recommended_threads;

const MODULE: &str = "decompress";

/// Check if a file needs decompression based on extension
pub fn needs_decompression(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext.to_lowercase().as_str(), "xz" | "gz" | "bz2" | "zst")
}

/// Decompress using Rust lzma-rust2 library (multi-threaded)
pub fn decompress_with_rust_xz(
    input_path: &Path,
    output_path: &Path,
    state: &Arc<DownloadState>,
) -> Result<(), String> {
    let input_file =
        File::open(input_path).map_err(|e| format!("Failed to open input file: {}", e))?;
    let threads = get_recommended_threads();

    log_info!(
        MODULE,
        "Using Rust lzma-rust2 with {} threads for XZ decompression",
        threads
    );

    // XzReaderMt requires Seek + Read, so we pass the file directly
    let decoder = XzReaderMt::new(input_file, false, threads as u32)
        .map_err(|e| format!("Failed to create XZ decoder: {}", e))?;

    decompress_with_reader_mt(decoder, output_path, state, "xz")
}

/// Decompress gzip files using flate2 (single-threaded - TODO: add pigz system tool support)
pub fn decompress_with_gz(
    input_path: &Path,
    output_path: &Path,
    state: &Arc<DownloadState>,
) -> Result<(), String> {
    let input_file =
        File::open(input_path).map_err(|e| format!("Failed to open input file: {}", e))?;
    let buf_reader = BufReader::with_capacity(config::download::DECOMPRESS_BUFFER_SIZE, input_file);
    let decoder = GzDecoder::new(buf_reader);
    decompress_with_reader_mt(decoder, output_path, state, "gz")
}

/// Decompress bzip2 files using bzip2 (single-threaded - TODO: add parallel support)
pub fn decompress_with_bz2(
    input_path: &Path,
    output_path: &Path,
    state: &Arc<DownloadState>,
) -> Result<(), String> {
    let input_file =
        File::open(input_path).map_err(|e| format!("Failed to open input file: {}", e))?;
    let buf_reader = BufReader::with_capacity(config::download::DECOMPRESS_BUFFER_SIZE, input_file);
    let decoder = BzDecoder::new(buf_reader);
    decompress_with_reader_mt(decoder, output_path, state, "bz2")
}

/// Decompress zstd files (single-threaded - zstd doesn't have good multithreaded Rust support yet)
pub fn decompress_with_zstd(
    input_path: &Path,
    output_path: &Path,
    state: &Arc<DownloadState>,
) -> Result<(), String> {
    let input_file =
        File::open(input_path).map_err(|e| format!("Failed to open input file: {}", e))?;
    let buf_reader = BufReader::with_capacity(config::download::DECOMPRESS_BUFFER_SIZE, input_file);
    let decoder = ZstdDecoder::new(buf_reader)
        .map_err(|e| format!("Failed to create zstd decoder: {}", e))?;
    decompress_with_reader_mt(decoder, output_path, state, "zstd")
}

/// Generic decompression using any Read implementation (mut reference for multithreaded decoders)
fn decompress_with_reader_mt<R: Read>(
    mut decoder: R,
    output_path: &Path,
    state: &Arc<DownloadState>,
    format_name: &str,
) -> Result<(), String> {
    let output_file =
        File::create(output_path).map_err(|e| format!("Failed to create output file: {}", e))?;

    let mut buf_writer =
        BufWriter::with_capacity(config::download::DECOMPRESS_BUFFER_SIZE, output_file);
    let mut buffer = vec![0u8; config::download::CHUNK_SIZE];

    loop {
        if state.is_cancelled.load(Ordering::SeqCst) {
            drop(buf_writer);
            let _ = std::fs::remove_file(output_path);
            return Err("Decompression cancelled".to_string());
        }

        let bytes_read = decoder
            .read(&mut buffer)
            .map_err(|e| format!("{} decompression error: {}", format_name, e))?;

        if bytes_read == 0 {
            break;
        }

        buf_writer
            .write_all(&buffer[..bytes_read])
            .map_err(|e| format!("Failed to write decompressed data: {}", e))?;
    }

    buf_writer
        .flush()
        .map_err(|e| format!("Failed to flush output: {}", e))?;

    Ok(())
}

/// Decompress a local file (for custom images)
/// Returns the path to the decompressed file
pub fn decompress_local_file(
    input_path: &PathBuf,
    state: &Arc<DownloadState>,
) -> Result<PathBuf, String> {
    let filename = input_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?;

    // Extract base filename (remove compression extension)
    let base_filename = filename
        .trim_end_matches(".xz")
        .trim_end_matches(".gz")
        .trim_end_matches(".bz2")
        .trim_end_matches(".zst");

    // Generate unique filename with timestamp to handle concurrent operations
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Failed to get timestamp: {}", e))?
        .as_millis();

    // Use base_filename directly (it already has the correct .img extension)
    let output_filename = format!("{}-{}", base_filename, timestamp);

    // Output to cache directory instead of user's directory
    let custom_cache_dir = crate::utils::get_cache_dir(config::app::NAME).join("custom-decompress");

    std::fs::create_dir_all(&custom_cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    let output_path = custom_cache_dir.join(&output_filename);

    // Check if already decompressed
    if output_path.exists() {
        log_info!(
            MODULE,
            "Decompressed file already exists: {}",
            output_path.display()
        );
        return Ok(output_path);
    }

    state.is_decompressing.store(true, Ordering::SeqCst);

    // Get input file size for progress indication
    if let Ok(metadata) = std::fs::metadata(input_path) {
        state.total_bytes.store(metadata.len(), Ordering::SeqCst);
    }

    log_info!(
        MODULE,
        "Decompressing custom image: {} -> {}",
        input_path.display(),
        output_path.display()
    );

    // Handle different compression formats
    let result = if filename.ends_with(".xz") {
        // Use Rust lzma-rust2 library (multi-threaded) on all platforms
        log_info!(
            MODULE,
            "Decompressing XZ format with Rust lzma-rust2 (multi-threaded)"
        );
        decompress_with_rust_xz(input_path, &output_path, state)
    } else if filename.ends_with(".gz") {
        log_info!(MODULE, "Decompressing GZ format");
        decompress_with_gz(input_path, &output_path, state)
    } else if filename.ends_with(".bz2") {
        log_info!(MODULE, "Decompressing BZ2 format");
        decompress_with_bz2(input_path, &output_path, state)
    } else if filename.ends_with(".zst") {
        log_info!(MODULE, "Decompressing ZSTD format");
        decompress_with_zstd(input_path, &output_path, state)
    } else {
        return Err(format!("Unsupported compression format for: {}", filename));
    };

    result?;

    state.is_decompressing.store(false, Ordering::SeqCst);
    log_info!(MODULE, "Decompression complete: {}", output_path.display());

    Ok(output_path)
}

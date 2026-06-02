//! Decompressing image files (XZ, GZ, BZ2, ZST) using native Rust libraries,
//! with multi-threading for XZ.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use lzma_rust2::XzReaderMt;
use xz2::read::XzDecoder;
use zstd::stream::read::Decoder as ZstdDecoder;

use crate::config;
use crate::download::DownloadState;
use crate::log_info;
use crate::utils::{get_recommended_threads, strip_compression_ext, ProgressTracker};

const MODULE: &str = "decompress";

/// Check if a file needs decompression based on extension
pub fn needs_decompression(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext.to_lowercase().as_str(), "xz" | "gz" | "bz2" | "zst")
}

/// Decompress XZ files. Uses multi-threaded lzma-rust2 for single-stream files,
/// falls back to xz2 (liblzma) for multi-stream files (e.g., Khadas OOWOW).
pub fn decompress_with_rust_xz(
    input_path: &Path,
    output_path: &Path,
    state: &Arc<DownloadState>,
) -> Result<(), String> {
    // Multi-threaded decoder is faster but can't handle multi-stream XZ.
    let threads = get_recommended_threads();
    let input_file =
        File::open(input_path).map_err(|e| format!("Failed to open input file: {}", e))?;

    match XzReaderMt::new(input_file, true, threads as u32) {
        Ok(decoder) => {
            log_info!(
                MODULE,
                "Using multi-threaded XZ decoder with {} threads",
                threads
            );
            decompress_with_reader_mt(decoder, output_path, state, "xz")
        }
        Err(mt_err) => {
            // Fall back to xz2 (liblzma), which handles multi-stream XZ natively.
            log_info!(
                MODULE,
                "Multi-threaded decoder failed ({}), using liblzma multi-stream decoder",
                mt_err
            );
            let input_file =
                File::open(input_path).map_err(|e| format!("Failed to open input file: {}", e))?;
            let buf_reader =
                BufReader::with_capacity(config::download::DECOMPRESS_BUFFER_SIZE, input_file);
            let decoder = XzDecoder::new_multi_decoder(buf_reader);
            decompress_with_reader_mt(decoder, output_path, state, "xz")
        }
    }
}

/// Decompress gzip files using flate2 (single-threaded)
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

/// Decompress bzip2 files (single-threaded)
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

/// Decompress zstd files (single-threaded)
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

/// Generic decompression over any Read. Takes the decoder by value to support
/// multi-threaded readers.
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

    // Decompressed size is unknown, so track output bytes against a total of 0.
    let operation_name = format!("Decompress ({})", format_name);
    let mut tracker = ProgressTracker::new(
        &operation_name,
        MODULE,
        0,
        config::logging::DECOMPRESS_LOG_INTERVAL_MB,
    );

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

        tracker.update(bytes_read as u64);
    }

    buf_writer
        .flush()
        .map_err(|e| format!("Failed to flush output: {}", e))?;

    tracker.finish();

    Ok(())
}

/// Decompress a local custom-image file, returning the decompressed path.
pub fn decompress_local_file(
    input_path: &PathBuf,
    state: &Arc<DownloadState>,
) -> Result<PathBuf, String> {
    let filename = input_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?;

    let base_filename = strip_compression_ext(filename);

    // Timestamp suffix keeps concurrent decompressions from colliding.
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Failed to get timestamp: {}", e))?
        .as_millis();

    let output_filename = format!("{}-{}", base_filename, timestamp);

    // Decompress into the cache dir, not the user's directory.
    let custom_cache_dir = crate::utils::custom_decompress_dir();

    std::fs::create_dir_all(&custom_cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    let output_path = custom_cache_dir.join(&output_filename);

    if output_path.exists() {
        log_info!(
            MODULE,
            "Decompressed file already exists: {}",
            output_path.display()
        );
        return Ok(output_path);
    }

    state.is_decompressing.store(true, Ordering::SeqCst);

    if let Ok(metadata) = std::fs::metadata(input_path) {
        state.total_bytes.store(metadata.len(), Ordering::SeqCst);
    }

    log_info!(
        MODULE,
        "Decompressing custom image: {} -> {}",
        input_path.display(),
        output_path.display()
    );

    let result = if filename.ends_with(".xz") {
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

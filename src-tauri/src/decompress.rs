//! Decompression module
//!
//! Handles decompressing compressed image files (XZ, GZ, BZ2, ZST)
//! using system tools or fallback Rust libraries.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use liblzma::read::XzDecoder;
use zstd::stream::read::Decoder as ZstdDecoder;

use crate::config;
use crate::download::DownloadState;
use crate::utils::{find_binary, get_recommended_threads};
use crate::{log_error, log_info, log_warn};

const MODULE: &str = "decompress";

/// Check if a file needs decompression based on extension
pub fn needs_decompression(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext.to_lowercase().as_str(), "xz" | "gz" | "bz2" | "zst")
}

/// Decompress using system xz command (much faster, uses multiple threads)
pub fn decompress_with_system_xz(
    input_path: &Path,
    output_path: &Path,
    state: &Arc<DownloadState>,
) -> Result<(), String> {
    use std::io::Read as IoRead;
    use std::process::Stdio;

    let xz_path = find_binary("xz").ok_or("xz command not found")?;
    let threads = get_recommended_threads();

    log_info!(
        MODULE,
        "Using system xz at: {} with {} threads",
        xz_path.display(),
        threads
    );

    let mut child = Command::new(&xz_path)
        .args(["-d", "-k", "-c"]) // decompress, keep original, output to stdout
        .arg(format!("-T{}", threads))
        .arg(input_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            log_error!(MODULE, "Failed to spawn xz: {}", e);
            format!("Failed to spawn xz: {}", e)
        })?;

    let mut stdout = child.stdout.take().ok_or("Failed to capture xz stdout")?;

    let mut output_file =
        File::create(output_path).map_err(|e| format!("Failed to create output file: {}", e))?;

    // Read in chunks to allow cancellation checks
    let mut buffer = vec![0u8; config::download::CHUNK_SIZE];
    loop {
        // Check for cancellation
        if state.is_cancelled.load(Ordering::SeqCst) {
            log_info!(MODULE, "Decompression cancelled by user, killing xz process");
            let _ = child.kill();
            let _ = child.wait();
            drop(output_file);
            let _ = std::fs::remove_file(output_path);
            return Err("Decompression cancelled".to_string());
        }

        let bytes_read = stdout
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read from xz: {}", e))?;

        if bytes_read == 0 {
            break;
        }

        output_file
            .write_all(&buffer[..bytes_read])
            .map_err(|e| format!("Failed to write decompressed data: {}", e))?;
    }

    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for xz: {}", e))?;

    if status.success() {
        log_info!(MODULE, "System xz decompression complete");
        Ok(())
    } else {
        let _ = std::fs::remove_file(output_path);
        log_error!(MODULE, "xz decompression failed");
        Err("xz decompression failed".to_string())
    }
}

/// Decompress using Rust xz2 library (slower, single-threaded fallback)
pub fn decompress_with_rust_xz(
    input_path: &Path,
    output_path: &Path,
    state: &Arc<DownloadState>,
) -> Result<(), String> {
    let input_file =
        File::open(input_path).map_err(|e| format!("Failed to open input file: {}", e))?;
    let buf_reader = BufReader::with_capacity(config::download::DECOMPRESS_BUFFER_SIZE, input_file);
    let decoder = XzDecoder::new(buf_reader);
    decompress_with_reader(decoder, output_path, state, "xz")
}

/// Decompress gzip files using flate2
pub fn decompress_with_gz(
    input_path: &Path,
    output_path: &Path,
    state: &Arc<DownloadState>,
) -> Result<(), String> {
    let input_file =
        File::open(input_path).map_err(|e| format!("Failed to open input file: {}", e))?;
    let buf_reader = BufReader::with_capacity(config::download::DECOMPRESS_BUFFER_SIZE, input_file);
    let decoder = GzDecoder::new(buf_reader);
    decompress_with_reader(decoder, output_path, state, "gz")
}

/// Decompress bzip2 files
pub fn decompress_with_bz2(
    input_path: &Path,
    output_path: &Path,
    state: &Arc<DownloadState>,
) -> Result<(), String> {
    let input_file =
        File::open(input_path).map_err(|e| format!("Failed to open input file: {}", e))?;
    let buf_reader = BufReader::with_capacity(config::download::DECOMPRESS_BUFFER_SIZE, input_file);
    let decoder = BzDecoder::new(buf_reader);
    decompress_with_reader(decoder, output_path, state, "bz2")
}

/// Decompress zstd files
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
    decompress_with_reader(decoder, output_path, state, "zstd")
}

/// Generic decompression using any Read implementation
fn decompress_with_reader<R: Read>(
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

    // Determine output filename (remove compression extension)
    let output_filename = filename
        .trim_end_matches(".xz")
        .trim_end_matches(".gz")
        .trim_end_matches(".bz2")
        .trim_end_matches(".zst");

    // Output to same directory as input
    let output_path = input_path
        .parent()
        .ok_or("Invalid input path")?
        .join(output_filename);

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
        // Try system xz first (faster, multi-threaded), fall back to Rust library
        log_info!(MODULE, "Decompressing XZ format");
        if let Err(e) = decompress_with_system_xz(input_path, &output_path, state) {
            if state.is_cancelled.load(Ordering::SeqCst) {
                return Err("Decompression cancelled".to_string());
            }
            log_warn!(
                MODULE,
                "System xz failed: {}, falling back to Rust library (slower)",
                e
            );
            decompress_with_rust_xz(input_path, &output_path, state)
        } else {
            Ok(())
        }
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
        return Err(format!(
            "Unsupported compression format for: {}",
            filename
        ));
    };

    result?;

    state.is_decompressing.store(false, Ordering::SeqCst);
    log_info!(MODULE, "Decompression complete: {}", output_path.display());

    Ok(output_path)
}

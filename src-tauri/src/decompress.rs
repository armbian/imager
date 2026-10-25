// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

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
use crate::utils::{
    discard_if_empty, ensure_regular_nonempty, get_recommended_threads, strip_compression_ext,
    tagged, ProgressTracker, TAG_CANCELLED,
};
use crate::{log_info, log_warn};

const MODULE: &str = "decompress";

pub(crate) const TAG_DECOMPRESS_FAILED: &str = "[DECOMPRESS_FAILED]";
const DECOMPRESS_CANCELLED: &str = "Decompression cancelled";
const OPEN_INPUT_FAILED: &str = "Failed to open input file";

fn decompress_err(what: &str, e: impl std::fmt::Display) -> String {
    tagged(TAG_DECOMPRESS_FAILED, format!("{}: {}", what, e))
}

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
    let input_file = File::open(input_path).map_err(|e| decompress_err(OPEN_INPUT_FAILED, e))?;

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
                File::open(input_path).map_err(|e| decompress_err(OPEN_INPUT_FAILED, e))?;
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
    let input_file = File::open(input_path).map_err(|e| decompress_err(OPEN_INPUT_FAILED, e))?;
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
    let input_file = File::open(input_path).map_err(|e| decompress_err(OPEN_INPUT_FAILED, e))?;
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
    let input_file = File::open(input_path).map_err(|e| decompress_err(OPEN_INPUT_FAILED, e))?;
    let buf_reader = BufReader::with_capacity(config::download::DECOMPRESS_BUFFER_SIZE, input_file);
    let decoder = ZstdDecoder::new(buf_reader)
        .map_err(|e| decompress_err("Failed to create zstd decoder", e))?;
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
        File::create(output_path).map_err(|e| decompress_err("Failed to create output file", e))?;

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
            return Err(tagged(TAG_CANCELLED, DECOMPRESS_CANCELLED));
        }

        let bytes_read = decoder
            .read(&mut buffer)
            .map_err(|e| decompress_err(&format!("{} decompression error", format_name), e))?;

        if bytes_read == 0 {
            break;
        }

        buf_writer
            .write_all(&buffer[..bytes_read])
            .map_err(|e| decompress_err("Failed to write decompressed data", e))?;

        tracker.update(bytes_read as u64);
    }

    buf_writer
        .flush()
        .map_err(|e| decompress_err("Failed to flush output", e))?;

    tracker.finish();

    Ok(())
}

/// Decompress a local custom-image file, returning the decompressed path.
pub fn decompress_local_file(
    input_path: &Path,
    state: &Arc<DownloadState>,
) -> Result<PathBuf, String> {
    // Decompress into the cache dir, not the user's directory.
    decompress_local_file_in(input_path, &crate::utils::custom_decompress_dir(), state)
}

fn decompress_local_file_in(
    input_path: &Path,
    out_dir: &Path,
    state: &Arc<DownloadState>,
) -> Result<PathBuf, String> {
    let filename = input_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?;

    let input_len = ensure_regular_nonempty(input_path)?;

    let base_filename = strip_compression_ext(filename);

    // Timestamp suffix keeps concurrent decompressions from colliding.
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Failed to get timestamp: {}", e))?
        .as_millis();

    let output_filename = format!("{}-{}", base_filename, timestamp);

    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    let output_path = out_dir.join(&output_filename);

    if output_path.exists() && discard_if_empty(&output_path).is_ok() {
        log_info!(
            MODULE,
            "Decompressed file already exists: {}",
            output_path.display()
        );
        return Ok(output_path);
    }

    state.is_decompressing.store(true, Ordering::SeqCst);
    state.total_bytes.store(input_len, Ordering::SeqCst);

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
        Err(format!("Unsupported compression format for: {}", filename))
    };

    state.is_decompressing.store(false, Ordering::SeqCst);

    if let Err(e) = result {
        if output_path.is_file() {
            match std::fs::remove_file(&output_path) {
                Ok(()) => log_warn!(MODULE, "Removed partial output {}", output_path.display()),
                Err(rm) => log_warn!(
                    MODULE,
                    "Failed to remove partial output {}: {}",
                    output_path.display(),
                    rm
                ),
            }
        }
        return Err(e);
    }

    let output_len = discard_if_empty(&output_path)?;
    log_info!(
        MODULE,
        "Decompression complete: {} ({} bytes)",
        output_path.display(),
        output_len
    );

    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> Arc<DownloadState> {
        Arc::new(DownloadState::new())
    }

    #[test]
    fn missing_input_is_tagged() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("out.img");
        let err = decompress_with_gz(&tmp.path().join("missing.gz"), &out, &state()).unwrap_err();
        assert!(
            err.starts_with(&format!("{TAG_DECOMPRESS_FAILED} {OPEN_INPUT_FAILED}: ")),
            "{err}"
        );
    }

    #[test]
    fn corrupt_archive_is_tagged() {
        let tmp = tempfile::tempdir().unwrap();
        let input = tmp.path().join("bad.gz");
        std::fs::write(&input, b"not a gzip stream at all").unwrap();
        let err = decompress_with_gz(&input, &tmp.path().join("out.img"), &state()).unwrap_err();
        assert!(
            err.starts_with("[DECOMPRESS_FAILED] gz decompression error: "),
            "{err}"
        );
    }

    #[test]
    fn cancel_is_tagged_and_still_says_cancelled() {
        let tmp = tempfile::tempdir().unwrap();
        let input = tmp.path().join("bad.gz");
        std::fs::write(&input, b"x").unwrap();
        let st = state();
        st.is_cancelled.store(true, Ordering::SeqCst);
        let err = decompress_with_gz(&input, &tmp.path().join("out.img"), &st).unwrap_err();
        assert_eq!(err, "[CANCELLED] Decompression cancelled");
    }

    fn entries(dir: &Path) -> usize {
        std::fs::read_dir(dir).map_or(0, |d| d.count())
    }

    #[test]
    fn a_valid_archive_of_nothing_is_refused_and_removed() {
        use flate2::{write::GzEncoder, Compression};
        let tmp = tempfile::tempdir().unwrap();
        let input = tmp.path().join("empty.img.gz");
        let gz = GzEncoder::new(Vec::new(), Compression::default())
            .finish()
            .unwrap();
        std::fs::write(&input, gz).unwrap();
        let out = tmp.path().join("out");

        let err = decompress_local_file_in(&input, &out, &state()).unwrap_err();

        assert!(err.starts_with(crate::utils::TAG_IMAGE_INVALID), "{err}");
        assert_eq!(entries(&out), 0, "the empty output must be deleted");
    }

    #[test]
    fn an_empty_input_is_refused_before_any_output() {
        let tmp = tempfile::tempdir().unwrap();
        let input = tmp.path().join("empty.img.xz");
        std::fs::write(&input, b"").unwrap();
        let out = tmp.path().join("out");

        let err = decompress_local_file_in(&input, &out, &state()).unwrap_err();

        assert!(err.starts_with(crate::utils::TAG_IMAGE_INVALID), "{err}");
        assert!(!out.exists());
    }

    #[test]
    fn a_failed_decompression_leaves_no_output() {
        let tmp = tempfile::tempdir().unwrap();
        let input = tmp.path().join("bad.img.gz");
        std::fs::write(&input, b"not a gzip stream at all").unwrap();
        let out = tmp.path().join("out");
        let st = state();

        let err = decompress_local_file_in(&input, &out, &st).unwrap_err();

        assert!(err.starts_with(TAG_DECOMPRESS_FAILED), "{err}");
        assert_eq!(entries(&out), 0);
        assert!(!st.is_decompressing.load(Ordering::SeqCst));
    }

    #[test]
    fn a_real_archive_still_decompresses() {
        use flate2::{write::GzEncoder, Compression};
        use std::io::Write as _;
        let tmp = tempfile::tempdir().unwrap();
        let input = tmp.path().join("a.img.gz");
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&[5u8; 4096]).unwrap();
        std::fs::write(&input, enc.finish().unwrap()).unwrap();
        let out = tmp.path().join("out");

        let path = decompress_local_file_in(&input, &out, &state()).unwrap();

        assert_eq!(std::fs::read(path).unwrap(), vec![5u8; 4096]);
    }
}

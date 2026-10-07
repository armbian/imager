// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

//! Shared verification logic for all platforms.

#![allow(dead_code)]

use crate::config;
use crate::utils::{bytes_to_gb, tagged, ProgressTracker, TAG_CANCELLED};
use crate::{log_error, log_info};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::FlashState;

const MODULE: &str = "flash::verify";

pub(crate) const TAG_VERIFY_MISMATCH: &str = "[VERIFY_MISMATCH]";
pub(crate) const TAG_VERIFY_READ_FAILED: &str = "[VERIFY_READ_FAILED]";
pub(crate) const VERIFY_CANCELLED: &str = "Verification cancelled";

pub(crate) fn verify_cancelled_err() -> String {
    tagged(TAG_CANCELLED, VERIFY_CANCELLED)
}

pub(crate) fn data_mismatch_err(offset: u64) -> String {
    tagged(
        TAG_VERIFY_MISMATCH,
        format!("Verification failed: data mismatch at byte {}", offset),
    )
}

/// Verification reader trait for platform-specific device reading
pub trait VerificationReader: Read + Send {}

impl<T: Read + Send> VerificationReader for T {}

/// Verify written data by comparing the image against a device reader.
/// Platform-agnostic; the caller supplies the device `Read`er.
pub fn verify_data<R: Read>(
    image_path: &PathBuf,
    device_reader: &mut R,
    state: Arc<FlashState>,
) -> Result<(), String> {
    state.is_verifying.store(true, Ordering::SeqCst);
    state.verified_bytes.store(0, Ordering::SeqCst);

    let mut image_file = File::open(image_path).map_err(|e| {
        tagged(
            TAG_VERIFY_READ_FAILED,
            format!("Failed to open image for verification: {}", e),
        )
    })?;

    let chunk_size = config::flash::CHUNK_SIZE;
    let mut image_buffer = vec![0u8; chunk_size];
    let mut device_buffer = vec![0u8; chunk_size];
    let mut verified: u64 = 0;

    let image_size = state.total_bytes.load(Ordering::SeqCst);

    let mut tracker = ProgressTracker::new(
        "Verify",
        MODULE,
        image_size,
        config::logging::WRITE_LOG_INTERVAL_MB,
    );

    log_info!(
        MODULE,
        "Starting verification of {} bytes ({:.2} GB)",
        image_size,
        bytes_to_gb(image_size)
    );

    while verified < image_size {
        if state.is_cancelled.load(Ordering::SeqCst) {
            return Err(verify_cancelled_err());
        }

        let to_read = std::cmp::min(chunk_size as u64, image_size - verified) as usize;

        let image_read = image_file.read(&mut image_buffer[..to_read]).map_err(|e| {
            tagged(
                TAG_VERIFY_READ_FAILED,
                format!("Failed to read image: {}", e),
            )
        })?;

        if image_read == 0 {
            break;
        }

        // Read the matching byte count back from the device.
        let mut device_read = 0;
        while device_read < image_read {
            let n = device_reader
                .read(&mut device_buffer[device_read..image_read])
                .map_err(|e| {
                    tagged(
                        TAG_VERIFY_READ_FAILED,
                        format!("Failed to read device: {}", e),
                    )
                })?;
            if n == 0 {
                break;
            }
            device_read += n;
        }

        if device_read != image_read {
            log_error!(
                MODULE,
                "Verification failed: size mismatch at byte {} (expected {}, got {})",
                verified,
                image_read,
                device_read
            );
            return Err(tagged(
                TAG_VERIFY_MISMATCH,
                format!(
                    "Verification failed: size mismatch at byte {} (expected {}, got {})",
                    verified, image_read, device_read
                ),
            ));
        }

        if image_buffer[..image_read] != device_buffer[..device_read] {
            log_error!(
                MODULE,
                "Verification failed: data mismatch at byte {}",
                verified
            );
            return Err(data_mismatch_err(verified));
        }

        verified += image_read as u64;
        state.verified_bytes.store(verified, Ordering::SeqCst);

        tracker.update(image_read as u64);
    }

    tracker.finish();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Error};

    fn run(image: &[u8], device: &mut impl Read, cancelled: bool) -> Result<(), String> {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("image.img");
        std::fs::write(&path, image).unwrap();
        let state = Arc::new(FlashState::new());
        state
            .total_bytes
            .store(image.len() as u64, Ordering::SeqCst);
        state.is_cancelled.store(cancelled, Ordering::SeqCst);
        verify_data(&path, device, state)
    }

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(Error::other("boom"))
        }
    }

    #[test]
    fn matching_data_passes() {
        assert!(run(&[7u8; 1024], &mut Cursor::new(vec![7u8; 1024]), false).is_ok());
    }

    #[test]
    fn data_mismatch_is_tagged() {
        let err = run(&[7u8; 1024], &mut Cursor::new(vec![8u8; 1024]), false).unwrap_err();
        assert_eq!(
            err,
            "[VERIFY_MISMATCH] Verification failed: data mismatch at byte 0"
        );
    }

    #[test]
    fn short_device_is_tagged_size_mismatch() {
        let err = run(&[7u8; 1024], &mut Cursor::new(vec![7u8; 10]), false).unwrap_err();
        assert!(err.starts_with(TAG_VERIFY_MISMATCH), "{err}");
        assert!(err.contains("size mismatch at byte 0 (expected 1024, got 10)"));
    }

    #[test]
    fn device_read_failure_is_tagged() {
        let err = run(&[7u8; 1024], &mut FailingReader, false).unwrap_err();
        assert_eq!(err, "[VERIFY_READ_FAILED] Failed to read device: boom");
    }

    #[test]
    fn missing_image_is_tagged_read_failed() {
        let state = Arc::new(FlashState::new());
        let err = verify_data(
            &PathBuf::from("/nonexistent/armbian-imager/image.img"),
            &mut Cursor::new(Vec::new()),
            state,
        )
        .unwrap_err();
        assert!(err.starts_with(TAG_VERIFY_READ_FAILED), "{err}");
    }

    #[test]
    fn cancel_is_tagged_and_still_says_cancelled() {
        let err = run(&[7u8; 1024], &mut Cursor::new(vec![7u8; 1024]), true).unwrap_err();
        assert_eq!(err, "[CANCELLED] Verification cancelled");
    }
}

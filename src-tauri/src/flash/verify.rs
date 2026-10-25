// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

//! Shared verification logic for all platforms.

#![allow(dead_code)]

use crate::config;
use crate::utils::{bytes_to_gb, tagged, ProgressTracker, TAG_CANCELLED, TAG_IMAGE_INVALID};
use crate::{log_error, log_info};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
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

/// The device returned fewer bytes than the image chunk it is compared with.
pub(crate) fn short_read_err(offset: u64, expected: usize, got: usize) -> String {
    let e = tagged(
        TAG_VERIFY_MISMATCH,
        format!(
            "Verification failed: size mismatch at byte {} (expected {}, got {})",
            offset, expected, got
        ),
    );
    log_error!(MODULE, "{}", e);
    e
}

/// Verification reader trait for platform-specific device reading
pub trait VerificationReader: Read + Send {}

impl<T: Read + Send> VerificationReader for T {}

/// Seek the image handle the writer used back to its start for the read-back.
pub(crate) fn rewind_image(image_file: &mut File) -> Result<(), String> {
    image_file
        .seek(SeekFrom::Start(0))
        .map(|_| ())
        .map_err(|e| {
            tagged(
                TAG_VERIFY_READ_FAILED,
                format!("Failed to rewind image for verification: {}", e),
            )
        })
}

/// Refuse a verification with nothing to compare, or one where the image ended early.
pub(crate) fn check_verified_length(verified: u64, image_size: u64) -> Result<(), String> {
    let e = if image_size == 0 {
        tagged(TAG_IMAGE_INVALID, "nothing to verify: image length is 0")
    } else if verified < image_size {
        super::image_length_changed_err(verified, image_size)
    } else {
        return Ok(());
    };
    log_error!(MODULE, "Verification failed: {}", e);
    Err(e)
}

/// Verify written data by comparing the image, read from its start, against a device reader.
pub fn verify_data<I: Read, R: Read>(
    image_file: &mut I,
    device_reader: &mut R,
    state: Arc<FlashState>,
) -> Result<(), String> {
    state.is_verifying.store(true, Ordering::SeqCst);
    state.verified_bytes.store(0, Ordering::SeqCst);

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
            return Err(short_read_err(verified, image_read, device_read));
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

    check_verified_length(verified, image_size)?;
    tracker.finish();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Error};

    fn run_with_total(
        image: &[u8],
        total: u64,
        device: &mut impl Read,
        cancelled: bool,
    ) -> Result<(), String> {
        let state = Arc::new(FlashState::new());
        state.total_bytes.store(total, Ordering::SeqCst);
        state.is_cancelled.store(cancelled, Ordering::SeqCst);
        verify_data(&mut Cursor::new(image.to_vec()), device, state)
    }

    fn run(image: &[u8], device: &mut impl Read, cancelled: bool) -> Result<(), String> {
        run_with_total(image, image.len() as u64, device, cancelled)
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
    fn image_read_failure_is_tagged() {
        let state = Arc::new(FlashState::new());
        state.total_bytes.store(1024, Ordering::SeqCst);
        let err =
            verify_data(&mut FailingReader, &mut Cursor::new(vec![7u8; 1024]), state).unwrap_err();
        assert_eq!(err, "[VERIFY_READ_FAILED] Failed to read image: boom");
    }

    #[test]
    fn an_image_shorter_than_the_written_length_fails() {
        let err = run_with_total(&[7u8; 1024], 4096, &mut Cursor::new(vec![7u8; 4096]), false)
            .unwrap_err();
        assert!(err.starts_with(TAG_IMAGE_INVALID), "{err}");
        assert!(err.contains("delivered 1024 bytes, expected 4096"), "{err}");
    }

    #[test]
    fn nothing_to_verify_is_refused() {
        let err = run_with_total(&[], 0, &mut Cursor::new(Vec::new()), false).unwrap_err();
        assert!(err.starts_with(TAG_IMAGE_INVALID), "{err}");
    }

    #[test]
    fn cancel_is_tagged_and_still_says_cancelled() {
        let err = run(&[7u8; 1024], &mut Cursor::new(vec![7u8; 1024]), true).unwrap_err();
        assert_eq!(err, "[CANCELLED] Verification cancelled");
    }
}

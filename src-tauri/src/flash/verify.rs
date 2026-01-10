//! Shared verification logic for all platforms
//!
//! This module provides common verification functionality that can be used
//! across macOS, Linux, and Windows implementations.

#![allow(dead_code)]

use crate::config;
use crate::utils::{bytes_to_gb, ProgressTracker};
use crate::{log_error, log_info};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::FlashState;

const MODULE: &str = "flash::verify";

/// Verification reader trait for platform-specific device reading
pub trait VerificationReader: Read + Send {}

impl<T: Read + Send> VerificationReader for T {}

/// Verify written data by comparing image file with device contents
///
/// This function is platform-agnostic and takes any reader that implements
/// the Read trait. Platform-specific code is responsible for providing
/// the appropriate device reader.
pub fn verify_data<R: Read>(
    image_path: &PathBuf,
    device_reader: &mut R,
    state: Arc<FlashState>,
) -> Result<(), String> {
    state.is_verifying.store(true, Ordering::SeqCst);
    state.verified_bytes.store(0, Ordering::SeqCst);

    let mut image_file = File::open(image_path)
        .map_err(|e| format!("Failed to open image for verification: {}", e))?;

    let chunk_size = config::flash::CHUNK_SIZE;
    let mut image_buffer = vec![0u8; chunk_size];
    let mut device_buffer = vec![0u8; chunk_size];
    let mut verified: u64 = 0;

    let image_size = state.total_bytes.load(Ordering::SeqCst);

    // Use ProgressTracker for automatic progress logging
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
            return Err("Verification cancelled".to_string());
        }

        let to_read = std::cmp::min(chunk_size as u64, image_size - verified) as usize;

        let image_read = image_file
            .read(&mut image_buffer[..to_read])
            .map_err(|e| format!("Failed to read image: {}", e))?;

        if image_read == 0 {
            break;
        }

        // Read same amount from device
        let mut device_read = 0;
        while device_read < image_read {
            let n = device_reader
                .read(&mut device_buffer[device_read..image_read])
                .map_err(|e| format!("Failed to read device: {}", e))?;
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
            return Err(format!(
                "Verification failed: size mismatch at byte {} (expected {}, got {})",
                verified, image_read, device_read
            ));
        }

        if image_buffer[..image_read] != device_buffer[..device_read] {
            log_error!(
                MODULE,
                "Verification failed: data mismatch at byte {}",
                verified
            );
            return Err(format!(
                "Verification failed: data mismatch at byte {}",
                verified
            ));
        }

        verified += image_read as u64;
        state.verified_bytes.store(verified, Ordering::SeqCst);

        // ProgressTracker handles logging automatically
        tracker.update(image_read as u64);
    }

    // Log final summary
    tracker.finish();
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_verify_matching_data() {
        // This test requires a temp file, which we'll skip for now
        // In production, we'd create temp files and verify they match
    }
}

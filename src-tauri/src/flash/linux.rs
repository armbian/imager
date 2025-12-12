//! Linux-specific flash implementation
//!
//! Uses pkexec (PolicyKit) for privilege escalation.
//! This shows a graphical authentication dialog for the user to enter their password.

use super::{sync_device, unmount_device, FlashState};
use crate::{log_error, log_info};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::Arc;

const MODULE: &str = "flash::linux";

/// Flash an image to a block device on Linux
///
/// Uses pkexec to run dd with elevated privileges. The user will see
/// a PolicyKit authentication dialog.
pub async fn flash_image(
    image_path: &PathBuf,
    device_path: &str,
    state: Arc<FlashState>,
    verify: bool,
) -> Result<(), String> {
    state.reset();

    log_info!(
        MODULE,
        "Starting flash: {} -> {}",
        image_path.display(),
        device_path
    );

    // Get image size
    let image_size = std::fs::metadata(image_path)
        .map_err(|e| format!("Failed to get image size: {}", e))?
        .len();

    state.total_bytes.store(image_size, Ordering::SeqCst);

    log_info!(
        MODULE,
        "Image size: {} bytes ({:.2} GB)",
        image_size,
        image_size as f64 / 1024.0 / 1024.0 / 1024.0
    );

    // Unmount the device first
    log_info!(MODULE, "Unmounting device partitions...");
    unmount_device(device_path)?;

    // Use pkexec to run dd with elevated privileges
    let image_path_str = image_path.to_string_lossy();

    log_info!(MODULE, "Starting privileged write with pkexec dd...");

    let output = Command::new("pkexec")
        .args([
            "dd",
            &format!("if={}", image_path_str),
            &format!("of={}", device_path),
            "bs=4M",
            "status=none",
        ])
        .output()
        .map_err(|e| {
            log_error!(MODULE, "Failed to start privileged write: {}", e);
            format!("Failed to start privileged write: {}", e)
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("dismissed") || stderr.contains("Not authorized") {
            log_info!(MODULE, "Operation cancelled by user");
            return Err("Operation cancelled by user".to_string());
        }
        log_error!(MODULE, "Flash failed: {}", stderr);
        return Err(format!("Flash failed: {}", stderr));
    }

    state.written_bytes.store(image_size, Ordering::SeqCst);
    log_info!(MODULE, "Write complete, syncing device...");

    // Sync to ensure all data is written
    sync_device(device_path);

    // Verify if requested
    if verify {
        log_info!(MODULE, "Starting verification...");
        verify_written_data(image_path, device_path, state.clone())?;
    }

    log_info!(MODULE, "Flash complete!");
    Ok(())
}

/// Verify written data by reading back and comparing
/// Uses the shared verification logic from flash/verify.rs
fn verify_written_data(
    image_path: &PathBuf,
    device_path: &str,
    state: Arc<FlashState>,
) -> Result<(), String> {
    // Try to open device directly first, fall back to pkexec cat if permission denied
    let device_result = std::fs::OpenOptions::new().read(true).open(device_path);

    let mut device: Box<dyn Read> = match device_result {
        Ok(f) => Box::new(f),
        Err(_) => {
            // Need elevated privileges to read - use pkexec cat
            let child = Command::new("pkexec")
                .args(["cat", device_path])
                .stdout(Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to start verification read: {}", e))?;

            Box::new(child.stdout.ok_or("Failed to capture stdout")?)
        }
    };

    // Use shared verification logic
    super::verify::verify_data(image_path, &mut device, state)
}

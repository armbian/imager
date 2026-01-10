//! Linux device writer
//!
//! Writes images to block devices using UDisks2 for privilege escalation.
//! UDisks2 handles authentication via polkit, so the app can run as a normal user.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::config;
use crate::flash::{sync_device, unmount_device, FlashState};
use crate::utils::{bytes_to_gb, ProgressTracker};
use crate::{log_debug, log_error, log_info};

const MODULE: &str = "flash::linux::writer";

/// Open a block device for writing using UDisks2
/// This will trigger a polkit authentication dialog if needed
async fn open_device_udisks2(device_path: &str) -> Result<File, String> {
    use std::collections::HashMap;

    log_debug!(MODULE, "Opening device via UDisks2: {}", device_path);

    // Create UDisks2 client
    let client = udisks2::Client::new()
        .await
        .map_err(|e| format!("Failed to connect to UDisks2: {}", e))?;

    // Convert /dev/sdX to UDisks2 object path: /org/freedesktop/UDisks2/block_devices/sdX
    let dev_name = device_path
        .strip_prefix("/dev/")
        .ok_or_else(|| format!("Invalid device path: {}", device_path))?;

    let object_path = format!("/org/freedesktop/UDisks2/block_devices/{}", dev_name);

    log_debug!(MODULE, "UDisks2 object path: {}", object_path);

    // Get the block device object
    let object = client
        .object(object_path.as_str())
        .map_err(|e| format!("Device not found in UDisks2: {} ({})", device_path, e))?;

    let block = object
        .block()
        .await
        .map_err(|e| format!("Failed to get block interface: {}", e))?;

    // Open device for read-write with exclusive access
    // Options: empty HashMap for default options
    let options: HashMap<&str, udisks2::zbus::zvariant::Value<'_>> = HashMap::new();

    let fd = block
        .open_device("rw", options)
        .await
        .map_err(|e| format!("Failed to open device (polkit auth may have failed): {}", e))?;

    log_debug!(MODULE, "Device opened successfully via UDisks2");

    // Convert the file descriptor to a File
    // OwnedFd implements AsRawFd, so we get the raw fd and create a File from it
    let raw_fd = fd.as_raw_fd();
    let file = unsafe { File::from_raw_fd(raw_fd) };
    // Note: fd is consumed here since we take ownership via from_raw_fd
    std::mem::forget(fd);

    Ok(file)
}

/// Fallback: try to open device directly (requires root)
fn open_device_direct(device_path: &str) -> Result<File, String> {
    use std::fs::OpenOptions;

    log_debug!(MODULE, "Attempting direct device open: {}", device_path);

    OpenOptions::new()
        .read(true)
        .write(true)
        .open(device_path)
        .map_err(|e| format!("Failed to open device {}: {}", device_path, e))
}

/// Flash an image to a block device
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
        bytes_to_gb(image_size)
    );

    // Unmount the device first
    log_info!(MODULE, "Unmounting device partitions...");
    unmount_device(device_path)?;

    // Small delay to ensure unmount completes
    std::thread::sleep(std::time::Duration::from_millis(
        config::flash::UNMOUNT_DELAY_MS,
    ));

    // Try to open device via UDisks2 first (handles polkit auth)
    // Fall back to direct open if UDisks2 fails (e.g., if running as root)
    log_debug!(MODULE, "Opening device for writing...");
    let mut device = match open_device_udisks2(device_path).await {
        Ok(file) => file,
        Err(e) => {
            log_debug!(MODULE, "UDisks2 open failed ({}), trying direct open...", e);
            open_device_direct(device_path)?
        }
    };

    let device_fd = device.as_raw_fd();

    // Quick erase - clear partition table area
    quick_erase(&mut device)?;

    // Open image file
    let mut image_file =
        File::open(image_path).map_err(|e| format!("Failed to open image: {}", e))?;

    // Write image in chunks with progress
    let chunk_size = config::flash::CHUNK_SIZE;
    let mut buffer = vec![0u8; chunk_size];
    let mut written: u64 = 0;

    // Use ProgressTracker for automatic progress logging
    let mut tracker = ProgressTracker::new(
        "Write",
        MODULE,
        image_size,
        config::logging::WRITE_LOG_INTERVAL_MB,
    );

    log_info!(MODULE, "Writing image...");

    // Sync interval to show real progress (not just cache writes)
    // This ensures the progress bar reflects actual disk writes, not just memory cache
    let mut bytes_since_sync: u64 = 0;

    loop {
        if state.is_cancelled.load(Ordering::SeqCst) {
            return Err("Flash cancelled".to_string());
        }

        let bytes_read = image_file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read image: {}", e))?;

        if bytes_read == 0 {
            break;
        }

        if let Err(e) = device.write_all(&buffer[..bytes_read]) {
            log_error!(MODULE, "Write error at byte {}: {}", written, e);
            return Err(format!("Failed to write at byte {}: {}", written, e));
        }

        written += bytes_read as u64;
        bytes_since_sync += bytes_read as u64;

        // Periodic sync to flush data to disk and show real progress
        if bytes_since_sync >= config::logging::LINUX_SYNC_INTERVAL {
            unsafe {
                libc::fdatasync(device_fd);
            }
            bytes_since_sync = 0;
            state.written_bytes.store(written, Ordering::SeqCst);
        }

        // ProgressTracker handles logging automatically
        tracker.update(bytes_read as u64);
    }

    // Log final summary
    tracker.finish();
    log_debug!(MODULE, "Syncing...");

    // Sync
    device.flush().ok();
    unsafe {
        libc::fsync(device_fd);
    }
    sync_device(device_path);

    // Verify if requested
    if verify {
        log_info!(MODULE, "Starting verification...");
        state.is_verifying.store(true, Ordering::SeqCst);
        state.verified_bytes.store(0, Ordering::SeqCst);

        // Invalidate page cache before verification to ensure we read from disk
        // This is critical - without this, we'd just be verifying cached data
        unsafe {
            libc::posix_fadvise(device_fd, 0, image_size as i64, libc::POSIX_FADV_DONTNEED);
        }

        // Seek back to beginning
        device
            .seek(SeekFrom::Start(0))
            .map_err(|e| format!("Failed to seek device: {}", e))?;

        verify_written_data(image_path, &mut device, state.clone())?;
    }

    log_info!(MODULE, "Flash complete!");
    Ok(())
}

/// Quick erase - write zeros to first portion of device
fn quick_erase(device: &mut File) -> Result<(), String> {
    let erase_size = config::flash::QUICK_ERASE_SIZE;
    let chunk_size = config::flash::ERASE_CHUNK_SIZE;

    log_debug!(
        MODULE,
        "Quick erase: writing zeros to first {} MB",
        erase_size / (1024 * 1024)
    );

    // Seek to beginning
    device
        .seek(SeekFrom::Start(0))
        .map_err(|e| format!("Failed to seek to start: {}", e))?;

    let zero_buffer = vec![0u8; chunk_size];
    let mut erased: usize = 0;

    while erased < erase_size {
        let to_write = std::cmp::min(chunk_size, erase_size - erased);
        device
            .write_all(&zero_buffer[..to_write])
            .map_err(|e| format!("Quick erase failed at byte {}: {}", erased, e))?;
        erased += to_write;
    }

    // Sync the erase
    device.flush().ok();

    // Seek back to beginning for image write
    device
        .seek(SeekFrom::Start(0))
        .map_err(|e| format!("Failed to seek to start: {}", e))?;

    log_debug!(MODULE, "Quick erase complete");
    Ok(())
}

/// Verify written data
fn verify_written_data(
    image_path: &PathBuf,
    device: &mut File,
    state: Arc<FlashState>,
) -> Result<(), String> {
    crate::flash::verify::verify_data(image_path, device, state)
}

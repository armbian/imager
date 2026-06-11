//! Linux device writer. Uses UDisks2 (polkit auth) so the app can run as a
//! normal user, falling back to a direct open when UDisks2 is unavailable.

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

/// Open a block device for writing via UDisks2, which prompts polkit auth as needed.
async fn open_device_udisks2(device_path: &str) -> Result<File, String> {
    use std::collections::HashMap;

    log_debug!(MODULE, "Opening device via UDisks2: {}", device_path);

    let client = udisks2::Client::new()
        .await
        .map_err(|e| format!("Failed to connect to UDisks2: {}", e))?;

    // /dev/sdX maps to /org/freedesktop/UDisks2/block_devices/sdX.
    let dev_name = device_path
        .strip_prefix("/dev/")
        .ok_or_else(|| format!("Invalid device path: {}", device_path))?;

    let object_path = format!("/org/freedesktop/UDisks2/block_devices/{}", dev_name);

    log_debug!(MODULE, "UDisks2 object path: {}", object_path);

    let object = client
        .object(object_path.as_str())
        .map_err(|e| format!("Device not found in UDisks2: {} ({})", device_path, e))?;

    let block = object
        .block()
        .await
        .map_err(|e| format!("Failed to get block interface: {}", e))?;

    let options: HashMap<&str, udisks2::zbus::zvariant::Value<'_>> = HashMap::new();

    let fd = block
        .open_device("rw", options)
        .await
        .map_err(|e| format!("Failed to open device (polkit auth may have failed): {}", e))?;

    log_debug!(MODULE, "Device opened successfully via UDisks2");

    // Take ownership of the fd as a File, then forget the OwnedFd so it isn't closed twice.
    let raw_fd = fd.as_raw_fd();
    let file = unsafe { File::from_raw_fd(raw_fd) };
    std::mem::forget(fd);

    Ok(file)
}

/// Fallback open requiring root, used when UDisks2 is unavailable.
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

    log_info!(MODULE, "Unmounting device partitions...");
    unmount_device(device_path)?;

    // Give the unmount a moment to settle before writing.
    std::thread::sleep(std::time::Duration::from_millis(
        config::flash::UNMOUNT_DELAY_MS,
    ));

    // UDisks2 first (handles polkit auth), direct open as a root fallback.
    log_debug!(MODULE, "Opening device for writing...");
    let mut device = match open_device_udisks2(device_path).await {
        Ok(file) => file,
        Err(e) => {
            log_debug!(MODULE, "UDisks2 open failed ({}), trying direct open...", e);
            open_device_direct(device_path)?
        }
    };

    let device_fd = device.as_raw_fd();

    quick_erase(&mut device)?;

    let mut image_file =
        File::open(image_path).map_err(|e| format!("Failed to open image: {}", e))?;

    let chunk_size = config::flash::CHUNK_SIZE;
    let mut buffer = vec![0u8; chunk_size];
    let mut written: u64 = 0;

    let mut tracker = ProgressTracker::new(
        "Write",
        MODULE,
        image_size,
        config::logging::WRITE_LOG_INTERVAL_MB,
    );

    log_info!(MODULE, "Writing image...");

    // Sync periodically so progress reflects actual disk writes, not page cache
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
            return Err(crate::flash::write_failed_err(written, e));
        }

        written += bytes_read as u64;
        bytes_since_sync += bytes_read as u64;

        if bytes_since_sync >= config::logging::LINUX_SYNC_INTERVAL {
            // A failing card often surfaces only here, when buffered pages hit the device.
            if unsafe { libc::fdatasync(device_fd) } != 0 {
                let e = std::io::Error::last_os_error();
                log_error!(MODULE, "fdatasync failed at byte {}: {}", written, e);
                return Err(crate::flash::write_failed_err(written, e));
            }
            bytes_since_sync = 0;
            state.written_bytes.store(written, Ordering::SeqCst);
        }

        tracker.update(bytes_read as u64);
    }

    tracker.finish();
    log_debug!(MODULE, "Syncing...");

    device
        .flush()
        .map_err(|e| crate::flash::write_failed_err(written, e))?;
    crate::flash::fsync_checked(device_fd, written)?;
    sync_device(device_path);

    if verify {
        log_info!(MODULE, "Starting verification...");
        state.is_verifying.store(true, Ordering::SeqCst);
        state.verified_bytes.store(0, Ordering::SeqCst);

        // Drop page cache so verification reads from the device, not cached data
        unsafe {
            libc::posix_fadvise(device_fd, 0, image_size as i64, libc::POSIX_FADV_DONTNEED);
        }

        device
            .seek(SeekFrom::Start(0))
            .map_err(|e| format!("Failed to seek device: {}", e))?;

        verify_written_data(image_path, &mut device, state.clone())?;
    }

    log_info!(MODULE, "Flash complete!");
    Ok(())
}

/// Zero the first portion of the device to wipe the old partition table.
fn quick_erase(device: &mut File) -> Result<(), String> {
    let erase_size = config::flash::QUICK_ERASE_SIZE;
    let chunk_size = config::flash::ERASE_CHUNK_SIZE;

    log_debug!(
        MODULE,
        "Quick erase: writing zeros to first {} MB",
        erase_size / (1024 * 1024)
    );

    device
        .seek(SeekFrom::Start(0))
        .map_err(|e| format!("Failed to seek to start: {}", e))?;

    crate::flash::write_zeros(device, erase_size, chunk_size)?;

    device
        .flush()
        .map_err(|e| crate::flash::write_failed_err(0, e))?;

    // Rewind so the image write starts at offset 0.
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

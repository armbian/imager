//! Windows-specific flash implementation.
//!
//! Requires Administrator privileges for raw disk access.

use super::FlashState;
use crate::config;
use crate::{log_debug, log_error, log_info, log_warn};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const MODULE: &str = "flash::windows";

#[cfg(target_os = "windows")]
use std::ffi::OsStr;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use std::os::windows::io::FromRawHandle;

#[cfg(target_os = "windows")]
const FILE_FLAG_NO_BUFFERING: u32 = 0x20000000;
#[cfg(target_os = "windows")]
const FILE_FLAG_WRITE_THROUGH: u32 = 0x80000000;

/// Flashes an image to a block device.
///
/// Requires Administrator privileges on Windows.
pub async fn flash_image(
    image_path: &PathBuf,
    device_path: &str,
    state: Arc<FlashState>,
    verify: bool,
) -> Result<(), String> {
    state.reset();

    log_info!(MODULE, "Starting flash: {} -> {}", image_path.display(), device_path);

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

    let disk_number = extract_disk_number(device_path)?;

    log_info!(MODULE, "Locking volumes on disk {}...", disk_number);
    let _volume_locks = lock_disk_volumes(disk_number)?;
    std::thread::sleep(std::time::Duration::from_millis(500));

    let mut image_file =
        std::fs::File::open(image_path).map_err(|e| format!("Failed to open image: {}", e))?;

    log_info!(MODULE, "Opening device for writing...");
    let mut device = open_device_for_write(device_path)?;

    let chunk_size = config::flash::CHUNK_SIZE;
    let mut buffer = vec![0u8; chunk_size];
    let mut written: u64 = 0;

    log_info!(MODULE, "Writing image to device...");

    loop {
        if state.is_cancelled.load(Ordering::SeqCst) {
            log_info!(MODULE, "Flash cancelled by user");
            return Err("Flash cancelled".to_string());
        }

        let bytes_read = image_file.read(&mut buffer).map_err(|e| {
            log_error!(MODULE, "Failed to read image: {}", e);
            format!("Failed to read image: {}", e)
        })?;

        if bytes_read == 0 {
            break;
        }

        device.write_all(&buffer[..bytes_read]).map_err(|e| {
            log_error!(MODULE, "Failed to write to device: {}", e);
            format!("Failed to write to device: {}", e)
        })?;

        written += bytes_read as u64;
        state.written_bytes.store(written, Ordering::SeqCst);

        if written % (512 * 1024 * 1024) == 0 {
            log_info!(
                MODULE,
                "Write progress: {:.1}%",
                (written as f64 / image_size as f64) * 100.0
            );
        }
    }

    log_info!(MODULE, "Flushing write cache...");
    device.flush().ok();
    flush_device_buffers(&device)?;

    log_info!(MODULE, "Write complete!");

    if verify {
        log_info!(MODULE, "Starting verification...");
        drop(device);
        std::thread::sleep(std::time::Duration::from_millis(500));
        let device = open_device_for_read(device_path)?;
        verify_with_sector_alignment(image_path, device, state)?;
    }

    log_info!(MODULE, "Flash complete, releasing volume locks...");
    Ok(())
}

/// Extracts the disk number from a device path (e.g., `\\.\PhysicalDrive1` -> `1`).
fn extract_disk_number(device_path: &str) -> Result<u32, String> {
    let prefix = r"\\.\PhysicalDrive";
    if !device_path.starts_with(prefix) {
        return Err(format!(
            "Invalid device path: {}. Expected: {}<number>",
            device_path, prefix
        ));
    }

    device_path[prefix.len()..]
        .parse::<u32>()
        .map_err(|e| format!("Failed to parse disk number: {}", e))
}

/// RAII container for locked volume handles.
#[cfg(target_os = "windows")]
struct VolumeLocks {
    handles: Vec<*mut std::ffi::c_void>,
}

#[cfg(target_os = "windows")]
impl Drop for VolumeLocks {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Ioctl::FSCTL_UNLOCK_VOLUME;
        use windows_sys::Win32::System::IO::DeviceIoControl;

        log_info!(MODULE, "Releasing {} volume lock(s)...", self.handles.len());

        for handle in self.handles.drain(..) {
            // Convert to usize for thread-safe transfer
            let handle_val = handle as usize;

            // Spawn cleanup in separate thread to avoid blocking
            std::thread::spawn(move || {
                unsafe {
                    let h = handle_val as *mut std::ffi::c_void;
                    // Unlock volume before closing to prevent hangs
                    let mut bytes_ret: u32 = 0;
                    DeviceIoControl(
                        h,
                        FSCTL_UNLOCK_VOLUME,
                        std::ptr::null(),
                        0,
                        std::ptr::null_mut(),
                        0,
                        &mut bytes_ret,
                        std::ptr::null_mut(),
                    );
                    CloseHandle(h);
                }
            });
        }

        log_info!(MODULE, "Volume lock release initiated");
    }
}

#[cfg(not(target_os = "windows"))]
struct VolumeLocks;

/// Locks and dismounts all volumes on the specified disk.
///
/// Uses `FindFirstVolume`/`FindNextVolume` to enumerate volumes and
/// `IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS` to identify which disk they belong to.
/// Handles are kept open to prevent Windows from remounting during the operation.
#[cfg(target_os = "windows")]
fn lock_disk_volumes(disk_number: u32) -> Result<VolumeLocks, String> {
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE, MAX_PATH,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, FindFirstVolumeW, FindNextVolumeW,
        FindVolumeClose, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Ioctl::{FSCTL_DISMOUNT_VOLUME, FSCTL_LOCK_VOLUME};
    use windows_sys::Win32::System::IO::DeviceIoControl;

    #[repr(C)]
    struct DiskExtent {
        disk_number: u32,
        starting_offset: i64,
        extent_length: i64,
    }

    #[repr(C)]
    struct VolumeDiskExtents {
        number_of_disk_extents: u32,
        extents: [DiskExtent; 1],
    }

    const IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS: u32 = 0x00560000;

    let mut locked_handles = Vec::new();
    log_info!(MODULE, "Enumerating volumes on disk {}", disk_number);

    unsafe {
        let mut volume_name: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];

        let find_handle = FindFirstVolumeW(volume_name.as_mut_ptr(), MAX_PATH);
        if find_handle.is_null() {
            log_warn!(MODULE, "FindFirstVolumeW failed: {}", GetLastError());
            return Ok(VolumeLocks { handles: locked_handles });
        }

        loop {
            let vol_len = volume_name.iter().position(|&c| c == 0).unwrap_or(volume_name.len());
            let vol_str = String::from_utf16_lossy(&volume_name[..vol_len]);

            // Remove trailing backslash for CreateFile
            let vol_path: Vec<u16> = if vol_len > 0 && volume_name[vol_len - 1] == b'\\' as u16 {
                volume_name[..vol_len - 1].iter().copied().chain(std::iter::once(0)).collect()
            } else {
                volume_name[..vol_len].iter().copied().chain(std::iter::once(0)).collect()
            };

            let vol_handle = CreateFileW(
                vol_path.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            );

            if vol_handle != INVALID_HANDLE_VALUE && !vol_handle.is_null() {
                let mut disk_extents: VolumeDiskExtents = std::mem::zeroed();
                let mut bytes_returned: u32 = 0;

                let result = DeviceIoControl(
                    vol_handle,
                    IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
                    std::ptr::null(),
                    0,
                    &mut disk_extents as *mut _ as *mut _,
                    std::mem::size_of::<VolumeDiskExtents>() as u32,
                    &mut bytes_returned,
                    std::ptr::null_mut(),
                );

                CloseHandle(vol_handle);

                if result != 0
                    && disk_extents.number_of_disk_extents > 0
                    && disk_extents.extents[0].disk_number == disk_number
                {
                    log_debug!(MODULE, "Found volume on disk {}: {}", disk_number, vol_str);

                    let lock_handle = CreateFileW(
                        vol_path.as_ptr(),
                        GENERIC_READ | GENERIC_WRITE,
                        FILE_SHARE_READ | FILE_SHARE_WRITE,
                        std::ptr::null(),
                        OPEN_EXISTING,
                        0,
                        std::ptr::null_mut(),
                    );

                    if lock_handle != INVALID_HANDLE_VALUE && !lock_handle.is_null() {
                        let mut bytes_ret: u32 = 0;

                        let lock_ok = DeviceIoControl(
                            lock_handle,
                            FSCTL_LOCK_VOLUME,
                            std::ptr::null(),
                            0,
                            std::ptr::null_mut(),
                            0,
                            &mut bytes_ret,
                            std::ptr::null_mut(),
                        );

                        if lock_ok != 0 {
                            DeviceIoControl(
                                lock_handle,
                                FSCTL_DISMOUNT_VOLUME,
                                std::ptr::null(),
                                0,
                                std::ptr::null_mut(),
                                0,
                                &mut bytes_ret,
                                std::ptr::null_mut(),
                            );
                            log_info!(MODULE, "Locked volume: {}", vol_str);
                            locked_handles.push(lock_handle);
                        } else {
                            log_warn!(MODULE, "Cannot lock {}: error {}", vol_str, GetLastError());
                            CloseHandle(lock_handle);
                        }
                    }
                }
            }

            if FindNextVolumeW(find_handle, volume_name.as_mut_ptr(), MAX_PATH) == 0 {
                break;
            }
        }

        FindVolumeClose(find_handle);
    }

    log_info!(MODULE, "Holding {} volume lock(s)", locked_handles.len());
    Ok(VolumeLocks { handles: locked_handles })
}

#[cfg(not(target_os = "windows"))]
fn lock_disk_volumes(_disk_number: u32) -> Result<VolumeLocks, String> {
    Ok(VolumeLocks)
}

/// Flushes all pending writes to the physical device.
#[cfg(target_os = "windows")]
fn flush_device_buffers(device: &std::fs::File) -> Result<(), String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::FlushFileBuffers;

    unsafe {
        let handle = device.as_raw_handle();
        let result = FlushFileBuffers(handle as *mut _);

        if result == 0 {
            let error_code = windows_sys::Win32::Foundation::GetLastError();
            log_error!(MODULE, "FlushFileBuffers failed: error {}", error_code);
            return Err(format!("Failed to flush buffers: error {}", error_code));
        }
    }

    log_debug!(MODULE, "Buffers flushed successfully");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn flush_device_buffers(_device: &std::fs::File) -> Result<(), String> {
    Ok(())
}

/// Verifies written data using sector-aligned reads.
///
/// Required when using `FILE_FLAG_NO_BUFFERING` which bypasses the OS cache.
#[cfg(target_os = "windows")]
fn verify_with_sector_alignment(
    image_path: &PathBuf,
    mut device: std::fs::File,
    state: Arc<FlashState>,
) -> Result<(), String> {
    state.is_verifying.store(true, Ordering::SeqCst);
    state.verified_bytes.store(0, Ordering::SeqCst);

    let mut image_file = std::fs::File::open(image_path)
        .map_err(|e| format!("Failed to open image: {}", e))?;

    let image_size = state.total_bytes.load(Ordering::SeqCst);

    log_info!(
        MODULE,
        "Verifying {} bytes ({:.2} GB)",
        image_size,
        image_size as f64 / 1024.0 / 1024.0 / 1024.0
    );

    let sector_size = get_device_sector_size(&device)?;
    let chunk_size = config::flash::CHUNK_SIZE;
    let aligned_chunk_size = (chunk_size / sector_size) * sector_size;

    log_debug!(MODULE, "Sector size: {} bytes, chunk size: {} bytes", sector_size, aligned_chunk_size);

    let mut image_buffer = vec![0u8; aligned_chunk_size];
    let mut device_buffer = vec![0u8; aligned_chunk_size];
    let mut verified: u64 = 0;

    while verified < image_size {
        if state.is_cancelled.load(Ordering::SeqCst) {
            return Err("Verification cancelled".to_string());
        }

        let remaining = image_size - verified;
        let read_size = std::cmp::min(aligned_chunk_size as u64, remaining) as usize;

        let image_read = image_file
            .read(&mut image_buffer[..read_size])
            .map_err(|e| format!("Failed to read image: {}", e))?;

        if image_read == 0 {
            break;
        }

        // Align device read to sector boundary
        let device_read_size = ((image_read + sector_size - 1) / sector_size) * sector_size;

        let mut total_read = 0;
        while total_read < device_read_size {
            let n = device
                .read(&mut device_buffer[total_read..device_read_size])
                .map_err(|e| format!("Failed to read device at byte {}: {}", verified + total_read as u64, e))?;
            if n == 0 {
                break;
            }
            total_read += n;
        }

        if image_buffer[..image_read] != device_buffer[..image_read] {
            log_error!(MODULE, "Data mismatch at byte {}", verified);

            for i in 0..std::cmp::min(image_read, 16) {
                if image_buffer[i] != device_buffer[i] {
                    log_error!(
                        MODULE,
                        "First mismatch at offset {}: expected {:02x}, got {:02x}",
                        i,
                        image_buffer[i],
                        device_buffer[i]
                    );
                    break;
                }
            }

            return Err(format!("Verification failed at byte {}", verified));
        }

        verified += image_read as u64;
        state.verified_bytes.store(verified, Ordering::SeqCst);

        if verified % (512 * 1024 * 1024) == 0 {
            log_info!(
                MODULE,
                "Verification progress: {:.1}%",
                (verified as f64 / image_size as f64) * 100.0
            );
        }
    }

    log_info!(MODULE, "Verification complete!");
    Ok(())
}

/// Retrieves the physical sector size of the device.
#[cfg(target_os = "windows")]
fn get_device_sector_size(device: &std::fs::File) -> Result<usize, String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Ioctl::IOCTL_DISK_GET_DRIVE_GEOMETRY;
    use windows_sys::Win32::System::IO::DeviceIoControl;

    #[repr(C)]
    struct DiskGeometry {
        cylinders: i64,
        media_type: u32,
        tracks_per_cylinder: u32,
        sectors_per_track: u32,
        bytes_per_sector: u32,
    }

    unsafe {
        let handle = device.as_raw_handle();
        let mut geometry: DiskGeometry = std::mem::zeroed();
        let mut bytes_returned: u32 = 0;

        let result = DeviceIoControl(
            handle as *mut _,
            IOCTL_DISK_GET_DRIVE_GEOMETRY,
            std::ptr::null(),
            0,
            &mut geometry as *mut _ as *mut _,
            std::mem::size_of::<DiskGeometry>() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );

        if result == 0 {
            log_warn!(MODULE, "Failed to query sector size, using default 512");
            return Ok(512);
        }

        let sector_size = geometry.bytes_per_sector as usize;

        if sector_size < 512 || sector_size > 8192 || (sector_size & (sector_size - 1)) != 0 {
            log_warn!(MODULE, "Invalid sector size {}, using default 512", sector_size);
            return Ok(512);
        }

        Ok(sector_size)
    }
}

/// Opens device for writing with write-through caching.
#[cfg(target_os = "windows")]
fn open_device_for_write(device_path: &str) -> Result<std::fs::File, String> {
    use windows_sys::Win32::Foundation::{GetLastError, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };

    log_debug!(MODULE, "Opening {} for writing", device_path);

    let wide_path: Vec<u16> = OsStr::new(device_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let handle = CreateFileW(
            wide_path.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_WRITE_THROUGH,
            std::ptr::null_mut(),
        );

        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            let error_code = GetLastError();
            let msg = match error_code {
                5 => "Access denied. Run as Administrator.".to_string(),
                32 => "Device in use by another process.".to_string(),
                33 => "Device is locked.".to_string(),
                _ => format!("Error code {}", error_code),
            };
            return Err(format!("Failed to open {}: {}", device_path, msg));
        }

        log_debug!(MODULE, "Device opened for writing");
        Ok(std::fs::File::from_raw_handle(handle as *mut _))
    }
}

#[cfg(not(target_os = "windows"))]
fn open_device_for_write(device_path: &str) -> Result<std::fs::File, String> {
    std::fs::OpenOptions::new()
        .write(true)
        .read(true)
        .open(device_path)
        .map_err(|e| format!("Failed to open device: {}", e))
}

/// Opens device for reading with cache bypass for verification.
#[cfg(target_os = "windows")]
fn open_device_for_read(device_path: &str) -> Result<std::fs::File, String> {
    use windows_sys::Win32::Foundation::{GetLastError, GENERIC_READ, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };

    log_debug!(MODULE, "Opening {} for reading", device_path);

    let wide_path: Vec<u16> = OsStr::new(device_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let handle = CreateFileW(
            wide_path.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_NO_BUFFERING,
            std::ptr::null_mut(),
        );

        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            let error_code = GetLastError();
            return Err(format!("Failed to open {} for reading: error {}", device_path, error_code));
        }

        log_debug!(MODULE, "Device opened for reading");
        Ok(std::fs::File::from_raw_handle(handle as *mut _))
    }
}

#[cfg(not(target_os = "windows"))]
fn open_device_for_read(device_path: &str) -> Result<std::fs::File, String> {
    std::fs::OpenOptions::new()
        .read(true)
        .open(device_path)
        .map_err(|e| format!("Failed to open device: {}", e))
}

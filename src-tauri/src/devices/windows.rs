//! Windows device detection using native Win32 APIs

use std::ffi::c_void;
use std::mem;

use crate::log_error;
use crate::utils::format_size;

use super::types::BlockDevice;

#[cfg(target_os = "windows")]
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, GENERIC_READ, HANDLE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING},
    System::Ioctl::IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
    System::IO::DeviceIoControl,
};

// ===== IOCTL Codes =====

const IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS: u32 = 0x00560000;
const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D1400;
/// IOCTL to get media types with characteristics including write protection status
/// More reliable than IOCTL_DISK_IS_WRITABLE for SD cards with physical lock switch
const IOCTL_STORAGE_GET_MEDIA_TYPES_EX: u32 = 0x002D0030;

// ===== Write Protection Constants =====

/// Flag indicating media is write-protected (physical lock switch engaged)
/// Used in DeviceMediaInfo.media_characteristics
const MEDIA_WRITE_PROTECTED: u32 = 0x00000100;

// ===== Storage Property Constants =====

const STORAGE_DEVICE_PROPERTY: u32 = 0;
const PROPERTY_STANDARD_QUERY: u32 = 0;

// ===== Structures =====

/// STORAGE_PROPERTY_QUERY - matches C++ winioctl.h layout
#[repr(C)]
#[derive(Debug, Clone)]
struct STORAGE_PROPERTY_QUERY {
    property_id: u32,
    query_type: u32,
    additional_parameters: [u8; 1],
}

/// DISK_GEOMETRY_EX - returned by IOCTL_DISK_GET_DRIVE_GEOMETRY_EX
#[repr(C)]
#[derive(Debug, Clone)]
struct DiskGeometryEx {
    geometry: DiskGeometry,
    disk_size: u64,
    data: [u8; 1],
}

/// DISK_GEOMETRY - disk geometry parameters
#[repr(C)]
#[derive(Debug, Clone)]
struct DiskGeometry {
    cylinders: i64,
    media_type: u32,
    tracks_per_cylinder: u32,
    sectors_per_track: u32,
    bytes_per_sector: u32,
}

/// VOLUME_DISK_EXTENT - maps a volume extent to a physical disk
#[repr(C)]
#[derive(Debug, Clone)]
struct VolumeDiskExtent {
    disk_number: u32,
    starting_offset: u64,
    extent_length: u64,
}

/// VOLUME_DISK_EXTENTS - contains array of volume-to-disk mappings
#[repr(C)]
#[derive(Debug, Clone)]
struct VolumeDiskExtents {
    number_of_extents: u32,
    extents: [VolumeDiskExtent; 1],
}

/// GET_MEDIA_TYPES structure header returned by IOCTL_STORAGE_GET_MEDIA_TYPES_EX
/// See: https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ni-winioctl-ioctl_storage_get_media_types_ex
#[repr(C)]
#[derive(Debug, Clone)]
struct GetMediaTypes {
    /// Device type (FILE_DEVICE_*)
    device_type: u32,
    /// Number of DEVICE_MEDIA_INFO structures that follow
    media_info_count: u32,
    // Followed by array of DEVICE_MEDIA_INFO
}

/// DEVICE_MEDIA_INFO for removable media
/// Contains media characteristics including write protection status
/// See: https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ns-winioctl-device_media_info
#[repr(C)]
#[derive(Debug, Clone)]
struct DeviceMediaInfo {
    cylinders: i64,
    media_type: u32,
    tracks_per_cylinder: u32,
    sectors_per_track: u32,
    bytes_per_sector: u32,
    number_media_sides: u32,
    /// Bitmask of media characteristics - check MEDIA_WRITE_PROTECTED (0x100)
    media_characteristics: u32,
}

// ===== External Win32 API =====

extern "system" {
    fn GetLogicalDrives() -> u32;
}

// ===== Helper Functions =====

/// Converts a string path to UTF-16 null-terminated vector for Win32 APIs
fn to_utf16(path: &str) -> Vec<u16> {
    path.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Attempts to open a device handle, returns Ok(handle) or Err(error_code)
fn try_open_device(path_utf16: &[u16]) -> Result<HANDLE, u32> {
    let handle = unsafe {
        CreateFileW(
            path_utf16.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            HANDLE::default(),
        )
    };

    if handle == INVALID_HANDLE_VALUE {
        Err(unsafe { GetLastError() })
    } else {
        Ok(handle)
    }
}

/// Maps STORAGE_BUS_TYPE enum byte to human-readable string
fn bus_type_to_string(bus_type_enum: u8) -> Option<&'static str> {
    const BUS_TYPE_MAP: &[(&str, u8)] = &[
        ("Unknown", 0x00),
        ("SCSI", 0x01),
        ("ATAPI", 0x02),
        ("ATA", 0x03),
        ("1394", 0x04),
        ("SSA", 0x05),
        ("Fibre", 0x06),
        ("USB", 0x07),
        ("RAID", 0x08),
        ("iSCSI", 0x09),
        ("SAS", 0x0A),
        ("SATA", 0x0B),
        ("SD", 0x0C),
        ("MMC", 0x0D),
        ("Virtual", 0x0E),
        ("FileBacked", 0x0F),
        ("Spaces", 0x10),
        ("NVMe", 0x11),
        ("SCM", 0x12),
        ("UFS", 0x13),
        ("NVMe-oF", 0x14),
    ];

    BUS_TYPE_MAP
        .iter()
        .find(|(_, code)| *code == bus_type_enum)
        .map(|(name, _)| *name)
}

/// Extracts null-terminated ASCII string from buffer at offset
fn extract_ascii_string(buffer: &[u8], offset: usize) -> String {
    if offset == 0 || offset >= buffer.len() {
        return "Physical Drive".to_string();
    }

    let end = buffer[offset..]
        .iter()
        .position(|&b| b == 0)
        .map(|pos| offset + pos)
        .unwrap_or(buffer.len());

    if end > offset {
        String::from_utf8_lossy(&buffer[offset..end])
            .trim()
            .to_string()
    } else {
        "Physical Drive".to_string()
    }
}

/// Checks if a disk is write-protected using IOCTL_STORAGE_GET_MEDIA_TYPES_EX
///
/// This method checks the MEDIA_WRITE_PROTECTED flag in MediaCharacteristics,
/// which reliably detects the physical write protection lock switch on SD cards.
///
/// Note: IOCTL_DISK_IS_WRITABLE only checks driver capability, not physical lock state.
///
/// Returns true if the disk is write-protected (lock switch engaged)
#[cfg(target_os = "windows")]
fn is_disk_read_only(handle: HANDLE) -> bool {
    // Buffer size for GET_MEDIA_TYPES response (header + array of DEVICE_MEDIA_INFO)
    let mut buffer = [0u8; 2048];
    let mut bytes_returned: u32 = 0;

    let result = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_GET_MEDIA_TYPES_EX,
            std::ptr::null(),
            0,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };

    // Size of GetMediaTypes header (device_type + media_info_count)
    const HEADER_SIZE: u32 = 8;
    // Size of DeviceMediaInfo structure
    const MEDIA_INFO_SIZE: u32 = 32;

    // IOCTL failed or insufficient data returned - assume writable
    if result == 0 || bytes_returned < HEADER_SIZE {
        return false;
    }

    // Parse GET_MEDIA_TYPES header
    let header = unsafe { &*(buffer.as_ptr() as *const GetMediaTypes) };

    // No media info available - assume writable
    if header.media_info_count == 0 {
        return false;
    }

    // Check if we have enough data for at least one DEVICE_MEDIA_INFO
    if bytes_returned < HEADER_SIZE + MEDIA_INFO_SIZE {
        return false;
    }

    // Parse first DEVICE_MEDIA_INFO (immediately after header)
    let media_info =
        unsafe { &*(buffer.as_ptr().add(HEADER_SIZE as usize) as *const DeviceMediaInfo) };

    // Check MEDIA_WRITE_PROTECTED flag (0x100) in media_characteristics
    (media_info.media_characteristics & MEDIA_WRITE_PROTECTED) != 0
}

/// Queries device properties via IOCTL_STORAGE_QUERY_PROPERTY
fn query_device_properties(disk_number: i32) -> Result<(String, bool, Option<String>), String> {
    const MIN_DESCRIPTOR_SIZE: u32 = 33;
    const PRODUCT_ID_OFFSET: usize = 16;
    const BUS_TYPE_OFFSET: usize = 28;

    let device_path = format!("\\\\.\\PhysicalDrive{}", disk_number);
    let device_path_utf16 = to_utf16(&device_path);

    let handle = match try_open_device(&device_path_utf16) {
        Ok(h) => h,
        Err(_) => return Ok(("Physical Drive".to_string(), false, None)),
    };

    let query = STORAGE_PROPERTY_QUERY {
        property_id: STORAGE_DEVICE_PROPERTY,
        query_type: PROPERTY_STANDARD_QUERY,
        additional_parameters: [0],
    };

    let mut buffer = [0u8; 2048];
    let mut bytes_returned = 0u32;

    let result = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            &query as *const _ as *mut c_void,
            mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };

    unsafe { CloseHandle(handle) };

    if result == 0 || bytes_returned < MIN_DESCRIPTOR_SIZE {
        return Ok(("Physical Drive".to_string(), false, None));
    }

    let bus_type_enum = buffer[BUS_TYPE_OFFSET];
    let bus_type = bus_type_to_string(bus_type_enum).map(|s| s.to_string());

    let product_id_offset = u32::from_le_bytes(
        buffer[PRODUCT_ID_OFFSET..PRODUCT_ID_OFFSET + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    let model = extract_ascii_string(&buffer, product_id_offset);
    let model = if model.is_empty() {
        "Physical Drive".to_string()
    } else {
        model
    };

    let is_removable = match bus_type.as_deref() {
        Some(bt) => bt == "USB" || bt == "SD",
        None => disk_number > 0,
    };

    Ok((model, is_removable, bus_type))
}

/// Retrieves drive letters mounted on a specific physical disk
fn get_drive_letters_for_disk(disk_number: i32) -> Option<Vec<String>> {
    let drives_mask = unsafe { GetLogicalDrives() };
    if drives_mask == 0 {
        log_error!("devices", "GetLogicalDrives failed: {}", unsafe {
            GetLastError()
        });
        return None;
    }

    let mut drive_letters = Vec::new();

    for i in 0..26 {
        if (drives_mask & (1 << i)) == 0 {
            continue;
        }

        let letter_char = (b'A' + i) as char;
        let drive_path = format!(r"\\?\{}:", letter_char);
        let drive_path_utf16 = to_utf16(&drive_path);

        let handle = match try_open_device(&drive_path_utf16) {
            Ok(h) if h != INVALID_HANDLE_VALUE => h,
            _ => continue,
        };

        let mut extents_bytes = [0u8; 1024];
        let mut bytes_returned = 0u32;

        let result = unsafe {
            DeviceIoControl(
                handle,
                IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
                std::ptr::null_mut(),
                0,
                extents_bytes.as_mut_ptr() as *mut c_void,
                extents_bytes.len() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        unsafe { CloseHandle(handle) };

        if result != 0 {
            let extents = unsafe { &*(extents_bytes.as_ptr() as *const VolumeDiskExtents) };

            for j in 0..extents.number_of_extents {
                if extents.extents[j as usize].disk_number as i32 == disk_number {
                    drive_letters.push(format!("{}:", letter_char));
                    break;
                }
            }
        }
    }

    if drive_letters.is_empty() {
        None
    } else {
        Some(drive_letters)
    }
}

/// Enumerates all block devices on Windows using native Win32 APIs
pub fn get_block_devices() -> Result<Vec<BlockDevice>, String> {
    #[cfg(target_os = "windows")]
    {
        let mut devices = Vec::new();
        let mut consecutive_errors = 0;
        const MAX_CONSECUTIVE_ERRORS: usize = 4; // Stop after 4 consecutive non-existent drives

        for disk_number in 0..32 {
            let device_path = format!("\\\\.\\PhysicalDrive{}", disk_number);
            let device_path_utf16 = to_utf16(&device_path);

            let handle = match try_open_device(&device_path_utf16) {
                Ok(h) if h != INVALID_HANDLE_VALUE => {
                    consecutive_errors = 0; // Reset counter on success
                    h
                }
                Err(1 | 2 | 5 | 21) => {
                    consecutive_errors += 1;
                    if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                        break;
                    }
                    continue;
                }
                Err(err) => {
                    log_error!("devices", "Failed to open {}: error {}", device_path, err);
                    consecutive_errors += 1;
                    continue;
                }
                _ => continue,
            };

            let mut geometry_bytes = [0u8; 256];
            let mut bytes_returned = 0u32;

            let result = unsafe {
                DeviceIoControl(
                    handle,
                    IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
                    std::ptr::null_mut(),
                    0,
                    geometry_bytes.as_mut_ptr() as *mut c_void,
                    geometry_bytes.len() as u32,
                    &mut bytes_returned,
                    std::ptr::null_mut(),
                )
            };

            if result == 0 {
                let err = unsafe { GetLastError() };
                unsafe { CloseHandle(handle) };
                // Skip expected errors silently
                if err == 1 || err == 2 || err == 5 || err == 21 {
                    continue;
                }
                log_error!(
                    "devices",
                    "DeviceIoControl failed for {}: error {}",
                    device_path,
                    err
                );
                continue;
            }

            let geometry = unsafe { &*(geometry_bytes.as_ptr() as *const DiskGeometryEx) };
            let size = geometry.disk_size;

            // Check if disk is read-only before closing handle
            let is_read_only = is_disk_read_only(handle);

            unsafe { CloseHandle(handle) };

            if size == 0 {
                continue;
            }

            let (model, is_removable, bus_type) = query_device_properties(disk_number)?;
            let drive_letters = get_drive_letters_for_disk(disk_number);

            let is_system = drive_letters
                .as_ref()
                .map_or(false, |letters| letters.iter().any(|l| l == "C:"));

            let name = match &drive_letters {
                Some(letters) => format!("Disk {} ({})", disk_number, letters.join(", ")),
                None => format!("Disk {}", disk_number),
            };

            devices.push(BlockDevice {
                path: device_path,
                name,
                size,
                size_formatted: format_size(size),
                model,
                is_removable,
                is_system,
                bus_type,
                is_read_only,
            });
        }

        Ok(devices)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Windows device enumeration is only available on Windows".to_string())
    }
}

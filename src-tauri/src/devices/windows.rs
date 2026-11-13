// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

//! Windows device detection using native Win32 APIs

use std::collections::BTreeSet;
use std::ffi::c_void;
use std::mem;
use std::sync::Mutex;

use crate::utils::format_size;
use crate::{log_error, log_info, log_warn};

use super::types::{
    classify_windows_disk, normalize_bus_type, parse_extent_disk_numbers, BlockDevice,
    WindowsDiskSignals, DISK_EXTENTS_HEADER_SIZE, DISK_EXTENT_SIZE,
};

#[cfg(target_os = "windows")]
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, GetLastError, ERROR_MORE_DATA, GENERIC_READ, HANDLE, INVALID_HANDLE_VALUE,
        MAX_PATH,
    },
    Storage::FileSystem::{
        CreateFileW, GetVolumeNameForVolumeMountPointW, GetVolumePathNameW, FILE_SHARE_READ,
        FILE_SHARE_WRITE, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, OPEN_EXISTING,
    },
    System::Ioctl::{
        RemovableMedia, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX, IOCTL_STORAGE_GET_HOTPLUG_INFO,
        MEDIA_TYPE,
    },
    System::SystemInformation::GetSystemWindowsDirectoryW,
    System::IO::DeviceIoControl,
};

// IOCTL Codes
const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D1400;
/// Reports media characteristics incl. write protection; beats IOCTL_DISK_IS_WRITABLE for SD lock switches.
const IOCTL_STORAGE_GET_MEDIA_TYPES_EX: u32 = 0x002D0030;

const DEFAULT_DISK_MODEL: &str = "Physical Drive";

// Write Protection Constants
/// DeviceMediaInfo.media_characteristics bit set when the media is write-protected.
const MEDIA_WRITE_PROTECTED: u32 = 0x00000100;

// Storage Property Constants
const STORAGE_DEVICE_PROPERTY: u32 = 0;
const PROPERTY_STANDARD_QUERY: u32 = 0;

// STORAGE_DEVICE_DESCRIPTOR layout
const MIN_DESCRIPTOR_SIZE: u32 = 33;
const DESCRIPTOR_REMOVABLE_MEDIA_OFFSET: usize = 10;
const DESCRIPTOR_PRODUCT_ID_OFFSET: usize = 16;
const DESCRIPTOR_BUS_TYPE_OFFSET: usize = 28;

const DISK_EXTENTS_INITIAL_CAPACITY: usize = 42;
const DISK_EXTENTS_MAX_CAPACITY: usize = 1024;

// STORAGE_HOTPLUG_INFO layout, read as raw bytes: windows-sys types its BOOLEANs as Rust bool
const HOTPLUG_INFO_SIZE: usize = 8;
const HOTPLUG_MEDIA_REMOVABLE_OFFSET: usize = 4;
const HOTPLUG_MEDIA_HOTPLUG_OFFSET: usize = 5;
const HOTPLUG_DEVICE_HOTPLUG_OFFSET: usize = 6;

static LAST_CLASSIFICATION_LOG: Mutex<Option<String>> = Mutex::new(None);

// Structures
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
    media_type: MEDIA_TYPE,
    tracks_per_cylinder: u32,
    sectors_per_track: u32,
    bytes_per_sector: u32,
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

/// DEVICE_MEDIA_INFO: media characteristics incl. write protection.
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

// External Win32 API
extern "system" {
    fn GetLogicalDrives() -> u32;
}

// Helper Functions
/// Converts a string path to UTF-16 null-terminated vector for Win32 APIs
fn to_utf16(path: &str) -> Vec<u16> {
    path.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Attempts to open a device handle, returns Ok(handle) or Err(error_code)
fn try_open_device(path_utf16: &[u16]) -> Result<HANDLE, u32> {
    try_open_with_access(path_utf16, GENERIC_READ)
}

fn try_open_with_access(path_utf16: &[u16], desired_access: u32) -> Result<HANDLE, u32> {
    let handle = unsafe {
        CreateFileW(
            path_utf16.as_ptr(),
            desired_access,
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
        return DEFAULT_DISK_MODEL.to_string();
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
        DEFAULT_DISK_MODEL.to_string()
    }
}

/// Check MEDIA_WRITE_PROTECTED via IOCTL_STORAGE_GET_MEDIA_TYPES_EX: detects an SD
/// card's physical lock switch, which IOCTL_DISK_IS_WRITABLE misses.
#[cfg(target_os = "windows")]
fn is_disk_read_only(handle: HANDLE) -> bool {
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

    const HEADER_SIZE: u32 = 8; // device_type + media_info_count
    const MEDIA_INFO_SIZE: u32 = 32;

    // On failure or short data, assume writable.
    if result == 0 || bytes_returned < HEADER_SIZE {
        return false;
    }

    let header = unsafe { &*(buffer.as_ptr() as *const GetMediaTypes) };

    if header.media_info_count == 0 {
        return false;
    }

    if bytes_returned < HEADER_SIZE + MEDIA_INFO_SIZE {
        return false;
    }

    let media_info =
        unsafe { &*(buffer.as_ptr().add(HEADER_SIZE as usize) as *const DeviceMediaInfo) };

    (media_info.media_characteristics & MEDIA_WRITE_PROTECTED) != 0
}

struct DeviceDescriptor {
    model: String,
    bus_code: Option<u8>,
    bus_type: Option<String>,
    removable_media_byte: Option<u8>,
}

impl DeviceDescriptor {
    fn unread() -> Self {
        Self {
            model: DEFAULT_DISK_MODEL.to_string(),
            bus_code: None,
            bus_type: None,
            removable_media_byte: None,
        }
    }

    fn is_read(&self) -> bool {
        self.bus_code.is_some()
    }
}

fn query_device_properties(disk_number: i32) -> DeviceDescriptor {
    let device_path = format!("\\\\.\\PhysicalDrive{}", disk_number);
    let device_path_utf16 = to_utf16(&device_path);

    let handle = match try_open_device(&device_path_utf16) {
        Ok(h) => h,
        Err(_) => return DeviceDescriptor::unread(),
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
        return DeviceDescriptor::unread();
    }

    let bus_code = buffer[DESCRIPTOR_BUS_TYPE_OFFSET];
    let bus_type = bus_type_to_string(bus_code).and_then(normalize_bus_type);

    let product_id_offset = u32::from_le_bytes([
        buffer[DESCRIPTOR_PRODUCT_ID_OFFSET],
        buffer[DESCRIPTOR_PRODUCT_ID_OFFSET + 1],
        buffer[DESCRIPTOR_PRODUCT_ID_OFFSET + 2],
        buffer[DESCRIPTOR_PRODUCT_ID_OFFSET + 3],
    ]) as usize;
    let model = extract_ascii_string(&buffer, product_id_offset);
    let model = if model.is_empty() {
        DEFAULT_DISK_MODEL.to_string()
    } else {
        model
    };

    DeviceDescriptor {
        model,
        bus_code: Some(bus_code),
        bus_type,
        removable_media_byte: Some(buffer[DESCRIPTOR_REMOVABLE_MEDIA_OFFSET]),
    }
}

// Retries with a larger buffer on ERROR_MORE_DATA, up to a cap.
fn volume_disk_numbers(handle: HANDLE) -> Result<Vec<u32>, u32> {
    let mut capacity = DISK_EXTENTS_INITIAL_CAPACITY;

    loop {
        let mut buffer = vec![0u8; DISK_EXTENTS_HEADER_SIZE + capacity * DISK_EXTENT_SIZE];
        let mut bytes_returned = 0u32;

        let result = unsafe {
            DeviceIoControl(
                handle,
                IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
                std::ptr::null_mut(),
                0,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        if result != 0 {
            return Ok(parse_extent_disk_numbers(&buffer, bytes_returned));
        }

        let err = unsafe { GetLastError() };
        if err != ERROR_MORE_DATA {
            return Err(err);
        }

        // On ERROR_MORE_DATA the header still carries the full extent count.
        let needed = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
        if needed <= capacity || needed > DISK_EXTENTS_MAX_CAPACITY {
            return Err(err);
        }
        capacity = needed;
    }
}

fn utf16_until_nul(buffer: &[u16]) -> &[u16] {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    &buffer[..end]
}

fn windows_disk_numbers() -> Result<BTreeSet<u32>, String> {
    let mut windows_dir = [0u16; MAX_PATH as usize];
    let len = unsafe { GetSystemWindowsDirectoryW(windows_dir.as_mut_ptr(), MAX_PATH) };
    if len == 0 || len >= MAX_PATH {
        return Err(format!(
            "GetSystemWindowsDirectoryW failed: returned {len}, error {}",
            unsafe { GetLastError() }
        ));
    }

    let mut volume_root = [0u16; MAX_PATH as usize];
    if unsafe { GetVolumePathNameW(windows_dir.as_ptr(), volume_root.as_mut_ptr(), MAX_PATH) } == 0
    {
        return Err(format!("GetVolumePathNameW failed: error {}", unsafe {
            GetLastError()
        }));
    }

    let mut volume_name = [0u16; MAX_PATH as usize];
    if unsafe {
        GetVolumeNameForVolumeMountPointW(volume_root.as_ptr(), volume_name.as_mut_ptr(), MAX_PATH)
    } == 0
    {
        return Err(format!(
            "GetVolumeNameForVolumeMountPointW failed: error {}",
            unsafe { GetLastError() }
        ));
    }

    // The volume GUID path opens only without its trailing backslash.
    let mut volume_path = utf16_until_nul(&volume_name).to_vec();
    while volume_path.last() == Some(&u16::from(b'\\')) {
        volume_path.pop();
    }
    let volume_display = String::from_utf16_lossy(&volume_path);
    volume_path.push(0);

    // Access 0 is enough for the extents query and needs no admin rights.
    let handle = try_open_with_access(&volume_path, 0)
        .map_err(|e| format!("Opening system volume {volume_display} failed: error {e}"))?;
    let extents = volume_disk_numbers(handle);
    unsafe { CloseHandle(handle) };

    let disks: BTreeSet<u32> = extents
        .map_err(|e| format!("Reading extents of {volume_display} failed: error {e}"))?
        .into_iter()
        .collect();
    if disks.is_empty() {
        return Err(format!(
            "System volume {volume_display} reported no disk extents"
        ));
    }
    Ok(disks)
}

// Log-only: these flags must not feed the classification.
fn query_hotplug_flags(handle: HANDLE) -> Option<[bool; 3]> {
    let mut buffer = [0u8; HOTPLUG_INFO_SIZE];
    let mut bytes_returned = 0u32;

    let result = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_GET_HOTPLUG_INFO,
            std::ptr::null(),
            0,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };

    if result == 0 || (bytes_returned as usize) < HOTPLUG_INFO_SIZE {
        return None;
    }

    Some([
        buffer[HOTPLUG_MEDIA_REMOVABLE_OFFSET] != 0,
        buffer[HOTPLUG_MEDIA_HOTPLUG_OFFSET] != 0,
        buffer[HOTPLUG_DEVICE_HOTPLUG_OFFSET] != 0,
    ])
}

fn format_hotplug_flags(flags: Option<[bool; 3]>) -> String {
    match flags {
        Some([media_removable, media_hotplug, device_hotplug]) => format!(
            "hotplug_media_removable={media_removable}, hotplug_media_hotplug={media_hotplug}, \
             hotplug_device_hotplug={device_hotplug}"
        ),
        None => "hotplug_media_removable=unknown, hotplug_media_hotplug=unknown, \
                 hotplug_device_hotplug=unknown"
            .to_string(),
    }
}

// Logs only when the device set changes: the scan runs every 2 s.
fn log_classification_if_changed(system_disks: &Result<BTreeSet<u32>, String>, lines: &[String]) {
    let header = match system_disks {
        Ok(disks) => format!("System disk set (Windows directory): {disks:?}"),
        Err(e) => format!("System disk lookup failed, removable-media flag ignored this scan: {e}"),
    };
    let signature = format!("{header}\n{}", lines.join("\n"));

    let mut last = LAST_CLASSIFICATION_LOG
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if last.as_deref() == Some(signature.as_str()) {
        return;
    }

    match system_disks {
        Ok(_) => log_info!("devices", "{header}"),
        Err(_) => log_warn!("devices", "{header}"),
    }
    for line in lines {
        log_info!("devices", "{line}");
    }
    *last = Some(signature);
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

        let extents = volume_disk_numbers(handle);
        unsafe { CloseHandle(handle) };

        if extents.is_ok_and(|disks| disks.iter().any(|&d| d as i32 == disk_number)) {
            drive_letters.push(format!("{}:", letter_char));
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
        let mut classification_lines = Vec::new();
        let mut consecutive_errors = 0;
        const MAX_CONSECUTIVE_ERRORS: usize = 4;

        let system_disks = windows_disk_numbers();

        for disk_number in 0..32 {
            let device_path = format!("\\\\.\\PhysicalDrive{}", disk_number);
            let device_path_utf16 = to_utf16(&device_path);

            let handle = match try_open_device(&device_path_utf16) {
                Ok(h) if h != INVALID_HANDLE_VALUE => {
                    consecutive_errors = 0;
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
                // 1/2/5/21 are normal for absent or locked drives; skip quietly.
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
            let media_type = geometry.geometry.media_type;

            // Must query before closing the handle.
            let is_read_only = is_disk_read_only(handle);
            let hotplug_flags = query_hotplug_flags(handle);

            unsafe { CloseHandle(handle) };

            if size == 0 {
                continue;
            }

            let descriptor = query_device_properties(disk_number);
            let drive_letters = get_drive_letters_for_disk(disk_number);

            let has_c_drive = drive_letters
                .as_ref()
                .is_some_and(|letters| letters.iter().any(|l| l == "C:"));
            let on_windows_disk = system_disks
                .as_ref()
                .is_ok_and(|disks| u32::try_from(disk_number).is_ok_and(|n| disks.contains(&n)));

            let class = classify_windows_disk(&WindowsDiskSignals {
                bus_type: descriptor.bus_type.as_deref(),
                descriptor_read: descriptor.is_read(),
                media_is_removable: media_type == RemovableMedia,
                on_windows_disk,
                system_lookup_ok: system_disks.is_ok(),
                has_c_drive,
            });

            classification_lines.push(format!(
                "Disk {disk_number} \"{}\": bus={:?} ({:?}), geometry_media_type={media_type}, \
                 descriptor_removable={:?}, {}, on_windows_disk={on_windows_disk}, \
                 has_c_drive={has_c_drive}, is_removable={}, is_system={}",
                descriptor.model,
                descriptor.bus_code,
                descriptor.bus_type,
                descriptor.removable_media_byte,
                format_hotplug_flags(hotplug_flags),
                class.is_removable,
                class.is_system,
            ));

            let DeviceDescriptor {
                model, bus_type, ..
            } = descriptor;

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
                is_removable: class.is_removable,
                is_system: class.is_system,
                bus_type,
                is_read_only,
            });
        }

        log_classification_if_changed(&system_disks, &classification_lines);

        Ok(devices)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Windows device enumeration is only available on Windows".to_string())
    }
}

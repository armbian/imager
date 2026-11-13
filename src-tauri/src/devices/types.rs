// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

//! Device types and shared helpers for block device representation.

use std::mem;

use serde::{Deserialize, Serialize};

/// Represents a block device (disk) on the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDevice {
    /// Device path (e.g., /dev/sda, /dev/disk2, \\.\PhysicalDrive1)
    pub path: String,
    /// Device name (e.g., sda, disk2)
    pub name: String,
    /// Size in bytes
    pub size: u64,
    /// Human-readable size (e.g., "32 GB")
    pub size_formatted: String,
    /// Device model/name
    pub model: String,
    /// Whether the device is removable (USB, SD card)
    pub is_removable: bool,
    /// Whether this is a system disk (contains OS)
    pub is_system: bool,
    /// Bus type (e.g., "USB", "SD", "SATA", "NVMe", "SAS")
    pub bus_type: Option<String>,
    /// Whether the device is read-only (e.g., SD card with write-protect lock)
    pub is_read_only: bool,
}

/// Normalize a platform transport/protocol string into a canonical bus type; None if empty.
pub fn normalize_bus_type(transport: &str) -> Option<String> {
    if transport.is_empty() {
        return None;
    }

    let upper = transport.to_uppercase();

    if upper.contains("SECURE DIGITAL") || upper == "SD" || upper == "MMC" {
        Some("SD".to_string())
    } else if upper.contains("USB") {
        Some("USB".to_string())
    } else if upper.contains("NVME") {
        Some("NVMe".to_string())
    } else if upper.contains("SATA") || upper == "ATA" || upper == "ATAPI" {
        Some("SATA".to_string())
    } else if upper.contains("SAS") {
        Some("SAS".to_string())
    } else {
        Some(transport.to_string())
    }
}

/// Detect an SD card from a model/media name by matching SDXC/SDHC/SD Card markers.
#[allow(dead_code)]
pub fn detect_sd_from_name(name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    if lower.contains("sdxc") || lower.contains("sdhc") || lower.contains("sd card") {
        Some("SD".to_string())
    } else {
        None
    }
}

/// Normalized bus types on which a built-in card reader can report removable media.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const REMOVABLE_MEDIA_BUSES: &[&str] = &["Unknown", "SCSI", "SATA", "RAID", "SD"];

/// Signals gathered for one Windows physical disk.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
#[derive(Debug, Clone, Copy)]
pub struct WindowsDiskSignals<'a> {
    pub bus_type: Option<&'a str>,
    pub descriptor_read: bool,
    pub media_is_removable: bool,
    pub on_windows_disk: bool,
    pub system_lookup_ok: bool,
    pub has_c_drive: bool,
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskClass {
    pub is_removable: bool,
    pub is_system: bool,
}

/// Decide removable/system for a Windows disk; anything not proven removable is system.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn classify_windows_disk(s: &WindowsDiskSignals) -> DiskClass {
    let is_usb = s.bus_type == Some("USB");
    let is_sd = s.bus_type == Some("SD");
    // None is a bus code newer than our table: fail closed.
    let bus_trusts_media_flag = s
        .bus_type
        .is_some_and(|bus| REMOVABLE_MEDIA_BUSES.contains(&bus));
    let removable_media_counts =
        s.descriptor_read && s.media_is_removable && bus_trusts_media_flag && s.system_lookup_ok;

    let is_removable = is_usb || is_sd || removable_media_counts;
    let is_system = !is_removable || s.on_windows_disk || s.has_c_drive;

    DiskClass {
        is_removable,
        is_system,
    }
}

// VOLUME_DISK_EXTENTS layout
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const DISK_EXTENTS_HEADER_SIZE: usize = 8; // u32 count + 4 padding
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const DISK_EXTENT_SIZE: usize = 24; // DISK_EXTENT: u32 + pad + u64 + u64

// Count is clamped to the returned bytes: multi-extent volumes would read out of bounds (#136).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn parse_extent_disk_numbers(buffer: &[u8], bytes_returned: u32) -> Vec<u32> {
    if buffer.len() < DISK_EXTENTS_HEADER_SIZE {
        return Vec::new();
    }
    let count = read_u32_le(buffer, 0) as usize;
    let usable = (bytes_returned as usize).min(buffer.len());
    let count = count.min(usable.saturating_sub(DISK_EXTENTS_HEADER_SIZE) / DISK_EXTENT_SIZE);

    (0..count)
        .map(|j| read_u32_le(buffer, DISK_EXTENTS_HEADER_SIZE + j * DISK_EXTENT_SIZE))
        .collect()
}

// Callers bound-check the offset first.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn read_u32_le(buffer: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        buffer[offset],
        buffer[offset + 1],
        buffer[offset + 2],
        buffer[offset + 3],
    ])
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn read_i32_le(buffer: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        buffer[offset],
        buffer[offset + 1],
        buffer[offset + 2],
        buffer[offset + 3],
    ])
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn read_u64_le(buffer: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        buffer[offset],
        buffer[offset + 1],
        buffer[offset + 2],
        buffer[offset + 3],
        buffer[offset + 4],
        buffer[offset + 5],
        buffer[offset + 6],
        buffer[offset + 7],
    ])
}

// GET_MEDIA_TYPES / DEVICE_MEDIA_INFO layout, pinned to windows-sys in windows.rs
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const MEDIA_TYPES_DEVICE_TYPE_OFFSET: usize = 0;
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const MEDIA_TYPES_COUNT_OFFSET: usize = 4;
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const MEDIA_TYPES_HEADER_SIZE: usize = 8;
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const DEVICE_MEDIA_INFO_SIZE: usize = 32;
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const MEDIA_CHARACTERISTICS_OFFSET: usize = 28;
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const FILE_DEVICE_DISK: u32 = 7;
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const MEDIA_WRITE_PROTECTED: u32 = 0x0000_0100;
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const MEDIA_CURRENTLY_MOUNTED: u32 = 0x8000_0000;

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaWriteProtection {
    pub is_read_only: bool,
    pub characteristics: Option<u32>,
}

impl MediaWriteProtection {
    const WRITABLE: Self = Self {
        is_read_only: false,
        characteristics: None,
    };
}

/// Read the write-protect bit from an IOCTL_STORAGE_GET_MEDIA_TYPES_EX reply; anything unclear is writable.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn parse_media_write_protection(buffer: &[u8], bytes_returned: u32) -> MediaWriteProtection {
    let usable = (bytes_returned as usize).min(buffer.len());
    if usable < MEDIA_TYPES_HEADER_SIZE {
        return MediaWriteProtection::WRITABLE;
    }
    // Tape replies use a different DEVICE_MEDIA_INFO member with another layout.
    if read_u32_le(buffer, MEDIA_TYPES_DEVICE_TYPE_OFFSET) != FILE_DEVICE_DISK {
        return MediaWriteProtection::WRITABLE;
    }

    let count = read_u32_le(buffer, MEDIA_TYPES_COUNT_OFFSET) as usize;
    let count = count.min((usable - MEDIA_TYPES_HEADER_SIZE) / DEVICE_MEDIA_INFO_SIZE);
    let entries: Vec<u32> = (0..count)
        .map(|j| {
            read_u32_le(
                buffer,
                MEDIA_TYPES_HEADER_SIZE + j * DEVICE_MEDIA_INFO_SIZE + MEDIA_CHARACTERISTICS_OFFSET,
            )
        })
        .collect();

    let is_protected = |c: u32| c & MEDIA_WRITE_PROTECTED != 0;
    let mounted: Vec<u32> = entries
        .iter()
        .copied()
        .filter(|c| c & MEDIA_CURRENTLY_MOUNTED != 0)
        .collect();

    let chosen = if let Some(&first_mounted) = mounted.first() {
        Some(
            mounted
                .iter()
                .copied()
                .find(|&c| is_protected(c))
                .unwrap_or(first_mounted),
        )
    } else if let [only] = entries.as_slice() {
        Some(*only)
    } else {
        None
    };

    MediaWriteProtection {
        is_read_only: chosen.is_some_and(is_protected),
        characteristics: chosen,
    }
}

// DISK_GEOMETRY_EX layout, pinned to windows-sys in windows.rs
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const DISK_GEOMETRY_MEDIA_TYPE_OFFSET: usize = 8;
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const DISK_GEOMETRY_EX_SIZE_OFFSET: usize = 24;
// Ends at DiskSize, not size_of (40): a stricter minimum would refuse the device at flash time.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const DISK_GEOMETRY_EX_MIN_SIZE: usize = DISK_GEOMETRY_EX_SIZE_OFFSET + mem::size_of::<u64>();

/// Read (MediaType, DiskSize) from an IOCTL_DISK_GET_DRIVE_GEOMETRY_EX reply.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn parse_disk_geometry_ex(buffer: &[u8], bytes_returned: u32) -> Option<(i32, u64)> {
    let usable = (bytes_returned as usize).min(buffer.len());
    if usable < DISK_GEOMETRY_EX_MIN_SIZE {
        return None;
    }
    let media_type = read_i32_le(buffer, DISK_GEOMETRY_MEDIA_TYPE_OFFSET);
    let size = read_u64_le(buffer, DISK_GEOMETRY_EX_SIZE_OFFSET);
    Some((media_type, size))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signals(bus_type: Option<&str>, media_is_removable: bool) -> WindowsDiskSignals<'_> {
        WindowsDiskSignals {
            bus_type,
            descriptor_read: true,
            media_is_removable,
            on_windows_disk: false,
            system_lookup_ok: true,
            has_c_drive: false,
        }
    }

    const REMOVABLE: DiskClass = DiskClass {
        is_removable: true,
        is_system: false,
    };
    const INTERNAL: DiskClass = DiskClass {
        is_removable: false,
        is_system: true,
    };

    #[test]
    fn pcie_reader_on_unknown_bus_is_removable() {
        assert_eq!(
            classify_windows_disk(&signals(Some("Unknown"), true)),
            REMOVABLE
        );
    }

    #[test]
    fn unmapped_bus_fails_closed_unlike_unknown_bus() {
        assert_eq!(classify_windows_disk(&signals(None, true)), INTERNAL);
        assert_eq!(classify_windows_disk(&signals(None, false)), INTERNAL);
        assert_eq!(
            classify_windows_disk(&signals(Some("Unknown"), true)),
            REMOVABLE
        );
    }

    #[test]
    fn unknown_bus_fixed_media_is_system() {
        assert_eq!(
            classify_windows_disk(&signals(Some("Unknown"), false)),
            INTERNAL
        );
    }

    #[test]
    fn nvme_is_system_whatever_the_media_flag() {
        assert_eq!(
            classify_windows_disk(&signals(Some("NVMe"), false)),
            INTERNAL
        );
        assert_eq!(
            classify_windows_disk(&signals(Some("NVMe"), true)),
            INTERNAL
        );
    }

    #[test]
    fn buses_outside_the_allowlist_ignore_the_media_flag() {
        for bus in [
            "UFS",
            "SAS",
            "Spaces",
            "Virtual",
            "FileBacked",
            "iSCSI",
            "Fibre",
            "SCM",
            "1394",
            "SSA",
            "SomethingNew",
        ] {
            assert_eq!(
                classify_windows_disk(&signals(Some(bus), true)),
                INTERNAL,
                "{bus}"
            );
        }
    }

    #[test]
    fn usb_fixed_media_stays_selectable() {
        assert_eq!(
            classify_windows_disk(&signals(Some("USB"), false)),
            DiskClass {
                is_removable: true,
                is_system: false,
            }
        );
    }

    #[test]
    fn usb_hosting_windows_is_system() {
        let s = WindowsDiskSignals {
            on_windows_disk: true,
            ..signals(Some("USB"), false)
        };
        assert!(classify_windows_disk(&s).is_system);
    }

    #[test]
    fn sd_bus_fixed_media_is_removable() {
        assert_eq!(
            classify_windows_disk(&signals(Some("SD"), false)),
            REMOVABLE
        );
    }

    #[test]
    fn c_drive_overrides_sd_bus() {
        let s = WindowsDiskSignals {
            has_c_drive: true,
            ..signals(Some("SD"), false)
        };
        assert!(classify_windows_disk(&s).is_system);
    }

    #[test]
    fn windows_disk_overrides_sd_bus() {
        let s = WindowsDiskSignals {
            on_windows_disk: true,
            ..signals(Some("SD"), false)
        };
        assert!(classify_windows_disk(&s).is_system);
    }

    #[test]
    fn c_drive_overrides_removable_media() {
        let s = WindowsDiskSignals {
            has_c_drive: true,
            ..signals(Some("Unknown"), true)
        };
        assert!(classify_windows_disk(&s).is_system);
    }

    #[test]
    fn raid_fixed_media_is_system() {
        assert_eq!(
            classify_windows_disk(&signals(Some("RAID"), false)),
            INTERNAL
        );
    }

    #[test]
    fn failed_system_lookup_disables_the_media_flag() {
        for bus in [Some("Unknown"), None, Some("SCSI")] {
            let s = WindowsDiskSignals {
                system_lookup_ok: false,
                ..signals(bus, true)
            };
            assert_eq!(classify_windows_disk(&s), INTERNAL, "{bus:?}");
        }
    }

    #[test]
    fn failed_system_lookup_keeps_usb_and_sd_rules() {
        for bus in ["USB", "SD"] {
            let s = WindowsDiskSignals {
                system_lookup_ok: false,
                ..signals(Some(bus), false)
            };
            assert_eq!(classify_windows_disk(&s), REMOVABLE, "{bus}");
        }
    }

    #[test]
    fn unread_descriptor_is_system_even_with_removable_media() {
        let s = WindowsDiskSignals {
            descriptor_read: false,
            ..signals(None, true)
        };
        assert_eq!(classify_windows_disk(&s), INTERNAL);
    }

    #[test]
    fn sata_with_removable_media_is_removable() {
        assert_eq!(
            classify_windows_disk(&signals(Some("SATA"), true)),
            REMOVABLE
        );
    }

    #[test]
    fn allowlist_matches_normalized_bus_names() {
        for raw in [
            "Unknown", "SCSI", "ATA", "ATAPI", "SATA", "RAID", "SD", "MMC",
        ] {
            let normalized = normalize_bus_type(raw).expect("non-empty");
            assert!(
                REMOVABLE_MEDIA_BUSES.contains(&normalized.as_str()),
                "{raw} -> {normalized}"
            );
        }
        for raw in ["NVMe", "NVMe-oF", "UFS", "SAS", "USB", "Spaces", "Virtual"] {
            let normalized = normalize_bus_type(raw).expect("non-empty");
            assert!(
                !REMOVABLE_MEDIA_BUSES.contains(&normalized.as_str()),
                "{raw} -> {normalized}"
            );
        }
    }

    #[test]
    fn allowlisted_bus_hosting_windows_is_system() {
        let s = WindowsDiskSignals {
            on_windows_disk: true,
            ..signals(Some("Unknown"), true)
        };
        assert!(classify_windows_disk(&s).is_system);
    }

    #[test]
    fn usb_with_c_drive_is_system() {
        let s = WindowsDiskSignals {
            has_c_drive: true,
            ..signals(Some("USB"), false)
        };
        assert!(classify_windows_disk(&s).is_system);
    }

    fn extents_buffer(count: u32, disks: &[u32]) -> Vec<u8> {
        let mut buffer = vec![0u8; DISK_EXTENTS_HEADER_SIZE + disks.len() * DISK_EXTENT_SIZE];
        buffer[..4].copy_from_slice(&count.to_le_bytes());
        for (j, disk) in disks.iter().enumerate() {
            let base = DISK_EXTENTS_HEADER_SIZE + j * DISK_EXTENT_SIZE;
            buffer[base..base + 4].copy_from_slice(&disk.to_le_bytes());
        }
        buffer
    }

    #[test]
    fn one_extent_returns_its_disk() {
        let buffer = extents_buffer(1, &[2]);
        assert_eq!(
            parse_extent_disk_numbers(&buffer, buffer.len() as u32),
            vec![2]
        );
    }

    #[test]
    fn three_extents_return_every_disk() {
        let buffer = extents_buffer(3, &[0, 4, 7]);
        assert_eq!(
            parse_extent_disk_numbers(&buffer, buffer.len() as u32),
            vec![0, 4, 7]
        );
    }

    #[test]
    fn count_beyond_returned_bytes_is_clamped() {
        let buffer = extents_buffer(5, &[1, 2, 3]);
        assert_eq!(
            parse_extent_disk_numbers(&buffer, buffer.len() as u32),
            vec![1, 2, 3]
        );
        let partial = (DISK_EXTENTS_HEADER_SIZE + DISK_EXTENT_SIZE + DISK_EXTENT_SIZE / 2) as u32;
        assert_eq!(parse_extent_disk_numbers(&buffer, partial), vec![1]);
    }

    #[test]
    fn empty_or_short_extents_buffer_returns_nothing() {
        assert!(parse_extent_disk_numbers(&[], 0).is_empty());
        assert!(parse_extent_disk_numbers(&[1, 0, 0, 0], 4).is_empty());
        let header_only = extents_buffer(3, &[]);
        assert!(parse_extent_disk_numbers(&header_only, header_only.len() as u32).is_empty());
    }

    const LOCKED: u32 = MEDIA_WRITE_PROTECTED;
    const MOUNTED: u32 = MEDIA_CURRENTLY_MOUNTED;
    // MEDIA_READ_ONLY and MEDIA_READ_WRITE must never decide the result.
    const READ_ONLY_HINT: u32 = 0x4;
    const READ_WRITE_HINT: u32 = 0x8;

    fn media_types_buffer(device_type: u32, count: u32, characteristics: &[u32]) -> Vec<u8> {
        let mut buffer =
            vec![0u8; MEDIA_TYPES_HEADER_SIZE + characteristics.len() * DEVICE_MEDIA_INFO_SIZE];
        buffer[MEDIA_TYPES_DEVICE_TYPE_OFFSET..MEDIA_TYPES_COUNT_OFFSET]
            .copy_from_slice(&device_type.to_le_bytes());
        buffer[MEDIA_TYPES_COUNT_OFFSET..MEDIA_TYPES_HEADER_SIZE]
            .copy_from_slice(&count.to_le_bytes());
        for (j, c) in characteristics.iter().enumerate() {
            let at = MEDIA_TYPES_HEADER_SIZE + j * DEVICE_MEDIA_INFO_SIZE;
            // Cylinders and geometry fields filled so a misplaced offset reads garbage.
            buffer[at..at + MEDIA_CHARACTERISTICS_OFFSET].fill(0xFF);
            buffer[at + MEDIA_CHARACTERISTICS_OFFSET..at + DEVICE_MEDIA_INFO_SIZE]
                .copy_from_slice(&c.to_le_bytes());
        }
        buffer
    }

    fn disk_reply(characteristics: &[u32]) -> Vec<u8> {
        media_types_buffer(
            FILE_DEVICE_DISK,
            characteristics.len() as u32,
            characteristics,
        )
    }

    fn parse_all(buffer: &[u8]) -> MediaWriteProtection {
        parse_media_write_protection(buffer, buffer.len() as u32)
    }

    const WRITABLE: MediaWriteProtection = MediaWriteProtection::WRITABLE;

    #[test]
    fn media_reply_shorter_than_header_is_writable() {
        let buffer = disk_reply(&[LOCKED]);
        for len in 0..MEDIA_TYPES_HEADER_SIZE {
            assert_eq!(
                parse_media_write_protection(&buffer, len as u32),
                WRITABLE,
                "{len}"
            );
            assert_eq!(parse_all(&buffer[..len]), WRITABLE, "{len}");
        }
    }

    #[test]
    fn media_header_only_is_writable() {
        let buffer = media_types_buffer(FILE_DEVICE_DISK, 1, &[]);
        assert_eq!(parse_all(&buffer), WRITABLE);
    }

    #[test]
    fn media_entry_cut_short_is_writable() {
        let buffer = disk_reply(&[LOCKED]);
        let cut = (MEDIA_TYPES_HEADER_SIZE + DEVICE_MEDIA_INFO_SIZE - 1) as u32;
        assert_eq!(parse_media_write_protection(&buffer, cut), WRITABLE);
    }

    #[test]
    fn media_count_zero_is_writable() {
        let buffer = media_types_buffer(FILE_DEVICE_DISK, 0, &[LOCKED]);
        assert_eq!(parse_all(&buffer), WRITABLE);
    }

    #[test]
    fn media_count_beyond_returned_bytes_is_clamped() {
        let buffer = media_types_buffer(FILE_DEVICE_DISK, 9, &[LOCKED]);
        assert_eq!(
            parse_all(&buffer),
            MediaWriteProtection {
                is_read_only: true,
                characteristics: Some(LOCKED),
            }
        );
        let buffer = media_types_buffer(FILE_DEVICE_DISK, 9, &[READ_WRITE_HINT]);
        assert!(!parse_all(&buffer).is_read_only);
    }

    #[test]
    fn non_disk_device_type_is_writable() {
        let tape = media_types_buffer(0x1F, 1, &[LOCKED | MOUNTED]);
        assert_eq!(parse_all(&tape), WRITABLE);
    }

    #[test]
    fn single_locked_entry_is_read_only() {
        assert_eq!(
            parse_all(&disk_reply(&[LOCKED | READ_WRITE_HINT])),
            MediaWriteProtection {
                is_read_only: true,
                characteristics: Some(LOCKED | READ_WRITE_HINT),
            }
        );
    }

    #[test]
    fn single_unlocked_entry_is_writable_whatever_the_hints() {
        assert_eq!(
            parse_all(&disk_reply(&[READ_ONLY_HINT])),
            MediaWriteProtection {
                is_read_only: false,
                characteristics: Some(READ_ONLY_HINT),
            }
        );
        assert!(!parse_all(&disk_reply(&[0])).is_read_only);
    }

    #[test]
    fn locked_entry_that_is_not_mounted_is_ignored() {
        let reply = disk_reply(&[LOCKED, MOUNTED | READ_WRITE_HINT]);
        assert_eq!(
            parse_all(&reply),
            MediaWriteProtection {
                is_read_only: false,
                characteristics: Some(MOUNTED | READ_WRITE_HINT),
            }
        );
    }

    #[test]
    fn locked_mounted_entry_among_several_is_read_only() {
        let reply = disk_reply(&[READ_WRITE_HINT, MOUNTED | LOCKED, MOUNTED]);
        assert_eq!(
            parse_all(&reply),
            MediaWriteProtection {
                is_read_only: true,
                characteristics: Some(MOUNTED | LOCKED),
            }
        );
    }

    #[test]
    fn locked_mounted_entry_after_unlocked_mounted_entry_is_read_only() {
        let reply = disk_reply(&[MOUNTED | READ_WRITE_HINT, MOUNTED | LOCKED]);
        assert_eq!(
            parse_all(&reply),
            MediaWriteProtection {
                is_read_only: true,
                characteristics: Some(MOUNTED | LOCKED),
            }
        );
    }

    #[test]
    fn several_entries_none_mounted_is_writable() {
        let reply = disk_reply(&[LOCKED, LOCKED | READ_ONLY_HINT]);
        assert_eq!(parse_all(&reply), WRITABLE);
    }

    #[test]
    fn media_bytes_returned_beyond_buffer_is_clamped() {
        let buffer = disk_reply(&[LOCKED]);
        assert!(parse_media_write_protection(&buffer, u32::MAX).is_read_only);
        let cut = &buffer[..buffer.len() - 1];
        assert_eq!(parse_media_write_protection(cut, u32::MAX), WRITABLE);
    }

    fn geometry_buffer(len: usize, media_type: i32, size: u64) -> Vec<u8> {
        let mut buffer = vec![0xAAu8; len];
        let mt = DISK_GEOMETRY_MEDIA_TYPE_OFFSET;
        let sz = DISK_GEOMETRY_EX_SIZE_OFFSET;
        if len >= sz + mem::size_of::<u64>() {
            buffer[mt..mt + mem::size_of::<i32>()].copy_from_slice(&media_type.to_le_bytes());
            buffer[sz..sz + mem::size_of::<u64>()].copy_from_slice(&size.to_le_bytes());
        }
        buffer
    }

    const REMOVABLE_MEDIA_TYPE: i32 = 11;
    const CARD_SIZE: u64 = 63_864_569_856;

    #[test]
    fn geometry_shorter_than_disk_size_end_is_none() {
        let buffer = geometry_buffer(256, REMOVABLE_MEDIA_TYPE, CARD_SIZE);
        assert_eq!(DISK_GEOMETRY_EX_MIN_SIZE, 32);
        for len in 0..DISK_GEOMETRY_EX_MIN_SIZE {
            assert_eq!(parse_disk_geometry_ex(&buffer, len as u32), None, "{len}");
            assert_eq!(
                parse_disk_geometry_ex(&buffer[..len], u32::MAX),
                None,
                "{len}"
            );
        }
    }

    #[test]
    fn geometry_of_exactly_32_bytes_parses() {
        let buffer = geometry_buffer(32, REMOVABLE_MEDIA_TYPE, CARD_SIZE);
        assert_eq!(
            parse_disk_geometry_ex(&buffer, 32),
            Some((REMOVABLE_MEDIA_TYPE, CARD_SIZE))
        );
    }

    #[test]
    fn geometry_full_buffer_reads_fields_at_their_offsets() {
        let buffer = geometry_buffer(256, -3, u64::MAX - 7);
        assert_eq!(
            parse_disk_geometry_ex(&buffer, buffer.len() as u32),
            Some((-3, u64::MAX - 7))
        );
        let buffer = geometry_buffer(256, 12, CARD_SIZE);
        assert_eq!(
            parse_disk_geometry_ex(&buffer, u32::MAX),
            Some((12, CARD_SIZE))
        );
    }
}

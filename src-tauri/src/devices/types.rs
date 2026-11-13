// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

//! Device types and shared helpers for block device representation.

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
    let count = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
    let usable = (bytes_returned as usize).min(buffer.len());
    let count = count.min(usable.saturating_sub(DISK_EXTENTS_HEADER_SIZE) / DISK_EXTENT_SIZE);

    (0..count)
        .map(|j| {
            let base = DISK_EXTENTS_HEADER_SIZE + j * DISK_EXTENT_SIZE;
            u32::from_le_bytes([
                buffer[base],
                buffer[base + 1],
                buffer[base + 2],
                buffer[base + 3],
            ])
        })
        .collect()
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
}

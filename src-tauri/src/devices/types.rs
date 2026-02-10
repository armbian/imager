//! Device types module
//!
//! Common types for block device representation.

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
    /// Bus type (e.g., "USB", "SD", "SATA", "NVMe", "MMC")
    pub bus_type: Option<String>,
    /// Whether the device is read-only (e.g., SD card with write-protect lock)
    pub is_read_only: bool,
}

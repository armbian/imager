//! Device types and shared utilities
//!
//! Common types and helpers for block device representation across platforms.

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

/// Normalize a transport/protocol string into a canonical bus type.
///
/// Handles platform-specific variations:
/// - macOS protocols: "Secure Digital", "Apple Fabric", "USB", etc.
/// - Linux transports: "usb", "mmc", "sata", "nvme", "sas"
/// - Windows bus types: "SD", "MMC", "USB", "SATA", "NVMe", "SAS"
///
/// Returns None for unknown or empty inputs.
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

/// Detect SD card from a model or media name string.
///
/// Checks for common SD card identifiers (SDXC, SDHC, SD Card)
/// that may appear in device model names across platforms.
pub fn detect_sd_from_name(name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    if lower.contains("sdxc") || lower.contains("sdhc") || lower.contains("sd card") {
        Some("SD".to_string())
    } else {
        None
    }
}

//! macOS device detection
//!
//! Uses diskutil to enumerate block devices.

use std::process::Command;

use crate::log_error;
use crate::utils::format_size;

use super::types::BlockDevice;

/// Get list of block devices on macOS
pub fn get_block_devices() -> Result<Vec<BlockDevice>, String> {
    let output = Command::new("diskutil")
        .args(["list", "-plist", "external", "physical"])
        .output()
        .map_err(|e| {
            log_error!("devices", "Failed to run diskutil: {}", e);
            format!("Failed to run diskutil: {}", e)
        })?;

    if !output.status.success() {
        // Try without external flag for older macOS
        let output = Command::new("diskutil")
            .args(["list", "-plist"])
            .output()
            .map_err(|e| {
                log_error!("devices", "Failed to run diskutil (fallback): {}", e);
                format!("Failed to run diskutil: {}", e)
            })?;

        return parse_diskutil(&output.stdout);
    }

    parse_diskutil(&output.stdout)
}

/// Parse diskutil output to extract devices
fn parse_diskutil(_plist_data: &[u8]) -> Result<Vec<BlockDevice>, String> {
    let mut devices = Vec::new();

    let output = Command::new("diskutil")
        .args(["list"])
        .output()
        .map_err(|e| {
            log_error!("devices", "Failed to run diskutil list: {}", e);
            format!("Failed to run diskutil list: {}", e)
        })?;

    let list_output = String::from_utf8_lossy(&output.stdout);
    let system_disk = get_system_disk();

    for line in list_output.lines() {
        if !line.starts_with("/dev/disk") {
            continue;
        }

        // Skip synthesized disks (APFS containers)
        if line.contains("synthesized") {
            continue;
        }

        // Skip disk images
        if line.contains("disk image") {
            continue;
        }

        let disk_path = line.split_whitespace().next().unwrap_or("");
        let disk_name = disk_path.trim_end_matches(':');

        if let Ok(mut info) = get_disk_info(disk_name) {
            // Mark as system disk if it contains the system
            if let Some(ref sys_disk) = system_disk {
                if disk_name.contains(sys_disk) || disk_name == "/dev/disk0" {
                    info.is_system = true;
                }
            }

            // Mark as system if internal and not removable
            let is_internal = line.contains("(internal");
            if is_internal && !info.is_removable {
                info.is_system = true;
            }

            if info.size > 0 {
                devices.push(info);
            }
        }
    }

    Ok(devices)
}

/// Get the system disk identifier
fn get_system_disk() -> Option<String> {
    let output = Command::new("diskutil").args(["info", "/"]).output().ok()?;

    let info = String::from_utf8_lossy(&output.stdout);
    for line in info.lines() {
        if line.contains("Part of Whole:") {
            return line.split(':').nth(1).map(|s| s.trim().to_string());
        }
    }
    None
}

/// Get detailed info for a specific disk
fn get_disk_info(disk_path: &str) -> Result<BlockDevice, String> {
    let output = Command::new("diskutil")
        .args(["info", disk_path])
        .output()
        .map_err(|e| {
            log_error!(
                "devices",
                "Failed to get disk info for {}: {}",
                disk_path,
                e
            );
            format!("Failed to get disk info: {}", e)
        })?;

    let info = String::from_utf8_lossy(&output.stdout);

    let mut size: u64 = 0;
    let mut model = String::new();
    let mut is_removable = true;
    let mut is_internal = false;
    let mut protocol = String::new();
    let mut is_read_only = false;

    for line in info.lines() {
        let line = line.trim();
        if line.starts_with("Disk Size:") {
            if let Some(bytes_part) = line.split('(').nth(1) {
                if let Some(bytes_str) = bytes_part.split_whitespace().next() {
                    size = bytes_str.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with("Device / Media Name:") {
            model = line
                .split(':')
                .nth(1)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
        } else if line.starts_with("Removable Media:") {
            is_removable = line.contains("Removable");
        } else if line.starts_with("Device Location:") {
            is_internal = line.contains("Internal");
        } else if line.starts_with("Protocol:") {
            protocol = line
                .split(':')
                .nth(1)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
        } else if line.starts_with("Media Read-Only:") {
            // Check if "Media Read-Only: Yes" - indicates write-protect lock is active
            is_read_only = line.contains("Yes");
        }
    }

    // Determine bus type from protocol and model
    let bus_type = if !protocol.is_empty() {
        // Protocol can be: USB, SATA, NVMe, Secure Digital, Apple Fabric, etc.
        let p = protocol.to_uppercase();
        if p.contains("SECURE DIGITAL") || p.contains("SD") {
            Some("SD".to_string())
        } else if p.contains("USB") {
            Some("USB".to_string())
        } else if p.contains("NVME") {
            Some("NVMe".to_string())
        } else if p.contains("SATA") {
            Some("SATA".to_string())
        } else {
            Some(protocol)
        }
    } else {
        // Fallback: detect from model name
        let m = model.to_lowercase();
        if m.contains("sdxc") || m.contains("sdhc") || m.contains("sd card") {
            Some("SD".to_string())
        } else {
            None
        }
    };

    Ok(BlockDevice {
        path: disk_path.to_string(),
        name: disk_path
            .split('/')
            .next_back()
            .unwrap_or(disk_path)
            .to_string(),
        size,
        size_formatted: format_size(size),
        model,
        is_removable,
        is_system: is_internal && !is_removable,
        bus_type,
        is_read_only,
    })
}

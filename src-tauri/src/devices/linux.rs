//! Linux device detection
//!
//! Uses lsblk to enumerate block devices.

use std::fs;
use std::process::Command;

use crate::log_error;
use crate::utils::format_size;

use super::types::BlockDevice;

/// Checks if a device is read-only by reading /sys/block/{device}/ro
/// Returns true if the device is read-only (write-protected)
fn is_device_read_only(device_name: &str) -> bool {
    // Extract base device name (e.g., "sda" from "/dev/sda" or "mmcblk0" from "/dev/mmcblk0")
    let base_name = device_name
        .trim_start_matches("/dev/")
        .split('p')
        .next()
        .unwrap_or(device_name);

    let ro_path = format!("/sys/block/{}/ro", base_name);

    fs::read_to_string(&ro_path)
        .map(|s| s.trim() == "1")
        .unwrap_or(false)
}

/// Get list of block devices on Linux
pub fn get_block_devices() -> Result<Vec<BlockDevice>, String> {
    // Use JSON output for reliable parsing (handles spaces in model names)
    let output = Command::new("lsblk")
        .args(["-dpJo", "NAME,SIZE,MODEL,RM,TRAN", "-b"])
        .output()
        .map_err(|e| {
            log_error!("devices", "Failed to run lsblk: {}", e);
            format!("Failed to run lsblk: {}", e)
        })?;

    if !output.status.success() {
        log_error!(
            "devices",
            "lsblk command failed with status: {:?}",
            output.status
        );
        return Err("lsblk command failed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();
    let system_disks = get_system_disks();

    // Parse JSON output
    let json: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| format!("Failed to parse lsblk JSON: {}", e))?;

    let blockdevices = json["blockdevices"]
        .as_array()
        .ok_or("Invalid lsblk JSON structure")?;

    for dev in blockdevices {
        let path = dev["name"].as_str().unwrap_or("");

        // Skip non-standard devices
        if !path.starts_with("/dev/sd")
            && !path.starts_with("/dev/hd")
            && !path.starts_with("/dev/vd")
            && !path.starts_with("/dev/nvme")
            && !path.starts_with("/dev/mmcblk")
        {
            continue;
        }

        // Skip mmcblk boot/rpmb partitions
        if path.contains("boot") || path.contains("rpmb") {
            continue;
        }

        let dev_name = path.strip_prefix("/dev/").unwrap_or(path);

        // Mark as system disk instead of skipping (consistent with macOS behavior)
        let is_system = system_disks
            .iter()
            .any(|sys| sys.starts_with(dev_name) || dev_name.starts_with(sys));

        // Parse size - can be string or number in JSON
        let size: u64 = match &dev["size"] {
            serde_json::Value::Number(n) => n.as_u64().unwrap_or(0),
            serde_json::Value::String(s) => s.parse().unwrap_or(0),
            _ => 0,
        };
        if size == 0 {
            continue;
        }

        let model = dev["model"].as_str().unwrap_or("").trim().to_string();

        // RM field: "1" or true means removable
        let is_removable = match &dev["rm"] {
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::String(s) => s == "1",
            serde_json::Value::Number(n) => n.as_u64() == Some(1),
            _ => false,
        };

        // Get transport type from TRAN field (already in JSON)
        let tran = dev["tran"].as_str().unwrap_or("");
        let bus_type = match tran.to_uppercase().as_str() {
            "USB" => Some("USB".to_string()),
            "MMC" => Some("SD".to_string()),
            "SATA" => Some("SATA".to_string()),
            "NVME" => Some("NVMe".to_string()),
            "SAS" => Some("SAS".to_string()),
            "" => {
                // Fallback for devices without TRAN (mmcblk, nvme)
                if path.contains("mmcblk") {
                    Some("SD".to_string())
                } else if path.contains("nvme") {
                    Some("NVMe".to_string())
                } else {
                    None
                }
            }
            other => Some(other.to_string()),
        };

        // Check read-only status via sysfs
        let is_read_only = is_device_read_only(dev_name);

        devices.push(BlockDevice {
            path: path.to_string(),
            name: dev_name.to_string(),
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

/// Get list of system disk names to exclude
fn get_system_disks() -> Vec<String> {
    let mut system_disks = Vec::new();

    for mount in &["/", "/boot", "/boot/efi"] {
        if let Ok(output) = Command::new("findmnt")
            .args(["-n", "-o", "SOURCE", mount])
            .output()
        {
            let source = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !source.is_empty() {
                if let Ok(pkname_output) = Command::new("lsblk")
                    .args(["-no", "PKNAME", &source])
                    .output()
                {
                    let pkname = String::from_utf8_lossy(&pkname_output.stdout)
                        .trim()
                        .to_string();
                    if !pkname.is_empty() {
                        system_disks.push(pkname);
                    }
                }
                if let Some(name) = source.split('/').next_back() {
                    system_disks.push(name.to_string());
                }
            }
        }
    }

    system_disks.sort();
    system_disks.dedup();
    system_disks
}

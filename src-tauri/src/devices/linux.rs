//! Linux device detection via lsblk.

use std::fs;
use std::process::Command;

use crate::log_error;
use crate::utils::format_size;

use super::types::{normalize_bus_type, BlockDevice};

/// Whether a device is write-protected, per /sys/block/{device}/ro.
fn is_device_read_only(device_name: &str) -> bool {
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
    // JSON output parses reliably even when model names contain spaces.
    let output = Command::new("lsblk")
        .args(["-dpJo", "NAME,SIZE,MODEL,RM,HOTPLUG,TRAN", "-b"])
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

    let json: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| format!("Failed to parse lsblk JSON: {}", e))?;

    let blockdevices = json["blockdevices"]
        .as_array()
        .ok_or("Invalid lsblk JSON structure")?;

    for dev in blockdevices {
        let path = dev["name"].as_str().unwrap_or("");

        // Only consider real disk device paths.
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

        // The disk backing the running root/boot mounts is always treated as system.
        let is_running_system = system_disks
            .iter()
            .any(|sys| sys.starts_with(dev_name) || dev_name.starts_with(sys));

        // lsblk JSON may encode size as a number or a string.
        let size: u64 = match &dev["size"] {
            serde_json::Value::Number(n) => n.as_u64().unwrap_or(0),
            serde_json::Value::String(s) => s.parse().unwrap_or(0),
            _ => 0,
        };
        if size == 0 {
            continue;
        }

        let model = dev["model"].as_str().unwrap_or("").trim().to_string();

        // RM is "1"/true when the device is removable.
        let is_removable = match &dev["rm"] {
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::String(s) => s == "1",
            serde_json::Value::Number(n) => n.as_u64() == Some(1),
            _ => false,
        };

        // Fallback signal for the unknown-bus case below.
        let is_hotplug = match &dev["hotplug"] {
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::String(s) => s == "1",
            serde_json::Value::Number(n) => n.as_u64() == Some(1),
            _ => false,
        };

        // Prefer the TRAN field, then infer the bus from the device path.
        let tran = dev["tran"].as_str().unwrap_or("");
        let bus_type = normalize_bus_type(tran).or_else(|| {
            if path.contains("mmcblk") {
                Some("SD".to_string())
            } else if path.contains("nvme") {
                Some("NVMe".to_string())
            } else {
                None
            }
        });

        // Internal buses are system; USB/SD stay selectable; the running OS disk always wins.
        let is_system = match bus_type.as_deref() {
            Some("USB") | Some("SD") => false,
            Some(_) => true,
            None => !(is_removable || is_hotplug),
        } || is_running_system;

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

/// Block device names backing the root and boot mounts.
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

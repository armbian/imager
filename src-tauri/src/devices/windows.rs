//! Windows device detection
//!
//! Uses PowerShell Get-Disk to enumerate block devices.

use std::process::Command;
use std::collections::HashMap;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::utils::format_size;
use crate::log_error;

use super::types::BlockDevice;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// PowerShell script that gets all disk info in a single call
const PS_SCRIPT: &str = r#"
$disks = Get-Disk | Select-Object Number, FriendlyName, Size, BusType
$partitions = Get-Partition | Where-Object { $_.DriveLetter } | Select-Object DiskNumber, DriveLetter
$systemDisk = (Get-Partition -DriveLetter C -ErrorAction SilentlyContinue | Get-Disk -ErrorAction SilentlyContinue).Number

$result = @{
    Disks = $disks
    Partitions = $partitions
    SystemDisk = $systemDisk
}
$result | ConvertTo-Json -Depth 3
"#;

/// Get list of block devices on Windows
pub fn get_block_devices() -> Result<Vec<BlockDevice>, String> {
    #[cfg(target_os = "windows")]
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", PS_SCRIPT])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| {
            log_error!("devices", "Failed to run PowerShell: {}", e);
            format!("Failed to run PowerShell: {}", e)
        })?;

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", PS_SCRIPT])
        .output()
        .map_err(|e| {
            log_error!("devices", "Failed to run PowerShell: {}", e);
            format!("Failed to run PowerShell: {}", e)
        })?;

    if !output.status.success() {
        log_error!("devices", "PowerShell command failed with status: {:?}", output.status);
        return Err("PowerShell command failed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let json: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
        log_error!("devices", "Failed to parse disk info JSON: {}", e);
        format!("Failed to parse disk info: {}", e)
    })?;

    // Get system disk number
    let system_disk = json["SystemDisk"].as_i64();

    // Build a map of disk number -> drive letters
    let mut drive_letters_map: HashMap<i64, Vec<String>> = HashMap::new();
    if let Some(partitions) = json["Partitions"].as_array() {
        for part in partitions {
            let disk_num = part["DiskNumber"].as_i64().unwrap_or(-1);
            let letter = part["DriveLetter"].as_str().unwrap_or("");
            if disk_num >= 0 && !letter.is_empty() {
                drive_letters_map
                    .entry(disk_num)
                    .or_default()
                    .push(format!("{}:", letter));
            }
        }
    }

    // Parse disks
    let mut devices = Vec::new();

    let disks_value = &json["Disks"];
    let disks: Vec<serde_json::Value> = if disks_value.is_array() {
        disks_value.as_array().unwrap().clone()
    } else if disks_value.is_object() {
        vec![disks_value.clone()]
    } else {
        return Ok(devices);
    };

    for disk in disks {
        let number = disk["Number"].as_i64().unwrap_or(-1);
        if number < 0 {
            continue;
        }

        let size = disk["Size"].as_u64().unwrap_or(0);
        if size == 0 {
            continue;
        }

        let model = disk["FriendlyName"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();

        let bus_type_str = disk["BusType"].as_str().unwrap_or("");
        let is_removable = bus_type_str == "USB" || bus_type_str == "SD";

        // Mark as system disk (consistent with macOS/Linux behavior)
        let is_system = system_disk.map(|sys_num| number == sys_num).unwrap_or(false);

        // Get drive letters from our pre-built map
        let name = match drive_letters_map.get(&number) {
            Some(letters) if !letters.is_empty() => {
                format!("Disk {} ({})", number, letters.join(", "))
            }
            _ => format!("Disk {}", number),
        };

        devices.push(BlockDevice {
            path: format!("\\\\.\\PhysicalDrive{}", number),
            name,
            size,
            size_formatted: format_size(size),
            model,
            is_removable,
            is_system,
            bus_type: if bus_type_str.is_empty() {
                None
            } else {
                Some(bus_type_str.to_string())
            },
        });
    }

    Ok(devices)
}

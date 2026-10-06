// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Backend validation of the device the frontend asks to write to.

use super::BlockDevice;

/// A block device that passed `select_flash_target`; writers accept nothing else.
#[derive(Debug)]
pub struct FlashTarget {
    path: String,
    size: u64,
}

impl FlashTarget {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}

pub(crate) const TAG_INVALID_PATH: &str = "[DEVICE_INVALID_PATH]";
pub(crate) const TAG_NOT_FOUND: &str = "[DEVICE_NOT_FOUND]";
pub(crate) const TAG_CHANGED: &str = "[DEVICE_CHANGED]";
pub(crate) const TAG_READ_ONLY: &str = "[DEVICE_READ_ONLY]";
pub(crate) const TAG_SYSTEM_BLOCKED: &str = "[DEVICE_SYSTEM_BLOCKED]";

/// Why a requested target was refused, as the tagged string the frontend maps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetRefusal {
    /// The only refusal worth a rescan.
    NotFound(String),
    Refused(String),
}

impl TargetRefusal {
    pub fn into_message(self) -> String {
        match self {
            Self::NotFound(msg) | Self::Refused(msg) => msg,
        }
    }
}

/// Shared by the scan check and the writers' post-open size checks.
pub(crate) fn device_changed_error(path: &str, expected: u64, actual: u64) -> String {
    format!("{TAG_CHANGED} {path:?} is now {actual} bytes, {expected} bytes were selected")
}

/// Write guards for a device already matched by path: size, write protection, system disk.
pub(crate) fn check_target(
    dev: &BlockDevice,
    expected_size: u64,
    allow_system: bool,
) -> Result<(), String> {
    let path = &dev.path;
    if expected_size == 0 {
        return Err(format!(
            "{TAG_INVALID_PATH} {path:?}: no device size was selected"
        ));
    }
    if dev.size != expected_size {
        return Err(device_changed_error(path, expected_size, dev.size));
    }
    if dev.is_read_only {
        return Err(format!("{TAG_READ_ONLY} {path:?} is write-protected"));
    }
    if dev.is_system && !allow_system {
        return Err(format!(
            "{TAG_SYSTEM_BLOCKED} {path:?} is a system disk and system devices are not allowed"
        ));
    }
    Ok(())
}

fn is_canonical_decimal(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s == "0" || !s.starts_with('0'))
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn is_linux_device_path(path: &str) -> bool {
    path.strip_prefix("/dev/")
        .is_some_and(|name| !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric()))
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn is_macos_device_path(path: &str) -> bool {
    path.strip_prefix("/dev/disk")
        .is_some_and(is_canonical_decimal)
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn is_windows_device_path(path: &str) -> bool {
    path.strip_prefix(r"\\.\PhysicalDrive")
        .is_some_and(is_canonical_decimal)
}

fn is_device_path_form(path: &str) -> bool {
    #[cfg(target_os = "linux")]
    return is_linux_device_path(path);
    #[cfg(target_os = "macos")]
    return is_macos_device_path(path);
    #[cfg(target_os = "windows")]
    return is_windows_device_path(path);
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return {
        let _ = path;
        false
    };
}

/// Match `path` byte-for-byte against a fresh scan and apply the write guards.
pub fn select_flash_target(
    path: &str,
    expected_size: u64,
    devices: &[BlockDevice],
    allow_system: bool,
) -> Result<FlashTarget, TargetRefusal> {
    if !is_device_path_form(path) {
        return Err(TargetRefusal::Refused(format!(
            "{TAG_INVALID_PATH} {path:?}: not a whole-disk device path"
        )));
    }

    let matches: Vec<&BlockDevice> = devices.iter().filter(|d| d.path == path).collect();
    let device = match matches.as_slice() {
        [] => {
            return Err(TargetRefusal::NotFound(format!(
                "{TAG_NOT_FOUND} {path:?} is not a detected device"
            )))
        }
        [device] => *device,
        many => {
            return Err(TargetRefusal::Refused(format!(
                "{TAG_INVALID_PATH} {path:?} matches {} detected devices",
                many.len()
            )))
        }
    };

    check_target(device, expected_size, allow_system).map_err(TargetRefusal::Refused)?;

    Ok(FlashTarget {
        path: device.path.clone(),
        size: device.size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    const HOST_PATH: &str = "/dev/sdb";
    #[cfg(target_os = "macos")]
    const HOST_PATH: &str = "/dev/disk4";
    #[cfg(target_os = "windows")]
    const HOST_PATH: &str = r"\\.\PhysicalDrive1";

    const SIZE: u64 = 31_914_983_424;

    fn device(path: &str, size: u64, is_system: bool, is_read_only: bool) -> BlockDevice {
        BlockDevice {
            path: path.to_string(),
            name: "test".to_string(),
            size,
            size_formatted: String::new(),
            model: "Test".to_string(),
            is_removable: !is_system,
            is_system,
            bus_type: Some(if is_system { "NVMe" } else { "USB" }.to_string()),
            is_read_only,
        }
    }

    fn removable() -> Vec<BlockDevice> {
        vec![device(HOST_PATH, SIZE, false, false)]
    }

    #[test]
    fn linux_path_forms() {
        assert!(is_linux_device_path("/dev/sdb"));
        assert!(is_linux_device_path("/dev/mmcblk0"));
        assert!(is_linux_device_path("/dev/nvme0n1"));
        assert!(is_linux_device_path("/dev/sdb1"));
        assert!(!is_linux_device_path("/dev/"));
        assert!(!is_linux_device_path("/dev/../etc/passwd"));
        assert!(!is_linux_device_path("/dev/disk/by-id/usb-x"));
        assert!(!is_linux_device_path("/dev/sdb "));
        assert!(!is_linux_device_path("/dev/sdb\n"));
        assert!(!is_linux_device_path("/dev/sd\0b"));
        assert!(!is_linux_device_path("/tmp/x"));
        assert!(!is_linux_device_path(""));
    }

    #[test]
    fn macos_path_forms() {
        assert!(is_macos_device_path("/dev/disk2"));
        assert!(is_macos_device_path("/dev/disk0"));
        assert!(!is_macos_device_path("/dev/rdisk2"));
        assert!(!is_macos_device_path("/dev/disk2s1"));
        assert!(!is_macos_device_path("/dev/disk02"));
        assert!(!is_macos_device_path("/dev/disk"));
        assert!(!is_macos_device_path("/dev/disk2 "));
        assert!(!is_macos_device_path("/dev/disk2\n"));
        assert!(!is_macos_device_path("/dev/disk\u{0}2"));
        assert!(!is_macos_device_path(""));
    }

    #[test]
    fn windows_path_forms() {
        assert!(is_windows_device_path(r"\\.\PhysicalDrive1"));
        assert!(is_windows_device_path(r"\\.\PhysicalDrive0"));
        assert!(!is_windows_device_path(r"\\?\PhysicalDrive1"));
        assert!(!is_windows_device_path(r"\\.\physicaldrive1"));
        assert!(!is_windows_device_path(r"\\.\PhysicalDrive01"));
        assert!(!is_windows_device_path(r"\\.\PhysicalDrive"));
        assert!(!is_windows_device_path("\\\\.\\PhysicalDrive1 "));
        assert!(!is_windows_device_path("\\\\.\\PhysicalDrive1\n"));
        assert!(!is_windows_device_path("\\\\.\\PhysicalDrive\u{0}1"));
        assert!(!is_windows_device_path(""));
    }

    #[test]
    fn accepts_detected_removable_device() {
        let target = select_flash_target(HOST_PATH, SIZE, &removable(), false).unwrap();
        assert_eq!(target.path(), HOST_PATH);
        assert_eq!(target.size(), SIZE);
    }

    #[test]
    fn refuses_paths_that_are_not_byte_identical() {
        let devices = removable();
        for path in [
            format!("{HOST_PATH} "),
            format!("{HOST_PATH}\n"),
            format!(" {HOST_PATH}"),
            HOST_PATH.to_uppercase(),
            String::new(),
            "devsim://DevSim-1".to_string(),
            "qdl://1/5".to_string(),
            "/dev/sdb1".to_string(),
            "/dev/rdisk4".to_string(),
            "/dev/disk4s1".to_string(),
            r"\\?\PhysicalDrive1".to_string(),
            r"\\.\physicaldrive1".to_string(),
            r"\\.\PhysicalDrive01".to_string(),
            format!("{HOST_PATH}\0"),
        ] {
            assert!(
                select_flash_target(&path, SIZE, &devices, true).is_err(),
                "{path:?} was accepted"
            );
        }
    }

    fn refused_with(devices: &[BlockDevice], expected_size: u64, allow: bool) -> String {
        match select_flash_target(HOST_PATH, expected_size, devices, allow).unwrap_err() {
            TargetRefusal::Refused(msg) => msg,
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    #[test]
    fn refuses_undetected_device_as_not_found() {
        let refusal = select_flash_target(HOST_PATH, SIZE, &[], true).unwrap_err();
        assert!(matches!(&refusal, TargetRefusal::NotFound(m) if m.starts_with(TAG_NOT_FOUND)));
    }

    #[test]
    fn refuses_zero_expected_size() {
        assert!(refused_with(&removable(), 0, true).starts_with(TAG_INVALID_PATH));
    }

    #[test]
    fn refuses_size_mismatch() {
        let msg = refused_with(&removable(), SIZE - 512, true);
        assert_eq!(msg, device_changed_error(HOST_PATH, SIZE - 512, SIZE));
        assert!(msg.starts_with(TAG_CHANGED));
    }

    #[test]
    fn refuses_read_only_even_when_system_allowed() {
        let devices = vec![device(HOST_PATH, SIZE, false, true)];
        assert!(refused_with(&devices, SIZE, true).starts_with(TAG_READ_ONLY));
    }

    #[test]
    fn refuses_system_disk_unless_allowed() {
        let devices = vec![device(HOST_PATH, SIZE, true, false)];
        assert!(refused_with(&devices, SIZE, false).starts_with(TAG_SYSTEM_BLOCKED));
        assert!(select_flash_target(HOST_PATH, SIZE, &devices, true).is_ok());
    }

    #[test]
    fn refuses_path_matching_several_devices() {
        let devices = vec![
            device(HOST_PATH, SIZE, false, false),
            device(HOST_PATH, SIZE, false, false),
        ];
        assert!(refused_with(&devices, SIZE, true).starts_with(TAG_INVALID_PATH));
    }

    #[test]
    fn check_target_applies_guards_without_path_form() {
        let dev = device("devsim://sd", SIZE, false, false);
        assert!(check_target(&dev, SIZE, false).is_ok());
        assert!(check_target(&dev, 0, true).is_err());
        let ro = device("devsim://ro", SIZE, false, true);
        assert!(check_target(&ro, SIZE, true)
            .unwrap_err()
            .starts_with(TAG_READ_ONLY));
        let sys = device("devsim://nvme", SIZE, true, false);
        assert!(check_target(&sys, SIZE, false)
            .unwrap_err()
            .starts_with(TAG_SYSTEM_BLOCKED));
        assert!(check_target(&sys, SIZE, true).is_ok());
    }
}

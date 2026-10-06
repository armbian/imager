// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

//! Platform-specific image flashing: privilege escalation + raw device writing.
//! macOS uses authopen (Touch ID), Linux uses pkexec, Windows needs Administrator.

mod verify;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::Mutex;

#[cfg(target_os = "windows")]
use crate::devices::FlashTarget;

/// QDL (Qualcomm EDL) progress state. Uses `std::sync::Mutex` because `qdl_flash`
/// runs in `spawn_blocking`.
pub struct QdlProgress {
    pub is_active: AtomicBool,
    /// Current stage (e.g., "sahara", "firehose", "partition:boot.img")
    pub stage: std::sync::Mutex<String>,
    pub partitions_total: AtomicU64,
    pub partitions_written: AtomicU64,
}

impl QdlProgress {
    pub fn new() -> Self {
        Self {
            is_active: AtomicBool::new(false),
            stage: std::sync::Mutex::new(String::new()),
            partitions_total: AtomicU64::new(0),
            partitions_written: AtomicU64::new(0),
        }
    }

    pub fn reset(&self) {
        self.is_active.store(false, Ordering::SeqCst);
        {
            let mut s = self.stage.lock().unwrap_or_else(|p| p.into_inner());
            *s = String::new();
        }
        self.partitions_total.store(0, Ordering::SeqCst);
        self.partitions_written.store(0, Ordering::SeqCst);
    }
}

/// Flash progress state shared between frontend and backend
pub struct FlashState {
    pub total_bytes: AtomicU64,
    pub written_bytes: AtomicU64,
    pub verified_bytes: AtomicU64,
    pub is_verifying: AtomicBool,
    pub is_cancelled: AtomicBool,
    pub error: Mutex<Option<String>>,
    pub qdl: QdlProgress,
}

impl FlashState {
    pub fn new() -> Self {
        Self {
            total_bytes: AtomicU64::new(0),
            written_bytes: AtomicU64::new(0),
            verified_bytes: AtomicU64::new(0),
            is_verifying: AtomicBool::new(false),
            is_cancelled: AtomicBool::new(false),
            error: Mutex::new(None),
            qdl: QdlProgress::new(),
        }
    }

    pub fn reset(&self) {
        self.total_bytes.store(0, Ordering::SeqCst);
        self.written_bytes.store(0, Ordering::SeqCst);
        self.verified_bytes.store(0, Ordering::SeqCst);
        self.is_verifying.store(false, Ordering::SeqCst);
        self.is_cancelled.store(false, Ordering::SeqCst);
        self.qdl.reset();
    }
}

#[cfg(target_os = "linux")]
pub use linux::flash_image;
#[cfg(target_os = "macos")]
pub use macos::flash_image;
#[cfg(target_os = "windows")]
pub use windows::flash_image;

#[cfg(target_os = "linux")]
pub use linux::request_authorization;
#[cfg(target_os = "macos")]
pub use macos::request_authorization;

/// Request authorization before flashing: Touch ID on macOS, pkexec re-launch
/// on Linux when not root, no-op on Windows.
#[cfg(target_os = "windows")]
pub fn request_authorization(target: &FlashTarget) -> Result<bool, String> {
    reject_simulated(target.path())?;
    Ok(true)
}

pub(crate) fn is_simulated_path(path: &str) -> bool {
    path.trim()
        .to_ascii_lowercase()
        .starts_with(crate::config::flash::SIMULATED_DEVICE_PREFIX)
}

/// Called again at every writer entry so a simulated path can never reach real device I/O.
pub(crate) fn reject_simulated(path: &str) -> Result<(), String> {
    if is_simulated_path(path) {
        return Err(format!(
            "{} simulated device path refused: {path:?}",
            crate::devices::TAG_INVALID_PATH
        ));
    }
    Ok(())
}

pub(crate) fn check_capacity(image_size: u64, device_size: u64) -> Result<(), String> {
    // Device sizes are whole sectors, so this also covers the sector padding of the last write.
    if device_size == 0 || image_size > device_size {
        return Err(format!(
            "[DEVICE_TOO_SMALL:{image_size}:{device_size}] image needs {image_size} bytes, device has {device_size}"
        ));
    }
    Ok(())
}

/// Unmount a device before flashing (platform-specific)
#[allow(dead_code)]
pub(crate) fn unmount_device(device_path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("diskutil")
            .args(["unmountDisk", device_path])
            .output();
    }

    #[cfg(target_os = "linux")]
    {
        let output = Command::new("lsblk")
            .args(["-ln", "-o", "NAME", device_path])
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let part_path = format!("/dev/{}", line.trim());
                let _ = Command::new("umount").arg(&part_path).output();
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let _ = device_path;
    }

    Ok(())
}

/// Tagged device-write failure; the frontend maps `[WRITE_FAILED:<offset>]` to a translated message.
pub(crate) fn write_failed_err(offset: u64, e: impl std::fmt::Display) -> String {
    format!("[WRITE_FAILED:{}] {}", offset, e)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn fsync_checked(fd: i32, written: u64) -> Result<(), String> {
    if unsafe { libc::fsync(fd) } != 0 {
        return Err(write_failed_err(written, std::io::Error::last_os_error()));
    }
    Ok(())
}

/// Write `total` zero bytes in `chunk_size` chunks, shared by the platform
/// quick_erase routines (which keep their own seek/sync).
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn write_zeros(
    device: &mut impl std::io::Write,
    total: usize,
    chunk_size: usize,
) -> Result<(), String> {
    let zero_buffer = vec![0u8; chunk_size];
    let mut erased: usize = 0;
    while erased < total {
        let to_write = std::cmp::min(chunk_size, total - erased);
        device
            .write_all(&zero_buffer[..to_write])
            .map_err(|e| format!("Quick erase failed at byte {}: {}", erased, e))?;
        erased += to_write;
    }
    Ok(())
}

/// Sync device to ensure all data is written to disk
#[allow(dead_code)]
pub(crate) fn sync_device(_device_path: &str) {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let _ = Command::new("sync").output();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_simulated_matches_any_case_and_padding() {
        for path in [
            "devsim://sd-32g",
            "DEVSIM://sd-32g",
            "DevSim://x",
            "  devsim://x ",
            "\tdevsim://x\n",
            "devsim://",
        ] {
            assert!(reject_simulated(path).is_err(), "{path:?} was accepted");
        }
        assert!(reject_simulated("devsim://x")
            .unwrap_err()
            .starts_with(crate::devices::TAG_INVALID_PATH));
    }

    #[test]
    fn reject_simulated_lets_real_paths_through() {
        for path in [
            "/dev/disk4",
            "/dev/sdb",
            r"\\.\PhysicalDrive1",
            "qdl://1/5",
            "",
            "/dev/devsim",
        ] {
            assert!(reject_simulated(path).is_ok(), "{path:?} was refused");
        }
    }

    #[test]
    fn capacity_allows_exact_fit() {
        assert!(check_capacity(1024, 1024).is_ok());
        assert!(check_capacity(0, 1024).is_ok());
    }

    #[test]
    fn capacity_refuses_one_byte_over() {
        let err = check_capacity(1025, 1024).unwrap_err();
        assert!(err.starts_with("[DEVICE_TOO_SMALL:1025:1024]"));
    }

    #[test]
    fn capacity_refuses_unknown_device_size() {
        assert!(check_capacity(1, 0)
            .unwrap_err()
            .starts_with("[DEVICE_TOO_SMALL:1:0]"));
    }
}

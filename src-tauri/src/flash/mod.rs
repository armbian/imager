//! Flash module - Platform-specific image flashing implementations
//!
//! This module provides privilege escalation and raw device writing for each platform:
//! - macOS: Uses authopen with Touch ID support
//! - Linux: Uses pkexec for privilege escalation
//! - Windows: Requires running as Administrator

mod verify;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::Mutex;

/// Flash progress state shared between frontend and backend
pub struct FlashState {
    pub total_bytes: AtomicU64,
    pub written_bytes: AtomicU64,
    pub verified_bytes: AtomicU64,
    pub is_verifying: AtomicBool,
    pub is_cancelled: AtomicBool,
    pub error: Mutex<Option<String>>,
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
        }
    }

    pub fn reset(&self) {
        self.total_bytes.store(0, Ordering::SeqCst);
        self.written_bytes.store(0, Ordering::SeqCst);
        self.verified_bytes.store(0, Ordering::SeqCst);
        self.is_verifying.store(false, Ordering::SeqCst);
        self.is_cancelled.store(false, Ordering::SeqCst);
    }
}

// Re-export the platform-specific flash_image function
#[cfg(target_os = "macos")]
pub use macos::flash_image;
#[cfg(target_os = "linux")]
pub use linux::flash_image;
#[cfg(target_os = "windows")]
pub use windows::flash_image;

// Re-export authorization functions
#[cfg(target_os = "macos")]
pub use macos::request_authorization;
#[cfg(target_os = "linux")]
pub use linux::request_authorization;

/// Request authorization before flashing (platform-specific)
/// On macOS: Shows Touch ID / password dialog
/// On Linux: If not root, launches pkexec and restarts the app elevated
/// On Windows: No-op (authorization happens during flash)
#[cfg(target_os = "windows")]
pub fn request_authorization(_device_path: &str) -> Result<bool, String> {
    Ok(true)
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

/// Sync device to ensure all data is written to disk
#[allow(dead_code)]
pub(crate) fn sync_device(_device_path: &str) {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let _ = Command::new("sync").output();
    }
}

//! Platform-specific block device detection.

mod types;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

pub use types::BlockDevice;

#[cfg(target_os = "macos")]
pub use macos::get_block_devices;

#[cfg(target_os = "linux")]
pub use linux::get_block_devices;

#[cfg(target_os = "windows")]
pub use windows::get_block_devices;

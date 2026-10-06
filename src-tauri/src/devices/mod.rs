// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

//! Platform-specific block device detection.

pub(crate) mod target;
mod types;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

pub(crate) use target::{device_changed_error, TAG_INVALID_PATH};
pub use target::{select_flash_target, FlashTarget, TargetRefusal};
pub use types::BlockDevice;

#[cfg(target_os = "macos")]
pub use macos::get_block_devices;

#[cfg(target_os = "linux")]
pub use linux::get_block_devices;

#[cfg(target_os = "windows")]
pub use windows::get_block_devices;

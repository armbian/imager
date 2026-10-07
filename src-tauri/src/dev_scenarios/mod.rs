// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Debug-only emulator of devices, flash outcomes and network faults; release refuses every `devsim://` path.

pub mod devices;
pub mod flash_sim;
pub mod fs;
pub mod model;
pub mod network;
pub mod presets;
pub mod state;
pub mod test_image;
#[cfg(target_os = "macos")]
pub mod vdisk;

pub const MODULE: &str = "dev_scenarios";

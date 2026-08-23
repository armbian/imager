// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Armbian and Armbian Imager contributors

//! Linux-specific flash implementation. Uses UDisks2 (polkit) for device
//! access, falling back to a direct root open.

mod privileges;
mod writer;

pub use privileges::request_authorization;
pub use writer::flash_image;

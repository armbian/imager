// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! QDL (Qualcomm Device Loader): flashing for boards using Qualcomm EDL (Emergency Download) mode instead of block-device
//! writes. Uses the Sahara protocol to upload a firehose programmer, then the Firehose protocol to program partitions.

pub mod boards;
pub mod detect;
pub mod extract;
pub mod flash;
pub mod loader;
pub mod provision;
pub mod registry;

use serde::{Deserialize, Serialize};

/// USB Vendor ID for Qualcomm devices
pub const QUALCOMM_VID: u16 = 0x05c6;

/// USB Product ID for EDL (Emergency Download) mode
pub const EDL_PID: u16 = 0x9008;

/// USB Product ID for Qualcomm ramdump mode, which qdlrs also connects to
pub const EDL_RAMDUMP_PID: u16 = 0x900e;

/// Scheme of the device path identifying one EDL device (`qdl://<bus_id>/<address>`)
pub const QDL_PATH_PREFIX: &str = "qdl://";

pub const TAG_QDL_DEVICE_NOT_FOUND: &str = "[QDL_DEVICE_NOT_FOUND]";
pub const TAG_QDL_MULTIPLE_DEVICES: &str = "[QDL_MULTIPLE_DEVICES]";
pub const TAG_QDL_DISCONNECTED: &str = "[QDL_DISCONNECTED]";
pub const TAG_QDL_CANCELLED: &str = "[QDL_CANCELLED]";
pub const TAG_QDL_ERROR: &str = "[QDL_ERROR]";

pub const QDL_CANCELLED_ERROR: &str = "QDL flash cancelled by user";

/// `FlashState.qdl.stage` values; src/hooks/useFlashOperation.ts matches them.
pub const STAGE_CONNECTING: &str = "connecting";
pub const STAGE_SAHARA: &str = "sahara";
pub const STAGE_CONFIGURING: &str = "configuring";
pub const STAGE_PROVISIONING: &str = "provisioning";
pub const STAGE_FIREHOSE: &str = "firehose";
pub const STAGE_PATCHING: &str = "patching";
pub const STAGE_RESETTING: &str = "resetting";
pub const STAGE_COMPLETE: &str = "complete";
pub const STAGE_PARTITION_PREFIX: &str = "partition:";

pub const UFS_PARTITION_LABEL: &str = "system";
pub const TAG_QDL_AUTOCONFIG_FAILED: &str = "[QDL_AUTOCONFIG_FAILED]";

pub const SECTOR_SIZE_EMMC: usize = 512;
pub const SECTOR_SIZE_UFS: usize = 4096;

/// Filename variant markers identifying a UFS build (vs the SD variant of the same board).
pub const UFS_MARKERS: [&str; 2] = ["-ufs", "_ufs"];

/// Storage backend for a QDL flash, mapping to its Firehose storage type and sector size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QdlStorage {
    Emmc,
    Ufs,
}

impl QdlStorage {
    /// Parse the API/bundle `storage` string into a backend we can write. Unknown
    /// values yield `None`.
    pub fn from_storage_str(s: &str) -> Option<Self> {
        match s {
            "ufs" => Some(QdlStorage::Ufs),
            "emmc" => Some(QdlStorage::Emmc),
            _ => None,
        }
    }

    pub fn sector_size(self) -> usize {
        match self {
            QdlStorage::Emmc => SECTOR_SIZE_EMMC,
            QdlStorage::Ufs => SECTOR_SIZE_UFS,
        }
    }

    pub fn firehose_type(self) -> qdl::types::FirehoseStorageType {
        match self {
            QdlStorage::Emmc => qdl::types::FirehoseStorageType::Emmc,
            QdlStorage::Ufs => qdl::types::FirehoseStorageType::Ufs,
        }
    }
}

/// True when this build has a working QDL write path for the given `storage`.
/// The flash pipeline is generic (Sahara + Firehose, blobs fetched from the API),
/// so the only thing that gates a board is its storage backend: each needs a
/// sector size and write path we implement explicitly. An unmapped value has no
/// write path, so unknown => unsupported.
pub fn qdl_storage_supported(storage: &str) -> bool {
    QdlStorage::from_storage_str(storage).is_some()
}

/// Represents a Qualcomm device in EDL mode detected via USB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdlDevice {
    /// Device path the frontend passes back to target this device
    pub path: String,
    /// USB serial number (may be empty on some devices)
    pub serial: String,
    /// USB bus identifier (platform-specific format)
    pub bus_id: String,
    /// USB device address on the bus
    pub device_address: u8,
    /// Human-readable description
    pub description: String,
}

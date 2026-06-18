//! QDL (Qualcomm Device Loader): flashing for boards using Qualcomm EDL (Emergency Download) mode instead of block-device
//! writes. Uses the Sahara protocol to upload a firehose programmer, then the Firehose protocol to program partitions.

pub mod boards;
pub mod detect;
pub mod extract;
pub mod flash;
pub mod loader;
pub mod provision;

use serde::{Deserialize, Serialize};

/// USB Vendor ID for Qualcomm devices
pub const QUALCOMM_VID: u16 = 0x05c6;

/// USB Product ID for EDL (Emergency Download) mode
pub const EDL_PID: u16 = 0x9008;

pub const SECTOR_SIZE_EMMC: usize = 512;
pub const SECTOR_SIZE_UFS: usize = 4096;

/// Storage backend for a QDL flash, mapping to its Firehose storage type and sector size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QdlStorage {
    Emmc,
    Ufs,
}

impl QdlStorage {
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

/// Represents a Qualcomm device in EDL mode detected via USB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdlDevice {
    /// USB serial number (may be empty on some devices)
    pub serial: String,
    /// USB bus identifier (platform-specific format)
    pub bus_id: String,
    /// USB device address on the bus
    pub device_address: u8,
    /// Human-readable description
    pub description: String,
}

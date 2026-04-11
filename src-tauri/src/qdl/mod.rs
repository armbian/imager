//! QDL (Qualcomm Device Loader) module
//!
//! Provides QDL-based flashing for boards that use Qualcomm EDL
//! (Emergency Download) mode instead of block-device writes.
//! Uses the Sahara protocol to upload a firehose programmer,
//! then the Firehose protocol to program partitions.

pub mod detect;
pub mod extract;
pub mod flash;

use serde::{Deserialize, Serialize};

/// USB Vendor ID for Qualcomm devices
pub const QUALCOMM_VID: u16 = 0x05c6;

/// USB Product ID for EDL (Emergency Download) mode
pub const EDL_PID: u16 = 0x9008;

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

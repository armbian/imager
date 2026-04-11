//! QDL device detection via USB enumeration
//!
//! Scans for Qualcomm devices in EDL mode (VID 0x05c6, PID 0x9008)
//! using the nusb pure-Rust USB library. Cross-platform: Linux (usbfs),
//! macOS (IOKit), Windows (WinUSB).

use nusb::MaybeFuture;

use super::{QdlDevice, EDL_PID, QUALCOMM_VID};
use crate::log_debug;

/// Detect all Qualcomm EDL devices currently connected via USB
///
/// Returns a list of QDL devices matching the Qualcomm EDL USB identifiers.
/// Each device represents a board in Emergency Download mode ready for flashing.
pub fn get_qdl_devices() -> Result<Vec<QdlDevice>, String> {
    let devices: Vec<QdlDevice> = nusb::list_devices()
        .wait()
        .map_err(|e| format!("Failed to enumerate USB devices: {}", e))?
        .filter(|dev| dev.vendor_id() == QUALCOMM_VID && dev.product_id() == EDL_PID)
        .map(|dev| {
            let serial = dev.serial_number().unwrap_or("").to_string();
            let bus_id = dev.bus_id().to_string();
            let addr = dev.device_address();

            QdlDevice {
                serial,
                bus_id: bus_id.clone(),
                device_address: addr,
                description: format!("Qualcomm EDL Device (Bus {} Addr {})", bus_id, addr),
            }
        })
        .collect();

    log_debug!("qdl::detect", "Found {} EDL device(s)", devices.len());

    Ok(devices)
}

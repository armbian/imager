// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! QDL device detection: scans for Qualcomm EDL-mode devices (VID 0x05c6, PID 0x9008) via the nusb
//! pure-Rust USB library. Cross-platform: Linux (usbfs), macOS (IOKit), Windows (WinUSB).

use nusb::MaybeFuture;

use super::{
    QdlDevice, EDL_PID, EDL_RAMDUMP_PID, QDL_PATH_PREFIX, QUALCOMM_VID, TAG_QDL_DEVICE_NOT_FOUND,
    TAG_QDL_MULTIPLE_DEVICES,
};
use crate::devices::TAG_INVALID_PATH;
use crate::log_debug;

const PRODUCT_SN_MARKER: &str = "_SN:";

/// Device path identifying one EDL device; matched byte-for-byte when flashing.
pub fn qdl_device_path(bus_id: &str, device_address: u8) -> String {
    format!("{QDL_PATH_PREFIX}{bus_id}/{device_address}")
}

/// Detect connected Qualcomm devices in EDL (Emergency Download) mode over USB
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
                path: qdl_device_path(&bus_id, addr),
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

/// A USB device qdlrs may open, as its own selection logic sees it.
#[derive(Debug, Clone)]
pub(crate) struct EdlCandidate {
    pub path: String,
    pub product: Option<String>,
}

/// Same filter as qdlrs `setup_usb_device`: Qualcomm VID with the EDL or ramdump PID.
fn edl_candidates() -> Result<Vec<EdlCandidate>, String> {
    Ok(nusb::list_devices()
        .wait()
        .map_err(|e| format!("Failed to enumerate USB devices: {}", e))?
        .filter(|dev| {
            dev.vendor_id() == QUALCOMM_VID
                && [EDL_PID, EDL_RAMDUMP_PID].contains(&dev.product_id())
        })
        .map(|dev| EdlCandidate {
            path: qdl_device_path(dev.bus_id(), dev.device_address()),
            product: dev.product_string().map(str::to_string),
        })
        .collect())
}

/// The serial qdlrs matches on: the whole tail after the first `_SN:` of the product string.
fn product_sn(product: &str) -> Option<&str> {
    product
        .find(PRODUCT_SN_MARKER)
        .map(|i| &product[i + PRODUCT_SN_MARKER.len()..])
}

/// Serial that makes qdlrs open exactly the device at `path`; None only when it is the sole candidate.
pub(crate) fn select_edl_serial(
    path: &str,
    candidates: &[EdlCandidate],
) -> Result<Option<String>, String> {
    if !path.starts_with(QDL_PATH_PREFIX) {
        return Err(format!(
            "{TAG_INVALID_PATH} {path:?} is not an EDL device path"
        ));
    }

    let matches: Vec<&EdlCandidate> = candidates.iter().filter(|c| c.path == path).collect();
    let selected = match matches.as_slice() {
        [] => {
            return Err(format!(
                "{TAG_QDL_DEVICE_NOT_FOUND} {path:?} is not a connected EDL device"
            ))
        }
        [one] => *one,
        many => {
            return Err(format!(
                "{TAG_QDL_MULTIPLE_DEVICES} {path:?} matches {} EDL devices",
                many.len()
            ))
        }
    };

    let sn = selected
        .product
        .as_deref()
        .and_then(product_sn)
        .filter(|sn| !sn.is_empty());

    if candidates.len() == 1 {
        return Ok(sn.map(str::to_string));
    }

    let ambiguous = || {
        format!(
            "{TAG_QDL_MULTIPLE_DEVICES} {} EDL devices connected and {path:?} cannot be told apart by serial",
            candidates.len()
        )
    };
    let sn = sn.ok_or_else(ambiguous)?;

    // qdlrs unwraps `_SN:` on every product string it walks and opens the first serial match.
    for other in candidates.iter().filter(|c| !std::ptr::eq(*c, selected)) {
        if let Some(product) = other.product.as_deref() {
            match product_sn(product) {
                Some(other_sn) if !other_sn.eq_ignore_ascii_case(sn) => {}
                _ => return Err(ambiguous()),
            }
        }
    }

    Ok(Some(sn.to_string()))
}

/// Scan now and pick the serial for `path`; call right before connecting.
pub(crate) fn resolve_edl_serial(path: &str) -> Result<Option<String>, String> {
    select_edl_serial(path, &edl_candidates()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRODUCT_A: &str = "QUSB__BULK_CID:0402_SN:ABCD1234";
    const PRODUCT_B: &str = "QUSB__BULK_CID:0402_SN:FFFF0000";

    fn candidate(bus: &str, addr: u8, product: Option<&str>) -> EdlCandidate {
        EdlCandidate {
            path: qdl_device_path(bus, addr),
            product: product.map(str::to_string),
        }
    }

    #[test]
    fn device_path_keeps_bus_id_verbatim() {
        assert_eq!(qdl_device_path("1", 5), "qdl://1/5");
        assert_eq!(qdl_device_path("", 5), "qdl:///5");
        let windows_bus = "PCIROOT(0)#PCI(1400)#USBROOT(0)";
        assert_eq!(
            qdl_device_path(windows_bus, 3),
            format!("qdl://{windows_bus}/3")
        );
    }

    #[test]
    fn product_sn_takes_tail_after_first_marker() {
        assert_eq!(product_sn(PRODUCT_A), Some("ABCD1234"));
        assert_eq!(product_sn("X_SN:A_SN:B"), Some("A_SN:B"));
        assert_eq!(product_sn("QUSB__BULK"), None);
        assert_eq!(product_sn("_SN:"), Some(""));
    }

    #[test]
    fn single_device_with_serial_is_targeted_by_serial() {
        let c = vec![candidate("1", 5, Some(PRODUCT_A))];
        assert_eq!(
            select_edl_serial("qdl://1/5", &c).unwrap(),
            Some("ABCD1234".to_string())
        );
    }

    #[test]
    fn single_device_without_serial_falls_back_to_first() {
        for product in [None, Some("QUSB__BULK"), Some("QUSB__BULK_SN:")] {
            let c = vec![candidate("1", 5, product)];
            assert_eq!(select_edl_serial("qdl://1/5", &c).unwrap(), None);
        }
    }

    #[test]
    fn missing_device_is_refused() {
        let c = vec![candidate("1", 5, Some(PRODUCT_A))];
        assert!(select_edl_serial("qdl://1/6", &c)
            .unwrap_err()
            .starts_with(TAG_QDL_DEVICE_NOT_FOUND));
        assert!(select_edl_serial("qdl://1/5", &[])
            .unwrap_err()
            .starts_with(TAG_QDL_DEVICE_NOT_FOUND));
    }

    #[test]
    fn path_must_match_exactly() {
        let c = vec![candidate("1", 5, Some(PRODUCT_A))];
        for path in ["qdl://1/5 ", "QDL://1/5", "qdl://1/05", "qdl://01/5"] {
            assert!(
                select_edl_serial(path, &c).is_err(),
                "{path:?} was accepted"
            );
        }
    }

    #[test]
    fn non_qdl_paths_are_refused() {
        let c = vec![candidate("1", 5, Some(PRODUCT_A))];
        for path in ["/dev/disk4", "devsim://edl", ""] {
            assert!(select_edl_serial(path, &c)
                .unwrap_err()
                .starts_with(TAG_INVALID_PATH));
        }
    }

    #[test]
    fn two_devices_with_distinct_serials_are_targeted() {
        let c = vec![
            candidate("1", 5, Some(PRODUCT_A)),
            candidate("1", 6, Some(PRODUCT_B)),
        ];
        assert_eq!(
            select_edl_serial("qdl://1/6", &c).unwrap(),
            Some("FFFF0000".to_string())
        );
    }

    #[test]
    fn other_device_without_product_string_is_tolerated() {
        let c = vec![candidate("1", 5, Some(PRODUCT_A)), candidate("1", 6, None)];
        assert_eq!(
            select_edl_serial("qdl://1/5", &c).unwrap(),
            Some("ABCD1234".to_string())
        );
    }

    #[test]
    fn selected_device_without_serial_among_several_is_refused() {
        let c = vec![candidate("1", 5, None), candidate("1", 6, Some(PRODUCT_B))];
        assert!(select_edl_serial("qdl://1/5", &c)
            .unwrap_err()
            .starts_with(TAG_QDL_MULTIPLE_DEVICES));
    }

    #[test]
    fn other_product_string_without_marker_is_refused() {
        let c = vec![
            candidate("1", 5, Some(PRODUCT_A)),
            candidate("1", 6, Some("QUSB__BULK")),
        ];
        assert!(select_edl_serial("qdl://1/5", &c)
            .unwrap_err()
            .starts_with(TAG_QDL_MULTIPLE_DEVICES));
    }

    #[test]
    fn case_duplicate_serials_are_refused() {
        let c = vec![
            candidate("1", 5, Some("QUSB__BULK_SN:abcd1234")),
            candidate("1", 6, Some(PRODUCT_A)),
        ];
        assert!(select_edl_serial("qdl://1/6", &c)
            .unwrap_err()
            .starts_with(TAG_QDL_MULTIPLE_DEVICES));
    }

    #[test]
    fn two_devices_on_the_same_path_are_refused() {
        let c = vec![
            candidate("", 5, Some(PRODUCT_A)),
            candidate("", 5, Some(PRODUCT_B)),
        ];
        assert!(select_edl_serial("qdl:///5", &c)
            .unwrap_err()
            .starts_with(TAG_QDL_MULTIPLE_DEVICES));
    }
}

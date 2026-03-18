//! macOS device detection using DiskArbitration framework
//!
//! Uses the native DiskArbitration API for fast, low-latency
//! device enumeration without spawning external processes.

use std::sync::{Mutex, OnceLock};

use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::runloop::{kCFRunLoopDefaultMode, CFRunLoop};
use core_foundation::string::CFString;

use crate::log_error;
use crate::utils::format_size;

use super::types::{detect_sd_from_name, normalize_bus_type, BlockDevice};

/// Cached system disk identifier — resolved once at startup
static SYSTEM_DISK: OnceLock<Option<String>> = OnceLock::new();

// ---------------------------------------------------------------------------
// DiskArbitration FFI bindings
// ---------------------------------------------------------------------------
mod da {
    use core_foundation::base::{kCFAllocatorDefault, CFType};
    use core_foundation::base::{CFAllocatorRef, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::number::CFNumber;
    use core_foundation::string::{CFString, CFStringRef};
    use std::ffi::c_void;
    use std::os::raw::c_char;

    pub enum DASession {}
    pub type DASessionRef = *mut DASession;

    pub enum DADisk {}
    pub type DADiskRef = *mut DADisk;

    pub type DADiskAppearedCallback = extern "C" fn(disk: DADiskRef, context: *mut c_void);

    #[link(name = "DiskArbitration", kind = "framework")]
    extern "C" {
        pub fn DASessionCreate(allocator: CFAllocatorRef) -> DASessionRef;
        pub fn DASessionScheduleWithRunLoop(
            session: DASessionRef,
            runLoop: core_foundation::runloop::CFRunLoopRef,
            mode: CFStringRef,
        );
        pub fn DASessionUnscheduleFromRunLoop(
            session: DASessionRef,
            runLoop: core_foundation::runloop::CFRunLoopRef,
            mode: CFStringRef,
        );
        pub fn DARegisterDiskAppearedCallback(
            session: DASessionRef,
            match_dict: CFDictionaryRef,
            callback: DADiskAppearedCallback,
            context: *mut c_void,
        );
        pub fn DADiskCopyDescription(disk: DADiskRef) -> CFDictionaryRef;
        pub fn DADiskGetBSDName(disk: DADiskRef) -> *const c_char;
    }

    // Description keys
    #[link(name = "DiskArbitration", kind = "framework")]
    extern "C" {
        pub static kDADiskDescriptionMediaSizeKey: CFStringRef;
        pub static kDADiskDescriptionDeviceProtocolKey: CFStringRef;
        pub static kDADiskDescriptionDeviceInternalKey: CFStringRef;
        pub static kDADiskDescriptionMediaRemovableKey: CFStringRef;
        pub static kDADiskDescriptionMediaWritableKey: CFStringRef;
        pub static kDADiskDescriptionMediaNameKey: CFStringRef;
        pub static kDADiskDescriptionMediaWholeKey: CFStringRef;
        pub static kDADiskDescriptionMediaIconKey: CFStringRef;
        pub static kDADiskDescriptionDeviceModelKey: CFStringRef;
        pub static kDADiskDescriptionMediaContentKey: CFStringRef;
    }

    /// RAII wrapper for DASessionRef
    pub struct SafeSession(pub DASessionRef);

    impl Drop for SafeSession {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    core_foundation::base::CFRelease(self.0 as core_foundation::base::CFTypeRef);
                }
            }
        }
    }

    impl SafeSession {
        pub fn new() -> Option<Self> {
            unsafe {
                let session = DASessionCreate(kCFAllocatorDefault);
                if session.is_null() {
                    None
                } else {
                    Some(SafeSession(session))
                }
            }
        }
    }

    // Typed helpers for reading disk description dictionaries

    pub fn get_string(dict: &CFDictionary<CFString, CFType>, key: CFStringRef) -> Option<String> {
        unsafe {
            let cf_key = CFString::wrap_under_get_rule(key);
            dict.find(cf_key)
                .and_then(|val| val.downcast::<CFString>())
                .map(|s| s.to_string())
        }
    }

    pub fn get_bool(dict: &CFDictionary<CFString, CFType>, key: CFStringRef) -> Option<bool> {
        unsafe {
            let cf_key = CFString::wrap_under_get_rule(key);
            dict.find(cf_key)
                .and_then(|val| val.downcast::<CFBoolean>())
                .map(|b| b == CFBoolean::true_value())
        }
    }

    pub fn get_u64(dict: &CFDictionary<CFString, CFType>, key: CFStringRef) -> Option<u64> {
        unsafe {
            let cf_key = CFString::wrap_under_get_rule(key);
            dict.find(cf_key)
                .and_then(|val| val.downcast::<CFNumber>())
                .and_then(|n| n.to_i64())
                .map(|n| n as u64)
        }
    }
}

// ---------------------------------------------------------------------------
// Device enumeration
// ---------------------------------------------------------------------------

/// Callback invoked by DiskArbitration for each discovered disk
extern "C" fn disk_appeared_callback(disk: da::DADiskRef, context: *mut std::ffi::c_void) {
    unsafe {
        let devices = &*(context as *const Mutex<Vec<BlockDevice>>);
        if let Some(device) = process_disk(disk) {
            if let Ok(mut list) = devices.lock() {
                list.push(device);
            }
        }
    }
}

/// Extract a BlockDevice from a DiskArbitration disk reference.
/// Returns None for partitions, APFS containers, virtual disks, and zero-size media.
unsafe fn process_disk(disk: da::DADiskRef) -> Option<BlockDevice> {
    let desc_ref = da::DADiskCopyDescription(disk);
    if desc_ref.is_null() {
        return None;
    }
    let desc: CFDictionary<CFString, CFType> = CFDictionary::wrap_under_create_rule(desc_ref);

    // Skip partitions
    if !da::get_bool(&desc, da::kDADiskDescriptionMediaWholeKey).unwrap_or(false) {
        return None;
    }

    let bsd_name_ptr = da::DADiskGetBSDName(disk);
    if bsd_name_ptr.is_null() {
        return None;
    }
    let bsd_name = std::ffi::CStr::from_ptr(bsd_name_ptr)
        .to_string_lossy()
        .to_string();

    let size = da::get_u64(&desc, da::kDADiskDescriptionMediaSizeKey).unwrap_or(0);
    if size == 0 {
        return None;
    }

    let protocol =
        da::get_string(&desc, da::kDADiskDescriptionDeviceProtocolKey).unwrap_or_default();
    let content = da::get_string(&desc, da::kDADiskDescriptionMediaContentKey).unwrap_or_default();

    // Physical disks have content "GUID_partition_scheme", "FDisk_partition_scheme",
    // or empty. APFS containers and other synthesized disks carry a GUID instead.
    if !content.is_empty() && !content.contains("partition_scheme") {
        return None;
    }

    // Skip virtual / disk-image protocols
    let proto_upper = protocol.to_uppercase();
    if proto_upper.contains("VIRTUAL") || proto_upper.contains("DISK IMAGE") {
        return None;
    }

    let is_internal = da::get_bool(&desc, da::kDADiskDescriptionDeviceInternalKey).unwrap_or(false);
    let is_removable =
        da::get_bool(&desc, da::kDADiskDescriptionMediaRemovableKey).unwrap_or(false);
    let is_writable = da::get_bool(&desc, da::kDADiskDescriptionMediaWritableKey).unwrap_or(true);
    let media_name = da::get_string(&desc, da::kDADiskDescriptionMediaNameKey).unwrap_or_default();
    let model = da::get_string(&desc, da::kDADiskDescriptionDeviceModelKey).unwrap_or_default();

    // Determine bus type from protocol, media name, or icon resource
    let bus_type = normalize_bus_type(&protocol)
        .or_else(|| detect_sd_from_name(&media_name))
        .or_else(|| check_sd_icon(&desc));

    let display_model = if !model.is_empty() {
        model.trim().to_string()
    } else {
        media_name
    };

    Some(BlockDevice {
        path: format!("/dev/{}", bsd_name),
        name: bsd_name,
        size,
        size_formatted: format_size(size),
        model: display_model,
        is_removable,
        is_system: is_internal && !is_removable,
        bus_type,
        is_read_only: !is_writable,
    })
}

/// Detect SD card readers via the IOBundleResourceFile icon property.
/// Returns Some("SD") when the icon references an SD-related resource.
unsafe fn check_sd_icon(desc: &CFDictionary<CFString, CFType>) -> Option<String> {
    use core_foundation_sys::dictionary::CFDictionaryGetValue;
    use core_foundation_sys::string::CFStringRef;

    let icon_key = CFString::wrap_under_get_rule(da::kDADiskDescriptionMediaIconKey);
    let icon_val = desc.find(icon_key)?;

    let icon_dict_ref = icon_val.as_CFTypeRef() as core_foundation_sys::dictionary::CFDictionaryRef;
    let resource_key = CFString::new("IOBundleResourceFile");
    let resource_ptr = CFDictionaryGetValue(icon_dict_ref, resource_key.as_concrete_TypeRef() as _);
    if resource_ptr.is_null() {
        return None;
    }

    let resource = CFString::wrap_under_get_rule(resource_ptr as CFStringRef);
    if resource.to_string().contains("SD") {
        Some("SD".to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Enumerate block devices via the DiskArbitration framework.
/// The system disk identifier is resolved once and cached.
pub fn get_block_devices() -> Result<Vec<BlockDevice>, String> {
    let session = da::SafeSession::new().ok_or_else(|| {
        log_error!("devices", "Failed to create DiskArbitration session");
        "Failed to create DiskArbitration session".to_string()
    })?;

    let devices: Mutex<Vec<BlockDevice>> = Mutex::new(Vec::new());

    unsafe {
        let run_loop = CFRunLoop::get_current();

        da::DASessionScheduleWithRunLoop(
            session.0,
            run_loop.as_concrete_TypeRef(),
            kCFRunLoopDefaultMode,
        );

        da::DARegisterDiskAppearedCallback(
            session.0,
            std::ptr::null(),
            disk_appeared_callback,
            &devices as *const Mutex<Vec<BlockDevice>> as *mut std::ffi::c_void,
        );

        // Brief run-loop spin to collect all disk-appeared callbacks
        CFRunLoop::run_in_mode(
            kCFRunLoopDefaultMode,
            std::time::Duration::from_millis(50),
            false,
        );

        da::DASessionUnscheduleFromRunLoop(
            session.0,
            run_loop.as_concrete_TypeRef(),
            kCFRunLoopDefaultMode,
        );
    }

    let mut result = devices.into_inner().map_err(|e| {
        log_error!("devices", "Failed to collect devices: {}", e);
        format!("Failed to collect devices: {}", e)
    })?;

    // Mark system disk
    let system_disk = SYSTEM_DISK.get_or_init(get_system_disk);
    if let Some(ref sys_disk) = system_disk {
        for device in &mut result {
            if device.name == *sys_disk || device.name == "disk0" {
                device.is_system = true;
            }
        }
    }

    Ok(result)
}

/// Resolve the system disk identifier via diskutil (called once)
fn get_system_disk() -> Option<String> {
    let output = std::process::Command::new("diskutil")
        .args(["info", "/"])
        .output()
        .ok()?;

    let info = String::from_utf8_lossy(&output.stdout);
    for line in info.lines() {
        if line.contains("Part of Whole:") {
            return line.split(':').nth(1).map(|s| s.trim().to_string());
        }
    }
    None
}

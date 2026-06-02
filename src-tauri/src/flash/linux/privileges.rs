//! Linux privilege management. UDisks2/polkit prompts when the device is opened,
//! so the app can run as a normal user.

use crate::log_info;

const MODULE: &str = "flash::linux::privileges";

/// No-op authorization: polkit prompts later when the device is opened, so just signal go-ahead.
pub fn request_authorization(device_path: &str) -> Result<bool, String> {
    log_info!(
        MODULE,
        "Authorization will be requested via polkit when accessing: {}",
        device_path
    );
    Ok(true)
}

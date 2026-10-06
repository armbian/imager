// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

//! Linux privilege management. UDisks2/polkit prompts when the device is opened,
//! so the app can run as a normal user.

use crate::devices::FlashTarget;
use crate::flash::reject_simulated;
use crate::log_info;

const MODULE: &str = "flash::linux::privileges";

/// No-op authorization: polkit prompts later when the device is opened, so just signal go-ahead.
pub fn request_authorization(target: &FlashTarget) -> Result<bool, String> {
    reject_simulated(target.path())?;
    log_info!(
        MODULE,
        "Authorization will be requested via polkit when accessing: {}",
        target.path()
    );
    Ok(true)
}

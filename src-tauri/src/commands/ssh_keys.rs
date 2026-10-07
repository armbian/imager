// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

//! Public SSH key lookup for a profile's key source (read-only, nothing is written).

use crate::ssh_keys::{self, SshKeyLookup, SshKeySource};

/// Public keys behind a GitHub/GitLab username or an https link; errors are `[SSH_KEYS_*]` codes only.
#[tauri::command]
pub async fn lookup_ssh_keys(source: SshKeySource, value: String) -> Result<SshKeyLookup, String> {
    ssh_keys::lookup(source, &value).await
}

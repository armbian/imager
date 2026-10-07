// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Machine tags on error strings; src/utils/errorUtils.ts maps them to translated messages.

use std::fmt::Display;

pub const TAG_CANCELLED: &str = "[CANCELLED]";

/// Prefix `detail` with a machine tag, keeping the human text for the logs.
pub fn tagged(tag: &str, detail: impl Display) -> String {
    format!("{tag} {detail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_leads_and_detail_is_kept() {
        let msg = tagged(TAG_CANCELLED, "Decompression cancelled");
        assert_eq!(msg, "[CANCELLED] Decompression cancelled");
        assert!(msg.starts_with(TAG_CANCELLED));
    }
}

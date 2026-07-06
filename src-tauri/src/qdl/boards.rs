//! Bundled fallback registry of per-board QDL/EDL facts, used when the Armbian API
//! carries no `qdl` block for a board (offline, older API). The API is the primary source.

use crate::qdl::QdlStorage;

pub struct QdlBoard {
    /// Matched case-insensitively as a substring of the board slug.
    pub slug_token: &'static str,
    /// Resolves the loader family via the SoC→family map.
    pub soc: &'static str,
    pub storage: QdlStorage,
    pub provision_rel: Option<&'static str>,
}

pub const QDL_BOARDS: &[QdlBoard] = &[
    QdlBoard {
        slug_token: "dragon-q6a",
        soc: "QCS6490",
        storage: QdlStorage::Ufs,
        provision_rel: Some("radxa-dragon-q6a/provision_ufs31_lun0_only.xml"),
    },
    QdlBoard {
        slug_token: "arduino-uno-q",
        soc: "QRB2210",
        storage: QdlStorage::Emmc,
        provision_rel: None,
    },
];

pub fn find(board_slug: &str) -> Option<&'static QdlBoard> {
    let slug = board_slug.to_lowercase();
    QDL_BOARDS.iter().find(|b| slug.contains(b.slug_token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_board_by_slug_substring() {
        let b = find("radxa-dragon-q6a").expect("dragon-q6a is registered");
        assert_eq!(b.soc, "QCS6490");
        assert!(b.provision_rel.is_some());
        assert!(find("orangepi-5").is_none());
    }
}

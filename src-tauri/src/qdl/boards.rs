//! Bundled fallback registry of per-board QDL/EDL facts, used when the Armbian API
//! carries no `qdl` block for a board (offline, older API). The API is the primary source.

use std::path::Path;

use super::UFS_MARKERS;
use crate::qdl::QdlStorage;
use crate::utils::{normalize_slug, parse_armbian_filename};

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

/// Normalized slug if `filename` is a UFS build of a UFS-capable QDL board, else None.
pub fn ufs_board_slug_for_filename(filename: &str) -> Option<String> {
    let basename = Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(filename);
    let parsed = parse_armbian_filename(basename)?;

    // The UFS suffix lives in the variant field; fall back to the whole name if absent.
    let variant = parsed.desktop.as_deref().unwrap_or(basename).to_lowercase();
    if !UFS_MARKERS.iter().any(|m| variant.contains(m)) {
        return None;
    }

    let slug = normalize_slug(&parsed.board_slug);
    find(&slug).filter(|b| b.storage == QdlStorage::Ufs)?;
    Some(slug)
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
        assert!(find("arduino-uno-q")
            .expect("registered")
            .provision_rel
            .is_none());
    }

    #[test]
    fn ufs_marker_and_capable_board_is_ufs() {
        assert_eq!(
            ufs_board_slug_for_filename(
                "Armbian-unofficial_26.08.0-trunk_Radxa-dragon-q6a_resolute_edge_7.1.3_gnome-ufs_desktop.img",
            ),
            Some("radxa-dragon-q6a".to_string())
        );
    }

    #[test]
    fn ufs_marker_survives_compression_ext() {
        assert!(ufs_board_slug_for_filename(
            "Armbian-unofficial_26.08.0-trunk_Radxa-dragon-q6a_resolute_edge_7.1.3_gnome-ufs_desktop.img.xz",
        )
        .is_some());
    }

    #[test]
    fn sd_variant_of_ufs_board_is_not_ufs() {
        assert!(ufs_board_slug_for_filename(
            "Armbian-unofficial_26.08.0-trunk_Radxa-dragon-q6a_resolute_edge_7.1.3_gnome_desktop.img",
        )
        .is_none());
    }

    #[test]
    fn ufs_marker_on_non_ufs_board_is_none() {
        // arduino-uno-q storage is eMMC, not UFS => not a raw-UFS target.
        assert!(ufs_board_slug_for_filename(
            "Armbian_26.02.0_Arduino-uno-q_trixie_edge_6.19.4_minimal-ufs.img",
        )
        .is_none());
    }

    #[test]
    fn non_armbian_name_is_none() {
        assert!(ufs_board_slug_for_filename("ubuntu-24.04-ufs.img").is_none());
    }
}

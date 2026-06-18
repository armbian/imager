//! Single source of truth for per-board QDL/EDL facts (loader family, UFS provisioning, EDL hint).

#[derive(Clone, Copy)]
pub enum EdlEntry {
    Button,
    Jumper,
}

impl EdlEntry {
    pub fn as_str(self) -> &'static str {
        match self {
            EdlEntry::Button => "button",
            EdlEntry::Jumper => "jumper",
        }
    }
}

pub struct QdlBoard {
    /// Matched case-insensitively as a substring of the board slug.
    pub slug_token: &'static str,
    /// Used to resolve the loader family when the Armbian API leaves `soc` null.
    pub soc: &'static str,
    pub provision_rel: Option<&'static str>,
    pub edl_entry: EdlEntry,
}

pub const QDL_BOARDS: &[QdlBoard] = &[
    QdlBoard {
        slug_token: "dragon-q6a",
        soc: "QCS6490",
        provision_rel: Some("radxa-dragon-q6a/provision_ufs31_lun0_only.xml"),
        edl_entry: EdlEntry::Button,
    },
    QdlBoard {
        slug_token: "arduino-uno-q",
        soc: "QRB2210",
        provision_rel: None,
        edl_entry: EdlEntry::Jumper,
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
        assert_eq!(b.edl_entry.as_str(), "button");
        assert!(b.provision_rel.is_some());
        assert!(find("orangepi-5").is_none());
    }
}

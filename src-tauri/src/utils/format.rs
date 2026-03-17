//! Formatting utilities for human-readable output
//!
//! Provides consistent formatting functions for the application.

/// Bytes per megabyte constant
pub const MB: u64 = 1024 * 1024;
/// Bytes per gigabyte constant
pub const GB: u64 = 1024 * 1024 * 1024;

/// Convert bytes to megabytes as f64 (for calculations and logging)
#[inline]
pub fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / MB as f64
}

/// Convert bytes to gigabytes as f64 (for calculations and logging)
#[inline]
pub fn bytes_to_gb(bytes: u64) -> f64 {
    bytes as f64 / GB as f64
}

/// Format bytes into human-readable size string (e.g., "1.5 GB", "256 MB")
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.0} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Parsed metadata from an Armbian image filename
#[derive(Debug, Clone, PartialEq)]
pub struct ArmbianFilenameInfo {
    /// Board slug extracted from filename (lowercase, e.g. "nanopi-m5")
    pub board_slug: String,
    /// Version string (e.g. "25.02.0" or "26.2.0-trunk.493")
    pub version: Option<String>,
    /// Distribution name (e.g. "bookworm", "trixie")
    pub distro: Option<String>,
    /// Branch name (e.g. "current", "edge", "vendor")
    pub branch: Option<String>,
    /// Kernel version (e.g. "6.12.8")
    pub kernel: Option<String>,
    /// Desktop environment or "minimal" (e.g. "gnome", "minimal")
    pub desktop: Option<String>,
}

/// Strip compression and image extensions from a filename
///
/// Removes trailing `.xz`, `.gz`, `.zst`, `.bz2` and then `.img`.
fn strip_image_extensions(filename: &str) -> &str {
    let name = filename
        .strip_suffix(".xz")
        .or_else(|| filename.strip_suffix(".gz"))
        .or_else(|| filename.strip_suffix(".zst"))
        .or_else(|| filename.strip_suffix(".bz2"))
        .unwrap_or(filename);

    name.strip_suffix(".img").unwrap_or(name)
}

/// Parse an Armbian image filename into structured metadata
///
/// Handles multiple naming conventions:
/// - Standard: `Armbian_{version}_{board}_{distro}_{branch}_{kernel}[_{desktop}].img[.ext]`
/// - Labeled: `Armbian_{label}_{version}_{board}_{distro}_{branch}_{kernel}[_{desktop}].img[.ext]`
/// - Prefixed: `Armbian-unofficial_{version}_{board}_...` (parts[0] starts with "armbian")
///
/// Detection logic: if `parts[1]` does not start with a digit, it is treated as
/// a label and all subsequent field indices are shifted by 1.
///
/// Examples:
/// - `Armbian_25.02.0_Nanopi-m5_bookworm_current_6.12.8_gnome.img.xz`
/// - `Armbian_community_26.2.0-trunk.493_Youyeetoo-r1-v3_trixie_edge_6.19.3_minimal.img`
/// - `Armbian-unofficial_26.02.0-trunk_Cix-acpi_trixie_edge_6.19.4_minimal.img.xz`
pub fn parse_armbian_filename(filename: &str) -> Option<ArmbianFilenameInfo> {
    let name = strip_image_extensions(filename);
    let parts: Vec<&str> = name.split('_').collect();

    // Must start with "Armbian" (possibly hyphenated, e.g. "Armbian-unofficial")
    if parts.len() < 4 || !parts[0].to_ascii_lowercase().starts_with("armbian") {
        return None;
    }

    // If parts[1] doesn't start with a digit, it's a label (e.g. "community")
    let offset = if !parts[1].starts_with(|c: char| c.is_ascii_digit()) {
        1
    } else {
        0
    };

    // Board is at index 2+offset; need at least that many parts
    let board_index = 2 + offset;
    if board_index >= parts.len() {
        return None;
    }

    let board_slug = parts[board_index].to_lowercase();

    Some(ArmbianFilenameInfo {
        board_slug,
        version: parts.get(1 + offset).map(|s| s.to_string()),
        distro: parts.get(3 + offset).map(|s| s.to_string()),
        branch: parts.get(4 + offset).map(|s| s.to_string()),
        kernel: parts.get(5 + offset).map(|s| s.to_string()),
        desktop: if parts.len() > 6 + offset {
            Some(parts[(6 + offset)..].join("_"))
        } else {
            None
        },
    })
}

/// Normalize a slug by replacing non-alphanumeric chars with hyphens
/// and collapsing multiple hyphens into one
pub fn normalize_slug(slug: &str) -> String {
    slug.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1 KB");
        assert_eq!(format_size(1536), "2 KB");
        assert_eq!(format_size(1048576), "1 MB");
        assert_eq!(format_size(1073741824), "1.0 GB");
        assert_eq!(format_size(1610612736), "1.5 GB");
    }

    #[test]
    fn test_normalize_slug() {
        assert_eq!(normalize_slug("Orange-Pi-5"), "orange-pi-5");
        assert_eq!(normalize_slug("rock__pi__4"), "rock-pi-4");
        assert_eq!(normalize_slug("Banana PI M5"), "banana-pi-m5");
    }

    #[test]
    fn test_bytes_to_mb() {
        assert!((bytes_to_mb(0) - 0.0).abs() < f64::EPSILON);
        assert!((bytes_to_mb(1048576) - 1.0).abs() < f64::EPSILON);
        assert!((bytes_to_mb(10485760) - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_bytes_to_gb() {
        assert!((bytes_to_gb(0) - 0.0).abs() < f64::EPSILON);
        assert!((bytes_to_gb(1073741824) - 1.0).abs() < f64::EPSILON);
        assert!((bytes_to_gb(2147483648) - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_armbian_filename_standard() {
        let info = parse_armbian_filename(
            "Armbian_25.02.0_Nanopi-m5_bookworm_current_6.12.8_gnome.img.xz",
        )
        .unwrap();
        assert_eq!(info.board_slug, "nanopi-m5");
        assert_eq!(info.version.as_deref(), Some("25.02.0"));
        assert_eq!(info.distro.as_deref(), Some("bookworm"));
        assert_eq!(info.branch.as_deref(), Some("current"));
        assert_eq!(info.kernel.as_deref(), Some("6.12.8"));
        assert_eq!(info.desktop.as_deref(), Some("gnome"));
    }

    #[test]
    fn test_parse_armbian_filename_community() {
        let info = parse_armbian_filename(
            "Armbian_community_26.2.0-trunk.493_Youyeetoo-r1-v3_trixie_edge_6.19.3_minimal.img",
        )
        .unwrap();
        assert_eq!(info.board_slug, "youyeetoo-r1-v3");
        assert_eq!(info.version.as_deref(), Some("26.2.0-trunk.493"));
        assert_eq!(info.distro.as_deref(), Some("trixie"));
        assert_eq!(info.branch.as_deref(), Some("edge"));
        assert_eq!(info.kernel.as_deref(), Some("6.19.3"));
        assert_eq!(info.desktop.as_deref(), Some("minimal"));
    }

    #[test]
    fn test_parse_armbian_filename_unofficial() {
        let info = parse_armbian_filename(
            "Armbian-unofficial_26.02.0-trunk_Cix-acpi_trixie_edge_6.19.4_minimal.img.xz",
        )
        .unwrap();
        assert_eq!(info.board_slug, "cix-acpi");
        assert_eq!(info.version.as_deref(), Some("26.02.0-trunk"));
        assert_eq!(info.distro.as_deref(), Some("trixie"));
        assert_eq!(info.branch.as_deref(), Some("edge"));
    }

    #[test]
    fn test_parse_armbian_filename_minimal_no_desktop() {
        let info =
            parse_armbian_filename("Armbian_26.2.1_Nanopim4v2_trixie_current_6.18.8_minimal.img")
                .unwrap();
        assert_eq!(info.board_slug, "nanopim4v2");
        assert_eq!(info.desktop.as_deref(), Some("minimal"));
    }

    #[test]
    fn test_parse_armbian_filename_not_armbian() {
        assert!(parse_armbian_filename("ubuntu-24.04-desktop.img.xz").is_none());
    }

    #[test]
    fn test_parse_armbian_filename_too_short() {
        assert!(parse_armbian_filename("Armbian_25.02.0.img").is_none());
    }
}

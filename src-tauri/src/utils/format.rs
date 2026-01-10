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
}

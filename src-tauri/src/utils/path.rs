//! Path utility functions
//!
//! Common path manipulation helpers used across the application.

/// Strip compression extension from filename (.xz, .gz, .bz2, .zst)
///
/// # Arguments
/// * `filename` - The filename to strip the extension from
///
/// # Returns
/// The filename without the compression extension, or the original if no match
pub fn strip_compression_ext(filename: &str) -> &str {
    for ext in &[".xz", ".gz", ".bz2", ".zst"] {
        if let Some(stripped) = filename.strip_suffix(ext) {
            return stripped;
        }
    }
    filename
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_compression_ext() {
        assert_eq!(strip_compression_ext("image.img.xz"), "image.img");
        assert_eq!(strip_compression_ext("image.img.gz"), "image.img");
        assert_eq!(strip_compression_ext("image.img.bz2"), "image.img");
        assert_eq!(strip_compression_ext("image.img.zst"), "image.img");
        assert_eq!(strip_compression_ext("image.img"), "image.img");
        assert_eq!(strip_compression_ext("no-extension"), "no-extension");
    }
}

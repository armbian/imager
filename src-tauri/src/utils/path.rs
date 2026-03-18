//! Path utility functions
//!
//! Common path manipulation helpers used across the application.

use std::path::{Path, PathBuf};

use crate::config;

use super::get_cache_dir;

/// Validates that a path is within the application cache directory
///
/// Canonicalizes both the target path and the cache directory to resolve
/// symlinks and prevent path traversal attacks, then checks containment.
///
/// # Arguments
/// * `path` - The path to validate
///
/// # Returns
/// The canonicalized path if it is within the cache directory
///
/// # Errors
/// Returns an error if the path cannot be resolved or is outside the cache directory
pub fn validate_cache_path(path: &Path) -> Result<PathBuf, String> {
    let cache_dir = get_cache_dir(config::app::NAME)
        .canonicalize()
        .map_err(|e| format!("Failed to resolve cache directory: {}", e))?;
    let canonical_path = path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve path: {}", e))?;
    if !canonical_path.starts_with(&cache_dir) {
        return Err("Cannot operate on files outside cache directory".to_string());
    }
    Ok(canonical_path)
}

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

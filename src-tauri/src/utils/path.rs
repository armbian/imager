//! Path manipulation helpers used across the application.

use std::path::{Path, PathBuf};

use super::app_cache_dir;

/// Validate that a path resolves to within the cache directory, returning its
/// canonical form. Canonicalizes both paths to defeat symlink/traversal tricks.
pub fn validate_cache_path(path: &Path) -> Result<PathBuf, String> {
    let cache_dir = app_cache_dir()
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

/// Strip a compression extension (.xz, .gz, .bz2, .zst), returning the original if none matches
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

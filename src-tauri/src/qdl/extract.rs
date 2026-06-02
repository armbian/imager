//! TAR archive extraction for QDL images. Archives contain flash/prog_firehose_ddr.elf (Sahara programmer),
//! flash/rawprogram0.xml (partition instructions), flash/patch0.xml (post-flash patches), and partition images.

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::{log_error, log_info};

/// Required files that must exist after extraction for QDL flashing
pub const REQUIRED_FILES: &[&str] = &["rawprogram0.xml"];

/// The firehose programmer ELF (required for Sahara upload)
pub const FIREHOSE_ELF: &str = "prog_firehose_ddr.elf";

/// Extract a QDL TAR archive under `output_dir`, validate the required flash
/// files are present, and return the directory holding them.
pub fn extract_qdl_archive(tar_path: &Path, output_dir: &Path) -> Result<PathBuf, String> {
    log_info!(
        "qdl::extract",
        "Extracting QDL archive: {} -> {}",
        tar_path.display(),
        output_dir.display()
    );

    let extract_dir = output_dir.join("qdl-extract");
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir)
            .map_err(|e| format!("Failed to clean existing extraction directory: {}", e))?;
    }
    fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Failed to create extraction directory: {}", e))?;

    let reader = open_tar_reader(tar_path)?;
    safe_unpack(reader, &extract_dir)?;

    let flash_dir = find_flash_dir(&extract_dir)?;
    validate_required_files(&flash_dir)?;

    log_info!(
        "qdl::extract",
        "Extraction complete. Flash directory: {}",
        flash_dir.display()
    );

    Ok(flash_dir)
}

/// Find the flash-files directory in the extracted archive (a "flash" dir or
/// rawprogram0.xml directly), handling nesting like arduino-images/flash/.
fn find_flash_dir(extract_dir: &Path) -> Result<PathBuf, String> {
    if extract_dir.join("rawprogram0.xml").exists() {
        return Ok(extract_dir.to_path_buf());
    }

    if let Some(flash_dir) = find_dir_recursive(extract_dir, "flash", 3) {
        if flash_dir.join("rawprogram0.xml").exists() {
            return Ok(flash_dir);
        }
    }

    // Last resort: locate rawprogram0.xml anywhere in the tree.
    if let Some(parent) = find_file_parent(extract_dir, "rawprogram0.xml", 4) {
        return Ok(parent);
    }

    Err(
        "Could not find flash directory or rawprogram0.xml in the extracted archive. \
         The archive may be corrupt or in an unsupported format."
            .to_string(),
    )
}

/// Recursively search for a directory with a given name, up to max_depth levels
fn find_dir_recursive(base: &Path, name: &str, max_depth: u32) -> Option<PathBuf> {
    if max_depth == 0 {
        return None;
    }

    let entries = fs::read_dir(base).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|n| n.to_str()) == Some(name) {
                return Some(path);
            }
            if let Some(found) = find_dir_recursive(&path, name, max_depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

/// Find the parent directory of a file, searching recursively up to max_depth
fn find_file_parent(base: &Path, filename: &str, max_depth: u32) -> Option<PathBuf> {
    if max_depth == 0 {
        return None;
    }

    let entries = fs::read_dir(base).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some(filename) {
            return Some(base.to_path_buf());
        }
        if path.is_dir() {
            if let Some(found) = find_file_parent(&path, filename, max_depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

/// Validate that all required files for QDL flashing exist in the flash directory
fn validate_required_files(flash_dir: &Path) -> Result<(), String> {
    if !flash_dir.join(FIREHOSE_ELF).exists() {
        log_error!(
            "qdl::extract",
            "Missing firehose programmer: {}",
            FIREHOSE_ELF
        );
        return Err(format!(
            "Missing required file: {}. The archive may be incomplete.",
            FIREHOSE_ELF
        ));
    }

    for filename in REQUIRED_FILES {
        if !flash_dir.join(filename).exists() {
            log_error!("qdl::extract", "Missing required file: {}", filename);
            return Err(format!(
                "Missing required file: {}. The archive may be incomplete.",
                filename
            ));
        }
    }

    Ok(())
}

/// Return a boxed reader for the archive, picking a decompressor by extension
/// (.tar, .tar.xz, .tar.gz, .tar.bz2, .tar.zst).
pub fn open_tar_reader(path: &Path) -> Result<Box<dyn std::io::Read>, String> {
    let file = fs::File::open(path).map_err(|e| format!("Failed to open archive: {}", e))?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    if filename.ends_with(".tar.xz") {
        Ok(Box::new(xz2::read::XzDecoder::new(file)))
    } else if filename.ends_with(".tar.gz") || filename.ends_with(".tar.gzip") {
        Ok(Box::new(flate2::read::GzDecoder::new(file)))
    } else if filename.ends_with(".tar.bz2") {
        Ok(Box::new(bzip2::read::BzDecoder::new(file)))
    } else if filename.ends_with(".tar.zst") || filename.ends_with(".tar.zstd") {
        let decoder = zstd::stream::Decoder::new(file)
            .map_err(|e| format!("Failed to create zstd decoder: {}", e))?;
        Ok(Box::new(decoder))
    } else {
        Ok(Box::new(file))
    }
}

/// Unpack a TAR archive, rejecting entries with `../` components to prevent
/// path traversal (CWE-22 / ZipSlip).
fn safe_unpack<R: std::io::Read>(reader: R, extract_dir: &Path) -> Result<(), String> {
    let mut archive = tar::Archive::new(reader);
    archive.set_preserve_permissions(false);
    archive.set_unpack_xattrs(false);

    for entry in archive
        .entries()
        .map_err(|e| format!("Failed to read archive entries: {}", e))?
    {
        let mut entry = entry.map_err(|e| format!("Failed to read archive entry: {}", e))?;
        let path = entry
            .path()
            .map_err(|e| format!("Failed to read entry path: {}", e))?;

        for component in path.components() {
            if matches!(component, Component::ParentDir) {
                return Err("Archive contains path traversal entry (../)".to_string());
            }
        }

        let full_path = extract_dir.join(&path);
        entry
            .unpack_in(extract_dir)
            .map_err(|e| format!("Failed to extract {}: {}", full_path.display(), e))?;
    }
    Ok(())
}

/// Clean up an extracted QDL archive directory
pub fn cleanup_extraction(extract_dir: &Path) {
    if extract_dir.exists() {
        log_info!(
            "qdl::extract",
            "Cleaning up extraction directory: {}",
            extract_dir.display()
        );
        if let Err(e) = fs::remove_dir_all(extract_dir) {
            log_error!(
                "qdl::extract",
                "Failed to clean up extraction directory: {}",
                e
            );
        }
    }
}

// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! TAR archive extraction for QDL images. Archives contain flash/prog_firehose_ddr.elf (Sahara programmer),
//! flash/rawprogram0.xml (partition instructions), flash/patch0.xml (post-flash patches), and partition images.

use std::fs;
use std::path::{Component, Path, PathBuf};

use tar::EntryType;

use crate::config;
use crate::{log_error, log_info};

/// Partition instructions every QDL archive carries
pub const RAWPROGRAM_XML: &str = "rawprogram0.xml";

/// Optional post-flash patches beside `RAWPROGRAM_XML`
pub const PATCH_XML: &str = "patch0.xml";

/// Required files that must exist after extraction for QDL flashing
pub const REQUIRED_FILES: &[&str] = &[RAWPROGRAM_XML];

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

    let extract_dir = extraction_dir(output_dir);
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

pub fn extraction_dir(output_dir: &Path) -> PathBuf {
    output_dir.join(config::flash::QDL_EXTRACT_DIR)
}

/// Find the flash-files directory in the extracted archive (a "flash" dir or
/// rawprogram0.xml directly), handling nesting like arduino-images/flash/.
fn find_flash_dir(extract_dir: &Path) -> Result<PathBuf, String> {
    if extract_dir.join(RAWPROGRAM_XML).exists() {
        return Ok(extract_dir.to_path_buf());
    }

    if let Some(flash_dir) = find_dir_recursive(extract_dir, "flash", 3) {
        if flash_dir.join(RAWPROGRAM_XML).exists() {
            return Ok(flash_dir);
        }
    }

    // Last resort: locate rawprogram0.xml anywhere in the tree.
    if let Some(parent) = find_file_parent(extract_dir, RAWPROGRAM_XML, 4) {
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

/// Unpack only regular files and directories: no `..` (CWE-22 ZipSlip), links, devices or FIFOs.
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
            .map_err(|e| format!("Failed to read entry path: {}", e))?
            .into_owned();

        for component in path.components() {
            if matches!(component, Component::ParentDir) {
                return Err("Archive contains path traversal entry (../)".to_string());
            }
        }

        let entry_type = entry.header().entry_type();
        match entry_type {
            EntryType::Regular | EntryType::Continuous | EntryType::Directory => {}
            EntryType::XGlobalHeader
            | EntryType::XHeader
            | EntryType::GNULongName
            | EntryType::GNULongLink => continue,
            other => {
                log_error!(
                    "qdl::extract",
                    "Refusing archive entry of type {:?}: {}",
                    other,
                    path.display()
                );
                return Err(format!(
                    "Archive contains an unsupported entry ({:?}): {}",
                    other,
                    path.display()
                ));
            }
        }

        let full_path = extract_dir.join(&path);
        entry
            .unpack_in(extract_dir)
            .map_err(|e| format!("Failed to extract {}: {}", full_path.display(), e))?;
    }
    Ok(())
}

/// Resolve a rawprogram `filename` from `flash_dir`, refusing any path that leaves `root` (links included).
pub fn resolve_flash_file(
    root: &Path,
    flash_dir: &Path,
    filename: &str,
) -> Result<PathBuf, String> {
    let outside = || {
        log_error!(
            "qdl::extract",
            "Refusing rawprogram file outside the extraction: {:?}",
            filename
        );
        format!(
            "rawprogram0.xml points outside the extracted archive: {:?}",
            filename
        )
    };
    let root = root
        .canonicalize()
        .map_err(|e| format!("Failed to resolve the extraction directory: {}", e))?;
    let mut resolved = flash_dir
        .canonicalize()
        .map_err(|e| format!("Failed to resolve the flash directory: {}", e))?;
    if !resolved.starts_with(&root) {
        return Err(outside());
    }
    for component in Path::new(filename).components() {
        match component {
            Component::Normal(part) => resolved.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() || !resolved.starts_with(&root) {
                    return Err(outside());
                }
            }
            Component::RootDir | Component::Prefix(_) => return Err(outside()),
        }
    }
    if !resolved.starts_with(&root) {
        return Err(outside());
    }
    match resolved.canonicalize() {
        Ok(real) if real.starts_with(&root) => Ok(resolved),
        Ok(_) => Err(outside()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(resolved),
        Err(e) => Err(format!("Failed to resolve {:?}: {}", filename, e)),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::utils::test_scratch_dir;

    // Arduino UNO Q: flash files in `arduino-images/flash/`, the rootfs one level up (`../disk-sdcard.img.root`).
    fn arduino_layout(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
        let dir = test_scratch_dir(tag);
        let root = extraction_dir(&dir);
        let flash_dir = root.join("arduino-images").join("flash");
        fs::create_dir_all(&flash_dir).unwrap();
        fs::write(
            root.join("arduino-images").join("disk-sdcard.img.root"),
            b"rootfs",
        )
        .unwrap();
        (dir, root, flash_dir)
    }

    #[test]
    fn rawprogram_files_resolve_only_inside_the_extraction() {
        let (dir, root, flash_dir) = arduino_layout("resolve");
        let rootfs = resolve_flash_file(&root, &flash_dir, "../disk-sdcard.img.root").unwrap();
        assert_eq!(
            rootfs,
            root.canonicalize()
                .unwrap()
                .join("arduino-images")
                .join("disk-sdcard.img.root")
        );
        assert!(resolve_flash_file(&root, &flash_dir, "./gpt_main0.bin").is_ok());
        assert!(resolve_flash_file(&root, &flash_dir, "missing.bin").is_ok());
        for escape in ["../../../outside.img", "/etc/passwd"] {
            assert!(
                resolve_flash_file(&root, &flash_dir, escape).is_err(),
                "{escape}"
            );
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_link_out_of_the_extraction_is_refused() {
        let (dir, root, flash_dir) = arduino_layout("resolve-link");
        let victim = dir.join("user.img");
        fs::write(&victim, b"mine").unwrap();
        std::os::unix::fs::symlink(&victim, flash_dir.join("rootfs.img")).unwrap();
        assert!(resolve_flash_file(&root, &flash_dir, "rootfs.img").is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    fn tar_with(entry_type: tar::EntryType, path: &str) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        let mut file = tar::Header::new_gnu();
        file.set_size(4);
        file.set_mode(0o644);
        file.set_cksum();
        builder
            .append_data(&mut file, "flash/rawprogram0.xml", &b"<x/>"[..])
            .unwrap();
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(entry_type);
        header.set_size(0);
        header.set_mode(0o644);
        if matches!(entry_type, tar::EntryType::Symlink | tar::EntryType::Link) {
            builder
                .append_link(&mut header, path, "flash/rawprogram0.xml")
                .unwrap();
        } else {
            header.set_cksum();
            builder.append_data(&mut header, path, &b""[..]).unwrap();
        }
        builder.into_inner().unwrap()
    }

    #[test]
    fn only_files_and_directories_are_unpacked() {
        let dir = test_scratch_dir("allowlist");
        let unpack = |entry_type, path: &str| {
            let target = dir.join(format!("{entry_type:?}"));
            fs::create_dir_all(&target).unwrap();
            safe_unpack(&tar_with(entry_type, path)[..], &target)
        };
        assert!(unpack(tar::EntryType::Regular, "flash/patch0.xml").is_ok());
        assert!(unpack(tar::EntryType::Directory, "flash/sub").is_ok());
        for refused in [
            tar::EntryType::Symlink,
            tar::EntryType::Link,
            tar::EntryType::Fifo,
            tar::EntryType::Char,
            tar::EntryType::Block,
        ] {
            assert!(
                unpack(refused, "flash/disk-sdcard.img.root").is_err(),
                "{refused:?}"
            );
        }
        assert!(!dir
            .join("Symlink")
            .join("flash")
            .join("disk-sdcard.img.root")
            .exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}

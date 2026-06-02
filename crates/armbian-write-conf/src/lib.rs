//! Write a config file into a RAW disk image's ext4 rootfs in userspace (no mount/privileges), then validate.
//! Parses partition scheme (GPT/MBR), locates the Linux ext4 rootfs, writes via `armbian-ext4fs`, re-validates read-only with `ext4-view`.

use std::fmt;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use armbian_ext4fs::{BlockDevice, Ext4, BLOCK_SIZE};

mod detect;
mod validate;

pub use detect::Scheme;

/// Outcome of a successful write-and-validate operation.
#[derive(Debug, Clone)]
pub struct WriteConfReport {
    /// Partition scheme of the image ("GPT" or "MBR").
    pub scheme: &'static str,
    /// Byte offset of the rootfs partition within the image.
    pub partition_offset: u64,
    /// Byte length of the rootfs partition.
    pub partition_len: u64,
    /// Destination path written inside the rootfs.
    pub dest_path: String,
    /// Number of content bytes written.
    pub bytes_written: usize,
    /// True only when post-write validation fully succeeded.
    pub validated: bool,
}

/// Errors that can occur while writing into an image.
#[derive(Debug)]
pub enum WriteConfError {
    /// Underlying I/O failure on the image file.
    Io(std::io::Error),
    /// Partition table could not be parsed or is unsupported.
    UnsupportedImage(String),
    /// No ext4 Linux rootfs partition was found.
    NoExt4Rootfs(String),
    /// The ext4 write layer reported a failure.
    Ext4(String),
    /// Post-write validation failed (checksum/corruption/mismatch).
    ValidationFailed(String),
}

impl fmt::Display for WriteConfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteConfError::Io(e) => write!(f, "I/O error: {e}"),
            WriteConfError::UnsupportedImage(m) => write!(f, "unsupported image: {m}"),
            WriteConfError::NoExt4Rootfs(m) => write!(f, "no ext4 rootfs: {m}"),
            WriteConfError::Ext4(m) => write!(f, "ext4 write error: {m}"),
            WriteConfError::ValidationFailed(m) => write!(f, "validation failed: {m}"),
        }
    }
}

impl std::error::Error for WriteConfError {}

impl From<std::io::Error> for WriteConfError {
    fn from(e: std::io::Error) -> Self {
        WriteConfError::Io(e)
    }
}

/// ext4-rs block device over a partition window of the image file. Offsets from
/// the ext4 layer are partition-relative, so the partition base is added here.
struct PartDev {
    file: Mutex<std::fs::File>,
    base: u64,
}

impl BlockDevice for PartDev {
    fn read_offset(&self, offset: usize) -> Vec<u8> {
        let mut f = self.file.lock().unwrap();
        f.seek(SeekFrom::Start(self.base + offset as u64)).unwrap();
        let mut buf = vec![0u8; BLOCK_SIZE];
        let mut filled = 0;
        while filled < BLOCK_SIZE {
            match std::io::Read::read(&mut *f, &mut buf[filled..]) {
                Ok(0) => break, // short read near EOF: leave zeros
                Ok(n) => filled += n,
                Err(e) => panic!("read_offset error: {e}"),
            }
        }
        buf
    }

    fn write_offset(&self, offset: usize, data: &[u8]) {
        let mut f = self.file.lock().unwrap();
        f.seek(SeekFrom::Start(self.base + offset as u64)).unwrap();
        f.write_all(data).unwrap();
    }
}

/// Write `content` to `dest_path` in the image's ext4 rootfs, then validate read-only; [`WriteConfReport`]'s `validated` is true only if it read back identically and the tree walk found no corruption.
/// Errors ([`WriteConfError`]) on non-ext4 raw image, write failure, or bad validation.
pub fn write_file_into_image(
    image_path: &Path,
    dest_path: &str,
    content: &[u8],
) -> Result<WriteConfReport, WriteConfError> {
    let part = detect::detect_rootfs(image_path)?;

    // Open the image read+write and wrap the rootfs window for ext4-rs.
    let file = OpenOptions::new().read(true).write(true).open(image_path)?;
    let dev = Arc::new(PartDev {
        file: Mutex::new(file),
        base: part.offset,
    });
    let fs = Ext4::open(dev.clone());

    let ino = fs
        .ext4_file_open(dest_path, "w+")
        .map_err(|e| WriteConfError::Ext4(format!("open {dest_path}: {e:?}")))?;
    let written = fs
        .ext4_file_write(ino as u64, 0, content)
        .map_err(|e| WriteConfError::Ext4(format!("write {dest_path}: {e:?}")))?;

    // Flush to disk before reloading for validation, then release handles.
    dev.file
        .lock()
        .unwrap()
        .sync_all()
        .map_err(WriteConfError::Io)?;
    drop(fs);
    drop(dev);

    validate::validate(image_path, part.offset, dest_path, content)?;

    Ok(WriteConfReport {
        scheme: part.scheme.as_str(),
        partition_offset: part.offset,
        partition_len: part.len,
        dest_path: dest_path.to_string(),
        bytes_written: written,
        validated: true,
    })
}

/// Write `content` to `dest_path` in a BARE ext4 image (no partition table, superblock at byte 0; e.g. Armbian QDL `disk-sdcard.img.root` blobs). Confirms ext4 magic at 0, writes via `armbian-ext4fs`, validates read-only.
/// Errors ([`WriteConfError`]) on non-bare-ext4 image, write failure, or bad validation.
pub fn write_file_into_bare_ext4_image(
    image_path: &Path,
    dest_path: &str,
    content: &[u8],
) -> Result<WriteConfReport, WriteConfError> {
    // The filesystem starts at file byte 0 (no partition table).
    detect::verify_ext4(image_path, 0)?;

    // Open the image read+write; the ext4-rs window covers the whole file.
    let file = OpenOptions::new().read(true).write(true).open(image_path)?;
    let dev = Arc::new(PartDev {
        file: Mutex::new(file),
        base: 0,
    });
    let fs = Ext4::open(dev.clone());

    let ino = fs
        .ext4_file_open(dest_path, "w+")
        .map_err(|e| WriteConfError::Ext4(format!("open {dest_path}: {e:?}")))?;
    let written = fs
        .ext4_file_write(ino as u64, 0, content)
        .map_err(|e| WriteConfError::Ext4(format!("write {dest_path}: {e:?}")))?;

    // Flush to disk before reloading for validation, then release handles.
    dev.file
        .lock()
        .unwrap()
        .sync_all()
        .map_err(WriteConfError::Io)?;
    drop(fs);
    drop(dev);

    validate::validate(image_path, 0, dest_path, content)?;

    let len = std::fs::metadata(image_path)?.len();

    Ok(WriteConfReport {
        scheme: "bare-ext4",
        partition_offset: 0,
        partition_len: len,
        dest_path: dest_path.to_string(),
        bytes_written: written,
        validated: true,
    })
}

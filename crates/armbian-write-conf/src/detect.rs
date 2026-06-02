//! Partition-scheme detection and ext4 rootfs location for RAW disk images.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::WriteConfError;

/// Logical sector size assumed for image layout (Armbian images use 512).
const SECTOR_SIZE: u64 = 512;
/// Offset of the ext4 superblock within a partition, and its magic value.
const EXT4_SB_OFFSET: u64 = 0x438;
const EXT4_MAGIC: [u8; 2] = [0x53, 0xEF];

/// Partition scheme of a RAW disk image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Gpt,
    Mbr,
}

impl Scheme {
    /// Human-readable label used in reports.
    pub fn as_str(&self) -> &'static str {
        match self {
            Scheme::Gpt => "GPT",
            Scheme::Mbr => "MBR",
        }
    }
}

/// A located rootfs partition: scheme plus its byte window in the image.
#[derive(Debug, Clone)]
pub struct RootfsPartition {
    pub scheme: Scheme,
    pub offset: u64,
    pub len: u64,
}

/// Detect partition scheme (GPT if protective-MBR type 0xEE or "EFI PART" sig, else MBR) and locate the Linux
/// ext4 rootfs: prefer a root-named/typed partition, else largest used, then gate on ext4 superblock magic.
pub fn detect_rootfs(image_path: &Path) -> Result<RootfsPartition, WriteConfError> {
    let scheme = detect_scheme(image_path)?;
    let (offset, len) = match scheme {
        Scheme::Gpt => locate_gpt_rootfs(image_path)?,
        Scheme::Mbr => locate_mbr_rootfs(image_path)?,
    };
    verify_ext4(image_path, offset)?;
    Ok(RootfsPartition {
        scheme,
        offset,
        len,
    })
}

/// Decide GPT vs MBR by inspecting the protective MBR entry and the LBA1 signature.
fn detect_scheme(image_path: &Path) -> Result<Scheme, WriteConfError> {
    let mut f = File::open(image_path)?;

    // Protective-MBR first-entry type byte lives at 0x1C2.
    let mut pmbr_type = [0u8; 1];
    f.seek(SeekFrom::Start(0x1C2))?;
    f.read_exact(&mut pmbr_type)?;

    // GPT header signature "EFI PART" at start of LBA1.
    let mut sig = [0u8; 8];
    f.seek(SeekFrom::Start(SECTOR_SIZE))?;
    let sig_read = f.read(&mut sig).unwrap_or(0);

    let has_gpt_sig = sig_read == 8 && &sig == b"EFI PART";
    if pmbr_type[0] == 0xEE || has_gpt_sig {
        Ok(Scheme::Gpt)
    } else {
        Ok(Scheme::Mbr)
    }
}

/// Locate the rootfs partition in a GPT-partitioned image.
fn locate_gpt_rootfs(image_path: &Path) -> Result<(u64, u64), WriteConfError> {
    let mut f = File::open(image_path)?;
    let gpt = gptman::GPT::read_from(&mut f, SECTOR_SIZE)
        .map_err(|e| WriteConfError::UnsupportedImage(format!("GPT parse failed: {e}")))?;

    // Linux filesystem-data partition type GUID (0FC63DAF-8483-4772-8E79-3D69D8477DE4).
    const LINUX_FS_GUID: [u8; 16] = [
        0xAF, 0x3D, 0xC6, 0x0F, 0x83, 0x84, 0x72, 0x47, 0x8E, 0x79, 0x3D, 0x69, 0xD8, 0x47, 0x7D,
        0xE4,
    ];

    let mut by_name: Option<(u64, u64)> = None;
    let mut by_type: Option<(u64, u64)> = None;
    let mut largest: Option<(u64, u64)> = None;

    for (_, p) in gpt.iter() {
        if !p.is_used() {
            continue;
        }
        let start = p.starting_lba * SECTOR_SIZE;
        let len = (p.ending_lba - p.starting_lba + 1) * SECTOR_SIZE;

        if p.partition_name.as_str().to_lowercase().contains("root") && by_name.is_none() {
            by_name = Some((start, len));
        }
        if p.partition_type_guid == LINUX_FS_GUID && by_type.is_none() {
            by_type = Some((start, len));
        }
        if largest.map(|(_, l)| len > l).unwrap_or(true) {
            largest = Some((start, len));
        }
    }

    by_name
        .or(by_type)
        .or(largest)
        .ok_or_else(|| WriteConfError::NoExt4Rootfs("no usable GPT partition found".into()))
}

/// Locate the rootfs partition in an MBR-partitioned image.
fn locate_mbr_rootfs(image_path: &Path) -> Result<(u64, u64), WriteConfError> {
    let mut f = File::open(image_path)?;
    let mbr = mbrman::MBR::read_from(&mut f, SECTOR_SIZE as u32)
        .map_err(|e| WriteConfError::UnsupportedImage(format!("MBR parse failed: {e}")))?;

    let mut linux: Option<(u64, u64)> = None;
    let mut largest: Option<(u64, u64)> = None;

    for (_, p) in mbr.iter() {
        if !p.is_used() {
            continue;
        }
        let start = p.starting_lba as u64 * SECTOR_SIZE;
        let len = p.sectors as u64 * SECTOR_SIZE;

        // 0x83 is the Linux native partition type.
        if p.sys == 0x83 && linux.map(|(_, l)| len > l).unwrap_or(true) {
            linux = Some((start, len));
        }
        if largest.map(|(_, l)| len > l).unwrap_or(true) {
            largest = Some((start, len));
        }
    }

    linux
        .or(largest)
        .ok_or_else(|| WriteConfError::NoExt4Rootfs("no usable MBR partition found".into()))
}

/// Confirm the ext4 superblock magic at the partition base, giving a clearer
/// message for known non-ext4 filesystems (btrfs, f2fs).
pub(crate) fn verify_ext4(image_path: &Path, base: u64) -> Result<(), WriteConfError> {
    let mut f = File::open(image_path)?;
    f.seek(SeekFrom::Start(base + EXT4_SB_OFFSET))?;
    let mut magic = [0u8; 2];
    f.read_exact(&mut magic)?;
    if magic == EXT4_MAGIC {
        return Ok(());
    }

    // btrfs magic "_BHRfS_M" at partition offset 0x10040.
    let mut btrfs = [0u8; 8];
    if f.seek(SeekFrom::Start(base + 0x10040)).is_ok()
        && f.read_exact(&mut btrfs).is_ok()
        && &btrfs == b"_BHRfS_M"
    {
        return Err(WriteConfError::NoExt4Rootfs(
            "rootfs is btrfs, not ext4".into(),
        ));
    }
    // f2fs magic 0xF2F52010 (LE) at partition offset 0x400.
    let mut f2fs = [0u8; 4];
    if f.seek(SeekFrom::Start(base + 0x400)).is_ok()
        && f.read_exact(&mut f2fs).is_ok()
        && f2fs == [0x10, 0x20, 0xF5, 0xF2]
    {
        return Err(WriteConfError::NoExt4Rootfs(
            "rootfs is f2fs, not ext4".into(),
        ));
    }

    Err(WriteConfError::NoExt4Rootfs(
        "ext4 superblock magic 0xEF53 not found at rootfs partition".into(),
    ))
}

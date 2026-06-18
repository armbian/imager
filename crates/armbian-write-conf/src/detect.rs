//! Partition-scheme detection and ext4 rootfs location for RAW disk images.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::WriteConfError;

/// Logical sector sizes we probe: 512 for SD/eMMC, 4096 for UFS (e.g. Radxa Dragon
/// Q6A). The GPT primary header ("EFI PART") sits at LBA1, i.e. one sector in, so the
/// sector size is recovered by checking where that signature lands.
const SECTOR_SIZE_512: u64 = 512;
const SECTOR_SIZE_4096: u64 = 4096;
const GPT_SIG: &[u8; 8] = b"EFI PART";
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
    let (scheme, sector_size) = detect_scheme(image_path)?;
    let (offset, len) = match scheme {
        Scheme::Gpt => locate_gpt_rootfs(image_path, sector_size)?,
        Scheme::Mbr => locate_mbr_rootfs(image_path, sector_size)?,
    };
    verify_ext4(image_path, offset)?;
    Ok(RootfsPartition {
        scheme,
        offset,
        len,
    })
}

/// Decide GPT vs MBR and recover the logical sector size from the "EFI PART" location.
fn detect_scheme(image_path: &Path) -> Result<(Scheme, u64), WriteConfError> {
    let mut f = File::open(image_path)?;

    // Protective-MBR first-entry type byte lives at 0x1C2.
    let mut pmbr_type = [0u8; 1];
    f.seek(SeekFrom::Start(0x1C2))?;
    f.read_exact(&mut pmbr_type)?;

    // UFS images put the GPT header at offset 4096, not 512 — probe both so the
    // partition offsets below scale by the right sector size.
    if let Some(sector_size) = probe_gpt_sector_size(&mut f) {
        Ok((Scheme::Gpt, sector_size))
    } else if pmbr_type[0] == 0xEE {
        Ok((Scheme::Gpt, SECTOR_SIZE_512))
    } else {
        Ok((Scheme::Mbr, SECTOR_SIZE_512))
    }
}

/// Return the sector size at whose LBA1 the "EFI PART" signature is found, if any.
fn probe_gpt_sector_size(f: &mut File) -> Option<u64> {
    [SECTOR_SIZE_512, SECTOR_SIZE_4096].into_iter().find(|&s| {
        let mut sig = [0u8; 8];
        f.seek(SeekFrom::Start(s)).is_ok() && f.read_exact(&mut sig).is_ok() && &sig == GPT_SIG
    })
}

/// Locate the rootfs partition in a GPT-partitioned image.
fn locate_gpt_rootfs(image_path: &Path, sector_size: u64) -> Result<(u64, u64), WriteConfError> {
    let mut f = File::open(image_path)?;
    let gpt = gptman::GPT::read_from(&mut f, sector_size)
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
        let start = p.starting_lba * sector_size;
        let len = (p.ending_lba - p.starting_lba + 1) * sector_size;

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
fn locate_mbr_rootfs(image_path: &Path, sector_size: u64) -> Result<(u64, u64), WriteConfError> {
    let mut f = File::open(image_path)?;
    let mbr = mbrman::MBR::read_from(&mut f, sector_size as u32)
        .map_err(|e| WriteConfError::UnsupportedImage(format!("MBR parse failed: {e}")))?;

    let mut linux: Option<(u64, u64)> = None;
    let mut largest: Option<(u64, u64)> = None;

    for (_, p) in mbr.iter() {
        if !p.is_used() {
            continue;
        }
        let start = p.starting_lba as u64 * sector_size;
        let len = p.sectors as u64 * sector_size;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    const LINUX_FS_GUID: [u8; 16] = [
        0xAF, 0x3D, 0xC6, 0x0F, 0x83, 0x84, 0x72, 0x47, 0x8E, 0x79, 0x3D, 0x69, 0xD8, 0x47, 0x7D,
        0xE4,
    ];

    /// Build a 1 MiB GPT image with a single Linux partition spanning the usable area,
    /// carrying a valid ext4 superblock magic. Returns (bytes, expected rootfs offset, len).
    fn make_gpt_image(sector_size: u64) -> (Vec<u8>, u64, u64) {
        let disk_len = 1024 * 1024;
        let mut cur = Cursor::new(vec![0u8; disk_len]);
        let mut gpt = gptman::GPT::new_from(&mut cur, sector_size, [0x11; 16]).unwrap();

        let start = gpt.header.first_usable_lba;
        let end = gpt.header.last_usable_lba;
        gpt[1] = gptman::GPTPartitionEntry {
            partition_type_guid: LINUX_FS_GUID,
            unique_partition_guid: [0x22; 16],
            starting_lba: start,
            ending_lba: end,
            attribute_bits: 0,
            partition_name: "rootfs".into(),
        };
        gpt.write_into(&mut cur).unwrap();

        let mut bytes = cur.into_inner();
        let offset = start * sector_size;
        // Plant the ext4 superblock magic so verify_ext4 accepts the partition.
        let sb = (offset + EXT4_SB_OFFSET) as usize;
        bytes[sb] = EXT4_MAGIC[0];
        bytes[sb + 1] = EXT4_MAGIC[1];

        (bytes, offset, (end - start + 1) * sector_size)
    }

    fn write_temp(bytes: &[u8]) -> tempfile::NamedTempFile {
        let mut tf = tempfile::NamedTempFile::new().unwrap();
        tf.write_all(bytes).unwrap();
        tf.flush().unwrap();
        tf
    }

    #[test]
    fn detects_gpt_rootfs_512() {
        let (bytes, offset, len) = make_gpt_image(SECTOR_SIZE_512);
        let tf = write_temp(&bytes);
        let part = detect_rootfs(tf.path()).unwrap();
        assert_eq!(part.scheme, Scheme::Gpt);
        assert_eq!(part.offset, offset);
        assert_eq!(part.len, len);
    }

    #[test]
    fn detects_gpt_rootfs_4096() {
        let (bytes, offset, len) = make_gpt_image(SECTOR_SIZE_4096);
        let tf = write_temp(&bytes);
        let part = detect_rootfs(tf.path()).unwrap();
        assert_eq!(part.scheme, Scheme::Gpt);
        // 4096-sector image: rootfs starts at a 4096-aligned offset the 512 path would miss.
        assert_eq!(part.offset, offset);
        assert_eq!(part.len, len);
        assert_eq!(offset % SECTOR_SIZE_4096, 0);
    }

    #[test]
    fn probe_returns_sector_size_of_signature() {
        let (b512, _, _) = make_gpt_image(SECTOR_SIZE_512);
        let (b4096, _, _) = make_gpt_image(SECTOR_SIZE_4096);
        let f512 = write_temp(&b512);
        let f4096 = write_temp(&b4096);
        assert_eq!(
            probe_gpt_sector_size(&mut File::open(f512.path()).unwrap()),
            Some(SECTOR_SIZE_512)
        );
        assert_eq!(
            probe_gpt_sector_size(&mut File::open(f4096.path()).unwrap()),
            Some(SECTOR_SIZE_4096)
        );
    }
}

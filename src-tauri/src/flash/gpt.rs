// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Moves the backup GPT of a raw-written image to the device's last LBA (UEFI 2.11 5.3.1).

use std::io::{self, Read, Seek, SeekFrom};

use crate::config::flash::{GPT_IMAGE_SECTOR_SIZE, GPT_MAX_ENTRY_ARRAY_BYTES};
use crate::utils::tagged;
use crate::{log_error, log_info, log_warn};

const MODULE: &str = "flash::gpt";

/// The device was changed and needs a new flash; twin in src/utils/errorUtils.ts.
pub(crate) const TAG_GPT_RELOCATE_FAILED: &str = "[GPT_RELOCATE_FAILED]";
/// Warning: the image was written but its backup GPT stays where the image put it.
pub(crate) const TAG_GPT_RELOCATE_SKIPPED: &str = "[GPT_RELOCATE_SKIPPED]";
/// Warning: the device wraps writes past its real capacity; the image on it is intact.
pub(crate) const TAG_GPT_FAKE_CAPACITY: &str = "[GPT_FAKE_CAPACITY]";

const SECTOR: u64 = GPT_IMAGE_SECTOR_SIZE;
const SECTOR_LEN: usize = GPT_IMAGE_SECTOR_SIZE as usize;

const MBR_SIGNATURE: usize = 510;
const MBR_BOOT_SIGNATURE: [u8; 2] = [0x55, 0xAA];
const MBR_RECORDS: usize = 446;
const MBR_RECORD_COUNT: usize = 4;
const MBR_RECORD_LEN: usize = 16;
const MBR_RECORD_TYPE: usize = 4;
const MBR_RECORD_START: usize = 8;
const MBR_RECORD_SIZE: usize = 12;
const MBR_PROTECTIVE_TYPE: u8 = 0xEE;

const ISO9660_ID_OFFSET: u64 = 0x8001;
const ISO9660_ID: &[u8; 5] = b"CD001";

const GPT_SIGNATURE: &[u8; 8] = b"EFI PART";
// Protective MBR, primary header, one entry-array sector, backup header.
const GPT_MIN_IMAGE_SECTORS: u64 = 4;
const HDR_SIZE: usize = 12;
const HDR_CRC: usize = 16;
const HDR_MY_LBA: usize = 24;
const HDR_ALT_LBA: usize = 32;
const HDR_FIRST_USABLE: usize = 40;
const HDR_LAST_USABLE: usize = 48;
const HDR_DISK_GUID: usize = 56;
const HDR_ENTRY_LBA: usize = 72;
const HDR_ENTRY_COUNT: usize = 80;
const HDR_ENTRY_SIZE: usize = 84;
const HDR_ENTRIES_CRC: usize = 88;
const HDR_MIN_SIZE: usize = 92;

const GUID_LEN: usize = 16;
const ENTRY_MIN_SIZE: u32 = 128;
const ENTRY_TYPE_GUID: usize = 0;
const ENTRY_FIRST_LBA: usize = 32;
const ENTRY_LAST_LBA: usize = 40;
// C12A7328-F81F-11D2-BA4B-00A0C93EC93B in its on-disk mixed-endian form.
const ESP_TYPE_GUID: [u8; GUID_LEN] = [
    0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11, 0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B,
];

/// Whole-sector positional I/O on the target device.
pub(crate) trait SectorDevice {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()>;
    fn write_at(&mut self, offset: u64, buf: &[u8]) -> io::Result<()>;
    /// Push written data to the medium before reporting success.
    fn flush(&mut self) -> io::Result<()>;
    /// Drop cached pages so the next read of the range reaches the medium.
    fn drop_cache(&mut self, _offset: u64, _len: u64) {}
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DeviceGeometry {
    pub size: u64,
    pub sector_size: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Relocated,
    Skipped(String),
    /// Tagged message for the user; the image on the device is intact.
    Warning(String),
}

enum Stop {
    Skip(String),
    FakeCapacity(String),
    Warn(String),
}

#[derive(Clone)]
struct Header {
    sector: Vec<u8>,
    size: usize,
    my_lba: u64,
    alt_lba: u64,
    first_usable: u64,
    last_usable: u64,
    disk_guid: [u8; GUID_LEN],
    entry_lba: u64,
    entry_count: u32,
    entry_size: u32,
    entries_crc: u32,
}

struct Plan {
    image_sectors: u64,
    device_sectors: u64,
    old_last_usable: u64,
    new_array_lba: u64,
    new_last_usable: u64,
    array: Vec<u8>,
    array_bytes: usize,
    new_primary: Vec<u8>,
    new_backup: Vec<u8>,
    new_pmbr: Option<Vec<u8>>,
    alias_sectors: Vec<u64>,
}

fn le_u32(b: &[u8], at: usize) -> u32 {
    let mut a = [0u8; 4];
    a.copy_from_slice(&b[at..at + 4]);
    u32::from_le_bytes(a)
}

fn le_u64(b: &[u8], at: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[at..at + 8]);
    u64::from_le_bytes(a)
}

fn put_u32(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn put_u64(b: &mut [u8], at: usize, v: u64) {
    b[at..at + 8].copy_from_slice(&v.to_le_bytes());
}

fn header_crc(raw: &[u8]) -> u32 {
    let mut h = crc32fast::Hasher::new();
    h.update(&raw[..HDR_CRC]);
    h.update(&[0u8; 4]);
    h.update(&raw[HDR_CRC + 4..]);
    h.finalize()
}

fn parse_header(sector: &[u8]) -> Result<Header, String> {
    if sector.len() != SECTOR_LEN {
        return Err(format!("header sector is {} bytes", sector.len()));
    }
    if &sector[..GPT_SIGNATURE.len()] != GPT_SIGNATURE {
        return Err("no EFI PART signature".into());
    }
    let size = le_u32(sector, HDR_SIZE) as usize;
    if !(HDR_MIN_SIZE..=SECTOR_LEN).contains(&size) {
        return Err(format!("header size {size} out of range"));
    }
    if header_crc(&sector[..size]) != le_u32(sector, HDR_CRC) {
        return Err("header CRC mismatch".into());
    }
    let entry_size = le_u32(sector, HDR_ENTRY_SIZE);
    if entry_size < ENTRY_MIN_SIZE || !entry_size.is_power_of_two() {
        return Err(format!("partition entry size {entry_size} is invalid"));
    }
    let mut disk_guid = [0u8; GUID_LEN];
    disk_guid.copy_from_slice(&sector[HDR_DISK_GUID..HDR_DISK_GUID + GUID_LEN]);
    Ok(Header {
        sector: sector.to_vec(),
        size,
        my_lba: le_u64(sector, HDR_MY_LBA),
        alt_lba: le_u64(sector, HDR_ALT_LBA),
        first_usable: le_u64(sector, HDR_FIRST_USABLE),
        last_usable: le_u64(sector, HDR_LAST_USABLE),
        disk_guid,
        entry_lba: le_u64(sector, HDR_ENTRY_LBA),
        entry_count: le_u32(sector, HDR_ENTRY_COUNT),
        entry_size,
        entries_crc: le_u32(sector, HDR_ENTRIES_CRC),
    })
}

impl Header {
    fn array_bytes(&self) -> Result<u64, String> {
        let bytes = u64::from(self.entry_count) * u64::from(self.entry_size);
        if bytes == 0 || bytes > GPT_MAX_ENTRY_ARRAY_BYTES {
            return Err(format!("partition entry array of {bytes} bytes"));
        }
        Ok(bytes)
    }

    fn rebuilt(&self, my_lba: u64, alt_lba: u64, entry_lba: u64, last_usable: u64) -> Vec<u8> {
        let mut s = self.sector.clone();
        put_u64(&mut s, HDR_MY_LBA, my_lba);
        put_u64(&mut s, HDR_ALT_LBA, alt_lba);
        put_u64(&mut s, HDR_ENTRY_LBA, entry_lba);
        put_u64(&mut s, HDR_LAST_USABLE, last_usable);
        let crc = header_crc(&s[..self.size]);
        put_u32(&mut s, HDR_CRC, crc);
        s
    }
}

fn is_pure_pmbr(lba0: &[u8]) -> bool {
    let record = |i: usize| &lba0[MBR_RECORDS + i * MBR_RECORD_LEN..][..MBR_RECORD_LEN];
    let first = record(0);
    lba0[MBR_SIGNATURE..MBR_SIGNATURE + 2] == MBR_BOOT_SIGNATURE
        && first[MBR_RECORD_TYPE] == MBR_PROTECTIVE_TYPE
        && le_u32(first, MBR_RECORD_START) == 1
        && (1..MBR_RECORD_COUNT).all(|i| record(i).iter().all(|b| *b == 0))
}

fn is_constant(sector: &[u8]) -> bool {
    sector.iter().all(|b| *b == sector[0])
}

fn byte_range(lba: u64, sectors: u64) -> Result<(u64, usize), String> {
    let offset = lba
        .checked_mul(SECTOR)
        .ok_or_else(|| format!("LBA {lba} is out of range"))?;
    let len = sectors
        .checked_mul(SECTOR)
        .and_then(|l| usize::try_from(l).ok())
        .ok_or_else(|| format!("{sectors} sectors is out of range"))?;
    Ok((offset, len))
}

fn read_image<I: Read + Seek>(image: &mut I, lba: u64, sectors: u64) -> Result<Vec<u8>, String> {
    let (offset, len) = byte_range(lba, sectors)?;
    let mut buf = vec![0u8; len];
    image
        .seek(SeekFrom::Start(offset))
        .and_then(|_| image.read_exact(&mut buf))
        .map_err(|e| format!("Failed to read image at LBA {lba}: {e}"))?;
    Ok(buf)
}

fn read_device<D: SectorDevice>(dev: &mut D, lba: u64, sectors: u64) -> Result<Vec<u8>, String> {
    let (offset, len) = byte_range(lba, sectors)?;
    dev.drop_cache(offset, len as u64);
    let mut buf = vec![0u8; len];
    dev.read_at(offset, &mut buf)
        .map_err(|e| format!("Failed to read device at LBA {lba}: {e}"))?;
    Ok(buf)
}

/// Every write goes through here: whole sectors, never past the device end.
fn write_device<D: SectorDevice>(
    dev: &mut D,
    device_sectors: u64,
    lba: u64,
    data: &[u8],
) -> Result<(), String> {
    if data.is_empty() || !data.len().is_multiple_of(SECTOR_LEN) {
        return Err(format!(
            "write of {} bytes is not whole sectors",
            data.len()
        ));
    }
    let sectors = (data.len() / SECTOR_LEN) as u64;
    if lba
        .checked_add(sectors)
        .is_none_or(|end| end > device_sectors)
    {
        return Err(format!(
            "write of {sectors} sectors at LBA {lba} passes the device end"
        ));
    }
    dev.write_at(lba * SECTOR, data)
        .map_err(|e| format!("Failed to write device at LBA {lba}: {e}"))
}

fn flush_device<D: SectorDevice>(dev: &mut D, what: &str) -> Result<(), String> {
    dev.flush()
        .map_err(|e| format!("Failed to flush {what}: {e}"))
}

/// Pairs (target, alias) where a tail target lands below the image end on a card that wraps at a power of two.
fn tail_aliases(image_sectors: u64, device_sectors: u64, first: u64, last: u64) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    let mut wrap = match image_sectors.checked_next_power_of_two() {
        Some(p) => p,
        None => return out,
    };
    while wrap < device_sectors {
        for target in first..=last {
            let alias = target % wrap;
            if alias < image_sectors {
                out.push((target, alias));
            }
        }
        wrap = match wrap.checked_mul(2) {
            Some(p) => p,
            None => break,
        };
    }
    out
}

fn prepare<D: SectorDevice, I: Read + Seek>(
    dev: &mut D,
    image: &mut I,
    image_size: u64,
    geometry: DeviceGeometry,
) -> Result<Plan, Stop> {
    let skip = |r: String| Stop::Skip(r);
    if geometry.sector_size != SECTOR {
        return Err(skip(format!(
            "logical sector size is {} bytes, not {SECTOR}",
            geometry.sector_size
        )));
    }
    if !image_size.is_multiple_of(SECTOR) || !geometry.size.is_multiple_of(SECTOR) {
        return Err(skip(format!(
            "image ({image_size} bytes) or device ({} bytes) is not whole sectors",
            geometry.size
        )));
    }
    let m = image_size / SECTOR;
    let n = geometry.size / SECTOR;
    if n <= m {
        return Err(skip("the image fills the device".into()));
    }
    if m < GPT_MIN_IMAGE_SECTORS {
        return Err(skip("image too small to hold a GPT".into()));
    }

    let lba0 = read_image(image, 0, 1).map_err(Stop::Warn)?;
    if !is_pure_pmbr(&lba0) {
        return Err(skip("LBA 0 is not a pure protective MBR".into()));
    }
    if image_size >= ISO9660_ID_OFFSET + ISO9660_ID.len() as u64 {
        let mut id = [0u8; 5];
        image
            .seek(SeekFrom::Start(ISO9660_ID_OFFSET))
            .and_then(|_| image.read_exact(&mut id))
            .map_err(|e| Stop::Warn(format!("Failed to read image: {e}")))?;
        if &id == ISO9660_ID {
            return Err(skip("image is an ISO9660 hybrid".into()));
        }
    }

    let lba1 = read_image(image, 1, 1).map_err(Stop::Warn)?;
    let primary = parse_header(&lba1).map_err(|e| skip(format!("primary GPT: {e}")))?;
    if primary.my_lba != 1 || primary.alt_lba != m - 1 {
        return Err(skip(format!(
            "primary GPT has MyLBA {} and AlternateLBA {}, image ends at LBA {}",
            primary.my_lba,
            primary.alt_lba,
            m - 1
        )));
    }
    let array_bytes = primary
        .array_bytes()
        .map_err(|e| skip(format!("primary GPT: {e}")))?;
    let entry_sectors = array_bytes.div_ceil(SECTOR);
    let array_end = primary.entry_lba.checked_add(entry_sectors);
    if primary.entry_lba < 2
        || array_end.is_none_or(|end| end > primary.first_usable)
        || primary.first_usable > primary.last_usable
        || primary.last_usable >= m - 1
    {
        return Err(skip("primary GPT layout is not sane".into()));
    }
    let array = read_image(image, primary.entry_lba, entry_sectors).map_err(Stop::Warn)?;
    let array_bytes = array_bytes as usize;
    if crc32fast::hash(&array[..array_bytes]) != primary.entries_crc {
        return Err(skip("primary partition entry array CRC mismatch".into()));
    }

    let backup_sector = read_image(image, m - 1, 1).map_err(Stop::Warn)?;
    let backup = parse_header(&backup_sector).map_err(|e| skip(format!("backup GPT: {e}")))?;
    let backup_array_end = backup.entry_lba.checked_add(entry_sectors);
    if backup.my_lba != m - 1
        || backup.alt_lba != 1
        || backup.disk_guid != primary.disk_guid
        || backup.entries_crc != primary.entries_crc
        || backup.entry_count != primary.entry_count
        || backup.entry_size != primary.entry_size
        || backup.first_usable != primary.first_usable
        || backup.last_usable != primary.last_usable
        || backup.entry_lba <= primary.last_usable
        || backup_array_end.is_none_or(|end| end > m - 1)
    {
        return Err(skip("backup GPT does not mirror the primary".into()));
    }
    let backup_array = read_image(image, backup.entry_lba, entry_sectors).map_err(Stop::Warn)?;
    if backup_array[..array_bytes] != array[..array_bytes] {
        return Err(skip("backup partition entry array differs".into()));
    }

    let entry_size = primary.entry_size as usize;
    let mut has_esp = false;
    for entry in array[..array_bytes].chunks_exact(entry_size) {
        let type_guid = &entry[ENTRY_TYPE_GUID..ENTRY_TYPE_GUID + GUID_LEN];
        if type_guid.iter().all(|b| *b == 0) {
            continue;
        }
        let first = le_u64(entry, ENTRY_FIRST_LBA);
        let last = le_u64(entry, ENTRY_LAST_LBA);
        if first > last || first < primary.first_usable || last > primary.last_usable {
            return Err(skip(format!(
                "partition {first}..{last} lies outside the usable area"
            )));
        }
        has_esp |= type_guid == ESP_TYPE_GUID;
    }
    if !has_esp {
        return Err(skip("no EFI System Partition".into()));
    }

    let new_array_lba = n
        .checked_sub(1 + entry_sectors)
        .filter(|lba| *lba > m - 1)
        .ok_or_else(|| skip("no room for a new backup GPT past the image".into()))?;
    let new_last_usable = new_array_lba - 1;
    let new_primary = primary.rebuilt(1, n - 1, primary.entry_lba, new_last_usable);
    let new_backup = primary.rebuilt(n - 1, 1, new_array_lba, new_last_usable);

    let size_field = MBR_RECORDS + MBR_RECORD_SIZE;
    let new_size = u32::try_from(n - 1).unwrap_or(u32::MAX);
    let new_pmbr = (le_u32(&lba0, size_field) != new_size).then(|| {
        let mut s = lba0.clone();
        put_u32(&mut s, size_field, new_size);
        s
    });

    // Read from the medium, not trusted from the write: the device must still hold the image's GPT.
    let dev_lba1 = read_device(dev, 1, 1).map_err(Stop::Warn)?;
    if dev_lba1 == new_primary {
        return Err(skip("backup GPT is already at the device end".into()));
    }
    let checks = [
        (0, 1, &lba0),
        (1, 1, &lba1),
        (primary.entry_lba, entry_sectors, &array),
        (backup.entry_lba, entry_sectors, &backup_array),
        (m - 1, 1, &backup_sector),
    ];
    for (lba, sectors, expected) in checks {
        if read_device(dev, lba, sectors).map_err(Stop::Warn)? != *expected {
            return Err(Stop::Warn(format!(
                "device LBA {lba} no longer matches the image"
            )));
        }
    }

    let aliases = tail_aliases(m, n, new_array_lba, n - 1);
    if !aliases.is_empty() {
        let tail = read_device(dev, new_array_lba, entry_sectors + 1).map_err(Stop::Warn)?;
        for (target, alias) in &aliases {
            let img = read_image(image, *alias, 1).map_err(Stop::Warn)?;
            let at = ((target - new_array_lba) * SECTOR) as usize;
            if !is_constant(&img) && tail[at..at + SECTOR_LEN] == img[..] {
                return Err(Stop::FakeCapacity(format!(
                    "device LBA {target} mirrors image LBA {alias}: the device holds less than its reported {n} sectors; the backup GPT was left in place"
                )));
            }
        }
    }
    let mut alias_sectors: Vec<u64> = aliases.into_iter().map(|(_, a)| a).collect();
    alias_sectors.sort_unstable();
    alias_sectors.dedup();

    Ok(Plan {
        image_sectors: m,
        device_sectors: n,
        old_last_usable: primary.last_usable,
        new_array_lba,
        new_last_usable,
        array,
        array_bytes,
        new_primary,
        new_backup,
        new_pmbr,
        alias_sectors,
    })
}

fn write_backup<D: SectorDevice>(dev: &mut D, plan: &Plan) -> Result<(), String> {
    let n = plan.device_sectors;
    write_device(dev, n, plan.new_array_lba, &plan.array)?;
    write_device(dev, n, n - 1, &plan.new_backup)?;
    flush_device(dev, "the backup GPT")?;

    let sectors = (plan.array.len() / SECTOR_LEN) as u64 + 1;
    let tail = read_device(dev, plan.new_array_lba, sectors)?;
    let (array, header) = tail.split_at(plan.array.len());
    let valid = parse_header(header)
        .is_ok_and(|h| crc32fast::hash(&array[..plan.array_bytes]) == h.entries_crc);
    if array != plan.array || header != plan.new_backup || !valid {
        return Err("the backup GPT read back differs from what was written".into());
    }
    Ok(())
}

/// Re-read every image sector the tail could alias to; a change means the card wrapped the backup write.
fn recheck_aliases<D: SectorDevice, I: Read + Seek>(
    dev: &mut D,
    image: &mut I,
    plan: &Plan,
) -> Result<Option<Outcome>, String> {
    let mut changed = Vec::new();
    for lba in &plan.alias_sectors {
        let expected = match read_image(image, *lba, 1) {
            Ok(s) => s,
            Err(e) => return Ok(Some(warning(TAG_GPT_RELOCATE_SKIPPED, e))),
        };
        match read_device(dev, *lba, 1) {
            Ok(actual) if actual == expected => {}
            Ok(_) => changed.push((*lba, expected)),
            Err(e) => return Ok(Some(warning(TAG_GPT_RELOCATE_SKIPPED, e))),
        }
    }
    if changed.is_empty() {
        return Ok(None);
    }

    log_warn!(
        MODULE,
        "The backup GPT write changed {} image sector(s) (first LBA {}): the device wraps, restoring them",
        changed.len(),
        changed[0].0
    );
    let n = plan.device_sectors;
    for (lba, data) in &changed {
        write_device(dev, n, *lba, data).map_err(failed)?;
    }
    flush_device(dev, "the restored image sectors").map_err(failed)?;
    for (lba, data) in &changed {
        if read_device(dev, *lba, 1).map_err(failed)? != *data {
            return Err(failed(format!(
                "image LBA {lba} could not be restored on a device that wraps"
            )));
        }
    }
    Ok(Some(warning(
        TAG_GPT_FAKE_CAPACITY,
        format!(
            "the device holds less than its reported {} sectors; the backup GPT was left in place",
            n
        ),
    )))
}

fn write_and_check<D: SectorDevice>(
    dev: &mut D,
    plan: &Plan,
    lba: u64,
    data: &[u8],
    what: &str,
) -> Result<(), String> {
    write_device(dev, plan.device_sectors, lba, data)?;
    flush_device(dev, what)?;
    if read_device(dev, lba, (data.len() / SECTOR_LEN) as u64)? != data {
        return Err(format!("{what} read back differs from what was written"));
    }
    Ok(())
}

fn write_primary<D: SectorDevice>(dev: &mut D, plan: &Plan) -> Result<(), String> {
    write_and_check(dev, plan, 1, &plan.new_primary, "the primary GPT header")?;
    if let Some(pmbr) = &plan.new_pmbr {
        write_and_check(dev, plan, 0, pmbr, "the protective MBR")?;
    }
    // prepare() checked every partition ends at or before the old LastUsableLBA, below this header.
    let zeros = vec![0u8; SECTOR_LEN];
    write_and_check(
        dev,
        plan,
        plan.image_sectors - 1,
        &zeros,
        "the old backup GPT header",
    )
}

fn failed(e: impl std::fmt::Display) -> String {
    let e = tagged(TAG_GPT_RELOCATE_FAILED, e);
    log_error!(MODULE, "Backup GPT relocation failed: {}", e);
    e
}

fn warning(tag: &str, e: impl std::fmt::Display) -> Outcome {
    let e = tagged(tag, e);
    log_error!(MODULE, "Backup GPT not relocated: {}", e);
    Outcome::Warning(e)
}

/// Move the image's backup GPT to the device end; `Err` only once the primary header may have changed.
pub(crate) fn relocate_backup_gpt<D: SectorDevice, I: Read + Seek>(
    dev: &mut D,
    image: &mut I,
    image_size: u64,
    geometry: DeviceGeometry,
) -> Result<Outcome, String> {
    let plan = match prepare(dev, image, image_size, geometry) {
        Ok(plan) => plan,
        Err(Stop::Skip(reason)) => {
            log_info!(MODULE, "Backup GPT left in place: {}", reason);
            return Ok(Outcome::Skipped(reason));
        }
        Err(Stop::FakeCapacity(e)) => return Ok(warning(TAG_GPT_FAKE_CAPACITY, e)),
        Err(Stop::Warn(e)) => return Ok(warning(TAG_GPT_RELOCATE_SKIPPED, e)),
    };

    log_info!(
        MODULE,
        "Relocating backup GPT: image {} sectors, device {} sectors, AlternateLBA {} -> {}, LastUsableLBA {} -> {}",
        plan.image_sectors,
        plan.device_sectors,
        plan.image_sectors - 1,
        plan.device_sectors - 1,
        plan.old_last_usable,
        plan.new_last_usable
    );

    if let Err(e) = write_backup(dev, &plan) {
        return Ok(warning(TAG_GPT_RELOCATE_SKIPPED, e));
    }
    if let Some(outcome) = recheck_aliases(dev, image, &plan)? {
        return Ok(outcome);
    }
    write_primary(dev, &plan).map_err(failed)?;

    log_info!(
        MODULE,
        "Backup GPT relocated to LBA {}",
        plan.device_sectors - 1
    );
    Ok(Outcome::Relocated)
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use std::collections::{BTreeMap, HashSet};

    pub(crate) const LINUX_FS_GUID: [u8; GUID_LEN] = [
        0xAF, 0x3D, 0xC6, 0x0F, 0x83, 0x84, 0x72, 0x47, 0x8E, 0x79, 0x3D, 0x69, 0xD8, 0x47, 0x7D,
        0xE4,
    ];
    pub(crate) const ESP_GUID: [u8; GUID_LEN] = ESP_TYPE_GUID;
    pub(crate) const BOOT_CODE: u8 = 0xEB;

    pub(crate) struct Layout {
        pub sectors: u64,
        pub entry_lba: u64,
        pub entry_count: u32,
        pub entry_size: u32,
        pub first_usable: u64,
        pub partitions: Vec<([u8; GUID_LEN], u64, u64)>,
    }

    impl Layout {
        /// ESP with patterned data and a zero-filled root, like an sfdisk-built Armbian image.
        pub(crate) fn uefi(sectors: u64) -> Self {
            let last_usable = sectors - 2 - 32;
            Layout {
                sectors,
                entry_lba: 2,
                entry_count: 128,
                entry_size: 128,
                first_usable: 2048,
                partitions: vec![(ESP_GUID, 2048, 6143), (LINUX_FS_GUID, 6144, last_usable)],
            }
        }

        pub(crate) fn entry_sectors(&self) -> u64 {
            (u64::from(self.entry_count) * u64::from(self.entry_size)).div_ceil(SECTOR)
        }

        pub(crate) fn last_usable(&self) -> u64 {
            self.sectors - 2 - self.entry_sectors()
        }
    }

    pub(crate) fn header(layout: &Layout, my: u64, alt: u64, entry_lba: u64, crc: u32) -> Vec<u8> {
        let mut h = vec![0u8; SECTOR_LEN];
        h[..8].copy_from_slice(GPT_SIGNATURE);
        h[8..12].copy_from_slice(&[0, 0, 1, 0]);
        put_u32(&mut h, HDR_SIZE, HDR_MIN_SIZE as u32);
        put_u64(&mut h, HDR_MY_LBA, my);
        put_u64(&mut h, HDR_ALT_LBA, alt);
        put_u64(&mut h, HDR_FIRST_USABLE, layout.first_usable);
        put_u64(&mut h, HDR_LAST_USABLE, layout.last_usable());
        h[HDR_DISK_GUID..HDR_DISK_GUID + GUID_LEN].copy_from_slice(&[0x5A; GUID_LEN]);
        put_u64(&mut h, HDR_ENTRY_LBA, entry_lba);
        put_u32(&mut h, HDR_ENTRY_COUNT, layout.entry_count);
        put_u32(&mut h, HDR_ENTRY_SIZE, layout.entry_size);
        put_u32(&mut h, HDR_ENTRIES_CRC, crc);
        reseal(&mut h);
        h
    }

    /// Recompute the header CRC after a test edit.
    pub(crate) fn reseal(h: &mut [u8]) {
        let size = (le_u32(h, HDR_SIZE) as usize).clamp(HDR_MIN_SIZE, SECTOR_LEN);
        let crc = header_crc(&h[..size]);
        put_u32(h, HDR_CRC, crc);
    }

    pub(crate) fn build_image(layout: &Layout) -> Vec<u8> {
        let m = layout.sectors;
        let mut img = vec![0u8; (m * SECTOR) as usize];
        if let Some((_, first, last)) = layout.partitions.iter().find(|p| p.0 == ESP_GUID) {
            for lba in *first..=*last {
                let at = (lba * SECTOR) as usize;
                for (i, b) in img[at..at + SECTOR_LEN].iter_mut().enumerate() {
                    *b = ((lba as usize * 31 + i) % 251) as u8 + 1;
                }
            }
        }

        img[..440].fill(BOOT_CODE);
        let rec = MBR_RECORDS;
        img[rec + 1..rec + 4].copy_from_slice(&[0x00, 0x02, 0x00]);
        img[rec + MBR_RECORD_TYPE] = MBR_PROTECTIVE_TYPE;
        img[rec + 5..rec + 8].copy_from_slice(&[0xFF, 0xFF, 0xFF]);
        put_u32(&mut img, rec + MBR_RECORD_START, 1);
        put_u32(
            &mut img,
            rec + MBR_RECORD_SIZE,
            u32::try_from(m - 1).unwrap_or(u32::MAX),
        );
        img[MBR_SIGNATURE..MBR_SIGNATURE + 2].copy_from_slice(&MBR_BOOT_SIGNATURE);

        let e = layout.entry_sectors();
        let array_bytes = (u64::from(layout.entry_count) * u64::from(layout.entry_size)) as usize;
        let mut array = vec![0u8; (e * SECTOR) as usize];
        for (i, (guid, first, last)) in layout.partitions.iter().enumerate() {
            let at = i * layout.entry_size as usize;
            array[at..at + GUID_LEN].copy_from_slice(guid);
            array[at + GUID_LEN..at + 2 * GUID_LEN].copy_from_slice(&[i as u8 + 0x10; GUID_LEN]);
            put_u64(&mut array, at + ENTRY_FIRST_LBA, *first);
            put_u64(&mut array, at + ENTRY_LAST_LBA, *last);
            array[at + 56] = b'p';
        }
        let crc = crc32fast::hash(&array[..array_bytes]);

        let put = |img: &mut Vec<u8>, lba: u64, data: &[u8]| {
            let at = (lba * SECTOR) as usize;
            img[at..at + data.len()].copy_from_slice(data);
        };
        put(
            &mut img,
            1,
            &header(layout, 1, m - 1, layout.entry_lba, crc),
        );
        put(&mut img, layout.entry_lba, &array);
        put(&mut img, m - 1 - e, &array);
        put(&mut img, m - 1, &header(layout, m - 1, 1, m - 1 - e, crc));
        img
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) enum Event {
        Write(u64, u64),
        Flush,
    }

    /// Sparse in-memory disk; `wrap` makes it alias every sector modulo a smaller real capacity.
    pub(crate) struct MemDisk {
        pub sectors: BTreeMap<u64, Vec<u8>>,
        pub size: u64,
        pub wrap: Option<u64>,
        pub events: Vec<Event>,
        pub fail_write: Option<usize>,
        pub fail_flush: Option<usize>,
        pub fail_read_lba: Option<u64>,
        pub corrupt_reads: HashSet<u64>,
        /// Corrupt reads only once this many write calls were made.
        pub corrupt_after: usize,
        writes: usize,
        flushes: usize,
    }

    impl MemDisk {
        pub(crate) fn with_image(image: &[u8], size: u64) -> Self {
            let mut disk = MemDisk {
                sectors: BTreeMap::new(),
                size,
                wrap: None,
                events: Vec::new(),
                fail_write: None,
                fail_flush: None,
                fail_read_lba: None,
                corrupt_reads: HashSet::new(),
                corrupt_after: 0,
                writes: 0,
                flushes: 0,
            };
            for (lba, chunk) in image.chunks(SECTOR_LEN).enumerate() {
                if chunk.iter().any(|b| *b != 0) {
                    let mut s = chunk.to_vec();
                    s.resize(SECTOR_LEN, 0);
                    disk.sectors.insert(lba as u64, s);
                }
            }
            disk
        }

        fn physical(&self, lba: u64) -> u64 {
            self.wrap.map_or(lba, |w| lba % w)
        }

        pub(crate) fn sector(&self, lba: u64) -> Vec<u8> {
            self.sectors
                .get(&self.physical(lba))
                .cloned()
                .unwrap_or_else(|| vec![0u8; SECTOR_LEN])
        }

        pub(crate) fn to_vec(&self) -> Vec<u8> {
            let mut out = vec![0u8; self.size as usize];
            for lba in 0..self.size / SECTOR {
                let at = (lba * SECTOR) as usize;
                out[at..at + SECTOR_LEN].copy_from_slice(&self.sector(lba));
            }
            out
        }

        fn check(&self, offset: u64, len: usize) -> io::Result<()> {
            if !offset.is_multiple_of(SECTOR)
                || !len.is_multiple_of(SECTOR_LEN)
                || offset + len as u64 > self.size
            {
                return Err(io::Error::other(format!("bad I/O {offset}+{len}")));
            }
            Ok(())
        }
    }

    impl SectorDevice for MemDisk {
        fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
            self.check(offset, buf.len())?;
            for (i, chunk) in buf.chunks_mut(SECTOR_LEN).enumerate() {
                let lba = offset / SECTOR + i as u64;
                if self.fail_read_lba == Some(lba) {
                    return Err(io::Error::other("injected read failure"));
                }
                chunk.copy_from_slice(&self.sector(lba));
                if self.writes >= self.corrupt_after && self.corrupt_reads.contains(&lba) {
                    chunk[0] ^= 0xFF;
                }
            }
            Ok(())
        }

        fn write_at(&mut self, offset: u64, buf: &[u8]) -> io::Result<()> {
            self.check(offset, buf.len())?;
            let call = self.writes;
            self.writes += 1;
            if self.fail_write == Some(call) {
                return Err(io::Error::other("injected write failure"));
            }
            self.events.push(Event::Write(
                offset / SECTOR,
                (buf.len() / SECTOR_LEN) as u64,
            ));
            for (i, chunk) in buf.chunks(SECTOR_LEN).enumerate() {
                let lba = self.physical(offset / SECTOR + i as u64);
                self.sectors.insert(lba, chunk.to_vec());
            }
            Ok(())
        }

        fn flush(&mut self) -> io::Result<()> {
            let call = self.flushes;
            self.flushes += 1;
            if self.fail_flush == Some(call) {
                return Err(io::Error::other("injected flush failure"));
            }
            self.events.push(Event::Flush);
            Ok(())
        }
    }

    /// Any Read + Write + Seek as a device, for file-backed tests.
    pub(crate) struct SeekDevice<T>(pub T);

    impl<T: Read + io::Write + Seek> SectorDevice for SeekDevice<T> {
        fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
            self.0.seek(SeekFrom::Start(offset))?;
            self.0.read_exact(buf)
        }

        fn write_at(&mut self, offset: u64, buf: &[u8]) -> io::Result<()> {
            self.0.seek(SeekFrom::Start(offset))?;
            self.0.write_all(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use std::io::Cursor;

    const M: u64 = 16384;
    const N: u64 = 40000;

    fn geometry(sectors: u64) -> DeviceGeometry {
        DeviceGeometry {
            size: sectors * SECTOR,
            sector_size: SECTOR,
        }
    }

    fn run(image: &[u8], disk: &mut MemDisk) -> Result<Outcome, String> {
        let size = disk.size;
        relocate_backup_gpt(
            disk,
            &mut Cursor::new(image.to_vec()),
            image.len() as u64,
            DeviceGeometry {
                size,
                sector_size: SECTOR,
            },
        )
    }

    fn flashed(layout: &Layout, device_sectors: u64) -> (Vec<u8>, MemDisk) {
        let image = build_image(layout);
        let disk = MemDisk::with_image(&image, device_sectors * SECTOR);
        (image, disk)
    }

    fn oracle(disk: &MemDisk) -> gptman::GPT {
        gptman::GPT::read_from(&mut Cursor::new(disk.to_vec()), SECTOR).expect("valid GPT")
    }

    fn entries(gpt: &gptman::GPT) -> Vec<gptman::GPTPartitionEntry> {
        gpt.iter().map(|(_, p)| p.clone()).collect()
    }

    fn backup_header(disk: &MemDisk, lba: u64) -> gptman::GPTHeader {
        let mut c = Cursor::new(disk.to_vec());
        c.seek(SeekFrom::Start(lba * SECTOR)).unwrap();
        gptman::GPTHeader::read_from(&mut c).expect("valid backup header")
    }

    fn assert_skipped_untouched(image: &[u8], mut disk: MemDisk, reason: &str) {
        let before = disk.sectors.clone();
        let outcome = run(image, &mut disk).unwrap();
        match outcome {
            Outcome::Skipped(r) => assert!(r.contains(reason), "{r:?} lacks {reason:?}"),
            other => panic!("expected a skip, got {other:?}"),
        }
        assert!(disk.events.is_empty(), "{:?}", disk.events);
        assert!(disk.sectors == before, "disk changed");
    }

    #[test]
    fn relocates_a_uefi_image_to_the_device_end() {
        let layout = Layout::uefi(M);
        let (image, mut disk) = flashed(&layout, N);

        assert_eq!(run(&image, &mut disk).unwrap(), Outcome::Relocated);

        let gpt = oracle(&disk);
        assert_eq!(gpt.header.primary_lba, 1);
        assert_eq!(gpt.header.backup_lba, N - 1);
        assert_eq!(gpt.header.last_usable_lba, N - 2 - 32);
        assert_eq!(gpt.header.first_usable_lba, 2048);
        assert_eq!(gpt.header.partition_entry_lba, 2);
        assert_eq!(gpt[1].starting_lba, 2048);
        assert_eq!(gpt[2].ending_lba, layout.last_usable());

        let backup = backup_header(&disk, N - 1);
        assert_eq!(backup.primary_lba, N - 1);
        assert_eq!(backup.backup_lba, 1);
        assert_eq!(backup.partition_entry_lba, N - 1 - 32);
        let mut c = Cursor::new(disk.to_vec());
        assert_eq!(
            backup.read_partitions(&mut c, SECTOR).unwrap(),
            entries(&gpt)
        );

        let lba0 = disk.sector(0);
        assert_eq!(le_u32(&lba0, MBR_RECORDS + MBR_RECORD_SIZE), (N - 1) as u32);
        assert_eq!(&lba0[..446], &image[..446]);
        assert_eq!(&lba0[446..458], &image[446..458]);
        assert_eq!(&lba0[462..], &image[462..512]);
        assert!(disk.sector(M - 1).iter().all(|b| *b == 0));
        for lba in 2..M - 1 {
            let at = (lba * SECTOR) as usize;
            assert_eq!(disk.sector(lba), &image[at..at + SECTOR_LEN], "LBA {lba}");
        }
    }

    #[test]
    fn writes_backup_first_then_primary_then_pmbr_each_flushed() {
        let (image, mut disk) = flashed(&Layout::uefi(M), N);
        run(&image, &mut disk).unwrap();
        use Event::*;
        assert_eq!(
            disk.events,
            vec![
                Write(N - 33, 32),
                Write(N - 1, 1),
                Flush,
                Write(1, 1),
                Flush,
                Write(0, 1),
                Flush,
                Write(M - 1, 1),
                Flush,
            ]
        );
    }

    #[test]
    fn keeps_a_non_default_entry_lba_and_a_short_entry_array() {
        let mut layout = Layout::uefi(M);
        layout.entry_lba = 8160;
        layout.entry_count = 32;
        layout.first_usable = 8192;
        let last = layout.last_usable();
        layout.partitions = vec![(ESP_GUID, 8192, 10239), (LINUX_FS_GUID, 10240, last)];
        let (image, mut disk) = flashed(&layout, N);

        assert_eq!(run(&image, &mut disk).unwrap(), Outcome::Relocated);

        let gpt = oracle(&disk);
        assert_eq!(gpt.header.partition_entry_lba, 8160);
        assert_eq!(gpt.header.number_of_partition_entries, 32);
        assert_eq!(gpt.header.backup_lba, N - 1);
        assert_eq!(gpt.header.last_usable_lba, N - 2 - 8);
        let backup = backup_header(&disk, N - 1);
        assert_eq!(backup.partition_entry_lba, N - 1 - 8);
        assert!(disk
            .events
            .iter()
            .all(|e| !matches!(e, Event::Write(lba, _) if (2..M - 1).contains(lba))));
    }

    #[test]
    fn accepts_a_gpt_built_by_gptman() {
        let mut cur = Cursor::new(vec![0u8; (M * SECTOR) as usize]);
        let mut gpt = gptman::GPT::new_from(&mut cur, SECTOR, [0x11; 16]).unwrap();
        gpt[1] = gptman::GPTPartitionEntry {
            partition_type_guid: ESP_GUID,
            unique_partition_guid: [0x22; 16],
            starting_lba: 2048,
            ending_lba: 4095,
            attribute_bits: 0,
            partition_name: "efi".into(),
        };
        gpt.write_into(&mut cur).unwrap();
        gptman::GPT::write_protective_mbr_into(&mut cur, SECTOR).unwrap();
        let image = cur.into_inner();
        let mut disk = MemDisk::with_image(&image, N * SECTOR);

        assert_eq!(run(&image, &mut disk).unwrap(), Outcome::Relocated);
        let after = oracle(&disk);
        assert_eq!(after.header.backup_lba, N - 1);
        assert_eq!(after[1].ending_lba, 4095);
        backup_header(&disk, N - 1);
    }

    #[test]
    fn running_twice_leaves_the_second_run_a_no_op() {
        let (image, mut disk) = flashed(&Layout::uefi(M), N);
        assert_eq!(run(&image, &mut disk).unwrap(), Outcome::Relocated);
        disk.events.clear();
        assert_skipped_untouched(&image, disk, "already at the device end");
    }

    #[test]
    fn skips_without_an_efi_system_partition() {
        let mut layout = Layout::uefi(M);
        layout.partitions[0].0 = LINUX_FS_GUID;
        let (image, disk) = flashed(&layout, N);
        assert_skipped_untouched(&image, disk, "no EFI System Partition");
    }

    #[test]
    fn skips_a_non_512_sector_device() {
        let (image, mut disk) = flashed(&Layout::uefi(M), N);
        let outcome = relocate_backup_gpt(
            &mut disk,
            &mut Cursor::new(image.clone()),
            image.len() as u64,
            DeviceGeometry {
                size: N * SECTOR,
                sector_size: 4096,
            },
        )
        .unwrap();
        assert!(matches!(outcome, Outcome::Skipped(r) if r.contains("sector size")));
        assert!(disk.events.is_empty());
    }

    #[test]
    fn skips_partial_sector_sizes() {
        let (mut image, disk) = flashed(&Layout::uefi(M), N);
        image.extend_from_slice(&[1u8; 100]);
        assert_skipped_untouched(&image, disk, "not whole sectors");

        let (image, mut disk) = flashed(&Layout::uefi(M), N);
        let outcome = relocate_backup_gpt(
            &mut disk,
            &mut Cursor::new(image.clone()),
            image.len() as u64,
            DeviceGeometry {
                size: N * SECTOR + 7,
                sector_size: SECTOR,
            },
        )
        .unwrap();
        assert!(matches!(outcome, Outcome::Skipped(r) if r.contains("not whole sectors")));
        assert!(disk.events.is_empty());
    }

    #[test]
    fn size_boundaries() {
        let layout = Layout::uefi(M);
        let (image, disk) = flashed(&layout, M);
        assert_skipped_untouched(&image, disk, "fills the device");

        // The new array must start past the image's last LBA.
        let (image, disk) = flashed(&layout, M + 32);
        assert_skipped_untouched(&image, disk, "no room");

        let (image, mut disk) = flashed(&layout, M + 33);
        assert_eq!(run(&image, &mut disk).unwrap(), Outcome::Relocated);
        let gpt = oracle(&disk);
        assert_eq!(gpt.header.backup_lba, M + 32);
        assert_eq!(gpt.header.last_usable_lba, M - 1);
        assert_eq!(gpt.header.partition_entry_lba, 2);
        backup_header(&disk, M + 32);
    }

    #[test]
    fn clamps_the_pmbr_size_on_a_2tib_device() {
        for (n, expected) in [
            ((1u64 << 32) + 1000, u32::MAX),
            (1u64 << 32, u32::MAX),
            ((1u64 << 32) - 1, u32::MAX - 1),
        ] {
            let (image, mut disk) = flashed(&Layout::uefi(M), n);
            assert_eq!(run(&image, &mut disk).unwrap(), Outcome::Relocated);
            let lba0 = disk.sector(0);
            assert_eq!(
                le_u32(&lba0, MBR_RECORDS + MBR_RECORD_SIZE),
                expected,
                "{n}"
            );
            let primary = parse_header(&disk.sector(1)).unwrap();
            assert_eq!(primary.alt_lba, n - 1);
            let backup = parse_header(&disk.sector(n - 1)).unwrap();
            assert_eq!(backup.my_lba, n - 1);
            assert_eq!(backup.entry_lba, n - 33);
        }
    }

    #[test]
    fn leaves_an_already_clamped_pmbr_alone() {
        let layout = Layout::uefi(M);
        let mut image = build_image(&layout);
        put_u32(&mut image, MBR_RECORDS + MBR_RECORD_SIZE, u32::MAX);
        let n = (1u64 << 32) + 5;
        let mut disk = MemDisk::with_image(&image, n * SECTOR);
        assert_eq!(run(&image, &mut disk).unwrap(), Outcome::Relocated);
        assert!(!disk.events.contains(&Event::Write(0, 1)));
    }

    type SectorEdit = Box<dyn Fn(&mut [u8])>;
    type ImageEdit = Box<dyn Fn(&mut Vec<u8>)>;

    fn edited(edit: impl Fn(&mut Vec<u8>)) -> (Vec<u8>, MemDisk) {
        let mut image = build_image(&Layout::uefi(M));
        edit(&mut image);
        let disk = MemDisk::with_image(&image, N * SECTOR);
        (image, disk)
    }

    fn edit_sector(image: &mut [u8], lba: u64, edit: impl Fn(&mut [u8])) {
        let at = (lba * SECTOR) as usize;
        edit(&mut image[at..at + SECTOR_LEN]);
    }

    #[test]
    fn skips_anything_but_a_pure_protective_mbr() {
        let cases: Vec<(&str, SectorEdit)> = vec![
            ("no signature", Box::new(|s| s[511] = 0)),
            (
                "not 0xEE",
                Box::new(|s| s[MBR_RECORDS + MBR_RECORD_TYPE] = 0x83),
            ),
            (
                "hybrid",
                Box::new(|s| s[MBR_RECORDS + MBR_RECORD_LEN + MBR_RECORD_TYPE] = 0x0C),
            ),
            (
                "start 2",
                Box::new(|s| put_u32(s, MBR_RECORDS + MBR_RECORD_START, 2)),
            ),
        ];
        for (name, edit) in cases {
            let (image, disk) = edited(|img| edit_sector(img, 0, &edit));
            eprintln!("case {name}");
            assert_skipped_untouched(&image, disk, "protective MBR");
        }
    }

    #[test]
    fn skips_an_iso9660_hybrid() {
        let (image, disk) = edited(|img| {
            let at = ISO9660_ID_OFFSET as usize;
            img[at..at + 5].copy_from_slice(ISO9660_ID);
        });
        assert_skipped_untouched(&image, disk, "ISO9660");
    }

    #[test]
    fn skips_a_damaged_or_inconsistent_gpt() {
        let resealed = |lba: u64, at: usize, v: u64| {
            move |img: &mut Vec<u8>| {
                edit_sector(img, lba, |s| {
                    put_u64(s, at, v);
                    reseal(s);
                })
            }
        };
        let cases: Vec<(&str, ImageEdit)> = vec![
            (
                "primary GPT: header CRC",
                Box::new(|img| edit_sector(img, 1, |s| s[HDR_DISK_GUID] ^= 1)),
            ),
            ("AlternateLBA", Box::new(resealed(1, HDR_ALT_LBA, M - 2))),
            ("AlternateLBA", Box::new(resealed(1, HDR_MY_LBA, 2))),
            (
                "backup GPT: no EFI PART",
                Box::new(|img| edit_sector(img, M - 1, |s| s.fill(0))),
            ),
            ("does not mirror", Box::new(resealed(M - 1, HDR_ALT_LBA, 2))),
            (
                "does not mirror",
                Box::new(resealed(M - 1, HDR_LAST_USABLE, M - 40)),
            ),
            (
                "entry array CRC",
                Box::new(|img| edit_sector(img, 2, |s| s[100] ^= 1)),
            ),
            (
                "backup partition entry array differs",
                Box::new(|img| edit_sector(img, M - 33, |s| s[100] ^= 1)),
            ),
            (
                "layout is not sane",
                Box::new(resealed(1, HDR_FIRST_USABLE, 20)),
            ),
        ];
        for (reason, edit) in cases {
            let (image, disk) = edited(&edit);
            eprintln!("case {reason}");
            assert_skipped_untouched(&image, disk, reason);
        }
    }

    #[test]
    fn skips_a_partition_past_last_usable() {
        let mut layout = Layout::uefi(M);
        layout.partitions[1].2 = M - 2;
        let (image, disk) = flashed(&layout, N);
        assert_skipped_untouched(&image, disk, "outside the usable area");
    }

    #[test]
    fn crafted_headers_never_panic() {
        let fields: [(usize, u64); 12] = [
            (HDR_SIZE, 0),
            (HDR_SIZE, 91),
            (HDR_SIZE, 513),
            (HDR_SIZE, u64::from(u32::MAX)),
            (HDR_ENTRY_COUNT, u64::from(u32::MAX)),
            (HDR_ENTRY_COUNT, 0),
            (HDR_ENTRY_SIZE, 0),
            (HDR_ENTRY_SIZE, 64),
            (HDR_ENTRY_SIZE, 1 << 31),
            (HDR_ENTRY_LBA, u64::MAX),
            (HDR_FIRST_USABLE, u64::MAX),
            (HDR_LAST_USABLE, u64::MAX),
        ];
        for (at, v) in fields {
            for lba in [1, M - 1] {
                let (image, disk) = edited(|img| {
                    edit_sector(img, lba, |s| {
                        if at == HDR_SIZE || at == HDR_ENTRY_COUNT || at == HDR_ENTRY_SIZE {
                            put_u32(s, at, v as u32);
                        } else {
                            put_u64(s, at, v);
                        }
                        reseal(s);
                    })
                });
                assert_skipped_untouched(&image, disk, "");
            }
        }
        for (at, v) in [(HDR_ALT_LBA, u64::MAX), (HDR_MY_LBA, u64::MAX)] {
            let (image, disk) = edited(|img| {
                edit_sector(img, M - 1, |s| {
                    put_u64(s, at, v);
                    reseal(s);
                })
            });
            assert_skipped_untouched(&image, disk, "");
        }
        let (image, disk) = edited(|img| {
            edit_sector(img, M - 1, |s| {
                put_u64(s, HDR_ENTRY_LBA, u64::MAX - 1);
                reseal(s);
            })
        });
        assert_skipped_untouched(&image, disk, "");
    }

    #[test]
    fn a_device_that_no_longer_holds_the_image_warns_and_writes_nothing() {
        for lba in [0, 1, 2, M - 33, M - 1] {
            let (image, mut disk) = flashed(&Layout::uefi(M), N);
            disk.sectors.insert(lba, vec![0x42; SECTOR_LEN]);
            let before = disk.sectors.clone();
            let outcome = run(&image, &mut disk).unwrap();
            assert_eq!(
                outcome,
                Outcome::Warning(format!(
                    "{TAG_GPT_RELOCATE_SKIPPED} device LBA {lba} no longer matches the image"
                )),
                "LBA {lba}"
            );
            assert!(disk.events.is_empty(), "LBA {lba}: {:?}", disk.events);
            assert!(disk.sectors == before, "LBA {lba}: disk changed");
        }
    }

    #[test]
    fn a_read_error_before_any_write_is_a_warning() {
        let (image, mut disk) = flashed(&Layout::uefi(M), N);
        disk.fail_read_lba = Some(1);
        let outcome = run(&image, &mut disk).unwrap();
        assert!(matches!(&outcome, Outcome::Warning(w) if w.starts_with(TAG_GPT_RELOCATE_SKIPPED)));
        assert!(disk.events.is_empty());
    }

    fn primary_untouched(image: &[u8], disk: &MemDisk) {
        assert_eq!(disk.sector(0), &image[..SECTOR_LEN]);
        assert_eq!(disk.sector(1), &image[SECTOR_LEN..2 * SECTOR_LEN]);
        let gpt = oracle(disk);
        assert_eq!(gpt.header.backup_lba, M - 1);
        backup_header(disk, M - 1);
    }

    #[test]
    fn a_failure_before_the_primary_write_warns_and_keeps_the_image_gpt() {
        for write in [0, 1] {
            let (image, mut disk) = flashed(&Layout::uefi(M), N);
            disk.fail_write = Some(write);
            let outcome = run(&image, &mut disk).unwrap();
            assert!(
                matches!(&outcome, Outcome::Warning(w) if w.starts_with(TAG_GPT_RELOCATE_SKIPPED)),
                "write {write}: {outcome:?}"
            );
            primary_untouched(&image, &disk);
        }
        let (image, mut disk) = flashed(&Layout::uefi(M), N);
        disk.fail_flush = Some(0);
        assert!(matches!(
            run(&image, &mut disk).unwrap(),
            Outcome::Warning(_)
        ));
        primary_untouched(&image, &disk);

        for lba in [N - 33, N - 1] {
            let (image, mut disk) = flashed(&Layout::uefi(M), N);
            disk.corrupt_reads.insert(lba);
            disk.corrupt_after = 1;
            let outcome = run(&image, &mut disk).unwrap();
            assert!(matches!(&outcome, Outcome::Warning(w) if w.contains("read back")));
            primary_untouched(&image, &disk);
        }
    }

    #[test]
    fn a_failure_from_the_primary_write_on_is_a_tagged_error() {
        // Write calls: 0 array, 1 backup header, 2 primary, 3 PMBR, 4 old header.
        for write in [2, 3, 4] {
            let (image, mut disk) = flashed(&Layout::uefi(M), N);
            disk.fail_write = Some(write);
            let err = run(&image, &mut disk).unwrap_err();
            assert!(err.starts_with(TAG_GPT_RELOCATE_FAILED), "{err}");
            oracle(&disk);
        }
        for flush in [1, 2, 3] {
            let (image, mut disk) = flashed(&Layout::uefi(M), N);
            disk.fail_flush = Some(flush);
            let err = run(&image, &mut disk).unwrap_err();
            assert!(err.starts_with(TAG_GPT_RELOCATE_FAILED), "{err}");
            oracle(&disk);
        }
        for (lba, after) in [(1, 3), (0, 4), (M - 1, 5)] {
            let (image, mut disk) = flashed(&Layout::uefi(M), N);
            disk.corrupt_reads.insert(lba);
            disk.corrupt_after = after;
            let err = run(&image, &mut disk).unwrap_err();
            assert!(err.starts_with(TAG_GPT_RELOCATE_FAILED), "{err}");
            assert!(err.contains("read back"), "{err}");
            oracle(&disk);
        }
    }

    #[test]
    fn aliases_cover_every_power_of_two_wrap() {
        assert!(tail_aliases(3000, 4096, 4000, 4095).is_empty());
        let a = tail_aliases(3000, 7096, 7063, 7095);
        assert_eq!(a.first(), Some(&(7063, 2967)));
        assert_eq!(a.last(), Some(&(7095, 2999)));
        assert_eq!(a.len(), 33);
        // Only the 2^12 wrap lands below 3000 here; 2^13 maps the tail to 4871..
        let a = tail_aliases(3000, 13096, 13063, 13095);
        assert_eq!(a.len(), 33);
        assert!(a.iter().all(|(t, alias)| *alias == t % 4096));
        // 2^12, 2^13 and 2^14 all alias this tail below 3000.
        let a = tail_aliases(3000, 16384 + 2033, 18384, 18416);
        assert_eq!(a.len(), 99);
        assert!(a.iter().all(|(_, alias)| (2000..=2032).contains(alias)));
        assert!(tail_aliases(u64::MAX, u64::MAX, 1, 2).is_empty());
    }

    #[test]
    fn a_card_that_mirrors_image_data_at_its_tail_warns_and_writes_nothing() {
        let layout = Layout::uefi(M);
        let image = build_image(&layout);
        // Real capacity 2^14 = M: the reported tail wraps onto the image's own backup GPT.
        let n = M + 16384;
        let mut disk = MemDisk::with_image(&image, n * SECTOR);
        disk.wrap = Some(16384);
        let before = disk.sectors.clone();

        let outcome = run(&image, &mut disk).unwrap();
        assert_eq!(
            outcome,
            Outcome::Warning(format!(
                "{TAG_GPT_FAKE_CAPACITY} device LBA {} mirrors image LBA {}: the device holds less than its reported {n} sectors; the backup GPT was left in place",
                n - 33,
                M - 33
            ))
        );
        assert!(disk.events.is_empty(), "{:?}", disk.events);
        assert!(disk.sectors == before, "disk changed");

        let state = crate::flash::FlashState::new();
        let mut disk = MemDisk::with_image(&image, n * SECTOR);
        disk.wrap = Some(16384);
        crate::flash::finalize_gpt(
            &mut disk,
            &mut Cursor::new(image.clone()),
            image.len() as u64,
            geometry(n),
            &state,
        )
        .unwrap();
        assert!(state
            .warning()
            .is_some_and(|w| w.starts_with(TAG_GPT_FAKE_CAPACITY)));
    }

    #[test]
    fn a_wrapping_card_over_constant_image_data_is_not_flagged_before_the_write() {
        // Tail aliases land on zeros: no evidence yet, so check (a) passes and check (b) decides.
        let image = build_image(&Layout::uefi(M));
        let n = 16384 + 10000 + 33;
        let mut disk = MemDisk::with_image(&image, n * SECTOR);
        disk.wrap = Some(16384);
        run(&image, &mut disk).unwrap();
        assert!(disk.events.contains(&Event::Write(n - 33, 32)));
    }

    #[test]
    fn a_card_that_wraps_onto_zeros_is_caught_and_restored() {
        let layout = Layout::uefi(M);
        let image = build_image(&layout);
        // Real capacity 2^14; the tail lands at LBA 10000.., inside the zero-filled root.
        let n = 16384 + 10000 + 33;
        let mut disk = MemDisk::with_image(&image, n * SECTOR);
        disk.wrap = Some(16384);

        let outcome = run(&image, &mut disk).unwrap();
        assert!(
            matches!(&outcome, Outcome::Warning(w) if w.starts_with(TAG_GPT_FAKE_CAPACITY)),
            "{outcome:?}"
        );
        assert!(
            !disk.events.contains(&Event::Write(1, 1)),
            "primary must stay"
        );
        for lba in 0..M {
            let at = (lba * SECTOR) as usize;
            assert_eq!(disk.sector(lba), &image[at..at + SECTOR_LEN], "LBA {lba}");
        }
    }

    #[test]
    fn a_wrapping_card_that_cannot_be_restored_is_an_error() {
        let image = build_image(&Layout::uefi(M));
        let n = 16384 + 10000 + 33;
        let mut disk = MemDisk::with_image(&image, n * SECTOR);
        disk.wrap = Some(16384);
        disk.fail_write = Some(2);
        let err = run(&image, &mut disk).unwrap_err();
        assert!(err.starts_with(TAG_GPT_RELOCATE_FAILED), "{err}");
        assert_eq!(disk.sector(1), &image[SECTOR_LEN..2 * SECTOR_LEN]);
    }

    #[test]
    fn a_real_card_with_aliases_below_the_image_relocates() {
        let image = build_image(&Layout::uefi(M));
        let n = 16384 + 10000 + 33;
        let mut disk = MemDisk::with_image(&image, n * SECTOR);
        assert_eq!(run(&image, &mut disk).unwrap(), Outcome::Relocated);
        assert_eq!(oracle(&disk).header.backup_lba, n - 1);
    }

    fn real_image() -> Option<std::path::PathBuf> {
        std::env::var_os("ARMBIAN_GPT_TEST_IMAGE").map(Into::into)
    }

    /// ARMBIAN_GPT_TEST_IMAGE=<image> cargo test real_armbian_image -- --ignored; the image is opened read-only,
    /// ARMBIAN_GPT_TEST_OUT=<path> keeps the relocated device copy.
    #[test]
    #[ignore]
    fn relocates_a_real_armbian_image() {
        let Some(path) = real_image() else {
            panic!("set ARMBIAN_GPT_TEST_IMAGE");
        };
        let mut image = std::fs::File::open(&path).unwrap();
        let image_size = image.metadata().unwrap().len();
        let before = gptman::GPT::read_from(&mut image, SECTOR).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let dev_path = std::env::var_os("ARMBIAN_GPT_TEST_OUT")
            .map_or_else(|| dir.path().join("device.img"), Into::into);
        std::fs::copy(&path, &dev_path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dev_path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let extra = 2 * 1024 * 1024 * 1024u64;
        let dev_size = image_size + extra;
        let dev = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&dev_path)
            .unwrap();
        dev.set_len(dev_size).unwrap();
        let mut dev = SeekDevice(dev);

        let outcome = relocate_backup_gpt(
            &mut dev,
            &mut image,
            image_size,
            geometry(dev_size / SECTOR),
        )
        .unwrap();
        assert_eq!(outcome, Outcome::Relocated);

        let n = dev_size / SECTOR;
        let after = gptman::GPT::read_from(&mut dev.0, SECTOR).unwrap();
        assert_eq!(after.header.backup_lba, n - 1);
        assert_eq!(
            after.header.partition_entry_lba,
            before.header.partition_entry_lba
        );
        assert_eq!(
            after.header.first_usable_lba,
            before.header.first_usable_lba
        );
        assert_eq!(entries(&after), entries(&before));
        dev.0.seek(SeekFrom::Start((n - 1) * SECTOR)).unwrap();
        let backup = gptman::GPTHeader::read_from(&mut dev.0).unwrap();
        assert_eq!(backup.backup_lba, 1);
        assert_eq!(
            backup.read_partitions(&mut dev.0, SECTOR).unwrap(),
            entries(&before)
        );
        eprintln!(
            "relocated: image {} sectors, device {n} sectors, entries at {}, last usable {} -> {}",
            image_size / SECTOR,
            after.header.partition_entry_lba,
            before.header.last_usable_lba,
            after.header.last_usable_lba
        );
        assert_eq!(
            relocate_backup_gpt(&mut dev, &mut image, image_size, geometry(n)).unwrap(),
            Outcome::Skipped("backup GPT is already at the device end".into())
        );
    }
}

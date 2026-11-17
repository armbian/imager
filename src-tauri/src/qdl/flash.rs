// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! QDL flash orchestration: connect to EDL device via USB, upload firehose programmer via Sahara, configure
//! Firehose and program partitions from rawprogram0.xml, apply patch0.xml patches, then reset the device.

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use indexmap::IndexMap;
use qdl::parsers::{firehose_parser_ack_nak, firehose_parser_configure_response};
use qdl::sahara::{sahara_run, SaharaMode};
use qdl::types::{
    FirehoseConfiguration, FirehoseResetMode, QdlBackend, QdlChan, QdlDevice, QdlReadWrite,
};
use qdl::{
    firehose_configure, firehose_patch, firehose_program_storage, firehose_read, firehose_reset,
    firehose_write_getack, setup_target_device,
};
use xmltree::{Element, XMLNode};

use super::extract::{resolve_flash_file, FIREHOSE_ELF, PATCH_XML, RAWPROGRAM_XML};
use super::outcome::WriteFailure;
use super::provision::ProvisionSource;
use super::{
    QdlStorage, QDL_CANCELLED_ERROR, STAGE_COMPLETE, STAGE_CONFIGURING, STAGE_CONNECTING,
    STAGE_FIREHOSE, STAGE_PARTITION_PREFIX, STAGE_PATCHING, STAGE_PROVISIONING, STAGE_RESETTING,
    STAGE_SAHARA, TAG_QDL_AUTOCONFIG_FAILED, UFS_PARTITION_LABEL,
};
use crate::flash::FlashState;
use crate::{log_error, log_info, log_warn};

/// Full QDL flash of `flash_dir` (inside the extraction `extract_root`) to the EDL device at `device_path`.
pub fn qdl_flash(
    flash_dir: &Path,
    extract_root: &Path,
    device_path: &str,
    autoconfig: Option<crate::autoconfig::AutoconfigConfig>,
    state: Arc<FlashState>,
) -> Result<(), String> {
    // Every file rawprogram0.xml names must stay inside the extraction, checked before USB is touched.
    let rawprogram_path = flash_dir.join(RAWPROGRAM_XML);
    for entry in program_elements(&parse_xml(&rawprogram_path)?) {
        program_file(extract_root, flash_dir, &entry.attributes)?;
    }

    state.qdl.is_active.store(true, Ordering::SeqCst);

    let elf_path = flash_dir.join(FIREHOSE_ELF);
    let device = connect_and_configure(device_path, &elf_path, QdlStorage::Emmc, &state)?;
    run_tar_session(device, flash_dir, extract_root, autoconfig.as_ref(), &state)
}

fn run_tar_session<T: QdlReadWrite + ?Sized>(
    mut device: QdlDevice<T>,
    flash_dir: &Path,
    extract_root: &Path,
    autoconfig: Option<&crate::autoconfig::AutoconfigConfig>,
    state: &Arc<FlashState>,
) -> Result<(), String> {
    // The rootfs is mutated in place before Firehose reads it; skipped when not injectable.
    if let Some(cfg) = autoconfig {
        check_cancelled(state)?;
        inject_autoconfig(flash_dir, extract_root, cfg)?;
    }

    check_cancelled(state)?;
    update_qdl_stage(state, STAGE_FIREHOSE);

    let rawprogram_path = flash_dir.join(RAWPROGRAM_XML);
    if let Err(failure) = program_from_xml(
        &mut device,
        &rawprogram_path,
        flash_dir,
        extract_root,
        state,
    ) {
        if !failure.keep_armed {
            device.reset_on_drop = false;
        }
        return Err(failure.message);
    }

    let patch_path = flash_dir.join(PATCH_XML);
    if patch_path.exists() {
        check_cancelled(state)?;
        update_qdl_stage(state, STAGE_PATCHING);
        log_info!("qdl::flash", "Applying patches from patch0.xml...");
        patch_from_xml(&mut device, &patch_path)?;
    }

    check_cancelled_after_write(state)?;
    reset_and_complete(&mut device, state);
    log_info!("qdl::flash", "QDL flash completed successfully");
    Ok(())
}

/// Flash a whole `.img` to UFS via one raw Firehose write to LUN 0 sector 0
/// (`edl-ng --memory ufs write-sector 0 <img>` equivalent).
pub fn qdl_flash_ufs(
    image_path: &Path,
    elf_path: &Path,
    device_path: &str,
    provision: ProvisionSource,
    state: Arc<FlashState>,
) -> Result<(), String> {
    // Refused before connecting, so an empty image can never reach the NAK-means-provision path.
    let img_len = crate::utils::ensure_regular_nonempty(image_path)?;

    state.qdl.is_active.store(true, Ordering::SeqCst);

    let device = connect_and_configure(device_path, elf_path, QdlStorage::Ufs, &state)?;
    run_ufs_session(device, image_path, img_len, provision, &state)
}

fn run_ufs_session<T: QdlReadWrite + ?Sized>(
    mut device: QdlDevice<T>,
    image_path: &Path,
    img_len: u64,
    provision: ProvisionSource,
    state: &Arc<FlashState>,
) -> Result<(), String> {
    check_cancelled(state)?;
    update_qdl_stage(state, STAGE_FIREHOSE);

    let sector_size = device.fh_cfg.storage_sector_size;
    let num_sectors = raw_num_sectors(img_len, sector_size);
    let total_bytes = num_sectors as u64 * sector_size as u64;
    state.qdl.partitions_total.store(1, Ordering::SeqCst);
    state.total_bytes.store(total_bytes, Ordering::SeqCst);
    update_qdl_stage(
        state,
        &format!("{STAGE_PARTITION_PREFIX}{UFS_PARTITION_LABEL}"),
    );

    log_info!(
        "qdl::flash",
        "Writing {} bytes ({} sectors) to UFS...",
        img_len,
        num_sectors
    );

    if let Err(failure) = write_ufs_image(&mut device, image_path, num_sectors, state) {
        if !failure.needs_provisioning {
            return Err(format!("Failed to write UFS image: {}", failure.message));
        }
        let e = failure.message;
        // A program NAK on UFS means the module has no LUN 0 (a brand-new, unprovisioned module):
        // provision it in-session, then retry the write once.
        match provision {
            ProvisionSource::Ready(prov) => {
                check_cancelled(state)?;
                update_qdl_stage(state, STAGE_PROVISIONING);
                provision_ufs(&mut device, &prov)?;
                check_cancelled(state)?;
                update_qdl_stage(state, STAGE_FIREHOSE);
                state.written_bytes.store(0, Ordering::SeqCst);
                if let Err(retry) = write_ufs_image(&mut device, image_path, num_sectors, state) {
                    device.reset_on_drop = false;
                    return Err(format!(
                        "Failed to write UFS image after provisioning: {}",
                        retry.message
                    ));
                }
            }
            ProvisionSource::Absent => {
                return Err(format!("Failed to write UFS image: {e} The UFS module is likely unprovisioned (no LUN 0); provision it before flashing."));
            }
            ProvisionSource::Unavailable(reason) => {
                return Err(format!("Failed to write UFS image: {e} The module looks unprovisioned and auto-provisioning could not run: {reason}"));
            }
        }
    }

    check_cancelled_after_write(state)?;
    state.qdl.partitions_written.store(1, Ordering::SeqCst);
    state.written_bytes.store(total_bytes, Ordering::SeqCst);

    reset_and_complete(&mut device, state);
    log_info!("qdl::flash", "QDL UFS flash completed successfully");
    Ok(())
}

fn reset_and_complete<T: QdlReadWrite + ?Sized>(device: &mut QdlDevice<T>, state: &FlashState) {
    update_qdl_stage(state, STAGE_RESETTING);
    log_info!("qdl::flash", "Resetting device...");
    device.reset_on_drop = false;
    firehose_reset(device, &FirehoseResetMode::Reset, 0)
        .map_err(|e| {
            log_warn!("qdl::flash", "Device reset failed (non-fatal): {}", e);
        })
        .ok();
    update_qdl_stage(state, STAGE_COMPLETE);
}

/// Sectors needed to hold `len` bytes, rounding the final partial sector up.
fn raw_num_sectors(len: u64, sector: usize) -> usize {
    len.div_ceil(sector as u64) as usize
}

// A broken stream disarms the reset-on-drop.
fn write_ufs_image<T: QdlReadWrite + ?Sized>(
    device: &mut QdlDevice<T>,
    image_path: &Path,
    num_sectors: usize,
    state: &Arc<FlashState>,
) -> Result<(), WriteFailure> {
    let file = fs::File::open(image_path)
        .map_err(|e| WriteFailure::before_write(format!("Failed to open image: {e}")))?;
    let progress_state = state.clone();
    let mut reader = ProgressReader::new(file, state.clone(), move |bytes_transferred| {
        progress_state
            .written_bytes
            .store(bytes_transferred, Ordering::SeqCst);
    });
    firehose_program_storage(
        device,
        &mut reader,
        UFS_PARTITION_LABEL,
        num_sectors,
        0,
        0,
        "0",
    )
    .map_err(|e| {
        let failure = WriteFailure::classify(e.as_ref());
        if !failure.keep_armed {
            device.reset_on_drop = false;
        }
        failure
    })
}

/// Provision a blank UFS module in the current session from the qcombin `<ufs>` descriptor.
/// Configures with SkipStorageInit (a blank module has no LUN to init), sends each `<ufs>`
/// command, then re-initialises storage so the new LUN 0 is writable.
fn provision_ufs<C: QdlChan>(device: &mut C, xml_path: &Path) -> Result<(), String> {
    let commands = super::provision::parse_ufs_commands(xml_path)?;
    log_info!(
        "qdl::flash",
        "Provisioning UFS ({} commands) from {}",
        commands.len(),
        xml_path.display()
    );

    firehose_configure(device, true).map_err(|e| format!("Provision configure failed: {e}"))?;
    firehose_read(device, firehose_parser_ack_nak)
        .map_err(|e| format!("Provision configure handshake failed: {e}"))?;

    for cmd in &commands {
        let mut packet = build_ufs_packet(cmd);
        firehose_write_getack(
            device,
            &mut packet,
            "send UFS provisioning command".to_string(),
        )
        .map_err(|e| format!("UFS provisioning command failed: {e}"))?;
    }

    firehose_configure(device, false)
        .map_err(|e| format!("Post-provision configure failed: {e}"))?;
    firehose_read(device, firehose_parser_ack_nak)
        .map_err(|e| format!("Post-provision configure handshake failed: {e}"))?;

    log_info!("qdl::flash", "UFS provisioning complete");
    Ok(())
}

/// Serialise one `<ufs>` command into a Firehose `<data>` packet.
fn build_ufs_packet(attrs: &[(String, String)]) -> Vec<u8> {
    let mut s = String::from("<?xml version=\"1.0\" ?>\n<data>\n  <ufs");
    for (k, v) in attrs {
        s.push(' ');
        s.push_str(k);
        s.push_str("=\"");
        s.push_str(v);
        s.push('"');
    }
    s.push_str(" />\n</data>");
    s.into_bytes()
}

/// Connect, upload the firehose programmer from `elf_path`, and configure Firehose for `storage`.
fn connect_and_configure(
    device_path: &str,
    elf_path: &Path,
    storage: QdlStorage,
    state: &Arc<FlashState>,
) -> Result<QdlDevice<dyn QdlReadWrite>, String> {
    update_qdl_stage(state, STAGE_CONNECTING);
    log_info!(
        "qdl::flash",
        "Connecting to EDL device {:?}...",
        device_path
    );

    let serial = super::detect::resolve_edl_serial(device_path).map_err(|e| {
        log_error!("qdl::flash", "Refusing EDL target {:?}: {}", device_path, e);
        e
    })?;
    log_info!(
        "qdl::flash",
        "EDL target {:?} resolved to serial {:?}",
        device_path,
        serial
    );

    let rw_channel = setup_target_device(QdlBackend::Usb, serial, None).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("errno 13")
            || msg.contains("Permission denied")
            || msg.contains("Access denied")
        {
            "[QDL_PERMISSION_DENIED]".to_string()
        } else {
            format!("[QDL_CONNECTION_FAILED] {}", msg)
        }
    })?;

    let mut device = QdlDevice {
        rw: rw_channel,
        fh_cfg: FirehoseConfiguration {
            storage_type: storage.firehose_type(),
            storage_sector_size: storage.sector_size(),
            bypass_storage: false,
            backend: QdlBackend::Usb,
            skip_firehose_log: true,
            verbose_firehose: false,
            ..Default::default()
        },
        reset_on_drop: false,
    };
    log_info!("qdl::flash", "Connected to EDL device");

    check_cancelled(state)?;
    update_qdl_stage(state, STAGE_SAHARA);
    log_info!("qdl::flash", "Starting Sahara handshake...");

    // Reading the chip serial number initiates the Sahara HELLO exchange.
    let sn = sahara_run(
        &mut device,
        SaharaMode::Command,
        Some(qdl::sahara::SaharaCmdModeCmd::ReadSerialNum),
        &mut [],
        vec![],
        false,
    )
    .map_err(|e| {
        format!(
            "Sahara handshake failed: {}. Ensure the device is in EDL mode.",
            e
        )
    })?;
    if sn.len() >= 4 {
        log_info!(
            "qdl::flash",
            "Chip serial number: {:#x}",
            u32::from_le_bytes([sn[0], sn[1], sn[2], sn[3]])
        );
    }

    // OEM key hash (best effort, result unused).
    let _ = sahara_run(
        &mut device,
        SaharaMode::Command,
        Some(qdl::sahara::SaharaCmdModeCmd::ReadOemKeyHash),
        &mut [],
        vec![],
        false,
    );

    log_info!("qdl::flash", "Uploading firehose programmer...");
    let elf_data =
        fs::read(elf_path).map_err(|e| format!("Failed to read firehose programmer: {}", e))?;
    sahara_run(
        &mut device,
        SaharaMode::WaitingForImage,
        None,
        &mut [elf_data],
        vec![],
        false,
    )
    .map_err(|e| {
        format!(
            "Sahara upload failed: {}. The firehose programmer may be incompatible.",
            e
        )
    })?;
    log_info!("qdl::flash", "Firehose programmer uploaded successfully");

    // Once the programmer is up, dropping the device should reset it.
    device.reset_on_drop = true;

    check_cancelled(state)?;
    update_qdl_stage(state, STAGE_CONFIGURING);
    log_info!("qdl::flash", "Configuring Firehose protocol...");
    firehose_read(&mut device, firehose_parser_ack_nak)
        .map_err(|e| format!("Failed to read firehose welcome: {}", e))?;
    firehose_configure(&mut device, false)
        .map_err(|e| format!("Firehose configuration failed: {}", e))?;
    firehose_read(&mut device, firehose_parser_configure_response)
        .map_err(|e| format!("Firehose configure handshake failed: {}", e))?;
    log_info!("qdl::flash", "Firehose configured successfully");

    Ok(device)
}

/// Offset of the ext4 superblock magic within a bare ext4 image, and its value.
const EXT4_SB_OFFSET: u64 = 0x438;
const EXT4_MAGIC: [u8; 2] = [0x53, 0xEF];

/// Inject first-boot autoconfig preset into the extracted ext4 rootfs from rawprogram0.xml. Skips (warn) if non-injectable
/// (non-ext4/sparse/readbackverify/file offset/multi-part, B3); ext4 write/validate fail or over window (B2) are fatal "[QDL_AUTOCONFIG_FAILED]".
fn inject_autoconfig(
    flash_dir: &Path,
    extract_root: &Path,
    config: &crate::autoconfig::AutoconfigConfig,
) -> Result<(), String> {
    let rawprogram_path = flash_dir.join(RAWPROGRAM_XML);

    let found = find_rootfs_image(extract_root, flash_dir)
        .map_err(|e| format!("{TAG_QDL_AUTOCONFIG_FAILED} {e}"))?;
    let (rootfs_path, window_bytes) = match found {
        Some(found) => found,
        None => {
            // B3: non-injectable rootfs -> skip, do not abort.
            log_warn!(
                "qdl::flash",
                "Autoconfig: no injectable ext4 rootfs found in {}; skipping injection",
                rawprogram_path.display()
            );
            return Ok(());
        }
    };

    log_info!(
        "qdl::flash",
        "Autoconfig: injecting preset into rootfs {}",
        rootfs_path.display()
    );

    // A confirmed-ext4 rootfs that fails to write/validate is fatal.
    crate::autoconfig::inject_into_bare_ext4_image(&rootfs_path, config)
        .map_err(|e| format!("{TAG_QDL_AUTOCONFIG_FAILED} {e}"))?;

    // B2: the mutated file must still fit within the partition window.
    let file_len = fs::metadata(&rootfs_path)
        .map_err(|e| {
            format!("{TAG_QDL_AUTOCONFIG_FAILED} failed to stat rootfs after injection: {e}")
        })?
        .len();
    if file_len > window_bytes {
        return Err(format!(
            "{TAG_QDL_AUTOCONFIG_FAILED} rootfs grew beyond partition window after injection \
             ({file_len} bytes > {window_bytes} bytes)"
        ));
    }

    log_info!(
        "qdl::flash",
        "Autoconfig: injection complete ({} bytes, window {} bytes)",
        file_len,
        window_bytes
    );

    Ok(())
}

fn parse_xml(xml_path: &Path) -> Result<Element, String> {
    let name = xml_path.file_name().unwrap_or_default().to_string_lossy();
    let xml_data = fs::read(xml_path).map_err(|e| format!("Failed to read {name}: {e}"))?;
    Element::parse(&xml_data[..]).map_err(|e| format!("Failed to parse {name}: {e}"))
}

fn program_elements(xml: &Element) -> impl Iterator<Item = &Element> {
    xml.children.iter().filter_map(|n| match n {
        XMLNode::Element(e) if e.name.eq_ignore_ascii_case("program") => Some(e),
        _ => None,
    })
}

// None for an entry with no file or a missing one; an error when the name leaves the extraction.
fn program_file(
    extract_root: &Path,
    flash_dir: &Path,
    attrs: &IndexMap<String, String>,
) -> Result<Option<PathBuf>, String> {
    let filename = attrs.get("filename").map(|s| s.as_str()).unwrap_or("");
    if filename.is_empty() {
        return Ok(None);
    }
    let path = resolve_flash_file(extract_root, flash_dir, filename)?;
    Ok(path.exists().then_some(path))
}

// The rootfs is written in place, so it must be a plain file owned by the extraction alone.
fn check_plain_file(path: &Path) -> Result<(), String> {
    let meta = fs::symlink_metadata(path)
        .map_err(|e| format!("failed to stat rootfs {}: {}", path.display(), e))?;
    if !meta.is_file() {
        return Err(format!("rootfs {} is not a regular file", path.display()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.nlink() != 1 {
            return Err(format!("rootfs {} has other hard links", path.display()));
        }
    }
    Ok(())
}

// Some((path, window)) only for one non-sparse, offset-0, no-readbackverify ext4 "rootfs" entry (B2/B3/M3), else None.
// A rootfs outside the extraction, a link or a hard-linked file is an error, never a skip.
fn find_rootfs_image(
    extract_root: &Path,
    flash_dir: &Path,
) -> Result<Option<(PathBuf, u64)>, String> {
    let Ok(xml) = parse_xml(&flash_dir.join(RAWPROGRAM_XML)) else {
        return Ok(None);
    };

    let rootfs_entries: Vec<&Element> = program_elements(&xml)
        .filter(|e| {
            e.attributes
                .get("label")
                .is_some_and(|s| s.eq_ignore_ascii_case("rootfs"))
        })
        .collect();

    // M3: a multi-part rootfs is not injectable.
    let [entry] = rootfs_entries[..] else {
        return Ok(None);
    };
    let attrs = &entry.attributes;

    // M3: sparse / readbackverify / non-zero file offset are not injectable.
    let is_true = |key: &str| {
        attrs
            .get(key)
            .is_some_and(|s| s.eq_ignore_ascii_case("true"))
    };
    if is_true("sparse") || is_true("readbackverify") {
        return Ok(None);
    }
    let file_sector_offset: u64 = attrs
        .get("file_sector_offset")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if file_sector_offset != 0 {
        return Ok(None);
    }

    let Some(rootfs_path) = program_file(extract_root, flash_dir, attrs)? else {
        return Ok(None);
    };
    check_plain_file(&rootfs_path)?;

    // B3: probe the ext4 superblock magic at offset 0x438; non-ext4 -> skip.
    let mut magic = [0u8; 2];
    let is_ext4 = fs::File::open(&rootfs_path)
        .and_then(|mut file| {
            file.seek(SeekFrom::Start(EXT4_SB_OFFSET))?;
            file.read_exact(&mut magic)
        })
        .is_ok()
        && magic == EXT4_MAGIC;
    if !is_ext4 {
        return Ok(None);
    }

    // B2: partition window = num_partition_sectors * SECTOR_SIZE_IN_BYTES.
    let num_partition_sectors: u64 = attrs
        .get("num_partition_sectors")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let sector_size: u64 = attrs
        .get("SECTOR_SIZE_IN_BYTES")
        .and_then(|s| s.parse().ok())
        .unwrap_or(512);
    let window_bytes = num_partition_sectors.saturating_mul(sector_size);
    if window_bytes == 0 {
        return Ok(None);
    }

    Ok(Some((rootfs_path, window_bytes)))
}

/// Parse rawprogram0.xml and program each partition
fn program_from_xml<T: QdlChan>(
    channel: &mut T,
    xml_path: &Path,
    flash_dir: &Path,
    extract_root: &Path,
    state: &Arc<FlashState>,
) -> Result<(), WriteFailure> {
    let xml = parse_xml(xml_path)?;

    // Pre-count real program entries to size the progress total.
    let mut program_entries: Vec<&Element> = Vec::new();
    for e in program_elements(&xml) {
        let num_sectors: usize = e
            .attributes
            .get("num_partition_sectors")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        if num_sectors > 0 && program_file(extract_root, flash_dir, &e.attributes)?.is_some() {
            program_entries.push(e);
        }
    }

    let total_entries = program_entries.len();
    state
        .qdl
        .partitions_total
        .store(total_entries as u64, Ordering::SeqCst);

    log_info!(
        "qdl::flash",
        "Programming {} partitions from rawprogram0.xml...",
        total_entries
    );

    // Progress total in bytes, derived from each entry's sector count.
    let sector_size = channel.fh_config().storage_sector_size;
    let total_bytes: u64 = program_entries
        .iter()
        .map(|e| {
            let num_sectors: usize = e
                .attributes
                .get("num_partition_sectors")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            num_sectors as u64 * sector_size as u64
        })
        .sum();
    state.total_bytes.store(total_bytes, Ordering::SeqCst);

    let mut bytes_written: u64 = 0;
    let mut partition_idx: u64 = 0;

    // Only <program> entries here; patches come from patch0.xml later.
    for node in &xml.children {
        if let XMLNode::Element(e) = node {
            match e.name.to_lowercase().as_str() {
                "program" => {
                    program_single_partition(
                        channel,
                        flash_dir,
                        extract_root,
                        &e.attributes,
                        state,
                        &mut bytes_written,
                        &mut partition_idx,
                    )?;
                }
                _ => {
                    // Non-program entries are ignored here.
                }
            }
        }
    }

    state
        .qdl
        .partitions_written
        .store(partition_idx, Ordering::SeqCst);
    log_info!("qdl::flash", "All partitions programmed successfully");

    Ok(())
}

/// Program a single partition from a <program> XML entry
fn program_single_partition<T: QdlChan>(
    channel: &mut T,
    flash_dir: &Path,
    extract_root: &Path,
    attrs: &IndexMap<String, String>,
    state: &Arc<FlashState>,
    bytes_written: &mut u64,
    partition_idx: &mut u64,
) -> Result<(), WriteFailure> {
    let filename = attrs.get("filename").map(|s| s.as_str()).unwrap_or("");
    let label = attrs.get("label").map(|s| s.as_str()).unwrap_or("");
    let num_sectors: usize = attrs
        .get("num_partition_sectors")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let slot: u8 = attrs.get("slot").and_then(|s| s.parse().ok()).unwrap_or(0);
    let phys_part_idx: u8 = attrs
        .get("physical_partition_number")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let start_sector = attrs.get("start_sector").map(|s| s.as_str()).unwrap_or("0");
    let file_sector_offset: u32 = attrs
        .get("file_sector_offset")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let sector_size = channel.fh_config().storage_sector_size;

    if num_sectors == 0 {
        return Ok(());
    }

    if filename.is_empty() {
        return Ok(());
    }

    let Some(file_path) = program_file(extract_root, flash_dir, attrs)? else {
        log_warn!(
            "qdl::flash",
            "Skipping missing file: {} (partition: {})",
            filename,
            label
        );
        return Ok(());
    };

    check_cancelled(state)?;

    let display_label = if label.is_empty() { filename } else { label };
    update_qdl_stage(state, &format!("{STAGE_PARTITION_PREFIX}{display_label}"));
    state
        .qdl
        .partitions_written
        .store(*partition_idx, Ordering::SeqCst);

    log_info!(
        "qdl::flash",
        "Programming partition: {} (file: {}, sectors: {})",
        display_label,
        filename,
        num_sectors,
    );

    let mut file =
        fs::File::open(&file_path).map_err(|e| format!("Failed to open {}: {}", filename, e))?;

    if file_sector_offset > 0 {
        file.seek(SeekFrom::Current(
            sector_size as i64 * file_sector_offset as i64,
        ))
        .map_err(|e| format!("Failed to seek in {}: {}", filename, e))?;
    }

    let base_bytes = *bytes_written;
    let progress_state = state.clone();
    let cancel_state = state.clone();
    let mut reader = ProgressReader::new(file, cancel_state, move |bytes_transferred| {
        progress_state
            .written_bytes
            .store(base_bytes + bytes_transferred, Ordering::SeqCst);
    });

    firehose_program_storage(
        channel,
        &mut reader,
        display_label,
        num_sectors,
        slot,
        phys_part_idx,
        start_sector,
    )
    .map_err(|e| {
        let failure = WriteFailure::classify(e.as_ref());
        WriteFailure {
            message: format!(
                "Failed to program partition {display_label}: {}",
                failure.message
            ),
            ..failure
        }
    })?;

    // Settle progress on the sector-count figure, matching the total_bytes math.
    *bytes_written += num_sectors as u64 * sector_size as u64;
    state.written_bytes.store(*bytes_written, Ordering::SeqCst);
    *partition_idx += 1;

    log_info!(
        "qdl::flash",
        "Partition {} programmed successfully",
        display_label
    );

    Ok(())
}

/// Parse patch0.xml and apply patches via Firehose
fn patch_from_xml<T: QdlChan>(channel: &mut T, patch_path: &Path) -> Result<(), String> {
    let xml = parse_xml(patch_path)?;

    let mut patch_count = 0;
    for node in &xml.children {
        if let XMLNode::Element(e) = node {
            if e.name.to_lowercase() == "patch" {
                // Apply only patches that target device storage (filename == "DISK").
                let filename = e
                    .attributes
                    .get("filename")
                    .map(|s| s.as_str())
                    .unwrap_or("");
                if filename != "DISK" {
                    continue;
                }

                let byte_off: u64 = e
                    .attributes
                    .get("byte_offset")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let slot: u8 = e
                    .attributes
                    .get("slot")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let phys_part_idx: u8 = e
                    .attributes
                    .get("physical_partition_number")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let size: u64 = e
                    .attributes
                    .get("size_in_bytes")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let start_sector = e
                    .attributes
                    .get("start_sector")
                    .map(|s| s.as_str())
                    .unwrap_or("0");
                let value = e.attributes.get("value").map(|s| s.as_str()).unwrap_or("");

                firehose_patch(
                    channel,
                    byte_off,
                    slot,
                    phys_part_idx,
                    size,
                    start_sector,
                    value,
                )
                .map_err(|e| format!("Patch command failed: {}", e))?;

                patch_count += 1;
            }
        }
    }

    log_info!("qdl::flash", "Applied {} patches", patch_count);
    Ok(())
}

/// Update the QDL stage name in the shared flash state
pub(crate) fn update_qdl_stage(state: &FlashState, stage: &str) {
    let mut s = state.qdl.stage.lock().unwrap_or_else(|p| p.into_inner());
    *s = stage.to_string();
}

/// Check if the operation has been cancelled and return an error if so
pub(crate) fn check_cancelled(state: &FlashState) -> Result<(), String> {
    if state.is_cancelled.load(Ordering::SeqCst) {
        log_info!("qdl::flash", "Operation cancelled by user");
        Err(QDL_CANCELLED_ERROR.to_string())
    } else {
        Ok(())
    }
}

// A cancel after the last byte skips the normal reset: the drop sends the board back to EDL.
fn check_cancelled_after_write(state: &FlashState) -> Result<(), String> {
    if state.is_cancelled.load(Ordering::SeqCst) {
        log_info!(
            "qdl::flash",
            "Operation cancelled by user after the image was fully written; the board is left in EDL"
        );
        Err(QDL_CANCELLED_ERROR.to_string())
    } else {
        Ok(())
    }
}

// Fills the whole buffer: qdlrs sends every chunk at full size and ignores the returned count.
struct ProgressReader<R: Read, F: FnMut(u64)> {
    inner: R,
    bytes_transferred: u64,
    on_progress: F,
    state: Arc<FlashState>,
}

impl<R: Read, F: FnMut(u64)> ProgressReader<R, F> {
    fn new(inner: R, state: Arc<FlashState>, on_progress: F) -> Self {
        Self {
            inner,
            bytes_transferred: 0,
            on_progress,
            state,
        }
    }
}

impl<R: Read, F: FnMut(u64)> Read for ProgressReader<R, F> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut filled = 0;
        while filled < buf.len() {
            if self.state.is_cancelled.load(Ordering::SeqCst) {
                return Err(std::io::Error::other(QDL_CANCELLED_ERROR));
            }
            match self.inner.read(&mut buf[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        // Progress counts what qdlrs sends per chunk, matching the sector-based totals.
        self.bytes_transferred += buf.len() as u64;
        (self.on_progress)(self.bytes_transferred);
        if filled == 0 {
            return Ok(0);
        }
        // Zero padding only after EOF, the same bytes qdlrs sends from its zeroed buffer.
        buf[filled..].fill(0);
        Ok(buf.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOTFS_WINDOW_SECTORS: u64 = 20_920_568;

    struct Layout {
        dir: PathBuf,
        root: PathBuf,
        flash_dir: PathBuf,
    }

    impl Drop for Layout {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    // The real Arduino UNO Q rawprogram0.xml shape, with `filename` for the rootfs entry.
    fn arduino_layout(tag: &str, rootfs_filename: &str) -> Layout {
        let dir = crate::utils::test_scratch_dir(tag);
        let root = super::super::extract::extraction_dir(&dir);
        let flash_dir = root.join("arduino-images").join("flash");
        fs::create_dir_all(&flash_dir).unwrap();
        let mut rootfs = vec![0u8; 4096];
        rootfs[EXT4_SB_OFFSET as usize..EXT4_SB_OFFSET as usize + 2].copy_from_slice(&EXT4_MAGIC);
        fs::write(
            root.join("arduino-images").join("disk-sdcard.img.root"),
            rootfs,
        )
        .unwrap();
        fs::write(
            flash_dir.join(RAWPROGRAM_XML),
            format!(
                r#"<?xml version="1.0" ?><data><program SECTOR_SIZE_IN_BYTES="512" file_sector_offset="0" filename="{rootfs_filename}" label="rootfs" num_partition_sectors="{ROOTFS_WINDOW_SECTORS}" physical_partition_number="0" readbackverify="false" sparse="false" start_sector="1050624"/></data>"#
            ),
        )
        .unwrap();
        Layout {
            dir,
            root,
            flash_dir,
        }
    }

    #[test]
    fn the_arduino_rootfs_resolves_inside_the_extraction() {
        let layout = arduino_layout("ok", "../disk-sdcard.img.root");
        let (path, window) = find_rootfs_image(&layout.root, &layout.flash_dir)
            .unwrap()
            .unwrap();
        assert!(path.starts_with(layout.root.canonicalize().unwrap()));
        assert_eq!(window, ROOTFS_WINDOW_SECTORS * 512);
    }

    #[test]
    fn a_rootfs_outside_the_extraction_is_an_error_not_a_skip() {
        for escape in ["../../../user.img", "/etc/hosts"] {
            let layout = arduino_layout("escape", escape);
            assert!(
                find_rootfs_image(&layout.root, &layout.flash_dir).is_err(),
                "{escape}"
            );
        }
        let missing = arduino_layout("missing", "../absent.root");
        assert!(find_rootfs_image(&missing.root, &missing.flash_dir)
            .unwrap()
            .is_none());
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_rootfs_is_never_injected() {
        let layout = arduino_layout("symlink", "rootfs.link");
        let real = layout
            .root
            .join("arduino-images")
            .join("disk-sdcard.img.root");
        std::os::unix::fs::symlink(&real, layout.flash_dir.join("rootfs.link")).unwrap();
        assert!(find_rootfs_image(&layout.root, &layout.flash_dir).is_err());

        let layout = arduino_layout("hardlink", "../disk-sdcard.img.root");
        let real = layout
            .root
            .join("arduino-images")
            .join("disk-sdcard.img.root");
        fs::hard_link(&real, layout.root.join("second-name")).unwrap();
        assert!(find_rootfs_image(&layout.root, &layout.flash_dir).is_err());
    }

    #[test]
    fn every_program_file_must_stay_inside_the_extraction() {
        let layout = arduino_layout("program", "../disk-sdcard.img.root");
        let attrs = |filename: &str| {
            let mut attrs = IndexMap::new();
            attrs.insert("filename".to_string(), filename.to_string());
            attrs
        };
        assert!(program_file(
            &layout.root,
            &layout.flash_dir,
            &attrs("../disk-sdcard.img.root")
        )
        .unwrap()
        .is_some());
        assert!(program_file(&layout.root, &layout.flash_dir, &attrs(""))
            .unwrap()
            .is_none());
        assert!(program_file(&layout.root, &layout.flash_dir, &attrs("../../../etc.img")).is_err());
    }

    #[test]
    fn an_empty_ufs_image_is_refused_before_connecting() {
        let tmp = tempfile::tempdir().unwrap();
        let image = tmp.path().join("empty-ufs.img");
        fs::write(&image, b"").unwrap();
        let state = Arc::new(FlashState::new());

        let err = qdl_flash_ufs(
            &image,
            &tmp.path().join("missing-loader.elf"),
            "qdl://missing",
            ProvisionSource::Absent,
            state.clone(),
        )
        .unwrap_err();

        assert!(err.starts_with(crate::utils::TAG_IMAGE_INVALID), "{err}");
        assert!(!state.qdl.is_active.load(Ordering::SeqCst));
    }

    #[test]
    fn ufs_provisioning_packets_keep_document_attribute_order() {
        let tmp = tempfile::tempdir().unwrap();
        let xml = tmp.path().join("provision.xml");
        fs::write(
            &xml,
            r#"<?xml version="1.0" ?>
<data>
  <ufs bNumberLU="1" bBootEnable="1" bDescrAccessEn="0" LUNum="0" bLUEnable="1" commit="false"/>
  <ufs LUNum="0" bLUEnable="1" size_in_kb="0" commit="true"/>
</data>"#,
        )
        .unwrap();

        let commands = super::super::provision::parse_ufs_commands(&xml).unwrap();
        let keys: Vec<&str> = commands[0].iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            keys,
            [
                "bNumberLU",
                "bBootEnable",
                "bDescrAccessEn",
                "LUNum",
                "bLUEnable",
                "commit"
            ]
        );
        assert_eq!(
            String::from_utf8(build_ufs_packet(&commands[1])).unwrap(),
            "<?xml version=\"1.0\" ?>\n<data>\n  <ufs LUNum=\"0\" bLUEnable=\"1\" size_in_kb=\"0\" commit=\"true\" />\n</data>"
        );
    }

    #[test]
    fn raw_num_sectors_rounds_up() {
        assert_eq!(raw_num_sectors(0, 4096), 0);
        assert_eq!(raw_num_sectors(4096, 4096), 1);
        assert_eq!(raw_num_sectors(4097, 4096), 2);
        assert_eq!(raw_num_sectors(8192, 4096), 2);
        assert_eq!(raw_num_sectors(512, 512), 1);
    }

    mod session {
        use super::super::super::outcome::settle;
        use super::super::super::{
            QDLRS_PROGRAM_NAK, QDLRS_RECEIVE_DATA, QDLRS_SEND_DATA, QDLRS_SEND_ZLP,
            QDLRS_WRITE_OP_NAK, SECTOR_SIZE_EMMC, SECTOR_SIZE_UFS, TAG_QDL_DISCONNECTED,
        };
        use super::*;
        use std::collections::VecDeque;
        use std::io::{self, BufRead, Write};
        use std::sync::Mutex;

        type WriteLog = Arc<Mutex<Vec<Vec<u8>>>>;

        const ACK: &[u8] =
            br#"<?xml version="1.0" encoding="UTF-8" ?><data><response value="ACK" /></data>"#;
        const NAK: &[u8] =
            br#"<?xml version="1.0" encoding="UTF-8" ?><data><response value="NAK" /></data>"#;
        const UFS_CHUNK: usize = 2 * SECTOR_SIZE_UFS;
        const EMMC_CHUNK: usize = 2 * SECTOR_SIZE_EMMC;
        const RESET_TO_EDL: &str = r#"<power value="reset_to_edl""#;
        const NORMAL_RESET: &str = r#"<power value="reset""#;

        /// Scripted EDL endpoint: one XML reply per Firehose read, TimedOut once the script ends.
        struct MockUsb {
            replies: VecDeque<Vec<u8>>,
            current: Vec<u8>,
            pos: usize,
            log: WriteLog,
            writes: usize,
            fail_write: Option<(usize, io::ErrorKind)>,
            cancel_on_write: Option<(usize, Arc<FlashState>)>,
            served: usize,
            cancel_on_reply: Option<(usize, Arc<FlashState>)>,
        }

        impl MockUsb {
            fn new(replies: &[&[u8]], log: &WriteLog) -> Self {
                Self {
                    replies: replies.iter().map(|r| r.to_vec()).collect(),
                    current: Vec::new(),
                    pos: 0,
                    log: log.clone(),
                    writes: 0,
                    fail_write: None,
                    cancel_on_write: None,
                    served: 0,
                    cancel_on_reply: None,
                }
            }

            fn cancel_on_reply(mut self, index: usize, state: &Arc<FlashState>) -> Self {
                self.cancel_on_reply = Some((index, state.clone()));
                self
            }

            fn fail_write(mut self, index: usize, kind: io::ErrorKind) -> Self {
                self.fail_write = Some((index, kind));
                self
            }

            fn cancel_on_write(mut self, index: usize, state: &Arc<FlashState>) -> Self {
                self.cancel_on_write = Some((index, state.clone()));
                self
            }
        }

        impl Write for MockUsb {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                let index = self.writes;
                self.writes += 1;
                if let Some((at, kind)) = self.fail_write {
                    if at == index {
                        return Err(io::Error::new(kind, "mock write failure"));
                    }
                }
                self.log.lock().unwrap().push(buf.to_vec());
                if let Some((at, state)) = &self.cancel_on_write {
                    if *at == index {
                        state.is_cancelled.store(true, Ordering::SeqCst);
                    }
                }
                Ok(buf.len())
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        impl Read for MockUsb {
            fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                let available = self.fill_buf()?;
                let n = available.len().min(out.len());
                out[..n].copy_from_slice(&available[..n]);
                self.consume(n);
                Ok(n)
            }
        }

        impl BufRead for MockUsb {
            fn fill_buf(&mut self) -> io::Result<&[u8]> {
                if self.pos >= self.current.len() {
                    let Some(next) = self.replies.pop_front() else {
                        return Err(io::Error::new(io::ErrorKind::TimedOut, "mock timeout"));
                    };
                    self.current = next;
                    self.pos = 0;
                    if let Some((at, state)) = &self.cancel_on_reply {
                        if *at == self.served {
                            state.is_cancelled.store(true, Ordering::SeqCst);
                        }
                    }
                    self.served += 1;
                }
                Ok(&self.current[self.pos..])
            }

            fn consume(&mut self, amt: usize) {
                self.pos = (self.pos + amt).min(self.current.len());
            }
        }

        impl QdlReadWrite for MockUsb {}

        fn device(mock: MockUsb, storage: QdlStorage, chunk: usize) -> QdlDevice<dyn QdlReadWrite> {
            QdlDevice {
                rw: Box::new(mock),
                fh_cfg: FirehoseConfiguration {
                    storage_type: storage.firehose_type(),
                    storage_sector_size: storage.sector_size(),
                    send_buffer_size: chunk,
                    bypass_storage: false,
                    backend: QdlBackend::Usb,
                    skip_firehose_log: true,
                    verbose_firehose: false,
                    ..Default::default()
                },
                reset_on_drop: true,
            }
        }

        fn count(log: &WriteLog, needle: &str) -> usize {
            log.lock()
                .unwrap()
                .iter()
                .filter(|w| w.windows(needle.len()).any(|win| win == needle.as_bytes()))
                .count()
        }

        fn writes(log: &WriteLog) -> usize {
            log.lock().unwrap().len()
        }

        fn stage(state: &FlashState) -> String {
            state.qdl.stage.lock().unwrap().clone()
        }

        fn pattern(len: usize) -> Vec<u8> {
            (0..len).map(|i| (i % 251) as u8).collect()
        }

        struct Ufs {
            _dir: tempfile::TempDir,
            image: PathBuf,
            image_bytes: Vec<u8>,
            provision: PathBuf,
            log: WriteLog,
            state: Arc<FlashState>,
        }

        // 18000 bytes = 5 UFS sectors: writes are #0 program, #1-#3 chunks, #4 ZLP.
        fn ufs() -> Ufs {
            let dir = tempfile::tempdir().unwrap();
            let image = dir.path().join("image.img");
            let image_bytes = pattern(18_000);
            fs::write(&image, &image_bytes).unwrap();
            let provision = dir.path().join("provision.xml");
            fs::write(
                &provision,
                r#"<?xml version="1.0" ?><data><ufs bNumberLU="1" LUNum="0" commit="false"/><ufs LUNum="0" size_in_kb="0" commit="true"/></data>"#,
            )
            .unwrap();
            Ufs {
                _dir: dir,
                image,
                image_bytes,
                provision,
                log: Arc::new(Mutex::new(Vec::new())),
                state: Arc::new(FlashState::new()),
            }
        }

        impl Ufs {
            fn mock(&self, replies: &[&[u8]]) -> MockUsb {
                MockUsb::new(replies, &self.log)
            }

            fn run(&self, mock: MockUsb, ready: bool) -> Result<(), String> {
                let provision = if ready {
                    ProvisionSource::Ready(self.provision.clone())
                } else {
                    ProvisionSource::Absent
                };
                run_ufs_session(
                    device(mock, QdlStorage::Ufs, UFS_CHUNK),
                    &self.image,
                    self.image_bytes.len() as u64,
                    provision,
                    &self.state,
                )
            }

            fn writing_stage(&self) -> String {
                format!("{STAGE_PARTITION_PREFIX}{UFS_PARTITION_LABEL}")
            }
        }

        #[test]
        fn a_cancel_mid_stream_stops_the_write_and_disarms() {
            let rig = ufs();
            let mock = rig.mock(&[ACK]).cancel_on_write(1, &rig.state);
            let result = rig.run(mock, true);

            assert!(result.is_err());
            let cancelled = rig.state.is_cancelled.load(Ordering::SeqCst);
            assert!(cancelled);
            assert_eq!(
                settle(result, cancelled),
                Err(QDL_CANCELLED_ERROR.to_string())
            );
            let log = rig.log.lock().unwrap().clone();
            assert_eq!(log.len(), 2, "program XML and the first chunk only");
            assert!(String::from_utf8_lossy(&log[0]).contains("<program"));
            assert_eq!(log[1], rig.image_bytes[..UFS_CHUNK]);
            assert_eq!(count(&rig.log, "<power"), 0);
            assert_eq!(stage(&rig.state), rig.writing_stage());
        }

        #[test]
        fn a_failed_data_chunk_sends_nothing_more_and_never_provisions() {
            for kind in [
                io::ErrorKind::ConnectionAborted,
                io::ErrorKind::Interrupted,
                io::ErrorKind::TimedOut,
                io::ErrorKind::Other,
            ] {
                let rig = ufs();
                let mock = rig.mock(&[ACK, ACK, ACK]).fail_write(2, kind);
                let err = rig.run(mock, true).unwrap_err();

                assert_eq!(writes(&rig.log), 2, "{kind:?}");
                assert_eq!(count(&rig.log, "<power"), 0, "{kind:?}");
                assert_eq!(count(&rig.log, "<ufs"), 0, "{kind:?}");
                assert_eq!(count(&rig.log, "<configure"), 0, "{kind:?}");
                assert_eq!(stage(&rig.state), rig.writing_stage(), "{kind:?}");
                assert!(err.contains(QDLRS_SEND_DATA), "{err}");
                assert!(err.contains(TAG_QDL_DISCONNECTED), "{err}");
                assert!(!err.contains(QDL_CANCELLED_ERROR), "{err}");
            }
        }

        #[test]
        fn a_failed_zlp_is_a_disconnect_and_disarms() {
            let rig = ufs();
            let mock = rig
                .mock(&[ACK, ACK, ACK])
                .fail_write(4, io::ErrorKind::Other);
            let err = rig.run(mock, true).unwrap_err();

            assert!(
                err.contains(&format!("{TAG_QDL_DISCONNECTED} {QDLRS_SEND_ZLP}")),
                "{err}"
            );
            assert_eq!(writes(&rig.log), 4);
            assert_eq!(count(&rig.log, "<power"), 0);
        }

        #[test]
        fn a_program_nak_provisions_then_writes_again() {
            let rig = ufs();
            let mock = rig.mock(&[NAK, ACK, ACK, ACK, ACK, ACK, ACK, ACK]);
            rig.run(mock, true).unwrap();

            assert_eq!(count(&rig.log, "<ufs "), 2);
            assert_eq!(count(&rig.log, "<configure"), 2);
            assert_eq!(count(&rig.log, "<program"), 2);
            let log = rig.log.lock().unwrap().clone();
            let first_ufs = log
                .iter()
                .position(|w| w.windows(5).any(|x| x == b"<ufs "))
                .unwrap();
            let last_program = log
                .iter()
                .rposition(|w| w.windows(8).any(|x| x == b"<program"))
                .unwrap();
            assert!(first_ufs < last_program);
            assert_eq!(stage(&rig.state), STAGE_COMPLETE);
        }

        #[test]
        fn a_cancel_at_the_program_nak_never_provisions() {
            let rig = ufs();
            let mock = rig
                .mock(&[NAK, ACK, ACK, ACK, ACK, ACK, ACK, ACK])
                .cancel_on_reply(0, &rig.state);
            let err = rig.run(mock, true).unwrap_err();

            assert_eq!(err, QDL_CANCELLED_ERROR);
            assert_eq!(count(&rig.log, "<ufs"), 0);
            assert_eq!(count(&rig.log, "<configure"), 0);
            assert_eq!(stage(&rig.state), rig.writing_stage());
            assert_eq!(count(&rig.log, RESET_TO_EDL), 1);
            assert_eq!(count(&rig.log, "<power"), 1);
        }

        #[test]
        fn a_program_nak_without_a_descriptor_reports_unprovisioned_and_resets_to_edl() {
            let rig = ufs();
            let err = rig.run(rig.mock(&[NAK]), false).unwrap_err();

            assert!(err.contains(QDLRS_PROGRAM_NAK), "{err}");
            assert!(err.contains("unprovisioned"), "{err}");
            assert_eq!(count(&rig.log, RESET_TO_EDL), 1);
            assert_eq!(count(&rig.log, "<power"), 1);
            assert_eq!(writes(&rig.log), 2);
        }

        #[test]
        fn an_end_of_write_nak_keeps_the_device_armed() {
            let rig = ufs();
            let err = rig.run(rig.mock(&[ACK, NAK, ACK]), true).unwrap_err();

            assert!(err.contains(QDLRS_WRITE_OP_NAK), "{err}");
            assert!(!err.contains(TAG_QDL_DISCONNECTED), "{err}");
            assert_eq!(count(&rig.log, RESET_TO_EDL), 1);
            assert_eq!(count(&rig.log, "<power"), 1);
            assert_eq!(count(&rig.log, "<ufs"), 0);
        }

        #[test]
        fn a_failure_in_the_write_after_provisioning_disarms() {
            // Writes: #0 program (NAK), #1-#4 provisioning, #5 program, #6 first chunk.
            let rig = ufs();
            let mock = rig
                .mock(&[NAK, ACK, ACK, ACK, ACK, ACK])
                .fail_write(6, io::ErrorKind::ConnectionAborted);
            let err = rig.run(mock, true).unwrap_err();
            assert!(err.contains("after provisioning"), "{err}");
            assert!(err.contains(TAG_QDL_DISCONNECTED), "{err}");
            assert_eq!(count(&rig.log, "<power"), 0);
            assert_eq!(writes(&rig.log), 6);

            let rig = ufs();
            let err = rig
                .run(rig.mock(&[NAK, ACK, ACK, ACK, ACK, NAK, ACK]), true)
                .unwrap_err();
            assert!(err.contains("after provisioning"), "{err}");
            assert_eq!(count(&rig.log, "<ufs "), 2, "no second provisioning");
            assert_eq!(count(&rig.log, "<program"), 2);
            assert_eq!(count(&rig.log, "<power"), 0);
        }

        #[test]
        fn a_completed_write_resets_and_completes() {
            let rig = ufs();
            rig.run(rig.mock(&[ACK, ACK, ACK]), true).unwrap();

            let log = rig.log.lock().unwrap().clone();
            let sent: Vec<u8> = log[1..4].concat();
            let mut expected = rig.image_bytes.clone();
            expected.resize(5 * SECTOR_SIZE_UFS, 0);
            assert_eq!(sent, expected);
            assert!(log[4].is_empty(), "ZLP");
            assert_eq!(count(&rig.log, NORMAL_RESET), 1);
            assert_eq!(count(&rig.log, RESET_TO_EDL), 0);
            assert_eq!(stage(&rig.state), STAGE_COMPLETE);
            assert_eq!(
                rig.state.written_bytes.load(Ordering::SeqCst),
                expected.len() as u64
            );
        }

        #[test]
        fn a_cancel_after_the_last_chunk_leaves_the_board_in_edl() {
            let rig = ufs();
            let mock = rig.mock(&[ACK, ACK, ACK]).cancel_on_write(4, &rig.state);
            let err = rig.run(mock, true).unwrap_err();

            assert_eq!(err, QDL_CANCELLED_ERROR);
            assert_eq!(count(&rig.log, RESET_TO_EDL), 1);
            assert_eq!(count(&rig.log, NORMAL_RESET), 0);
            assert_ne!(stage(&rig.state), STAGE_COMPLETE);
            assert_ne!(stage(&rig.state), STAGE_RESETTING);
        }

        struct Tar {
            _dir: tempfile::TempDir,
            root: PathBuf,
            flash_dir: PathBuf,
            log: WriteLog,
            state: Arc<FlashState>,
        }

        // Each 2048-byte partition at 512-byte sectors: program, two chunks, ZLP (#0-#3 for the first).
        fn tar(file: &str, with_patch: bool) -> Tar {
            tar_with(&[(file, "boot")], with_patch)
        }

        fn tar_with(partitions: &[(&str, &str)], with_patch: bool) -> Tar {
            let dir = tempfile::tempdir().unwrap();
            let root = super::super::super::extract::extraction_dir(dir.path());
            let flash_dir = root.join("flash");
            fs::create_dir_all(&flash_dir).unwrap();
            fs::write(flash_dir.join("boot.bin"), pattern(2048)).unwrap();
            fs::write(flash_dir.join("second.bin"), pattern(2048)).unwrap();
            let programs: String = partitions
                .iter()
                .map(|(file, label)| {
                    format!(
                        r#"<program SECTOR_SIZE_IN_BYTES="512" filename="{file}" label="{label}" num_partition_sectors="4" physical_partition_number="0" start_sector="0"/>"#
                    )
                })
                .collect();
            fs::write(
                flash_dir.join(RAWPROGRAM_XML),
                format!(r#"<?xml version="1.0" ?><data>{programs}</data>"#),
            )
            .unwrap();
            if with_patch {
                fs::write(
                    flash_dir.join(PATCH_XML),
                    r#"<?xml version="1.0" ?><data><patch SECTOR_SIZE_IN_BYTES="512" byte_offset="0" filename="DISK" physical_partition_number="0" size_in_bytes="8" start_sector="0" value="0"/></data>"#,
                )
                .unwrap();
            }
            Tar {
                _dir: dir,
                root,
                flash_dir,
                log: Arc::new(Mutex::new(Vec::new())),
                state: Arc::new(FlashState::new()),
            }
        }

        impl Tar {
            fn run(&self, mock: MockUsb) -> Result<(), String> {
                run_tar_session(
                    device(mock, QdlStorage::Emmc, EMMC_CHUNK),
                    &self.flash_dir,
                    &self.root,
                    None,
                    &self.state,
                )
            }
        }

        #[test]
        fn a_tar_partition_write_error_disarms() {
            let rig = tar("boot.bin", true);
            let mock = MockUsb::new(&[ACK, ACK, ACK], &rig.log)
                .fail_write(1, io::ErrorKind::ConnectionAborted);
            let err = rig.run(mock).unwrap_err();

            assert!(err.starts_with("Failed to program partition boot"), "{err}");
            assert!(err.contains(TAG_QDL_DISCONNECTED), "{err}");
            assert_eq!(writes(&rig.log), 1);
            assert_eq!(count(&rig.log, "<patch"), 0);
            assert_eq!(count(&rig.log, "<power"), 0);
        }

        #[cfg(unix)]
        #[test]
        fn a_tar_open_failure_before_any_data_keeps_the_device_armed() {
            // open(2) refuses a socket for root too, unlike a permission bit.
            let rig = tar("boot.sock", true);
            let _socket =
                std::os::unix::net::UnixListener::bind(rig.flash_dir.join("boot.sock")).unwrap();
            let err = rig.run(MockUsb::new(&[], &rig.log)).unwrap_err();

            assert!(err.starts_with("Failed to open boot.sock"), "{err}");
            assert_eq!(count(&rig.log, "<program"), 0);
            assert_eq!(count(&rig.log, "<patch"), 0);
            assert_eq!(count(&rig.log, RESET_TO_EDL), 1);
        }

        #[test]
        fn a_tar_missing_file_is_skipped_without_sending_data() {
            let rig = tar("absent.bin", false);
            rig.run(MockUsb::new(&[ACK], &rig.log)).unwrap();

            assert_eq!(count(&rig.log, "<program"), 0);
            assert_eq!(count(&rig.log, NORMAL_RESET), 1);
            assert_eq!(stage(&rig.state), STAGE_COMPLETE);
        }

        #[test]
        fn a_tar_cancel_after_the_last_partition_leaves_the_board_in_edl() {
            let rig = tar("boot.bin", false);
            let mock = MockUsb::new(&[ACK, ACK, ACK], &rig.log).cancel_on_write(3, &rig.state);
            let err = rig.run(mock).unwrap_err();

            assert_eq!(err, QDL_CANCELLED_ERROR);
            assert_eq!(count(&rig.log, RESET_TO_EDL), 1);
            assert_eq!(count(&rig.log, NORMAL_RESET), 0);
            assert_ne!(stage(&rig.state), STAGE_RESETTING);
        }

        #[test]
        fn a_tar_program_nak_never_provisions_and_resets_to_edl() {
            let rig = tar("boot.bin", true);
            let err = rig.run(MockUsb::new(&[NAK], &rig.log)).unwrap_err();

            assert!(err.starts_with("Failed to program partition boot"), "{err}");
            assert!(err.contains(QDLRS_PROGRAM_NAK), "{err}");
            assert_eq!(count(&rig.log, "<ufs"), 0);
            assert_eq!(count(&rig.log, "<patch"), 0);
            assert_eq!(count(&rig.log, RESET_TO_EDL), 1);
            assert_eq!(count(&rig.log, "<power"), 1);
        }

        #[test]
        fn a_tar_cancel_between_partitions_never_starts_the_next_one() {
            let rig = tar_with(&[("boot.bin", "boot"), ("second.bin", "second")], true);
            let mock = MockUsb::new(&[ACK, ACK, ACK, ACK], &rig.log).cancel_on_write(3, &rig.state);
            let err = rig.run(mock).unwrap_err();

            assert_eq!(err, QDL_CANCELLED_ERROR);
            assert_eq!(count(&rig.log, "<program"), 1);
            assert_eq!(count(&rig.log, r#"label="second""#), 0);
            assert_eq!(count(&rig.log, "<patch"), 0);
            assert_eq!(count(&rig.log, RESET_TO_EDL), 1);
            assert_eq!(count(&rig.log, NORMAL_RESET), 0);
            assert_ne!(stage(&rig.state), STAGE_COMPLETE);
        }

        #[test]
        fn a_tar_patch_nak_stops_before_the_normal_reset() {
            // Replies: program ACK, end-of-write ACK, patch NAK, ACK to the reset qdlrs sends on that NAK.
            let rig = tar("boot.bin", true);
            let err = rig
                .run(MockUsb::new(&[ACK, ACK, NAK, ACK], &rig.log))
                .unwrap_err();

            assert!(err.starts_with("Patch command failed"), "{err}");
            assert_eq!(count(&rig.log, "<patch"), 1);
            assert_eq!(count(&rig.log, NORMAL_RESET), 0);
            assert_eq!(stage(&rig.state), STAGE_PATCHING);
            // One reset_to_edl from qdlrs on the NAK, one from the still-armed drop.
            assert_eq!(count(&rig.log, RESET_TO_EDL), 2);
        }

        #[test]
        fn the_receive_fragment_matches_the_crate_panic_text() {
            let log: WriteLog = Arc::new(Mutex::new(Vec::new()));
            let mut dev = device(MockUsb::new(&[ACK], &log), QdlStorage::Ufs, UFS_CHUNK);
            dev.reset_on_drop = false;
            let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let _ = qdl::firehose_read_storage(&mut dev, &mut io::sink(), 1, 0, 0, 0);
            }))
            .unwrap_err();
            let text = panic.downcast_ref::<String>().cloned().unwrap_or_default();
            assert!(text.starts_with(QDLRS_RECEIVE_DATA), "{text}");
            assert!(super::super::super::outcome::is_disconnect_panic(&text));
        }

        /// Hands out at most 3 bytes per call and an Interrupted error every fourth call.
        struct ShortReader {
            data: Vec<u8>,
            pos: usize,
            calls: usize,
        }

        impl Read for ShortReader {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                self.calls += 1;
                if self.calls.is_multiple_of(4) {
                    return Err(io::Error::new(io::ErrorKind::Interrupted, "retry"));
                }
                let n = buf.len().min(3).min(self.data.len() - self.pos);
                buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
                self.pos += n;
                Ok(n)
            }
        }

        #[test]
        fn short_reads_never_leave_a_gap_and_pad_only_after_eof() {
            let source = pattern(10_000);
            let state = Arc::new(FlashState::new());
            let progress = Arc::new(std::sync::atomic::AtomicU64::new(0));
            let seen = progress.clone();
            let inner = ShortReader {
                data: source.clone(),
                pos: 0,
                calls: 0,
            };
            let mut reader = ProgressReader::new(inner, state, move |n| {
                seen.store(n, Ordering::SeqCst);
            });

            // Same shape as qdlrs: a fresh zeroed buffer per chunk, count ignored.
            let mut out = Vec::new();
            for _ in 0..3 {
                let mut buf = vec![0u8; SECTOR_SIZE_UFS];
                let _ = reader.read(&mut buf).unwrap();
                out.extend_from_slice(&buf);
            }
            let mut expected = source;
            expected.resize(3 * SECTOR_SIZE_UFS, 0);
            assert_eq!(out, expected);
            assert_eq!(progress.load(Ordering::SeqCst), out.len() as u64);

            let mut buf = vec![0u8; SECTOR_SIZE_UFS];
            assert_eq!(reader.read(&mut buf).unwrap(), 0);
        }

        #[test]
        fn a_cancelled_reader_fails_with_the_cancel_text_not_interrupted() {
            let state = Arc::new(FlashState::new());
            state.is_cancelled.store(true, Ordering::SeqCst);
            let inner = ShortReader {
                data: pattern(64),
                pos: 0,
                calls: 3,
            };
            let mut reader = ProgressReader::new(inner, state, |_| {});
            let err = reader.read(&mut [0u8; 16]).unwrap_err();
            assert_eq!(err.to_string(), QDL_CANCELLED_ERROR);
            assert_ne!(err.kind(), io::ErrorKind::Interrupted);
        }
    }
}

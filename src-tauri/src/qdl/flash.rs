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

    // Connect, upload the firehose programmer, and configure Firehose (eMMC defaults).
    let elf_path = flash_dir.join(FIREHOSE_ELF);
    let mut device = connect_and_configure(device_path, &elf_path, QdlStorage::Emmc, &state)?;

    // --- Autoconfig injection (still within the "configuring" stage) ---
    // Inject first-boot preset into the extracted ext4 rootfs blob IN PLACE before Firehose reads it.
    // Skipped silently when no profile selected or rootfs is not an injectable bare-ext4 image.
    if let Some(cfg) = autoconfig.as_ref() {
        check_cancelled(&state)?;
        inject_autoconfig(flash_dir, extract_root, cfg)?;
    }

    // --- Stage 4: Program partitions from rawprogram0.xml ---
    check_cancelled(&state)?;
    update_qdl_stage(&state, STAGE_FIREHOSE);

    program_from_xml(
        &mut device,
        &rawprogram_path,
        flash_dir,
        extract_root,
        &state,
    )?;

    // --- Stage 5: Apply patches from patch0.xml ---
    let patch_path = flash_dir.join(PATCH_XML);
    if patch_path.exists() {
        check_cancelled(&state)?;
        update_qdl_stage(&state, STAGE_PATCHING);
        log_info!("qdl::flash", "Applying patches from patch0.xml...");
        patch_from_xml(&mut device, &patch_path)?;
    }

    // --- Stage 6: Reset device ---
    update_qdl_stage(&state, STAGE_RESETTING);
    log_info!("qdl::flash", "Resetting device...");

    device.reset_on_drop = false;
    firehose_reset(&mut device, &FirehoseResetMode::Reset, 0)
        .map_err(|e| {
            log_warn!("qdl::flash", "Device reset failed (non-fatal): {}", e);
        })
        .ok();

    update_qdl_stage(&state, STAGE_COMPLETE);
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
    state.qdl.is_active.store(true, Ordering::SeqCst);

    let mut device = connect_and_configure(device_path, elf_path, QdlStorage::Ufs, &state)?;

    check_cancelled(&state)?;
    update_qdl_stage(&state, STAGE_FIREHOSE);

    let sector_size = QdlStorage::Ufs.sector_size();
    let img_len = fs::metadata(image_path)
        .map_err(|e| format!("Failed to stat image: {}", e))?
        .len();
    let num_sectors = raw_num_sectors(img_len, sector_size);
    let total_bytes = num_sectors as u64 * sector_size as u64;
    state.qdl.partitions_total.store(1, Ordering::SeqCst);
    state.total_bytes.store(total_bytes, Ordering::SeqCst);
    update_qdl_stage(
        &state,
        &format!("{STAGE_PARTITION_PREFIX}{UFS_PARTITION_LABEL}"),
    );

    log_info!(
        "qdl::flash",
        "Writing {} bytes ({} sectors) to UFS...",
        img_len,
        num_sectors
    );

    if let Err(e) = write_ufs_image(&mut device, image_path, num_sectors, &state) {
        // A program NAK on UFS means the module has no LUN 0 (a brand-new, unprovisioned module):
        // provision it in-session, then retry the write once.
        if e.contains("NAKed") {
            match provision {
                ProvisionSource::Ready(prov) => {
                    check_cancelled(&state)?;
                    update_qdl_stage(&state, STAGE_PROVISIONING);
                    provision_ufs(&mut device, &prov)?;
                    check_cancelled(&state)?;
                    update_qdl_stage(&state, STAGE_FIREHOSE);
                    state.written_bytes.store(0, Ordering::SeqCst);
                    write_ufs_image(&mut device, image_path, num_sectors, &state).map_err(|e| {
                        format!("Failed to write UFS image after provisioning: {e}")
                    })?;
                }
                ProvisionSource::Absent => {
                    return Err(format!("Failed to write UFS image: {e} The UFS module is likely unprovisioned (no LUN 0); provision it before flashing."));
                }
                ProvisionSource::Unavailable(reason) => {
                    return Err(format!("Failed to write UFS image: {e} The module looks unprovisioned and auto-provisioning could not run: {reason}"));
                }
            }
        } else {
            return Err(format!("Failed to write UFS image: {e}"));
        }
    }

    state.qdl.partitions_written.store(1, Ordering::SeqCst);
    state.written_bytes.store(total_bytes, Ordering::SeqCst);

    update_qdl_stage(&state, STAGE_RESETTING);
    log_info!("qdl::flash", "Resetting device...");
    device.reset_on_drop = false;
    firehose_reset(&mut device, &FirehoseResetMode::Reset, 0)
        .map_err(|e| {
            log_warn!("qdl::flash", "Device reset failed (non-fatal): {}", e);
        })
        .ok();

    update_qdl_stage(&state, STAGE_COMPLETE);
    log_info!("qdl::flash", "QDL UFS flash completed successfully");
    Ok(())
}

/// Sectors needed to hold `len` bytes, rounding the final partial sector up.
fn raw_num_sectors(len: u64, sector: usize) -> usize {
    len.div_ceil(sector as u64) as usize
}

/// Stream `image_path` to UFS LUN 0 sector 0. Returns the raw qdlrs error string on failure
/// so the caller can branch on a `<program>` NAK (unprovisioned module).
fn write_ufs_image(
    device: &mut QdlDevice<dyn QdlReadWrite>,
    image_path: &Path,
    num_sectors: usize,
    state: &Arc<FlashState>,
) -> Result<(), String> {
    let file = fs::File::open(image_path).map_err(|e| format!("Failed to open image: {}", e))?;
    let progress_state = state.clone();
    let mut reader = ProgressReader::new(file, state.clone(), move |bytes_transferred| {
        progress_state
            .written_bytes
            .store(bytes_transferred, Ordering::SeqCst);
    });
    firehose_program_storage(device, &mut reader, "system", num_sectors, 0, 0, "0")
        .map_err(|e| e.to_string())
}

/// Provision a blank UFS module in the current session from the qcombin `<ufs>` descriptor.
/// Configures with SkipStorageInit (a blank module has no LUN to init), sends each `<ufs>`
/// command, then re-initialises storage so the new LUN 0 is writable.
fn provision_ufs(device: &mut QdlDevice<dyn QdlReadWrite>, xml_path: &Path) -> Result<(), String> {
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
) -> Result<(), String> {
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
) -> Result<(), String> {
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

    // Wrap file in ProgressReader for real-time progress and mid-partition cancellation
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
    .map_err(|e| format!("Failed to program partition {}: {}", display_label, e))?;

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

/// Read wrapper reporting progress and aborting on cancellation; counts requested
/// buffer size (not bytes returned) so progress matches the sector-based totals.
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
        // Per-chunk cancellation check keeps cancel responsive mid-partition.
        if self.state.is_cancelled.load(Ordering::SeqCst) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "Operation cancelled by user",
            ));
        }
        let n = self.inner.read(buf)?;
        self.bytes_transferred += buf.len() as u64;
        (self.on_progress)(self.bytes_transferred);
        Ok(n)
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
    fn raw_num_sectors_rounds_up() {
        assert_eq!(raw_num_sectors(0, 4096), 0);
        assert_eq!(raw_num_sectors(4096, 4096), 1);
        assert_eq!(raw_num_sectors(4097, 4096), 2);
        assert_eq!(raw_num_sectors(8192, 4096), 2);
        assert_eq!(raw_num_sectors(512, 512), 1);
    }
}

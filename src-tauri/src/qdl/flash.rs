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

use super::extract::FIREHOSE_ELF;
use super::provision::ProvisionSource;
use super::QdlStorage;
use crate::flash::FlashState;
use crate::{log_info, log_warn};

/// Execute the full QDL flash: upload firehose, program partitions from `flash_dir`,
/// optionally targeting a device by `serial`, reporting progress via `state`.
pub fn qdl_flash(
    flash_dir: &Path,
    serial: Option<String>,
    autoconfig: Option<crate::autoconfig::AutoconfigConfig>,
    state: Arc<FlashState>,
) -> Result<(), String> {
    state.qdl.is_active.store(true, Ordering::SeqCst);

    // Connect, upload the firehose programmer, and configure Firehose (eMMC defaults).
    let elf_path = flash_dir.join(FIREHOSE_ELF);
    let mut device = connect_and_configure(serial, &elf_path, QdlStorage::Emmc, &state)?;

    // --- Autoconfig injection (still within the "configuring" stage) ---
    // Inject first-boot preset into the extracted ext4 rootfs blob IN PLACE before Firehose reads it.
    // Skipped silently when no profile selected or rootfs is not an injectable bare-ext4 image.
    if let Some(cfg) = autoconfig.as_ref() {
        check_cancelled(&state)?;
        inject_autoconfig(flash_dir, cfg)?;
    }

    // --- Stage 4: Program partitions from rawprogram0.xml ---
    check_cancelled(&state)?;
    update_qdl_stage(&state, "firehose");

    let rawprogram_path = flash_dir.join("rawprogram0.xml");
    program_from_xml(&mut device, &rawprogram_path, flash_dir, &state)?;

    // --- Stage 5: Apply patches from patch0.xml ---
    let patch_path = flash_dir.join("patch0.xml");
    if patch_path.exists() {
        check_cancelled(&state)?;
        update_qdl_stage(&state, "patching");
        log_info!("qdl::flash", "Applying patches from patch0.xml...");
        patch_from_xml(&mut device, &patch_path)?;
    }

    // --- Stage 6: Reset device ---
    update_qdl_stage(&state, "resetting");
    log_info!("qdl::flash", "Resetting device...");

    device.reset_on_drop = false;
    firehose_reset(&mut device, &FirehoseResetMode::Reset, 0)
        .map_err(|e| {
            log_warn!("qdl::flash", "Device reset failed (non-fatal): {}", e);
        })
        .ok();

    update_qdl_stage(&state, "complete");
    log_info!("qdl::flash", "QDL flash completed successfully");

    Ok(())
}

/// Flash a whole `.img` to UFS via one raw Firehose write to LUN 0 sector 0
/// (`edl-ng --memory ufs write-sector 0 <img>` equivalent).
pub fn qdl_flash_ufs(
    image_path: &Path,
    elf_path: &Path,
    serial: Option<String>,
    autoconfig: Option<crate::autoconfig::AutoconfigConfig>,
    provision: ProvisionSource,
    state: Arc<FlashState>,
) -> Result<(), String> {
    state.qdl.is_active.store(true, Ordering::SeqCst);

    let mut device = connect_and_configure(serial, elf_path, QdlStorage::Ufs, &state)?;

    // Inject before Firehose reads the image; detect.rs handles the 4096-byte UFS sectors.
    if let Some(cfg) = autoconfig.as_ref() {
        check_cancelled(&state)?;
        crate::autoconfig::inject_into_image(image_path, cfg)
            .map_err(|e| format!("[QDL_AUTOCONFIG_FAILED] {}", e))?;
    }

    check_cancelled(&state)?;
    update_qdl_stage(&state, "firehose");

    let sector_size = QdlStorage::Ufs.sector_size();
    let img_len = fs::metadata(image_path)
        .map_err(|e| format!("Failed to stat image: {}", e))?
        .len();
    let num_sectors = raw_num_sectors(img_len, sector_size);
    let total_bytes = num_sectors as u64 * sector_size as u64;
    state.qdl.partitions_total.store(1, Ordering::SeqCst);
    state.total_bytes.store(total_bytes, Ordering::SeqCst);
    {
        let mut stage = state.qdl.stage.lock().unwrap_or_else(|p| p.into_inner());
        *stage = "partition:system".to_string();
    }

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
                    update_qdl_stage(&state, "provisioning");
                    provision_ufs(&mut device, &prov)?;
                    check_cancelled(&state)?;
                    update_qdl_stage(&state, "firehose");
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

    update_qdl_stage(&state, "resetting");
    log_info!("qdl::flash", "Resetting device...");
    device.reset_on_drop = false;
    firehose_reset(&mut device, &FirehoseResetMode::Reset, 0)
        .map_err(|e| {
            log_warn!("qdl::flash", "Device reset failed (non-fatal): {}", e);
        })
        .ok();

    update_qdl_stage(&state, "complete");
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
    serial: Option<String>,
    elf_path: &Path,
    storage: QdlStorage,
    state: &Arc<FlashState>,
) -> Result<QdlDevice<dyn QdlReadWrite>, String> {
    update_qdl_stage(state, "connecting");
    log_info!("qdl::flash", "Connecting to EDL device...");

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
    update_qdl_stage(state, "sahara");
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
    update_qdl_stage(state, "configuring");
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
    config: &crate::autoconfig::AutoconfigConfig,
) -> Result<(), String> {
    let rawprogram_path = flash_dir.join("rawprogram0.xml");

    let (rootfs_path, window_bytes) = match find_rootfs_image(flash_dir) {
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
        .map_err(|e| format!("[QDL_AUTOCONFIG_FAILED] {}", e))?;

    // B2: the mutated file must still fit within the partition window.
    let file_len = fs::metadata(&rootfs_path)
        .map_err(|e| {
            format!(
                "[QDL_AUTOCONFIG_FAILED] failed to stat rootfs after injection: {}",
                e
            )
        })?
        .len();
    if file_len > window_bytes {
        return Err(format!(
            "[QDL_AUTOCONFIG_FAILED] rootfs grew beyond partition window after injection \
             ({} bytes > {} bytes)",
            file_len, window_bytes
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

/// Resolve injectable rootfs from rawprogram0.xml: `Some((path, window_bytes))` only if exactly one `label="rootfs"` entry is non-sparse,
/// `readbackverify != true`, `file_sector_offset == 0`, file exists, ext4 magic at 0x438; else `None` (skip, B3/M3). Window = `num_partition_sectors * SECTOR_SIZE_IN_BYTES` (B2).
fn find_rootfs_image(flash_dir: &Path) -> Option<(PathBuf, u64)> {
    let xml_path = flash_dir.join("rawprogram0.xml");
    let xml_data = fs::read(&xml_path).ok()?;
    let xml = Element::parse(&xml_data[..]).ok()?;

    // Collect all program entries labelled "rootfs".
    let rootfs_entries: Vec<&Element> = xml
        .children
        .iter()
        .filter_map(|n| {
            if let XMLNode::Element(e) = n {
                if e.name.to_lowercase() == "program"
                    && e.attributes
                        .get("label")
                        .map(|s| s.eq_ignore_ascii_case("rootfs"))
                        .unwrap_or(false)
                {
                    return Some(e);
                }
            }
            None
        })
        .collect();

    // M3: a multi-part rootfs is not injectable.
    if rootfs_entries.len() != 1 {
        return None;
    }
    let entry = rootfs_entries[0];
    let attrs = &entry.attributes;

    let filename = attrs.get("filename").map(|s| s.as_str()).unwrap_or("");
    if filename.is_empty() {
        return None;
    }

    // M3: sparse / readbackverify / non-zero file offset are not injectable.
    let sparse = attrs
        .get("sparse")
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if sparse {
        return None;
    }
    let readbackverify = attrs
        .get("readbackverify")
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if readbackverify {
        return None;
    }
    let file_sector_offset: u64 = attrs
        .get("file_sector_offset")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if file_sector_offset != 0 {
        return None;
    }

    // M2: resolve as flash_dir.join(filename) + .exists() (no canonicalize).
    let rootfs_path = flash_dir.join(filename);
    if !rootfs_path.exists() {
        return None;
    }

    // B3: probe the ext4 superblock magic at offset 0x438; non-ext4 -> skip.
    let mut file = fs::File::open(&rootfs_path).ok()?;
    file.seek(SeekFrom::Start(EXT4_SB_OFFSET)).ok()?;
    let mut magic = [0u8; 2];
    if file.read_exact(&mut magic).is_err() || magic != EXT4_MAGIC {
        return None;
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
        return None;
    }

    Some((rootfs_path, window_bytes))
}

/// Parse rawprogram0.xml and program each partition
fn program_from_xml<T: QdlChan>(
    channel: &mut T,
    xml_path: &Path,
    flash_dir: &Path,
    state: &Arc<FlashState>,
) -> Result<(), String> {
    let xml_data =
        fs::read(xml_path).map_err(|e| format!("Failed to read rawprogram0.xml: {}", e))?;

    let xml = Element::parse(&xml_data[..])
        .map_err(|e| format!("Failed to parse rawprogram0.xml: {}", e))?;

    // Pre-count real program entries to size the progress total.
    let program_entries: Vec<&Element> = xml
        .children
        .iter()
        .filter_map(|n| {
            if let XMLNode::Element(e) = n {
                if e.name.to_lowercase() == "program" {
                    let filename = e
                        .attributes
                        .get("filename")
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    let num_sectors: usize = e
                        .attributes
                        .get("num_partition_sectors")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    if !filename.is_empty() && num_sectors > 0 && flash_dir.join(filename).exists()
                    {
                        return Some(e);
                    }
                }
            }
            None
        })
        .collect();

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

    let file_path = flash_dir.join(filename);
    if !file_path.exists() {
        log_warn!(
            "qdl::flash",
            "Skipping missing file: {} (partition: {})",
            filename,
            label
        );
        return Ok(());
    }

    check_cancelled(state)?;

    let display_label = if label.is_empty() { filename } else { label };
    {
        let mut stage = state.qdl.stage.lock().unwrap_or_else(|p| p.into_inner());
        *stage = format!("partition:{}", display_label);
    }
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
    let xml_data = fs::read(patch_path).map_err(|e| format!("Failed to read patch0.xml: {}", e))?;

    let xml =
        Element::parse(&xml_data[..]).map_err(|e| format!("Failed to parse patch0.xml: {}", e))?;

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
fn update_qdl_stage(state: &FlashState, stage: &str) {
    let mut s = state.qdl.stage.lock().unwrap_or_else(|p| p.into_inner());
    *s = stage.to_string();
}

/// Check if the operation has been cancelled and return an error if so
fn check_cancelled(state: &FlashState) -> Result<(), String> {
    if state.is_cancelled.load(Ordering::SeqCst) {
        log_info!("qdl::flash", "Operation cancelled by user");
        Err("QDL flash cancelled by user".to_string())
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

    #[test]
    fn raw_num_sectors_rounds_up() {
        assert_eq!(raw_num_sectors(0, 4096), 0);
        assert_eq!(raw_num_sectors(4096, 4096), 1);
        assert_eq!(raw_num_sectors(4097, 4096), 2);
        assert_eq!(raw_num_sectors(8192, 4096), 2);
        assert_eq!(raw_num_sectors(512, 512), 1);
    }
}

//! QDL flash orchestration
//!
//! Coordinates the full QDL flashing pipeline:
//! 1. Connect to the EDL device via USB
//! 2. Upload firehose programmer via Sahara protocol
//! 3. Configure Firehose and program partitions from rawprogram0.xml
//! 4. Apply patches from patch0.xml
//! 5. Reset the device

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use indexmap::IndexMap;
use qdl::parsers::{firehose_parser_ack_nak, firehose_parser_configure_response};
use qdl::sahara::{sahara_run, SaharaMode};
use qdl::types::{
    FirehoseConfiguration, FirehoseResetMode, FirehoseStorageType, QdlBackend, QdlChan, QdlDevice,
};
use qdl::{
    firehose_configure, firehose_patch, firehose_program_storage, firehose_read, firehose_reset,
    setup_target_device,
};
use xmltree::{Element, XMLNode};

use crate::flash::FlashState;
use crate::{log_info, log_warn};

/// Execute the full QDL flash operation
///
/// # Arguments
/// * `flash_dir` - Directory containing firehose ELF, rawprogram0.xml, and partition images
/// * `serial` - Optional USB serial number to target a specific device
/// * `state` - Shared flash state for progress reporting and cancellation
pub fn qdl_flash(
    flash_dir: &Path,
    serial: Option<String>,
    state: Arc<FlashState>,
) -> Result<(), String> {
    // Set QDL mode flag
    state.qdl.is_active.store(true, Ordering::SeqCst);

    // --- Stage 1: Connect to EDL device ---
    update_qdl_stage(&state, "connecting");
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
            storage_type: FirehoseStorageType::Emmc,
            storage_sector_size: 512,
            bypass_storage: false,
            backend: QdlBackend::Usb,
            skip_firehose_log: true,
            verbose_firehose: false,
            ..Default::default()
        },
        reset_on_drop: false,
    };

    log_info!("qdl::flash", "Connected to EDL device");

    // --- Stage 2: Sahara handshake and firehose programmer upload ---
    check_cancelled(&state)?;
    update_qdl_stage(&state, "sahara");
    log_info!("qdl::flash", "Starting Sahara handshake...");

    // Step 2a: Read chip serial number (initiates the Sahara HELLO exchange)
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
        let serial = u32::from_le_bytes([sn[0], sn[1], sn[2], sn[3]]);
        log_info!("qdl::flash", "Chip serial number: {:#x}", serial);
    }

    // Step 2b: Read OEM key hash
    let _ = sahara_run(
        &mut device,
        SaharaMode::Command,
        Some(qdl::sahara::SaharaCmdModeCmd::ReadOemKeyHash),
        &mut [],
        vec![],
        false,
    );

    // Step 2c: Upload firehose programmer
    log_info!("qdl::flash", "Uploading firehose programmer...");

    let elf_path = flash_dir.join("prog_firehose_ddr.elf");
    let elf_data =
        fs::read(&elf_path).map_err(|e| format!("Failed to read firehose programmer: {}", e))?;

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

    // Activate reset-on-drop now that Sahara is done
    device.reset_on_drop = true;

    // --- Stage 3: Configure Firehose ---
    check_cancelled(&state)?;
    update_qdl_stage(&state, "configuring");
    log_info!("qdl::flash", "Configuring Firehose protocol...");

    // Read welcome logs
    firehose_read(&mut device, firehose_parser_ack_nak)
        .map_err(|e| format!("Failed to read firehose welcome: {}", e))?;

    // Send configuration
    firehose_configure(&mut device, false)
        .map_err(|e| format!("Firehose configuration failed: {}", e))?;

    // Parse configure response (may negotiate buffer sizes)
    firehose_read(&mut device, firehose_parser_configure_response)
        .map_err(|e| format!("Firehose configure handshake failed: {}", e))?;

    log_info!("qdl::flash", "Firehose configured successfully");

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

    // Count programmable entries for progress tracking
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
                    // Skip empty or zero-length entries
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

    // Calculate total bytes for progress based on sector counts
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

    // Process all XML children (program, patch, read, etc.)
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
                    // Skip non-program entries in rawprogram0.xml
                    // (patches are handled separately from patch0.xml)
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

    // Skip zero-length entries
    if num_sectors == 0 {
        return Ok(());
    }

    // Skip entries without files (allow-missing semantics)
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

    // Check cancellation before each partition
    check_cancelled(state)?;

    // Update progress stage
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

    // Apply file sector offset if specified
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

    // Finalize progress using sector count (matches total_bytes calculation)
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
                // Only apply patches targeting device storage (filename == "DISK")
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

/// A Read wrapper that tracks data transfer progress and supports cancellation.
///
/// Counts the buffer size requested per `read()` call (not bytes returned),
/// because `firehose_program_storage` sends full buffers even after the file
/// reaches EOF (zero-padded). This matches sector-based total_bytes.
///
/// Also checks `is_cancelled` on each read, allowing cancellation mid-partition
/// (within ~1MB granularity). The qdlrs library calls `.unwrap()` on the read
/// result, so returning an error triggers a panic that is caught by tokio's
/// `spawn_blocking` and converted to a user-visible error.
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
        // Check cancellation on every chunk (~1MB) for responsive cancel
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

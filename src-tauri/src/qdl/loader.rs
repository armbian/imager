//! Fetches and caches the Qualcomm firehose loader (`prog_firehose_ddr.elf`) for a
//! board's SoC family from the armbian/qcombin repo, used to flash QDL/EDL devices.

use std::path::{Path, PathBuf};

use crate::config;
use crate::log_info;
use crate::qdl::extract::FIREHOSE_ELF;
use crate::utils::{fetch_to_file, loaders_dir};

const MIN_LOADER_SIZE: u64 = 64 * 1024;
const ELF_MAGIC: &[u8; 4] = b"\x7fELF";

// SoC token -> qcombin family, matched case-insensitively by substring.
const SOC_FAMILY: &[(&str, &str)] = &[
    ("QCS6490", "Kodiak"),
    ("QCM6490", "Kodiak"),
    ("QCS5430", "Kodiak"),
    ("QCM5430", "Kodiak"),
    ("SC7280", "Kodiak"),
    ("SM7325", "Kodiak"),
    ("QRB2210", "Agatti"),
    ("QCM2290", "Agatti"),
    ("QCS2290", "Agatti"),
];

pub fn family_for_soc(soc: &str) -> Option<&'static str> {
    let soc = soc.to_uppercase();
    SOC_FAMILY
        .iter()
        .find(|(token, _)| soc.contains(token))
        .map(|(_, family)| *family)
}

pub fn resolve_family(soc: &str, board_slug: &str) -> Option<&'static str> {
    family_for_soc(soc)
        .or_else(|| super::boards::find(board_slug).and_then(|b| family_for_soc(b.soc)))
}

/// Ensure the board's firehose loader is cached locally, downloading from qcombin on a miss.
pub async fn ensure_loader(soc: &str, board_slug: &str) -> Result<PathBuf, String> {
    let family = resolve_family(soc, board_slug).ok_or_else(|| {
        format!("No QDL firehose loader known for SoC '{soc}' / board '{board_slug}'")
    })?;

    let path = loaders_dir().join(family).join(FIREHOSE_ELF);
    if path.exists() && validate_loader(&path).is_ok() {
        log_info!(
            "qdl::loader",
            "Using cached firehose loader: {}",
            path.display()
        );
        return Ok(path);
    }

    let url = format!(
        "{}{}/{}",
        config::urls::QCOMBIN_LOADER_BASE,
        family,
        FIREHOSE_ELF
    );
    log_info!("qdl::loader", "Downloading firehose loader: {}", url);
    fetch_to_file(&url, &path, validate_loader_bytes).await?;
    log_info!(
        "qdl::loader",
        "Cached firehose loader at {}",
        path.display()
    );
    Ok(path)
}

/// Validate an on-disk loader: ELF magic + a plausible minimum size.
fn validate_loader(path: &Path) -> Result<(), String> {
    use std::io::Read;
    let len = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if len < MIN_LOADER_SIZE {
        return Err(format!("loader too small ({len} bytes)"));
    }
    let mut head = [0u8; 4];
    std::fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut head))
        .map_err(|e| e.to_string())?;
    if &head != ELF_MAGIC {
        return Err("not an ELF file".into());
    }
    Ok(())
}

/// Validate freshly downloaded loader bytes before installing them.
fn validate_loader_bytes(bytes: &[u8]) -> Result<(), String> {
    if (bytes.len() as u64) < MIN_LOADER_SIZE {
        return Err(format!(
            "downloaded loader too small ({} bytes)",
            bytes.len()
        ));
    }
    if !bytes.starts_with(ELF_MAGIC) {
        return Err("downloaded loader is not an ELF file".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_socs_case_insensitively() {
        assert_eq!(family_for_soc("QCS6490"), Some("Kodiak"));
        assert_eq!(family_for_soc("qcm6490"), Some("Kodiak"));
        assert_eq!(family_for_soc("Qualcomm QCS6490"), Some("Kodiak"));
        assert_eq!(family_for_soc("QRB2210"), Some("Agatti"));
        assert_eq!(family_for_soc("rk3588"), None);
        assert_eq!(family_for_soc(""), None);
    }

    #[test]
    fn resolves_family_from_board_slug_when_soc_missing() {
        assert_eq!(resolve_family("", "radxa-dragon-q6a"), Some("Kodiak"));
        assert_eq!(resolve_family("QCS6490", ""), Some("Kodiak"));
        assert_eq!(resolve_family("", "orangepi-5"), None);
    }
}

//! Fetches and caches the Qualcomm firehose loader (`prog_firehose_ddr.elf`) for a
//! board's SoC family from the armbian/qcombin repo, used to flash QDL/EDL devices.

use std::path::PathBuf;

use crate::config;
use crate::log_info;
use crate::qdl::extract::FIREHOSE_ELF;
use crate::qdl::registry::ResolvedQdl;
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

/// Ensure the board's firehose loader is cached locally, downloading it from the
/// API blob proxy on a miss and verifying the API-supplied SHA-256.
pub async fn ensure_loader(resolved: &ResolvedQdl) -> Result<PathBuf, String> {
    let loader_name = resolved.loader_rel.as_deref().unwrap_or(FIREHOSE_ELF);
    let path = loaders_dir().join(&resolved.family).join(loader_name);
    let expected = resolved.loader_sha256.clone();

    // Cache hit only if the on-disk copy still matches the API's current digest, so an
    // updated loader on the server re-downloads instead of serving a stale file.
    if let Ok(bytes) = std::fs::read(&path) {
        if validate_loader_bytes(&bytes, expected.as_deref()).is_ok() {
            log_info!(
                "qdl::loader",
                "Using cached firehose loader: {}",
                path.display()
            );
            return Ok(path);
        }
    }

    let url = format!(
        "{}{}/{}",
        config::urls::qdl_blob_base(),
        resolved.family,
        loader_name
    );
    log_info!("qdl::loader", "Downloading firehose loader: {}", url);
    fetch_to_file(&url, &path, move |bytes| {
        validate_loader_bytes(bytes, expected.as_deref())
    })
    .await?;
    log_info!(
        "qdl::loader",
        "Cached firehose loader at {}",
        path.display()
    );
    Ok(path)
}

/// Validate loader bytes: ELF magic + a plausible minimum size + optional digest.
fn validate_loader_bytes(bytes: &[u8], expected_sha: Option<&str>) -> Result<(), String> {
    if (bytes.len() as u64) < MIN_LOADER_SIZE {
        return Err(format!(
            "downloaded loader too small ({} bytes)",
            bytes.len()
        ));
    }
    if !bytes.starts_with(ELF_MAGIC) {
        return Err("downloaded loader is not an ELF file".into());
    }
    crate::qdl::registry::verify_digest(bytes, expected_sha)
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
}

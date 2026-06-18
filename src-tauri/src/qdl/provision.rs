//! Fetches the per-board UFS provisioning descriptor from qcombin and parses its `<ufs>`
//! commands, used to set up a brand-new (unprovisioned) UFS module before the first write.

use std::path::PathBuf;

use xmltree::{Element, XMLNode};

use crate::config;
use crate::log_info;
use crate::qdl::loader::resolve_family;
use crate::utils::{fetch_to_file, loaders_dir};

fn provision_rel_path(board_slug: &str) -> Option<&'static str> {
    crate::qdl::boards::find(board_slug).and_then(|b| b.provision_rel)
}

/// Where a board's UFS provisioning descriptor stands: ready, not declared, or expected but unreachable.
pub enum ProvisionSource {
    Ready(PathBuf),
    /// Board declares no descriptor; a blank module must be provisioned manually.
    Absent,
    /// Board declares a descriptor but it couldn't be fetched (404/network); carries the reason.
    Unavailable(String),
}

/// Locate the board's UFS provisioning XML, downloading it to the cache on first use.
pub async fn ensure_provision_xml(soc: &str, board_slug: &str) -> ProvisionSource {
    let (Some(family), Some(rel)) = (
        resolve_family(soc, board_slug),
        provision_rel_path(board_slug),
    ) else {
        return ProvisionSource::Absent;
    };
    match fetch_provision_xml(family, rel).await {
        Ok(path) => ProvisionSource::Ready(path),
        Err(e) => ProvisionSource::Unavailable(e),
    }
}

async fn fetch_provision_xml(family: &str, rel: &str) -> Result<PathBuf, String> {
    let path = loaders_dir().join(family).join(rel);
    if path.exists() {
        log_info!(
            "qdl::provision",
            "Using cached provision XML: {}",
            path.display()
        );
        return Ok(path);
    }
    let url = format!("{}{}/{}", config::urls::QCOMBIN_LOADER_BASE, family, rel);
    log_info!("qdl::provision", "Downloading provision XML: {}", url);
    fetch_to_file(&url, &path, validate_provision_bytes).await?;
    Ok(path)
}

fn validate_provision_bytes(bytes: &[u8]) -> Result<(), String> {
    if !bytes.windows(5).any(|w| w == b"<ufs ") {
        return Err("Downloaded provision XML has no <ufs> commands".to_string());
    }
    Ok(())
}

/// Parse the `<ufs>` elements (in document order) into per-command attribute lists.
pub fn parse_ufs_commands(path: &std::path::Path) -> Result<Vec<Vec<(String, String)>>, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read provision XML: {e}"))?;
    let root =
        Element::parse(&data[..]).map_err(|e| format!("Failed to parse provision XML: {e}"))?;

    let commands: Vec<Vec<(String, String)>> = root
        .children
        .into_iter()
        .filter_map(|node| match node {
            XMLNode::Element(e) if e.name == "ufs" => Some(e.attributes.into_iter().collect()),
            _ => None,
        })
        .collect();

    if commands.is_empty() {
        return Err("Provision XML contains no <ufs> commands".to_string());
    }
    Ok(commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_provision_path_by_slug() {
        assert_eq!(
            provision_rel_path("radxa-dragon-q6a"),
            Some("radxa-dragon-q6a/provision_ufs31_lun0_only.xml")
        );
        assert_eq!(provision_rel_path("orangepi-5"), None);
    }
}

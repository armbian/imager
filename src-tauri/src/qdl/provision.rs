//! Fetches the per-board UFS provisioning descriptor from qcombin and parses its `<ufs>`
//! commands, used to set up a brand-new (unprovisioned) UFS module before the first write.

use std::path::PathBuf;

use xmltree::{Element, XMLNode};

use crate::config;
use crate::log_info;
use crate::qdl::registry::ResolvedQdl;
use crate::utils::{fetch_to_file, loaders_dir};

/// Where a board's UFS provisioning descriptor stands: ready, not declared, or expected but unreachable.
pub enum ProvisionSource {
    Ready(PathBuf),
    /// Board declares no descriptor; a blank module must be provisioned manually.
    Absent,
    /// Board declares a descriptor but it couldn't be fetched (404/network); carries the reason.
    Unavailable(String),
}

/// Locate the board's UFS provisioning XML, downloading it to the cache on first use.
pub async fn ensure_provision_xml(resolved: &ResolvedQdl) -> ProvisionSource {
    let Some(rel) = resolved.provision_rel.as_deref() else {
        return ProvisionSource::Absent;
    };
    match fetch_provision_xml(&resolved.family, rel, resolved.provision_sha256.as_deref()).await {
        Ok(path) => ProvisionSource::Ready(path),
        Err(e) => ProvisionSource::Unavailable(e),
    }
}

async fn fetch_provision_xml(
    family: &str,
    rel: &str,
    expected_sha: Option<&str>,
) -> Result<PathBuf, String> {
    let path = loaders_dir().join(family).join(rel);
    // Cache hit only if the on-disk copy still matches the API's current digest, so an
    // updated descriptor on the server re-downloads instead of serving a stale file.
    if let Ok(bytes) = std::fs::read(&path) {
        if validate_provision_bytes(&bytes, expected_sha).is_ok() {
            log_info!(
                "qdl::provision",
                "Using cached provision XML: {}",
                path.display()
            );
            return Ok(path);
        }
    }
    let url = format!("{}{}/{}", config::urls::qdl_blob_base(), family, rel);
    log_info!("qdl::provision", "Downloading provision XML: {}", url);
    let expected = expected_sha.map(str::to_string);
    fetch_to_file(&url, &path, move |bytes| {
        validate_provision_bytes(bytes, expected.as_deref())
    })
    .await?;
    Ok(path)
}

fn validate_provision_bytes(bytes: &[u8], expected_sha: Option<&str>) -> Result<(), String> {
    if !bytes.windows(5).any(|w| w == b"<ufs ") {
        return Err("Downloaded provision XML has no <ufs> commands".to_string());
    }
    crate::qdl::registry::verify_digest(bytes, expected_sha)
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

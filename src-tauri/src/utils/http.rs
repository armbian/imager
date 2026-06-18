//! Shared builder for reqwest clients with the standard user agent.

use std::path::Path;
use std::time::Duration;

use crate::config;

/// Build a reqwest client with the standard user agent and the given timeout.
/// For plain HTTP; the headered API client lives in images/mod.rs.
pub fn build_client(timeout: std::time::Duration) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(config::app::USER_AGENT)
        .timeout(timeout)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

/// Fetch `url` and install the body at `dest` atomically (temp + rename), after `validate`
/// accepts the bytes. Creates parent dirs. Backs the QDL loader/provision caches.
pub async fn fetch_to_file(
    url: &str,
    dest: &Path,
    validate: impl Fn(&[u8]) -> Result<(), String>,
) -> Result<(), String> {
    let client = build_client(Duration::from_secs(config::http::REQUEST_TIMEOUT_SECS))?;
    let bytes = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch {url}: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Download error for {url}: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read body from {url}: {e}"))?;

    validate(&bytes)?;

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create cache dir: {e}"))?;
    }
    let tmp = dest.with_extension("part");
    std::fs::write(&tmp, &bytes).map_err(|e| format!("Failed to write {}: {e}", dest.display()))?;
    std::fs::rename(&tmp, dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("Failed to install {}: {e}", dest.display())
    })
}

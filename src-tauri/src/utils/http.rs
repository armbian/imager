//! Shared builder for reqwest clients with the standard user agent.

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

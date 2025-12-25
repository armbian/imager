use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub body: Option<String>,
    pub html_url: String,
    pub published_at: String,
}

/// Fetches release information from GitHub API for a specific version tag
///
/// # Arguments
/// * `version` - Version number (with or without 'v' prefix, e.g., "1.0.0" or "v1.0.0")
///
/// # Returns
/// * `Ok(GitHubRelease)` - Release metadata including notes, published date, and HTML URL
/// * `Err(String)` - Error message if version is empty or API call fails
#[command]
pub async fn get_github_release(version: String) -> Result<GitHubRelease, String> {
    // Validate and trim version parameter
    let version = version.trim();
    if version.is_empty() {
        return Err("Version cannot be empty".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent("Armbian-Imager")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Ensure version has 'v' prefix (GitHub releases use v1.1.9 format)
    let version_tag = if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{}", version)
    };

    let url = format!(
        "https://api.github.com/repos/armbian/imager/releases/tags/{}",
        version_tag
    );

    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API returned error: {}", response.status()));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(release)
}

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

/// Fetch GitHub release metadata for a version tag (with or without a 'v' prefix)
#[command]
pub async fn get_github_release(version: String) -> Result<GitHubRelease, String> {
    let version = version.trim();
    if version.is_empty() {
        return Err("Version cannot be empty".to_string());
    }

    // Standard user agent comes from build_client; use the default request timeout.
    let client = crate::utils::build_client(std::time::Duration::from_secs(
        crate::config::http::REQUEST_TIMEOUT_SECS,
    ))?;

    // GitHub release tags are v-prefixed (e.g. v1.1.9).
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

/// Whether the app runs from /Applications (always true off macOS). macOS
/// auto-updates fail outside /Applications; the frontend uses this to warn.
#[command]
pub async fn is_app_in_applications() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let exe_path =
            std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;
        let canonical = std::fs::canonicalize(&exe_path).unwrap_or(exe_path);
        let path_str = canonical.to_str().unwrap_or("");
        Ok(path_str.starts_with("/Applications/"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}

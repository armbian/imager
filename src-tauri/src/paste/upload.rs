//! Log upload to paste.armbian.com, a Hastebin instance accepting raw text via POST.

use std::fs;

use crate::logging::{get_current_log_path, get_log_dir};
use crate::{log_error, log_info};

/// Paste service configuration
const PASTE_URL: &str = "https://paste.armbian.com";
const PASTE_ENDPOINT: &str = "/log";

/// Result of uploading logs
#[derive(serde::Serialize)]
pub struct UploadResult {
    /// URL to view the paste
    pub url: String,
    /// Short key for the paste
    pub key: String,
}

/// Collect all relevant log content for upload
fn collect_logs() -> Result<String, String> {
    let mut content = String::new();

    content.push_str("=== Armbian Imager Log Upload ===\n");
    content.push_str(&format!(
        "Timestamp: {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    ));
    content.push_str(&format!("App Version: {}\n", env!("CARGO_PKG_VERSION")));
    content.push_str(&format!(
        "OS: {} {}\n",
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    content.push('\n');

    if let Some(log_path) = get_current_log_path() {
        content.push_str("=== Current Session Log ===\n");
        match fs::read_to_string(&log_path) {
            Ok(log_content) => {
                content.push_str(&log_content);
            }
            Err(e) => {
                content.push_str(&format!("Error reading log file: {}\n", e));
            }
        }
    } else {
        content.push_str("No current log file available.\n");
    }

    // Also attach recent prior-session logs, useful after a crash.
    let log_dir = get_log_dir();
    if log_dir.exists() {
        let mut log_files: Vec<_> = fs::read_dir(&log_dir)
            .map_err(|e| format!("Failed to read log directory: {}", e))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "log"))
            .collect();

        // Newest first, so we attach the most recent prior sessions.
        log_files.sort_by(|a, b| {
            let a_time = a.metadata().and_then(|m| m.modified()).ok();
            let b_time = b.metadata().and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time)
        });

        // Include at most 2 previous logs.
        let current_log = get_current_log_path();
        let mut included = 0;
        for entry in log_files.iter() {
            let path = entry.path();

            // Current session log is already included above.
            if let Some(ref current) = current_log {
                if &path == current {
                    continue;
                }
            }

            if included >= 2 {
                break;
            }

            content.push_str(&format!(
                "\n=== Previous Log: {} ===\n",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));

            match fs::read_to_string(&path) {
                Ok(log_content) => {
                    // Cap each previous log at its last 500 lines.
                    let lines: Vec<&str> = log_content.lines().collect();
                    if lines.len() > 500 {
                        content.push_str(&format!(
                            "... (truncated, showing last 500 of {} lines)\n",
                            lines.len()
                        ));
                        for line in lines.iter().skip(lines.len() - 500) {
                            content.push_str(line);
                            content.push('\n');
                        }
                    } else {
                        content.push_str(&log_content);
                    }
                }
                Err(e) => {
                    content.push_str(&format!("Error reading log file: {}\n", e));
                }
            }

            included += 1;
        }
    }

    Ok(content)
}

/// Upload logs to paste.armbian.com, returning the paste URL and key.
#[tauri::command]
pub async fn upload_logs() -> Result<UploadResult, String> {
    log_info!("paste", "Starting log upload to paste.armbian.com");

    let content = collect_logs()?;

    if content.trim().is_empty() {
        return Err("No log content available to upload".to_string());
    }

    log_info!("paste", "Collected {} bytes of log data", content.len());

    let client = crate::utils::build_client(std::time::Duration::from_secs(30))?;

    let url = format!("{}{}", PASTE_URL, PASTE_ENDPOINT);
    let response = client
        .post(&url)
        .header("Content-Type", "text/plain")
        .body(content)
        .send()
        .await
        .map_err(|e| {
            log_error!("paste", "Failed to upload: {}", e);
            format!("Failed to upload logs: {}", e)
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        log_error!("paste", "Upload failed: HTTP {} - {}", status, body);
        return Err(format!("Upload failed: HTTP {}", status));
    }

    // The response body is the full paste URL as plain text.
    let paste_url = response
        .text()
        .await
        .map_err(|e| {
            log_error!("paste", "Failed to read response: {}", e);
            format!("Failed to read response: {}", e)
        })?
        .trim()
        .to_string();

    // The key is the last path segment of the URL.
    let key = paste_url
        .rsplit('/')
        .next()
        .unwrap_or(&paste_url)
        .to_string();

    log_info!("paste", "Successfully uploaded logs: {}", paste_url);

    Ok(UploadResult {
        url: paste_url,
        key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_logs() {
        let result = collect_logs();
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("Armbian Imager Log Upload"));
    }
}

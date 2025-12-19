//! System utilities commands
//!
//! Platform-specific system operations like opening URLs and locale detection.

use crate::log_info;
use sys_locale::get_locale;

const MODULE: &str = "commands::system";

/// Log a message from the frontend
#[tauri::command]
pub fn log_from_frontend(module: String, message: String) {
    log_info!(&format!("frontend::{}", module), "{}", message);
}

/// Get the system locale (e.g., "en-US", "it-IT", "de-DE")
/// Returns the language code for i18n initialization
#[tauri::command]
pub fn get_system_locale() -> String {
    let locale = get_locale().unwrap_or_else(|| "en-US".to_string());
    log_info!(MODULE, "Detected system locale: {}", locale);
    locale
}

/// Open a URL in the default browser
/// On Linux when running as root, uses runuser to open as the original user
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    log_info!(MODULE, "Opening URL: {}", url);

    #[cfg(target_os = "linux")]
    {
        open_url_linux(&url)
    }

    #[cfg(target_os = "macos")]
    {
        open_url_macos(&url)
    }

    #[cfg(target_os = "windows")]
    {
        open_url_windows(&url)
    }
}

#[cfg(target_os = "linux")]
fn open_url_linux(url: &str) -> Result<(), String> {
    use std::process::Command;

    // Check if running as root
    let euid = unsafe { libc::geteuid() };

    if euid == 0 {
        // Running as root - need to run xdg-open as the original user
        log_info!(MODULE, "Running as root, attempting to open URL as original user");

        // Try to get the original user from PKEXEC_UID or SUDO_UID
        let target_uid = std::env::var("PKEXEC_UID")
            .or_else(|_| std::env::var("SUDO_UID"))
            .ok()
            .and_then(|uid_str| uid_str.parse::<u32>().ok());

        if let Some(uid) = target_uid {
            // Get username from UID
            let username = get_username_from_uid(uid);

            if let Some(username) = username {
                log_info!(MODULE, "Opening URL as user: {} (uid: {})", username, uid);

                // Build environment variables to pass
                let mut env_args = vec!["env".to_string()];

                // Pass critical environment variables for D-Bus and display
                for var in &[
                    "DBUS_SESSION_BUS_ADDRESS",
                    "XDG_RUNTIME_DIR",
                    "DISPLAY",
                    "WAYLAND_DISPLAY",
                    "XAUTHORITY",
                ] {
                    if let Ok(value) = std::env::var(var) {
                        env_args.push(format!("{}={}", var, value));
                    }
                }

                env_args.push("xdg-open".to_string());
                env_args.push(url.to_string());

                // Try runuser first (better environment preservation)
                let result = Command::new("runuser")
                    .args(["-u", &username, "--"])
                    .args(&env_args)
                    .spawn();

                match result {
                    Ok(_) => {
                        log_info!(MODULE, "Successfully launched runuser xdg-open");
                        return Ok(());
                    }
                    Err(e) => {
                        log_info!(MODULE, "runuser failed: {}, trying pkexec", e);

                        // Fallback to pkexec --user
                        let result = Command::new("pkexec")
                            .args(["--user", &username, "xdg-open", url])
                            .spawn();

                        match result {
                            Ok(_) => {
                                log_info!(MODULE, "Successfully launched pkexec xdg-open");
                                return Ok(());
                            }
                            Err(e) => {
                                log_info!(MODULE, "pkexec also failed: {}", e);
                            }
                        }
                    }
                }
            }
        }

        // Fallback: try xdg-open directly (might not work but worth trying)
        log_info!(MODULE, "Could not determine original user, trying xdg-open directly");
    }

    // Not running as root, or fallback - use xdg-open directly
    Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map_err(|e| format!("Failed to open URL: {}", e))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn get_username_from_uid(uid: u32) -> Option<String> {
    use std::ffi::CStr;

    unsafe {
        let pw = libc::getpwuid(uid);
        if pw.is_null() {
            return None;
        }

        let name_ptr = (*pw).pw_name;
        if name_ptr.is_null() {
            return None;
        }

        CStr::from_ptr(name_ptr)
            .to_str()
            .ok()
            .map(|s| s.to_string())
    }
}

#[cfg(target_os = "macos")]
fn open_url_macos(url: &str) -> Result<(), String> {
    use std::process::Command;

    Command::new("open")
        .arg(url)
        .spawn()
        .map_err(|e| format!("Failed to open URL: {}", e))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn open_url_windows(url: &str) -> Result<(), String> {
    use std::process::Command;

    Command::new("cmd")
        .args(["/c", "start", "", url])
        .spawn()
        .map_err(|e| format!("Failed to open URL: {}", e))?;

    Ok(())
}

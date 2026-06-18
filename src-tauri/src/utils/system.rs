//! System utilities: CPU info, cache/data paths, and platform detection.

use std::path::PathBuf;

use crate::config;

/// Get the number of CPU cores available on the system
pub fn get_cpu_cores() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(2)
}

/// Recommended thread count for CPU-heavy work: half the cores, to avoid
/// saturating the system.
pub fn get_recommended_threads() -> usize {
    std::cmp::max(1, get_cpu_cores() / 2)
}

/// Application cache directory. On Linux under pkexec/sudo, prefers the original user's cache.
pub fn get_cache_dir(app_name: &str) -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        // When elevated, prefer the invoking user's ~/.cache over root's.
        let euid = unsafe { libc::geteuid() };
        if euid == 0 {
            if let Some(home) = get_original_user_home() {
                let cache_dir = PathBuf::from(home).join(".cache").join(app_name);
                let _ = std::fs::create_dir_all(&cache_dir);
                return cache_dir;
            }
        }
    }

    dirs::cache_dir()
        .or_else(dirs::data_local_dir)
        .or_else(|| std::env::temp_dir().parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(std::env::temp_dir)
        .join(app_name)
}

/// Root application cache directory.
pub fn app_cache_dir() -> PathBuf {
    get_cache_dir(config::app::NAME)
}

/// Directory holding downloaded OS images.
pub fn images_dir() -> PathBuf {
    app_cache_dir().join("images")
}

/// Directory holding cached board and vendor assets.
pub fn assets_dir() -> PathBuf {
    app_cache_dir().join("assets")
}

/// Directory holding decompressed custom images.
pub fn custom_decompress_dir() -> PathBuf {
    app_cache_dir().join("custom-decompress")
}

/// Directory holding temporary files for QDL operations.
pub fn qdl_temp_dir() -> PathBuf {
    app_cache_dir().join("qdl-temp")
}

/// Directory caching downloaded QDL firehose loaders, keyed by SoC family.
pub fn loaders_dir() -> PathBuf {
    app_cache_dir().join("loaders")
}

/// Directory holding session log files.
pub fn logs_dir() -> PathBuf {
    app_cache_dir().join("logs")
}

/// Get the original user's home directory when running as root via pkexec/sudo
#[cfg(target_os = "linux")]
fn get_original_user_home() -> Option<String> {
    use std::ffi::CStr;

    // PKEXEC_UID (set by pkexec) takes priority over SUDO_UID.
    let uid = std::env::var("PKEXEC_UID")
        .or_else(|_| std::env::var("SUDO_UID"))
        .ok()
        .and_then(|s| s.parse::<u32>().ok());

    if let Some(uid) = uid {
        unsafe {
            let pw = libc::getpwuid(uid);
            if !pw.is_null() {
                let home_ptr = (*pw).pw_dir;
                if !home_ptr.is_null() {
                    if let Ok(home) = CStr::from_ptr(home_ptr).to_str() {
                        return Some(home.to_string());
                    }
                }
            }
        }
    }

    // Fall back to resolving SUDO_USER by name.
    if let Ok(sudo_user) = std::env::var("SUDO_USER") {
        unsafe {
            let user_cstr = std::ffi::CString::new(sudo_user).ok()?;
            let pw = libc::getpwnam(user_cstr.as_ptr());
            if !pw.is_null() {
                let home_ptr = (*pw).pw_dir;
                if !home_ptr.is_null() {
                    if let Ok(home) = CStr::from_ptr(home_ptr).to_str() {
                        return Some(home.to_string());
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cpu_cores() {
        let cores = get_cpu_cores();
        assert!(cores >= 1);
    }

    #[test]
    fn test_get_recommended_threads() {
        let threads = get_recommended_threads();
        assert!(threads >= 1);
        assert!(threads <= get_cpu_cores());
    }

    #[test]
    fn test_get_cache_dir() {
        let cache = get_cache_dir("test-app");
        assert!(cache.to_string_lossy().contains("test-app"));
    }
}

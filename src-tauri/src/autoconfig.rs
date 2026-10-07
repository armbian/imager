// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Armbian first-boot autoconfig: render a preset (mirrors client-side AutoconfigConfig) and inject it.
//! [`inject_into_image`] writes it to `/root/.not_logged_in_yet` in the image's ext4 rootfs, consumed on first boot. See https://docs.armbian.com/User-Guide_Autoconfig/.

use std::path::{Path, PathBuf};

use armbian_write_conf::{write_file_into_bare_ext4_image, write_file_into_image, WriteConfError};
use serde::Deserialize;

use crate::config;
use crate::utils::{leftover_is_stale, sweep_dir, unique_suffix};
use crate::{log_error, log_info, log_warn};

/// Login shell choices offered for the first user.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserShell {
    Bash,
    Zsh,
}

impl UserShell {
    /// Value written to PRESET_USER_SHELL.
    fn as_str(&self) -> &'static str {
        match self {
            UserShell::Bash => "bash",
            UserShell::Zsh => "zsh",
        }
    }
}

/// First-boot autoconfig model. All fields optional; only set/non-empty fields
/// are emitted into the preset. Mirrors the TS `AutoconfigConfig` type.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoconfigConfig {
    pub apply_network: Option<bool>,
    pub ethernet_enabled: Option<bool>,
    pub wifi_enabled: Option<bool>,
    pub wifi_ssid: Option<String>,
    pub wifi_key: Option<String>,
    pub wifi_country_code: Option<String>,
    pub use_static_ip: Option<bool>,
    pub static_ip: Option<String>,
    pub static_mask: Option<String>,
    pub static_gateway: Option<String>,
    pub static_dns: Option<String>,

    pub locale: Option<String>,
    pub timezone: Option<String>,
    pub lang_based_on_location: Option<bool>,

    pub root_password: Option<String>,
    pub root_key_url: Option<String>,

    pub user_name: Option<String>,
    pub user_password: Option<String>,
    pub user_key_url: Option<String>,
    pub user_shell: Option<UserShell>,
    pub user_real_name: Option<String>,

    pub remote_config_url: Option<String>,
}

// Passwords, the Wi-Fi key and key/config URLs must never reach a log: only show which fields are set.
impl std::fmt::Debug for AutoconfigConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let set = |value: &Option<String>| value.as_deref().is_some_and(|v| !v.is_empty());
        f.debug_struct("AutoconfigConfig")
            .field("wifi_key_set", &set(&self.wifi_key))
            .field("root_password_set", &set(&self.root_password))
            .field("user_password_set", &set(&self.user_password))
            .field("root_key_url_set", &set(&self.root_key_url))
            .field("user_key_url_set", &set(&self.user_key_url))
            .field("remote_config_url_set", &set(&self.remote_config_url))
            .finish_non_exhaustive()
    }
}

/// Quote a value for a bash-sourced file: wrap in double quotes and escape the
/// chars that are special inside double quotes (backslash, quote, dollar, backtick).
fn shell_quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' | '"' | '$' | '`' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// Push `KEY="value"` if `value` is set and non-empty.
fn push_str(out: &mut String, key: &str, value: &Option<String>) {
    if let Some(v) = value {
        if !v.is_empty() {
            out.push_str(key);
            out.push('=');
            out.push_str(&shell_quote(v));
            out.push('\n');
        }
    }
}

/// Push `KEY="1"`/`KEY="0"` if the boolean is set.
fn push_bool(out: &mut String, key: &str, value: Option<bool>) {
    if let Some(v) = value {
        out.push_str(key);
        out.push_str(if v { "=\"1\"\n" } else { "=\"0\"\n" });
    }
}

/// Render the preset to a bash-sourced `KEY="value"` document, emitting only set/non-empty fields. Booleans become
/// "1"/"0", the language-from-location flag "y"/"n"; no PRESET_NET_* key emitted unless `apply_network` is true.
pub fn render_preset(config: &AutoconfigConfig) -> String {
    let mut out = String::new();

    // Network: gated entirely on apply_network being explicitly true.
    if config.apply_network == Some(true) {
        push_bool(&mut out, "PRESET_NET_CHANGE_DEFAULTS", config.apply_network);
        push_bool(
            &mut out,
            "PRESET_NET_ETHERNET_ENABLED",
            config.ethernet_enabled,
        );
        push_bool(&mut out, "PRESET_NET_WIFI_ENABLED", config.wifi_enabled);
        // Wi-Fi credentials only when Wi-Fi is enabled, so disabling it does not leave
        // stale SSID/key/country behind (matches the frontend preview).
        if config.wifi_enabled == Some(true) {
            push_str(&mut out, "PRESET_NET_WIFI_SSID", &config.wifi_ssid);
            push_str(&mut out, "PRESET_NET_WIFI_KEY", &config.wifi_key);
            push_str(
                &mut out,
                "PRESET_NET_WIFI_COUNTRYCODE",
                &config.wifi_country_code,
            );
        }
        push_bool(&mut out, "PRESET_NET_USE_STATIC", config.use_static_ip);
        // Static address keys only when static IP is enabled, so disabling it does not
        // leave stale values behind (matches the frontend preview).
        if config.use_static_ip == Some(true) {
            push_str(&mut out, "PRESET_NET_STATIC_IP", &config.static_ip);
            push_str(&mut out, "PRESET_NET_STATIC_MASK", &config.static_mask);
            push_str(
                &mut out,
                "PRESET_NET_STATIC_GATEWAY",
                &config.static_gateway,
            );
            push_str(&mut out, "PRESET_NET_STATIC_DNS", &config.static_dns);
        }
    }

    // Localization. Armbian applies locale/timezone only during first-user creation,
    // so emit them only when a full user is defined (matches the locked UI inputs).
    let is_set = |v: &Option<String>| v.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);
    let has_user = is_set(&config.user_name)
        && is_set(&config.user_password)
        && is_set(&config.user_real_name);
    if has_user {
        push_str(&mut out, "PRESET_LOCALE", &config.locale);
        push_str(&mut out, "PRESET_TIMEZONE", &config.timezone);
        if let Some(v) = config.lang_based_on_location {
            out.push_str("SET_LANG_BASED_ON_LOCATION");
            out.push_str(if v { "=\"y\"\n" } else { "=\"n\"\n" });
        }
    }

    // Root account.
    push_str(&mut out, "PRESET_ROOT_PASSWORD", &config.root_password);
    push_str(&mut out, "PRESET_ROOT_KEY", &config.root_key_url);

    // First user.
    push_str(&mut out, "PRESET_USER_NAME", &config.user_name);
    push_str(&mut out, "PRESET_USER_PASSWORD", &config.user_password);
    push_str(&mut out, "PRESET_USER_KEY", &config.user_key_url);
    if let Some(shell) = &config.user_shell {
        out.push_str("PRESET_USER_SHELL=");
        out.push_str(&shell_quote(shell.as_str()));
        out.push('\n');
    }
    push_str(&mut out, "PRESET_DEFAULT_REALNAME", &config.user_real_name);

    // Advanced.
    push_str(&mut out, "PRESET_CONFIGURATION", &config.remote_config_url);

    // First login runs unattended on the autologin console. Without an internet
    // connection it would otherwise stop at "Connect via wireless? [Y/n]", which on
    // a headless board nobody answers, so any preset declines it (wifi, if wanted,
    // comes from the PRESET_NET_* keys above).
    if !out.is_empty() {
        out.push_str("PRESET_CONNECT_WIRELESS=\"n\"\n");
    }

    out
}

/// Render the preset and write it into the image's ext4 rootfs; `image_path` must be a raw (decompressed) image
/// that will be mutated. Never logs secret values (password/wifi key).
pub fn inject_into_image(
    image_path: &Path,
    config: &AutoconfigConfig,
) -> Result<(), WriteConfError> {
    let preset = render_preset(config);
    log_info!(
        "autoconfig",
        "Injecting first-boot preset ({} bytes) into {}",
        preset.len(),
        image_path.display()
    );

    match write_file_into_image(
        image_path,
        config::autoconfig::PRESET_PATH,
        preset.as_bytes(),
    ) {
        Ok(report) => {
            log_info!(
                "autoconfig",
                "Preset written to {} ({} scheme, {} bytes, validated: {})",
                report.dest_path,
                report.scheme,
                report.bytes_written,
                report.validated
            );
            Ok(())
        }
        Err(e) => {
            log_error!("autoconfig", "Failed to inject preset: {}", e);
            Err(e)
        }
    }
}

/// Render the preset and write it into a BARE ext4 image (no partition table; e.g. Armbian QDL `disk-sdcard.img.root`);
/// `image_path` must be a flat ext4 filesystem that will be mutated. Never logs secret values (password/wifi key).
pub fn inject_into_bare_ext4_image(
    image_path: &Path,
    config: &AutoconfigConfig,
) -> Result<(), WriteConfError> {
    restrict_to_owner(image_path, config::autoconfig::FILE_MODE).map_err(WriteConfError::Io)?;
    let preset = render_preset(config);
    log_info!(
        "autoconfig",
        "Injecting first-boot preset ({} bytes) into bare ext4 image {}",
        preset.len(),
        image_path.display()
    );

    match write_file_into_bare_ext4_image(
        image_path,
        config::autoconfig::PRESET_PATH,
        preset.as_bytes(),
    ) {
        Ok(report) => {
            log_info!(
                "autoconfig",
                "Preset written to {} ({} scheme, {} bytes, validated: {})",
                report.dest_path,
                report.scheme,
                report.bytes_written,
                report.validated
            );
            Ok(())
        }
        Err(e) => {
            log_error!("autoconfig", "Failed to inject preset: {}", e);
            Err(e)
        }
    }
}

#[cfg(unix)]
fn restrict_to_owner(path: &Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

// Elsewhere the per-user cache directory's ACL applies.
#[cfg(not(unix))]
fn restrict_to_owner(_path: &Path, _mode: u32) -> std::io::Result<()> {
    Ok(())
}

/// A per-flash copy of an image with the preset injected; the file is removed on drop.
pub struct WorkingCopy {
    path: PathBuf,
}

impl WorkingCopy {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for WorkingCopy {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_file(&self.path) {
            log_warn!(
                "autoconfig",
                "Failed to remove autoconfig working copy {}: {}",
                self.path.display(),
                e
            );
        }
    }
}

fn no_ext4_rootfs(e: &WriteConfError) -> String {
    format!(
        "This image does not have a writable ext4 root filesystem, so the selected autoconfig profile cannot be applied: {}",
        e
    )
}

fn create_private_copy(source: &Path, temp_dir: &Path) -> Result<WorkingCopy, String> {
    // set_permissions follows links, so a linked dir would hand the chmod to its target.
    if std::fs::symlink_metadata(temp_dir).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("The autoconfig temp directory is a link; refusing to use it".to_string());
    }
    std::fs::create_dir_all(temp_dir)
        .map_err(|e| format!("Failed to create autoconfig temp directory: {}", e))?;
    restrict_to_owner(temp_dir, config::autoconfig::DIR_MODE)
        .map_err(|e| format!("Failed to protect autoconfig temp directory: {}", e))?;

    let stem = source.file_name().map_or_else(
        || config::autoconfig::COPY_FALLBACK_NAME.into(),
        |n| n.to_string_lossy(),
    );
    let copy = WorkingCopy {
        path: temp_dir.join(format!("{}.{}", unique_suffix(), stem)),
    };

    log_info!(
        "autoconfig",
        "Copying image for autoconfig injection: {} -> {}",
        source.display(),
        copy.path.display()
    );
    // fs::copy carries the source's permission bits over.
    std::fs::copy(source, &copy.path)
        .map_err(|e| format!("Failed to copy image for autoconfig: {}", e))?;
    restrict_to_owner(&copy.path, config::autoconfig::FILE_MODE)
        .map_err(|e| format!("Failed to protect the autoconfig working copy: {}", e))?;
    Ok(copy)
}

/// Copy `source` into `temp_dir` and inject the preset there.
pub fn prepare_working_copy(
    source: &Path,
    temp_dir: &Path,
    config: &AutoconfigConfig,
) -> Result<WorkingCopy, String> {
    let copy = create_private_copy(source, temp_dir)?;
    inject_into_image(&copy.path, config).map_err(|e| match e {
        WriteConfError::UnsupportedImage(_) | WriteConfError::NoExt4Rootfs(_) => no_ext4_rootfs(&e),
        other => format!("Failed to apply autoconfig profile: {}", other),
    })?;

    Ok(copy)
}

/// Remove the working copies a crashed or killed session left in `temp_dir`; subdirectories are kept.
pub fn sweep_stale_working_copies(temp_dir: &Path) -> usize {
    sweep_dir("autoconfig", temp_dir, |name, meta| {
        !meta.is_dir() && leftover_is_stale(name, meta.modified().ok())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_scratch_dir;

    fn empty() -> AutoconfigConfig {
        AutoconfigConfig {
            apply_network: None,
            ethernet_enabled: None,
            wifi_enabled: None,
            wifi_ssid: None,
            wifi_key: None,
            wifi_country_code: None,
            use_static_ip: None,
            static_ip: None,
            static_mask: None,
            static_gateway: None,
            static_dns: None,
            locale: None,
            timezone: None,
            lang_based_on_location: None,
            root_password: None,
            root_key_url: None,
            user_name: None,
            user_password: None,
            user_key_url: None,
            user_shell: None,
            user_real_name: None,
            remote_config_url: None,
        }
    }

    #[test]
    fn empty_config_renders_nothing() {
        assert_eq!(render_preset(&empty()), "");
    }

    #[test]
    fn network_keys_gated_on_apply_network() {
        let mut c = empty();
        c.wifi_ssid = Some("home".to_string());
        // apply_network unset: no PRESET_NET_* keys.
        assert!(!render_preset(&c).contains("PRESET_NET_WIFI_SSID"));

        c.apply_network = Some(true);
        // Wi-Fi credentials are only emitted when Wi-Fi is enabled.
        c.wifi_enabled = Some(true);
        let out = render_preset(&c);
        assert!(out.contains("PRESET_NET_CHANGE_DEFAULTS=\"1\"\n"));
        assert!(out.contains("PRESET_NET_WIFI_SSID=\"home\"\n"));
    }

    #[test]
    fn lang_flag_uses_y_n() {
        let mut c = empty();
        // Localization keys are only emitted once a full first user is defined.
        c.user_name = Some("u".to_string());
        c.user_password = Some("p".to_string());
        c.user_real_name = Some("User".to_string());
        c.lang_based_on_location = Some(true);
        assert!(render_preset(&c).contains("SET_LANG_BASED_ON_LOCATION=\"y\"\n"));
        c.lang_based_on_location = Some(false);
        assert!(render_preset(&c).contains("SET_LANG_BASED_ON_LOCATION=\"n\"\n"));
    }

    #[test]
    fn shell_special_chars_escaped() {
        let mut c = empty();
        c.user_password = Some("a\"b$c`d\\e".to_string());
        let out = render_preset(&c);
        assert!(out.contains("PRESET_USER_PASSWORD=\"a\\\"b\\$c\\`d\\\\e\"\n"));
    }

    #[test]
    fn user_shell_serializes() {
        let mut c = empty();
        c.user_shell = Some(UserShell::Zsh);
        assert!(render_preset(&c).contains("PRESET_USER_SHELL=\"zsh\"\n"));
    }

    #[test]
    fn empty_string_is_skipped() {
        let mut c = empty();
        c.locale = Some(String::new());
        assert_eq!(render_preset(&c), "");
    }

    #[test]
    fn any_preset_declines_wireless_prompt() {
        let mut c = empty();
        c.root_password = Some("secret".to_string());
        assert!(render_preset(&c).ends_with("PRESET_CONNECT_WIRELESS=\"n\"\n"));
    }

    #[test]
    fn working_copy_never_touches_the_source() {
        let dir = test_scratch_dir("wc-test");
        let temp_dir = dir.join("work");
        let source = dir.join("source.img");
        let original = vec![0x5au8; 64 * 1024];
        std::fs::write(&source, &original).unwrap();

        let mut config = empty();
        config.locale = Some("en_US.UTF-8".to_string());
        let result = prepare_working_copy(&source, &temp_dir, &config);

        assert!(result.is_err(), "a non-ext4 image must be refused");
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read_dir(&temp_dir).unwrap().count(), 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn working_copy_is_owner_only_before_the_preset_lands() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.img");
        std::fs::write(&source, b"image").unwrap();
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o644)).unwrap();
        let temp_dir = dir.path().join("work");

        let copy = create_private_copy(&source, &temp_dir).unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(copy.path()), config::autoconfig::FILE_MODE);
        assert_eq!(mode(&temp_dir), config::autoconfig::DIR_MODE);
        assert_eq!(mode(&source), 0o644, "the source keeps its own mode");
        drop(copy);
        assert_eq!(std::fs::read_dir(&temp_dir).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_temp_dir_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.img");
        std::fs::write(&source, b"image").unwrap();
        let elsewhere = dir.path().join("elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();
        let link = dir.path().join("work");
        std::os::unix::fs::symlink(&elsewhere, &link).unwrap();
        assert!(create_private_copy(&source, &link).is_err());
        assert_eq!(std::fs::read_dir(&elsewhere).unwrap().count(), 0);
    }

    #[test]
    fn the_startup_sweep_removes_only_stale_copies_in_its_dir() {
        let dir = tempfile::tempdir().unwrap();
        let temp_dir = dir.path().join("work");
        std::fs::create_dir_all(temp_dir.join("nested")).unwrap();
        let ours = temp_dir.join(format!("{}.1.img", std::process::id()));
        let junk = temp_dir.join("leftover.img");
        let fresh = temp_dir.join("fresh.img");
        let nested = temp_dir.join("nested").join("1.1.img");
        let outside = dir.path().join("1.1.img");
        for path in [&ours, &junk, &fresh, &nested, &outside] {
            std::fs::write(path, b"PRESET_USER_PASSWORD=x").unwrap();
        }
        filetime::set_file_mtime(&junk, filetime::FileTime::zero()).unwrap();
        #[cfg(unix)]
        let dead = {
            let dead = temp_dir.join(format!("{}.1.img", i32::MAX));
            std::fs::write(&dead, b"x").unwrap();
            dead
        };

        let removed = sweep_stale_working_copies(&temp_dir);
        assert!(ours.exists(), "a copy of this process is in use");
        assert!(!junk.exists());
        assert!(fresh.exists(), "no owner pid and still recent");
        assert!(
            nested.exists() && outside.exists(),
            "nothing outside the dir itself"
        );
        #[cfg(unix)]
        {
            assert!(!dead.exists(), "the process that made it is gone");
            assert_eq!(removed, 2);
        }
        #[cfg(not(unix))]
        assert_eq!(removed, 1);
    }

    #[test]
    fn debug_output_never_shows_secrets() {
        let mut config = empty();
        config.wifi_key = Some("SECRET-WIFI".to_string());
        config.root_password = Some("SECRET-ROOT".to_string());
        config.user_password = Some("SECRET-USER".to_string());
        config.root_key_url = Some("https://SECRET/keys".to_string());
        config.remote_config_url = Some("https://SECRET/conf".to_string());
        config.user_name = Some("SECRET-NAME".to_string());
        let shown = format!("{config:?}");
        assert!(!shown.contains("SECRET"), "{shown}");
        assert!(shown.contains("wifi_key_set: true"));
    }
}

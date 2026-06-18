//! Armbian first-boot autoconfig: render a preset (mirrors client-side AutoconfigConfig) and inject it.
//! [`inject_into_image`] writes it to `/root/.not_logged_in_yet` in the image's ext4 rootfs, consumed on first boot. See https://docs.armbian.com/User-Guide_Autoconfig/.

use std::path::Path;

use armbian_write_conf::{write_file_into_bare_ext4_image, write_file_into_image, WriteConfError};
use serde::Deserialize;

use crate::{log_error, log_info};

/// Destination of the first-boot preset file inside the rootfs.
const PRESET_DEST_PATH: &str = "/root/.not_logged_in_yet";

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
#[derive(Debug, Clone, Deserialize)]
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

    match write_file_into_image(image_path, PRESET_DEST_PATH, preset.as_bytes()) {
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
    let preset = render_preset(config);
    log_info!(
        "autoconfig",
        "Injecting first-boot preset ({} bytes) into bare ext4 image {}",
        preset.len(),
        image_path.display()
    );

    match write_file_into_bare_ext4_image(image_path, PRESET_DEST_PATH, preset.as_bytes()) {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}

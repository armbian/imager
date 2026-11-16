// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Armbian first-boot autoconfig: render a preset (mirrors client-side AutoconfigConfig) and inject it.
//! [`inject_into_image`] writes it to `/root/.not_logged_in_yet` in the image's ext4 rootfs, consumed on first boot,
//! together with any SSH keys fetched at flash time. See https://docs.armbian.com/User-Guide_Autoconfig/.

use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use armbian_write_conf::{
    read_bare_ext4_file_capped, read_rootfs_file_capped, rootfs_has_regular_file,
    write_entries_into_bare_ext4_image, write_entries_into_image, Entry, WriteConfError,
    WriteEntriesReport,
};
use serde::Deserialize;

use crate::config;
use crate::config::autoconfig as cfg;
use crate::flash::FlashState;
use crate::utils::{leftover_is_stale, sweep_dir, unique_suffix};
use crate::{log_error, log_info, log_warn};

/// A profile was requested for a custom image without Armbian's first-login script (twin in errorUtils.ts).
pub const TAG_NOT_ARMBIAN: &str = "[AUTOCONFIG_NOT_ARMBIAN]";
/// The profile's user name is not letters and digits starting with a letter, at most 32 (twin in errorUtils.ts).
pub const TAG_USER_NAME_INVALID: &str = "[AUTOCONFIG_USER_NAME_INVALID]";
/// The profile's user name already exists as a user or group in the image (twin in errorUtils.ts).
pub const TAG_USER_NAME_TAKEN: &str = "[AUTOCONFIG_USER_NAME_TAKEN]";
/// The image's /etc/passwd or /etc/group could not be read to check the user name (twin in errorUtils.ts).
pub const TAG_USERS_UNREADABLE: &str = "[AUTOCONFIG_USERS_UNREADABLE]";
/// A path the profile writes is a link or has the wrong type in the image (twin in errorUtils.ts).
pub const TAG_UNSAFE_TARGET: &str = "[AUTOCONFIG_UNSAFE_TARGET]";

const MODULE: &str = "autoconfig";

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

fn is_set(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|v| !v.is_empty())
}

// Passwords, the Wi-Fi key and key/config URLs must never reach a log: only show which fields are set.
impl std::fmt::Debug for AutoconfigConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutoconfigConfig")
            .field("wifi_key_set", &is_set(&self.wifi_key))
            .field("root_password_set", &is_set(&self.root_password))
            .field("user_password_set", &is_set(&self.user_password))
            .field("root_key_url_set", &is_set(&self.root_key_url))
            .field("user_key_url_set", &is_set(&self.user_key_url))
            .field("remote_config_url_set", &is_set(&self.remote_config_url))
            .finish_non_exhaustive()
    }
}

/// Locale and IANA time zone of the flashing computer, validated, else the Armbian defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostRegion {
    pub locale: String,
    pub timezone: String,
}

impl HostRegion {
    pub fn detect() -> Self {
        Self::from_raw(
            sys_locale::get_locale().as_deref(),
            iana_time_zone::get_timezone().ok().as_deref(),
        )
    }

    fn from_raw(locale: Option<&str>, timezone: Option<&str>) -> Self {
        HostRegion {
            locale: locale
                .and_then(host_locale)
                .unwrap_or_else(|| cfg::DEFAULT_LOCALE.to_string()),
            timezone: timezone
                .and_then(host_timezone)
                .unwrap_or_else(|| cfg::DEFAULT_TIMEZONE.to_string()),
        }
    }

    #[cfg(test)]
    fn defaults() -> Self {
        Self::from_raw(None, None)
    }
}

fn is_ascii_letters(s: &str, len: std::ops::RangeInclusive<usize>) -> bool {
    len.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphabetic())
}

/// `^[A-Za-z]{2,3}(_[A-Za-z]{2})?\.UTF-8$`
fn is_valid_locale(locale: &str) -> bool {
    let Some(body) = locale.strip_suffix(cfg::LOCALE_SUFFIX) else {
        return false;
    };
    match body.split_once('_') {
        Some((lang, region)) => is_ascii_letters(lang, 2..=3) && is_ascii_letters(region, 2..=2),
        None => is_ascii_letters(body, 2..=3),
    }
}

/// `^[A-Za-z0-9_+\-]+(/[A-Za-z0-9_+\-]+)*$`
fn is_valid_timezone(timezone: &str) -> bool {
    timezone.split('/').all(|part| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'+' | b'-'))
    })
}

/// A host locale such as `it-IT`, `zh-Hans-CN` or `de_DE.UTF-8@euro` as a glibc UTF-8 locale; None without a region.
fn host_locale(raw: &str) -> Option<String> {
    let base = raw.trim().split(['.', '@']).next()?;
    let mut parts = base.split(['-', '_']);
    let lang = parts.next()?.to_ascii_lowercase();
    let region = parts
        .find(|p| is_ascii_letters(p, 2..=2))?
        .to_ascii_uppercase();
    let locale = format!("{lang}_{region}{}", cfg::LOCALE_SUFFIX);
    is_valid_locale(&locale).then_some(locale)
}

fn host_timezone(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let zone = cfg::WINDOWS_TIMEZONES
        .iter()
        .find(|(windows, _)| *windows == raw)
        .map_or(raw, |(_, iana)| *iana);
    is_valid_timezone(zone).then(|| zone.to_string())
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

fn push_value(out: &mut String, key: &str, value: Option<&str>) {
    if let Some(v) = value.filter(|v| !v.is_empty()) {
        out.push_str(key);
        out.push('=');
        out.push_str(&shell_quote(v));
        out.push('\n');
    }
}

fn verbatim(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|v| !v.is_empty())
}

// Older profiles can carry stray spaces ("192.168.1.50 "), which netplan and the first-boot fetches reject.
fn trimmed(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|v| !v.is_empty())
}

// Verbatim: spaces can be meaningful in passwords, the Wi-Fi key and the SSID.
fn push_str(out: &mut String, key: &str, value: &Option<String>) {
    push_value(out, key, verbatim(value));
}

fn push_trimmed(out: &mut String, key: &str, value: &Option<String>) {
    push_value(out, key, trimmed(value));
}

/// Push `KEY="1"`/`KEY="0"` if the boolean is set.
fn push_bool(out: &mut String, key: &str, value: Option<bool>) {
    if let Some(v) = value {
        out.push_str(key);
        out.push_str(if v { "=\"1\"\n" } else { "=\"0\"\n" });
    }
}

/// Twin of presetIsEmpty in src/config/autoconfig.ts: a field counted here must count there.
fn has_content(c: &AutoconfigConfig) -> bool {
    c.apply_network == Some(true)
        || trimmed(&c.locale).is_some()
        || trimmed(&c.timezone).is_some()
        || c.lang_based_on_location.is_some()
        || verbatim(&c.root_password).is_some()
        || trimmed(&c.root_key_url).is_some()
        || trimmed(&c.user_name).is_some()
        || verbatim(&c.user_password).is_some()
        || trimmed(&c.user_key_url).is_some()
        || c.user_shell.is_some()
        || trimmed(&c.user_real_name).is_some()
        || trimmed(&c.remote_config_url).is_some()
}

/// Only addresses, comma separated: 26.8.x turns "8.8.8.8, 1.1.1.1" into an empty netplan entry and stops.
fn normalize_dns(raw: &str) -> Option<String> {
    let list: Vec<&str> = raw
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|token| token.parse::<IpAddr>().is_ok())
        .collect();
    (!list.is_empty()).then(|| list.join(","))
}

/// The preset user name, trimmed and lowercased; refused unless it is letters and digits starting with a letter.
pub fn preset_user_name(config: &AutoconfigConfig) -> Result<Option<String>, String> {
    let Some(raw) = trimmed(&config.user_name) else {
        return Ok(None);
    };
    let name = raw.to_ascii_lowercase();
    let valid = name.len() <= cfg::USER_NAME_MAX_CHARS
        && name.bytes().next().is_some_and(|b| b.is_ascii_alphabetic())
        && name.bytes().all(|b| b.is_ascii_alphanumeric());
    if !valid {
        return Err(format!(
            "{TAG_USER_NAME_INVALID} The user name must start with a letter, use only letters and digits, and be at most {} characters",
            cfg::USER_NAME_MAX_CHARS
        ));
    }
    Ok(Some(name))
}

/// The typed real name without ':' and control characters, else the user name with its first letter uppercased.
fn real_name(typed: &Option<String>, user_name: &str) -> String {
    let cleaned: String = typed
        .as_deref()
        .unwrap_or_default()
        .chars()
        .filter(|c| *c != ':' && !c.is_control())
        .collect();
    let cleaned = cleaned.trim();
    if !cleaned.is_empty() {
        return cleaned.to_string();
    }
    let mut chars = user_name.chars();
    chars
        .next()
        .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
        .unwrap_or_default()
}

fn warn_on_fragile_wifi(config: &AutoconfigConfig) {
    let fragile = |v: &str| v.contains(['"', '\\']);
    if config.wifi_ssid.as_deref().is_some_and(fragile) {
        log_warn!(MODULE, "Field wifi_ssid holds a quote or backslash");
    }
    if let Some(key) = config.wifi_key.as_deref() {
        if fragile(key) || key.starts_with(char::is_whitespace) {
            log_warn!(
                MODULE,
                "Field wifi_key holds a quote, a backslash or leading whitespace"
            );
        }
    }
}

/// Render the preset as bash-sourced `KEY="value"` lines (empty when the profile sets nothing), defaulting shell and region.
pub fn render_preset(
    config: &AutoconfigConfig,
    user_key_file: bool,
    host: &HostRegion,
) -> Result<String, String> {
    let user_name = preset_user_name(config)?;
    let mut out = String::new();
    if !has_content(config) {
        return Ok(out);
    }

    if config.apply_network == Some(true) {
        push_bool(&mut out, "PRESET_NET_CHANGE_DEFAULTS", config.apply_network);
        push_bool(
            &mut out,
            "PRESET_NET_ETHERNET_ENABLED",
            config.ethernet_enabled,
        );
        push_bool(&mut out, "PRESET_NET_WIFI_ENABLED", config.wifi_enabled);
        // Gated so disabling Wi-Fi leaves no stale SSID/key/country behind.
        if config.wifi_enabled == Some(true) {
            warn_on_fragile_wifi(config);
            push_str(&mut out, "PRESET_NET_WIFI_SSID", &config.wifi_ssid);
            push_str(&mut out, "PRESET_NET_WIFI_KEY", &config.wifi_key);
            push_trimmed(
                &mut out,
                "PRESET_NET_WIFI_COUNTRYCODE",
                &config.wifi_country_code,
            );
        }
        push_bool(&mut out, "PRESET_NET_USE_STATIC", config.use_static_ip);
        // Gated so disabling static IP leaves no stale address behind.
        if config.use_static_ip == Some(true) {
            push_trimmed(&mut out, "PRESET_NET_STATIC_IP", &config.static_ip);
            push_trimmed(&mut out, "PRESET_NET_STATIC_MASK", &config.static_mask);
            push_trimmed(
                &mut out,
                "PRESET_NET_STATIC_GATEWAY",
                &config.static_gateway,
            );
            let dns = config.static_dns.as_deref().and_then(normalize_dns);
            push_value(&mut out, "PRESET_NET_STATIC_DNS", dns.as_deref());
        }
    }

    let from_host = config.lang_based_on_location == Some(true);
    let (default_locale, default_timezone) = if from_host {
        (host.locale.as_str(), host.timezone.as_str())
    } else {
        (cfg::DEFAULT_LOCALE, cfg::DEFAULT_TIMEZONE)
    };
    push_value(
        &mut out,
        "PRESET_LOCALE",
        Some(trimmed(&config.locale).unwrap_or(default_locale)),
    );
    push_value(
        &mut out,
        "PRESET_TIMEZONE",
        Some(trimmed(&config.timezone).unwrap_or(default_timezone)),
    );
    // The locale is always written, so the "language from location?" question has nothing left to decide.
    out.push_str("SET_LANG_BASED_ON_LOCATION=\"n\"\n");

    // With no root password, root gets the user's, as the Armbian first-run wizard does.
    let root_password = verbatim(&config.root_password).or_else(|| {
        user_name
            .as_ref()
            .and_then(|_| verbatim(&config.user_password))
    });
    push_value(&mut out, "PRESET_ROOT_PASSWORD", root_password);

    push_value(&mut out, "PRESET_USER_NAME", user_name.as_deref());
    push_str(&mut out, "PRESET_USER_PASSWORD", &config.user_password);
    if user_key_file && user_name.is_some() {
        let url = format!("{}{}", cfg::USER_KEYS_URL_SCHEME, cfg::USER_KEYS_FILE);
        push_value(&mut out, "PRESET_USER_KEY", Some(&url));
    }
    let shell = config.user_shell.as_ref().unwrap_or(&UserShell::Bash);
    push_value(&mut out, "PRESET_USER_SHELL", Some(shell.as_str()));
    if let Some(name) = &user_name {
        let real = real_name(&config.user_real_name, name);
        push_value(&mut out, "PRESET_DEFAULT_REALNAME", Some(&real));
    }

    push_trimmed(&mut out, "PRESET_CONFIGURATION", &config.remote_config_url);

    // Decline "Connect via wireless? [Y/n]": unattended first login would hang on it offline (#193).
    out.push_str("PRESET_CONNECT_WIRELESS=\"n\"\n");

    Ok(out)
}

/// A profile with its SSH keys fetched and the host region read, ready to render and inject without network access.
pub struct ResolvedProfile {
    config: AutoconfigConfig,
    user_name: Option<String>,
    root_keys: Option<String>,
    user_keys: Option<String>,
    host: HostRegion,
}

impl std::fmt::Debug for ResolvedProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedProfile")
            .field("config", &self.config)
            .field("user_name_set", &self.user_name.is_some())
            .field("root_keys_set", &self.root_keys.is_some())
            .field("user_keys_set", &self.user_keys.is_some())
            .finish_non_exhaustive()
    }
}

impl ResolvedProfile {
    #[cfg(test)]
    fn new(
        config: AutoconfigConfig,
        root_keys: Option<String>,
        user_keys: Option<String>,
        host: HostRegion,
    ) -> Result<Self, String> {
        let user_name = preset_user_name(&config)?;
        let user_keys = user_keys.filter(|_| user_name.is_some());
        Ok(ResolvedProfile {
            config,
            user_name,
            root_keys,
            user_keys,
            host,
        })
    }

    pub fn user_name(&self) -> Option<&str> {
        self.user_name.as_deref()
    }

    fn render(&self) -> Result<String, String> {
        render_preset(&self.config, self.user_keys.is_some(), &self.host)
    }
}

/// Clears the published prep stage when the step it covers ends, errors and panics included.
struct PrepStageGuard<'a>(&'a FlashState);

impl Drop for PrepStageGuard<'_> {
    fn drop(&mut self) {
        self.0.set_prep_stage(None);
    }
}

async fn fetch_keys(account: &str, url: &str, state: &FlashState) -> Result<String, PrepError> {
    let kind = crate::ssh_keys::url_kind(url);
    let result = crate::ssh_keys::fetch_authorized_keys(url).await;
    state
        .ensure_not_cancelled()
        .map_err(|_| PrepError::Cancelled)?;
    match result {
        Ok((keys, count)) => {
            log_info!(
                MODULE,
                "SSH keys for {account}: source={kind}, {count} keys"
            );
            Ok(keys)
        }
        Err(code) => {
            log_warn!(MODULE, "SSH keys for {account}: source={kind}, {code}");
            Err(PrepError::Failed(format!(
                "{code} Could not get the SSH keys for the {account} account"
            )))
        }
    }
}

/// Check the user name and fetch the profile's SSH keys, publishing the fetch stage and stopping at a cancel.
pub async fn resolve_profile(
    config: AutoconfigConfig,
    state: &FlashState,
) -> Result<ResolvedProfile, PrepError> {
    state
        .ensure_not_cancelled()
        .map_err(|_| PrepError::Cancelled)?;
    let user_name = preset_user_name(&config).map_err(PrepError::Failed)?;
    let root_url = trimmed(&config.root_key_url).map(str::to_string);
    let user_url = user_name
        .as_ref()
        .and(trimmed(&config.user_key_url))
        .map(str::to_string);

    let _stage = PrepStageGuard(state);
    if root_url.is_some() || user_url.is_some() {
        state.set_prep_stage(Some(config::flash::PREP_STAGE_FETCHING_KEYS));
    }
    let root_keys = match &root_url {
        Some(url) => Some(fetch_keys(cfg::KEYS_ACCOUNT_ROOT, url, state).await?),
        None => None,
    };
    let user_keys = match &user_url {
        Some(url) => Some(fetch_keys(cfg::KEYS_ACCOUNT_USER, url, state).await?),
        None => None,
    };
    log_info!(
        MODULE,
        "Profile resolved: user set {}, root keys {}, user keys {}",
        user_name.is_some(),
        root_keys.is_some(),
        user_keys.is_some()
    );
    Ok(ResolvedProfile {
        config,
        user_name,
        root_keys,
        user_keys,
        host: HostRegion::detect(),
    })
}

/// User and group names of a passwd- or group-format file.
fn account_names(content: &[u8]) -> impl Iterator<Item = &[u8]> {
    content
        .split(|b| *b == b'\n')
        .filter_map(|line| line.split(|b| *b == b':').next())
        .filter(|name| !name.is_empty())
}

/// Refuse `user_name` when the image's passwd or group already has it; an unreadable file refuses too.
fn check_user_name_free(
    user_name: &str,
    read: impl Fn(&str) -> Result<Vec<u8>, WriteConfError>,
) -> Result<(), String> {
    for path in [cfg::PASSWD_PATH, cfg::GROUP_PATH] {
        let content = read(path).map_err(|e| {
            log_warn!(MODULE, "Could not read {} from the image: {}", path, e);
            format!("{TAG_USERS_UNREADABLE} Could not read {path} from the image to check the user name")
        })?;
        if account_names(&content).any(|name| name == user_name.as_bytes()) {
            return Err(format!(
                "{TAG_USER_NAME_TAKEN} The user name already exists in this image; choose another one"
            ));
        }
    }
    log_info!(MODULE, "User name is free in the image");
    Ok(())
}

/// [`check_user_name_free`] against a raw disk image. Read-only.
pub fn check_user_name_free_in_image(image_path: &Path, user_name: &str) -> Result<(), String> {
    check_user_name_free(user_name, |path| {
        read_rootfs_file_capped(image_path, path, cfg::USERS_FILE_MAX_BYTES)
    })
}

/// [`check_user_name_free`] against a bare ext4 image. Read-only.
pub fn check_user_name_free_in_bare_ext4(image_path: &Path, user_name: &str) -> Result<(), String> {
    check_user_name_free(user_name, |path| {
        read_bare_ext4_file_capped(image_path, path, cfg::USERS_FILE_MAX_BYTES)
    })
}

/// The SSH directory and key files (when fetched) followed by the preset, all root-owned and root-only.
fn image_entries<'a>(profile: &'a ResolvedProfile, preset: &'a str) -> Vec<Entry<'a>> {
    let root = cfg::IMAGE_ROOT_ID;
    let mut entries = Vec::new();
    if let Some(keys) = &profile.root_keys {
        entries.push(Entry::dir(
            cfg::ROOT_SSH_DIR,
            cfg::IMAGE_DIR_MODE,
            root,
            root,
        ));
        entries.push(Entry::file(
            cfg::ROOT_AUTHORIZED_KEYS,
            keys.as_bytes(),
            cfg::IMAGE_FILE_MODE,
            root,
            root,
        ));
    }
    if let Some(keys) = &profile.user_keys {
        entries.push(Entry::file(
            cfg::USER_KEYS_FILE,
            keys.as_bytes(),
            cfg::IMAGE_FILE_MODE,
            root,
            root,
        ));
    }
    entries.push(Entry::file(
        cfg::PRESET_PATH,
        preset.as_bytes(),
        cfg::IMAGE_FILE_MODE,
        root,
        root,
    ));
    entries
}

fn no_ext4_rootfs(e: &WriteConfError) -> String {
    format!(
        "This image does not have a writable ext4 root filesystem, so the selected autoconfig profile cannot be applied: {}",
        e
    )
}

fn injection_error(e: WriteConfError) -> String {
    match e {
        WriteConfError::UnsupportedImage(_) | WriteConfError::NoExt4Rootfs(_) => no_ext4_rootfs(&e),
        WriteConfError::UnsafeTarget(_) => format!(
            "{TAG_UNSAFE_TARGET} The image has a link or an unexpected file where the profile writes: {e}"
        ),
        other => format!("Failed to apply autoconfig profile: {}", other),
    }
}

fn inject_with(
    image_path: &Path,
    profile: &ResolvedProfile,
    write: impl FnOnce(&Path, &[Entry]) -> Result<WriteEntriesReport, WriteConfError>,
) -> Result<(), String> {
    let preset = profile.render()?;
    let entries = image_entries(profile, &preset);
    log_info!(
        MODULE,
        "Injecting first-boot preset ({} bytes), root keys {}, user keys {} into {}",
        preset.len(),
        profile.root_keys.is_some(),
        profile.user_keys.is_some(),
        image_path.display()
    );
    match write(image_path, &entries) {
        Ok(report) => {
            if report
                .replaced
                .iter()
                .any(|p| p == cfg::ROOT_AUTHORIZED_KEYS)
            {
                log_info!(MODULE, "Replaced the image's existing root authorized_keys");
            }
            log_info!(
                MODULE,
                "Preset written ({} scheme, {} entries, validated: {})",
                report.scheme,
                report.entries,
                report.validated
            );
            Ok(())
        }
        Err(e) => {
            log_error!(MODULE, "Failed to inject preset: {}", e);
            Err(injection_error(e))
        }
    }
}

/// Write the preset and fetched keys into the image's ext4 rootfs; `image_path` must be a raw (decompressed) image
/// that will be mutated. Never logs secret values.
pub fn inject_into_image(image_path: &Path, profile: &ResolvedProfile) -> Result<(), String> {
    inject_with(image_path, profile, write_entries_into_image)
}

/// [`inject_into_image`] for a BARE ext4 image (no partition table; e.g. Armbian QDL `disk-sdcard.img.root`).
pub fn inject_into_bare_ext4_image(
    image_path: &Path,
    profile: &ResolvedProfile,
) -> Result<(), String> {
    restrict_to_owner(image_path, config::autoconfig::FILE_MODE)
        .map_err(|e| format!("Failed to protect the rootfs image: {e}"))?;
    inject_with(image_path, profile, write_entries_into_bare_ext4_image)
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
                MODULE,
                "Failed to remove autoconfig working copy {}: {}",
                self.path.display(),
                e
            );
        }
    }
}

/// Whether the image's rootfs carries Armbian's first-login script. Opens `image_path` read-only.
pub fn has_firstlogin_marker(image_path: &Path) -> Result<bool, WriteConfError> {
    let meta = std::fs::metadata(image_path)?;
    if !meta.is_file() {
        return Err(WriteConfError::UnsupportedImage(
            "not a regular file".to_string(),
        ));
    }
    rootfs_has_regular_file(image_path, config::autoconfig::FIRSTLOGIN_MARKER)
}

fn marker_verdict(probe: Result<bool, WriteConfError>) -> Result<(), String> {
    match probe {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!(
            "{TAG_NOT_ARMBIAN} This image has no Armbian first-login setup, so the selected autoconfig profile cannot be applied"
        )),
        Err(e @ (WriteConfError::UnsupportedImage(_) | WriteConfError::NoExt4Rootfs(_))) => {
            Err(no_ext4_rootfs(&e))
        }
        Err(e) => Err(format!("Failed to read the image for autoconfig: {}", e)),
    }
}

/// Refuse a profile for a custom image whose rootfs has no first-login script, before any copy or write.
pub fn require_firstlogin_marker(image_path: &Path) -> Result<(), String> {
    let probe = has_firstlogin_marker(image_path);
    match &probe {
        Ok(found) => log_info!(
            MODULE,
            "First-login marker in {}: {}",
            image_path.display(),
            found
        ),
        Err(e) => log_warn!(
            MODULE,
            "First-login marker check failed for {}: {}",
            image_path.display(),
            e
        ),
    }
    marker_verdict(probe)
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
        MODULE,
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

/// Why a flash-time preparation produced no copy.
#[derive(Debug, PartialEq, Eq)]
pub enum PrepError {
    Cancelled,
    Failed(String),
}

impl PrepError {
    /// The error a block-device flash returns for this outcome.
    pub fn into_flash_error(self) -> String {
        match self {
            PrepError::Cancelled => crate::flash::cancelled_err(),
            PrepError::Failed(message) => message,
        }
    }
}

fn cancel_check(state: &FlashState) -> Result<(), PrepError> {
    state
        .ensure_not_cancelled()
        .map_err(|_| PrepError::Cancelled)
}

/// Read-only refusals on the source before any network access or copy: the first-login marker when
/// `require_marker`, then a user name the image already has.
fn check_source(
    source: &Path,
    require_marker: bool,
    user_name: Option<&str>,
    state: &FlashState,
) -> Result<(), PrepError> {
    cancel_check(state)?;
    if require_marker {
        require_firstlogin_marker(source).map_err(PrepError::Failed)?;
        cancel_check(state)?;
    }
    if let Some(user_name) = user_name {
        check_user_name_free_in_image(source, user_name).map_err(PrepError::Failed)?;
        cancel_check(state)?;
    }
    Ok(())
}

fn copy_and_inject(
    source: &Path,
    temp_dir: &Path,
    profile: &ResolvedProfile,
    state: &FlashState,
) -> Result<WorkingCopy, PrepError> {
    let _stage = PrepStageGuard(state);
    cancel_check(state)?;
    state.set_prep_stage(Some(config::flash::PREP_STAGE_COPYING));
    let copy = create_private_copy(source, temp_dir).map_err(PrepError::Failed)?;
    cancel_check(state)?;
    state.set_prep_stage(Some(config::flash::PREP_STAGE_APPLYING_PROFILE));
    inject_into_image(copy.path(), profile).map_err(PrepError::Failed)?;
    cancel_check(state)?;
    Ok(copy)
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, PrepError> + Send + 'static,
) -> Result<T, PrepError> {
    tokio::task::spawn_blocking(work).await.unwrap_or_else(|e| {
        let why = if e.is_panic() {
            "panicked"
        } else {
            "was aborted"
        };
        Err(PrepError::Failed(format!("Autoconfig preparation {why}")))
    })
}

async fn prepare_steps(
    source: PathBuf,
    temp_dir: PathBuf,
    config: AutoconfigConfig,
    require_marker: bool,
    state: Arc<FlashState>,
) -> Result<WorkingCopy, PrepError> {
    let user_name = preset_user_name(&config).map_err(PrepError::Failed)?;
    let (checked, check_state) = (source.clone(), state.clone());
    blocking(move || check_source(&checked, require_marker, user_name.as_deref(), &check_state))
        .await?;
    let profile = resolve_profile(config, &state).await?;
    blocking(move || copy_and_inject(&source, &temp_dir, &profile, &state)).await
}

/// Check the source read-only (marker, user name), then fetch the profile's keys, then on a blocking thread copy it
/// into `temp_dir` and inject there, publishing `prep_stage` and stopping at a cancel between steps.
pub async fn prepare_flash_copy(
    source: PathBuf,
    temp_dir: PathBuf,
    config: AutoconfigConfig,
    require_marker: bool,
    state: Arc<FlashState>,
) -> Result<WorkingCopy, PrepError> {
    log_info!(MODULE, "Preparing autoconfig copy of {}", source.display());
    let started = Instant::now();
    let result = prepare_steps(source, temp_dir, config, require_marker, state).await;
    let secs = started.elapsed().as_secs_f64();
    match &result {
        Ok(_) => log_info!(MODULE, "Autoconfig prep done in {:.1} s", secs),
        Err(PrepError::Cancelled) => {
            log_info!(MODULE, "Flash cancelled during autoconfig prep")
        }
        Err(PrepError::Failed(e)) => {
            log_error!(MODULE, "Autoconfig prep failed after {:.1} s: {}", secs, e)
        }
    }
    result
}

/// Remove the working copies a crashed or killed session left in `temp_dir`; subdirectories are kept.
pub fn sweep_stale_working_copies(temp_dir: &Path) -> usize {
    sweep_dir(MODULE, temp_dir, |name, meta| {
        !meta.is_dir() && leftover_is_stale(name, meta.modified().ok())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_scratch_dir;

    /// The two blocking steps of a prep with the keys already in hand.
    fn prepare_tracked(
        source: &Path,
        temp_dir: &Path,
        profile: &ResolvedProfile,
        require_marker: bool,
        state: &FlashState,
    ) -> Result<WorkingCopy, PrepError> {
        check_source(source, require_marker, profile.user_name(), state)?;
        copy_and_inject(source, temp_dir, profile, state)
    }

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

    fn host() -> HostRegion {
        HostRegion {
            locale: "it_IT.UTF-8".to_string(),
            timezone: "Europe/Rome".to_string(),
        }
    }

    fn render(c: &AutoconfigConfig) -> String {
        render_preset(c, false, &host()).unwrap()
    }

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    /// Everything a non-empty preset carries after the profile's own keys, with no user and no region set.
    const DEFAULTS_TAIL: &str = "PRESET_USER_SHELL=\"bash\"\nPRESET_CONNECT_WIRELESS=\"n\"\n";
    const DEFAULT_REGION: &str = "PRESET_LOCALE=\"en_US.UTF-8\"\nPRESET_TIMEZONE=\"Etc/UTC\"\nSET_LANG_BASED_ON_LOCATION=\"n\"\n";

    #[test]
    fn empty_config_renders_nothing() {
        assert_eq!(render(&empty()), "");
        let mut blank = empty();
        blank.locale = Some(String::new());
        blank.root_key_url = s("  ");
        blank.user_name = s(" ");
        blank.apply_network = Some(false);
        assert_eq!(
            render(&blank),
            "",
            "presetIsEmpty twin: blanks count as unset"
        );
    }

    #[test]
    fn every_field_presetisempty_counts_makes_the_preset_non_empty() {
        type Set = fn(&mut AutoconfigConfig);
        let setters: [(&str, Set); 12] = [
            ("applyNetwork", |c| c.apply_network = Some(true)),
            ("locale", |c| c.locale = s("de_DE.UTF-8")),
            ("timezone", |c| c.timezone = s("Europe/Berlin")),
            ("langBasedOnLocation", |c| {
                c.lang_based_on_location = Some(false)
            }),
            ("rootPassword", |c| c.root_password = s(" ")),
            ("rootKeyUrl", |c| {
                c.root_key_url = s("https://github.com/a.keys")
            }),
            ("userName", |c| c.user_name = s("pi")),
            ("userPassword", |c| c.user_password = s("pw")),
            ("userKeyUrl", |c| {
                c.user_key_url = s("https://github.com/a.keys")
            }),
            ("userShell", |c| c.user_shell = Some(UserShell::Zsh)),
            ("userRealName", |c| c.user_real_name = s("Pi")),
            ("remoteConfigUrl", |c| {
                c.remote_config_url = s("https://e.com/c")
            }),
        ];
        for (field, set) in setters {
            let mut c = empty();
            set(&mut c);
            let out = render(&c);
            assert!(
                out.ends_with("PRESET_CONNECT_WIRELESS=\"n\"\n"),
                "{field}: {out}"
            );
        }
    }

    #[test]
    fn a_network_only_preset_gets_shell_and_region_defaults() {
        let mut c = empty();
        c.apply_network = Some(true);
        c.ethernet_enabled = Some(true);
        assert_eq!(
            render(&c),
            format!(
                "PRESET_NET_CHANGE_DEFAULTS=\"1\"\nPRESET_NET_ETHERNET_ENABLED=\"1\"\n{DEFAULT_REGION}{DEFAULTS_TAIL}"
            )
        );
    }

    #[test]
    fn network_keys_gated_on_apply_network() {
        let mut c = empty();
        c.wifi_ssid = s("home");
        assert!(!render(&c).contains("PRESET_NET_WIFI_SSID"));

        c.apply_network = Some(true);
        c.wifi_enabled = Some(true);
        let out = render(&c);
        assert!(out.contains("PRESET_NET_CHANGE_DEFAULTS=\"1\"\n"));
        assert!(out.contains("PRESET_NET_WIFI_SSID=\"home\"\n"));
    }

    #[test]
    fn dns_becomes_a_plain_address_list() {
        for (typed, written) in [
            ("8.8.8.8, 1.1.1.1", Some("8.8.8.8,1.1.1.1")),
            ("8.8.8.8,1.1.1.1", Some("8.8.8.8,1.1.1.1")),
            (" 8.8.8.8  1.1.1.1 ", Some("8.8.8.8,1.1.1.1")),
            ("8.8.8.8,,;1.1.1.1", Some("8.8.8.8")),
            (
                "2001:4860:4860::8888, 9.9.9.9",
                Some("2001:4860:4860::8888,9.9.9.9"),
            ),
            ("dns.google, 1.1.1.1, 300.1.1.1", Some("1.1.1.1")),
            ("junk, more junk", None),
            ("", None),
        ] {
            assert_eq!(normalize_dns(typed).as_deref(), written, "{typed:?}");
        }
        let mut c = empty();
        c.apply_network = Some(true);
        c.use_static_ip = Some(true);
        c.static_dns = s("nope");
        assert!(!render(&c).contains("PRESET_NET_STATIC_DNS"));
        c.static_dns = s("8.8.8.8, 1.1.1.1");
        assert!(render(&c).contains("PRESET_NET_STATIC_DNS=\"8.8.8.8,1.1.1.1\"\n"));
    }

    #[test]
    fn user_names_are_lowercased_or_refused() {
        let named = |n: &str| {
            let mut c = empty();
            c.user_name = s(n);
            preset_user_name(&c)
        };
        assert_eq!(named(" John ").unwrap().as_deref(), Some("john"));
        assert_eq!(named("pi4").unwrap().as_deref(), Some("pi4"));
        assert_eq!(named(&"a".repeat(32)).unwrap().map(|n| n.len()), Some(32));
        assert_eq!(named("   ").unwrap(), None);
        for bad in [
            "john.doe",
            "John Smith",
            "john_doe",
            "jean-luc",
            "4you",
            "jöhn",
            &"a".repeat(33),
        ] {
            let err = named(bad).unwrap_err();
            assert!(err.starts_with(TAG_USER_NAME_INVALID), "{bad}: {err}");
            assert!(!err.contains(bad), "the error never echoes the name");
        }
        let mut c = empty();
        c.user_name = s("John Smith");
        assert!(render_preset(&c, false, &host()).is_err());
    }

    #[test]
    fn a_user_gets_a_default_real_name_and_root_gets_the_user_password() {
        let mut c = empty();
        c.user_name = s("JOHN");
        c.user_password = s(" user pw ");
        let out = render(&c);
        assert!(out.contains("PRESET_USER_NAME=\"john\"\n"), "{out}");
        assert!(out.contains("PRESET_DEFAULT_REALNAME=\"John\"\n"), "{out}");
        assert!(
            out.contains("PRESET_ROOT_PASSWORD=\" user pw \"\n"),
            "{out}"
        );
        assert!(out.contains("PRESET_USER_SHELL=\"bash\"\n"), "{out}");

        c.root_password = s("root pw");
        c.user_real_name = s(" Jo:hn\u{7} Smith ");
        let out = render(&c);
        assert!(out.contains("PRESET_ROOT_PASSWORD=\"root pw\"\n"), "{out}");
        assert!(
            out.contains("PRESET_DEFAULT_REALNAME=\"John Smith\"\n"),
            "{out}"
        );

        c.user_real_name = s(" : ");
        assert!(render(&c).contains("PRESET_DEFAULT_REALNAME=\"John\"\n"));
    }

    #[test]
    fn no_user_means_no_real_name_and_no_borrowed_root_password() {
        let mut c = empty();
        c.user_password = s("pw");
        c.user_real_name = s("Someone");
        let out = render(&c);
        assert!(!out.contains("PRESET_ROOT_PASSWORD"), "{out}");
        assert!(!out.contains("PRESET_DEFAULT_REALNAME"), "{out}");
    }

    #[test]
    fn keys_never_reach_the_preset_as_urls() {
        let mut c = empty();
        c.root_key_url = s("https://github.com/r.keys");
        c.user_name = s("pi");
        c.user_key_url = s("https://github.com/u.keys");
        let out = render_preset(&c, false, &host()).unwrap();
        assert!(!out.contains("PRESET_ROOT_KEY"), "{out}");
        assert!(!out.contains("PRESET_USER_KEY"), "{out}");
        assert!(!out.contains("github"), "{out}");
        let out = render_preset(&c, true, &host()).unwrap();
        assert!(
            out.contains("PRESET_USER_KEY=\"file:///root/.imager-user-authorized-keys\"\n"),
            "{out}"
        );
        c.user_name = None;
        assert!(!render_preset(&c, true, &host())
            .unwrap()
            .contains("PRESET_USER_KEY"));
    }

    #[test]
    fn region_comes_from_the_host_only_when_asked() {
        let mut c = empty();
        c.lang_based_on_location = Some(true);
        assert!(render(&c).starts_with(
            "PRESET_LOCALE=\"it_IT.UTF-8\"\nPRESET_TIMEZONE=\"Europe/Rome\"\nSET_LANG_BASED_ON_LOCATION=\"n\"\n"
        ));
        c.lang_based_on_location = Some(false);
        assert!(render(&c).starts_with(DEFAULT_REGION));
        c.lang_based_on_location = None;
        c.root_password = s("pw");
        assert!(render(&c).starts_with(DEFAULT_REGION));

        let mut typed = empty();
        typed.lang_based_on_location = Some(true);
        typed.locale = s(" de_DE.UTF-8 ");
        let out = render(&typed);
        assert!(out.starts_with(
            "PRESET_LOCALE=\"de_DE.UTF-8\"\nPRESET_TIMEZONE=\"Europe/Rome\"\nSET_LANG_BASED_ON_LOCATION=\"n\"\n"
        ), "{out}");
    }

    #[test]
    fn host_values_are_validated_or_defaulted() {
        for (raw, locale) in [
            ("it-IT", "it_IT.UTF-8"),
            ("en_US.UTF-8", "en_US.UTF-8"),
            ("de_DE.UTF-8@euro", "de_DE.UTF-8"),
            ("zh-Hans-CN", "zh_CN.UTF-8"),
            ("fil-PH", "fil_PH.UTF-8"),
            ("es-419", "en_US.UTF-8"),
            ("fr", "en_US.UTF-8"),
            ("C", "en_US.UTF-8"),
            ("x\"$(reboot)-IT", "en_US.UTF-8"),
        ] {
            assert_eq!(
                HostRegion::from_raw(Some(raw), None).locale,
                locale,
                "{raw}"
            );
        }
        for (raw, zone) in [
            ("Europe/Rome", "Europe/Rome"),
            (
                "America/Argentina/Buenos_Aires",
                "America/Argentina/Buenos_Aires",
            ),
            ("Etc/GMT+5", "Etc/GMT+5"),
            ("W. Europe Standard Time", "Europe/Berlin"),
            ("Pacific Standard Time", "America/Los_Angeles"),
            ("Some Unknown Standard Time", "Etc/UTC"),
            ("../etc/passwd", "Etc/UTC"),
            ("Europe//Rome", "Etc/UTC"),
            ("", "Etc/UTC"),
        ] {
            assert_eq!(
                HostRegion::from_raw(None, Some(raw)).timezone,
                zone,
                "{raw}"
            );
        }
        assert_eq!(HostRegion::from_raw(None, None), HostRegion::defaults());
        assert!(is_valid_locale(&HostRegion::detect().locale));
        assert!(is_valid_timezone(&HostRegion::detect().timezone));
    }

    #[test]
    fn shell_special_chars_escaped() {
        let mut c = empty();
        c.user_password = s("a\"b$c`d\\e");
        assert!(render(&c).contains("PRESET_USER_PASSWORD=\"a\\\"b\\$c\\`d\\\\e\"\n"));
    }

    #[test]
    fn a_chosen_shell_wins_over_the_default() {
        let mut c = empty();
        c.user_shell = Some(UserShell::Zsh);
        assert!(render(&c).contains("PRESET_USER_SHELL=\"zsh\"\n"));
    }

    fn padded() -> AutoconfigConfig {
        let mut c = empty();
        c.apply_network = Some(true);
        c.wifi_enabled = Some(true);
        c.wifi_ssid = s(" My Net ");
        c.wifi_key = s(" k e y ");
        c.wifi_country_code = s(" IT ");
        c.use_static_ip = Some(true);
        c.static_ip = s("192.168.1.50 ");
        c.static_mask = s(" 255.255.255.0");
        c.static_gateway = s(" 192.168.1.1 ");
        c.static_dns = s(" 8.8.8.8, 1.1.1.1 ");
        c.locale = s(" en_US.UTF-8 ");
        c.timezone = s("Europe/Rome ");
        c.root_password = s(" root pw ");
        c.root_key_url = s(" https://github.com/r.keys ");
        c.user_name = s(" armbian ");
        c.user_password = s(" user pw ");
        c.user_key_url = s("https://github.com/u.keys ");
        c.user_shell = Some(UserShell::Bash);
        c.user_real_name = s(" Armbian User ");
        c.remote_config_url = s(" https://example.com/config.txt ");
        c
    }

    const PADDED_PRESET: &str = "PRESET_NET_CHANGE_DEFAULTS=\"1\"\n\
        PRESET_NET_WIFI_ENABLED=\"1\"\n\
        PRESET_NET_WIFI_SSID=\" My Net \"\n\
        PRESET_NET_WIFI_KEY=\" k e y \"\n\
        PRESET_NET_WIFI_COUNTRYCODE=\"IT\"\n\
        PRESET_NET_USE_STATIC=\"1\"\n\
        PRESET_NET_STATIC_IP=\"192.168.1.50\"\n\
        PRESET_NET_STATIC_MASK=\"255.255.255.0\"\n\
        PRESET_NET_STATIC_GATEWAY=\"192.168.1.1\"\n\
        PRESET_NET_STATIC_DNS=\"8.8.8.8,1.1.1.1\"\n\
        PRESET_LOCALE=\"en_US.UTF-8\"\n\
        PRESET_TIMEZONE=\"Europe/Rome\"\n\
        SET_LANG_BASED_ON_LOCATION=\"n\"\n\
        PRESET_ROOT_PASSWORD=\" root pw \"\n\
        PRESET_USER_NAME=\"armbian\"\n\
        PRESET_USER_PASSWORD=\" user pw \"\n\
        PRESET_USER_KEY=\"file:///root/.imager-user-authorized-keys\"\n\
        PRESET_USER_SHELL=\"bash\"\n\
        PRESET_DEFAULT_REALNAME=\"Armbian User\"\n\
        PRESET_CONFIGURATION=\"https://example.com/config.txt\"\n\
        PRESET_CONNECT_WIRELESS=\"n\"\n";

    #[test]
    fn structural_values_are_trimmed_and_secrets_kept_verbatim() {
        assert_eq!(
            render_preset(&padded(), true, &host()).unwrap(),
            PADDED_PRESET
        );
    }

    #[test]
    fn a_blank_structural_value_is_skipped_or_defaulted() {
        let mut c = padded();
        c.static_ip = s("   ");
        c.timezone = s(" ");
        let out = render(&c);
        assert!(!out.contains("PRESET_NET_STATIC_IP="), "{out}");
        assert!(out.contains("PRESET_TIMEZONE=\"Etc/UTC\"\n"), "{out}");
        assert!(out.ends_with("PRESET_CONNECT_WIRELESS=\"n\"\n"), "{out}");
    }

    #[test]
    fn user_names_already_in_the_image_are_refused() {
        let files = |path: &str| -> Result<Vec<u8>, WriteConfError> {
            Ok(match path {
                cfg::PASSWD_PATH => b"root:x:0:0:root:/root:/bin/bash\nsshd:x:100:65534::/run/sshd:/usr/sbin/nologin\n".to_vec(),
                _ => b"root:x:0:\nnetdev:x:106:\n".to_vec(),
            })
        };
        assert!(check_user_name_free("john", files).is_ok());
        for taken in ["root", "sshd", "netdev"] {
            let err = check_user_name_free(taken, files).unwrap_err();
            assert!(err.starts_with(TAG_USER_NAME_TAKEN), "{taken}: {err}");
        }
        let unreadable = |_: &str| -> Result<Vec<u8>, WriteConfError> {
            Err(WriteConfError::Ext4("missing".into()))
        };
        let err = check_user_name_free("john", unreadable).unwrap_err();
        assert!(err.starts_with(TAG_USERS_UNREADABLE), "{err}");
    }

    #[test]
    fn the_resolved_profile_debug_hides_keys_and_secrets() {
        let mut c = empty();
        c.user_name = s("pi");
        c.root_password = s("SECRET-ROOT");
        let profile = ResolvedProfile::new(
            c,
            Some("ssh-ed25519 SECRETKEY".into()),
            Some("ssh-ed25519 SECRETUSER".into()),
            host(),
        )
        .unwrap();
        let shown = format!("{profile:?}");
        assert!(!shown.contains("SECRET"), "{shown}");
        assert!(shown.contains("root_keys_set: true"), "{shown}");
    }

    #[test]
    fn user_keys_are_dropped_without_a_user() {
        let profile =
            ResolvedProfile::new(empty(), None, Some("ssh-ed25519 AAAA".into()), host()).unwrap();
        assert!(profile.user_keys.is_none());
    }

    const KEY_LINE: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDgv2rp6WIB7aAlusRYNNcc8UkSS4fZFJbEjajT7nI/n t\n";

    fn e2fs_tool(name: &str) -> Option<PathBuf> {
        [
            "/opt/homebrew/opt/e2fsprogs/sbin/",
            "/usr/local/opt/e2fsprogs/sbin/",
            "/sbin/",
            "/usr/sbin/",
        ]
        .iter()
        .map(|dir| PathBuf::from(format!("{dir}{name}")))
        .find(|p| p.exists())
    }

    /// A bare mkfs.ext4 rootfs with /root, the first-login marker and an /etc/passwd holding `taken`; None without e2fsprogs.
    fn armbian_like_rootfs(dir: &Path, taken: &str) -> Option<PathBuf> {
        rootfs_with(dir, taken, true)
    }

    fn rootfs_with(dir: &Path, taken: &str, marker_present: bool) -> Option<PathBuf> {
        let (mkfs, debugfs) = (e2fs_tool("mkfs.ext4")?, e2fs_tool("debugfs")?);
        let image = dir.join("rootfs.img");
        std::fs::File::create(&image)
            .unwrap()
            .set_len(64 << 20)
            .unwrap();
        assert!(std::process::Command::new(mkfs)
            .args(["-q", "-F", "-b", "4096"])
            .arg(&image)
            .status()
            .unwrap()
            .success());
        let passwd = dir.join("passwd");
        std::fs::write(
            &passwd,
            format!("root:x:0:0::/root:/bin/bash\n{taken}:x:1000:1000::/home/{taken}:/bin/bash\n"),
        )
        .unwrap();
        let marker = dir.join("marker");
        std::fs::write(&marker, b"#!/bin/bash\n").unwrap();
        let script = dir.join("setup.debugfs");
        std::fs::write(
            &script,
            format!(
                "mkdir root\nsif root mode 040700\nmkdir etc\nwrite {p} etc/passwd\nwrite {p} etc/group\n\
                 mkdir usr\nmkdir usr/lib\nmkdir usr/lib/armbian\n{m}",
                p = passwd.display(),
                m = if marker_present {
                    format!("write {} usr/lib/armbian/armbian-firstlogin\n", marker.display())
                } else {
                    String::new()
                }
            ),
        )
        .unwrap();
        assert!(std::process::Command::new(debugfs)
            .arg("-w")
            .arg("-f")
            .arg(&script)
            .arg(&image)
            .output()
            .unwrap()
            .status
            .success());
        Some(image)
    }

    fn debugfs_stat(image: &Path, path: &str) -> String {
        let out = std::process::Command::new(e2fs_tool("debugfs").unwrap())
            .arg("-R")
            .arg(format!("stat {path}"))
            .arg(image)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    #[test]
    fn keys_and_preset_land_root_only_in_a_bare_rootfs() {
        let dir = tempfile::tempdir().unwrap();
        let Some(image) = armbian_like_rootfs(dir.path(), "armbian") else {
            return;
        };
        let mut c = empty();
        c.user_name = s("pi");
        c.user_password = s("pw");
        c.root_key_url = s("https://github.com/r.keys");
        c.user_key_url = s("https://github.com/u.keys");
        let profile =
            ResolvedProfile::new(c, Some(KEY_LINE.into()), Some(KEY_LINE.into()), host()).unwrap();

        check_user_name_free_in_bare_ext4(&image, "pi").unwrap();
        let taken = check_user_name_free_in_bare_ext4(&image, "armbian").unwrap_err();
        assert!(taken.starts_with(TAG_USER_NAME_TAKEN), "{taken}");
        inject_into_bare_ext4_image(&image, &profile).unwrap();

        for (path, mode) in [
            (cfg::ROOT_SSH_DIR, "0700"),
            (cfg::ROOT_AUTHORIZED_KEYS, "0600"),
            (cfg::USER_KEYS_FILE, "0600"),
            (cfg::PRESET_PATH, "0600"),
        ] {
            let stat = debugfs_stat(&image, path);
            assert!(stat.contains(&format!("Mode:  {mode}")), "{path}: {stat}");
            assert!(
                stat.contains("User:     0   Group:     0"),
                "{path}: {stat}"
            );
        }
        let fsck = std::process::Command::new(e2fs_tool("e2fsck").unwrap())
            .arg("-fn")
            .arg(&image)
            .output()
            .unwrap();
        assert!(
            fsck.status.success(),
            "{}",
            String::from_utf8_lossy(&fsck.stdout)
        );
    }

    /// An MBR disk around `rootfs`, as a cached image looks, written to `name` in `dir`.
    fn disk_around(dir: &Path, rootfs: &Path, name: &str) -> (PathBuf, Vec<u8>) {
        let mut disk = vec![0u8; 1 << 20];
        disk[0x1be + 4] = 0x83;
        disk[0x1be + 8..0x1be + 12].copy_from_slice(&2048u32.to_le_bytes());
        disk[0x1be + 12..0x1be + 16].copy_from_slice(&((64u32 << 20) / 512).to_le_bytes());
        disk[510] = 0x55;
        disk[511] = 0xaa;
        disk.extend_from_slice(&std::fs::read(rootfs).unwrap());
        let path = dir.join(name);
        std::fs::write(&path, &disk).unwrap();
        (path, disk)
    }

    /// Port 1 on loopback refuses at once, so a request that is made fails as [SSH_KEYS_NETWORK].
    const UNREACHABLE_KEYS: &str = "https://127.0.0.1:1/keys";

    async fn prep_error(source: &Path, temp_dir: &Path, c: AutoconfigConfig) -> String {
        let state = Arc::new(FlashState::new());
        match prepare_flash_copy(source.into(), temp_dir.into(), c, true, state.clone()).await {
            Err(PrepError::Failed(e)) => {
                assert_eq!(state.prep_stage(), None);
                e
            }
            other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
        }
    }

    #[tokio::test]
    async fn source_refusals_come_before_any_key_request() {
        let dir = tempfile::tempdir().unwrap();
        let Some(rootfs) = armbian_like_rootfs(dir.path(), "pi") else {
            return;
        };
        let (source, disk) = disk_around(dir.path(), &rootfs, "disk.img");
        let temp_dir = dir.path().join("work");
        let with_keys = |user: &str| {
            let mut c = empty();
            c.user_name = s(user);
            c.root_key_url = s(UNREACHABLE_KEYS);
            c.user_key_url = s(UNREACHABLE_KEYS);
            c
        };

        let taken = prep_error(&source, &temp_dir, with_keys("PI")).await;
        assert!(taken.starts_with(TAG_USER_NAME_TAKEN), "{taken}");

        let bare = dir.path().join("no-marker");
        std::fs::create_dir_all(&bare).unwrap();
        let no_marker_rootfs = rootfs_with(&bare, "pi", false).unwrap();
        let (no_marker, _) = disk_around(dir.path(), &no_marker_rootfs, "custom.img");
        let missing = prep_error(&no_marker, &temp_dir, with_keys("john")).await;
        assert!(missing.starts_with(TAG_NOT_ARMBIAN), "{missing}");

        // Control: once the source passes, the same URL is requested and fails as a network error.
        let fetched = prep_error(&source, &temp_dir, with_keys("john")).await;
        assert!(
            fetched.starts_with(crate::ssh_keys::TAG_NETWORK),
            "{fetched}"
        );

        assert!(!temp_dir.exists(), "no working copy at any step");
        assert!(std::fs::read(&source).unwrap() == disk, "source untouched");
    }

    #[test]
    fn a_taken_user_name_stops_before_the_copy() {
        let dir = tempfile::tempdir().unwrap();
        let Some(rootfs) = armbian_like_rootfs(dir.path(), "pi") else {
            return;
        };
        let (source, disk) = disk_around(dir.path(), &rootfs, "disk.img");
        let temp_dir = dir.path().join("work");
        let mut c = empty();
        c.user_name = s("PI");
        let profile = ResolvedProfile::new(c, None, None, host()).unwrap();

        let err = match prepare_tracked(&source, &temp_dir, &profile, true, &FlashState::new()) {
            Err(PrepError::Failed(e)) => e,
            other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
        };
        assert!(err.starts_with(TAG_USER_NAME_TAKEN), "{err}");
        assert!(!temp_dir.exists(), "no working copy");
        assert!(std::fs::read(&source).unwrap() == disk, "source untouched");

        let mut free = empty();
        free.user_name = s("john");
        let profile = ResolvedProfile::new(free, None, None, host()).unwrap();
        let copy = prepare_tracked(&source, &temp_dir, &profile, true, &FlashState::new()).unwrap();
        assert!(std::fs::read(&source).unwrap() == disk, "source untouched");
        drop(copy);
    }

    #[test]
    fn working_copy_never_touches_the_source() {
        let dir = test_scratch_dir("wc-test");
        let temp_dir = dir.join("work");
        let source = dir.join("source.img");
        let original = vec![0x5au8; 64 * 1024];
        std::fs::write(&source, &original).unwrap();

        let result = prepare_tracked(&source, &temp_dir, &resolved(), false, &FlashState::new());

        assert!(result.is_err(), "a non-ext4 image must be refused");
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read_dir(&temp_dir).unwrap().count(), 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn resolved() -> ResolvedProfile {
        ResolvedProfile::new(profile(), None, None, host()).unwrap()
    }

    fn profile() -> AutoconfigConfig {
        let mut config = empty();
        config.locale = Some("en_US.UTF-8".to_string());
        config
    }

    #[test]
    fn a_cancel_before_prep_makes_no_copy() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.img");
        std::fs::write(&source, b"image").unwrap();
        let temp_dir = dir.path().join("work");
        let state = FlashState::new();
        state
            .is_cancelled
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let result = prepare_tracked(&source, &temp_dir, &resolved(), false, &state);

        assert_eq!(result.err(), Some(PrepError::Cancelled));
        assert!(!temp_dir.exists());
        assert_eq!(state.prep_stage(), None);
    }

    #[tokio::test]
    async fn a_failed_prep_clears_the_stage_and_the_copy() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.img");
        std::fs::write(&source, vec![0x5au8; 64 * 1024]).unwrap();
        let temp_dir = dir.path().join("work");
        let state = Arc::new(FlashState::new());

        let result =
            prepare_flash_copy(source, temp_dir.clone(), profile(), false, state.clone()).await;

        assert!(matches!(result, Err(PrepError::Failed(_))));
        assert_eq!(state.prep_stage(), None);
        assert_eq!(std::fs::read_dir(&temp_dir).unwrap().count(), 0);
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
    fn a_custom_image_is_checked_before_any_copy() {
        let dir = test_scratch_dir("wc-marker");
        let temp_dir = dir.join("work");
        let source = dir.join("custom.img");
        let original = vec![0x5au8; 64 * 1024];
        std::fs::write(&source, &original).unwrap();
        let mut config = empty();
        config.root_password = Some("secret".to_string());
        let profile = ResolvedProfile::new(config, None, None, host()).unwrap();

        let err = match prepare_tracked(&source, &temp_dir, &profile, true, &FlashState::new()) {
            Err(PrepError::Failed(e)) => e,
            other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
        };

        assert!(err.contains("ext4"), "{err}");
        assert!(!temp_dir.exists(), "the refusal comes before the copy");
        assert_eq!(std::fs::read(&source).unwrap(), original);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_directory_is_never_probed_as_an_image() {
        let dir = test_scratch_dir("wc-marker-dir");
        assert!(matches!(
            has_firstlogin_marker(&dir),
            Err(WriteConfError::UnsupportedImage(_))
        ));
        assert!(require_firstlogin_marker(&dir.join("missing.img")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn marker_verdicts_keep_missing_apart_from_read_errors() {
        assert!(marker_verdict(Ok(true)).is_ok());
        let missing = marker_verdict(Ok(false)).unwrap_err();
        assert!(missing.starts_with(TAG_NOT_ARMBIAN), "{missing}");
        let no_ext4 = marker_verdict(Err(WriteConfError::NoExt4Rootfs("x".into()))).unwrap_err();
        assert!(!no_ext4.contains(TAG_NOT_ARMBIAN), "{no_ext4}");
        let io =
            marker_verdict(Err(WriteConfError::Io(std::io::Error::other("boom")))).unwrap_err();
        assert!(!io.contains(TAG_NOT_ARMBIAN), "{io}");
        assert!(io.contains("Failed to read"), "{io}");
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

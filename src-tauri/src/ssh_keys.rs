// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

//! Looks up the public SSH keys a profile's key source serves, to show what the board fetches at first boot.

use std::time::Duration;

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

use crate::config::ssh_keys as cfg;
use crate::{log_info, log_warn};

const MODULE: &str = "ssh_keys";

pub const TAG_INVALID_INPUT: &str = "[SSH_KEYS_INVALID_INPUT]";
pub const TAG_NOT_FOUND: &str = "[SSH_KEYS_NOT_FOUND]";
pub const TAG_NONE: &str = "[SSH_KEYS_NONE]";
pub const TAG_TOO_LARGE: &str = "[SSH_KEYS_TOO_LARGE]";
pub const TAG_NETWORK: &str = "[SSH_KEYS_NETWORK]";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SshKeySource {
    Github,
    Gitlab,
    Url,
}

impl SshKeySource {
    fn log_label(self) -> &'static str {
        match self {
            SshKeySource::Github => "github",
            SshKeySource::Gitlab => "gitlab",
            SshKeySource::Url => "link",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshKeyInfo {
    pub key_type: String,
    /// Short label as `ssh-keygen -l` prints it, e.g. `ED25519`
    pub label: String,
    pub comment: Option<String>,
    /// `SHA256:<base64 without padding>`, as `ssh-keygen -l` prints it
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshKeyLookup {
    /// Every valid key served; `keys` lists at most `MAX_KEYS` of them
    pub total: usize,
    pub keys: Vec<SshKeyInfo>,
}

/// GitHub logins: ASCII alphanumerics and single inner hyphens.
pub fn is_valid_github_username(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= cfg::GITHUB_USERNAME_MAX_CHARS
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
}

/// GitLab usernames: ASCII alphanumerics plus `.`, `_`, `-`, starting alphanumeric, not ending in `.`.
pub fn is_valid_gitlab_username(name: &str) -> bool {
    name.len() <= cfg::GITLAB_USERNAME_MAX_CHARS
        && name
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphanumeric())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        && !name.ends_with('.')
}

/// Accept only an absolute https URL with a host and no credentials.
pub fn parse_key_url(value: &str) -> Option<Url> {
    if value.len() > cfg::URL_MAX_CHARS
        || value
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || is_bidi_control(c))
    {
        return None;
    }
    // The URL parser would read "https:///x" or "https:host" as having a host; require one written out.
    let prefix = format!("{}://", cfg::REQUIRED_SCHEME);
    let authority = value
        .get(..prefix.len())
        .filter(|p| p.eq_ignore_ascii_case(&prefix))
        .map(|_| &value[prefix.len()..])?;
    if authority.is_empty() || authority.starts_with(['/', '\\']) {
        return None;
    }
    let url = Url::parse(value).ok()?;
    let ok = url.scheme() == cfg::REQUIRED_SCHEME
        && url.host_str().is_some_and(|h| !h.is_empty())
        && url.username().is_empty()
        && url.password().is_none();
    ok.then_some(url)
}

pub fn source_url(source: SshKeySource, value: &str) -> Option<Url> {
    let value = value.trim();
    match source {
        SshKeySource::Github => is_valid_github_username(value)
            .then(|| format!("{}{value}{}", cfg::GITHUB_KEYS_BASE, cfg::KEYS_SUFFIX)),
        SshKeySource::Gitlab => is_valid_gitlab_username(value)
            .then(|| format!("{}{value}{}", cfg::GITLAB_KEYS_BASE, cfg::KEYS_SUFFIX)),
        SshKeySource::Url => return parse_key_url(value),
    }
    .and_then(|s| Url::parse(&s).ok())
}

/// SHA256 fingerprint of a key blob, formatted like `ssh-keygen -l`.
pub fn fingerprint(blob: &[u8]) -> String {
    let digest = Sha256::digest(blob);
    format!(
        "{}{}",
        cfg::FINGERPRINT_PREFIX,
        base64::engine::general_purpose::STANDARD_NO_PAD.encode(digest)
    )
}

fn blob_key_type(blob: &[u8]) -> Option<&[u8]> {
    let len = u32::from_be_bytes(blob.get(..4)?.try_into().ok()?) as usize;
    let end = 4usize.checked_add(len)?;
    let key_type = blob.get(4..end)?;
    // A type string followed by nothing is not a key.
    (end < blob.len()).then_some(key_type)
}

fn sanitize_comment(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|c| !c.is_control() && !is_bidi_control(*c))
        .collect();
    let trimmed = cleaned.trim();
    (!trimmed.is_empty()).then(|| trimmed.chars().take(cfg::COMMENT_MAX_CHARS).collect())
}

fn is_bidi_control(c: char) -> bool {
    matches!(c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

fn next_token(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }
    Some(match s.find(char::is_whitespace) {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, ""),
    })
}

/// Parse one `type base64 [comment]` line; anything else (options, other types) yields None.
pub fn parse_key_line(line: &str) -> Option<SshKeyInfo> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (key_type, rest) = next_token(line)?;
    let label = cfg::KEY_TYPES
        .iter()
        .find(|(t, _)| *t == key_type)
        .map(|(_, l)| *l)?;
    let (encoded, comment) = next_token(rest)?;
    let blob = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    if blob_key_type(&blob)? != key_type.as_bytes() {
        return None;
    }
    Some(SshKeyInfo {
        key_type: key_type.to_string(),
        label: label.to_string(),
        comment: sanitize_comment(comment),
        fingerprint: fingerprint(&blob),
    })
}

/// Parse a keys body into the lookup result, or `TAG_NONE` when no line is a valid key.
pub fn parse_keys(body: &str) -> Result<SshKeyLookup, String> {
    let mut total = 0;
    let mut keys = Vec::new();
    for key in body.lines().filter_map(parse_key_line) {
        total += 1;
        if keys.len() < cfg::MAX_KEYS {
            keys.push(key);
        }
    }
    if total == 0 {
        return Err(TAG_NONE.to_string());
    }
    Ok(SshKeyLookup { total, keys })
}

pub fn append_capped(buf: &mut Vec<u8>, chunk: &[u8], cap: usize) -> Result<(), String> {
    if buf.len().saturating_add(chunk.len()) > cap {
        return Err(TAG_TOO_LARGE.to_string());
    }
    buf.extend_from_slice(chunk);
    Ok(())
}

fn redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() > cfg::MAX_REDIRECTS {
            attempt.error("too many redirects")
        } else if attempt.url().scheme() != cfg::REQUIRED_SCHEME {
            attempt.error("redirect to a non-https URL")
        } else {
            attempt.follow()
        }
    })
}

async fn fetch_body(url: Url) -> Result<Vec<u8>, String> {
    let client = crate::utils::client_builder(Duration::from_secs(cfg::TIMEOUT_SECS))
        .redirect(redirect_policy())
        .build()
        .map_err(|_| TAG_NETWORK.to_string())?;
    let mut response = client.get(url).send().await.map_err(|e| {
        log_warn!(
            MODULE,
            "Request failed: timeout={} redirect={}",
            e.is_timeout(),
            e.is_redirect()
        );
        TAG_NETWORK.to_string()
    })?;
    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(TAG_NOT_FOUND.to_string());
    }
    if !status.is_success() {
        log_warn!(MODULE, "Unexpected status {}", status.as_u16());
        return Err(TAG_NETWORK.to_string());
    }
    if response
        .content_length()
        .is_some_and(|len| len > cfg::MAX_BODY_BYTES as u64)
    {
        return Err(TAG_TOO_LARGE.to_string());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| TAG_NETWORK.to_string())?
    {
        append_capped(&mut body, &chunk, cfg::MAX_BODY_BYTES)?;
    }
    Ok(body)
}

/// Errors are the `TAG_*` codes only; logs carry the source kind and outcome, never the URL or the body.
pub async fn lookup(source: SshKeySource, value: &str) -> Result<SshKeyLookup, String> {
    let kind = source.log_label();
    let result = match source_url(source, value) {
        Some(url) => fetch_body(url)
            .await
            .and_then(|body| parse_keys(&String::from_utf8_lossy(&body))),
        None => Err(TAG_INVALID_INPUT.to_string()),
    };
    match &result {
        Ok(found) => log_info!(
            MODULE,
            "SSH key lookup: source={kind}, {} keys",
            found.total
        ),
        Err(code) => log_warn!(MODULE, "SSH key lookup: source={kind}, {code}"),
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // Throwaway keys made with ssh-keygen; fingerprints are its `-lf` output.
    const ED25519: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDgv2rp6WIB7aAlusRYNNcc8UkSS4fZFJbEjajT7nI/n test@ed25519";
    const ED25519_FP: &str = "SHA256:0Ueb+rQgTcN0PKZiNVtI5OjAqGow2RBgmUoIF3bOZTo";
    const RSA: &str = "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAAAgQDicsYwXdzcSMbvx3LHnkKbU/3QrUUrkbKymDOqdy4FJEYlqnKKciUhUozNJutCZmO0BlhbgDhXiZucyR8QU7epjiYNcISI+JD88YkiWpefVxCwlFKGaYyqX9Rq681uegAKLb2IlhpWlq+B1CtFhh3HPJMD7EkJDiDDsDBaLAcvmQ== rsa key";
    const RSA_FP: &str = "SHA256:lzPSX6CICuM+e5rnLCuAbffCIE3a/kPe4OZ99yHO5Wg";
    const P256: &str = "ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBDx8B00Or/gXBID3SN1xQB6Q+A9GMktYnpvtEbq8nJI6E9ssh7gYUDkvIXXtO/VA53e4tMy4JWrzEt1mLYczJnw= p256";
    const P256_FP: &str = "SHA256:DwIEu+IH+TB6cEElbWgdAkg0XydmADATgaMpctjZT8E";
    const P384: &str = "ecdsa-sha2-nistp384 AAAAE2VjZHNhLXNoYTItbmlzdHAzODQAAAAIbmlzdHAzODQAAABhBMMPMvSEDS+mY7wpOkgNSWOATHa+oEh6RE8l207+fTMyaSDZVsmyqmybRB0bZwoSWZz3ZZifCJrhaw6zkn+r94lqZtS2uzeQ85245y1/7PsiIF1xeAYfjzJX3DD64LpBuQ==";
    const P384_FP: &str = "SHA256:IP84ButayjXEDzqrpHl//7UDRcXq+EhqAdV1LHvSZ30";
    const P521: &str = "ecdsa-sha2-nistp521 AAAAE2VjZHNhLXNoYTItbmlzdHA1MjEAAAAIbmlzdHA1MjEAAACFBAE92Pc2goU5xCOWSrM0f11ahEjZX5JKdZ0YYJ1avzfTpB2YA0vFMh820hIlYegsUlWj5ka/V+O2sL7ySwtxQX/i3ADUOuQRuCANWsn230mrvEyObOZhowxlvXW4FFQ5E6LSZGyA2+duP1BnkloQDe6Vm4T2t7qJ2pxfM6dH209XEjmULQ== p521";
    const P521_FP: &str = "SHA256:7m8VPdf4x/UbP/KDYzf12fcc3pFfQp1Lz/XtvFK276s";
    const SK_ED25519: &str = "sk-ssh-ed25519@openssh.com AAAAGnNrLXNzaC1lZDI1NTE5QG9wZW5zc2guY29tAAAAIAABAgMEBQYHCAkKCwwNDg8QERITFBUWFxgZGhscHR4fAAAABHNzaDo= yubi";
    const SK_ED25519_FP: &str = "SHA256:/p0CbeE3dk2SyW1OXXsThGc12ezDVD8eGw2/vtztDfk";
    const SK_ECDSA: &str = "sk-ecdsa-sha2-nistp256@openssh.com AAAAInNrLWVjZHNhLXNoYTItbmlzdHAyNTZAb3BlbnNzaC5jb20AAAAIbmlzdHAyNTYAAABBBDx8B00Or/gXBID3SN1xQB6Q+A9GMktYnpvtEbq8nJI6E9ssh7gYUDkvIXXtO/VA53e4tMy4JWrzEt1mLYczJnwAAAAEc3NoOg== yubi-ec";
    const SK_ECDSA_FP: &str = "SHA256:eyojox/Nidcc9hB5ash6t/1SCago1/t+AF5M8nNKgDE";

    #[test]
    fn github_usernames() {
        for ok in ["octocat", "a", "Super-Kali", "a1-b2-c3", &"x".repeat(39)] {
            assert!(is_valid_github_username(ok), "{ok}");
        }
        for bad in [
            "",
            "-lead",
            "trail-",
            "dou--ble",
            "under_score",
            "dot.ted",
            "sp ace",
            "../etc",
            "a/b",
            "ünï",
            &"x".repeat(40),
        ] {
            assert!(!is_valid_github_username(bad), "{bad}");
        }
    }

    #[test]
    fn gitlab_usernames() {
        for ok in ["gitlab-user", "j.doe", "a_b-c.d", "x_", "9lives"] {
            assert!(is_valid_gitlab_username(ok), "{ok}");
        }
        for bad in [
            "",
            ".lead",
            "_lead",
            "end.",
            "a/b",
            "a b",
            "a?b",
            &"x".repeat(256),
        ] {
            assert!(!is_valid_gitlab_username(bad), "{bad}");
        }
    }

    #[test]
    fn source_urls_are_built_from_constants() {
        assert_eq!(
            source_url(SshKeySource::Github, " octocat ")
                .unwrap()
                .as_str(),
            "https://github.com/octocat.keys"
        );
        assert_eq!(
            source_url(SshKeySource::Gitlab, "j.doe").unwrap().as_str(),
            "https://gitlab.com/j.doe.keys"
        );
        assert!(source_url(SshKeySource::Github, "a/b").is_none());
    }

    #[test]
    fn key_urls() {
        assert!(parse_key_url("https://example.com/keys").is_some());
        assert!(parse_key_url("https://example.com:8443/k?t=1").is_some());
        assert!(parse_key_url("HTTPS://Example.com/keys").is_some());
        for bad in [
            "http://example.com/keys",
            "https://user:pw@example.com/keys",
            "https://user@example.com/keys",
            "https:///keys",
            "https://",
            "file:///etc/passwd",
            "ftp://example.com/keys",
            "example.com/keys",
            "https://exa mple.com/keys",
            "https://example.com/\nkeys",
            "https:example.com/keys",
            "https:\\\\example.com/keys",
            "https://example.com/\u{202E}syek",
            "",
        ] {
            assert!(parse_key_url(bad).is_none(), "{bad}");
        }
        assert!(parse_key_url(&format!("https://example.com/{}", "a".repeat(2048))).is_none());
    }

    #[test]
    fn every_type_parses_with_ssh_keygen_fingerprint() {
        for (line, fp, label, comment) in [
            (ED25519, ED25519_FP, "ED25519", Some("test@ed25519")),
            (RSA, RSA_FP, "RSA", Some("rsa key")),
            (P256, P256_FP, "ECDSA", Some("p256")),
            (P384, P384_FP, "ECDSA", None),
            (P521, P521_FP, "ECDSA", Some("p521")),
            (SK_ED25519, SK_ED25519_FP, "ED25519-SK", Some("yubi")),
            (SK_ECDSA, SK_ECDSA_FP, "ECDSA-SK", Some("yubi-ec")),
        ] {
            let key = parse_key_line(line).unwrap_or_else(|| panic!("{line}"));
            assert_eq!(key.fingerprint, fp);
            assert_eq!(key.label, label);
            assert_eq!(key.comment.as_deref(), comment);
            assert_eq!(key.key_type, line.split(' ').next().unwrap());
        }
    }

    #[test]
    fn mismatched_embedded_type_is_rejected() {
        let blob = ED25519.split(' ').nth(1).unwrap();
        assert!(parse_key_line(&format!("ssh-rsa {blob}")).is_none());
        assert!(parse_key_line(&format!("sk-ssh-ed25519@openssh.com {blob}")).is_none());
    }

    #[test]
    fn malformed_lines_are_skipped() {
        for bad in [
            "",
            "   ",
            "# ssh-ed25519 AAAA comment",
            "ssh-ed25519",
            "ssh-ed25519 !!!notbase64!!!",
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5",
            "ssh-dss AAAAB3NzaC1kc3M= old",
            "command=\"x\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDgv2rp6WIB7aAlusRYNNcc8UkSS4fZFJbEjajT7nI/n",
            "<html><body>Sign in</body></html>",
        ] {
            assert!(parse_key_line(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn comments_are_sanitized() {
        let blob = ED25519.split(' ').nth(1).unwrap();
        let key = parse_key_line(&format!("ssh-ed25519 {blob}   me\u{7}@\u{202E}host\t ")).unwrap();
        assert_eq!(key.comment.as_deref(), Some("me@host"));
        let long = parse_key_line(&format!("ssh-ed25519 {blob} {}", "c".repeat(500))).unwrap();
        assert_eq!(
            long.comment.unwrap().chars().count(),
            cfg::COMMENT_MAX_CHARS
        );
        let empty = parse_key_line(&format!("ssh-ed25519 {blob} \u{1b} ")).unwrap();
        assert_eq!(empty.comment, None);
    }

    #[test]
    fn body_with_crlf_comments_and_junk() {
        let body = format!("# keys\r\n{ED25519}\r\n\r\njunk line\r\n{RSA}\r\nssh-rsa AAAA\r\n");
        let found = parse_keys(&body).unwrap();
        assert_eq!(found.total, 2);
        assert_eq!(found.keys[0].fingerprint, ED25519_FP);
        assert_eq!(found.keys[1].fingerprint, RSA_FP);
        assert_eq!(found.keys[1].comment.as_deref(), Some("rsa key"));
    }

    #[test]
    fn empty_or_invalid_body_is_none() {
        assert_eq!(parse_keys("").unwrap_err(), TAG_NONE);
        assert_eq!(parse_keys("Not Found\n<html>").unwrap_err(), TAG_NONE);
    }

    #[test]
    fn key_list_is_capped_but_total_counts_all() {
        let body = format!("{ED25519}\n").repeat(cfg::MAX_KEYS + 7);
        let found = parse_keys(&body).unwrap();
        assert_eq!(found.total, cfg::MAX_KEYS + 7);
        assert_eq!(found.keys.len(), cfg::MAX_KEYS);
    }

    #[test]
    fn body_cap() {
        let mut buf = Vec::new();
        append_capped(&mut buf, &[0; 10], 16).unwrap();
        append_capped(&mut buf, &[0; 6], 16).unwrap();
        assert_eq!(buf.len(), 16);
        assert_eq!(
            append_capped(&mut buf, &[0; 1], 16).unwrap_err(),
            TAG_TOO_LARGE
        );
        assert_eq!(buf.len(), 16);
        let mut fresh = Vec::new();
        assert_eq!(
            append_capped(
                &mut fresh,
                &vec![0; cfg::MAX_BODY_BYTES + 1],
                cfg::MAX_BODY_BYTES
            )
            .unwrap_err(),
            TAG_TOO_LARGE
        );
    }

    #[test]
    fn source_deserializes_lowercase() {
        let s: SshKeySource = serde_json::from_str("\"gitlab\"").unwrap();
        assert_eq!(s, SshKeySource::Gitlab);
        assert!(serde_json::from_str::<SshKeySource>("\"GitHub\"").is_err());
    }
}

// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Network and API faults, applied at the command layer so the on-disk API cache is never overwritten.

use std::time::Duration;

use super::model::{ApiFault, DownloadFault, NetworkSim};
use super::state;
use super::MODULE;
use crate::config::dev::SIM_EXPECTED_SHA;
use crate::download::sha_mismatch_err;
use crate::log_warn;
use crate::utils::KB;

#[derive(Debug, PartialEq, Eq)]
pub enum ApiAction {
    Proceed { delay_ms: u64 },
    Empty,
}

/// Err mimics the message of the real fetch.
pub fn api_action(net: &NetworkSim, what: &str) -> Result<ApiAction, String> {
    match net.api {
        ApiFault::Normal => Ok(ApiAction::Proceed { delay_ms: 0 }),
        ApiFault::Slow => Ok(ApiAction::Proceed {
            delay_ms: net.api_delay_ms,
        }),
        ApiFault::Empty => Ok(ApiAction::Empty),
        ApiFault::Offline => Err(format!(
            "Failed to fetch {what}: error sending request (simulated offline)"
        )),
        ApiFault::ServerError => Err(format!(
            "{what} API returned error: HTTP status server error (500 Internal Server Error) (simulated)"
        )),
    }
}

pub fn connectivity(net: &NetworkSim) -> Option<bool> {
    (net.api == ApiFault::Offline).then_some(false)
}

fn download_offline(net: &NetworkSim) -> bool {
    net.api == ApiFault::Offline || net.download == DownloadFault::Offline
}

fn download_bytes_per_sec(net: &NetworkSim) -> Option<u64> {
    (net.download == DownloadFault::Slow).then(|| u64::from(net.download_kb_per_sec) * KB)
}

/// Cached images never get here.
pub fn check_download_start() -> Result<(), String> {
    if download_offline(&state::view().scenario.network) {
        log_warn!(MODULE, "Simulated offline download");
        return Err("Failed to start download: error sending request (simulated offline)".into());
    }
    Ok(())
}

pub struct Throttle(Option<u64>);

pub fn download_throttle() -> Throttle {
    Throttle(download_bytes_per_sec(&state::view().scenario.network))
}

impl Throttle {
    pub async fn pace(&self, bytes: usize) {
        if let Some(rate) = self.0 {
            tokio::time::sleep(Duration::from_secs_f64(bytes as f64 / rate as f64)).await;
        }
    }
}

/// The caller then deletes only its temp file.
pub fn override_sha(net: &NetworkSim, verified: Result<(), String>) -> Result<(), String> {
    match verified {
        Ok(()) if net.download == DownloadFault::ShaMismatch => Err(sha_mismatch_err(
            SIM_EXPECTED_SHA,
            "a different hash (simulated)",
        )),
        other => other,
    }
}

pub fn sha_result(verified: Result<(), String>) -> Result<(), String> {
    let net = state::view().scenario.network;
    let result = override_sha(&net, verified);
    if net.download == DownloadFault::ShaMismatch {
        log_warn!(MODULE, "Simulated SHA256 mismatch");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net(api: ApiFault, download: DownloadFault) -> NetworkSim {
        NetworkSim {
            api,
            download,
            ..Default::default()
        }
    }

    #[test]
    fn api_faults() {
        let n = |api| net(api, DownloadFault::Normal);
        assert_eq!(
            api_action(&n(ApiFault::Normal), "boards").unwrap(),
            ApiAction::Proceed { delay_ms: 0 }
        );
        assert_eq!(
            api_action(&n(ApiFault::Slow), "boards").unwrap(),
            ApiAction::Proceed {
                delay_ms: NetworkSim::default().api_delay_ms
            }
        );
        assert_eq!(
            api_action(&n(ApiFault::Empty), "boards").unwrap(),
            ApiAction::Empty
        );
        assert!(api_action(&n(ApiFault::Offline), "boards").is_err());
        assert!(api_action(&n(ApiFault::ServerError), "boards")
            .unwrap_err()
            .contains("500"));
    }

    #[test]
    fn offline_covers_connectivity_and_downloads() {
        let offline = net(ApiFault::Offline, DownloadFault::Normal);
        assert_eq!(connectivity(&offline), Some(false));
        assert!(download_offline(&offline));
        assert!(download_offline(&net(
            ApiFault::Normal,
            DownloadFault::Offline
        )));
        assert_eq!(
            connectivity(&net(ApiFault::ServerError, DownloadFault::Normal)),
            None
        );
        assert!(!download_offline(&NetworkSim::default()));
    }

    #[test]
    fn slow_download_rate() {
        assert_eq!(download_bytes_per_sec(&NetworkSim::default()), None);
        let slow = NetworkSim {
            download: DownloadFault::Slow,
            download_kb_per_sec: 2,
            ..Default::default()
        };
        assert_eq!(download_bytes_per_sec(&slow), Some(2048));
    }

    #[test]
    fn sha_mismatch_only_replaces_a_pass() {
        let bad = net(ApiFault::Normal, DownloadFault::ShaMismatch);
        assert!(override_sha(&bad, Ok(()))
            .unwrap_err()
            .starts_with(crate::download::TAG_SHA_MISMATCH));
        let unavailable = Err("[SHA_UNAVAILABLE] x".to_string());
        assert_eq!(override_sha(&bad, unavailable.clone()), unavailable);
        assert!(override_sha(&NetworkSim::default(), Ok(())).is_ok());
    }
}

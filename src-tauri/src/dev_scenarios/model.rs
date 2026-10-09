// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Scenario model shared by the dev panel, app-tester and the simulators (camelCase JSON).

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::commands::system::ArmbianReleaseInfo;
use crate::config::dev;
use crate::config::devices::BUS_TYPES;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct Scenario {
    pub hide_real_devices: bool,
    pub devices: Vec<FakeDevice>,
    pub edl_devices: Vec<FakeEdlDevice>,
    pub flash: FlashSim,
    pub network: NetworkSim,
    /// What `get_armbian_release` reports instead of reading /etc/armbian-release.
    pub armbian_host: Option<ArmbianReleaseInfo>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct FakeDevice {
    pub id: String,
    pub model: String,
    /// Must be 0 for a virtual disk, which takes its file's length.
    pub size_bytes: u64,
    pub bus_type: Option<String>,
    pub is_removable: bool,
    pub is_system: bool,
    pub is_read_only: bool,
    pub appear_after_ms: Option<u64>,
    pub disappear_after_ms: Option<u64>,
    pub vdisk: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct FakeEdlDevice {
    pub id: String,
    pub serial: String,
    pub description: String,
    pub appear_after_ms: Option<u64>,
    pub disappear_after_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlashOutcome {
    #[default]
    Success,
    WriteError,
    VerifyMismatch,
    Unplug,
    AuthDenied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct FlashSim {
    pub outcome: FlashOutcome,
    pub fail_at_percent: u8,
    pub write_mb_per_sec: u32,
    pub verify_mb_per_sec: u32,
    pub auth_delay_ms: u64,
}

impl Default for FlashSim {
    fn default() -> Self {
        Self {
            outcome: FlashOutcome::Success,
            fail_at_percent: dev::DEFAULT_FAIL_AT_PERCENT,
            write_mb_per_sec: dev::DEFAULT_WRITE_MB_PER_SEC,
            verify_mb_per_sec: dev::DEFAULT_VERIFY_MB_PER_SEC,
            auth_delay_ms: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApiFault {
    #[default]
    Normal,
    Offline,
    Slow,
    Empty,
    ServerError,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadFault {
    #[default]
    Normal,
    Offline,
    Slow,
    ShaMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct NetworkSim {
    pub api: ApiFault,
    pub api_delay_ms: u64,
    pub download: DownloadFault,
    pub download_kb_per_sec: u32,
}

impl Default for NetworkSim {
    fn default() -> Self {
        Self {
            api: ApiFault::Normal,
            api_delay_ms: dev::DEFAULT_API_DELAY_MS,
            download: DownloadFault::Normal,
            download_kb_per_sec: dev::DEFAULT_DOWNLOAD_KB_PER_SEC,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub scenario: Scenario,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VdiskInfo {
    pub id: String,
    pub size_bytes: u64,
    pub path: String,
}

/// `[a-z0-9-]{1,ID_MAX_LEN}`, the only ids that become `devsim://` paths or file names.
pub fn validate_id(id: &str) -> Result<(), String> {
    let ok = !id.is_empty()
        && id.len() <= dev::ID_MAX_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(format!(
            "invalid id {id:?}: use 1 to {} characters from a-z, 0-9 and -",
            dev::ID_MAX_LEN
        ))
    }
}

pub fn visible_at(appear: Option<u64>, disappear: Option<u64>, elapsed_ms: u64) -> bool {
    appear.is_none_or(|a| elapsed_ms >= a) && disappear.is_none_or(|d| elapsed_ms < d)
}

fn check_label(what: &str, id: &str, text: &str) -> Result<(), String> {
    if text.len() > dev::MAX_LABEL_LEN || text.chars().any(char::is_control) {
        return Err(format!(
            "{id}: {what} must be at most {} printable characters",
            dev::MAX_LABEL_LEN
        ));
    }
    Ok(())
}

fn check_window(id: &str, appear: Option<u64>, disappear: Option<u64>) -> Result<(), String> {
    for ms in [appear, disappear].into_iter().flatten() {
        if ms > dev::MAX_DELAY_MS {
            return Err(format!(
                "{id}: hot-plug timings are capped at {} ms",
                dev::MAX_DELAY_MS
            ));
        }
    }
    if let (Some(a), Some(d)) = (appear, disappear) {
        if a >= d {
            return Err(format!(
                "{id}: appearAfterMs must be lower than disappearAfterMs"
            ));
        }
    }
    Ok(())
}

impl FakeDevice {
    fn validate(&self) -> Result<(), String> {
        let id = &self.id;
        if id.starts_with(dev::EDL_ID_PREFIX) {
            return Err(format!(
                "{id}: the {:?} prefix is reserved for EDL devices",
                dev::EDL_ID_PREFIX
            ));
        }
        if self.vdisk {
            if self.size_bytes != 0 {
                return Err(format!(
                    "{id}: a virtual disk takes its size from its file, sizeBytes must be 0"
                ));
            }
        } else if self.size_bytes == 0 || self.size_bytes % dev::SIM_SECTOR_SIZE != 0 {
            return Err(format!(
                "{id}: sizeBytes must be a positive multiple of {}",
                dev::SIM_SECTOR_SIZE
            ));
        }
        if let Some(bus) = &self.bus_type {
            if !BUS_TYPES.contains(&bus.as_str()) {
                return Err(format!("{id}: busType must be one of {BUS_TYPES:?}"));
            }
        }
        check_label("model", id, &self.model)?;
        check_window(id, self.appear_after_ms, self.disappear_after_ms)
    }
}

impl FakeEdlDevice {
    fn validate(&self) -> Result<(), String> {
        let id = &self.id;
        if !id.starts_with(dev::EDL_ID_PREFIX) {
            return Err(format!(
                "{id}: EDL device ids must start with {:?}",
                dev::EDL_ID_PREFIX
            ));
        }
        check_label("serial", id, &self.serial)?;
        check_label("description", id, &self.description)?;
        check_window(id, self.appear_after_ms, self.disappear_after_ms)
    }
}

fn check_armbian_host(host: &ArmbianReleaseInfo) -> Result<(), String> {
    let board = &host.board;
    if board.is_empty() || board.chars().any(char::is_whitespace) {
        return Err(format!(
            "armbianHost.board {board:?} must be a non-empty slug without spaces"
        ));
    }
    check_label("board", "armbianHost", board)?;
    check_label("board_name", "armbianHost", &host.board_name)
}

impl Scenario {
    pub fn is_active(&self) -> bool {
        *self != Self::default()
    }

    pub fn validate(&self) -> Result<(), String> {
        let count = self.devices.len() + self.edl_devices.len();
        if count > dev::MAX_FAKE_DEVICES {
            return Err(format!(
                "{count} fake devices, at most {} are allowed",
                dev::MAX_FAKE_DEVICES
            ));
        }

        let mut ids = HashSet::new();
        let all_ids = self
            .devices
            .iter()
            .map(|d| &d.id)
            .chain(self.edl_devices.iter().map(|d| &d.id));
        for id in all_ids {
            validate_id(id)?;
            if !ids.insert(id) {
                return Err(format!("duplicate device id {id:?}"));
            }
        }

        for device in &self.devices {
            device.validate()?;
        }
        for device in &self.edl_devices {
            device.validate()?;
        }

        let flash = &self.flash;
        if flash.fail_at_percent > dev::MAX_FAIL_AT_PERCENT {
            return Err(format!(
                "flash.failAtPercent must be at most {}",
                dev::MAX_FAIL_AT_PERCENT
            ));
        }
        for (name, speed) in [
            ("writeMbPerSec", flash.write_mb_per_sec),
            ("verifyMbPerSec", flash.verify_mb_per_sec),
        ] {
            if speed == 0 || speed > dev::MAX_MB_PER_SEC {
                return Err(format!(
                    "flash.{name} must be between 1 and {}",
                    dev::MAX_MB_PER_SEC
                ));
            }
        }
        if flash.auth_delay_ms > dev::MAX_DELAY_MS || self.network.api_delay_ms > dev::MAX_DELAY_MS
        {
            return Err(format!("delays are capped at {} ms", dev::MAX_DELAY_MS));
        }
        let kbps = self.network.download_kb_per_sec;
        if kbps == 0 || kbps > dev::MAX_DOWNLOAD_KB_PER_SEC {
            return Err(format!(
                "network.downloadKbPerSec must be between 1 and {}",
                dev::MAX_DOWNLOAD_KB_PER_SEC
            ));
        }
        if let Some(host) = &self.armbian_host {
            check_armbian_host(host)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sd(id: &str) -> FakeDevice {
        FakeDevice {
            id: id.to_string(),
            size_bytes: 31_914_983_424,
            ..Default::default()
        }
    }

    fn edl(id: &str) -> FakeEdlDevice {
        FakeEdlDevice {
            id: id.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn id_charset_and_length() {
        for ok in ["a", "sd-32g", "0", "edl-q6a", &"x".repeat(dev::ID_MAX_LEN)] {
            assert!(validate_id(ok).is_ok(), "{ok:?} refused");
        }
        for bad in [
            "",
            "SD",
            "sd_1",
            "sd/1",
            "../x",
            "sd 1",
            "sd.img",
            "é",
            &"x".repeat(dev::ID_MAX_LEN + 1),
        ] {
            assert!(validate_id(bad).is_err(), "{bad:?} accepted");
        }
    }

    #[test]
    fn hot_plug_window() {
        assert!(visible_at(None, None, 0));
        assert!(!visible_at(Some(1000), None, 999));
        assert!(visible_at(Some(1000), None, 1000));
        assert!(visible_at(None, Some(500), 499));
        assert!(!visible_at(None, Some(500), 500));
        assert!(visible_at(Some(10), Some(20), 15));
        assert!(!visible_at(Some(10), Some(20), 25));
    }

    #[test]
    fn default_scenario_is_inactive_and_valid() {
        let s = Scenario::default();
        assert!(!s.is_active());
        assert!(s.validate().is_ok());
    }

    #[test]
    fn json_is_camel_case_and_strict() {
        let parsed: Scenario = serde_json::from_value(json!({
            "hideRealDevices": true,
            "devices": [{ "id": "sd", "sizeBytes": 1024, "busType": "SD", "appearAfterMs": 5 }],
            "flash": { "outcome": "verifyMismatch" },
            "network": { "api": "serverError", "download": "shaMismatch" }
        }))
        .unwrap();
        assert!(parsed.hide_real_devices);
        assert_eq!(parsed.devices[0].appear_after_ms, Some(5));
        assert_eq!(parsed.flash.outcome, FlashOutcome::VerifyMismatch);
        assert_eq!(parsed.flash.write_mb_per_sec, dev::DEFAULT_WRITE_MB_PER_SEC);
        assert_eq!(parsed.network.api, ApiFault::ServerError);
        assert!(parsed.validate().is_ok());

        let out = serde_json::to_value(&parsed).unwrap();
        assert_eq!(out["devices"][0]["sizeBytes"], json!(1024));
        assert_eq!(out["edlDevices"], json!([]));

        assert!(serde_json::from_value::<Scenario>(json!({ "hide_real_devices": true })).is_err());
        assert!(serde_json::from_value::<Scenario>(
            json!({ "devices": [{ "id": "a", "path": "/dev/disk0" }] })
        )
        .is_err());
        assert!(
            serde_json::from_value::<Scenario>(json!({ "flash": { "outcome": "explode" } }))
                .is_err()
        );
    }

    #[test]
    fn refuses_duplicate_ids_across_kinds() {
        let s = Scenario {
            devices: vec![sd("a"), sd("a")],
            ..Default::default()
        };
        assert!(s.validate().unwrap_err().contains("duplicate"));
    }

    #[test]
    fn edl_prefix_is_reserved() {
        let block_with_prefix = Scenario {
            devices: vec![sd("edl-x")],
            ..Default::default()
        };
        assert!(block_with_prefix.validate().is_err());
        let edl_without_prefix = Scenario {
            edl_devices: vec![edl("q6a")],
            ..Default::default()
        };
        assert!(edl_without_prefix.validate().is_err());
        let ok = Scenario {
            devices: vec![sd("x")],
            edl_devices: vec![edl("edl-x")],
            ..Default::default()
        };
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn block_sizes_must_be_whole_sectors() {
        for size in [0, 1, 513] {
            let s = Scenario {
                devices: vec![FakeDevice {
                    size_bytes: size,
                    ..sd("a")
                }],
                ..Default::default()
            };
            assert!(s.validate().is_err(), "{size} accepted");
        }
    }

    #[test]
    fn vdisk_size_comes_from_the_file() {
        let vdisk = FakeDevice {
            vdisk: true,
            size_bytes: 0,
            ..sd("v")
        };
        let ok = Scenario {
            devices: vec![vdisk.clone()],
            ..Default::default()
        };
        assert!(ok.validate().is_ok());
        let sized = Scenario {
            devices: vec![FakeDevice {
                size_bytes: 512,
                ..vdisk
            }],
            ..Default::default()
        };
        assert!(sized.validate().is_err());
    }

    #[test]
    fn refuses_bad_limits() {
        let mut s = Scenario {
            devices: vec![sd("a")],
            ..Default::default()
        };
        s.flash.fail_at_percent = 100;
        assert!(s.validate().is_err());
        s.flash.fail_at_percent = 40;
        s.flash.write_mb_per_sec = 0;
        assert!(s.validate().is_err());
        s.flash.write_mb_per_sec = 1;
        s.network.download_kb_per_sec = 0;
        assert!(s.validate().is_err());
        s.network.download_kb_per_sec = 1;
        s.devices[0].bus_type = Some("Floppy".to_string());
        assert!(s.validate().is_err());
        s.devices[0].bus_type = Some("USB".to_string());
        s.devices[0].appear_after_ms = Some(5);
        s.devices[0].disappear_after_ms = Some(5);
        assert!(s.validate().is_err());
        s.devices[0].disappear_after_ms = Some(6);
        assert!(s.validate().is_ok());
        s.devices[0].model = "x\n".to_string();
        assert!(s.validate().is_err());
    }

    fn host(board: &str, board_name: &str) -> Scenario {
        Scenario {
            armbian_host: Some(ArmbianReleaseInfo {
                board: board.to_string(),
                board_name: board_name.to_string(),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn armbian_host_activates_the_scenario() {
        let s = host("orangepi-5", "Orange Pi 5");
        assert!(s.is_active());
        assert!(s.validate().is_ok());
        assert!(host("rock-5b", "").validate().is_ok());
    }

    #[test]
    fn armbian_host_refuses_bad_values() {
        for (board, name) in [
            ("", "Orange Pi 5"),
            ("orange pi", "Orange Pi 5"),
            ("orangepi-5\n", "Orange Pi 5"),
            ("orangepi-5", "Orange\u{7}Pi"),
        ] {
            assert!(host(board, name).validate().is_err(), "{board:?} accepted");
        }
        let long = "x".repeat(dev::MAX_LABEL_LEN + 1);
        assert!(host(&long, "x").validate().is_err());
        assert!(host("x", &long).validate().is_err());
    }

    #[test]
    fn armbian_host_json_matches_the_release_info_shape() {
        let parsed: Scenario = serde_json::from_value(json!({
            "armbianHost": { "board": "rock-5b", "board_name": "Radxa ROCK 5B" }
        }))
        .unwrap();
        assert_eq!(parsed, host("rock-5b", "Radxa ROCK 5B"));
        assert_eq!(
            serde_json::to_value(Scenario::default()).unwrap()["armbianHost"],
            json!(null)
        );
        assert!(serde_json::from_value::<Scenario>(
            json!({ "armbianHost": { "boardName": "Radxa ROCK 5B" } })
        )
        .is_err());
    }

    #[test]
    fn caps_the_device_count() {
        let s = Scenario {
            devices: (0..=dev::MAX_FAKE_DEVICES)
                .map(|i| sd(&format!("d{i}")))
                .collect(),
            ..Default::default()
        };
        assert!(s.validate().is_err());
    }
}

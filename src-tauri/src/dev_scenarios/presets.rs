// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Ready-made scenarios, served to the dev panel and app-tester by `dev_scenarios_status`.

use crate::config::dev::{
    PRESET_AUTH_DENIED_DELAY_MS, PRESET_HOT_PLUG_APPEAR_MS, PRESET_HOT_PLUG_DISAPPEAR_MS,
    PRESET_NVME_512G_BYTES, PRESET_SD_1G_BYTES, PRESET_SD_32G_BYTES, PRESET_SD_8G_BYTES,
    PRESET_SLOW_DOWNLOAD_KB_PER_SEC, PRESET_SLOW_VERIFY_MB_PER_SEC, PRESET_SLOW_WRITE_MB_PER_SEC,
    PRESET_USB_16G_BYTES,
};
use crate::config::devices::{BUS_NVME, BUS_SD, BUS_USB};

use super::model::{
    ApiFault, DownloadFault, FakeDevice, FakeEdlDevice, FlashOutcome, FlashSim, NetworkSim, Preset,
    Scenario,
};

fn sd(id: &str, size: u64, model: &str) -> FakeDevice {
    FakeDevice {
        id: id.to_string(),
        model: model.to_string(),
        size_bytes: size,
        bus_type: Some(BUS_SD.to_string()),
        is_removable: true,
        ..Default::default()
    }
}

fn sd_card() -> FakeDevice {
    sd("sd-32g", PRESET_SD_32G_BYTES, "Simulated SD card")
}

fn usb_stick() -> FakeDevice {
    FakeDevice {
        id: "usb-16g".to_string(),
        model: "Simulated USB stick".to_string(),
        size_bytes: PRESET_USB_16G_BYTES,
        bus_type: Some(BUS_USB.to_string()),
        is_removable: true,
        ..Default::default()
    }
}

fn system_nvme() -> FakeDevice {
    FakeDevice {
        id: "nvme-system".to_string(),
        model: "Simulated internal NVMe".to_string(),
        size_bytes: PRESET_NVME_512G_BYTES,
        bus_type: Some(BUS_NVME.to_string()),
        is_system: true,
        ..Default::default()
    }
}

fn edl_board() -> FakeEdlDevice {
    FakeEdlDevice {
        id: "edl-q6a".to_string(),
        serial: "SIM00001".to_string(),
        description: "Simulated Qualcomm EDL device (Dragon Q6A)".to_string(),
        ..Default::default()
    }
}

fn devices(list: Vec<FakeDevice>) -> Scenario {
    Scenario {
        hide_real_devices: true,
        devices: list,
        ..Default::default()
    }
}

fn sd_with(outcome: FlashOutcome) -> Scenario {
    Scenario {
        flash: FlashSim {
            outcome,
            ..Default::default()
        },
        ..devices(vec![sd_card()])
    }
}

fn edl_with(outcome: FlashOutcome) -> Scenario {
    Scenario {
        hide_real_devices: true,
        edl_devices: vec![edl_board()],
        flash: FlashSim {
            outcome,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn network(api: ApiFault, download: DownloadFault, download_kb_per_sec: u32) -> Scenario {
    Scenario {
        network: NetworkSim {
            api,
            download,
            download_kb_per_sec,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn presets() -> Vec<Preset> {
    let default_kbps = NetworkSim::default().download_kb_per_sec;
    vec![
        Preset {
            id: "clean",
            label: "No simulation",
            description: "Real devices, real flashing, real network.",
            scenario: Scenario::default(),
        },
        Preset {
            id: "sd-card",
            label: "One SD card",
            description: "A 32 GB SD card; real devices hidden; flash succeeds.",
            scenario: devices(vec![sd_card()]),
        },
        Preset {
            id: "mixed-devices",
            label: "SD, USB and system disk",
            description: "SD card, USB stick and an internal NVMe system disk.",
            scenario: devices(vec![sd_card(), usb_stick(), system_nvme()]),
        },
        Preset {
            id: "read-only-sd",
            label: "Write-protected SD",
            description: "An 8 GB SD card with the lock switch on.",
            scenario: devices(vec![FakeDevice {
                is_read_only: true,
                ..sd(
                    "sd-locked",
                    PRESET_SD_8G_BYTES,
                    "Simulated SD card (locked)",
                )
            }]),
        },
        Preset {
            id: "too-small",
            label: "Card too small",
            description: "A 1 GiB SD card, smaller than any Armbian image.",
            scenario: devices(vec![sd(
                "sd-1g",
                PRESET_SD_1G_BYTES,
                "Simulated SD card (1 GiB)",
            )]),
        },
        Preset {
            id: "hot-plug",
            label: "Hot-plug",
            description: "The USB stick leaves after 8 s, the SD card arrives after 4 s.",
            scenario: devices(vec![
                FakeDevice {
                    disappear_after_ms: Some(PRESET_HOT_PLUG_DISAPPEAR_MS),
                    ..usb_stick()
                },
                FakeDevice {
                    appear_after_ms: Some(PRESET_HOT_PLUG_APPEAR_MS),
                    ..sd_card()
                },
            ]),
        },
        Preset {
            id: "write-error",
            label: "Write error",
            description: "The write fails with an I/O error at 40%.",
            scenario: sd_with(FlashOutcome::WriteError),
        },
        Preset {
            id: "verify-mismatch",
            label: "Verify mismatch",
            description: "The write succeeds, verification finds different data at 40%.",
            scenario: sd_with(FlashOutcome::VerifyMismatch),
        },
        Preset {
            id: "unplug-mid-write",
            label: "Unplugged mid-write",
            description: "The card disappears at 40% of the write.",
            scenario: sd_with(FlashOutcome::Unplug),
        },
        Preset {
            id: "auth-denied",
            label: "Authorization denied",
            description: "The system prompt is cancelled after 1.5 s.",
            scenario: Scenario {
                flash: FlashSim {
                    outcome: FlashOutcome::AuthDenied,
                    auth_delay_ms: PRESET_AUTH_DENIED_DELAY_MS,
                    ..Default::default()
                },
                ..devices(vec![sd_card()])
            },
        },
        Preset {
            id: "slow-flash",
            label: "Slow card",
            description: "Writes at 4 MB/s and verifies at 8 MB/s.",
            scenario: Scenario {
                flash: FlashSim {
                    write_mb_per_sec: PRESET_SLOW_WRITE_MB_PER_SEC,
                    verify_mb_per_sec: PRESET_SLOW_VERIFY_MB_PER_SEC,
                    ..Default::default()
                },
                ..devices(vec![sd_card()])
            },
        },
        Preset {
            id: "edl-device",
            label: "EDL board",
            description: "A Qualcomm board in EDL mode; real EDL devices hidden; flash succeeds.",
            scenario: edl_with(FlashOutcome::Success),
        },
        Preset {
            id: "edl-unplug",
            label: "EDL board unplugged",
            description: "The EDL board disconnects at 40% of the write.",
            scenario: edl_with(FlashOutcome::Unplug),
        },
        Preset {
            id: "offline",
            label: "Offline",
            description: "API, connectivity check and downloads all fail.",
            scenario: network(ApiFault::Offline, DownloadFault::Normal, default_kbps),
        },
        Preset {
            id: "slow-network",
            label: "Slow network",
            description: "API calls wait 3 s, downloads crawl at 256 KB/s.",
            scenario: network(
                ApiFault::Slow,
                DownloadFault::Slow,
                PRESET_SLOW_DOWNLOAD_KB_PER_SEC,
            ),
        },
        Preset {
            id: "empty-lists",
            label: "Empty lists",
            description: "Vendors, boards and images come back empty.",
            scenario: network(ApiFault::Empty, DownloadFault::Normal, default_kbps),
        },
        Preset {
            id: "server-error",
            label: "Server error",
            description: "Vendors, boards and images fail with HTTP 500.",
            scenario: network(ApiFault::ServerError, DownloadFault::Normal, default_kbps),
        },
        Preset {
            id: "sha-mismatch",
            label: "SHA mismatch",
            description: "A fresh download fails its checksum and is deleted.",
            scenario: network(ApiFault::Normal, DownloadFault::ShaMismatch, default_kbps),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_preset_validates_and_ids_are_unique() {
        let all = presets();
        let mut ids = HashSet::new();
        for preset in &all {
            assert!(ids.insert(preset.id), "duplicate preset {}", preset.id);
            preset
                .scenario
                .validate()
                .unwrap_or_else(|e| panic!("{}: {e}", preset.id));
        }
        assert_eq!(all[0].id, "clean");
        assert!(!all[0].scenario.is_active());
    }

    #[test]
    fn device_presets_hide_real_devices() {
        for preset in presets() {
            let s = &preset.scenario;
            if !s.devices.is_empty() || !s.edl_devices.is_empty() {
                assert!(s.hide_real_devices, "{} shows real devices", preset.id);
            }
        }
    }

    #[test]
    fn presets_serialize_camel_case() {
        let json = serde_json::to_value(presets()).unwrap();
        assert_eq!(json[1]["scenario"]["hideRealDevices"], true);
        assert_eq!(
            json[1]["scenario"]["devices"][0]["sizeBytes"],
            PRESET_SD_32G_BYTES
        );
    }
}

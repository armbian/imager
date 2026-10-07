// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Fake block and EDL devices: only `devsim://` entries are added; real devices pass untouched or are hidden.

use crate::config::dev;
use crate::config::flash::SIMULATED_DEVICE_PREFIX;
use crate::devices::target::device_not_found_error;
use crate::devices::{BlockDevice, TAG_INVALID_PATH};
use crate::qdl::{QdlDevice, TAG_QDL_DEVICE_NOT_FOUND};
use crate::utils::format_size;

use super::model::{validate_id, FakeDevice, FakeEdlDevice};
use super::state::View;

pub fn sim_path(id: &str) -> String {
    format!("{SIMULATED_DEVICE_PREFIX}{id}")
}

pub fn parse_sim_path(path: &str) -> Result<&str, String> {
    path.strip_prefix(SIMULATED_DEVICE_PREFIX)
        .ok_or_else(|| format!("{TAG_INVALID_PATH} {path:?} is not a simulated device path"))
        .and_then(|id| {
            validate_id(id)
                .map(|_| id)
                .map_err(|e| format!("{TAG_INVALID_PATH} {path:?}: {e}"))
        })
}

fn present(view: &View, id: &str, appear: Option<u64>, disappear: Option<u64>) -> bool {
    super::model::visible_at(appear, disappear, view.elapsed_ms) && !view.unplugged.contains(id)
}

fn to_block(device: &FakeDevice, size: u64) -> BlockDevice {
    BlockDevice {
        path: sim_path(&device.id),
        name: device.id.clone(),
        size,
        size_formatted: format_size(size),
        model: if device.model.is_empty() {
            dev::DEFAULT_MODEL.to_string()
        } else {
            device.model.clone()
        },
        is_removable: device.is_removable,
        is_system: device.is_system,
        bus_type: device.bus_type.clone(),
        is_read_only: device.is_read_only,
    }
}

pub fn find_block_device(
    view: &View,
    id: &str,
    vdisk_size: impl Fn(&str) -> Option<u64>,
) -> Option<(FakeDevice, BlockDevice)> {
    let device = view.scenario.devices.iter().find(|d| d.id == id)?;
    if !present(view, id, device.appear_after_ms, device.disappear_after_ms) {
        return None;
    }
    let size = if device.vdisk {
        vdisk_size(id)?
    } else {
        device.size_bytes
    };
    Some((device.clone(), to_block(device, size)))
}

pub fn list_block_devices(
    view: &View,
    real: impl FnOnce() -> Result<Vec<BlockDevice>, String>,
    vdisk_size: impl Fn(&str) -> Option<u64>,
) -> Result<Vec<BlockDevice>, String> {
    let mut devices = if view.scenario.hide_real_devices {
        Vec::new()
    } else {
        real()?
    };
    devices.extend(
        view.scenario
            .devices
            .iter()
            .filter_map(|d| find_block_device(view, &d.id, &vdisk_size).map(|(_, block)| block)),
    );
    Ok(devices)
}

/// A `devsim://` path never falls through to real I/O; a real one is refused while hidden.
pub fn route_block(
    view: &View,
    path: &str,
    vdisk_size: impl Fn(&str) -> Option<u64>,
) -> Result<Option<(FakeDevice, BlockDevice)>, String> {
    if crate::flash::is_simulated_path(path) {
        let id = parse_sim_path(path)?;
        return find_block_device(view, id, vdisk_size)
            .map(Some)
            .ok_or_else(|| device_not_found_error(path));
    }
    if view.scenario.hide_real_devices {
        return Err(format!(
            "{TAG_INVALID_PATH} {path:?}: real devices are hidden by the dev scenario"
        ));
    }
    Ok(None)
}

fn to_qdl(device: &FakeEdlDevice, address: u8) -> QdlDevice {
    QdlDevice {
        path: sim_path(&device.id),
        serial: device.serial.clone(),
        bus_id: dev::EDL_BUS_ID.to_string(),
        device_address: address,
        description: if device.description.is_empty() {
            dev::DEFAULT_EDL_DESCRIPTION.to_string()
        } else {
            device.description.clone()
        },
    }
}

pub fn real_edl_blocked(view: &View) -> bool {
    view.scenario.hide_real_devices || !view.scenario.edl_devices.is_empty()
}

pub fn find_edl_device(view: &View, id: &str) -> Option<QdlDevice> {
    let (index, device) = view
        .scenario
        .edl_devices
        .iter()
        .enumerate()
        .find(|(_, d)| d.id == id)?;
    present(view, id, device.appear_after_ms, device.disappear_after_ms)
        .then(|| to_qdl(device, u8::try_from(index + 1).unwrap_or(u8::MAX)))
}

pub fn route_edl(view: &View, path: &str) -> Result<Option<QdlDevice>, String> {
    if crate::flash::is_simulated_path(path) {
        let id = parse_sim_path(path)?;
        return find_edl_device(view, id).map(Some).ok_or_else(|| {
            format!("{TAG_QDL_DEVICE_NOT_FOUND} {path:?} is not a connected EDL device")
        });
    }
    if real_edl_blocked(view) {
        return Err(format!(
            "{TAG_INVALID_PATH} {path:?}: real EDL devices are hidden by the dev scenario"
        ));
    }
    Ok(None)
}

pub fn edl_override(view: &View) -> Option<Vec<QdlDevice>> {
    real_edl_blocked(view).then(|| {
        view.scenario
            .edl_devices
            .iter()
            .filter_map(|d| find_edl_device(view, &d.id))
            .collect()
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::dev_scenarios::model::Scenario;
    use crate::devices::target::TAG_NOT_FOUND;
    use std::collections::BTreeSet;

    pub fn view(scenario: Scenario, elapsed_ms: u64) -> View {
        View {
            scenario,
            elapsed_ms,
            unplugged: BTreeSet::new(),
        }
    }

    pub fn fake(id: &str, size: u64) -> FakeDevice {
        FakeDevice {
            id: id.to_string(),
            size_bytes: size,
            bus_type: Some("SD".to_string()),
            is_removable: true,
            ..Default::default()
        }
    }

    pub fn real_disk() -> BlockDevice {
        BlockDevice {
            path: "/dev/disk4".to_string(),
            name: "disk4".to_string(),
            size: 64_021_856_256,
            size_formatted: "59.6 GB".to_string(),
            model: "Real reader".to_string(),
            is_removable: true,
            is_system: true,
            bus_type: Some("USB".to_string()),
            is_read_only: true,
        }
    }

    fn no_vdisk(_: &str) -> Option<u64> {
        None
    }

    #[test]
    fn appends_fakes_and_keeps_real_devices_untouched() {
        let v = view(
            Scenario {
                devices: vec![fake("sd", 1024)],
                ..Default::default()
            },
            0,
        );
        let list = list_block_devices(&v, || Ok(vec![real_disk()]), no_vdisk).unwrap();
        assert_eq!(list.len(), 2);
        let real = &list[0];
        let expected = real_disk();
        assert_eq!(real.path, expected.path);
        assert_eq!(real.size, expected.size);
        assert_eq!(real.is_system, expected.is_system);
        assert_eq!(real.is_read_only, expected.is_read_only);
        assert_eq!(real.bus_type, expected.bus_type);
        assert_eq!(list[1].path, "devsim://sd");
        assert_eq!(list[1].model, dev::DEFAULT_MODEL);
        assert_eq!(list[1].size_formatted, format_size(1024));
    }

    #[test]
    fn hidden_real_devices_are_not_scanned() {
        let v = view(
            Scenario {
                hide_real_devices: true,
                devices: vec![fake("sd", 1024)],
                ..Default::default()
            },
            0,
        );
        let list = list_block_devices(&v, || panic!("real scan ran"), no_vdisk).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].path, "devsim://sd");
    }

    #[test]
    fn inactive_scenario_is_the_real_scan() {
        let v = view(Scenario::default(), 0);
        let list = list_block_devices(&v, || Ok(vec![real_disk()]), no_vdisk).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list_block_devices(&v, || Err("scan failed".to_string()), no_vdisk).is_err());
    }

    #[test]
    fn hot_plug_and_unplug_change_the_list() {
        let scenario = Scenario {
            hide_real_devices: true,
            devices: vec![
                FakeDevice {
                    appear_after_ms: Some(1000),
                    ..fake("late", 1024)
                },
                FakeDevice {
                    disappear_after_ms: Some(1000),
                    ..fake("early", 1024)
                },
            ],
            ..Default::default()
        };
        let ids = |v: &View| -> Vec<String> {
            list_block_devices(v, || unreachable!(), no_vdisk)
                .unwrap()
                .into_iter()
                .map(|d| d.name)
                .collect()
        };
        assert_eq!(ids(&view(scenario.clone(), 0)), vec!["early"]);
        assert_eq!(ids(&view(scenario.clone(), 1000)), vec!["late"]);
        let mut unplugged = view(scenario, 1000);
        unplugged.unplugged.insert("late".to_string());
        assert!(ids(&unplugged).is_empty());
    }

    #[test]
    fn vdisk_devices_need_their_file() {
        let v = view(
            Scenario {
                devices: vec![FakeDevice {
                    vdisk: true,
                    ..fake("v", 0)
                }],
                ..Default::default()
            },
            0,
        );
        assert!(find_block_device(&v, "v", no_vdisk).is_none());
        let (_, block) = find_block_device(&v, "v", |_| Some(4096)).unwrap();
        assert_eq!(block.size, 4096);
    }

    fn route(v: &View, path: &str) -> Result<Option<String>, String> {
        route_block(v, path, no_vdisk).map(|r| r.map(|(_, block)| block.path))
    }

    #[test]
    fn simulated_paths_never_fall_through() {
        let empty = view(Scenario::default(), 0);
        for path in [
            "devsim://sd",
            "DEVSIM://sd",
            " devsim://sd",
            "devsim://",
            "devsim://SD",
            "devsim://sd/../x",
            "devsim://edl-q6a",
        ] {
            assert!(route(&empty, path).is_err(), "{path:?} fell through");
        }

        let v = view(
            Scenario {
                devices: vec![fake("sd", 1024)],
                edl_devices: vec![FakeEdlDevice {
                    id: "edl-q6a".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            0,
        );
        assert_eq!(
            route(&v, "devsim://sd").unwrap().as_deref(),
            Some("devsim://sd")
        );
        assert!(route(&v, "devsim://other")
            .unwrap_err()
            .starts_with(TAG_NOT_FOUND));
        assert!(route(&v, "DEVSIM://sd")
            .unwrap_err()
            .starts_with(TAG_INVALID_PATH));
        assert!(route(&v, "devsim://edl-q6a").is_err());
        assert_eq!(route(&v, "/dev/disk4").unwrap(), None);
    }

    #[test]
    fn absent_or_unplugged_fakes_are_not_found() {
        let scenario = Scenario {
            devices: vec![FakeDevice {
                appear_after_ms: Some(100),
                ..fake("sd", 1024)
            }],
            ..Default::default()
        };
        assert!(route(&view(scenario.clone(), 0), "devsim://sd").is_err());
        let mut gone = view(scenario, 100);
        assert!(route(&gone, "devsim://sd").is_ok());
        gone.unplugged.insert("sd".to_string());
        assert!(route(&gone, "devsim://sd").is_err());
    }

    #[test]
    fn hidden_real_devices_refuse_real_paths() {
        let v = view(
            Scenario {
                hide_real_devices: true,
                devices: vec![fake("sd", 1024)],
                ..Default::default()
            },
            0,
        );
        for path in ["/dev/disk4", "/dev/sdb", r"\\.\PhysicalDrive1", ""] {
            assert!(
                route(&v, path).unwrap_err().starts_with(TAG_INVALID_PATH),
                "{path:?} allowed"
            );
        }
        assert!(route(&v, "devsim://sd").is_ok());
    }

    #[test]
    fn edl_routing_blocks_real_boards_while_faked() {
        let fake_edl = view(
            Scenario {
                edl_devices: vec![FakeEdlDevice {
                    id: "edl-q6a".to_string(),
                    ..Default::default()
                }],
                devices: vec![fake("sd", 1024)],
                ..Default::default()
            },
            0,
        );
        assert_eq!(
            route_edl(&fake_edl, "devsim://edl-q6a")
                .unwrap()
                .unwrap()
                .path,
            "devsim://edl-q6a"
        );
        assert!(route_edl(&fake_edl, "devsim://edl-other")
            .unwrap_err()
            .starts_with(TAG_QDL_DEVICE_NOT_FOUND));
        assert!(route_edl(&fake_edl, "devsim://sd").is_err());
        assert!(route_edl(&fake_edl, "qdl://1/5").is_err());
        assert!(route_edl(&fake_edl, "DEVSIM://edl-q6a").is_err());

        let hidden = view(
            Scenario {
                hide_real_devices: true,
                ..Default::default()
            },
            0,
        );
        assert!(route_edl(&hidden, "qdl://1/5").is_err());
        assert!(route_edl(&view(Scenario::default(), 0), "qdl://1/5")
            .unwrap()
            .is_none());
        assert!(route_edl(&view(Scenario::default(), 0), "devsim://edl-q6a").is_err());
    }

    #[test]
    fn edl_override_only_when_real_edl_is_out_of_play() {
        assert!(edl_override(&view(Scenario::default(), 0)).is_none());

        let hidden = view(
            Scenario {
                hide_real_devices: true,
                ..Default::default()
            },
            0,
        );
        assert_eq!(edl_override(&hidden).unwrap().len(), 0);

        let fake_edl = view(
            Scenario {
                edl_devices: vec![FakeEdlDevice {
                    id: "edl-q6a".to_string(),
                    appear_after_ms: Some(500),
                    ..Default::default()
                }],
                ..Default::default()
            },
            0,
        );
        assert!(real_edl_blocked(&fake_edl));
        assert_eq!(edl_override(&fake_edl).unwrap().len(), 0);
        let later = View {
            elapsed_ms: 500,
            ..fake_edl
        };
        let devices = edl_override(&later).unwrap();
        assert_eq!(devices[0].path, "devsim://edl-q6a");
        assert_eq!(devices[0].description, dev::DEFAULT_EDL_DESCRIPTION);
    }
}

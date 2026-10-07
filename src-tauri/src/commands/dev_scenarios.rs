// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Debug-only dev scenarios commands; their presence is how the frontend detects a debug build.

use std::path::Path;
use std::sync::Arc;

use serde::Serialize;
use tauri::AppHandle;

use crate::autoconfig::{prepare_working_copy, AutoconfigConfig};
use crate::config::dev;
#[cfg(target_os = "macos")]
use crate::config::devices::BUS_USB;
use crate::dev_scenarios::model::{FakeDevice, Preset, Scenario, VdiskInfo};
use crate::dev_scenarios::network::{self, ApiAction};
#[cfg(target_os = "macos")]
use crate::dev_scenarios::vdisk;
use crate::dev_scenarios::{devices, flash_sim, presets, state, test_image, MODULE};
use crate::devices::target::check_target;
use crate::devices::BlockDevice;
use crate::flash::{check_capacity, FlashState};
use crate::qdl::QdlDevice;
use crate::utils::{app_cache_dir, autoconfig_temp_dir, image_size};
use crate::{log_error, log_info, log_warn};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevLimits {
    pub id_max_len: usize,
    pub max_fake_devices: usize,
    pub max_test_image_mb: u64,
    pub max_vdisk_mb: u64,
    pub max_delay_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevScenariosStatus {
    pub active: bool,
    pub scenario: Scenario,
    pub elapsed_ms: u64,
    pub unplugged: Vec<String>,
    pub presets: Vec<Preset>,
    pub limits: DevLimits,
    pub vdisk_supported: bool,
    pub vdisks: Vec<VdiskInfo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevTestImage {
    pub path: String,
    pub size_bytes: u64,
}

fn status() -> DevScenariosStatus {
    let view = state::view();
    DevScenariosStatus {
        active: view.scenario.is_active(),
        scenario: view.scenario,
        elapsed_ms: view.elapsed_ms,
        unplugged: view.unplugged.into_iter().collect(),
        presets: presets::presets(),
        limits: DevLimits {
            id_max_len: dev::ID_MAX_LEN,
            max_fake_devices: dev::MAX_FAKE_DEVICES,
            max_test_image_mb: dev::MAX_TEST_IMAGE_MB,
            max_vdisk_mb: dev::MAX_VDISK_MB,
            max_delay_ms: dev::MAX_DELAY_MS,
        },
        vdisk_supported: cfg!(target_os = "macos"),
        vdisks: list_vdisks(),
    }
}

#[cfg(target_os = "macos")]
fn list_vdisks() -> Vec<VdiskInfo> {
    vdisk::list_in(&app_cache_dir())
}

#[cfg(not(target_os = "macos"))]
fn list_vdisks() -> Vec<VdiskInfo> {
    Vec::new()
}

#[cfg(target_os = "macos")]
fn vdisk_size(id: &str) -> Option<u64> {
    vdisk::size_in(&app_cache_dir(), id)
}

#[cfg(not(target_os = "macos"))]
fn vdisk_size(_id: &str) -> Option<u64> {
    None
}

fn check_vdisks(scenario: &Scenario) -> Result<(), String> {
    for device in scenario.devices.iter().filter(|d| d.vdisk) {
        if !cfg!(target_os = "macos") {
            return Err(dev::VDISK_UNSUPPORTED.to_string());
        }
        if vdisk_size(&device.id).is_none() {
            return Err(format!(
                "{}: no virtual disk with this id, create it first",
                device.id
            ));
        }
    }
    Ok(())
}

fn install(scenario: Scenario) -> Result<Scenario, String> {
    scenario
        .validate()
        .and_then(|_| check_vdisks(&scenario))
        .map_err(|e| {
            log_error!(MODULE, "Scenario refused: {}", e);
            e
        })?;
    state::replace(scenario.clone());
    log_info!(
        MODULE,
        "Scenario set: active={}, hideRealDevices={}, {} fake device(s), {} fake EDL device(s), flash={:?}, api={:?}, download={:?}",
        scenario.is_active(),
        scenario.hide_real_devices,
        scenario.devices.len(),
        scenario.edl_devices.len(),
        scenario.flash.outcome,
        scenario.network.api,
        scenario.network.download
    );
    Ok(scenario)
}

#[tauri::command]
pub async fn dev_scenarios_status() -> Result<DevScenariosStatus, String> {
    Ok(status())
}

#[tauri::command]
pub async fn dev_set_scenario(scenario: Scenario) -> Result<Scenario, String> {
    install(scenario)
}

#[tauri::command]
pub async fn dev_reset() -> Result<Scenario, String> {
    log_info!(MODULE, "Resetting dev scenario");
    install(Scenario::default())
}

#[tauri::command]
pub async fn dev_make_test_image(
    size_mb: u64,
    unaligned: Option<bool>,
) -> Result<DevTestImage, String> {
    let unaligned = unaligned.unwrap_or(false);
    log_info!(
        MODULE,
        "Creating test image: {} MB (unaligned: {})",
        size_mb,
        unaligned
    );
    let path =
        test_image::make_test_image_in(&app_cache_dir(), size_mb, unaligned).map_err(|e| {
            log_error!(MODULE, "Test image failed: {}", e);
            format!("Failed to create test image: {e}")
        })?;
    let size_bytes = std::fs::metadata(&path)
        .map_err(|e| format!("Failed to stat test image: {e}"))?
        .len();
    log_info!(MODULE, "Test image ready: {}", path.display());
    Ok(DevTestImage {
        path: path.to_string_lossy().to_string(),
        size_bytes,
    })
}

/// Fresh flash validation never comes through here.
pub(crate) fn list_block_devices(
    real: impl FnOnce() -> Result<Vec<BlockDevice>, String>,
) -> Result<Vec<BlockDevice>, String> {
    devices::list_block_devices(&state::view(), real, vdisk_size)
}

pub(crate) fn qdl_devices_override() -> Option<Vec<QdlDevice>> {
    devices::edl_override(&state::view())
}

/// None lets a real path through; Some(Err) covers every refusal, unknown `devsim://` ids included.
fn route_write(device_path: &str) -> Option<Result<(FakeDevice, BlockDevice), String>> {
    match devices::route_block(&state::view(), device_path, vdisk_size) {
        Ok(None) => None,
        Ok(Some(target)) => Some(Ok(target)),
        Err(e) => {
            log_error!(MODULE, "Refusing target {:?}: {}", device_path, e);
            Some(Err(e))
        }
    }
}

pub(crate) async fn intercept_authorization(
    app: &AppHandle,
    device_path: &str,
    expected_size: u64,
) -> Option<Result<bool, String>> {
    let routed = route_write(device_path)?;
    Some(
        async {
            let (_, block) = routed?;
            check_target(
                &block,
                expected_size,
                super::operations::allow_system_devices(app),
            )
            .inspect_err(|e| log_error!(MODULE, "Refusing target {:?}: {}", device_path, e))?;
            let granted = flash_sim::simulate_authorization(&state::view().scenario.flash).await;
            log_info!(
                MODULE,
                "Simulated authorization for {}: {}",
                device_path,
                if granted { "granted" } else { "denied" }
            );
            Ok(granted)
        }
        .await,
    )
}

pub(crate) async fn intercept_flash(
    app: &AppHandle,
    image_path: &Path,
    device_path: &str,
    expected_size: u64,
    verify: bool,
    autoconfig: Option<&AutoconfigConfig>,
    flash_state: Arc<FlashState>,
) -> Option<Result<(), String>> {
    let routed = route_write(device_path)?;
    let result = async {
        let (device, block) = routed?;
        check_target(
            &block,
            expected_size,
            super::operations::allow_system_devices(app),
        )?;
        check_capacity(image_size(image_path)?, block.size)?;

        let working_copy = autoconfig
            .map(|config| prepare_working_copy(image_path, &autoconfig_temp_dir(), config))
            .transpose()?;
        let flash_path = working_copy.as_ref().map_or(image_path, |copy| copy.path());
        log_info!(
            MODULE,
            "Simulated flash: {} -> {} (verify: {}, outcome: {:?})",
            flash_path.display(),
            device_path,
            verify,
            state::view().scenario.flash.outcome
        );

        let result = simulated_write(&device, flash_path, verify, flash_state).await;
        drop(working_copy);
        result
    }
    .await;

    match &result {
        Ok(()) => log_info!(MODULE, "Simulated flash completed: {}", device_path),
        Err(e) => log_error!(MODULE, "Simulated flash failed for {}: {}", device_path, e),
    }
    Some(result)
}

async fn simulated_write(
    device: &FakeDevice,
    image_path: &Path,
    verify: bool,
    flash_state: Arc<FlashState>,
) -> Result<(), String> {
    if device.vdisk {
        return vdisk_write(&device.id, image_path, verify, flash_state).await;
    }
    let cfg = state::view().scenario.flash;
    flash_sim::simulate_block_flash(image_size(image_path)?, &cfg, verify, flash_state, || {
        state::mark_unplugged(&device.id)
    })
    .await
}

pub(crate) async fn intercept_qdl_flash(
    kind: flash_sim::QdlKind,
    image_path: &Path,
    device_path: &str,
    flash_state: Arc<FlashState>,
) -> Option<Result<(), String>> {
    let view = state::view();
    let routed = match devices::route_edl(&view, device_path) {
        Ok(None) => return None,
        Ok(Some(device)) => Ok(device),
        Err(e) => Err(e),
    };
    let result = async {
        let device = routed?;
        let image_size = image_size(image_path)?;
        log_info!(
            MODULE,
            "Simulated QDL {:?} flash: {} -> {} (outcome: {:?})",
            kind,
            image_path.display(),
            device.path,
            view.scenario.flash.outcome
        );
        let id = devices::parse_sim_path(&device.path)?;
        flash_sim::simulate_qdl_flash(kind, image_size, &view.scenario.flash, flash_state, || {
            state::mark_unplugged(id)
        })
        .await
    }
    .await;

    match &result {
        Ok(()) => log_info!(MODULE, "Simulated QDL flash completed: {}", device_path),
        Err(e) => log_error!(
            MODULE,
            "Simulated QDL flash refused or failed for {:?}: {}",
            device_path,
            e
        ),
    }
    Some(result)
}

/// The scenario's flash outcome does not apply to a virtual disk write.
#[cfg(target_os = "macos")]
async fn vdisk_write(
    id: &str,
    image_path: &Path,
    verify: bool,
    flash_state: Arc<FlashState>,
) -> Result<(), String> {
    let (file, path) = vdisk::open_in(&app_cache_dir(), id)?;
    let label = path.to_string_lossy().to_string();
    crate::flash::flash_to_vdisk(&image_path.to_path_buf(), &label, file, flash_state, verify).await
}

#[cfg(not(target_os = "macos"))]
async fn vdisk_write(
    _id: &str,
    _image_path: &Path,
    _verify: bool,
    _flash_state: Arc<FlashState>,
) -> Result<(), String> {
    Err(dev::VDISK_UNSUPPORTED.to_string())
}

#[cfg(target_os = "macos")]
fn vdisk_device(id: &str) -> FakeDevice {
    FakeDevice {
        id: id.to_string(),
        model: dev::VDISK_MODEL.to_string(),
        bus_type: Some(BUS_USB.to_string()),
        is_removable: true,
        vdisk: true,
        ..Default::default()
    }
}

/// Sparse and never overwritten; listed as `devsim://<id>`.
#[tauri::command]
pub async fn dev_create_vdisk(id: String, size_mb: u64) -> Result<DevScenariosStatus, String> {
    log_info!(MODULE, "Creating virtual disk {:?} ({} MB)", id, size_mb);
    #[cfg(target_os = "macos")]
    {
        let device = vdisk_device(&id);
        let mut prospective = state::view().scenario;
        prospective.devices.push(device.clone());
        prospective.validate()?;

        let path = vdisk::create_in(&app_cache_dir(), &id, size_mb).map_err(|e| {
            log_error!(MODULE, "Virtual disk refused: {}", e);
            e
        })?;
        if let Err(e) = state::try_modify(|s| {
            s.devices.push(device);
            Ok(())
        }) {
            log_error!(MODULE, "Virtual disk not listed: {}", e);
            if let Err(cleanup) = vdisk::delete_in(&app_cache_dir(), &id) {
                log_warn!(
                    MODULE,
                    "Failed to remove unlisted virtual disk: {}",
                    cleanup
                );
            }
            return Err(e);
        }
        log_info!(MODULE, "Virtual disk ready: {}", path.display());
        Ok(status())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (id, size_mb);
        Err(dev::VDISK_UNSUPPORTED.to_string())
    }
}

/// Same containment checks as opening the file.
#[tauri::command]
pub async fn dev_delete_vdisk(id: String) -> Result<DevScenariosStatus, String> {
    log_info!(MODULE, "Deleting virtual disk {:?}", id);
    #[cfg(target_os = "macos")]
    {
        vdisk::delete_in(&app_cache_dir(), &id).map_err(|e| {
            log_error!(MODULE, "Virtual disk delete refused: {}", e);
            e
        })?;
        state::try_modify(|s| {
            s.devices.retain(|d| !(d.vdisk && d.id == id));
            Ok(())
        })?;
        log_info!(MODULE, "Virtual disk deleted: {:?}", id);
        Ok(status())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = id;
        Err(dev::VDISK_UNSUPPORTED.to_string())
    }
}

/// API fault for one list command: Ok(true) means answer with an empty list, Err fails the call.
pub(crate) async fn api_returns_empty(what: &str) -> Result<bool, String> {
    match network::api_action(&state::view().scenario.network, what) {
        Ok(ApiAction::Proceed { delay_ms }) => {
            if delay_ms > 0 {
                log_warn!(MODULE, "Simulated slow API: {} waits {} ms", what, delay_ms);
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
            Ok(false)
        }
        Ok(ApiAction::Empty) => {
            log_warn!(MODULE, "Simulated empty {} list", what);
            Ok(true)
        }
        Err(e) => {
            log_warn!(MODULE, "Simulated API failure: {}", e);
            Err(e)
        }
    }
}

pub(crate) fn connectivity_override() -> Option<bool> {
    network::connectivity(&state::view().scenario.network)
}

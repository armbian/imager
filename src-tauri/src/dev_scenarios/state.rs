// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! The active scenario, its hot-plug clock and the devices unplugged by a simulated flash.

use std::collections::BTreeSet;
use std::sync::RwLock;
use std::time::Instant;

use once_cell::sync::Lazy;

use super::model::Scenario;

/// Simulators decide from this copy, never under the lock.
#[derive(Debug, Clone)]
pub struct View {
    pub scenario: Scenario,
    pub elapsed_ms: u64,
    pub unplugged: BTreeSet<String>,
}

struct Runtime {
    scenario: Scenario,
    started: Instant,
    unplugged: BTreeSet<String>,
}

static RUNTIME: Lazy<RwLock<Runtime>> = Lazy::new(|| {
    RwLock::new(Runtime {
        scenario: Scenario::default(),
        started: Instant::now(),
        unplugged: BTreeSet::new(),
    })
});

pub fn view() -> View {
    let rt = RUNTIME.read().unwrap_or_else(|p| p.into_inner());
    View {
        scenario: rt.scenario.clone(),
        elapsed_ms: u64::try_from(rt.started.elapsed().as_millis()).unwrap_or(u64::MAX),
        unplugged: rt.unplugged.clone(),
    }
}

/// Restarts the hot-plug clock.
pub fn replace(scenario: Scenario) {
    let mut rt = RUNTIME.write().unwrap_or_else(|p| p.into_inner());
    rt.scenario = scenario;
    rt.started = Instant::now();
    rt.unplugged.clear();
}

/// The device stays gone until the scenario is set again.
pub fn mark_unplugged(id: &str) {
    let mut rt = RUNTIME.write().unwrap_or_else(|p| p.into_inner());
    rt.unplugged.insert(id.to_string());
}

/// Keeps the hot-plug clock; an edit that leaves the scenario invalid is dropped.
#[cfg(target_os = "macos")]
pub fn try_modify(
    edit: impl FnOnce(&mut Scenario) -> Result<(), String>,
) -> Result<Scenario, String> {
    let mut rt = RUNTIME.write().unwrap_or_else(|p| p.into_inner());
    let mut next = rt.scenario.clone();
    edit(&mut next)?;
    next.validate()?;
    rt.scenario = next.clone();
    Ok(next)
}

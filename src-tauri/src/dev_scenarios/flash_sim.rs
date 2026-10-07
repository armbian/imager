// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Flash simulator: drives the shared FlashState like a real writer, without touching any device.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::config::dev;
use crate::flash::{write_failed_err, FlashState};
use crate::qdl::flash::{check_cancelled, update_qdl_stage};
use crate::qdl::{
    QDL_CANCELLED_ERROR, STAGE_COMPLETE, STAGE_CONFIGURING, STAGE_CONNECTING, STAGE_FIREHOSE,
    STAGE_PARTITION_PREFIX, STAGE_RESETTING, STAGE_SAHARA, TAG_QDL_DISCONNECTED, TAG_QDL_ERROR,
    UFS_PARTITION_LABEL,
};
use crate::utils::MB;

use super::model::{FlashOutcome, FlashSim};

pub enum Stop {
    Cancelled,
    FailedAt(u64),
}

pub fn fail_offset(total: u64, percent: u8) -> u64 {
    let raw = u128::from(total) * u128::from(percent) / 100;
    let raw = u64::try_from(raw).unwrap_or(total);
    raw - raw % dev::SIM_SECTOR_SIZE
}

pub async fn run_progress(
    base: u64,
    total: u64,
    mb_per_sec: u32,
    fail_at: Option<u64>,
    counter: &AtomicU64,
    cancelled: &AtomicBool,
) -> Result<(), Stop> {
    let step = (u64::from(mb_per_sec) * MB * dev::SIM_TICK_MS / 1000).max(1);
    let mut done = 0u64;
    loop {
        if cancelled.load(Ordering::SeqCst) {
            return Err(Stop::Cancelled);
        }
        if let Some(at) = fail_at {
            if done >= at {
                return Err(Stop::FailedAt(base + at));
            }
        }
        if done >= total {
            return Ok(());
        }
        let mut next = done.saturating_add(step).min(total);
        if let Some(at) = fail_at {
            next = next.min(at);
        }
        done = next;
        counter.store(base + done, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(dev::SIM_TICK_MS)).await;
    }
}

pub async fn simulate_authorization(cfg: &FlashSim) -> bool {
    if cfg.auth_delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(cfg.auth_delay_ms)).await;
    }
    cfg.outcome != FlashOutcome::AuthDenied
}

pub async fn simulate_block_flash(
    image_size: u64,
    cfg: &FlashSim,
    verify: bool,
    state: Arc<FlashState>,
    on_unplug: impl FnOnce(),
) -> Result<(), String> {
    state.reset();
    state.total_bytes.store(image_size, Ordering::SeqCst);
    let fail_at = fail_offset(image_size, cfg.fail_at_percent);

    let write_fail =
        matches!(cfg.outcome, FlashOutcome::WriteError | FlashOutcome::Unplug).then_some(fail_at);
    match run_progress(
        0,
        image_size,
        cfg.write_mb_per_sec,
        write_fail,
        &state.written_bytes,
        &state.is_cancelled,
    )
    .await
    {
        Ok(()) => {}
        Err(Stop::Cancelled) => return Err("Flash cancelled".to_string()),
        Err(Stop::FailedAt(at)) if cfg.outcome == FlashOutcome::Unplug => {
            on_unplug();
            return Err(write_failed_err(
                at,
                "Device not configured (os error 6), simulated",
            ));
        }
        Err(Stop::FailedAt(at)) => {
            return Err(write_failed_err(
                at,
                "Input/output error (os error 5), simulated",
            ));
        }
    }

    if !verify {
        return Ok(());
    }

    state.is_verifying.store(true, Ordering::SeqCst);
    state.verified_bytes.store(0, Ordering::SeqCst);
    let verify_fail = (cfg.outcome == FlashOutcome::VerifyMismatch).then_some(fail_at);
    match run_progress(
        0,
        image_size,
        cfg.verify_mb_per_sec,
        verify_fail,
        &state.verified_bytes,
        &state.is_cancelled,
    )
    .await
    {
        Ok(()) => Ok(()),
        Err(Stop::Cancelled) => Err("Verification cancelled".to_string()),
        Err(Stop::FailedAt(at)) => Err(format!("Verification failed: data mismatch at byte {at}")),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QdlKind {
    Tar,
    Ufs,
}

pub async fn simulate_qdl_flash(
    kind: QdlKind,
    image_size: u64,
    cfg: &FlashSim,
    state: Arc<FlashState>,
    on_unplug: impl FnOnce(),
) -> Result<(), String> {
    state.reset();
    state.qdl.is_active.store(true, Ordering::SeqCst);

    for stage in [STAGE_CONNECTING, STAGE_SAHARA, STAGE_CONFIGURING] {
        check_cancelled(&state)?;
        update_qdl_stage(&state, stage);
        tokio::time::sleep(Duration::from_millis(dev::QDL_STAGE_MS)).await;
    }
    update_qdl_stage(&state, STAGE_FIREHOSE);

    let partitions: &[&str] = match kind {
        QdlKind::Tar => &dev::QDL_SIM_PARTITIONS,
        QdlKind::Ufs => &[UFS_PARTITION_LABEL],
    };
    let count = partitions.len() as u64;
    state.qdl.partitions_total.store(count, Ordering::SeqCst);
    state.total_bytes.store(image_size, Ordering::SeqCst);

    let fail_at = matches!(cfg.outcome, FlashOutcome::WriteError | FlashOutcome::Unplug)
        .then(|| fail_offset(image_size, cfg.fail_at_percent));
    let mut base = 0u64;
    for (index, name) in partitions.iter().enumerate() {
        let index = index as u64;
        let part = if index + 1 == count {
            image_size - base
        } else {
            image_size / count
        };
        update_qdl_stage(&state, &format!("{STAGE_PARTITION_PREFIX}{name}"));
        let local_fail = fail_at
            .filter(|at| *at >= base && *at < base + part.max(1))
            .map(|at| at - base);
        match run_progress(
            base,
            part,
            cfg.write_mb_per_sec,
            local_fail,
            &state.written_bytes,
            &state.is_cancelled,
        )
        .await
        {
            Ok(()) => {}
            Err(Stop::Cancelled) => return Err(QDL_CANCELLED_ERROR.to_string()),
            Err(Stop::FailedAt(_)) if cfg.outcome == FlashOutcome::Unplug => {
                on_unplug();
                return Err(format!("{TAG_QDL_DISCONNECTED} simulated"));
            }
            Err(Stop::FailedAt(at)) => {
                return Err(format!(
                    "{TAG_QDL_ERROR} Firehose program NAKed at byte {at} (simulated)"
                ));
            }
        }
        base += part;
        state
            .qdl
            .partitions_written
            .store(index + 1, Ordering::SeqCst);
    }

    update_qdl_stage(&state, STAGE_RESETTING);
    tokio::time::sleep(Duration::from_millis(dev::QDL_STAGE_MS)).await;
    update_qdl_stage(&state, STAGE_COMPLETE);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    const FAST: u32 = dev::MAX_MB_PER_SEC;

    fn sim(outcome: FlashOutcome) -> FlashSim {
        FlashSim {
            outcome,
            fail_at_percent: 50,
            write_mb_per_sec: FAST,
            verify_mb_per_sec: FAST,
            auth_delay_ms: 0,
        }
    }

    async fn run(
        outcome: FlashOutcome,
        verify: bool,
    ) -> (Result<(), String>, Arc<FlashState>, bool) {
        let state = Arc::new(FlashState::new());
        let mut unplugged = false;
        let result = simulate_block_flash(8 * MB, &sim(outcome), verify, state.clone(), || {
            unplugged = true
        })
        .await;
        (result, state, unplugged)
    }

    #[test]
    fn fail_offset_is_sector_aligned() {
        assert_eq!(fail_offset(1000, 50), 0);
        assert_eq!(fail_offset(8 * MB, 50), 4 * MB);
        assert_eq!(fail_offset(1_000_003, 40) % dev::SIM_SECTOR_SIZE, 0);
        assert_eq!(fail_offset(u64::MAX, 99) % dev::SIM_SECTOR_SIZE, 0);
    }

    #[tokio::test]
    async fn success_writes_and_verifies_everything() {
        let (result, state, unplugged) = run(FlashOutcome::Success, true).await;
        assert!(result.is_ok());
        assert!(!unplugged);
        assert_eq!(state.total_bytes.load(Ordering::SeqCst), 8 * MB);
        assert_eq!(state.written_bytes.load(Ordering::SeqCst), 8 * MB);
        assert_eq!(state.verified_bytes.load(Ordering::SeqCst), 8 * MB);
        assert!(state.is_verifying.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn write_error_is_tagged_at_the_fail_offset() {
        let (result, state, unplugged) = run(FlashOutcome::WriteError, true).await;
        assert!(result
            .unwrap_err()
            .starts_with(&format!("[WRITE_FAILED:{}]", 4 * MB)));
        assert_eq!(state.written_bytes.load(Ordering::SeqCst), 4 * MB);
        assert!(!state.is_verifying.load(Ordering::SeqCst));
        assert!(!unplugged);
    }

    #[tokio::test]
    async fn unplug_fails_the_write_and_removes_the_device() {
        let (result, _, unplugged) = run(FlashOutcome::Unplug, false).await;
        assert!(result.unwrap_err().starts_with("[WRITE_FAILED:"));
        assert!(unplugged);
    }

    #[tokio::test]
    async fn verify_mismatch_only_when_verifying() {
        let (result, state, _) = run(FlashOutcome::VerifyMismatch, true).await;
        assert_eq!(
            result.unwrap_err(),
            format!("Verification failed: data mismatch at byte {}", 4 * MB)
        );
        assert_eq!(state.written_bytes.load(Ordering::SeqCst), 8 * MB);
        let (result, _, _) = run(FlashOutcome::VerifyMismatch, false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn cancel_stops_the_write() {
        let counter = AtomicU64::new(0);
        let cancelled = AtomicBool::new(true);
        let stop = run_progress(0, MB, 1, None, &counter, &cancelled).await;
        assert!(matches!(stop, Err(Stop::Cancelled)));
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    async fn run_qdl(
        kind: QdlKind,
        outcome: FlashOutcome,
    ) -> (Result<(), String>, Arc<FlashState>, bool) {
        let state = Arc::new(FlashState::new());
        let mut unplugged = false;
        let result = simulate_qdl_flash(kind, 9 * MB, &sim(outcome), state.clone(), || {
            unplugged = true
        })
        .await;
        (result, state, unplugged)
    }

    #[tokio::test]
    async fn qdl_success_reports_every_partition() {
        let (result, state, _) = run_qdl(QdlKind::Tar, FlashOutcome::Success).await;
        assert!(result.is_ok());
        assert!(state.qdl.is_active.load(Ordering::SeqCst));
        assert_eq!(state.qdl.partitions_total.load(Ordering::SeqCst), 3);
        assert_eq!(state.qdl.partitions_written.load(Ordering::SeqCst), 3);
        assert_eq!(state.written_bytes.load(Ordering::SeqCst), 9 * MB);
        assert_eq!(*state.qdl.stage.lock().unwrap(), STAGE_COMPLETE);

        let (result, state, _) = run_qdl(QdlKind::Ufs, FlashOutcome::VerifyMismatch).await;
        assert!(result.is_ok());
        assert_eq!(state.qdl.partitions_total.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn qdl_failures_are_tagged() {
        let (result, state, unplugged) = run_qdl(QdlKind::Tar, FlashOutcome::Unplug).await;
        assert!(result.unwrap_err().starts_with(TAG_QDL_DISCONNECTED));
        assert!(unplugged);
        assert_eq!(state.qdl.partitions_written.load(Ordering::SeqCst), 1);

        let (result, state, unplugged) = run_qdl(QdlKind::Ufs, FlashOutcome::WriteError).await;
        assert!(result.unwrap_err().starts_with(TAG_QDL_ERROR));
        assert!(!unplugged);
        assert_eq!(
            state.written_bytes.load(Ordering::SeqCst),
            fail_offset(9 * MB, 50)
        );
    }

    #[tokio::test]
    async fn auth_denied_is_a_refusal_not_an_error() {
        assert!(!simulate_authorization(&sim(FlashOutcome::AuthDenied)).await);
        assert!(simulate_authorization(&sim(FlashOutcome::WriteError)).await);
    }
}

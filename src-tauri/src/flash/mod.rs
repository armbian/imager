// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

//! Platform-specific image flashing: privilege escalation + raw device writing.
//! macOS uses authopen (Touch ID), Linux uses pkexec, Windows needs Administrator.

pub(crate) mod verify;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, OwnedMutexGuard};

#[cfg(target_os = "windows")]
use crate::devices::FlashTarget;
use crate::log_info;

/// QDL (Qualcomm EDL) progress state. Uses `std::sync::Mutex` because `qdl_flash`
/// runs in `spawn_blocking`.
pub struct QdlProgress {
    pub is_active: AtomicBool,
    /// Current stage (e.g., "sahara", "firehose", "partition:boot.img")
    pub stage: std::sync::Mutex<String>,
    pub partitions_total: AtomicU64,
    pub partitions_written: AtomicU64,
}

impl QdlProgress {
    pub fn new() -> Self {
        Self {
            is_active: AtomicBool::new(false),
            stage: std::sync::Mutex::new(String::new()),
            partitions_total: AtomicU64::new(0),
            partitions_written: AtomicU64::new(0),
        }
    }

    pub fn reset(&self) {
        self.is_active.store(false, Ordering::SeqCst);
        {
            let mut s = self.stage.lock().unwrap_or_else(|p| p.into_inner());
            *s = String::new();
        }
        self.partitions_total.store(0, Ordering::SeqCst);
        self.partitions_written.store(0, Ordering::SeqCst);
    }
}

// Each flash command takes the next number.
static GENERATION: AtomicU64 = AtomicU64::new(0);

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn current_generation() -> u64 {
    GENERATION.load(Ordering::SeqCst)
}

#[derive(Debug, PartialEq, Eq)]
pub enum SessionRefusal {
    Cancelled(u64),
    Busy(u64),
}

impl SessionRefusal {
    pub fn generation(&self) -> u64 {
        match self {
            SessionRefusal::Cancelled(g) | SessionRefusal::Busy(g) => *g,
        }
    }

    /// Message for the block-device path; QDL callers map `Cancelled` to their own error.
    pub fn message(&self) -> String {
        match self {
            SessionRefusal::Cancelled(_) => cancelled_err(),
            SessionRefusal::Busy(_) => crate::utils::tagged(
                crate::utils::TAG_FLASH_BUSY,
                crate::config::flash::BUSY_ERROR,
            ),
        }
    }
}

pub struct FlashSession {
    generation: u64,
    _slot: OwnedMutexGuard<()>,
}

struct WaitingGuard<'a>(&'a AtomicU64);

impl Drop for WaitingGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl FlashSession {
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

/// Flash progress state shared between frontend and backend
pub struct FlashState {
    pub total_bytes: AtomicU64,
    pub written_bytes: AtomicU64,
    pub verified_bytes: AtomicU64,
    pub is_verifying: AtomicBool,
    pub is_cancelled: AtomicBool,
    pub error: Mutex<Option<String>>,
    pub qdl: QdlProgress,
    prep_stage: std::sync::Mutex<Option<&'static str>>,
    slot: Arc<Mutex<()>>,
    session_generation: AtomicU64,
    cancelled_generation: AtomicU64,
    waiting: AtomicU64,
}

impl FlashState {
    pub fn new() -> Self {
        Self {
            total_bytes: AtomicU64::new(0),
            written_bytes: AtomicU64::new(0),
            verified_bytes: AtomicU64::new(0),
            is_verifying: AtomicBool::new(false),
            is_cancelled: AtomicBool::new(false),
            error: Mutex::new(None),
            qdl: QdlProgress::new(),
            prep_stage: std::sync::Mutex::new(None),
            slot: Arc::new(Mutex::new(())),
            session_generation: AtomicU64::new(0),
            cancelled_generation: AtomicU64::new(0),
            waiting: AtomicU64::new(0),
        }
    }

    /// Cancel every flash started so far, including one still waiting for its turn.
    pub fn cancel(&self) {
        // The global counter, so a flash that has just taken its number is covered too.
        self.cancelled_generation
            .fetch_max(GENERATION.load(Ordering::SeqCst), Ordering::SeqCst);
        self.is_cancelled.store(true, Ordering::SeqCst);
    }

    /// Wait for the previous flash to release the device, then clear progress.
    pub async fn begin_session(&self) -> Result<FlashSession, SessionRefusal> {
        self.begin_session_within(Duration::from_secs(crate::config::flash::BUSY_WAIT_SECS))
            .await
    }

    async fn begin_session_within(&self, wait: Duration) -> Result<FlashSession, SessionRefusal> {
        let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        self.waiting.fetch_add(1, Ordering::SeqCst);
        // Released after the progress reset, so a poll that sees no waiter never reads the previous flash.
        let _waiting = WaitingGuard(&self.waiting);
        let deadline = Instant::now() + wait;
        let mut waited = false;
        loop {
            if self.cancelled_generation.load(Ordering::SeqCst) >= generation {
                return Err(SessionRefusal::Cancelled(generation));
            }
            if let Ok(slot) = self.slot.clone().try_lock_owned() {
                self.session_generation.store(generation, Ordering::SeqCst);
                self.reset_progress();
                // Clear first, then re-check, so a cancel landing in between is never lost.
                self.is_cancelled.store(false, Ordering::SeqCst);
                if self.cancelled_generation.load(Ordering::SeqCst) >= generation {
                    self.is_cancelled.store(true, Ordering::SeqCst);
                }
                return Ok(FlashSession {
                    generation,
                    _slot: slot,
                });
            }
            if Instant::now() >= deadline {
                return Err(SessionRefusal::Busy(generation));
            }
            if !waited {
                waited = true;
                log_info!("flash", "Waiting for the previous flash to finish");
            }
            tokio::time::sleep(Duration::from_millis(crate::config::flash::BUSY_POLL_MS)).await;
        }
    }

    /// True while a flash waits for its turn; progress still describes the previous flash then.
    pub fn is_waiting(&self) -> bool {
        self.waiting.load(Ordering::SeqCst) > 0
    }

    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub fn session_generation(&self) -> u64 {
        self.session_generation.load(Ordering::SeqCst)
    }

    /// Writers call this instead of a full reset, so a cancel pressed before the write still stops it.
    pub fn reset_progress(&self) {
        self.total_bytes.store(0, Ordering::SeqCst);
        self.written_bytes.store(0, Ordering::SeqCst);
        self.verified_bytes.store(0, Ordering::SeqCst);
        self.is_verifying.store(false, Ordering::SeqCst);
        self.qdl.reset();
        self.set_prep_stage(None);
    }

    pub fn set_prep_stage(&self, stage: Option<&'static str>) {
        *self.prep_stage.lock().unwrap_or_else(|p| p.into_inner()) = stage;
    }

    pub fn prep_stage(&self) -> Option<&'static str> {
        *self.prep_stage.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn ensure_not_cancelled(&self) -> Result<(), String> {
        if self.is_cancelled.load(Ordering::SeqCst) {
            return Err(cancelled_err());
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub use linux::flash_image;
#[cfg(target_os = "macos")]
pub use macos::flash_image;
#[cfg(target_os = "windows")]
pub use windows::flash_image;

#[cfg(all(debug_assertions, target_os = "macos"))]
pub(crate) use macos::flash_to_vdisk;

/// Drop an authorization saved before flash `generation` began; a newer one belongs to a later flash.
pub fn discard_saved_authorization(generation: u64) {
    #[cfg(target_os = "macos")]
    macos::discard_saved_authorization(generation);
    #[cfg(not(target_os = "macos"))]
    let _ = generation;
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn saved_before(saved_at: u64, generation: u64) -> bool {
    saved_at < generation
}

#[cfg(target_os = "linux")]
pub use linux::request_authorization;
#[cfg(target_os = "macos")]
pub use macos::request_authorization;

/// Request authorization before flashing: Touch ID on macOS, pkexec re-launch
/// on Linux when not root, no-op on Windows.
#[cfg(target_os = "windows")]
pub fn request_authorization(target: &FlashTarget) -> Result<bool, String> {
    reject_simulated(target.path())?;
    Ok(true)
}

pub(crate) fn is_simulated_path(path: &str) -> bool {
    path.trim()
        .to_ascii_lowercase()
        .starts_with(crate::config::flash::SIMULATED_DEVICE_PREFIX)
}

/// Called again at every writer entry so a simulated path can never reach real device I/O.
pub(crate) fn reject_simulated(path: &str) -> Result<(), String> {
    if is_simulated_path(path) {
        // The prefix is formatted as data so the release binary check can find it.
        return Err(format!(
            "{} simulated device path refused: {path:?} ({} paths are never written)",
            crate::devices::TAG_INVALID_PATH,
            crate::config::flash::SIMULATED_DEVICE_PREFIX
        ));
    }
    Ok(())
}

pub(crate) fn check_capacity(image_size: u64, device_size: u64) -> Result<(), String> {
    // Device sizes are whole sectors, so this also covers the sector padding of the last write.
    if device_size == 0 || image_size > device_size {
        return Err(format!(
            "[DEVICE_TOO_SMALL:{image_size}:{device_size}] image needs {image_size} bytes, device has {device_size}"
        ));
    }
    Ok(())
}

/// Unmount a device before flashing (platform-specific)
#[allow(dead_code)]
pub(crate) fn unmount_device(device_path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("diskutil")
            .args(["unmountDisk", device_path])
            .output();
    }

    #[cfg(target_os = "linux")]
    {
        let output = Command::new("lsblk")
            .args(["-ln", "-o", "NAME", device_path])
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let part_path = format!("/dev/{}", line.trim());
                let _ = Command::new("umount").arg(&part_path).output();
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let _ = device_path;
    }

    Ok(())
}

/// Tagged device-write failure; the frontend maps `[WRITE_FAILED:<offset>]` to a translated message.
pub(crate) fn write_failed_err(offset: u64, e: impl std::fmt::Display) -> String {
    format!("[WRITE_FAILED:{}] {}", offset, e)
}

pub(crate) fn cancelled_err() -> String {
    crate::utils::tagged(
        crate::utils::TAG_CANCELLED,
        crate::config::flash::CANCELLED_ERROR,
    )
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn fsync_checked(fd: i32, written: u64) -> Result<(), String> {
    if unsafe { libc::fsync(fd) } != 0 {
        return Err(write_failed_err(written, std::io::Error::last_os_error()));
    }
    Ok(())
}

/// Write `total` zero bytes in `chunk_size` chunks, shared by the platform
/// quick_erase routines (which keep their own seek/sync).
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn write_zeros(
    device: &mut impl std::io::Write,
    total: usize,
    chunk_size: usize,
) -> Result<(), String> {
    let zero_buffer = vec![0u8; chunk_size];
    let mut erased: usize = 0;
    while erased < total {
        let to_write = std::cmp::min(chunk_size, total - erased);
        device
            .write_all(&zero_buffer[..to_write])
            .map_err(|e| format!("Quick erase failed at byte {}: {}", erased, e))?;
        erased += to_write;
    }
    Ok(())
}

/// Sync device to ensure all data is written to disk
#[allow(dead_code)]
pub(crate) fn sync_device(_device_path: &str) {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let _ = Command::new("sync").output();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_simulated_matches_any_case_and_padding() {
        for path in [
            "devsim://sd-32g",
            "DEVSIM://sd-32g",
            "DevSim://x",
            "  devsim://x ",
            "\tdevsim://x\n",
            "devsim://",
        ] {
            assert!(reject_simulated(path).is_err(), "{path:?} was accepted");
        }
        assert!(reject_simulated("devsim://x")
            .unwrap_err()
            .starts_with(crate::devices::TAG_INVALID_PATH));
    }

    #[test]
    fn reject_simulated_lets_real_paths_through() {
        for path in [
            "/dev/disk4",
            "/dev/sdb",
            r"\\.\PhysicalDrive1",
            "qdl://1/5",
            "",
            "/dev/devsim",
        ] {
            assert!(reject_simulated(path).is_ok(), "{path:?} was refused");
        }
    }

    #[test]
    fn reset_progress_keeps_a_pending_cancel() {
        let state = FlashState::new();
        state.written_bytes.store(42, Ordering::SeqCst);
        state.is_cancelled.store(true, Ordering::SeqCst);
        state.set_prep_stage(Some(crate::config::flash::PREP_STAGE_COPYING));
        state.reset_progress();
        assert_eq!(state.written_bytes.load(Ordering::SeqCst), 0);
        assert_eq!(state.prep_stage(), None);
        assert_eq!(
            state.ensure_not_cancelled().unwrap_err(),
            "[CANCELLED] Flash cancelled"
        );
    }

    fn spawn_session(
        state: &Arc<FlashState>,
        wait: Duration,
    ) -> tokio::task::JoinHandle<Result<FlashSession, SessionRefusal>> {
        let state = state.clone();
        tokio::spawn(async move { state.begin_session_within(wait).await })
    }

    #[tokio::test]
    async fn a_cancel_survives_a_new_flash_starting() {
        let state = Arc::new(FlashState::new());
        let old = state.begin_session().await.unwrap();
        state.cancel();
        let new = spawn_session(&state, Duration::from_secs(5));
        tokio::time::sleep(Duration::from_millis(200)).await;

        assert!(state.ensure_not_cancelled().is_err(), "old flash must stop");
        assert!(!new.is_finished(), "new flash waits for the old one");
        drop(old);
        let new = new.await.unwrap().unwrap();
        assert!(state.ensure_not_cancelled().is_ok());
        assert!(new.generation() > 0);
    }

    #[tokio::test]
    async fn a_cancel_while_waiting_stops_the_waiting_flash() {
        let state = Arc::new(FlashState::new());
        let old = state.begin_session().await.unwrap();
        let new = spawn_session(&state, Duration::from_secs(5));
        tokio::time::sleep(Duration::from_millis(200)).await;
        state.cancel();

        assert!(matches!(
            new.await.unwrap().err(),
            Some(SessionRefusal::Cancelled(_))
        ));
        assert!(state.ensure_not_cancelled().is_err());
        drop(old);
        assert!(state.begin_session().await.is_ok());
        assert!(state.ensure_not_cancelled().is_ok());
    }

    #[tokio::test]
    async fn a_waiting_flash_hides_the_previous_progress() {
        let state = Arc::new(FlashState::new());
        let old = state.begin_session().await.unwrap();
        state.written_bytes.store(42, Ordering::SeqCst);
        assert!(!state.is_waiting());
        let new = spawn_session(&state, Duration::from_secs(5));
        tokio::time::sleep(Duration::from_millis(200)).await;

        assert!(state.is_waiting());
        drop(old);
        let _new = new.await.unwrap().unwrap();
        assert!(!state.is_waiting());
        assert_eq!(state.written_bytes.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn a_refused_flash_stops_waiting() {
        let state = Arc::new(FlashState::new());
        let _old = state.begin_session().await.unwrap();
        let refusal = spawn_session(&state, Duration::from_millis(200))
            .await
            .unwrap();
        assert!(matches!(refusal.err(), Some(SessionRefusal::Busy(_))));
        assert!(!state.is_waiting());
    }

    #[tokio::test]
    async fn a_stuck_flash_refuses_the_next_one() {
        let state = Arc::new(FlashState::new());
        let _old = state.begin_session().await.unwrap();
        let new = spawn_session(&state, Duration::from_millis(300));
        let refusal = new.await.unwrap().err().unwrap();
        assert!(matches!(refusal, SessionRefusal::Busy(_)));
        assert!(refusal.message().starts_with("[FLASH_BUSY]"));
    }

    #[test]
    fn a_cancel_covers_a_flash_that_only_took_its_number() {
        let state = FlashState::new();
        let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        state.cancel();
        assert!(state.cancelled_generation.load(Ordering::SeqCst) >= generation);
    }

    #[test]
    fn only_an_authorization_older_than_the_flash_is_discarded() {
        assert!(saved_before(4, 5));
        assert!(!saved_before(5, 5));
        assert!(!saved_before(6, 5));
    }

    #[test]
    fn capacity_allows_exact_fit() {
        assert!(check_capacity(1024, 1024).is_ok());
        assert!(check_capacity(0, 1024).is_ok());
    }

    #[test]
    fn capacity_refuses_one_byte_over() {
        let err = check_capacity(1025, 1024).unwrap_err();
        assert!(err.starts_with("[DEVICE_TOO_SMALL:1025:1024]"));
    }

    #[test]
    fn capacity_refuses_unknown_device_size() {
        assert!(check_capacity(1, 0)
            .unwrap_err()
            .starts_with("[DEVICE_TOO_SMALL:1:0]"));
    }
}

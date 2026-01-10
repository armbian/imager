//! Progress tracking utility
//!
//! Provides a reusable progress tracker with speed calculation for
//! download, flash, verification, SHA256, and decompression operations.

use std::time::Instant;

use super::bytes_to_mb;
use crate::{log_debug, log_info};

/// Progress tracker for operations with speed calculation
pub struct ProgressTracker {
    /// Operation name for logging (e.g., "Download", "Flash", "Verify")
    operation_name: String,
    /// Module name for logging
    module_name: String,
    /// Total bytes to process
    total_bytes: u64,
    /// Bytes processed so far
    processed_bytes: u64,
    /// Start time of the operation
    start_time: Instant,
    /// Time of the last progress log
    last_log_time: Instant,
    /// Bytes processed at the last log
    last_log_bytes: u64,
    /// Interval in bytes between progress logs
    log_interval_bytes: u64,
}

/// Progress update data
pub struct ProgressUpdate {
    /// Current MB processed
    pub current_mb: f64,
    /// Total MB to process
    pub total_mb: f64,
    /// Percentage complete
    pub percent: f64,
    /// Current speed in MB/s
    pub speed_mbps: f64,
}

/// Final summary data
pub struct ProgressSummary {
    /// Total MB processed
    pub total_mb: f64,
    /// Total elapsed time in seconds
    pub elapsed_secs: f64,
    /// Average speed in MB/s
    pub avg_speed_mbps: f64,
}

impl ProgressTracker {
    /// Create a new progress tracker
    ///
    /// # Arguments
    /// * `operation` - Name of the operation (e.g., "Download", "Flash")
    /// * `module` - Module name for logging
    /// * `total_bytes` - Total bytes to process (0 if unknown)
    /// * `log_interval_mb` - Interval in MB between progress logs
    pub fn new(operation: &str, module: &str, total_bytes: u64, log_interval_mb: u64) -> Self {
        let now = Instant::now();
        Self {
            operation_name: operation.to_string(),
            module_name: module.to_string(),
            total_bytes,
            processed_bytes: 0,
            start_time: now,
            last_log_time: now,
            last_log_bytes: 0,
            log_interval_bytes: log_interval_mb * 1024 * 1024,
        }
    }

    /// Update progress and automatically log if interval reached
    ///
    /// Returns Some(ProgressUpdate) if it's time to log, None otherwise
    pub fn update(&mut self, bytes_added: u64) -> Option<ProgressUpdate> {
        self.processed_bytes += bytes_added;

        // Check if we've crossed a log interval boundary
        if self.log_interval_bytes == 0 {
            return None;
        }

        let current_interval = self.processed_bytes / self.log_interval_bytes;
        let last_interval = self.last_log_bytes / self.log_interval_bytes;

        if current_interval > last_interval {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_log_time).as_secs_f64();
            let bytes_since_last = self.processed_bytes - self.last_log_bytes;

            let speed_mbps = if elapsed > 0.0 {
                bytes_to_mb(bytes_since_last) / elapsed
            } else {
                0.0
            };

            self.last_log_time = now;
            self.last_log_bytes = self.processed_bytes;

            let update = ProgressUpdate {
                current_mb: bytes_to_mb(self.processed_bytes),
                total_mb: bytes_to_mb(self.total_bytes),
                percent: if self.total_bytes > 0 {
                    (self.processed_bytes as f64 / self.total_bytes as f64) * 100.0
                } else {
                    0.0
                },
                speed_mbps,
            };

            // Auto-log progress (debug level)
            if self.total_bytes > 0 {
                log_debug!(
                    &self.module_name,
                    "{} progress: {:.1} MB / {:.1} MB ({:.1}%) @ {:.1} MB/s",
                    self.operation_name,
                    update.current_mb,
                    update.total_mb,
                    update.percent,
                    update.speed_mbps
                );
            } else {
                log_debug!(
                    &self.module_name,
                    "{} progress: {:.1} MB @ {:.1} MB/s",
                    self.operation_name,
                    update.current_mb,
                    update.speed_mbps
                );
            }

            Some(update)
        } else {
            None
        }
    }

    /// Get final summary with average speed and log completion
    pub fn finish(&self) -> ProgressSummary {
        let total_elapsed = self.start_time.elapsed().as_secs_f64();
        let total_mb = bytes_to_mb(self.processed_bytes);

        let avg_speed = if total_elapsed > 0.0 {
            total_mb / total_elapsed
        } else {
            0.0
        };

        let summary = ProgressSummary {
            total_mb,
            elapsed_secs: total_elapsed,
            avg_speed_mbps: avg_speed,
        };

        log_info!(
            &self.module_name,
            "{} complete: {:.1} MB in {:.1}s (avg {:.1} MB/s)",
            self.operation_name,
            summary.total_mb,
            summary.elapsed_secs,
            summary.avg_speed_mbps
        );

        summary
    }
}

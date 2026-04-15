//! Application state module
//!
//! Defines the shared application state used across commands.

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::download::DownloadState;
use crate::flash::FlashState;
use crate::images::{ApiBoardSummary, ApiVendor};

/// Application state shared across all commands
pub struct AppState {
    /// Cached board list from the REST API
    pub boards: Mutex<Option<Vec<ApiBoardSummary>>>,
    /// Cached vendor list from the REST API
    pub vendors: Mutex<Option<Vec<ApiVendor>>>,
    pub download_state: Arc<DownloadState>,
    pub flash_state: Arc<FlashState>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            boards: Mutex::new(None),
            vendors: Mutex::new(None),
            download_state: Arc::new(DownloadState::new()),
            flash_state: Arc::new(FlashState::new()),
        }
    }
}

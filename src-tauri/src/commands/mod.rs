// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

//! Tauri command handlers organized by responsibility.

pub mod board_queries;
pub mod custom_image;
pub mod operations;
pub mod progress;
pub mod qdl_operations;
pub mod scraping;
pub mod settings;
mod state;
pub mod system;
pub mod update;

// Re-export state for use in main.rs
pub use state::AppState;

//! Commands module
//!
//! Tauri command handlers organized by responsibility.

pub mod board_queries;
pub mod custom_image;
pub mod operations;
pub mod progress;
pub mod scraping;
mod state;
pub mod system;
pub mod update;

// Re-export state for use in main.rs
pub use state::AppState;

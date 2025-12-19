//! Image data models
//!
//! Types representing Armbian images and boards.

use serde::{Deserialize, Serialize};

/// Raw Armbian image data from the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmbianImage {
    pub board_slug: Option<String>,
    pub board_name: Option<String>,
    pub armbian_version: Option<String>,
    pub distro_release: Option<String>,
    pub kernel_branch: Option<String>,
    pub image_variant: Option<String>,
    pub preinstalled_application: Option<String>,
    pub promoted: Option<String>,
    pub file_url: Option<String>,
    pub file_url_sha: Option<String>,
    pub file_extension: Option<String>,
    pub file_size: Option<String>,
    pub download_repository: Option<String>,
    pub redi_url: Option<String>,
}

/// Board information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardInfo {
    pub slug: String,
    pub name: String,
    pub image_count: usize,
    pub has_promoted: bool,
}

/// Processed image information for the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub armbian_version: String,
    pub distro_release: String,
    pub kernel_branch: String,
    pub image_variant: String,
    pub preinstalled_application: String,
    pub promoted: bool,
    pub file_url: String,
    pub file_url_sha: Option<String>,
    pub file_size: u64,
    pub download_repository: String,
}

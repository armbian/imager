//! Image data models
//!
//! Types representing Armbian images and boards.

use serde::{Deserialize, Serialize};

/// Raw Armbian image data from the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmbianImage {
    pub board_slug: Option<String>,
    pub board_name: Option<String>,
    pub board_vendor: Option<String>,
    pub company_name: Option<String>,
    pub company_logo: Option<String>,
    pub armbian_version: Option<String>,
    /// API field: "distro"
    #[serde(alias = "distro")]
    pub distro_release: Option<String>,
    /// API field: "branch"
    #[serde(alias = "branch")]
    pub kernel_branch: Option<String>,
    /// API field: "variant"
    #[serde(alias = "variant")]
    pub image_variant: Option<String>,
    /// API field: "file_application"
    #[serde(alias = "file_application")]
    pub preinstalled_application: Option<String>,
    pub promoted: Option<String>,
    pub file_url: Option<String>,
    pub file_url_sha: Option<String>,
    pub file_extension: Option<String>,
    pub file_size: Option<String>,
    pub download_repository: Option<String>,
    pub redi_url: Option<String>,
    /// API field: "platinum"
    #[serde(alias = "platinum")]
    pub platinum_support: Option<String>,
    /// API field: "platinum_until"
    #[serde(alias = "platinum_until")]
    pub platinum_support_until: Option<String>,
    /// Board support level: "conf", "csc", "eos", "tvb", "wip"
    pub board_support: Option<String>,
}

/// Board information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardInfo {
    pub slug: String,
    pub name: String,
    pub vendor: String,
    pub vendor_name: String,
    pub vendor_logo: Option<String>,
    pub image_count: usize,
    pub has_standard_support: bool,
    pub has_community_support: bool,
    pub has_platinum_support: bool,
    pub has_eos_support: bool,
    pub has_tvb_support: bool,
    pub has_wip_support: bool,
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

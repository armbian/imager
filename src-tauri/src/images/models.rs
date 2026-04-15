//! Image data models
//!
//! Types representing Armbian API responses, boards, images, and vendors.

use serde::{Deserialize, Serialize};

// ─── API response envelope ───────────────────────────────────────────────────

/// Generic API response envelope wrapping data + metadata
#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
    #[serde(default)]
    pub meta: ApiMeta,
}

/// Metadata attached to every API response
#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)]
pub struct ApiMeta {
    pub last_sync: Option<String>,
    pub total: Option<u32>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

// ─── API types: Boards ───────────────────────────────────────────────────────

/// Board summary from `GET /boards`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiBoardSummary {
    pub slug: String,
    pub name: String,
    pub vendor_slug: String,
    pub vendor_name: String,
    pub support_tier: String,
    pub image_count: u32,
    pub has_desktop: bool,
    pub promoted: bool,
    pub image_url: Option<String>,
    pub soc: Option<String>,
    pub architecture: Option<String>,
    pub summary: Option<String>,
}

// ─── API types: Images ───────────────────────────────────────────────────────

/// Image entry from `GET /boards/:slug/images`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiImage {
    pub id: String,
    pub board_slug: String,
    pub variant: String,
    pub distribution: String,
    pub release: String,
    pub kernel_branch: String,
    pub kernel_version: String,
    pub application: Option<String>,
    pub promoted: bool,
    pub stability: String,
    pub format: String,
    pub storage: Option<String>,
    #[serde(default)]
    pub companions: Vec<ApiCompanion>,
    #[serde(default)]
    pub display_variants: Vec<ApiDisplayVariant>,
    pub download: ApiDownloadInfo,
}

/// Download metadata nested inside an image entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDownloadInfo {
    pub file_url: String,
    pub direct_url: String,
    pub sha_url: Option<String>,
    pub asc_url: Option<String>,
    pub torrent_url: Option<String>,
    pub size_bytes: u64,
    pub updated_at: Option<String>,
}

/// Companion file required for flashing (bootloader, fip, recovery, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCompanion {
    #[serde(rename = "type")]
    pub type_name: String,
    pub label: String,
    pub url: String,
    pub size_bytes: u64,
}

/// Display variant for multi-panel devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDisplayVariant {
    pub label: String,
    pub url: String,
    pub size_bytes: u64,
}

// ─── API types: Vendors ──────────────────────────────────────────────────────

/// Vendor/manufacturer from `GET /vendors`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiVendor {
    pub slug: String,
    pub name: String,
    pub logo_url: Option<String>,
    pub website: Option<String>,
    pub description: Option<String>,
    pub board_count: u32,
    pub partner_tier: Option<String>,
}

// ─── Frontend-facing types ───────────────────────────────────────────────────

/// Board information for display in the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardInfo {
    pub slug: String,
    pub name: String,
    /// Vendor slug identifier (e.g., "radxa")
    pub vendor: String,
    /// Vendor display name (e.g., "Radxa")
    pub vendor_name: String,
    /// Support tier: "platinum", "standard", "community", "eos", "tvb", "wip"
    pub support_tier: String,
    pub image_count: usize,
    /// Whether desktop environment images are available
    pub has_desktop: bool,
    /// Whether this board is featured/promoted
    pub promoted: bool,
    /// System-on-Chip model (e.g., "RK3588")
    pub soc: Option<String>,
    /// CPU architecture (e.g., "arm64")
    pub architecture: Option<String>,
    /// Short board description
    pub summary: Option<String>,
}

/// Processed image information for the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    /// Armbian release version (e.g., "24.02.0")
    pub release: String,
    pub distro_release: String,
    pub kernel_branch: String,
    pub kernel_version: String,
    pub image_variant: String,
    pub preinstalled_application: String,
    pub promoted: bool,
    pub file_url: String,
    /// Direct CDN download URL
    pub direct_url: String,
    /// SHA256 checksum file URL
    pub sha_url: Option<String>,
    /// Compressed image size in bytes
    pub file_size: u64,
    /// Stability level: "stable", "edge", "nightly"
    pub stability: String,
    /// Image format: "sd" (block), "qdl" (Qualcomm EDL), "rootfs", "qemu", "hyperv"
    pub format: String,
    /// Companion files (bootloaders, firmware, etc.)
    #[serde(default)]
    pub companions: Vec<CompanionInfo>,
    /// Display variant files for multi-panel devices
    #[serde(default)]
    pub display_variants: Vec<DisplayVariantInfo>,
}

/// Companion file info for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionInfo {
    pub type_name: String,
    pub label: String,
    pub url: String,
    pub size_bytes: u64,
}

/// Display variant info for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayVariantInfo {
    pub label: String,
    pub url: String,
    pub size_bytes: u64,
}

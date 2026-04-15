//! API data mappers
//!
//! Simple mapping functions that convert API response types into
//! frontend-facing types. The new REST API returns pre-processed data,
//! so no complex extraction, validation, or deduplication is needed.

use super::models::{
    ApiBoardSummary, ApiImage, BoardInfo, CompanionInfo, DisplayVariantInfo, ImageInfo,
};

/// Map an API board summary to a frontend-facing BoardInfo
pub fn map_board(api: &ApiBoardSummary) -> BoardInfo {
    BoardInfo {
        slug: api.slug.clone(),
        name: api.name.clone(),
        vendor: api.vendor_slug.clone(),
        vendor_name: api.vendor_name.clone(),
        support_tier: api.support_tier.clone(),
        image_count: api.image_count as usize,
        has_desktop: api.has_desktop,
        promoted: api.promoted,
        soc: api.soc.clone(),
        architecture: api.architecture.clone(),
        summary: api.summary.clone(),
    }
}

/// Map a list of API images to frontend-facing ImageInfo, sorted by promoted first then release
pub fn map_images(api_images: Vec<ApiImage>) -> Vec<ImageInfo> {
    let mut images: Vec<ImageInfo> = api_images.iter().map(map_image).collect();

    // Sort: promoted first, then by release version descending
    images.sort_by(|a, b| match (a.promoted, b.promoted) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => b.release.cmp(&a.release),
    });

    images
}

/// Map a single API image to a frontend-facing ImageInfo
fn map_image(api: &ApiImage) -> ImageInfo {
    ImageInfo {
        release: api.release.clone(),
        distro_release: api.distribution.clone(),
        kernel_branch: api.kernel_branch.clone(),
        kernel_version: api.kernel_version.clone(),
        image_variant: api.variant.clone(),
        preinstalled_application: api.application.clone().unwrap_or_default(),
        promoted: api.promoted,
        file_url: api.download.file_url.clone(),
        direct_url: api.download.direct_url.clone(),
        sha_url: api.download.sha_url.clone(),
        file_size: api.download.size_bytes,
        stability: api.stability.clone(),
        format: api.format.clone(),
        companions: api
            .companions
            .iter()
            .map(|c| CompanionInfo {
                type_name: c.type_name.clone(),
                label: c.label.clone(),
                url: c.url.clone(),
                size_bytes: c.size_bytes,
            })
            .collect(),
        display_variants: api
            .display_variants
            .iter()
            .map(|dv| DisplayVariantInfo {
                label: dv.label.clone(),
                url: dv.url.clone(),
                size_bytes: dv.size_bytes,
            })
            .collect(),
    }
}

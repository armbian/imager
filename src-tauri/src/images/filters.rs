//! Map API response types into frontend-facing types. The REST API returns
//! pre-processed data, so no extraction or deduplication is needed.

use super::models::{
    ApiBoardSummary, ApiImage, BoardInfo, BoardQdlInfo, CompanionInfo, DisplayVariantInfo,
    ImageInfo,
};
use crate::qdl::qdl_storage_supported;

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
        qdl: api.qdl.as_ref().map(|q| BoardQdlInfo {
            supported: qdl_storage_supported(&q.storage),
            edl_entry: q.edl_entry.clone(),
        }),
    }
}

/// Formats the imager can actually write: raw block images (`sd`/`block`) and
/// Qualcomm EDL (`qdl`). VM disk images (`qemu`/`hyperv`) and rootfs tarballs are
/// not flashable to a device, so they are dropped from the listing.
fn is_flashable_format(format: &str) -> bool {
    matches!(format, "sd" | "block" | "qdl")
}

/// Map a list of API images to frontend-facing ImageInfo, sorted by promoted first then release
pub fn map_images(api_images: Vec<ApiImage>) -> Vec<ImageInfo> {
    let mut images: Vec<ImageInfo> = api_images
        .iter()
        .filter(|api| is_flashable_format(&api.format))
        .map(map_image)
        .collect();

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
        build_date: api.download.updated_at.clone(),
        stability: api.stability.clone(),
        format: api.format.clone(),
        storage: api.storage.clone(),
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

//! Application configuration and constants
//!
//! Centralizes all hard-coded values, URLs, and configuration options.
//! Some constants are defined for future use and consistency.

#![allow(dead_code)]

/// Application metadata
pub mod app {
    /// Application name used for cache directories
    pub const NAME: &str = "armbian-imager";

    /// Application display name
    pub const DISPLAY_NAME: &str = "Armbian Imager";

    /// User agent for HTTP requests
    pub const USER_AGENT: &str = "Armbian-Imager/1.0";
}

/// API endpoints and URLs
pub mod urls {
    /// Armbian all-images JSON endpoint
    pub const ALL_IMAGES: &str = "https://github.armbian.com/armbian-images.json";

    /// Base URL for board images (cache.armbian.com/images/{size}/{board_slug}.png)
    pub const BOARD_IMAGES_BASE: &str = "https://cache.armbian.com/images/";

    /// Default image size for board photos (272px width, natural aspect ratio)
    pub const BOARD_IMAGE_SIZE: &str = "272";
}

/// Download and decompression settings
pub mod download {
    /// Download buffer size (1 MB)
    pub const BUFFER_SIZE: usize = 1024 * 1024;

    /// Decompression buffer size (8 MB)
    pub const DECOMPRESS_BUFFER_SIZE: usize = 8 * 1024 * 1024;

    /// Chunk size for streaming writes (4 MB)
    pub const CHUNK_SIZE: usize = 4 * 1024 * 1024;
}

/// Flash operation settings
pub mod flash {
    /// Write chunk size (4 MB)
    pub const CHUNK_SIZE: usize = 4 * 1024 * 1024;

    /// Quick erase size - zeros written before flashing (10 MB)
    pub const QUICK_ERASE_SIZE: usize = 10 * 1024 * 1024;

    /// Erase chunk size (1 MB)
    pub const ERASE_CHUNK_SIZE: usize = 1024 * 1024;

    /// Progress log interval (percentage points)
    pub const LOG_INTERVAL_PERCENT: u64 = 6;
}

/// Device detection settings
pub mod devices {
    /// Minimum device size to show (1 GB) - filters out small partitions
    pub const MIN_SIZE_BYTES: u64 = 1024 * 1024 * 1024;

    /// Maximum device size for removable media (2 TB)
    pub const MAX_SIZE_BYTES: u64 = 2 * 1024 * 1024 * 1024 * 1024;
}

/// HTTP client settings
pub mod http {
    /// Connection timeout in seconds
    pub const CONNECT_TIMEOUT_SECS: u64 = 30;

    /// Request timeout in seconds
    pub const REQUEST_TIMEOUT_SECS: u64 = 300;

    /// Short timeout for quick requests like board info (10 seconds)
    pub const SHORT_TIMEOUT_SECS: u64 = 10;
}

/// Image filtering constants
pub mod images {
    /// Filter value for empty preinstalled application
    pub const EMPTY_FILTER: &str = "__EMPTY__";

    /// Stable repository identifier
    pub const STABLE_REPO: &str = "archive";

    /// Temporary download file suffix
    pub const DOWNLOAD_SUFFIX: &str = ".downloading";
}

/// Cache management settings
pub mod cache {
    /// Minimum cache size: 1 GB
    pub const MIN_SIZE: u64 = 1024 * 1024 * 1024;

    /// Maximum cache size: 100 GB
    pub const MAX_SIZE: u64 = 100 * 1024 * 1024 * 1024;

    /// Default maximum cache size (20 GB)
    pub const DEFAULT_MAX_SIZE: u64 = 20 * 1024 * 1024 * 1024;

    /// Maximum consecutive flash failures before auto-deleting cached image
    pub const MAX_FLASH_FAILURES: u32 = 3;
}

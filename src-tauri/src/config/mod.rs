//! Centralized hard-coded values, URLs, and configuration options.
//! Allows dead_code: some consts are only referenced under platform cfg blocks.

#![allow(dead_code)]

/// Application metadata
pub mod app {
    /// Application name used for cache directories
    pub const NAME: &str = "armbian-imager";

    /// User agent for HTTP requests
    pub const USER_AGENT: &str = "Armbian-Imager/1.0";
}

/// API endpoints and URLs
pub mod urls {
    const API_BASE_DEFAULT: &str = "https://api.armbian.com/api/v1";

    /// Armbian REST API base. Overridable at runtime via `ARMBIAN_API_BASE` so a
    /// dev/test build can be pointed at a local API without a rebuild.
    pub fn api_base() -> String {
        std::env::var("ARMBIAN_API_BASE").unwrap_or_else(|_| API_BASE_DEFAULT.to_string())
    }

    /// Health check endpoint (no auth required)
    pub fn health() -> String {
        format!("{}/health", api_base())
    }

    /// Base URL for board images (api.armbian.com/api/v1/images/boards/{size}/{slug}.png)
    pub const BOARD_IMAGES_BASE: &str = "https://api.armbian.com/api/v1/images/boards/";

    /// Default image size for board photos (480px width, natural aspect ratio)
    pub const BOARD_IMAGE_SIZE: &str = "480";

    /// Base URL for vendor logos (api.armbian.com/api/v1/images/vendors/{size}/{slug}.png)
    pub const VENDOR_IMAGES_BASE: &str = "https://api.armbian.com/api/v1/images/vendors/480/";

    /// Base URL for QDL firehose loaders / provisioning XML, served by the API blob
    /// proxy ({base}{family}/{path}). Follows `api_base` so it routes locally in dev.
    pub fn qdl_blob_base() -> String {
        format!("{}/qdl/blob/", api_base())
    }
}

/// Download and decompression settings
pub mod download {
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

    /// Delay after unmount before writing (milliseconds)
    pub const UNMOUNT_DELAY_MS: u64 = 500;
}

/// Log file management settings
pub mod log_files {
    /// Maximum number of log files to retain (oldest are deleted)
    pub const MAX_LOG_FILES: usize = 10;
}

/// Progress logging intervals
pub mod logging {
    /// SHA256 calculation buffer size
    pub const SHA_BUFFER_SIZE: usize = 8192;

    /// Download progress log interval (MB)
    pub const DOWNLOAD_LOG_INTERVAL_MB: u64 = 10;

    /// Write progress log interval (MB)
    pub const WRITE_LOG_INTERVAL_MB: u64 = 512;

    /// Decompression progress log interval (MB)
    pub const DECOMPRESS_LOG_INTERVAL_MB: u64 = 100;

    /// Linux sync interval for flush operations
    pub const LINUX_SYNC_INTERVAL: u64 = 32 * 1024 * 1024;
}

/// Log paste service settings
pub mod paste {
    /// Maximum log file size to upload (5 MB)
    pub const MAX_LOG_SIZE: u64 = 5 * 1024 * 1024;

    /// Maximum log lines to process
    pub const MAX_LOG_LINES: usize = 10_000;
}

/// HTTP client settings
pub mod http {
    /// Connection timeout in seconds
    pub const CONNECT_TIMEOUT_SECS: u64 = 30;

    /// Request timeout in seconds
    pub const REQUEST_TIMEOUT_SECS: u64 = 300;

    /// Short timeout for quick requests like board info (10 seconds)
    pub const SHORT_TIMEOUT_SECS: u64 = 10;

    /// Client identification header name for the Armbian REST API
    pub const CLIENT_HEADER_NAME: &str = "X-Armbian-Client";

    /// Client identification header value for the Armbian Imager
    pub const CLIENT_HEADER_VALUE: &str = "armbian-imager";
}

/// Image filtering constants
pub mod images {
    /// Filter value for empty preinstalled application
    pub const EMPTY_FILTER: &str = "__EMPTY__";

    /// Temporary download file suffix
    pub const DOWNLOAD_SUFFIX: &str = ".downloading";
}

/// Cache management settings
pub mod cache {
    /// Default maximum cache size (20 GB)
    pub const DEFAULT_MAX_SIZE: u64 = 20 * 1024 * 1024 * 1024;
}

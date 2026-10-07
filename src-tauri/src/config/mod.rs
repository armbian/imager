// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

//! Centralized hard-coded values, URLs, and configuration options.
//! Allows dead_code: some consts are only referenced under platform cfg blocks.

#![allow(dead_code)]

/// Application metadata
pub mod app {
    /// Application name used for cache directories
    pub const NAME: &str = "armbian-imager";

    /// User agent for HTTP requests
    pub const USER_AGENT: &str = "Armbian-Imager/1.0";

    /// Settings store shared with the frontend (tauri-plugin-store)
    pub const SETTINGS_STORE: &str = "settings.json";

    /// Settings key unlocking internal/system disks as flash targets
    pub const SETTING_ALLOW_SYSTEM_DEVICES: &str = "allow_system_devices";
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

    /// Path prefix reserved for simulated devices; never a valid write target
    pub const SIMULATED_DEVICE_PREFIX: &str = "devsim://";

    /// Extra device scans when the selected target is missing (detection can miss a disk briefly)
    pub const TARGET_RESCAN_RETRIES: u32 = 3;

    /// Delay between those rescans (milliseconds)
    pub const TARGET_RESCAN_INTERVAL_MS: u64 = 100;

    /// Subdirectory of the QDL temp dir a TAR archive is extracted into
    pub const QDL_EXTRACT_DIR: &str = "qdl-extract";

    /// Detail behind the [CANCELLED] tag a block-device flash returns once the user cancelled it
    pub const CANCELLED_ERROR: &str = "Flash cancelled";
}

/// First-boot autoconfig preset injection
pub mod autoconfig {
    /// Where the preset lands in the rootfs; Armbian's first login sources it
    pub const PRESET_PATH: &str = "/root/.not_logged_in_yet";

    /// Script that consumes the preset on first boot; a custom image without it cannot take a profile
    pub const FIRSTLOGIN_MARKER: &str = "/usr/lib/armbian/armbian-firstlogin";

    /// Cache subdirectory for per-flash copies with the preset injected
    pub const TEMP_DIR: &str = "autoconfig-temp";

    /// Unix modes of that directory and of each copy: they hold the preset, passwords included
    pub const DIR_MODE: u32 = 0o700;
    pub const FILE_MODE: u32 = 0o600;

    /// Working copy name when the source path has no file name
    pub const COPY_FALLBACK_NAME: &str = "image.img";
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

/// Public SSH key lookup behind a profile's key source (PRESET_USER_KEY / PRESET_ROOT_KEY)
pub mod ssh_keys {
    /// Keys URLs are {base}{username}{KEYS_SUFFIX}
    pub const GITHUB_KEYS_BASE: &str = "https://github.com/";
    pub const GITLAB_KEYS_BASE: &str = "https://gitlab.com/";
    pub const KEYS_SUFFIX: &str = ".keys";

    pub const GITHUB_USERNAME_MAX_CHARS: usize = 39;
    pub const GITLAB_USERNAME_MAX_CHARS: usize = 255;
    pub const URL_MAX_CHARS: usize = 2048;

    /// The only scheme accepted for a link and for every redirect hop
    pub const REQUIRED_SCHEME: &str = "https";

    pub const TIMEOUT_SECS: u64 = 8;
    pub const MAX_REDIRECTS: usize = 3;

    /// Response bodies above this are refused, not truncated
    pub const MAX_BODY_BYTES: usize = 64 * 1024;

    /// Keys listed in a lookup result; the total still counts every valid key
    pub const MAX_KEYS: usize = 50;

    pub const COMMENT_MAX_CHARS: usize = 100;

    /// Accepted OpenSSH public key types and the label `ssh-keygen -l` prints for each
    pub const KEY_TYPES: &[(&str, &str)] = &[
        ("ssh-ed25519", "ED25519"),
        ("ssh-rsa", "RSA"),
        ("ecdsa-sha2-nistp256", "ECDSA"),
        ("ecdsa-sha2-nistp384", "ECDSA"),
        ("ecdsa-sha2-nistp521", "ECDSA"),
        ("sk-ssh-ed25519@openssh.com", "ED25519-SK"),
        ("sk-ecdsa-sha2-nistp256@openssh.com", "ECDSA-SK"),
    ];

    /// Prefix of the fingerprint string, as ssh-keygen prints it
    pub const FINGERPRINT_PREFIX: &str = "SHA256:";
}

/// Image filtering constants
pub mod images {
    /// Filter value for empty preinstalled application
    pub const EMPTY_FILTER: &str = "__EMPTY__";

    /// Temporary download file suffix
    pub const DOWNLOAD_SUFFIX: &str = ".downloading";

    /// Extension of a finished, decompressed image in the cache
    pub const CACHED_IMAGE_EXT: &str = ".img";

    /// Longest cache file name accepted, leaving room for DOWNLOAD_SUFFIX under the usual 255-byte limit
    pub const MAX_FILE_NAME_LEN: usize = 200;

    /// Windows device names refused as a file stem (before the first dot), compared case-insensitively
    pub const RESERVED_DEVICE_NAMES: &[&str] = &[
        "CON",
        "PRN",
        "AUX",
        "NUL",
        "COM0",
        "COM1",
        "COM2",
        "COM3",
        "COM4",
        "COM5",
        "COM6",
        "COM7",
        "COM8",
        "COM9",
        "COM\u{b9}",
        "COM\u{b2}",
        "COM\u{b3}",
        "LPT0",
        "LPT1",
        "LPT2",
        "LPT3",
        "LPT4",
        "LPT5",
        "LPT6",
        "LPT7",
        "LPT8",
        "LPT9",
        "LPT\u{b9}",
        "LPT\u{b2}",
        "LPT\u{b3}",
    ];

    /// Raw disk image extension; only these custom images are probed for the first-login marker
    pub const RAW_IMAGE_EXTENSION: &str = ".img";

    /// Custom image names that can take a profile without a board match (lowercase suffixes)
    pub const AUTOCONFIG_CUSTOM_EXTENSIONS: &[&str] = &[".img", ".img.xz"];
}

/// Cache management settings
pub mod cache {
    /// Default maximum cache size (20 GB)
    pub const DEFAULT_MAX_SIZE: u64 = 20 * 1024 * 1024 * 1024;

    /// A temp leftover with no live owner pid is removed at startup once older than this
    pub const STALE_TEMP_SECS: u64 = 12 * 60 * 60;

    /// Marker in the app cache root: the one-time purge of UFS images tainted by issue #196 ran
    pub const UFS_PRESET_PURGE_MARKER: &str = ".ufs-preset-purge-done";
}

/// Bus types for the dev scenarios presets, model and vdisk; same strings `devices/types.rs` emits
#[cfg(debug_assertions)]
pub mod devices {
    pub const BUS_SD: &str = "SD";
    pub const BUS_USB: &str = "USB";
    pub const BUS_NVME: &str = "NVMe";
    pub const BUS_SATA: &str = "SATA";
    pub const BUS_SAS: &str = "SAS";
    pub const BUS_TYPES: [&str; 5] = [BUS_SD, BUS_USB, BUS_NVME, BUS_SATA, BUS_SAS];
}

/// Dev scenarios emulator
#[cfg(debug_assertions)]
pub mod dev {
    pub const ID_MAX_LEN: usize = 32;
    /// `devsim://edl-*` paths are EDL targets
    pub const EDL_ID_PREFIX: &str = "edl-";
    pub const MAX_FAKE_DEVICES: usize = 16;
    pub const MAX_LABEL_LEN: usize = 128;
    pub const DEFAULT_MODEL: &str = "Simulated disk";
    pub const DEFAULT_EDL_DESCRIPTION: &str = "Simulated Qualcomm EDL device";
    pub const EDL_BUS_ID: &str = "devsim";

    pub const MAX_DELAY_MS: u64 = 10 * 60 * 1000;
    pub const DEFAULT_FAIL_AT_PERCENT: u8 = 40;
    pub const MAX_FAIL_AT_PERCENT: u8 = 99;
    pub const DEFAULT_WRITE_MB_PER_SEC: u32 = 200;
    pub const DEFAULT_VERIFY_MB_PER_SEC: u32 = 400;
    pub const MAX_MB_PER_SEC: u32 = 10_000;
    pub const DEFAULT_API_DELAY_MS: u64 = 3_000;
    pub const DEFAULT_DOWNLOAD_KB_PER_SEC: u32 = 512;
    pub const MAX_DOWNLOAD_KB_PER_SEC: u32 = 1_000_000;

    pub const SIM_TICK_MS: u64 = 100;
    pub const SIM_SECTOR_SIZE: u64 = 512;
    pub const SIM_EXPECTED_SHA: &str =
        "0000000000000000000000000000000000000000000000000000000000000000";
    pub const QDL_STAGE_MS: u64 = 400;
    pub const QDL_SIM_PARTITIONS: [&str; 3] = ["boot", "dtb", "rootfs"];

    pub const TEST_IMAGE_DIR: &str = "dev-test-images";
    /// Generated test image name: `<prefix><size><unit>[<unaligned>]<ext>`
    pub const TEST_IMAGE_PREFIX: &str = "devsim-test-";
    pub const TEST_IMAGE_SIZE_UNIT: &str = "mb";
    pub const TEST_IMAGE_UNALIGNED: &str = "-unaligned";
    pub const TEST_IMAGE_EXT: &str = ".img";
    pub const MAX_TEST_IMAGE_MB: u64 = 32 * 1024;
    pub const UNALIGNED_TAIL_BYTES: u64 = 300;
    pub const TEST_PATTERN_BYTES: u64 = 1024 * 1024;

    pub const VDISK_DIR: &str = "dev-vdisks";
    pub const VDISK_SUFFIX: &str = ".vdisk.img";
    pub const MAX_VDISK_MB: u64 = 32 * 1024;
    pub const VDISK_MODEL: &str = "Virtual disk (file)";
    pub const VDISK_UNSUPPORTED: &str = "Virtual disks are only supported on macOS";

    /// As real cards report them
    pub const PRESET_SD_32G_BYTES: u64 = 31_914_983_424;
    pub const PRESET_SD_8G_BYTES: u64 = 7_948_206_080;
    pub const PRESET_SD_1G_BYTES: u64 = 1_073_741_824;
    pub const PRESET_USB_16G_BYTES: u64 = 15_745_024_000;
    pub const PRESET_NVME_512G_BYTES: u64 = 512_110_190_592;
    pub const PRESET_HOT_PLUG_APPEAR_MS: u64 = 4_000;
    pub const PRESET_HOT_PLUG_DISAPPEAR_MS: u64 = 8_000;
    pub const PRESET_AUTH_DENIED_DELAY_MS: u64 = 1_500;
    pub const PRESET_SLOW_WRITE_MB_PER_SEC: u32 = 4;
    pub const PRESET_SLOW_VERIFY_MB_PER_SEC: u32 = 8;
    pub const PRESET_SLOW_DOWNLOAD_KB_PER_SEC: u32 = 256;
}

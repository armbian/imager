//! Armbian Imager: cross-platform Tauri app for downloading and flashing
//! Armbian OS images to SD cards and USB drives.

// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autoconfig;
mod cache;
mod commands;
mod config;
mod decompress;
mod devices;
mod download;
mod flash;
mod images;
mod logging;
mod paste;
mod picture_cache;
mod qdl;
mod utils;

use commands::AppState;
#[allow(unused_imports)] // Used by get_webview_window in debug builds
use tauri::Manager;
use tauri_plugin_store::StoreExt;

use crate::utils::custom_decompress_dir;

/// Apply cache settings: clear all images when disabled, else evict to the size limit.
fn manage_download_cache(app: &tauri::App) {
    let (cache_enabled, cache_max_size) = match app.store("settings.json") {
        Ok(store) => {
            let enabled = store
                .get("cache_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let max_size = store
                .get("cache_max_size")
                .and_then(|v| v.as_u64())
                .unwrap_or(cache::DEFAULT_MAX_SIZE);
            (enabled, max_size)
        }
        Err(e) => {
            log_warn!(
                "main",
                "Failed to load cache settings: {}. Using defaults.",
                e
            );
            (true, cache::DEFAULT_MAX_SIZE)
        }
    };

    if !cache_enabled {
        log_info!("main", "Image cache disabled, clearing cache directory");
        if let Err(e) = cache::clear_cache() {
            log_warn!("main", "Failed to clear cache: {}", e);
        }
    } else {
        log_info!(
            "main",
            "Image cache enabled with {} GB limit",
            cache_max_size / (1024 * 1024 * 1024)
        );
        if let Err(e) = cache::evict_to_size(cache_max_size) {
            log_warn!("main", "Failed to enforce cache size limit: {}", e);
        }
    }
}

/// Clean up orphaned decompressed custom images from previous sessions
fn cleanup_custom_decompress_cache() {
    let custom_dir = custom_decompress_dir();

    if custom_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&custom_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    log_info!(
                        "main",
                        "Cleaning up orphaned decompressed file: {}",
                        path.display()
                    );
                    let _ = std::fs::remove_file(&path);
                }
            }
        }

        let _ = std::fs::remove_dir(&custom_dir);
    }
}

/// Returns true if running as AppImage (APPIMAGE env var is set by AppImage runtime)
#[cfg(target_os = "linux")]
fn is_appimage() -> bool {
    std::env::var("APPIMAGE").is_ok()
}

fn main() {
    logging::init();

    log_info!("main", "=== Armbian Imager Starting ===");
    log_info!("main", "Version: {}", env!("CARGO_PKG_VERSION"));
    log_info!(
        "main",
        "OS: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    log_info!("main", "Config URLs:");
    log_info!("main", "  - API Base: {}", config::urls::api_base());
    log_info!(
        "main",
        "  - Board images: {}",
        config::urls::BOARD_IMAGES_BASE
    );

    // Drop the legacy pre-migration api-images.json cache.
    images::cleanup_legacy_cache();

    // Image cache management happens later in setup, where settings are available.
    cleanup_custom_decompress_cache();

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_store::Builder::new().build());

    // Enable updater only for AppImage on Linux (other formats like .deb don't support it)
    #[cfg(target_os = "linux")]
    {
        if is_appimage() {
            builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
        } else {
            log_info!("main", "Updater disabled (not running as AppImage)");
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::board_queries::get_boards,
            commands::board_queries::get_images_for_board,
            commands::board_queries::get_vendors,
            commands::board_queries::get_block_devices,
            commands::scraping::get_cached_board_image,
            commands::scraping::get_cached_vendor_logo,
            commands::operations::request_write_authorization,
            commands::operations::download_image,
            commands::operations::flash_image,
            commands::operations::delete_downloaded_image,
            commands::operations::force_delete_cached_image,
            commands::operations::continue_download_without_sha,
            commands::operations::cleanup_failed_download,
            commands::progress::cancel_operation,
            commands::progress::get_download_progress,
            commands::progress::get_flash_progress,
            commands::qdl_operations::get_qdl_devices,
            commands::qdl_operations::flash_qdl_image,
            commands::qdl_operations::flash_qdl_ufs_image,
            commands::custom_image::select_custom_image,
            commands::custom_image::check_needs_decompression,
            commands::custom_image::decompress_custom_image,
            commands::custom_image::delete_decompressed_custom_image,
            commands::custom_image::detect_board_from_filename,
            commands::custom_image::classify_custom_image,
            commands::system::open_url,
            commands::system::get_system_locale,
            commands::system::log_from_frontend,
            commands::system::log_warn_from_frontend,
            commands::system::log_debug_from_frontend,
            commands::system::get_armbian_release,
            commands::system::check_connectivity,
            commands::update::get_github_release,
            commands::update::is_app_in_applications,
            paste::upload::upload_logs,
            commands::settings::get_logs,
            commands::settings::get_system_info,
            commands::settings::get_tauri_version,
            commands::settings::get_cache_size,
            commands::settings::get_cache_breakdown,
            commands::settings::clear_cache,
            commands::settings::list_cached_images,
            commands::settings::delete_cached_image,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }

            // Raise the log level to DEBUG when developer mode is on.
            match app.store("settings.json") {
                Ok(store) => {
                    let developer_mode = store
                        .get("developer_mode")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    if developer_mode {
                        log_info!("main", "Developer mode enabled, setting log level to DEBUG");
                        logging::set_log_level(true);
                    } else {
                        log_info!("main", "Developer mode disabled, using default log level");
                    }
                }
                Err(e) => {
                    log_warn!(
                        "main",
                        "Failed to access settings store: {}. Using default log level (INFO).",
                        e
                    );
                }
            }

            manage_download_cache(app);

            // Background asset cache: refresh stale entries, then pre-populate everything.
            tauri::async_runtime::spawn(async {
                picture_cache::refresh_stale_assets().await;
                picture_cache::prepopulate_assets().await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

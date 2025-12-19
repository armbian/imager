//! Application logging system
//!
//! Provides structured, formatted logging with file output support.
//! Logs are written to the application's log directory with timestamps
//! and log level indicators.

use chrono::{DateTime, Local};
use once_cell::sync::Lazy;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::config;
use crate::utils::get_cache_dir;

/// Log levels for categorizing messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    #[allow(dead_code)]
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

}

/// Logger configuration
pub struct LoggerConfig {
    /// Minimum log level to output
    pub min_level: LogLevel,
    /// Whether to output to stderr
    pub console_output: bool,
    /// Whether to output to file
    pub file_output: bool,
    /// Whether to use colors in console output
    pub use_colors: bool,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            min_level: LogLevel::Info,
            console_output: true,
            file_output: true,
            use_colors: true,
        }
    }
}

/// Global logger instance
struct Logger {
    config: LoggerConfig,
    log_file: Option<File>,
    log_path: Option<PathBuf>,
}

impl Logger {
    fn new() -> Self {
        let config = LoggerConfig::default();
        let (log_file, log_path) = Self::create_log_file();

        Self {
            config,
            log_file,
            log_path,
        }
    }

    fn create_log_file() -> (Option<File>, Option<PathBuf>) {
        let log_dir = get_log_dir();

        if let Err(e) = fs::create_dir_all(&log_dir) {
            eprintln!("Failed to create log directory: {}", e);
            return (None, None);
        }

        // Create log file with timestamp
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let log_filename = format!("armbian-imager_{}.log", timestamp);
        let log_path = log_dir.join(&log_filename);

        match OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&log_path)
        {
            Ok(file) => (Some(file), Some(log_path)),
            Err(e) => {
                eprintln!("Failed to create log file: {}", e);
                (None, None)
            }
        }
    }

    fn log(&mut self, level: LogLevel, module: &str, message: &str) {
        if level < self.config.min_level {
            return;
        }

        let timestamp = Local::now();
        let formatted_colored = self.format_message_colored(level, module, message, &timestamp);

        // Console output
        if self.config.console_output {
            if self.config.use_colors {
                eprintln!("{}", formatted_colored);
            } else {
                let formatted = self.format_message_plain(level, module, message, &timestamp);
                eprintln!("{}", formatted);
            }
        }

        // File output (with ANSI colors for hastebin)
        if self.config.file_output {
            if let Some(ref mut file) = self.log_file {
                let _ = writeln!(file, "{}", formatted_colored);
                let _ = file.flush();
            }
        }
    }

    fn format_message_colored(
        &self,
        level: LogLevel,
        module: &str,
        message: &str,
        timestamp: &DateTime<Local>,
    ) -> String {
        let reset = "\x1b[0m";
        let dim = "\x1b[90m";

        let (icon, level_color) = match level {
            LogLevel::Debug => ("●", "\x1b[35m"),
            LogLevel::Info => ("●", "\x1b[32m"),
            LogLevel::Warn => ("●", "\x1b[33m"),
            LogLevel::Error => ("●", "\x1b[31m"),
        };

        format!(
            "{}{} {}{}{} {}{}:{} {}",
            dim,
            timestamp.format("%H:%M:%S"),
            level_color,
            icon,
            reset,
            level_color,
            module,
            reset,
            message
        )
    }

    fn format_message_plain(
        &self,
        level: LogLevel,
        module: &str,
        message: &str,
        timestamp: &DateTime<Local>,
    ) -> String {
        format!(
            "[{}] [{}] [{}] {}",
            timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            level.as_str(),
            module,
            message
        )
    }
}

/// Global logger instance
static LOGGER: Lazy<Mutex<Logger>> = Lazy::new(|| Mutex::new(Logger::new()));

/// Get the log directory path
pub fn get_log_dir() -> PathBuf {
    get_cache_dir(config::app::NAME).join("logs")
}

/// Get the current log file path (if any)
pub fn get_current_log_path() -> Option<PathBuf> {
    LOGGER.lock().ok()?.log_path.clone()
}

/// Log a debug message
#[allow(dead_code)]
pub fn debug(module: &str, message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Debug, module, message);
    }
}

/// Log an info message
pub fn info(module: &str, message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Info, module, message);
    }
}

/// Log a warning message
pub fn warn(module: &str, message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Warn, module, message);
    }
}

/// Log an error message
pub fn error(module: &str, message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Error, module, message);
    }
}

/// Log a message with format arguments (debug level)
#[macro_export]
macro_rules! log_debug {
    ($module:expr, $($arg:tt)*) => {
        $crate::logging::debug($module, &format!($($arg)*))
    };
}

/// Log a message with format arguments (info level)
#[macro_export]
macro_rules! log_info {
    ($module:expr, $($arg:tt)*) => {
        $crate::logging::info($module, &format!($($arg)*))
    };
}

/// Log a message with format arguments (warn level)
#[macro_export]
macro_rules! log_warn {
    ($module:expr, $($arg:tt)*) => {
        $crate::logging::warn($module, &format!($($arg)*))
    };
}

/// Log a message with format arguments (error level)
#[macro_export]
macro_rules! log_error {
    ($module:expr, $($arg:tt)*) => {
        $crate::logging::error($module, &format!($($arg)*))
    };
}

/// Clean up old log files, keeping only the most recent ones
pub fn cleanup_old_logs(keep_count: usize) -> Result<usize, String> {
    let log_dir = get_log_dir();

    if !log_dir.exists() {
        return Ok(0);
    }

    let mut log_files: Vec<_> = fs::read_dir(&log_dir)
        .map_err(|e| format!("Failed to read log directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension().map_or(false, |ext| ext == "log")
        })
        .collect();

    // Sort by modification time (newest first)
    log_files.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });

    let mut deleted = 0;
    for entry in log_files.into_iter().skip(keep_count) {
        if fs::remove_file(entry.path()).is_ok() {
            deleted += 1;
        }
    }

    Ok(deleted)
}

/// Get total size of log files in bytes
#[allow(dead_code)]
pub fn get_logs_size() -> u64 {
    let log_dir = get_log_dir();

    if !log_dir.exists() {
        return 0;
    }

    fs::read_dir(&log_dir)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| entry.metadata().ok())
                .map(|meta| meta.len())
                .sum()
        })
        .unwrap_or(0)
}

/// Initialize the logger (call at application startup)
pub fn init() {
    // Force initialization of the lazy static
    drop(LOGGER.lock());

    info("logger", "Armbian Imager logging initialized");

    if let Some(path) = get_current_log_path() {
        info("logger", &format!("Log file: {}", path.display()));
    }

    // Clean up old logs (keep last 10)
    match cleanup_old_logs(10) {
        Ok(deleted) if deleted > 0 => {
            info("logger", &format!("Cleaned up {} old log files", deleted));
        }
        Err(e) => {
            warn("logger", &format!("Failed to cleanup old logs: {}", e));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn test_log_dir() {
        let log_dir = get_log_dir();
        assert!(log_dir.to_string_lossy().contains("armbian-imager"));
        assert!(log_dir.to_string_lossy().contains("logs"));
    }
}

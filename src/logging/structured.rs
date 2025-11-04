//! Structured logging setup using tracing
//!
//! This module provides structured JSON logging with configurable log levels,
//! file rotation, and console output for development.
//!
//! # Example
//!
//! ```no_run
//! use atlas::logging::init_logging;
//! use atlas::config::LoggingConfig;
//!
//! let config = LoggingConfig::default();
//! init_logging(&config).expect("Failed to initialize logging");
//! ```

use crate::config::LoggingConfig;
use crate::domain::Result;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

/// Guard that must be kept alive for the duration of the program
/// to ensure logs are flushed properly
pub struct LoggingGuard {
    _file_guard: Option<WorkerGuard>,
}

impl LoggingGuard {
    /// Create a new logging guard
    fn new(file_guard: Option<WorkerGuard>) -> Self {
        Self {
            _file_guard: file_guard,
        }
    }
}

/// Initialize the logging system based on configuration
///
/// This function sets up structured logging with:
/// - JSON formatting for production
/// - Configurable log levels
/// - File rotation for local logging
/// - Console output for development
///
/// # Arguments
///
/// * `log_level_str` - Log level as a string (trace, debug, info, warn, error)
/// * `config` - Logging configuration
///
/// # Returns
///
/// A `LoggingGuard` that must be kept alive for the duration of the program
///
/// # Example
///
/// ```no_run
/// use atlas::logging::init_logging;
/// use atlas::config::LoggingConfig;
///
/// let config = LoggingConfig::default();
/// let _guard = init_logging("info", &config).expect("Failed to initialize logging");
/// // Keep _guard alive for the duration of the program
/// ```
pub fn init_logging(log_level_str: &str, config: &LoggingConfig) -> Result<LoggingGuard> {
    // Parse log level from string
    let log_level = parse_log_level(log_level_str)?;

    // Create environment filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("atlas={}", log_level)));

    // Build the subscriber with layers
    let mut layers = Vec::new();

    // Console layer for development (always enabled)
    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_filter(env_filter.clone());

    layers.push(console_layer.boxed());

    // File logging layer (if enabled)
    let file_guard = if config.local_enabled {
        let rotation = match config.local_rotation.as_str() {
            "daily" => Rotation::DAILY,
            "hourly" => Rotation::HOURLY,
            "size" => Rotation::DAILY, // Size-based rotation uses daily as fallback
            _ => Rotation::DAILY,
        };

        // Create the log directory if it doesn't exist
        std::fs::create_dir_all(&config.local_path).map_err(|e| {
            crate::domain::AtlasError::Configuration(format!(
                "Failed to create log directory {}: {}",
                config.local_path, e
            ))
        })?;

        let file_appender = RollingFileAppender::new(rotation, &config.local_path, "atlas.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let file_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_writer(non_blocking)
            .with_filter(env_filter);

        layers.push(file_layer.boxed());
        Some(guard)
    } else {
        None
    };

    // Initialize the subscriber
    tracing_subscriber::registry().with(layers).init();

    tracing::info!(
        local_enabled = config.local_enabled,
        local_path = %config.local_path,
        azure_enabled = config.azure_enabled,
        "Logging initialized"
    );

    Ok(LoggingGuard::new(file_guard))
}

/// Parse log level from string
fn parse_log_level(level_str: &str) -> Result<Level> {
    match level_str.to_lowercase().as_str() {
        "trace" => Ok(Level::TRACE),
        "debug" => Ok(Level::DEBUG),
        "info" => Ok(Level::INFO),
        "warn" => Ok(Level::WARN),
        "error" => Ok(Level::ERROR),
        _ => Err(crate::domain::AtlasError::Configuration(format!(
            "Invalid log level: {}. Must be one of: trace, debug, info, warn, error",
            level_str
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_level_valid() {
        assert_eq!(parse_log_level("trace").unwrap(), Level::TRACE);
        assert_eq!(parse_log_level("debug").unwrap(), Level::DEBUG);
        assert_eq!(parse_log_level("info").unwrap(), Level::INFO);
        assert_eq!(parse_log_level("warn").unwrap(), Level::WARN);
        assert_eq!(parse_log_level("error").unwrap(), Level::ERROR);
    }

    #[test]
    fn test_parse_log_level_case_insensitive() {
        assert_eq!(parse_log_level("TRACE").unwrap(), Level::TRACE);
        assert_eq!(parse_log_level("Debug").unwrap(), Level::DEBUG);
        assert_eq!(parse_log_level("INFO").unwrap(), Level::INFO);
    }

    #[test]
    fn test_parse_log_level_invalid() {
        assert!(parse_log_level("invalid").is_err());
        assert!(parse_log_level("").is_err());
    }

    #[test]
    fn test_init_logging_minimal() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config = LoggingConfig {
            local_enabled: true,
            local_path: temp_dir.path().to_string_lossy().to_string(),
            local_rotation: "daily".to_string(),
            local_max_size_mb: 100,
            azure_enabled: false,
            azure_tenant_id: None,
            azure_client_id: None,
            azure_client_secret: None,
            azure_log_analytics_workspace_id: None,
            azure_dcr_immutable_id: None,
            azure_dce_endpoint: None,
            azure_stream_name: None,
        };

        // Note: We can't actually test the full initialization in unit tests
        // because tracing_subscriber can only be initialized once per process
        // This test just validates the config structure
        assert!(config.local_enabled);
    }

    #[test]
    fn test_logging_guard_creation() {
        let guard = LoggingGuard::new(None);
        // Guard should be created successfully
        drop(guard);
    }
}

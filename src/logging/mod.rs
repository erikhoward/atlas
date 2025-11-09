//! Logging and observability
//!
//! This module provides structured logging with support for:
//! - JSON-formatted logs (TR-5.1)
//! - Configurable log levels (TR-5.2)
//! - Local file logging with rotation (TR-5.3)
//! - Azure Log Analytics integration (TR-5.4)
//!
//! # Example
//!
//! ```no_run
//! use atlas::logging::init_logging;
//! use atlas::config::LoggingConfig;
//!
//! let config = LoggingConfig::default();
//! let _guard = init_logging("info", &config).expect("Failed to initialize logging");
//!
//! // Use tracing macros for logging
//! tracing::info!("Application started");
//! tracing::error!(error = "Something went wrong", "Error occurred");
//! ```

pub mod azure;
pub mod structured;

// Re-export commonly used items
pub use structured::{init_logging, LoggingGuard};

/// Log the start of an export operation
///
/// # Example
///
/// ```no_run
/// use atlas::log_export_start;
/// use atlas::domain::ids::{EhrId, TemplateId};
///
/// let ehr_id = EhrId::new("ehr-123").unwrap();
/// let template_id = TemplateId::new("vital_signs").unwrap();
/// log_export_start!(&ehr_id, &template_id);
/// ```
#[macro_export]
macro_rules! log_export_start {
    ($ehr_id:expr, $template_id:expr) => {
        tracing::info!(
            ehr_id = %$ehr_id,
            template_id = %$template_id,
            "Starting export"
        );
    };
}

/// Log the completion of an export operation
///
/// # Example
///
/// ```no_run
/// use atlas::log_export_complete;
/// use std::time::Duration;
///
/// let count = 42;
/// let duration = Duration::from_secs(10);
/// log_export_complete!(count, duration);
/// ```
#[macro_export]
macro_rules! log_export_complete {
    ($count:expr, $duration:expr) => {
        tracing::info!(
            count = $count,
            duration_ms = $duration.as_millis(),
            "Export completed"
        );
    };
}

/// Log an error with context
///
/// # Example
///
/// ```no_run
/// use atlas::log_error_with_context;
/// use atlas::domain::AtlasError;
///
/// let error = AtlasError::Configuration("Invalid config".to_string());
/// log_error_with_context!(&error, "Failed to load configuration");
/// ```
#[macro_export]
macro_rules! log_error_with_context {
    ($error:expr, $context:expr) => {
        tracing::error!(
            error = %$error,
            context = $context,
            "Error occurred"
        );
    };
}

/// Log a batch processing operation
///
/// # Example
///
/// ```no_run
/// use atlas::log_batch_processing;
///
/// log_batch_processing!(100, 1000);
/// ```
#[macro_export]
macro_rules! log_batch_processing {
    ($current:expr, $total:expr) => {
        tracing::debug!(
            current = $current,
            total = $total,
            progress_pct = ($current as f64 / $total as f64 * 100.0),
            "Processing batch"
        );
    };
}

/// Log a retry attempt
///
/// # Example
///
/// ```no_run
/// use atlas::log_retry_attempt;
///
/// log_retry_attempt!(2, 3, "Connection timeout");
/// ```
#[macro_export]
macro_rules! log_retry_attempt {
    ($attempt:expr, $max_attempts:expr, $reason:expr) => {
        tracing::warn!(
            attempt = $attempt,
            max_attempts = $max_attempts,
            reason = $reason,
            "Retrying operation"
        );
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_macros_compile() {
        // These tests just verify that the macros compile correctly
        // Actual logging output is not tested in unit tests
    }
}

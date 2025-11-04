//! Export summary and reporting
//!
//! This module defines structures for tracking and reporting export results.

use std::time::Duration;

/// Summary of an export operation
#[derive(Debug, Clone)]
pub struct ExportSummary {
    /// Total number of EHRs processed
    pub total_ehrs: usize,

    /// Total number of compositions processed
    pub total_compositions: usize,

    /// Number of successful exports
    pub successful_exports: usize,

    /// Number of failed exports
    pub failed_exports: usize,

    /// Number of duplicates skipped
    pub duplicates_skipped: usize,

    /// Duration of the export
    pub duration: Duration,

    /// Errors encountered during export
    pub errors: Vec<ExportError>,
}

impl ExportSummary {
    /// Create a new empty export summary
    pub fn new() -> Self {
        Self {
            total_ehrs: 0,
            total_compositions: 0,
            successful_exports: 0,
            failed_exports: 0,
            duplicates_skipped: 0,
            duration: Duration::from_secs(0),
            errors: Vec::new(),
        }
    }

    /// Set the duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Add an error
    pub fn add_error(&mut self, error: ExportError) {
        self.errors.push(error);
    }

    /// Check if the export was successful (no failures)
    pub fn is_successful(&self) -> bool {
        self.failed_exports == 0 && self.errors.is_empty()
    }

    /// Get success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_compositions == 0 {
            return 100.0;
        }
        (self.successful_exports as f64 / self.total_compositions as f64) * 100.0
    }

    /// Log the summary
    pub fn log_summary(&self) {
        tracing::info!(
            total_ehrs = self.total_ehrs,
            total_compositions = self.total_compositions,
            successful = self.successful_exports,
            failed = self.failed_exports,
            duplicates_skipped = self.duplicates_skipped,
            duration_secs = self.duration.as_secs(),
            success_rate = format!("{:.2}%", self.success_rate()),
            "Export completed"
        );

        if !self.errors.is_empty() {
            tracing::warn!(
                error_count = self.errors.len(),
                "Export completed with errors"
            );
            for error in &self.errors {
                tracing::warn!(
                    error_type = ?error.error_type,
                    message = %error.message,
                    "Export error"
                );
            }
        }
    }
}

impl Default for ExportSummary {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of export error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportErrorType {
    /// Connection error (OpenEHR or Cosmos DB)
    Connection,
    /// Authentication error
    Authentication,
    /// Query error (OpenEHR)
    Query,
    /// Transformation error
    Transformation,
    /// Storage error (Cosmos DB)
    Storage,
    /// State management error
    State,
    /// Configuration error
    Configuration,
    /// Unknown error
    Unknown,
}

/// Export error with context
#[derive(Debug, Clone)]
pub struct ExportError {
    /// Type of error
    pub error_type: ExportErrorType,

    /// Error message
    pub message: String,

    /// Optional context (e.g., EHR ID, template ID)
    pub context: Option<String>,
}

impl ExportError {
    /// Create a new export error
    pub fn new(error_type: ExportErrorType, message: String) -> Self {
        Self {
            error_type,
            message,
            context: None,
        }
    }

    /// Add context to the error
    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_summary_creation() {
        let summary = ExportSummary::new();

        assert_eq!(summary.total_ehrs, 0);
        assert_eq!(summary.total_compositions, 0);
        assert_eq!(summary.successful_exports, 0);
        assert_eq!(summary.failed_exports, 0);
        assert_eq!(summary.duplicates_skipped, 0);
        assert_eq!(summary.duration, Duration::from_secs(0));
        assert!(summary.errors.is_empty());
    }

    #[test]
    fn test_export_summary_with_duration() {
        let summary = ExportSummary::new().with_duration(Duration::from_secs(120));

        assert_eq!(summary.duration, Duration::from_secs(120));
    }

    #[test]
    fn test_export_summary_is_successful() {
        let mut summary = ExportSummary::new();
        summary.successful_exports = 100;
        summary.total_compositions = 100;

        assert!(summary.is_successful());

        summary.failed_exports = 1;
        assert!(!summary.is_successful());
    }

    #[test]
    fn test_export_summary_success_rate() {
        let mut summary = ExportSummary::new();
        summary.total_compositions = 100;
        summary.successful_exports = 95;

        assert_eq!(summary.success_rate(), 95.0);

        summary.total_compositions = 0;
        assert_eq!(summary.success_rate(), 100.0);
    }

    #[test]
    fn test_export_error_creation() {
        let error = ExportError::new(ExportErrorType::Connection, "Failed to connect".to_string());

        assert_eq!(error.error_type, ExportErrorType::Connection);
        assert_eq!(error.message, "Failed to connect");
        assert!(error.context.is_none());
    }

    #[test]
    fn test_export_error_with_context() {
        let error = ExportError::new(ExportErrorType::Query, "Query failed".to_string())
            .with_context("template_id=vital_signs.v1".to_string());

        assert_eq!(error.error_type, ExportErrorType::Query);
        assert_eq!(
            error.context,
            Some("template_id=vital_signs.v1".to_string())
        );
    }

    #[test]
    fn test_export_summary_add_error() {
        let mut summary = ExportSummary::new();

        let error = ExportError::new(ExportErrorType::Storage, "Failed to write".to_string());

        summary.add_error(error);

        assert_eq!(summary.errors.len(), 1);
        assert_eq!(summary.errors[0].error_type, ExportErrorType::Storage);
    }
}

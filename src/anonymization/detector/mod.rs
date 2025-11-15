//! PII detection module
//!
//! Provides trait-based detection interface and implementations for
//! identifying PHI/PII in openEHR compositions.

pub mod patterns;
pub mod regex;

use crate::anonymization::models::PiiEntity;
use anyhow::Result;
use serde_json::Value;

/// Trait for PII detection implementations
pub trait PiiDetector: Send + Sync {
    /// Detect PII in a JSON value
    fn detect(&self, value: &Value, field_path: &str) -> Result<Vec<PiiEntity>>;

    /// Detect PII in a specific field
    fn detect_in_field(
        &self,
        field_name: &str,
        field_value: &str,
        field_path: &str,
    ) -> Result<Vec<PiiEntity>>;

    /// Get the confidence threshold for this detector
    fn confidence_threshold(&self) -> f32;
}

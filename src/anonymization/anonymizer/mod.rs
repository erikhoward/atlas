//! Anonymization strategy module
//!
//! Provides different strategies for anonymizing detected PII.

pub mod redaction;
pub mod tokenization;

use crate::anonymization::models::{PiiCategory, PiiEntity};
use anyhow::Result;

/// Trait for anonymization strategy implementations
pub trait Anonymizer: Send + Sync {
    /// Anonymize a detected PII entity
    fn anonymize(&mut self, entity: &PiiEntity) -> Result<String>;

    /// Anonymize a field value
    fn anonymize_field(&mut self, category: PiiCategory, value: &str) -> Result<String>;
}

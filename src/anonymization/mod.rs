//! Anonymization module for Atlas
//!
//! This module provides PHI/PII detection and anonymization capabilities
//! for openEHR compositions during export. It supports GDPR and HIPAA Safe Harbor
//! compliance modes with configurable anonymization strategies.
//!
//! # Architecture
//!
//! The anonymization pipeline consists of:
//! - **Detection**: Regex-based PII detection (Phase I)
//! - **Anonymization**: Strategy-based replacement (redaction, tokenization, generalization)
//! - **Compliance**: GDPR and HIPAA Safe Harbor rule sets
//! - **Audit**: Structured logging with hashed PII values
//!
//! # Usage
//!
//! ```rust,ignore
//! use atlas::anonymization::{AnonymizationEngine, config::AnonymizationConfig};
//!
//! let config = AnonymizationConfig::default();
//! let engine = AnonymizationEngine::new(config)?;
//! let anonymized = engine.anonymize_composition(composition)?;
//! ```

pub mod anonymizer;
pub mod audit;
pub mod compliance;
pub mod config;
pub mod detector;
pub mod engine;
pub mod models;
pub mod report;

// Re-export main types
pub use config::AnonymizationConfig;
pub use engine::AnonymizationEngine;
pub use models::{AnonymizedComposition, PiiCategory, PiiEntity};
pub use report::DryRunReport;

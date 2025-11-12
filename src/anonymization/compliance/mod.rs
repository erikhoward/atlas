//! Compliance module
//!
//! Provides GDPR and HIPAA Safe Harbor compliance rule sets.

pub mod gdpr;
pub mod hipaa;

use serde::{Deserialize, Serialize};

/// Compliance mode for anonymization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceMode {
    /// GDPR compliance (EU)
    Gdpr,
    /// HIPAA Safe Harbor compliance (US)
    HipaaSafeHarbor,
}

impl Default for ComplianceMode {
    fn default() -> Self {
        Self::Gdpr
    }
}


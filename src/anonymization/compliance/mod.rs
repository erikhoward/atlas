//! Compliance module
//!
//! Provides GDPR and HIPAA Safe Harbor compliance rule sets for PII detection
//! and anonymization.
//!
//! # Compliance Modes
//!
//! ## HIPAA Safe Harbor
//!
//! Implements the 18 identifiers specified in the HIPAA Safe Harbor method
//! (45 CFR §164.514(b)(2)) for de-identification of protected health information.
//!
//! ## GDPR
//!
//! Implements GDPR requirements by detecting all HIPAA identifiers plus
//! additional quasi-identifiers that could enable re-identification under
//! European data protection regulations.
//!
//! # Examples
//!
//! ```
//! use atlas::anonymization::compliance::ComplianceMode;
//!
//! let mode = ComplianceMode::HipaaSafeHarbor;
//! assert_eq!(mode.to_string(), "hipaa_safe_harbor");
//!
//! let mode = ComplianceMode::Gdpr;
//! assert_eq!(mode, ComplianceMode::default());
//! ```

pub mod gdpr;
pub mod hipaa;

use serde::{Deserialize, Serialize};
use std::fmt;

/// Compliance mode for anonymization
///
/// Determines which PII categories are detected and anonymized.
///
/// # Modes
///
/// - **HIPAA Safe Harbor**: 18 identifiers per 45 CFR §164.514(b)(2)
/// - **GDPR**: All HIPAA identifiers + 6 GDPR quasi-identifiers (24 total)
///
/// # Serialization
///
/// Uses snake_case for TOML/JSON serialization:
/// - `Gdpr` → `"gdpr"`
/// - `HipaaSafeHarbor` → `"hipaa_safe_harbor"`
///
/// # Examples
///
/// ```
/// use atlas::anonymization::compliance::ComplianceMode;
///
/// // Default is GDPR
/// let mode = ComplianceMode::default();
/// assert_eq!(mode, ComplianceMode::Gdpr);
///
/// // HIPAA Safe Harbor for US healthcare
/// let mode = ComplianceMode::HipaaSafeHarbor;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceMode {
    /// GDPR compliance (European Union)
    ///
    /// Detects all 18 HIPAA Safe Harbor identifiers plus 6 GDPR quasi-identifiers:
    /// - Occupation
    /// - Education level
    /// - Marital status
    /// - Ethnicity/race
    /// - Age
    /// - Gender
    Gdpr,

    /// HIPAA Safe Harbor compliance (United States)
    ///
    /// Detects the 18 identifiers specified in 45 CFR §164.514(b)(2):
    /// 1. Names
    /// 2. Geographic subdivisions smaller than state
    /// 3. Dates (except year)
    /// 4. Telephone numbers
    /// 5. Fax numbers
    /// 6. Email addresses
    /// 7. Social Security numbers
    /// 8. Medical record numbers
    /// 9. Health plan beneficiary numbers
    /// 10. Account numbers
    /// 11. Certificate/license numbers
    /// 12. Vehicle identifiers
    /// 13. Device identifiers
    /// 14. URLs
    /// 15. IP addresses
    /// 16. Biometric identifiers
    /// 17. Full-face photographs
    /// 18. Any other unique identifying number
    HipaaSafeHarbor,
}

impl Default for ComplianceMode {
    fn default() -> Self {
        Self::Gdpr
    }
}

impl fmt::Display for ComplianceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gdpr => write!(f, "gdpr"),
            Self::HipaaSafeHarbor => write!(f, "hipaa_safe_harbor"),
        }
    }
}

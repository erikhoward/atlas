//! Anonymization configuration
//!
//! This module defines the configuration structures for the anonymization feature,
//! including compliance modes, anonymization strategies, and audit settings.
//!
//! # Examples
//!
//! ```
//! use atlas::anonymization::config::{AnonymizationConfig, AnonymizationStrategy};
//! use atlas::anonymization::compliance::ComplianceMode;
//!
//! // Create default configuration
//! let config = AnonymizationConfig::default();
//! assert_eq!(config.enabled, false);
//! assert_eq!(config.mode, ComplianceMode::Gdpr);
//! assert_eq!(config.strategy, AnonymizationStrategy::Token);
//!
//! // Create custom configuration
//! let mut config = AnonymizationConfig::default();
//! config.enabled = true;
//! config.mode = ComplianceMode::HipaaSafeHarbor;
//! config.strategy = AnonymizationStrategy::Redact;
//! config.dry_run = true;
//!
//! // Validate configuration
//! assert!(config.validate().is_ok());
//! ```

use crate::anonymization::compliance::ComplianceMode;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Anonymization strategy for replacing detected PII
///
/// Determines how detected PII values are replaced in the anonymized output.
///
/// # Strategies
///
/// - **Redact**: Replace with category-specific markers like `[REDACTED_NAME]`
/// - **Token**: Replace with unique random tokens like `TOKEN_NAME_a1b2c3d4`
/// - **Generalize**: Replace with generalized values (Phase I - minimal implementation)
///
/// # Examples
///
/// ```
/// use atlas::anonymization::config::AnonymizationStrategy;
///
/// let strategy = AnonymizationStrategy::Token;
/// assert_eq!(strategy, AnonymizationStrategy::default());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnonymizationStrategy {
    /// Replace with `[REDACTED_CATEGORY]` markers
    ///
    /// Example: `"John Doe"` → `"[REDACTED_NAME]"`
    Redact,

    /// Replace with unique random tokens
    ///
    /// Example: `"John Doe"` → `"TOKEN_NAME_a1b2c3d4"`
    ///
    /// Tokens are unique per PII value within a single export run,
    /// maintaining referential integrity for analytics.
    Token,

    /// Replace with generalized values (Phase I - minimal implementation)
    ///
    /// Example: `"1985-03-15"` → `"1985"`
    Generalize,
}

impl Default for AnonymizationStrategy {
    fn default() -> Self {
        Self::Token
    }
}

/// Anonymization configuration for Phase I
///
/// This structure defines all configuration options for the anonymization feature,
/// including compliance mode, anonymization strategy, dry-run mode, and audit settings.
///
/// # Configuration Sources
///
/// Configuration can be loaded from:
/// 1. TOML configuration file (`atlas.toml`)
/// 2. Environment variables (prefix: `ATLAS_ANONYMIZATION_`)
/// 3. CLI flags (highest precedence)
///
/// # Examples
///
/// ```
/// use atlas::anonymization::config::AnonymizationConfig;
/// use atlas::anonymization::compliance::ComplianceMode;
///
/// // Create with defaults
/// let config = AnonymizationConfig::default();
/// assert!(!config.enabled);
///
/// // Validate configuration
/// assert!(config.validate().is_ok());
/// ```
///
/// # TOML Configuration
///
/// ```toml
/// [anonymization]
/// enabled = true
/// mode = "hipaa_safe_harbor"  # or "gdpr"
/// strategy = "token"          # or "redact"
/// dry_run = false
///
/// [anonymization.audit]
/// enabled = true
/// log_path = "./audit/anonymization.log"
/// json_format = true
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizationConfig {
    /// Enable or disable anonymization
    ///
    /// When `false`, compositions are exported without anonymization.
    /// Default: `false`
    #[serde(default)]
    pub enabled: bool,

    /// Compliance mode (GDPR or HIPAA Safe Harbor)
    ///
    /// Determines which PII categories are detected:
    /// - `Gdpr`: All HIPAA identifiers + GDPR quasi-identifiers (24 categories)
    /// - `HipaaSafeHarbor`: 18 HIPAA Safe Harbor identifiers only
    ///
    /// Default: `Gdpr`
    #[serde(default)]
    pub mode: ComplianceMode,

    /// Default anonymization strategy
    ///
    /// Determines how detected PII is replaced:
    /// - `Token`: Random unique tokens (e.g., `TOKEN_NAME_a1b2c3d4`)
    /// - `Redact`: Category markers (e.g., `[REDACTED_NAME]`)
    ///
    /// Default: `Token`
    #[serde(default)]
    pub strategy: AnonymizationStrategy,

    /// Dry-run mode (detect PII without anonymizing)
    ///
    /// When `true`, PII is detected and reported, but data is not anonymized
    /// and no database writes occur. Useful for testing and validation.
    ///
    /// Default: `false`
    #[serde(default)]
    pub dry_run: bool,

    /// Path to custom pattern library TOML file
    ///
    /// Optional path to a TOML file containing additional PII detection patterns.
    /// If not specified, the built-in pattern library is used.
    ///
    /// Default: `None` (use built-in patterns)
    pub pattern_library: Option<PathBuf>,

    /// Audit logging configuration
    ///
    /// Controls audit log generation for anonymization operations.
    /// Default: Enabled with JSON format
    #[serde(default)]
    pub audit: AuditConfig,
}

impl Default for AnonymizationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: ComplianceMode::Gdpr,
            strategy: AnonymizationStrategy::Token,
            dry_run: false,
            pattern_library: None,
            audit: AuditConfig::default(),
        }
    }
}

impl AnonymizationConfig {
    /// Validate the configuration
    ///
    /// Checks that all configuration values are valid and consistent.
    ///
    /// # Validation Rules
    ///
    /// - If `pattern_library` is specified, the file must exist and be a `.toml` file
    /// - Audit configuration must be valid (see [`AuditConfig::validate`])
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Pattern library file doesn't exist
    /// - Pattern library is not a TOML file
    /// - Audit configuration is invalid
    ///
    /// # Examples
    ///
    /// ```
    /// use atlas::anonymization::config::AnonymizationConfig;
    ///
    /// let config = AnonymizationConfig::default();
    /// assert!(config.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<()> {
        // Validate pattern library path if specified
        if let Some(ref path) = self.pattern_library {
            if !path.exists() {
                anyhow::bail!("Pattern library file not found: {}", path.display());
            }
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                anyhow::bail!("Pattern library must be a TOML file: {}", path.display());
            }
        }

        // Validate audit configuration
        self.audit
            .validate()
            .context("Invalid audit configuration")?;

        Ok(())
    }

    /// Apply environment variable overrides
    ///
    /// Overrides configuration values from environment variables following
    /// the 12-factor app pattern. Environment variables take precedence over
    /// TOML configuration but are overridden by CLI flags.
    ///
    /// # Environment Variables
    ///
    /// - `ATLAS_ANONYMIZATION_ENABLED`: Enable/disable anonymization (`true`/`false`)
    /// - `ATLAS_ANONYMIZATION_MODE`: Compliance mode (`gdpr`/`hipaa_safe_harbor`)
    /// - `ATLAS_ANONYMIZATION_STRATEGY`: Strategy (`token`/`redact`)
    /// - `ATLAS_ANONYMIZATION_DRY_RUN`: Dry-run mode (`true`/`false`)
    /// - `ATLAS_ANONYMIZATION_PATTERN_LIBRARY`: Path to pattern library file
    /// - `ATLAS_ANONYMIZATION_AUDIT_ENABLED`: Enable audit logging (`true`/`false`)
    /// - `ATLAS_ANONYMIZATION_AUDIT_LOG_PATH`: Audit log file path
    /// - `ATLAS_ANONYMIZATION_AUDIT_JSON_FORMAT`: Use JSON format (`true`/`false`)
    ///
    /// # Errors
    ///
    /// Returns an error if environment variable values are invalid.
    pub fn apply_env_overrides(&mut self) -> Result<()> {
        if let Ok(val) = std::env::var("ATLAS_ANONYMIZATION_ENABLED") {
            self.enabled = val
                .parse()
                .context("Invalid ATLAS_ANONYMIZATION_ENABLED value")?;
        }

        if let Ok(val) = std::env::var("ATLAS_ANONYMIZATION_MODE") {
            self.mode = match val.to_lowercase().as_str() {
                "gdpr" => ComplianceMode::Gdpr,
                "hipaa_safe_harbor" => ComplianceMode::HipaaSafeHarbor,
                _ => anyhow::bail!("Invalid ATLAS_ANONYMIZATION_MODE: {}", val),
            };
        }

        if let Ok(val) = std::env::var("ATLAS_ANONYMIZATION_STRATEGY") {
            self.strategy = match val.to_lowercase().as_str() {
                "redact" => AnonymizationStrategy::Redact,
                "token" => AnonymizationStrategy::Token,
                "generalize" => AnonymizationStrategy::Generalize,
                _ => anyhow::bail!("Invalid ATLAS_ANONYMIZATION_STRATEGY: {}", val),
            };
        }

        if let Ok(val) = std::env::var("ATLAS_ANONYMIZATION_DRY_RUN") {
            self.dry_run = val
                .parse()
                .context("Invalid ATLAS_ANONYMIZATION_DRY_RUN value")?;
        }

        if let Ok(val) = std::env::var("ATLAS_ANONYMIZATION_PATTERN_LIBRARY") {
            self.pattern_library = Some(PathBuf::from(val));
        }

        // Apply audit env overrides
        self.audit.apply_env_overrides()?;

        Ok(())
    }
}

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    #[serde(default = "default_audit_enabled")]
    pub enabled: bool,

    /// Audit log file path
    #[serde(default = "default_audit_log_path")]
    pub log_path: PathBuf,

    /// Use JSON format for audit logs
    #[serde(default = "default_audit_json_format")]
    pub json_format: bool,
}

fn default_audit_enabled() -> bool {
    true
}

fn default_audit_log_path() -> PathBuf {
    PathBuf::from("./audit/anonymization.log")
}

fn default_audit_json_format() -> bool {
    true
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: default_audit_enabled(),
            log_path: default_audit_log_path(),
            json_format: default_audit_json_format(),
        }
    }
}

impl AuditConfig {
    /// Validate audit configuration
    pub fn validate(&self) -> Result<()> {
        if self.enabled {
            // Ensure parent directory exists or can be created
            if let Some(parent) = self.log_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).with_context(|| {
                        format!("Failed to create audit log directory: {}", parent.display())
                    })?;
                }
            }
        }
        Ok(())
    }

    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) -> Result<()> {
        if let Ok(val) = std::env::var("ATLAS_ANONYMIZATION_AUDIT_ENABLED") {
            self.enabled = val
                .parse()
                .context("Invalid ATLAS_ANONYMIZATION_AUDIT_ENABLED value")?;
        }

        if let Ok(val) = std::env::var("ATLAS_ANONYMIZATION_AUDIT_LOG_PATH") {
            self.log_path = PathBuf::from(val);
        }

        if let Ok(val) = std::env::var("ATLAS_ANONYMIZATION_AUDIT_JSON_FORMAT") {
            self.json_format = val
                .parse()
                .context("Invalid ATLAS_ANONYMIZATION_AUDIT_JSON_FORMAT value")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AnonymizationConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.mode, ComplianceMode::Gdpr);
        assert_eq!(config.strategy, AnonymizationStrategy::Token);
        assert!(!config.dry_run);
        assert!(config.audit.enabled);
        assert!(config.audit.json_format);
    }

    #[test]
    fn test_config_validation() {
        let config = AnonymizationConfig::default();
        assert!(config.validate().is_ok());
    }
}

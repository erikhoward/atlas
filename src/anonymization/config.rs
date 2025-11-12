//! Anonymization configuration

use crate::anonymization::compliance::ComplianceMode;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Anonymization strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnonymizationStrategy {
    /// Replace with [CATEGORY] tokens
    Redact,
    /// Replace with unique tokens (CATEGORY_NNN)
    Token,
    /// Replace with generalized values (Phase I - minimal implementation)
    Generalize,
}

impl Default for AnonymizationStrategy {
    fn default() -> Self {
        Self::Token
    }
}

/// Minimal anonymization configuration for Phase I
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizationConfig {
    /// Enable/disable anonymization
    #[serde(default)]
    pub enabled: bool,
    
    /// Compliance mode (GDPR or HIPAA Safe Harbor)
    #[serde(default)]
    pub mode: ComplianceMode,
    
    /// Default anonymization strategy
    #[serde(default)]
    pub strategy: AnonymizationStrategy,
    
    /// Dry-run mode (detect but don't anonymize)
    #[serde(default)]
    pub dry_run: bool,
    
    /// Path to pattern library TOML file
    pub pattern_library: Option<PathBuf>,
    
    /// Audit logging configuration
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
        self.audit.validate()
            .context("Invalid audit configuration")?;
        
        Ok(())
    }
    
    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) -> Result<()> {
        if let Ok(val) = std::env::var("ATLAS_ANONYMIZATION_ENABLED") {
            self.enabled = val.parse()
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
            self.dry_run = val.parse()
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
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("Failed to create audit log directory: {}", parent.display()))?;
                }
            }
        }
        Ok(())
    }
    
    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) -> Result<()> {
        if let Ok(val) = std::env::var("ATLAS_ANONYMIZATION_AUDIT_ENABLED") {
            self.enabled = val.parse()
                .context("Invalid ATLAS_ANONYMIZATION_AUDIT_ENABLED value")?;
        }
        
        if let Ok(val) = std::env::var("ATLAS_ANONYMIZATION_AUDIT_LOG_PATH") {
            self.log_path = PathBuf::from(val);
        }
        
        if let Ok(val) = std::env::var("ATLAS_ANONYMIZATION_AUDIT_JSON_FORMAT") {
            self.json_format = val.parse()
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


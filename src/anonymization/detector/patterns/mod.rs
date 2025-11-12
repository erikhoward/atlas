//! Pattern library for PII detection

use crate::anonymization::models::PiiCategory;
use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Pattern definition from TOML
#[derive(Debug, Clone, Deserialize)]
pub struct PatternDefinition {
    /// Regex patterns for this category
    pub patterns: Vec<String>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// PII category label
    pub category: String,
}

/// Compiled pattern with metadata
#[derive(Debug, Clone)]
pub struct CompiledPattern {
    /// Compiled regex
    pub regex: Regex,
    /// PII category
    pub category: PiiCategory,
    /// Confidence score
    pub confidence: f32,
}

/// Pattern library container
#[derive(Debug, Deserialize)]
struct PatternLibrary {
    patterns: HashMap<String, PatternDefinition>,
}

/// Pattern registry for PII detection
pub struct PatternRegistry {
    patterns: Vec<CompiledPattern>,
    patterns_by_category: HashMap<PiiCategory, Vec<CompiledPattern>>,
}

impl PatternRegistry {
    /// Create a new pattern registry from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).with_context(|| {
            format!(
                "Failed to read pattern library: {}",
                path.as_ref().display()
            )
        })?;

        Self::from_toml(&content)
    }

    /// Create a pattern registry from TOML content
    pub fn from_toml(content: &str) -> Result<Self> {
        let library: PatternLibrary =
            toml::from_str(content).context("Failed to parse pattern library TOML")?;

        let mut patterns = Vec::new();
        let mut patterns_by_category: HashMap<PiiCategory, Vec<CompiledPattern>> = HashMap::new();

        for (name, def) in library.patterns {
            let category = Self::parse_category(&def.category).with_context(|| {
                format!("Invalid category in pattern '{}': {}", name, def.category)
            })?;

            for pattern_str in &def.patterns {
                let regex = Regex::new(pattern_str)
                    .with_context(|| format!("Invalid regex in pattern '{name}': {pattern_str}"))?;

                let compiled = CompiledPattern {
                    regex,
                    category,
                    confidence: def.confidence,
                };

                patterns.push(compiled.clone());
                patterns_by_category
                    .entry(category)
                    .or_default()
                    .push(compiled);
            }
        }

        Ok(Self {
            patterns,
            patterns_by_category,
        })
    }

    /// Create a default pattern registry with built-in patterns
    pub fn default_patterns() -> Result<Self> {
        // Use embedded default patterns
        let default_toml = include_str!("../../../../patterns/pii_patterns.toml");
        Self::from_toml(default_toml)
    }

    /// Get all patterns
    pub fn all_patterns(&self) -> &[CompiledPattern] {
        &self.patterns
    }

    /// Get patterns for a specific category
    pub fn patterns_for_category(&self, category: PiiCategory) -> Option<&[CompiledPattern]> {
        self.patterns_by_category
            .get(&category)
            .map(|v| v.as_slice())
    }

    /// Parse category string to PiiCategory enum
    fn parse_category(s: &str) -> Result<PiiCategory> {
        match s.to_uppercase().as_str() {
            "NAME" => Ok(PiiCategory::Name),
            "EMAIL" => Ok(PiiCategory::Email),
            "PHONE" => Ok(PiiCategory::Phone),
            "FAX" => Ok(PiiCategory::Fax),
            "SSN" => Ok(PiiCategory::Ssn),
            "MEDICAL_RECORD_NUMBER" | "MRN" => Ok(PiiCategory::MedicalRecordNumber),
            "DATE" => Ok(PiiCategory::Date),
            "GEOGRAPHIC_LOCATION" | "LOCATION" => Ok(PiiCategory::GeographicLocation),
            "IP_ADDRESS" => Ok(PiiCategory::IpAddress),
            "URL" => Ok(PiiCategory::Url),
            "ACCOUNT_NUMBER" | "ACCOUNT" => Ok(PiiCategory::AccountNumber),
            "DEVICE_IDENTIFIER" | "DEVICE" => Ok(PiiCategory::DeviceIdentifier),
            "VEHICLE_IDENTIFIER" | "VEHICLE" => Ok(PiiCategory::VehicleIdentifier),
            "HEALTH_PLAN_NUMBER" | "HEALTH_PLAN" => Ok(PiiCategory::HealthPlanNumber),
            "CERTIFICATE_LICENSE_NUMBER" | "LICENSE" => Ok(PiiCategory::CertificateLicenseNumber),
            "BIOMETRIC_IDENTIFIER" | "BIOMETRIC" => Ok(PiiCategory::BiometricIdentifier),
            "FACE_PHOTOGRAPH" | "PHOTO" => Ok(PiiCategory::FacePhotograph),
            "UNIQUE_IDENTIFIER" | "IDENTIFIER" => Ok(PiiCategory::UniqueIdentifier),
            "OCCUPATION" => Ok(PiiCategory::Occupation),
            "EDUCATION_LEVEL" | "EDUCATION" => Ok(PiiCategory::EducationLevel),
            "MARITAL_STATUS" => Ok(PiiCategory::MaritalStatus),
            "ETHNICITY" => Ok(PiiCategory::Ethnicity),
            "AGE" => Ok(PiiCategory::Age),
            "GENDER" => Ok(PiiCategory::Gender),
            _ => anyhow::bail!("Unknown PII category: {s}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_default_patterns() {
        let registry = PatternRegistry::default_patterns().unwrap();
        assert!(!registry.all_patterns().is_empty());
    }

    #[test]
    fn test_email_pattern() {
        let registry = PatternRegistry::default_patterns().unwrap();
        let email_patterns = registry.patterns_for_category(PiiCategory::Email).unwrap();
        assert!(!email_patterns.is_empty());

        let pattern = &email_patterns[0];
        assert!(pattern.regex.is_match("test@example.com"));
        assert!(!pattern.regex.is_match("not-an-email"));
    }

    #[test]
    fn test_phone_pattern() {
        let registry = PatternRegistry::default_patterns().unwrap();
        let phone_patterns = registry.patterns_for_category(PiiCategory::Phone).unwrap();
        assert!(!phone_patterns.is_empty());

        // Test US phone format
        let text = "Call me at (555) 123-4567";
        let has_match = phone_patterns.iter().any(|p| p.regex.is_match(text));
        assert!(has_match);
    }
}

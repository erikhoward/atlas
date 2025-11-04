//! Domain identifier types with validation
//!
//! This module provides newtype wrappers for OpenEHR identifiers following TR-6.3.
//! Each type ensures type safety and provides validation for format compliance.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// EHR identifier newtype wrapper
///
/// Represents a unique identifier for an Electronic Health Record.
/// Typically a UUID format but can vary by OpenEHR implementation.
///
/// # Examples
///
/// ```
/// use atlas::domain::ids::EhrId;
/// use std::str::FromStr;
///
/// let ehr_id = EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap();
/// assert_eq!(ehr_id.as_str(), "7d44b88c-4199-4bad-97dc-d78268e01398");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EhrId(String);

impl EhrId {
    /// Creates a new EhrId from a string
    ///
    /// # Arguments
    ///
    /// * `id` - The EHR identifier string
    ///
    /// # Returns
    ///
    /// Returns `Ok(EhrId)` if the ID is valid, `Err` otherwise
    pub fn new(id: impl Into<String>) -> Result<Self, String> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err("EHR ID cannot be empty".to_string());
        }
        Ok(Self(id))
    }

    /// Returns the EHR ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes self and returns the inner String
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for EhrId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for EhrId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl AsRef<str> for EhrId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Composition UID newtype wrapper
///
/// Represents a unique identifier for an OpenEHR composition including version.
/// Format: `{uuid}::{system_id}::{version}`
///
/// # Examples
///
/// ```
/// use atlas::domain::ids::CompositionUid;
/// use std::str::FromStr;
///
/// let uid = CompositionUid::from_str(
///     "84d7c3f5-1f6a-4f87-aa95-5d9c6b8f3a29::local.ehrbase.org::2"
/// ).unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompositionUid(String);

impl CompositionUid {
    /// Creates a new CompositionUid from a string
    ///
    /// # Arguments
    ///
    /// * `uid` - The composition UID string
    ///
    /// # Returns
    ///
    /// Returns `Ok(CompositionUid)` if the UID is valid, `Err` otherwise
    pub fn new(uid: impl Into<String>) -> Result<Self, String> {
        let uid = uid.into();
        if uid.trim().is_empty() {
            return Err("Composition UID cannot be empty".to_string());
        }

        // Basic validation: should contain :: separators for versioned UID
        let parts: Vec<&str> = uid.split("::").collect();
        if parts.len() != 3 {
            return Err(format!(
                "Invalid composition UID format. Expected format: {{uuid}}::{{system_id}}::{{version}}, got: {}",
                uid
            ));
        }

        Ok(Self(uid))
    }

    /// Returns the composition UID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes self and returns the inner String
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Extracts the base UUID part (before first ::)
    pub fn base_uuid(&self) -> &str {
        self.0.split("::").next().unwrap_or(&self.0)
    }

    /// Extracts the system ID part (between :: separators)
    pub fn system_id(&self) -> Option<&str> {
        self.0.split("::").nth(1)
    }

    /// Extracts the version number
    pub fn version(&self) -> Option<&str> {
        self.0.split("::").nth(2)
    }

    /// Parse a composition UID string (alias for `new`)
    ///
    /// This method is provided for API compatibility and clarity.
    ///
    /// # Arguments
    ///
    /// * `uid` - The composition UID string in format `{uuid}::{system_id}::{version}`
    ///
    /// # Errors
    ///
    /// Returns an error if the UID format is invalid
    pub fn parse(uid: impl Into<String>) -> Result<Self, String> {
        Self::new(uid)
    }
}

impl fmt::Display for CompositionUid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for CompositionUid {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl AsRef<str> for CompositionUid {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Template ID newtype wrapper
///
/// Represents an OpenEHR operational template identifier.
///
/// # Examples
///
/// ```
/// use atlas::domain::ids::TemplateId;
/// use std::str::FromStr;
///
/// let template_id = TemplateId::from_str("IDCR - Lab Report.v1").unwrap();
/// assert_eq!(template_id.as_str(), "IDCR - Lab Report.v1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TemplateId(String);

impl TemplateId {
    /// Creates a new TemplateId from a string
    ///
    /// # Arguments
    ///
    /// * `id` - The template identifier string
    ///
    /// # Returns
    ///
    /// Returns `Ok(TemplateId)` if the ID is valid, `Err` otherwise
    pub fn new(id: impl Into<String>) -> Result<Self, String> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err("Template ID cannot be empty".to_string());
        }
        Ok(Self(id))
    }

    /// Returns the template ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes self and returns the inner String
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Generates a sanitized container name from the template ID
    ///
    /// Replaces spaces and special characters with underscores and converts to lowercase
    pub fn to_container_name(&self, prefix: &str) -> String {
        let sanitized = self
            .0
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != '_', "_");

        // Remove consecutive underscores
        let mut result = String::new();
        let mut last_was_underscore = false;
        for c in sanitized.chars() {
            if c == '_' {
                if !last_was_underscore {
                    result.push(c);
                    last_was_underscore = true;
                }
            } else {
                result.push(c);
                last_was_underscore = false;
            }
        }

        let result = result.trim_matches('_').to_string();

        if prefix.is_empty() {
            result
        } else {
            format!("{}_{}", prefix, result)
        }
    }
}

impl fmt::Display for TemplateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TemplateId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl AsRef<str> for TemplateId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ehr_id_creation() {
        let id = EhrId::new("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap();
        assert_eq!(id.as_str(), "7d44b88c-4199-4bad-97dc-d78268e01398");
    }

    #[test]
    fn test_ehr_id_empty_fails() {
        assert!(EhrId::new("").is_err());
        assert!(EhrId::new("   ").is_err());
    }

    #[test]
    fn test_ehr_id_display() {
        let id = EhrId::new("test-id").unwrap();
        assert_eq!(format!("{}", id), "test-id");
    }

    #[test]
    fn test_ehr_id_from_str() {
        let id: EhrId = "7d44b88c-4199-4bad-97dc-d78268e01398".parse().unwrap();
        assert_eq!(id.as_str(), "7d44b88c-4199-4bad-97dc-d78268e01398");
    }

    #[test]
    fn test_composition_uid_creation() {
        let uid = CompositionUid::new("84d7c3f5-1f6a-4f87-aa95-5d9c6b8f3a29::local.ehrbase.org::2")
            .unwrap();
        assert_eq!(
            uid.as_str(),
            "84d7c3f5-1f6a-4f87-aa95-5d9c6b8f3a29::local.ehrbase.org::2"
        );
    }

    #[test]
    fn test_composition_uid_invalid_format() {
        assert!(CompositionUid::new("invalid-uid").is_err());
        assert!(CompositionUid::new("only::one").is_err());
    }

    #[test]
    fn test_composition_uid_parts() {
        let uid = CompositionUid::new("84d7c3f5-1f6a-4f87-aa95-5d9c6b8f3a29::local.ehrbase.org::2")
            .unwrap();
        assert_eq!(uid.base_uuid(), "84d7c3f5-1f6a-4f87-aa95-5d9c6b8f3a29");
        assert_eq!(uid.system_id(), Some("local.ehrbase.org"));
        assert_eq!(uid.version(), Some("2"));
    }

    #[test]
    fn test_template_id_creation() {
        let id = TemplateId::new("IDCR - Lab Report.v1").unwrap();
        assert_eq!(id.as_str(), "IDCR - Lab Report.v1");
    }

    #[test]
    fn test_template_id_empty_fails() {
        assert!(TemplateId::new("").is_err());
    }

    #[test]
    fn test_template_id_to_container_name() {
        let id = TemplateId::new("IDCR - Lab Report.v1").unwrap();
        assert_eq!(
            id.to_container_name("compositions"),
            "compositions_idcr_lab_report_v1"
        );

        let id2 = TemplateId::new("IDCR - Vital Signs.v1").unwrap();
        assert_eq!(id2.to_container_name(""), "idcr_vital_signs_v1");
    }

    #[test]
    fn test_template_id_serialization() {
        let id = TemplateId::new("IDCR - Lab Report.v1").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: TemplateId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }
}

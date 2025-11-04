//! EHR domain model
//!
//! This module defines the EHR (Electronic Health Record) type.

use super::ids::EhrId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents an Electronic Health Record
///
/// An EHR is a container for all health data related to a single subject (patient).
/// This type holds the EHR identifier and associated metadata.
///
/// # Examples
///
/// ```
/// use atlas::domain::ehr::Ehr;
/// use atlas::domain::ids::EhrId;
/// use chrono::Utc;
///
/// let ehr = Ehr::new(
///     EhrId::new("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap(),
///     Utc::now()
/// );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ehr {
    /// Unique identifier for this EHR
    pub id: EhrId,

    /// Timestamp when the EHR was created
    pub time_created: DateTime<Utc>,

    /// System ID where the EHR was created
    pub system_id: Option<String>,
}

impl Ehr {
    /// Creates a new EHR with the given ID and creation time
    ///
    /// # Arguments
    ///
    /// * `id` - The EHR identifier
    /// * `time_created` - When the EHR was created
    ///
    /// # Examples
    ///
    /// ```
    /// use atlas::domain::ehr::Ehr;
    /// use atlas::domain::ids::EhrId;
    /// use chrono::Utc;
    ///
    /// let ehr = Ehr::new(
    ///     EhrId::new("test-ehr-id").unwrap(),
    ///     Utc::now()
    /// );
    /// ```
    pub fn new(id: EhrId, time_created: DateTime<Utc>) -> Self {
        Self {
            id,
            time_created,
            system_id: None,
        }
    }

    /// Creates a new EHR with a system ID
    pub fn with_system_id(id: EhrId, time_created: DateTime<Utc>, system_id: String) -> Self {
        Self {
            id,
            time_created,
            system_id: Some(system_id),
        }
    }

    /// Returns a builder for constructing an EHR
    pub fn builder() -> EhrBuilder {
        EhrBuilder::default()
    }
}

/// Builder for constructing EHR instances
///
/// Follows the builder pattern (TR-6.2) for ergonomic construction.
#[derive(Debug, Default)]
pub struct EhrBuilder {
    id: Option<EhrId>,
    time_created: Option<DateTime<Utc>>,
    system_id: Option<String>,
}

impl EhrBuilder {
    /// Creates a new EhrBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the EHR ID
    pub fn id(mut self, id: EhrId) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the creation time
    pub fn time_created(mut self, time_created: DateTime<Utc>) -> Self {
        self.time_created = Some(time_created);
        self
    }

    /// Sets the system ID
    pub fn system_id(mut self, system_id: impl Into<String>) -> Self {
        self.system_id = Some(system_id.into());
        self
    }

    /// Builds the EHR
    ///
    /// # Errors
    ///
    /// Returns an error if required fields are missing
    pub fn build(self) -> Result<Ehr, String> {
        Ok(Ehr {
            id: self.id.ok_or("id is required")?,
            time_created: self.time_created.ok_or("time_created is required")?,
            system_id: self.system_id,
        })
    }
}

impl Default for Ehr {
    /// Creates a default EHR with a placeholder ID and current time
    ///
    /// Note: This is primarily for testing. Production code should use the builder.
    fn default() -> Self {
        Self {
            id: EhrId::new("default-ehr-id").unwrap(),
            time_created: Utc::now(),
            system_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ehr_creation() {
        let ehr_id = EhrId::new("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap();
        let now = Utc::now();
        let ehr = Ehr::new(ehr_id.clone(), now);

        assert_eq!(ehr.id, ehr_id);
        assert_eq!(ehr.time_created, now);
        assert_eq!(ehr.system_id, None);
    }

    #[test]
    fn test_ehr_with_system_id() {
        let ehr_id = EhrId::new("test-id").unwrap();
        let now = Utc::now();
        let ehr = Ehr::with_system_id(ehr_id.clone(), now, "local.ehrbase.org".to_string());

        assert_eq!(ehr.id, ehr_id);
        assert_eq!(ehr.system_id, Some("local.ehrbase.org".to_string()));
    }

    #[test]
    fn test_ehr_builder() {
        let ehr_id = EhrId::new("test-id").unwrap();
        let now = Utc::now();

        let ehr = Ehr::builder()
            .id(ehr_id.clone())
            .time_created(now)
            .system_id("local.ehrbase.org")
            .build()
            .unwrap();

        assert_eq!(ehr.id, ehr_id);
        assert_eq!(ehr.time_created, now);
        assert_eq!(ehr.system_id, Some("local.ehrbase.org".to_string()));
    }

    #[test]
    fn test_ehr_builder_missing_field() {
        let result = Ehr::builder().id(EhrId::new("test-id").unwrap()).build();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("time_created is required"));
    }

    #[test]
    fn test_ehr_serialization() {
        let ehr = Ehr::new(EhrId::new("test-id").unwrap(), Utc::now());

        let json = serde_json::to_string(&ehr).unwrap();
        let deserialized: Ehr = serde_json::from_str(&json).unwrap();

        assert_eq!(ehr.id, deserialized.id);
    }

    #[test]
    fn test_ehr_default() {
        let ehr = Ehr::default();
        assert_eq!(ehr.id.as_str(), "default-ehr-id");
        assert!(ehr.system_id.is_none());
    }
}

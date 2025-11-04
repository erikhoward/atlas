//! Composition domain model
//!
//! This module defines the core Composition type representing OpenEHR compositions.

use super::ids::{CompositionUid, EhrId, TemplateId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents an OpenEHR composition in FLAT format
///
/// A composition is a clinical document containing structured health data.
/// This type holds the composition metadata and content in FLAT JSON format.
///
/// # Examples
///
/// ```
/// use atlas::domain::composition::CompositionBuilder;
/// use atlas::domain::ids::{CompositionUid, EhrId, TemplateId};
/// use chrono::Utc;
/// use serde_json::json;
///
/// let composition = CompositionBuilder::new()
///     .uid(CompositionUid::new("84d7c3f5::local.ehrbase.org::1").unwrap())
///     .ehr_id(EhrId::new("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap())
///     .template_id(TemplateId::new("IDCR - Lab Report.v1").unwrap())
///     .time_committed(Utc::now())
///     .content(json!({"ctx/language": "en"}))
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Composition {
    /// Unique identifier for this composition (includes version)
    pub uid: CompositionUid,

    /// EHR ID this composition belongs to
    pub ehr_id: EhrId,

    /// Template ID defining the structure
    pub template_id: TemplateId,

    /// Timestamp when the composition was committed
    pub time_committed: DateTime<Utc>,

    /// Composition content in FLAT JSON format
    pub content: serde_json::Value,
}

impl Composition {
    /// Creates a new builder for constructing a Composition
    pub fn builder() -> CompositionBuilder {
        CompositionBuilder::default()
    }
}

/// Builder for constructing Composition instances
///
/// Follows the builder pattern (TR-6.2) for ergonomic construction of complex types.
#[derive(Debug, Default)]
pub struct CompositionBuilder {
    uid: Option<CompositionUid>,
    ehr_id: Option<EhrId>,
    template_id: Option<TemplateId>,
    time_committed: Option<DateTime<Utc>>,
    content: Option<serde_json::Value>,
}

impl CompositionBuilder {
    /// Creates a new CompositionBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the composition UID
    pub fn uid(mut self, uid: CompositionUid) -> Self {
        self.uid = Some(uid);
        self
    }

    /// Sets the EHR ID
    pub fn ehr_id(mut self, ehr_id: EhrId) -> Self {
        self.ehr_id = Some(ehr_id);
        self
    }

    /// Sets the template ID
    pub fn template_id(mut self, template_id: TemplateId) -> Self {
        self.template_id = Some(template_id);
        self
    }

    /// Sets the time committed
    pub fn time_committed(mut self, time_committed: DateTime<Utc>) -> Self {
        self.time_committed = Some(time_committed);
        self
    }

    /// Sets the composition content
    pub fn content(mut self, content: serde_json::Value) -> Self {
        self.content = Some(content);
        self
    }

    /// Builds the Composition
    ///
    /// # Errors
    ///
    /// Returns an error if any required field is missing
    pub fn build(self) -> Result<Composition, String> {
        Ok(Composition {
            uid: self.uid.ok_or("uid is required")?,
            ehr_id: self.ehr_id.ok_or("ehr_id is required")?,
            template_id: self.template_id.ok_or("template_id is required")?,
            time_committed: self.time_committed.ok_or("time_committed is required")?,
            content: self.content.ok_or("content is required")?,
        })
    }
}

/// Metadata about a composition (lightweight version without full content)
///
/// Used for listing and filtering compositions before fetching full content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionMetadata {
    /// Unique identifier for this composition
    pub uid: CompositionUid,

    /// EHR ID this composition belongs to
    pub ehr_id: EhrId,

    /// Template ID defining the structure
    pub template_id: TemplateId,

    /// Timestamp when the composition was committed
    pub time_committed: DateTime<Utc>,
}

impl CompositionMetadata {
    /// Creates a new CompositionMetadata
    pub fn new(
        uid: CompositionUid,
        ehr_id: EhrId,
        template_id: TemplateId,
        time_committed: DateTime<Utc>,
    ) -> Self {
        Self {
            uid,
            ehr_id,
            template_id,
            time_committed,
        }
    }

    /// Converts metadata to a full Composition by adding content
    pub fn with_content(self, content: serde_json::Value) -> Composition {
        Composition {
            uid: self.uid,
            ehr_id: self.ehr_id,
            template_id: self.template_id,
            time_committed: self.time_committed,
            content,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_composition_builder() {
        let composition = CompositionBuilder::new()
            .uid(CompositionUid::new("84d7c3f5::local.ehrbase.org::1").unwrap())
            .ehr_id(EhrId::new("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap())
            .template_id(TemplateId::new("IDCR - Lab Report.v1").unwrap())
            .time_committed(Utc::now())
            .content(json!({"ctx/language": "en"}))
            .build();

        assert!(composition.is_ok());
        let comp = composition.unwrap();
        assert_eq!(comp.uid.as_str(), "84d7c3f5::local.ehrbase.org::1");
    }

    #[test]
    fn test_composition_builder_missing_field() {
        let result = CompositionBuilder::new()
            .uid(CompositionUid::new("84d7c3f5::local.ehrbase.org::1").unwrap())
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ehr_id is required"));
    }

    #[test]
    fn test_composition_serialization() {
        let composition = CompositionBuilder::new()
            .uid(CompositionUid::new("84d7c3f5::local.ehrbase.org::1").unwrap())
            .ehr_id(EhrId::new("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap())
            .template_id(TemplateId::new("IDCR - Lab Report.v1").unwrap())
            .time_committed(Utc::now())
            .content(json!({"test": "data"}))
            .build()
            .unwrap();

        let json = serde_json::to_string(&composition).unwrap();
        let deserialized: Composition = serde_json::from_str(&json).unwrap();

        assert_eq!(composition.uid, deserialized.uid);
        assert_eq!(composition.ehr_id, deserialized.ehr_id);
    }

    #[test]
    fn test_composition_metadata() {
        let metadata = CompositionMetadata::new(
            CompositionUid::new("84d7c3f5::local.ehrbase.org::1").unwrap(),
            EhrId::new("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap(),
            TemplateId::new("IDCR - Lab Report.v1").unwrap(),
            Utc::now(),
        );

        let composition = metadata.with_content(json!({"test": "data"}));
        assert_eq!(composition.content, json!({"test": "data"}));
    }

    #[test]
    fn test_composition_builder_default() {
        let builder = Composition::builder();
        assert!(builder.uid.is_none());
    }
}

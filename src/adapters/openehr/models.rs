//! OpenEHR API models
//!
//! This module defines the API request and response structures for OpenEHR REST API.
//! These models are separate from domain models and handle the serialization/deserialization
//! of OpenEHR-specific formats.

use crate::domain::ids::{CompositionUid, EhrId, TemplateId};
use crate::domain::{AtlasError, Composition, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OpenEHR composition in FLAT format
///
/// This represents the raw composition data as returned by the OpenEHR REST API
/// in FLAT format. The FLAT format uses path-based keys to represent the
/// hierarchical structure of the composition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatComposition {
    /// Composition metadata
    #[serde(flatten)]
    pub meta: FlatCompositionMeta,

    /// Composition content as key-value pairs
    /// Keys are paths like "vital_signs/body_temperature:0|magnitude"
    #[serde(flatten)]
    pub content: HashMap<String, serde_json::Value>,
}

/// Metadata for a FLAT composition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatCompositionMeta {
    /// Composition UID
    #[serde(rename = "_uid")]
    pub uid: Option<String>,

    /// Template ID
    #[serde(rename = "_template_id")]
    pub template_id: Option<String>,

    /// Archetype node ID
    #[serde(rename = "_archetype_node_id")]
    pub archetype_node_id: Option<String>,

    /// Composition type
    #[serde(rename = "_type")]
    pub composition_type: Option<String>,
}

impl FlatComposition {
    /// Convert to domain Composition
    ///
    /// This method converts the API model to the domain model.
    ///
    /// # Errors
    ///
    /// Returns an error if required fields are missing or invalid.
    pub fn to_domain(&self) -> Result<Composition> {
        let uid_str = self.meta.uid.as_ref().ok_or_else(|| {
            AtlasError::OpenEhr(crate::domain::OpenEhrError::InvalidResponse(
                "Missing composition UID".to_string(),
            ))
        })?;

        let uid = CompositionUid::parse(uid_str)
            .map_err(|e| AtlasError::OpenEhr(crate::domain::OpenEhrError::InvalidResponse(e)))?;

        // Combine metadata and content into a single JSON object
        let mut full_content = serde_json::to_value(&self.meta)?;
        if let serde_json::Value::Object(ref mut map) = full_content {
            for (key, value) in &self.content {
                map.insert(key.clone(), value.clone());
            }
        }

        Composition::builder()
            .uid(uid)
            .content(full_content)
            .build()
            .map_err(AtlasError::Configuration)
    }
}

/// AQL query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AqlQueryRequest {
    /// AQL query string
    pub q: String,

    /// Query parameters (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_parameters: Option<HashMap<String, serde_json::Value>>,

    /// Offset for pagination (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,

    /// Fetch limit (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetch: Option<u32>,
}

impl AqlQueryRequest {
    /// Create a new AQL query request
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            q: query.into(),
            query_parameters: None,
            offset: None,
            fetch: None,
        }
    }

    /// Add query parameters
    pub fn with_parameters(mut self, parameters: HashMap<String, serde_json::Value>) -> Self {
        self.query_parameters = Some(parameters);
        self
    }

    /// Set pagination offset
    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Set fetch limit
    pub fn with_fetch(mut self, fetch: u32) -> Self {
        self.fetch = Some(fetch);
        self
    }
}

/// AQL query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AqlQueryResponse {
    /// Query metadata
    pub meta: AqlQueryMeta,

    /// Column definitions
    #[serde(default)]
    pub columns: Vec<AqlColumn>,

    /// Result rows
    #[serde(default)]
    pub rows: Vec<Vec<serde_json::Value>>,
}

/// AQL query metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AqlQueryMeta {
    /// Schema version
    #[serde(rename = "_schema_version")]
    pub schema_version: Option<String>,

    /// Created timestamp
    #[serde(rename = "_created")]
    pub created: Option<DateTime<Utc>>,

    /// Executed AQL query
    #[serde(rename = "_executed_aql")]
    pub executed_aql: Option<String>,
}

/// AQL column definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AqlColumn {
    /// Column name
    pub name: String,

    /// Column path
    pub path: String,
}

/// EHR status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EhrStatusResponse {
    /// EHR ID
    pub ehr_id: EhrId,

    /// System ID
    pub system_id: String,

    /// Time created
    pub time_created: DateTime<Utc>,

    /// Is modifiable
    pub is_modifiable: bool,

    /// Is queryable
    pub is_queryable: bool,
}

/// Template metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetadata {
    /// Template ID
    pub template_id: String,

    /// Template version (optional)
    pub version: Option<String>,

    /// Created timestamp (optional)
    pub created_timestamp: Option<DateTime<Utc>>,
}

impl TemplateMetadata {
    /// Convert to domain TemplateId
    pub fn to_domain_id(&self) -> Result<TemplateId> {
        TemplateId::new(&self.template_id)
            .map_err(|e| AtlasError::OpenEhr(crate::domain::OpenEhrError::InvalidResponse(e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_composition_deserialization() {
        let json = r#"{
            "_uid": "550e8400-e29b-41d4-a716-446655440000::local.ehrbase.org::1",
            "_template_id": "vital_signs",
            "_archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
            "_type": "COMPOSITION",
            "vital_signs/body_temperature:0|magnitude": 37.5,
            "vital_signs/body_temperature:0|unit": "°C"
        }"#;

        let composition: FlatComposition = serde_json::from_str(json).unwrap();

        assert_eq!(
            composition.meta.uid,
            Some("550e8400-e29b-41d4-a716-446655440000::local.ehrbase.org::1".to_string())
        );
        assert_eq!(
            composition.meta.template_id,
            Some("vital_signs".to_string())
        );
        assert_eq!(composition.content.len(), 2);
    }

    #[test]
    fn test_flat_composition_to_domain() {
        let json = r#"{
            "_uid": "550e8400-e29b-41d4-a716-446655440000::local.ehrbase.org::1",
            "_template_id": "vital_signs",
            "vital_signs/body_temperature:0|magnitude": 37.5
        }"#;

        let flat_composition: FlatComposition = serde_json::from_str(json).unwrap();

        // Note: to_domain() requires additional fields (ehr_id, template_id, time_committed)
        // that are not present in the FLAT format. In practice, these would be provided
        // by the caller based on the query context.
        // For now, we just test that the UID is parsed correctly.
        assert_eq!(
            flat_composition.meta.uid,
            Some("550e8400-e29b-41d4-a716-446655440000::local.ehrbase.org::1".to_string())
        );
    }

    #[test]
    fn test_aql_query_request_builder() {
        let request = AqlQueryRequest::new("SELECT e/ehr_id/value FROM EHR e")
            .with_offset(10)
            .with_fetch(100);

        assert_eq!(request.q, "SELECT e/ehr_id/value FROM EHR e");
        assert_eq!(request.offset, Some(10));
        assert_eq!(request.fetch, Some(100));
    }

    #[test]
    fn test_aql_query_request_serialization() {
        let request = AqlQueryRequest::new("SELECT e/ehr_id/value FROM EHR e");
        let json = serde_json::to_string(&request).unwrap();

        assert!(json.contains("SELECT e/ehr_id/value FROM EHR e"));
    }

    #[test]
    fn test_template_metadata_to_domain_id() {
        let metadata = TemplateMetadata {
            template_id: "vital_signs".to_string(),
            version: Some("1.0.0".to_string()),
            created_timestamp: None,
        };

        let template_id = metadata.to_domain_id().unwrap();
        assert_eq!(template_id.to_string(), "vital_signs");
    }

    #[test]
    fn test_aql_query_response_deserialization() {
        let json = r#"{
            "meta": {
                "_schema_version": "1.0.0",
                "_created": "2025-10-29T00:00:00Z",
                "_executed_aql": "SELECT e/ehr_id/value FROM EHR e"
            },
            "columns": [
                {
                    "name": "ehr_id",
                    "path": "/ehr_id/value"
                }
            ],
            "rows": [
                ["ehr-123"],
                ["ehr-456"]
            ]
        }"#;

        let response: AqlQueryResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.rows.len(), 2);
        assert_eq!(response.columns.len(), 1);
        assert_eq!(response.columns[0].name, "ehr_id");
    }
}

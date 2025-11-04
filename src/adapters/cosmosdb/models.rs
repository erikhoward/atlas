//! Cosmos DB document models
//!
//! This module defines the document structures used when storing compositions
//! in Azure Cosmos DB.

use crate::domain::composition::Composition;
use crate::domain::ids::TemplateId;
use crate::domain::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Metadata added by Atlas to track export information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasMetadata {
    /// When this composition was exported to Cosmos DB
    pub exported_at: DateTime<Utc>,

    /// Version of Atlas that exported this composition
    pub atlas_version: String,

    /// Checksum of the composition content (SHA-256)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,

    /// Export mode used (full or incremental)
    pub export_mode: String,

    /// Template ID this composition belongs to
    pub template_id: String,
}

impl AtlasMetadata {
    /// Create new Atlas metadata
    pub fn new(template_id: TemplateId, export_mode: String) -> Self {
        Self {
            exported_at: Utc::now(),
            atlas_version: env!("CARGO_PKG_VERSION").to_string(),
            checksum: None,
            export_mode,
            template_id: template_id.to_string(),
        }
    }

    /// Set the checksum
    pub fn with_checksum(mut self, checksum: String) -> Self {
        self.checksum = Some(checksum);
        self
    }
}

/// Composition document in preserved format (maintains FLAT structure)
///
/// This format preserves the exact FLAT JSON structure from OpenEHR,
/// storing it as-is in the `content` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosComposition {
    /// Document ID (composition UID)
    pub id: String,

    /// EHR ID (partition key)
    pub ehr_id: String,

    /// Composition UID
    pub composition_uid: String,

    /// Template ID
    pub template_id: String,

    /// Time the composition was committed in OpenEHR
    pub time_committed: DateTime<Utc>,

    /// Original FLAT JSON content from OpenEHR
    pub content: Value,

    /// Atlas metadata
    pub atlas_metadata: AtlasMetadata,
}

impl CosmosComposition {
    /// Convert from domain Composition to Cosmos document (preserved format)
    pub fn from_domain(composition: Composition, export_mode: String) -> Result<Self> {
        let id = composition.uid.to_string();
        let ehr_id = composition.ehr_id.to_string();
        let composition_uid = composition.uid.to_string();
        let template_id = composition.template_id.to_string();

        let atlas_metadata = AtlasMetadata::new(composition.template_id.clone(), export_mode);

        Ok(Self {
            id,
            ehr_id,
            composition_uid,
            template_id,
            time_committed: composition.time_committed,
            content: composition.content,
            atlas_metadata,
        })
    }

    /// Calculate SHA-256 checksum of the content
    pub fn calculate_checksum(&self) -> Result<String> {
        use sha2::{Digest, Sha256};

        let content_str = serde_json::to_string(&self.content)
            .map_err(|e| crate::domain::AtlasError::Serialization(e.to_string()))?;

        let mut hasher = Sha256::new();
        hasher.update(content_str.as_bytes());
        let result = hasher.finalize();

        Ok(format!("{:x}", result))
    }

    /// Add checksum to metadata
    pub fn with_checksum(mut self) -> Result<Self> {
        let checksum = self.calculate_checksum()?;
        self.atlas_metadata.checksum = Some(checksum);
        Ok(self)
    }
}

/// Composition document in flattened format
///
/// This format converts the nested FLAT paths (e.g., "vital_signs/body_temperature:0|magnitude")
/// into flat field names (e.g., "vital_signs_body_temperature_0_magnitude").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosCompositionFlattened {
    /// Document ID (composition UID)
    pub id: String,

    /// EHR ID (partition key)
    pub ehr_id: String,

    /// Composition UID
    pub composition_uid: String,

    /// Template ID
    pub template_id: String,

    /// Time the composition was committed in OpenEHR
    pub time_committed: DateTime<Utc>,

    /// Flattened fields from the FLAT JSON content
    #[serde(flatten)]
    pub fields: HashMap<String, Value>,

    /// Atlas metadata
    pub atlas_metadata: AtlasMetadata,
}

impl CosmosCompositionFlattened {
    /// Convert from domain Composition to Cosmos document (flattened format)
    pub fn from_domain(composition: Composition, export_mode: String) -> Result<Self> {
        let id = composition.uid.to_string();
        let ehr_id = composition.ehr_id.to_string();
        let composition_uid = composition.uid.to_string();
        let template_id = composition.template_id.to_string();

        let atlas_metadata = AtlasMetadata::new(composition.template_id.clone(), export_mode);

        // Flatten the content
        let fields = Self::flatten_content(&composition.content)?;

        Ok(Self {
            id,
            ehr_id,
            composition_uid,
            template_id,
            time_committed: composition.time_committed,
            fields,
            atlas_metadata,
        })
    }

    /// Flatten FLAT JSON content into a HashMap
    ///
    /// Converts paths like "vital_signs/body_temperature:0|magnitude" to
    /// "vital_signs_body_temperature_0_magnitude"
    fn flatten_content(content: &Value) -> Result<HashMap<String, Value>> {
        let mut fields = HashMap::new();

        if let Value::Object(map) = content {
            for (key, value) in map {
                // Convert FLAT path to flat field name
                let field_name = Self::flatten_path(key);
                fields.insert(field_name, value.clone());
            }
        }

        Ok(fields)
    }

    /// Convert a FLAT path to a flat field name
    ///
    /// Replaces special characters with underscores:
    /// - `/` -> `_`
    /// - `:` -> `_`
    /// - `|` -> `_`
    fn flatten_path(path: &str) -> String {
        path.replace(['/', ':', '|'], "_")
    }

    /// Calculate SHA-256 checksum of the fields
    pub fn calculate_checksum(&self) -> Result<String> {
        use sha2::{Digest, Sha256};

        let fields_str = serde_json::to_string(&self.fields)
            .map_err(|e| crate::domain::AtlasError::Serialization(e.to_string()))?;

        let mut hasher = Sha256::new();
        hasher.update(fields_str.as_bytes());
        let result = hasher.finalize();

        Ok(format!("{:x}", result))
    }

    /// Add checksum to metadata
    pub fn with_checksum(mut self) -> Result<Self> {
        let checksum = self.calculate_checksum()?;
        self.atlas_metadata.checksum = Some(checksum);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::composition::CompositionBuilder;
    use crate::domain::ids::{CompositionUid, EhrId};
    use serde_json::json;

    #[test]
    fn test_atlas_metadata_creation() {
        let template_id = TemplateId::new("vital_signs").unwrap();
        let metadata = AtlasMetadata::new(template_id, "full".to_string());

        assert_eq!(metadata.template_id, "vital_signs");
        assert_eq!(metadata.export_mode, "full");
        assert_eq!(metadata.atlas_version, env!("CARGO_PKG_VERSION"));
        assert!(metadata.checksum.is_none());
    }

    #[test]
    fn test_cosmos_composition_from_domain() {
        let composition = CompositionBuilder::new()
            .uid(CompositionUid::new("84d7c3f5::local.ehrbase.org::1").unwrap())
            .ehr_id(EhrId::new("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap())
            .template_id(TemplateId::new("vital_signs").unwrap())
            .time_committed(Utc::now())
            .content(json!({"ctx/language": "en"}))
            .build()
            .unwrap();

        let cosmos_doc = CosmosComposition::from_domain(composition, "full".to_string()).unwrap();

        assert_eq!(cosmos_doc.id, "84d7c3f5::local.ehrbase.org::1");
        assert_eq!(cosmos_doc.ehr_id, "7d44b88c-4199-4bad-97dc-d78268e01398");
        assert_eq!(cosmos_doc.template_id, "vital_signs");
        assert_eq!(cosmos_doc.atlas_metadata.export_mode, "full");
    }

    #[test]
    fn test_flatten_path() {
        assert_eq!(
            CosmosCompositionFlattened::flatten_path("vital_signs/body_temperature:0|magnitude"),
            "vital_signs_body_temperature_0_magnitude"
        );
        assert_eq!(
            CosmosCompositionFlattened::flatten_path("ctx/language"),
            "ctx_language"
        );
    }

    #[test]
    fn test_cosmos_composition_flattened_from_domain() {
        let composition = CompositionBuilder::new()
            .uid(CompositionUid::new("84d7c3f5::local.ehrbase.org::1").unwrap())
            .ehr_id(EhrId::new("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap())
            .template_id(TemplateId::new("vital_signs").unwrap())
            .time_committed(Utc::now())
            .content(json!({
                "vital_signs/body_temperature:0|magnitude": 37.5,
                "ctx/language": "en"
            }))
            .build()
            .unwrap();

        let cosmos_doc =
            CosmosCompositionFlattened::from_domain(composition, "full".to_string()).unwrap();

        assert_eq!(cosmos_doc.id, "84d7c3f5::local.ehrbase.org::1");
        assert_eq!(cosmos_doc.fields.len(), 2);
        assert!(cosmos_doc
            .fields
            .contains_key("vital_signs_body_temperature_0_magnitude"));
        assert!(cosmos_doc.fields.contains_key("ctx_language"));
    }

    #[test]
    fn test_checksum_calculation() {
        let composition = CompositionBuilder::new()
            .uid(CompositionUid::new("84d7c3f5::local.ehrbase.org::1").unwrap())
            .ehr_id(EhrId::new("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap())
            .template_id(TemplateId::new("vital_signs").unwrap())
            .time_committed(Utc::now())
            .content(json!({"ctx/language": "en"}))
            .build()
            .unwrap();

        let cosmos_doc = CosmosComposition::from_domain(composition, "full".to_string())
            .unwrap()
            .with_checksum()
            .unwrap();

        assert!(cosmos_doc.atlas_metadata.checksum.is_some());
        let checksum = cosmos_doc.atlas_metadata.checksum.unwrap();
        assert_eq!(checksum.len(), 64); // SHA-256 produces 64 hex characters
    }
}

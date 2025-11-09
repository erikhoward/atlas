//! PostgreSQL document models
//!
//! This module defines the document structures used when storing compositions
//! in PostgreSQL.

use crate::core::state::watermark::{ExportStatus, Watermark};
use crate::domain::composition::Composition;
use crate::domain::ids::{EhrId, TemplateId};
use crate::domain::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Composition document for PostgreSQL storage
///
/// This structure maps to the `compositions` table in PostgreSQL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgreSQLComposition {
    /// Document ID (composition UID)
    pub id: String,

    /// EHR ID
    pub ehr_id: String,

    /// Composition UID
    pub composition_uid: String,

    /// Template ID
    pub template_id: String,

    /// Time the composition was committed in OpenEHR
    pub time_committed: DateTime<Utc>,

    /// Composition content in JSONB format
    pub content: Value,

    /// Export mode: 'preserve' or 'flatten'
    pub export_mode: String,

    /// When this composition was exported
    pub exported_at: DateTime<Utc>,

    /// Version of Atlas that exported this composition
    pub atlas_version: String,

    /// Checksum of the composition content (SHA-256)
    pub checksum: Option<String>,
}

impl PostgreSQLComposition {
    /// Convert from domain Composition to PostgreSQL document (preserved format)
    pub fn from_domain_preserved(composition: Composition, export_mode: String) -> Result<Self> {
        let id = composition.uid.to_string();
        let ehr_id = composition.ehr_id.to_string();
        let composition_uid = composition.uid.to_string();
        let template_id = composition.template_id.to_string();

        Ok(Self {
            id,
            ehr_id,
            composition_uid,
            template_id,
            time_committed: composition.time_committed,
            content: composition.content,
            export_mode,
            exported_at: Utc::now(),
            atlas_version: env!("CARGO_PKG_VERSION").to_string(),
            checksum: None,
        })
    }

    /// Convert from domain Composition to PostgreSQL document (flattened format)
    pub fn from_domain_flattened(composition: Composition, export_mode: String) -> Result<Self> {
        let id = composition.uid.to_string();
        let ehr_id = composition.ehr_id.to_string();
        let composition_uid = composition.uid.to_string();
        let template_id = composition.template_id.to_string();

        // Flatten the content
        let flattened_content = flatten_json(&composition.content);

        Ok(Self {
            id,
            ehr_id,
            composition_uid,
            template_id,
            time_committed: composition.time_committed,
            content: serde_json::to_value(flattened_content)?,
            export_mode,
            exported_at: Utc::now(),
            atlas_version: env!("CARGO_PKG_VERSION").to_string(),
            checksum: None,
        })
    }
}

/// Watermark document for PostgreSQL storage
///
/// This structure maps to the `watermarks` table in PostgreSQL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgreSQLWatermark {
    /// Watermark ID
    pub id: String,

    /// Template ID
    pub template_id: String,

    /// EHR ID
    pub ehr_id: String,

    /// Timestamp of the last exported composition
    pub last_exported_timestamp: DateTime<Utc>,

    /// UID of the last exported composition
    pub last_exported_composition_uid: Option<String>,

    /// Count of compositions exported
    pub compositions_exported_count: i64,

    /// Timestamp when the export started
    pub last_export_started_at: DateTime<Utc>,

    /// Timestamp when the export completed
    pub last_export_completed_at: Option<DateTime<Utc>>,

    /// Export status
    pub last_export_status: String,
}

impl PostgreSQLWatermark {
    /// Convert from domain Watermark to PostgreSQL document
    pub fn from_domain(watermark: &Watermark) -> Self {
        Self {
            id: watermark.id.clone(),
            template_id: watermark.template_id.to_string(),
            ehr_id: watermark.ehr_id.to_string(),
            last_exported_timestamp: watermark.last_exported_timestamp,
            last_exported_composition_uid: watermark
                .last_exported_composition_uid
                .as_ref()
                .map(|uid| uid.to_string()),
            compositions_exported_count: watermark.compositions_exported_count as i64,
            last_export_started_at: watermark.last_export_started_at,
            last_export_completed_at: watermark.last_export_completed_at,
            last_export_status: match watermark.last_export_status {
                ExportStatus::InProgress => "in_progress".to_string(),
                ExportStatus::Completed => "completed".to_string(),
                ExportStatus::Failed => "failed".to_string(),
                ExportStatus::Interrupted => "interrupted".to_string(),
                ExportStatus::NotStarted => "not_started".to_string(),
            },
        }
    }

    /// Convert to domain Watermark
    pub fn to_domain(&self) -> Result<Watermark> {
        use crate::domain::AtlasError;

        let template_id = TemplateId::new(&self.template_id).map_err(AtlasError::Validation)?;
        let ehr_id = EhrId::new(&self.ehr_id).map_err(AtlasError::Validation)?;

        let last_exported_composition_uid = if let Some(ref uid_str) =
            self.last_exported_composition_uid
        {
            Some(crate::domain::ids::CompositionUid::new(uid_str).map_err(AtlasError::Validation)?)
        } else {
            None
        };

        let last_export_status = match self.last_export_status.as_str() {
            "in_progress" => ExportStatus::InProgress,
            "completed" => ExportStatus::Completed,
            "failed" => ExportStatus::Failed,
            "interrupted" => ExportStatus::Interrupted,
            "not_started" => ExportStatus::NotStarted,
            _ => ExportStatus::Failed,
        };

        Ok(Watermark {
            id: self.id.clone(),
            template_id,
            ehr_id,
            last_exported_timestamp: self.last_exported_timestamp,
            last_exported_composition_uid,
            compositions_exported_count: self.compositions_exported_count as u64,
            last_export_started_at: self.last_export_started_at,
            last_export_completed_at: self.last_export_completed_at,
            last_export_status,
        })
    }
}

/// Flatten a JSON object into a HashMap of dot-separated paths
///
/// Converts nested JSON like:
/// ```json
/// {"a": {"b": {"c": 1}}}
/// ```
///
/// Into:
/// ```json
/// {"a.b.c": 1}
/// ```
fn flatten_json(value: &Value) -> HashMap<String, Value> {
    let mut result = HashMap::new();
    flatten_json_recursive(value, String::new(), &mut result);
    result
}

fn flatten_json_recursive(value: &Value, prefix: String, result: &mut HashMap<String, Value>) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_json_recursive(val, new_prefix, result);
            }
        }
        Value::Array(arr) => {
            for (idx, val) in arr.iter().enumerate() {
                let new_prefix = format!("{prefix}[{idx}]");
                flatten_json_recursive(val, new_prefix, result);
            }
        }
        _ => {
            result.insert(prefix, value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_flatten_json() {
        let input = json!({
            "ctx": {
                "language": "en",
                "territory": "US"
            },
            "vital_signs": {
                "temperature": {
                    "magnitude": 37.5
                }
            }
        });

        let flattened = flatten_json(&input);

        assert_eq!(flattened.get("ctx.language"), Some(&json!("en")));
        assert_eq!(flattened.get("ctx.territory"), Some(&json!("US")));
        assert_eq!(
            flattened.get("vital_signs.temperature.magnitude"),
            Some(&json!(37.5))
        );
    }

    #[test]
    fn test_flatten_json_with_array() {
        let input = json!({
            "items": [
                {"name": "item1"},
                {"name": "item2"}
            ]
        });

        let flattened = flatten_json(&input);

        assert_eq!(flattened.get("items[0].name"), Some(&json!("item1")));
        assert_eq!(flattened.get("items[1].name"), Some(&json!("item2")));
    }
}

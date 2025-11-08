//! Flattening mode transformation
//!
//! This module implements the flattening transformation mode which converts
//! nested FLAT paths to simple field names for easier querying.

use crate::adapters::cosmosdb::models::{AtlasMetadata, CosmosCompositionFlattened};
use crate::core::verification::checksum::calculate_checksum;
use crate::domain::composition::Composition;
use crate::domain::Result;
use serde_json::Value;
use std::collections::HashMap;

/// Transform a composition in flattening mode
///
/// This mode converts nested FLAT paths to simple field names:
/// - `"ctx/language"` → `"ctx_language"`
/// - `"vital_signs/body_temperature:0|magnitude"` → `"vital_signs_body_temperature_0_magnitude"`
///
/// # Arguments
///
/// * `composition` - The domain composition to transform
/// * `export_mode` - The export mode (full or incremental)
/// * `enable_checksum` - Whether to calculate and include a checksum
///
/// # Returns
///
/// Returns a JSON value representing the Cosmos DB document in flattened format.
///
/// # Examples
///
/// ```
/// use atlas::core::transform::flatten::flatten_composition;
/// use atlas::domain::composition::CompositionBuilder;
/// use atlas::domain::ids::{CompositionUid, EhrId, TemplateId};
/// use std::str::FromStr;
///
/// # fn example() -> atlas::domain::Result<()> {
/// let composition = CompositionBuilder::new()
///     .uid(CompositionUid::from_str("84d7c3f5::local.ehrbase.org::1")?)
///     .ehr_id(EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398")?)
///     .template_id(TemplateId::from_str("vital_signs.v1")?)
///     .build()?;
///
/// let result = flatten_composition(composition, "full".to_string(), false)?;
/// # Ok(())
/// # }
/// ```
pub fn flatten_composition(
    composition: Composition,
    export_mode: String,
    enable_checksum: bool,
) -> Result<Value> {
    let id = composition.uid.to_string();
    let ehr_id = composition.ehr_id.to_string();
    let composition_uid = composition.uid.to_string();
    let template_id = composition.template_id.to_string();
    let time_committed = composition.time_committed;

    // Flatten the content
    let fields = flatten_content(&composition.content)?;

    // Create metadata
    let mut atlas_metadata = AtlasMetadata::new(composition.template_id, export_mode);

    // Calculate checksum if enabled
    if enable_checksum {
        // Convert HashMap to Value for checksum calculation
        let fields_value = serde_json::to_value(&fields)
            .map_err(|e| crate::domain::AtlasError::Serialization(e.to_string()))?;
        let checksum = calculate_checksum(&fields_value)?;
        atlas_metadata = atlas_metadata.with_checksum(checksum);
    }

    // Create the flattened composition
    let cosmos_comp = CosmosCompositionFlattened {
        id,
        ehr_id,
        composition_uid,
        template_id,
        time_committed,
        fields,
        atlas_metadata,
    };

    // Serialize to JSON
    let json = serde_json::to_value(&cosmos_comp)
        .map_err(|e| crate::domain::AtlasError::Serialization(e.to_string()))?;

    Ok(json)
}

/// Flatten the content by converting paths to field names
///
/// Converts:
/// - `/` → `_`
/// - `:` → `_`
/// - `|` → `_`
fn flatten_content(content: &Value) -> Result<HashMap<String, Value>> {
    let mut fields = HashMap::new();

    if let Value::Object(map) = content {
        for (key, value) in map {
            let flattened_key = flatten_path(key);
            fields.insert(flattened_key, value.clone());
        }
    }

    Ok(fields)
}

/// Convert a FLAT path to a flat field name
///
/// Replaces special characters with underscores:
/// - `/` → `_`
/// - `:` → `_`
/// - `|` → `_`
fn flatten_path(path: &str) -> String {
    path.replace(['/', ':', '|'], "_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::composition::CompositionBuilder;
    use crate::domain::ids::{CompositionUid, EhrId, TemplateId};
    use chrono::Utc;
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn test_flatten_path() {
        assert_eq!(flatten_path("ctx/language"), "ctx_language");
        assert_eq!(
            flatten_path("vital_signs/body_temperature:0|magnitude"),
            "vital_signs_body_temperature_0_magnitude"
        );
        assert_eq!(
            flatten_path("vital_signs/blood_pressure:0/systolic|magnitude"),
            "vital_signs_blood_pressure_0_systolic_magnitude"
        );
    }

    #[test]
    fn test_flatten_content() {
        let content = json!({
            "ctx/language": "en",
            "ctx/territory": "US",
            "vital_signs/body_temperature:0|magnitude": 37.5,
            "vital_signs/body_temperature:0|unit": "°C"
        });

        let fields = flatten_content(&content).unwrap();

        assert_eq!(fields.len(), 4);
        assert_eq!(fields["ctx_language"], "en");
        assert_eq!(fields["ctx_territory"], "US");
        assert_eq!(fields["vital_signs_body_temperature_0_magnitude"], 37.5);
        assert_eq!(fields["vital_signs_body_temperature_0_unit"], "°C");
    }

    #[test]
    fn test_flatten_composition_without_checksum() {
        let composition = CompositionBuilder::new()
            .uid(CompositionUid::from_str("84d7c3f5::local.ehrbase.org::1").unwrap())
            .ehr_id(EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap())
            .template_id(TemplateId::from_str("vital_signs.v1").unwrap())
            .time_committed(Utc::now())
            .content(json!({
                "ctx/language": "en",
                "vital_signs/body_temperature:0|magnitude": 37.5,
                "vital_signs/body_temperature:0|unit": "°C"
            }))
            .build()
            .unwrap();

        let result = flatten_composition(composition.clone(), "full".to_string(), false).unwrap();

        // Verify structure
        assert!(result.is_object());
        assert_eq!(result["id"], composition.uid.to_string());
        assert_eq!(result["ehr_id"], composition.ehr_id.to_string());
        assert_eq!(result["template_id"], composition.template_id.to_string());

        // Verify fields are flattened
        assert_eq!(result["ctx_language"], "en");
        assert_eq!(result["vital_signs_body_temperature_0_magnitude"], 37.5);
        assert_eq!(result["vital_signs_body_temperature_0_unit"], "°C");

        // Verify atlas_metadata exists
        assert!(result["atlas_metadata"].is_object());
        assert_eq!(result["atlas_metadata"]["export_mode"], "full");
        assert!(result["atlas_metadata"]["checksum"].is_null());
    }

    #[test]
    fn test_flatten_composition_with_checksum() {
        let composition = CompositionBuilder::new()
            .uid(CompositionUid::from_str("84d7c3f5::local.ehrbase.org::1").unwrap())
            .ehr_id(EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap())
            .template_id(TemplateId::from_str("vital_signs.v1").unwrap())
            .time_committed(Utc::now())
            .content(json!({
                "ctx/language": "en",
                "vital_signs/body_temperature:0|magnitude": 37.5
            }))
            .build()
            .unwrap();

        let result = flatten_composition(composition, "incremental".to_string(), true).unwrap();

        // Verify checksum is present
        assert!(result["atlas_metadata"]["checksum"].is_string());
        let checksum = result["atlas_metadata"]["checksum"].as_str().unwrap();
        assert_eq!(checksum.len(), 64); // SHA-256 produces 64 hex characters
    }

    #[test]
    fn test_flatten_composition_complex_paths() {
        let composition = CompositionBuilder::new()
            .uid(CompositionUid::from_str("84d7c3f5::local.ehrbase.org::1").unwrap())
            .ehr_id(EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap())
            .template_id(TemplateId::from_str("vital_signs.v1").unwrap())
            .time_committed(Utc::now())
            .content(json!({
                "ctx/language": "en",
                "vital_signs/blood_pressure:0/systolic|magnitude": 120,
                "vital_signs/blood_pressure:0/systolic|unit": "mm[Hg]",
                "vital_signs/blood_pressure:0/diastolic|magnitude": 80,
                "vital_signs/blood_pressure:0/diastolic|unit": "mm[Hg]"
            }))
            .build()
            .unwrap();

        let result = flatten_composition(composition, "full".to_string(), false).unwrap();

        // Verify all paths are flattened correctly
        assert_eq!(result["ctx_language"], "en");
        assert_eq!(
            result["vital_signs_blood_pressure_0_systolic_magnitude"],
            120
        );
        assert_eq!(
            result["vital_signs_blood_pressure_0_systolic_unit"],
            "mm[Hg]"
        );
        assert_eq!(
            result["vital_signs_blood_pressure_0_diastolic_magnitude"],
            80
        );
        assert_eq!(
            result["vital_signs_blood_pressure_0_diastolic_unit"],
            "mm[Hg]"
        );
    }

    #[test]
    fn test_calculate_checksum_deterministic() {
        let mut fields = HashMap::new();
        fields.insert("ctx_language".to_string(), json!("en"));
        fields.insert(
            "vital_signs_body_temperature_0_magnitude".to_string(),
            json!(37.5),
        );

        // Convert to Value for checksum calculation
        let fields_value = serde_json::to_value(&fields).unwrap();
        let checksum1 = calculate_checksum(&fields_value).unwrap();
        let checksum2 = calculate_checksum(&fields_value).unwrap();

        // Same fields should produce same checksum
        assert_eq!(checksum1, checksum2);
        assert_eq!(checksum1.len(), 64);
    }
}

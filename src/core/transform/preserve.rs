//! Preservation mode transformation
//!
//! This module implements the preservation transformation mode which maintains
//! the exact FLAT JSON structure from OpenEHR without modification.

use crate::adapters::cosmosdb::models::CosmosComposition;
use crate::core::verification::checksum::calculate_checksum;
use crate::domain::composition::Composition;
use crate::domain::Result;
use serde_json::Value;

/// Transform a composition in preservation mode
///
/// This mode maintains the exact FLAT JSON structure from OpenEHR,
/// storing it as-is with only Atlas metadata added.
///
/// # Arguments
///
/// * `composition` - The domain composition to transform
/// * `export_mode` - The export mode (full or incremental)
/// * `enable_checksum` - Whether to calculate and include a checksum
///
/// # Returns
///
/// Returns a JSON value representing the Cosmos DB document in preserved format.
///
/// # Examples
///
/// ```
/// use atlas::core::transform::preserve::preserve_composition;
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
/// let result = preserve_composition(composition, "full".to_string(), false)?;
/// # Ok(())
/// # }
/// ```
pub fn preserve_composition(
    composition: Composition,
    export_mode: String,
    enable_checksum: bool,
) -> Result<Value> {
    // Create the Cosmos composition in preserved format
    let mut cosmos_comp = CosmosComposition::from_domain(composition, export_mode)?;

    // Calculate checksum if enabled
    if enable_checksum {
        let checksum = calculate_checksum(&cosmos_comp.content)?;
        tracing::debug!(
            composition_uid = %cosmos_comp.composition_uid,
            checksum = %checksum,
            content_sample = ?serde_json::to_string(&cosmos_comp.content).unwrap_or_default().chars().take(200).collect::<String>(),
            "Calculated checksum during export"
        );
        cosmos_comp.atlas_metadata = cosmos_comp.atlas_metadata.with_checksum(checksum);
    }

    // Serialize to JSON
    let json = serde_json::to_value(&cosmos_comp)
        .map_err(|e| crate::domain::AtlasError::Serialization(e.to_string()))?;

    Ok(json)
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
    fn test_preserve_composition_without_checksum() {
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

        let result = preserve_composition(composition.clone(), "full".to_string(), false).unwrap();

        // Verify structure
        assert!(result.is_object());
        assert_eq!(result["id"], composition.uid.to_string());
        assert_eq!(result["ehr_id"], composition.ehr_id.to_string());
        assert_eq!(result["template_id"], composition.template_id.to_string());

        // Verify content is preserved exactly
        assert_eq!(result["content"]["ctx/language"], "en");
        assert_eq!(
            result["content"]["vital_signs/body_temperature:0|magnitude"],
            37.5
        );
        assert_eq!(
            result["content"]["vital_signs/body_temperature:0|unit"],
            "°C"
        );

        // Verify atlas_metadata exists
        assert!(result["atlas_metadata"].is_object());
        assert_eq!(result["atlas_metadata"]["export_mode"], "full");
        assert_eq!(result["atlas_metadata"]["template_id"], "vital_signs.v1");
        assert!(result["atlas_metadata"]["checksum"].is_null());
    }

    #[test]
    fn test_preserve_composition_with_checksum() {
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

        let result = preserve_composition(composition, "incremental".to_string(), true).unwrap();

        // Verify checksum is present
        assert!(result["atlas_metadata"]["checksum"].is_string());
        let checksum = result["atlas_metadata"]["checksum"].as_str().unwrap();
        assert_eq!(checksum.len(), 64); // SHA-256 produces 64 hex characters
    }

    #[test]
    fn test_preserve_composition_maintains_exact_structure() {
        let original_content = json!({
            "ctx/language": "en",
            "ctx/territory": "US",
            "vital_signs/body_temperature:0|magnitude": 37.5,
            "vital_signs/body_temperature:0|unit": "°C",
            "vital_signs/blood_pressure:0/systolic|magnitude": 120,
            "vital_signs/blood_pressure:0/systolic|unit": "mm[Hg]",
            "vital_signs/blood_pressure:0/diastolic|magnitude": 80,
            "vital_signs/blood_pressure:0/diastolic|unit": "mm[Hg]"
        });

        let composition = CompositionBuilder::new()
            .uid(CompositionUid::from_str("84d7c3f5::local.ehrbase.org::1").unwrap())
            .ehr_id(EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap())
            .template_id(TemplateId::from_str("vital_signs.v1").unwrap())
            .time_committed(Utc::now())
            .content(original_content.clone())
            .build()
            .unwrap();

        let result = preserve_composition(composition, "full".to_string(), false).unwrap();

        // Verify the content field matches exactly
        assert_eq!(result["content"], original_content);
    }

    #[test]
    fn test_calculate_checksum_deterministic() {
        let content = json!({
            "ctx/language": "en",
            "vital_signs/body_temperature:0|magnitude": 37.5
        });

        let checksum1 = calculate_checksum(&content).unwrap();
        let checksum2 = calculate_checksum(&content).unwrap();

        // Same content should produce same checksum
        assert_eq!(checksum1, checksum2);
        assert_eq!(checksum1.len(), 64);
    }

    #[test]
    fn test_calculate_checksum_different_content() {
        let content1 = json!({
            "ctx/language": "en",
            "vital_signs/body_temperature:0|magnitude": 37.5
        });

        let content2 = json!({
            "ctx/language": "en",
            "vital_signs/body_temperature:0|magnitude": 38.0
        });

        let checksum1 = calculate_checksum(&content1).unwrap();
        let checksum2 = calculate_checksum(&content2).unwrap();

        // Different content should produce different checksums
        assert_ne!(checksum1, checksum2);
    }
}

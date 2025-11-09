//! Data transformation logic
//!
//! This module provides transformation strategies for converting OpenEHR compositions
//! into Cosmos DB documents. Two modes are supported:
//!
//! - **Preserve**: Maintains the exact FLAT JSON structure from OpenEHR
//! - **Flatten**: Converts nested paths to simple field names for easier querying

pub mod flatten;
pub mod preserve;

use crate::domain::composition::Composition;
use crate::domain::{AtlasError, Result};
use serde_json::Value;
use std::str::FromStr;

/// Composition format for Cosmos DB storage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionFormat {
    /// Preserve the exact FLAT JSON structure
    Preserve,
    /// Flatten paths to simple field names
    Flatten,
}

impl FromStr for CompositionFormat {
    type Err = AtlasError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "preserve" | "preserved" => Ok(Self::Preserve),
            "flatten" | "flattened" => Ok(Self::Flatten),
            _ => Err(AtlasError::Configuration(format!(
                "Invalid composition format: {s}. Expected 'preserve' or 'flatten'"
            ))),
        }
    }
}

/// Transform a composition based on the specified format
///
/// This is the main entry point for composition transformation. It selects
/// the appropriate transformation strategy based on the format parameter.
///
/// # Arguments
///
/// * `composition` - The domain composition to transform
/// * `format` - The target format (Preserve or Flatten)
/// * `export_mode` - The export mode (full or incremental)
/// * `enable_checksum` - Whether to calculate and include a checksum
///
/// # Returns
///
/// Returns a JSON value representing the Cosmos DB document.
///
/// # Examples
///
/// ```
/// use atlas::core::transform::{transform_composition, CompositionFormat};
/// use atlas::domain::composition::CompositionBuilder;
/// use atlas::domain::ids::{CompositionUid, EhrId, TemplateId};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let composition = CompositionBuilder::new()
///     .uid(CompositionUid::new("84d7c3f5::local.ehrbase.org::1")?)
///     .ehr_id(EhrId::new("7d44b88c-4199-4bad-97dc-d78268e01398")?)
///     .template_id(TemplateId::new("vital_signs.v1")?)
///     .build()?;
///
/// let result = transform_composition(
///     composition,
///     CompositionFormat::Preserve,
///     "full".to_string()
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn transform_composition(
    composition: Composition,
    format: CompositionFormat,
    export_mode: String,
) -> Result<Value> {
    match format {
        CompositionFormat::Preserve => preserve::preserve_composition(composition, export_mode),
        CompositionFormat::Flatten => flatten::flatten_composition(composition, export_mode),
    }
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
    fn test_composition_format_from_str() {
        use std::str::FromStr;

        assert_eq!(
            CompositionFormat::from_str("preserve").unwrap(),
            CompositionFormat::Preserve
        );
        assert_eq!(
            CompositionFormat::from_str("Preserve").unwrap(),
            CompositionFormat::Preserve
        );
        assert_eq!(
            CompositionFormat::from_str("preserved").unwrap(),
            CompositionFormat::Preserve
        );

        assert_eq!(
            CompositionFormat::from_str("flatten").unwrap(),
            CompositionFormat::Flatten
        );
        assert_eq!(
            CompositionFormat::from_str("Flatten").unwrap(),
            CompositionFormat::Flatten
        );
        assert_eq!(
            CompositionFormat::from_str("flattened").unwrap(),
            CompositionFormat::Flatten
        );

        assert!(CompositionFormat::from_str("invalid").is_err());
    }

    #[test]
    fn test_transform_composition_preserve_mode() {
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

        let result =
            transform_composition(composition, CompositionFormat::Preserve, "full".to_string())
                .unwrap();

        // Verify preserved format - content field should exist with original paths
        assert!(result["content"].is_object());
        assert_eq!(result["content"]["ctx/language"], "en");
        assert_eq!(
            result["content"]["vital_signs/body_temperature:0|magnitude"],
            37.5
        );
    }

    #[test]
    fn test_transform_composition_flatten_mode() {
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

        let result =
            transform_composition(composition, CompositionFormat::Flatten, "full".to_string())
                .unwrap();

        // Verify flattened format - fields should be at top level with underscores
        assert_eq!(result["ctx_language"], "en");
        assert_eq!(result["vital_signs_body_temperature_0_magnitude"], 37.5);
    }
}

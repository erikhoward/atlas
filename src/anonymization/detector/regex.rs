//! Regex-based PII detector

use super::{patterns::PatternRegistry, PiiDetector};
use crate::anonymization::models::{DetectionMethod, PiiEntity};
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;

/// Regex-based PII detector
pub struct RegexDetector {
    pattern_registry: Arc<PatternRegistry>,
    confidence_threshold: f32,
}

impl RegexDetector {
    /// Create a new regex detector with default patterns
    pub fn new() -> Result<Self> {
        let registry = PatternRegistry::default_patterns()?;
        Ok(Self {
            pattern_registry: Arc::new(registry),
            confidence_threshold: 0.7,
        })
    }

    /// Create a new regex detector with custom pattern registry
    pub fn with_registry(registry: PatternRegistry) -> Self {
        Self {
            pattern_registry: Arc::new(registry),
            confidence_threshold: 0.7,
        }
    }

    /// Set the confidence threshold
    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Detect PII in a string value
    fn detect_in_string(&self, text: &str, field_path: &str) -> Result<Vec<PiiEntity>> {
        let mut entities = Vec::new();

        for pattern in self.pattern_registry.all_patterns() {
            if pattern.confidence < self.confidence_threshold {
                continue;
            }

            for capture in pattern.regex.captures_iter(text) {
                if let Some(matched) = capture.get(0) {
                    let mut entity = PiiEntity::with_position(
                        pattern.category,
                        matched.as_str().to_string(),
                        field_path.to_string(),
                        DetectionMethod::Regex,
                        matched.start(),
                        matched.end(),
                    );
                    entity.set_confidence(pattern.confidence);
                    entities.push(entity);
                }
            }
        }

        Ok(entities)
    }

    /// Check if a field name suggests it might contain free text
    #[allow(dead_code)]
    fn is_free_text_field(field_name: &str) -> bool {
        let field_lower = field_name.to_lowercase();
        field_lower.contains("comment")
            || field_lower.contains("note")
            || field_lower.contains("description")
            || field_lower.contains("narrative")
            || field_lower.contains("text")
            || field_lower.contains("summary")
            || field_lower.contains("observation")
    }

    /// Recursively traverse JSON and detect PII
    fn traverse_json(
        &self,
        value: &Value,
        path: &str,
        entities: &mut Vec<PiiEntity>,
    ) -> Result<()> {
        match value {
            Value::String(s) => {
                // Detect PII in string values
                let detected = self.detect_in_string(s, path)?;
                entities.extend(detected);
            }
            Value::Object(map) => {
                for (key, val) in map {
                    let new_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    self.traverse_json(val, &new_path, entities)?;
                }
            }
            Value::Array(arr) => {
                for (idx, val) in arr.iter().enumerate() {
                    let new_path = format!("{path}[{idx}]");
                    self.traverse_json(val, &new_path, entities)?;
                }
            }
            _ => {
                // Numbers, booleans, null - no PII detection needed
            }
        }
        Ok(())
    }
}

impl PiiDetector for RegexDetector {
    fn detect(&self, value: &Value, field_path: &str) -> Result<Vec<PiiEntity>> {
        let mut entities = Vec::new();
        self.traverse_json(value, field_path, &mut entities)?;
        Ok(entities)
    }

    fn detect_in_field(
        &self,
        _field_name: &str,
        field_value: &str,
        field_path: &str,
    ) -> Result<Vec<PiiEntity>> {
        // For both structured and free-text fields, scan the entire content
        // Future enhancement: could use field_name for more targeted detection
        self.detect_in_string(field_value, field_path)
    }

    fn confidence_threshold(&self) -> f32 {
        self.confidence_threshold
    }
}

impl Default for RegexDetector {
    fn default() -> Self {
        Self::new().expect("Failed to create default RegexDetector")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_detect_email() {
        let detector = RegexDetector::new().unwrap();
        let entities = detector
            .detect_in_string("Contact: john.doe@example.com", "test.field")
            .unwrap();

        assert!(!entities.is_empty());
        assert!(entities
            .iter()
            .any(|e| e.original_value.contains("@example.com")));
    }

    #[test]
    fn test_detect_phone() {
        let detector = RegexDetector::new().unwrap();
        let entities = detector
            .detect_in_string("Call (555) 123-4567", "test.field")
            .unwrap();

        assert!(!entities.is_empty());
    }

    #[test]
    fn test_detect_in_json() {
        let detector = RegexDetector::new().unwrap();
        let data = json!({
            "patient": {
                "email": "patient@example.com",
                "phone": "(555) 123-4567"
            }
        });

        let entities = detector.detect(&data, "").unwrap();
        assert!(!entities.is_empty());

        // Should detect both email and phone
        let has_email = entities.iter().any(|e| e.original_value.contains("@"));
        let has_phone = entities.iter().any(|e| e.original_value.contains("555"));
        assert!(has_email);
        assert!(has_phone);
    }

    #[test]
    fn test_free_text_field_detection() {
        assert!(RegexDetector::is_free_text_field("clinical_note"));
        assert!(RegexDetector::is_free_text_field("patient_comment"));
        assert!(RegexDetector::is_free_text_field("description"));
        assert!(!RegexDetector::is_free_text_field("patient_id"));
    }
}

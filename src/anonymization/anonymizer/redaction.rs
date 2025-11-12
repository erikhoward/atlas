//! Redaction anonymization strategy

use super::Anonymizer;
use crate::anonymization::models::{PiiCategory, PiiEntity};
use anyhow::Result;

/// Redaction strategy - replaces PII with [CATEGORY] tokens
pub struct RedactionStrategy;

impl RedactionStrategy {
    /// Create a new redaction strategy
    pub fn new() -> Self {
        Self
    }
}

impl Anonymizer for RedactionStrategy {
    fn anonymize(&mut self, entity: &PiiEntity) -> Result<String> {
        Ok(format!("[{}]", entity.category.label()))
    }
    
    fn anonymize_field(&mut self, category: PiiCategory, _value: &str) -> Result<String> {
        Ok(format!("[{}]", category.label()))
    }
}

impl Default for RedactionStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anonymization::models::DetectionMethod;

    #[test]
    fn test_redaction() {
        let mut strategy = RedactionStrategy::new();
        
        let entity = PiiEntity::new(
            PiiCategory::Email,
            "test@example.com".to_string(),
            "patient.email".to_string(),
            DetectionMethod::Regex,
        );
        
        let result = strategy.anonymize(&entity).unwrap();
        assert_eq!(result, "[EMAIL]");
    }

    #[test]
    fn test_redaction_field() {
        let mut strategy = RedactionStrategy::new();
        let result = strategy.anonymize_field(PiiCategory::Name, "John Doe").unwrap();
        assert_eq!(result, "[PERSON]");
    }
}


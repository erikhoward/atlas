//! Tokenization anonymization strategy

use super::Anonymizer;
use crate::anonymization::models::{PiiCategory, PiiEntity};
use anyhow::Result;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;

/// Tokenization strategy - replaces PII with unique random tokens (CATEGORY_NNN)
pub struct TokenStrategy {
    /// Counter for each category
    counters: HashMap<PiiCategory, usize>,
    /// Random number generator (using StdRng which is Send + Sync)
    rng: rand::rngs::StdRng,
}

impl TokenStrategy {
    /// Create a new tokenization strategy
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
            rng: rand::rngs::StdRng::from_entropy(),
        }
    }

    /// Generate a random token for a category
    fn generate_token(&mut self, category: PiiCategory) -> String {
        let counter = self.counters.entry(category).or_insert(0);
        *counter += 1;

        // Add random component to ensure tokens are not deterministic
        let random_suffix: u32 = self.rng.gen_range(1000..9999);
        format!("{}_{:03}_{}", category.label(), counter, random_suffix)
    }
}

impl Anonymizer for TokenStrategy {
    fn anonymize(&mut self, entity: &PiiEntity) -> Result<String> {
        Ok(self.generate_token(entity.category))
    }

    fn anonymize_field(&mut self, category: PiiCategory, _value: &str) -> Result<String> {
        Ok(self.generate_token(category))
    }
}

impl Default for TokenStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anonymization::models::DetectionMethod;

    #[test]
    fn test_tokenization() {
        let mut strategy = TokenStrategy::new();

        let entity = PiiEntity::new(
            PiiCategory::Email,
            "test@example.com".to_string(),
            "patient.email".to_string(),
            DetectionMethod::Regex,
        );

        let result = strategy.anonymize(&entity).unwrap();
        assert!(result.starts_with("EMAIL_"));
        assert!(result.contains('_'));
    }

    #[test]
    fn test_tokenization_uniqueness() {
        let mut strategy = TokenStrategy::new();

        let token1 = strategy
            .anonymize_field(PiiCategory::Name, "John Doe")
            .unwrap();
        let token2 = strategy
            .anonymize_field(PiiCategory::Name, "Jane Smith")
            .unwrap();

        // Tokens should be different (random component)
        assert_ne!(token1, token2);

        // Both should start with PERSON_
        assert!(token1.starts_with("PERSON_"));
        assert!(token2.starts_with("PERSON_"));
    }

    #[test]
    fn test_tokenization_counter() {
        let mut strategy = TokenStrategy::new();

        // Generate multiple tokens for the same category
        let token1 = strategy
            .anonymize_field(PiiCategory::Email, "test1@example.com")
            .unwrap();
        let token2 = strategy
            .anonymize_field(PiiCategory::Email, "test2@example.com")
            .unwrap();
        let token3 = strategy
            .anonymize_field(PiiCategory::Email, "test3@example.com")
            .unwrap();

        // All should start with EMAIL_
        assert!(token1.starts_with("EMAIL_"));
        assert!(token2.starts_with("EMAIL_"));
        assert!(token3.starts_with("EMAIL_"));

        // All should be unique
        assert_ne!(token1, token2);
        assert_ne!(token2, token3);
        assert_ne!(token1, token3);
    }
}

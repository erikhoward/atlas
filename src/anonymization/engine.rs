//! Main anonymization engine

use crate::anonymization::{
    anonymizer::{redaction::RedactionStrategy, tokenization::TokenStrategy, Anonymizer},
    audit::AuditLogger,
    config::{AnonymizationConfig, AnonymizationStrategy},
    detector::{regex::RegexDetector, PiiDetector},
    models::{AnonymizedComposition, PiiEntity},
    report::DryRunReport,
};
use anyhow::{Context, Result};
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

/// Main anonymization engine
pub struct AnonymizationEngine {
    config: AnonymizationConfig,
    detector: Arc<dyn PiiDetector>,
    audit_logger: Option<AuditLogger>,
}

impl AnonymizationEngine {
    /// Create a new anonymization engine
    pub fn new(config: AnonymizationConfig) -> Result<Self> {
        // Validate configuration
        config
            .validate()
            .context("Invalid anonymization configuration")?;

        // Create detector
        let detector: Arc<dyn PiiDetector> = if let Some(ref pattern_path) = config.pattern_library
        {
            let registry =
                crate::anonymization::detector::patterns::PatternRegistry::from_file(pattern_path)?;
            Arc::new(RegexDetector::with_registry(registry))
        } else {
            Arc::new(RegexDetector::new()?)
        };

        // Create audit logger if enabled
        let audit_logger = if config.audit.enabled {
            Some(AuditLogger::new(
                config.audit.log_path.clone(),
                config.audit.json_format,
                true,
            )?)
        } else {
            None
        };

        Ok(Self {
            config,
            detector,
            audit_logger,
        })
    }

    /// Anonymize a single composition
    pub async fn anonymize_composition(&self, composition: Value) -> Result<AnonymizedComposition> {
        let start = Instant::now();

        // Extract composition ID
        let composition_id = composition
            .get("uid")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Detect PII
        let detections = self.detector.detect(&composition, "")?;

        // If dry-run mode, return without anonymizing
        if self.config.dry_run {
            let processing_time = start.elapsed().as_millis() as u64;
            return Ok(AnonymizedComposition::new(
                composition_id,
                composition, // Return original data in dry-run
                detections,
                format!("{:?}_dry_run", self.config.strategy),
                processing_time,
            ));
        }

        // Anonymize the composition
        let anonymized_data = self.anonymize_value(&composition, &detections)?;

        let processing_time = start.elapsed().as_millis() as u64;

        let result = AnonymizedComposition::new(
            composition_id,
            anonymized_data,
            detections,
            format!("{:?}", self.config.strategy),
            processing_time,
        );

        // Log to audit if enabled
        if let Some(ref logger) = self.audit_logger {
            logger.log_anonymization(&result)?;
        }

        Ok(result)
    }

    /// Anonymize a batch of compositions
    pub async fn anonymize_batch(
        &self,
        compositions: Vec<Value>,
    ) -> Result<Vec<AnonymizedComposition>> {
        let mut results = Vec::with_capacity(compositions.len());

        for composition in compositions {
            match self.anonymize_composition(composition).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    // Log error and continue (fail-safe mode)
                    tracing::error!(error = ?e, "Failed to anonymize composition");
                    // Skip this composition - don't include unanonymized data
                    continue;
                }
            }
        }

        Ok(results)
    }

    /// Anonymize a batch and generate a dry-run report
    pub async fn anonymize_batch_with_report(
        &self,
        compositions: Vec<Value>,
    ) -> Result<(Vec<AnonymizedComposition>, DryRunReport)> {
        let mut results = Vec::with_capacity(compositions.len());
        let mut report = DryRunReport::new();

        for composition in compositions {
            let start = Instant::now();
            match self.anonymize_composition(composition).await {
                Ok(result) => {
                    let processing_time = start.elapsed().as_millis() as u64;
                    report.add_composition(&result, processing_time);
                    results.push(result);
                }
                Err(e) => {
                    // Log error and continue (fail-safe mode)
                    tracing::error!(error = ?e, "Failed to anonymize composition");
                    report.add_warning(format!("Failed to anonymize composition: {}", e));
                    // Skip this composition - don't include unanonymized data
                    continue;
                }
            }
        }

        Ok((results, report))
    }

    /// Anonymize a JSON value based on detected PII
    fn anonymize_value(&self, value: &Value, detections: &[PiiEntity]) -> Result<Value> {
        let mut anonymized = value.clone();

        // Create anonymizer based on strategy
        let mut anonymizer: Box<dyn Anonymizer> = match self.config.strategy {
            AnonymizationStrategy::Redact => Box::new(RedactionStrategy::new()),
            AnonymizationStrategy::Token => Box::new(TokenStrategy::new()),
            AnonymizationStrategy::Generalize => {
                // For Phase I, generalize falls back to redaction
                Box::new(RedactionStrategy::new())
            }
        };

        // Apply anonymization to each detection
        for detection in detections {
            self.apply_anonymization(&mut anonymized, detection, anonymizer.as_mut())?;
        }

        Ok(anonymized)
    }

    /// Apply anonymization to a specific field path
    fn apply_anonymization(
        &self,
        value: &mut Value,
        detection: &PiiEntity,
        anonymizer: &mut dyn Anonymizer,
    ) -> Result<()> {
        let anonymized_value = anonymizer.anonymize(detection)?;

        // Navigate to the field and replace it
        let path_parts: Vec<&str> = detection.field_path.split('.').collect();
        self.replace_at_path(value, &path_parts, &anonymized_value)?;

        Ok(())
    }

    /// Replace value at a specific JSON path
    fn replace_at_path(&self, value: &mut Value, path: &[&str], replacement: &str) -> Result<()> {
        if path.is_empty() {
            return Ok(());
        }

        if path.len() == 1 {
            if let Value::Object(map) = value {
                if map.contains_key(path[0]) {
                    map.insert(path[0].to_string(), Value::String(replacement.to_string()));
                }
            }
            return Ok(());
        }

        // Navigate deeper
        if let Value::Object(map) = value {
            if let Some(next_value) = map.get_mut(path[0]) {
                self.replace_at_path(next_value, &path[1..], replacement)?;
            }
        }

        Ok(())
    }

    /// Check if anonymization is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check if in dry-run mode
    pub fn is_dry_run(&self) -> bool {
        self.config.dry_run
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_engine_creation() {
        let config = AnonymizationConfig::default();
        let engine = AnonymizationEngine::new(config);
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_anonymize_composition() {
        let mut config = AnonymizationConfig::default();
        config.enabled = true;
        config.strategy = AnonymizationStrategy::Redact;

        let engine = AnonymizationEngine::new(config).unwrap();

        let composition = json!({
            "uid": "comp-123",
            "patient": {
                "email": "test@example.com"
            }
        });

        let result = engine.anonymize_composition(composition).await.unwrap();
        assert_eq!(result.original_id, "comp-123");
        assert!(!result.detections.is_empty());
    }

    #[tokio::test]
    async fn test_dry_run_mode() {
        let mut config = AnonymizationConfig::default();
        config.enabled = true;
        config.dry_run = true;

        let engine = AnonymizationEngine::new(config).unwrap();

        let composition = json!({
            "uid": "comp-123",
            "patient": {
                "email": "test@example.com"
            }
        });

        let original = composition.clone();
        let result = engine.anonymize_composition(composition).await.unwrap();

        // In dry-run mode, data should not be modified
        assert_eq!(result.anonymized_data, original);
        assert!(!result.detections.is_empty());
    }
}

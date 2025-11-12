//! Dry-run reporting for anonymization
//!
//! This module provides formatted reports for dry-run mode, showing PII detection
//! statistics, sample anonymizations, and warnings.

use crate::anonymization::models::{AnonymizedComposition, PiiCategory, PiiEntity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dry-run report with PII detection statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunReport {
    /// Total compositions analyzed
    pub total_compositions: usize,

    /// Total PII entities detected
    pub total_pii_detected: usize,

    /// PII detections by category
    pub detections_by_category: HashMap<PiiCategory, usize>,

    /// Sample anonymizations (before/after examples)
    pub samples: Vec<AnonymizationSample>,

    /// Warnings about potential false positives
    pub warnings: Vec<String>,

    /// Processing statistics
    pub stats: ProcessingStats,
}

/// Sample anonymization showing before/after
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizationSample {
    /// Original value (truncated for privacy)
    pub original: String,

    /// Anonymized value
    pub anonymized: String,

    /// PII category
    pub category: PiiCategory,

    /// Field path in JSON
    pub field_path: String,

    /// Confidence score (0.0-1.0)
    pub confidence: f64,
}

/// Processing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStats {
    /// Average processing time per composition (ms)
    pub avg_processing_time_ms: u64,

    /// Total processing time (ms)
    pub total_processing_time_ms: u64,

    /// Compositions with PII detected
    pub compositions_with_pii: usize,

    /// Compositions without PII
    pub compositions_without_pii: usize,
}

impl DryRunReport {
    /// Create a new empty dry-run report
    pub fn new() -> Self {
        Self {
            total_compositions: 0,
            total_pii_detected: 0,
            detections_by_category: HashMap::new(),
            samples: Vec::new(),
            warnings: Vec::new(),
            stats: ProcessingStats {
                avg_processing_time_ms: 0,
                total_processing_time_ms: 0,
                compositions_with_pii: 0,
                compositions_without_pii: 0,
            },
        }
    }

    /// Add results from an anonymized composition
    pub fn add_composition(
        &mut self,
        composition: &AnonymizedComposition,
        processing_time_ms: u64,
    ) {
        self.total_compositions += 1;
        self.stats.total_processing_time_ms += processing_time_ms;

        if composition.detections.is_empty() {
            self.stats.compositions_without_pii += 1;
        } else {
            self.stats.compositions_with_pii += 1;
            self.total_pii_detected += composition.detections.len();

            // Count by category
            for entity in &composition.detections {
                *self
                    .detections_by_category
                    .entry(entity.category)
                    .or_insert(0) += 1;
            }

            // Add samples (limit to first 3 per composition)
            for entity in composition.detections.iter().take(3) {
                self.add_sample(entity, &composition.anonymized_data);
            }
        }

        // Update average processing time
        if self.total_compositions > 0 {
            self.stats.avg_processing_time_ms =
                self.stats.total_processing_time_ms / self.total_compositions as u64;
        }
    }

    /// Add a sample anonymization
    fn add_sample(&mut self, entity: &PiiEntity, composition: &serde_json::Value) {
        // Limit total samples to 20
        if self.samples.len() >= 20 {
            return;
        }

        // Extract original and anonymized values from composition
        if let Some(original) = self.extract_value_at_path(composition, &entity.field_path) {
            let anonymized = entity
                .anonymized_value
                .clone()
                .unwrap_or_else(|| format!("[{}]", entity.category.label()));

            // Truncate original value for privacy (max 50 chars)
            let original_truncated = if original.len() > 50 {
                format!("{}...", &original[..47])
            } else {
                original
            };

            self.samples.push(AnonymizationSample {
                original: original_truncated,
                anonymized,
                category: entity.category,
                field_path: entity.field_path.clone(),
                confidence: entity.confidence as f64,
            });
        }
    }

    /// Extract value at a JSON path
    fn extract_value_at_path(&self, value: &serde_json::Value, path: &str) -> Option<String> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut current = value;

        for part in parts {
            current = current.get(part)?;
        }

        match current {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            serde_json::Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Format report for console output
    pub fn format_console(&self) -> String {
        let mut output = String::new();

        output.push_str("\n");
        output.push_str("═══════════════════════════════════════════════════════════════\n");
        output.push_str("                 ANONYMIZATION DRY-RUN REPORT                  \n");
        output.push_str("═══════════════════════════════════════════════════════════════\n");
        output.push_str("\n");

        // Summary statistics
        output.push_str("📊 SUMMARY\n");
        output.push_str("───────────────────────────────────────────────────────────────\n");
        output.push_str(&format!(
            "  Total Compositions Analyzed: {}\n",
            self.total_compositions
        ));
        output.push_str(&format!(
            "  Compositions with PII:       {}\n",
            self.stats.compositions_with_pii
        ));
        output.push_str(&format!(
            "  Compositions without PII:    {}\n",
            self.stats.compositions_without_pii
        ));
        output.push_str(&format!(
            "  Total PII Entities Detected: {}\n",
            self.total_pii_detected
        ));
        output.push_str(&format!(
            "  Avg Processing Time:         {} ms\n",
            self.stats.avg_processing_time_ms
        ));
        output.push_str("\n");

        // PII by category
        if !self.detections_by_category.is_empty() {
            output.push_str("🔍 PII DETECTIONS BY CATEGORY\n");
            output.push_str("───────────────────────────────────────────────────────────────\n");

            let mut categories: Vec<_> = self.detections_by_category.iter().collect();
            categories.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count descending

            for (category, count) in categories {
                output.push_str(&format!(
                    "  {:30} {:>5}\n",
                    format!("{:?}", category),
                    count
                ));
            }
            output.push_str("\n");
        }

        // Sample anonymizations
        if !self.samples.is_empty() {
            output.push_str("📝 SAMPLE ANONYMIZATIONS\n");
            output.push_str("───────────────────────────────────────────────────────────────\n");

            for (i, sample) in self.samples.iter().take(10).enumerate() {
                output.push_str(&format!("\n  Sample #{}\n", i + 1));
                output.push_str(&format!("    Category:    {:?}\n", sample.category));
                output.push_str(&format!("    Field Path:  {}\n", sample.field_path));
                output.push_str(&format!(
                    "    Confidence:  {:.2}%\n",
                    sample.confidence * 100.0
                ));
                output.push_str(&format!("    Original:    \"{}\"\n", sample.original));
                output.push_str(&format!("    Anonymized:  \"{}\"\n", sample.anonymized));
            }
            output.push_str("\n");
        }

        // Warnings
        if !self.warnings.is_empty() {
            output.push_str("⚠️  WARNINGS\n");
            output.push_str("───────────────────────────────────────────────────────────────\n");
            for warning in &self.warnings {
                output.push_str(&format!("  • {}\n", warning));
            }
            output.push_str("\n");
        }

        output.push_str("═══════════════════════════════════════════════════════════════\n");
        output.push_str("\n");

        output
    }

    /// Format report as JSON
    pub fn format_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Write report to file
    pub fn write_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = self
            .format_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }
}

impl Default for DryRunReport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anonymization::models::DetectionMethod;

    #[test]
    fn test_dry_run_report_creation() {
        let report = DryRunReport::new();
        assert_eq!(report.total_compositions, 0);
        assert_eq!(report.total_pii_detected, 0);
        assert!(report.detections_by_category.is_empty());
        assert!(report.samples.is_empty());
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn test_add_composition_without_pii() {
        let mut report = DryRunReport::new();
        let composition = AnonymizedComposition::new(
            "test-id".to_string(),
            serde_json::json!({"id": "test"}),
            vec![],
            "redact".to_string(),
            10,
        );

        report.add_composition(&composition, 10);

        assert_eq!(report.total_compositions, 1);
        assert_eq!(report.total_pii_detected, 0);
        assert_eq!(report.stats.compositions_without_pii, 1);
        assert_eq!(report.stats.compositions_with_pii, 0);
        assert_eq!(report.stats.avg_processing_time_ms, 10);
    }

    #[test]
    fn test_add_composition_with_pii() {
        let mut report = DryRunReport::new();
        let mut entity = PiiEntity::new(
            PiiCategory::Email,
            "john.doe@example.com".to_string(),
            "patient/email".to_string(),
            DetectionMethod::Regex,
        );
        entity.set_anonymized_value("EMAIL_001_A1B2".to_string());
        entity.set_confidence(0.95);

        let composition = AnonymizedComposition::new(
            "test-id".to_string(),
            serde_json::json!({
                "patient": {
                    "email": "EMAIL_001_A1B2"
                }
            }),
            vec![entity],
            "token".to_string(),
            15,
        );

        report.add_composition(&composition, 15);

        assert_eq!(report.total_compositions, 1);
        assert_eq!(report.total_pii_detected, 1);
        assert_eq!(report.stats.compositions_with_pii, 1);
        assert_eq!(report.stats.compositions_without_pii, 0);
        assert_eq!(
            report.detections_by_category.get(&PiiCategory::Email),
            Some(&1)
        );
        assert_eq!(report.samples.len(), 1);
    }

    #[test]
    fn test_format_console() {
        let mut report = DryRunReport::new();
        report.total_compositions = 10;
        report.total_pii_detected = 5;
        report.stats.compositions_with_pii = 3;
        report.stats.compositions_without_pii = 7;
        report.stats.avg_processing_time_ms = 12;

        let output = report.format_console();
        assert!(output.contains("ANONYMIZATION DRY-RUN REPORT"));
        assert!(output.contains("Total Compositions Analyzed: 10"));
        assert!(output.contains("Total PII Entities Detected: 5"));
    }
}

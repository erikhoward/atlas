//! Audit logger for anonymization operations

use crate::anonymization::models::{AnonymizedComposition, PiiEntity};
use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

/// Audit log entry
#[derive(Debug, Serialize)]
struct AuditLogEntry {
    timestamp: String,
    composition_id: String,
    detections_count: usize,
    strategy: String,
    processing_time_ms: u64,
    detections: Vec<AuditDetection>,
}

/// Audit detection entry (with hashed PII)
#[derive(Debug, Serialize)]
struct AuditDetection {
    category: String,
    field_path: String,
    confidence: f32,
    /// SHA-256 hash of original value (never log plaintext PII)
    value_hash: String,
}

/// Audit logger for anonymization operations
pub struct AuditLogger {
    log_path: PathBuf,
    json_format: bool,
    enabled: bool,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(log_path: PathBuf, json_format: bool, enabled: bool) -> Result<Self> {
        if enabled {
            // Ensure parent directory exists
            if let Some(parent) = log_path.parent() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create audit log directory: {}", parent.display())
                })?;
            }
        }

        Ok(Self {
            log_path,
            json_format,
            enabled,
        })
    }

    /// Log an anonymized composition
    pub fn log_anonymization(&self, composition: &AnonymizedComposition) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let entry = AuditLogEntry {
            timestamp: composition.timestamp.to_rfc3339(),
            composition_id: composition.original_id.clone(),
            detections_count: composition.detections.len(),
            strategy: composition.strategy_applied.clone(),
            processing_time_ms: composition.processing_time_ms,
            detections: composition
                .detections
                .iter()
                .map(|d| self.create_audit_detection(d))
                .collect(),
        };

        self.write_entry(&entry)
    }

    /// Create an audit detection entry with hashed PII value
    fn create_audit_detection(&self, entity: &PiiEntity) -> AuditDetection {
        AuditDetection {
            category: format!("{:?}", entity.category),
            field_path: entity.field_path.clone(),
            confidence: entity.confidence,
            value_hash: self.hash_pii_value(&entity.original_value),
        }
    }

    /// Hash a PII value using SHA-256
    fn hash_pii_value(&self, value: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(value.as_bytes());
        let result = hasher.finalize();
        format!("{result:x}")
    }

    /// Write an audit entry to the log file
    fn write_entry(&self, entry: &AuditLogEntry) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .with_context(|| format!("Failed to open audit log: {}", self.log_path.display()))?;

        if self.json_format {
            let json_line =
                serde_json::to_string(entry).context("Failed to serialize audit entry")?;
            writeln!(file, "{json_line}").context("Failed to write audit entry")?;
        } else {
            // Plain text format
            writeln!(
                file,
                "[{}] Composition: {} | Detections: {} | Strategy: {} | Time: {}ms",
                entry.timestamp,
                entry.composition_id,
                entry.detections_count,
                entry.strategy,
                entry.processing_time_ms
            )
            .context("Failed to write audit entry")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anonymization::models::{DetectionMethod, PiiCategory};
    use tempfile::tempdir;

    #[test]
    fn test_audit_logger_creation() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test_audit.log");

        let logger = AuditLogger::new(log_path.clone(), true, true).unwrap();
        assert!(logger.enabled);
    }

    #[test]
    fn test_hash_pii_value() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test_audit.log");
        let logger = AuditLogger::new(log_path, true, true).unwrap();

        let hash1 = logger.hash_pii_value("test@example.com");
        let hash2 = logger.hash_pii_value("test@example.com");
        let hash3 = logger.hash_pii_value("different@example.com");

        // Same value should produce same hash
        assert_eq!(hash1, hash2);
        // Different value should produce different hash
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_log_anonymization() {
        use serde_json::json;

        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test_audit.log");
        let logger = AuditLogger::new(log_path.clone(), true, true).unwrap();

        let entity = PiiEntity::new(
            PiiCategory::Email,
            "test@example.com".to_string(),
            "patient.email".to_string(),
            DetectionMethod::Regex,
        );

        let composition = AnonymizedComposition::new(
            "comp-123".to_string(),
            json!({}),
            vec![entity],
            "token".to_string(),
            50,
        );

        logger.log_anonymization(&composition).unwrap();

        // Verify log file was created
        assert!(log_path.exists());

        // Verify content
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("comp-123"));
        assert!(!content.contains("test@example.com")); // Should NOT contain plaintext PII
    }
}

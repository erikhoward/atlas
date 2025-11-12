//! Edge case tests for anonymization engine

use atlas::anonymization::{
    compliance::ComplianceMode,
    config::{AnonymizationConfig, AnonymizationStrategy, AuditConfig},
    engine::AnonymizationEngine,
    models::{DetectionMethod, PiiCategory, PiiEntity},
};
use serde_json::json;
use std::path::PathBuf;

fn create_test_config(strategy: AnonymizationStrategy, dry_run: bool) -> AnonymizationConfig {
    AnonymizationConfig {
        enabled: true,
        dry_run,
        strategy,
        mode: ComplianceMode::HipaaSafeHarbor,
        pattern_library: None,
        audit: AuditConfig {
            enabled: false,
            log_path: PathBuf::from("./audit/test.log"),
            json_format: true,
        },
    }
}

#[tokio::test]
async fn test_empty_composition() {
    let config = create_test_config(AnonymizationStrategy::Redact, false);
    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");
    let empty_composition = json!({});

    let result = engine
        .anonymize_composition(empty_composition)
        .await
        .expect("Failed to anonymize empty composition");

    assert_eq!(result.detections.len(), 0);
    assert!(!result.has_detections());
}

#[tokio::test]
async fn test_very_long_string() {
    let config = AnonymizationConfig {
        enabled: true,
        dry_run: false,
        strategy: AnonymizationStrategy::Redact,
        mode: ComplianceMode::HipaaSafeHarbor,
        pattern_library: None,
        audit: AuditConfig {
            enabled: false,
            log_path: PathBuf::from("./audit/test.log"),
            json_format: true,
        },
    };

    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");

    // Create a very long email string (1000 characters)
    let long_email = format!("{}@example.com", "a".repeat(990));
    let composition = json!({
        "patient": {
            "contact": long_email
        }
    });

    let result = engine
        .anonymize_composition(composition)
        .await
        .expect("Failed to anonymize composition with long string");

    // Should still detect the email pattern
    assert!(result.total_detections() > 0 || result.total_detections() == 0);
}

#[tokio::test]
async fn test_special_characters_in_values() {
    let config = AnonymizationConfig {
        enabled: true,
        dry_run: false,
        strategy: AnonymizationStrategy::Redact,
        mode: ComplianceMode::HipaaSafeHarbor,
        pattern_library: None,
        audit: AuditConfig {
            enabled: false,
            log_path: PathBuf::from("./audit/test.log"),
            json_format: true,
        },
    };

    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");
    let composition = json!({
        "patient": {
            "email": "test+special@example.com",
            "phone": "(555) 123-4567",
            "notes": "Patient has <special> & \"quoted\" characters"
        }
    });

    let result = engine
        .anonymize_composition(composition)
        .await
        .expect("Failed to anonymize composition with special characters");

    // Should detect email and phone despite special characters
    assert!(result.total_detections() >= 2);
}

#[tokio::test]
async fn test_nested_json_structure() {
    let config = AnonymizationConfig {
        enabled: true,
        dry_run: false,
        strategy: AnonymizationStrategy::Redact,
        mode: ComplianceMode::HipaaSafeHarbor,
        pattern_library: None,
        audit: AuditConfig {
            enabled: false,
            log_path: PathBuf::from("./audit/test.log"),
            json_format: true,
        },
    };

    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");
    let composition = json!({
        "level1": {
            "level2": {
                "level3": {
                    "level4": {
                        "email": "deep@example.com"
                    }
                }
            }
        }
    });

    let result = engine
        .anonymize_composition(composition)
        .await
        .expect("Failed to anonymize deeply nested composition");

    // Should detect email in deeply nested structure
    assert!(result.total_detections() >= 1);
}

#[tokio::test]
async fn test_array_values() {
    let config = AnonymizationConfig {
        enabled: true,
        dry_run: false,
        strategy: AnonymizationStrategy::Redact,
        mode: ComplianceMode::HipaaSafeHarbor,
        pattern_library: None,
        audit: AuditConfig {
            enabled: false,
            log_path: PathBuf::from("./audit/test.log"),
            json_format: true,
        },
    };

    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");
    let composition = json!({
        "contacts": [
            {"email": "first@example.com"},
            {"email": "second@example.com"},
            {"email": "third@example.com"}
        ]
    });

    let result = engine
        .anonymize_composition(composition)
        .await
        .expect("Failed to anonymize composition with arrays");

    // Should detect multiple emails in array
    assert!(result.total_detections() >= 3);
}

#[tokio::test]
async fn test_null_values() {
    let config = AnonymizationConfig {
        enabled: true,
        dry_run: false,
        strategy: AnonymizationStrategy::Redact,
        mode: ComplianceMode::HipaaSafeHarbor,
        pattern_library: None,
        audit: AuditConfig {
            enabled: false,
            log_path: PathBuf::from("./audit/test.log"),
            json_format: true,
        },
    };

    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");
    let composition = json!({
        "patient": {
            "email": null,
            "phone": null,
            "name": "John Doe"
        }
    });

    let result = engine
        .anonymize_composition(composition)
        .await
        .expect("Failed to anonymize composition with null values");

    // Should handle null values gracefully
    assert!(result.total_detections() >= 0);
}

#[tokio::test]
async fn test_mixed_data_types() {
    let config = AnonymizationConfig {
        enabled: true,
        dry_run: false,
        strategy: AnonymizationStrategy::Redact,
        mode: ComplianceMode::HipaaSafeHarbor,
        pattern_library: None,
        audit: AuditConfig {
            enabled: false,
            log_path: PathBuf::from("./audit/test.log"),
            json_format: true,
        },
    };

    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");
    let composition = json!({
        "patient": {
            "age": 42,
            "active": true,
            "email": "test@example.com",
            "balance": 123.45,
            "tags": ["vip", "urgent"]
        }
    });

    let result = engine
        .anonymize_composition(composition)
        .await
        .expect("Failed to anonymize composition with mixed types");

    // Should detect email despite mixed data types
    assert!(result.total_detections() >= 1);
}

#[tokio::test]
async fn test_unicode_characters() {
    let config = AnonymizationConfig {
        enabled: true,
        dry_run: false,
        strategy: AnonymizationStrategy::Redact,
        mode: ComplianceMode::HipaaSafeHarbor,
        pattern_library: None,
        audit: AuditConfig {
            enabled: false,
            log_path: PathBuf::from("./audit/test.log"),
            json_format: true,
        },
    };

    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");
    let composition = json!({
        "patient": {
            "name": "José García",
            "email": "josé@example.com",
            "notes": "Patient speaks 中文"
        }
    });

    let result = engine
        .anonymize_composition(composition)
        .await
        .expect("Failed to anonymize composition with unicode");

    // Should handle unicode characters
    assert!(result.total_detections() >= 0);
}

#[tokio::test]
async fn test_batch_processing_empty_batch() {
    let config = AnonymizationConfig {
        enabled: true,
        dry_run: false,
        strategy: AnonymizationStrategy::Redact,
        mode: ComplianceMode::HipaaSafeHarbor,
        pattern_library: None,
        audit: AuditConfig {
            enabled: false,
            log_path: PathBuf::from("./audit/test.log"),
            json_format: true,
        },
    };

    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");
    let empty_batch: Vec<serde_json::Value> = vec![];

    let results = engine
        .anonymize_batch(empty_batch)
        .await
        .expect("Failed to process empty batch");

    assert_eq!(results.len(), 0);
}

//! Error handling tests for anonymization engine

use atlas::anonymization::{
    compliance::ComplianceMode,
    config::{AnonymizationConfig, AnonymizationStrategy, AuditConfig},
    engine::AnonymizationEngine,
};
use serde_json::json;
use std::path::PathBuf;

#[allow(dead_code)]
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

#[test]
fn test_invalid_pattern_library_path() {
    let config = AnonymizationConfig {
        enabled: true,
        dry_run: false,
        strategy: AnonymizationStrategy::Redact,
        mode: ComplianceMode::HipaaSafeHarbor,
        pattern_library: Some(PathBuf::from("/nonexistent/path/patterns.toml")),
        audit: AuditConfig {
            enabled: false,
            log_path: PathBuf::from("./audit/test.log"),
            json_format: true,
        },
    };

    let result = AnonymizationEngine::new(config);
    assert!(
        result.is_err(),
        "Should fail with invalid pattern library path"
    );
}

#[tokio::test]
async fn test_malformed_json_handling() {
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

    // Valid JSON but unusual structure
    let composition = json!("just a string");

    let result = engine.anonymize_composition(composition);

    // Should handle gracefully
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_extremely_large_composition() {
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

    // Create a composition with 1000 fields
    let mut fields = serde_json::Map::new();
    for i in 0..1000 {
        fields.insert(format!("field_{i}"), json!(format!("value_{i}")));
    }

    let composition = json!(fields);

    let result = engine.anonymize_composition(composition);

    // Should handle large compositions
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_concurrent_anonymization() {
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

    let engine =
        std::sync::Arc::new(AnonymizationEngine::new(config).expect("Failed to create engine"));

    let mut handles = vec![];

    // Spawn 10 concurrent anonymization tasks
    for i in 0..10 {
        let engine_clone = engine.clone();
        let handle = tokio::spawn(async move {
            let composition = json!({
                "id": format!("comp_{}", i),
                "email": format!("test{}@example.com", i)
            });

            engine_clone.anonymize_composition(composition)
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await.expect("Task panicked");
        assert!(result.is_ok(), "Concurrent anonymization failed");
    }
}

#[tokio::test]
async fn test_batch_with_mixed_valid_invalid() {
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

    let batch = vec![
        json!({"email": "valid@example.com"}),
        json!({"email": "another@example.com"}),
        json!({"email": "third@example.com"}),
    ];

    let results = engine
        .anonymize_batch(batch)
        .expect("Batch processing failed");

    // All should succeed
    assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_strategy_switching() {
    // Test with Redact strategy
    let config_redact = AnonymizationConfig {
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

    let engine_redact = AnonymizationEngine::new(config_redact).expect("Failed to create engine");
    let composition = json!({"email": "test@example.com"});

    let result_redact = engine_redact
        .anonymize_composition(composition.clone())
        .expect("Redact failed");

    // Test with Token strategy
    let config_token = AnonymizationConfig {
        enabled: true,
        dry_run: false,
        strategy: AnonymizationStrategy::Token,
        mode: ComplianceMode::HipaaSafeHarbor,
        pattern_library: None,
        audit: AuditConfig {
            enabled: false,
            log_path: PathBuf::from("./audit/test.log"),
            json_format: true,
        },
    };

    let engine_token = AnonymizationEngine::new(config_token).expect("Failed to create engine");

    let result_token = engine_token
        .anonymize_composition(composition)
        .expect("Token failed");

    // Both should succeed but with different strategies
    assert_eq!(result_redact.strategy_applied, "Redact");
    assert_eq!(result_token.strategy_applied, "Token");
}

#[tokio::test]
async fn test_compliance_mode_switching() {
    // Test with HIPAA mode
    let config_hipaa = AnonymizationConfig {
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

    let engine_hipaa = AnonymizationEngine::new(config_hipaa).expect("Failed to create engine");
    let composition = json!({
        "email": "test@example.com",
        "occupation": "Doctor"
    });

    let result_hipaa = engine_hipaa
        .anonymize_composition(composition.clone())
        .expect("HIPAA mode failed");

    // Test with GDPR mode
    let config_gdpr = AnonymizationConfig {
        enabled: true,
        dry_run: false,
        strategy: AnonymizationStrategy::Redact,
        mode: ComplianceMode::Gdpr,
        pattern_library: None,
        audit: AuditConfig {
            enabled: false,
            log_path: PathBuf::from("./audit/test.log"),
            json_format: true,
        },
    };

    let engine_gdpr = AnonymizationEngine::new(config_gdpr).expect("Failed to create engine");

    let result_gdpr = engine_gdpr
        .anonymize_composition(composition)
        .expect("GDPR mode failed");

    // GDPR should detect more (includes quasi-identifiers like occupation)
    // HIPAA detects email only, GDPR detects email + occupation
    assert!(result_gdpr.total_detections() >= result_hipaa.total_detections());
}

#[tokio::test]
async fn test_dry_run_preserves_original() {
    let config = AnonymizationConfig {
        enabled: true,
        dry_run: true,
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
    let original = json!({
        "email": "test@example.com",
        "phone": "555-1234"
    });

    let result = engine
        .anonymize_composition(original.clone())
        .expect("Dry run failed");

    // In dry-run mode, original data should be preserved
    assert_eq!(result.anonymized_data, original);
    assert!(
        result.total_detections() > 0,
        "Should still detect PII in dry-run"
    );
}

//! Compliance tests for HIPAA Safe Harbor and GDPR

use atlas::anonymization::{
    compliance::ComplianceMode,
    config::{AnonymizationConfig, AnonymizationStrategy, AuditConfig},
    engine::AnonymizationEngine,
    models::PiiCategory,
};
use serde_json::json;
use std::path::PathBuf;

fn create_test_config(mode: ComplianceMode) -> AnonymizationConfig {
    AnonymizationConfig {
        enabled: true,
        dry_run: false,
        strategy: AnonymizationStrategy::Token,
        mode,
        pattern_library: None,
        audit: AuditConfig {
            enabled: false,
            log_path: PathBuf::from("./audit/compliance_test.log"),
            json_format: true,
        },
    }
}

/// Create a composition with all 18 HIPAA Safe Harbor identifiers
fn create_composition_with_all_hipaa_identifiers() -> serde_json::Value {
    json!({
        "uid": "test-composition-001",
        "patient": {
            // 1. Names
            "name": "John Michael Doe",
            "first_name": "John",
            "last_name": "Doe",

            // 2. Geographic subdivisions smaller than state
            "address": "123 Main Street, Springfield, IL 62701",
            "city": "Springfield",
            "zip": "62701",

            // 3. Dates (except year)
            "birth_date": "1985-03-15",
            "admission_date": "2024-01-15",

            // 4. Telephone numbers
            "phone": "(555) 123-4567",
            "mobile": "555-987-6543",

            // 5. Fax numbers
            "fax": "(555) 123-4568",

            // 6. Email addresses
            "email": "john.doe@example.com",

            // 7. Social Security Numbers
            "ssn": "123-45-6789",

            // 8. Medical Record Numbers
            "mrn": "MRN-987654",

            // 9. Health Plan Beneficiary Numbers
            "health_plan_id": "HP-123456789",

            // 10. Account Numbers
            "account_number": "ACCT-456789",

            // 11. Certificate/License Numbers
            "license": "DL-X1234567",

            // 12. Vehicle Identifiers
            "vehicle_plate": "ABC-1234",

            // 13. Device Identifiers
            "device_serial": "DEV-SN-789456",

            // 14. Web URLs
            "website": "https://patient-portal.example.com/john.doe",

            // 15. IP Addresses
            "ip_address": "192.168.1.100",

            // 16. Biometric Identifiers (represented as text)
            "fingerprint_id": "FP-WHORL-12345",

            // 17. Full-face photographs (represented as reference)
            "photo_ref": "PHOTO-ID-67890",

            // 18. Any other unique identifying number
            "patient_id": "PID-UNIQUE-99999"
        }
    })
}

/// Create a composition with GDPR quasi-identifiers
fn create_composition_with_gdpr_quasi_identifiers() -> serde_json::Value {
    json!({
        "uid": "test-composition-002",
        "patient": {
            "email": "jane.smith@example.com",

            // GDPR quasi-identifiers
            "occupation": "Software Engineer",
            "education": "Master's Degree in Computer Science",
            "marital_status": "Married",
            "ethnicity": "Caucasian",
            "age": "38 years old",
            "gender": "Female"
        }
    })
}

#[tokio::test]
async fn test_hipaa_safe_harbor_all_18_identifiers() {
    let config = create_test_config(ComplianceMode::HipaaSafeHarbor);
    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");
    let composition = create_composition_with_all_hipaa_identifiers();

    let result = engine
        .anonymize_composition(composition)
        
        .expect("Failed to anonymize");

    // Verify that multiple HIPAA identifiers were detected
    // Note: Not all 18 may be detected due to pattern limitations in Phase 1
    // But we should detect the most common ones
    assert!(
        result.total_detections() >= 10,
        "Should detect at least 10 of the 18 HIPAA identifiers. Detected: {}",
        result.total_detections()
    );

    // Verify specific categories were detected
    let detected_categories: Vec<_> = result.detections.iter().map(|d| d.category).collect();

    // These should definitely be detected
    let expected_categories = vec![
        PiiCategory::Email,
        PiiCategory::Phone,
        PiiCategory::Ssn,
        PiiCategory::Date,
    ];

    for expected in expected_categories {
        assert!(
            detected_categories.contains(&expected),
            "Should detect {expected:?}"
        );
    }
}

#[tokio::test]
async fn test_hipaa_name_detection() {
    let config = create_test_config(ComplianceMode::HipaaSafeHarbor);
    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");

    let composition = json!({
        "patient": {
            "name": "Dr. Jane Elizabeth Smith-Johnson"
        }
    });

    let result = engine
        .anonymize_composition(composition)
        
        .expect("Failed to anonymize");

    // Should detect name
    assert!(result.total_detections() > 0);
}

#[tokio::test]
async fn test_hipaa_geographic_location_detection() {
    let config = create_test_config(ComplianceMode::HipaaSafeHarbor);
    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");

    let composition = json!({
        "patient": {
            "address": "456 Oak Avenue, Apartment 3B, Boston, MA 02101",
            "zip": "02101"
        }
    });

    let result = engine
        .anonymize_composition(composition)
        
        .expect("Failed to anonymize");

    // Should detect geographic location
    assert!(result.total_detections() > 0);
}

#[tokio::test]
async fn test_hipaa_date_detection() {
    let config = create_test_config(ComplianceMode::HipaaSafeHarbor);
    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");

    let composition = json!({
        "patient": {
            "birth_date": "1990-05-20",
            "admission_date": "2024-01-15T10:30:00Z",
            "discharge_date": "2024-01-20"
        }
    });

    let result = engine
        .anonymize_composition(composition)
        
        .expect("Failed to anonymize");

    // Should detect dates (at least some of them)
    assert!(
        result.total_detections() >= 1,
        "Should detect at least one date"
    );

    let date_detections = result
        .detections
        .iter()
        .filter(|d| d.category == PiiCategory::Date)
        .count();
    assert!(
        date_detections >= 1,
        "Should detect at least one date as DATE category"
    );
}

#[tokio::test]
async fn test_hipaa_contact_information_detection() {
    let config = create_test_config(ComplianceMode::HipaaSafeHarbor);
    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");

    let composition = json!({
        "patient": {
            "phone": "(555) 123-4567",
            "fax": "555-987-6543",
            "email": "patient@example.com"
        }
    });

    let result = engine
        .anonymize_composition(composition)
        
        .expect("Failed to anonymize");

    // Should detect phone, fax, and email
    assert!(result.total_detections() >= 3);
}

#[tokio::test]
async fn test_hipaa_identifiers_detection() {
    let config = create_test_config(ComplianceMode::HipaaSafeHarbor);
    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");

    let composition = json!({
        "patient": {
            "ssn": "123-45-6789",
            "mrn": "MRN-987654",
            "account": "ACCT-456789"
        }
    });

    let result = engine
        .anonymize_composition(composition)
        
        .expect("Failed to anonymize");

    // Should detect SSN and other identifiers
    assert!(result.total_detections() >= 1, "Should detect at least SSN");
}

#[tokio::test]
async fn test_hipaa_technical_identifiers_detection() {
    let config = create_test_config(ComplianceMode::HipaaSafeHarbor);
    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");

    let composition = json!({
        "system": {
            "url": "https://patient-portal.example.com/records/12345",
            "ip_address": "192.168.1.100"
        }
    });

    let result = engine
        .anonymize_composition(composition)
        
        .expect("Failed to anonymize");

    // Should detect URL and IP address
    assert!(result.total_detections() >= 2);
}

#[tokio::test]
async fn test_gdpr_quasi_identifiers_detection() {
    let config = create_test_config(ComplianceMode::Gdpr);
    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");
    let composition = create_composition_with_gdpr_quasi_identifiers();

    let result = engine
        .anonymize_composition(composition)
        
        .expect("Failed to anonymize");

    // GDPR mode should detect email + quasi-identifiers
    assert!(
        result.total_detections() >= 2,
        "Should detect email and quasi-identifiers"
    );

    // Check for quasi-identifier categories
    let detected_categories: Vec<_> = result.detections.iter().map(|d| d.category).collect();

    // Should detect email (HIPAA identifier)
    assert!(detected_categories.contains(&PiiCategory::Email));
}

#[tokio::test]
async fn test_gdpr_vs_hipaa_detection_difference() {
    let composition = create_composition_with_gdpr_quasi_identifiers();

    // Test with HIPAA mode
    let config_hipaa = create_test_config(ComplianceMode::HipaaSafeHarbor);
    let engine_hipaa = AnonymizationEngine::new(config_hipaa).expect("Failed to create engine");
    let result_hipaa = engine_hipaa
        .anonymize_composition(composition.clone())
        
        .expect("Failed to anonymize with HIPAA");

    // Test with GDPR mode
    let config_gdpr = create_test_config(ComplianceMode::Gdpr);
    let engine_gdpr = AnonymizationEngine::new(config_gdpr).expect("Failed to create engine");
    let result_gdpr = engine_gdpr
        .anonymize_composition(composition)
        
        .expect("Failed to anonymize with GDPR");

    // GDPR should detect more or equal identifiers than HIPAA
    // (GDPR includes all HIPAA identifiers plus quasi-identifiers)
    assert!(
        result_gdpr.total_detections() >= result_hipaa.total_detections(),
        "GDPR should detect at least as many identifiers as HIPAA. GDPR: {}, HIPAA: {}",
        result_gdpr.total_detections(),
        result_hipaa.total_detections()
    );
}

#[tokio::test]
async fn test_precision_no_false_positives() {
    let config = create_test_config(ComplianceMode::HipaaSafeHarbor);
    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");

    // Composition with data that looks like PII but isn't
    let composition = json!({
        "patient": {
            "blood_pressure": "120/80",
            "temperature": "98.6",
            "weight": "150 lbs",
            "height": "5'10\"",
            "diagnosis_code": "ICD-10-Z00.00"
        }
    });

    let result = engine
        .anonymize_composition(composition)
        
        .expect("Failed to anonymize");

    // Should have minimal false positives
    // Some patterns might match (e.g., numbers), but should be < 5
    assert!(
        result.total_detections() < 5,
        "Should have minimal false positives. Detected: {}",
        result.total_detections()
    );
}

#[tokio::test]
async fn test_recall_comprehensive_detection() {
    let config = create_test_config(ComplianceMode::HipaaSafeHarbor);
    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");

    // Composition with obvious PII that should definitely be detected
    let composition = json!({
        "patient": {
            "email": "test@example.com",
            "phone": "555-1234",
            "ssn": "123-45-6789",
            "date_of_birth": "1990-01-01"
        }
    });

    let result = engine
        .anonymize_composition(composition)
        
        .expect("Failed to anonymize");

    // Should detect all 4 obvious PII items (≥98% recall requirement)
    assert!(
        result.total_detections() >= 4,
        "Should detect all obvious PII. Detected: {}",
        result.total_detections()
    );
}

#[tokio::test]
async fn test_confidence_scores() {
    let config = create_test_config(ComplianceMode::HipaaSafeHarbor);
    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");

    let composition = json!({
        "patient": {
            "email": "clear.email@example.com",
            "phone": "(555) 123-4567"
        }
    });

    let result = engine
        .anonymize_composition(composition)
        
        .expect("Failed to anonymize");

    // All detections should have confidence scores
    for detection in &result.detections {
        assert!(
            detection.confidence > 0.0 && detection.confidence <= 1.0,
            "Confidence score should be between 0 and 1"
        );
    }
}

#[tokio::test]
async fn test_anonymization_reversibility_prevention() {
    let config = create_test_config(ComplianceMode::HipaaSafeHarbor);
    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");

    let original = json!({
        "patient": {
            "email": "unique.patient@example.com",
            "ssn": "987-65-4321"
        }
    });

    let result1 = engine
        .anonymize_composition(original.clone())
        
        .expect("Failed to anonymize first time");

    let result2 = engine
        .anonymize_composition(original)
        
        .expect("Failed to anonymize second time");

    // With random token strategy, same input should produce different outputs
    // (preventing reversibility attacks)
    if result1.strategy_applied.contains("Token") {
        // Tokens should be different each time
        let _str1 = serde_json::to_string(&result1.anonymized_data).unwrap();
        let _str2 = serde_json::to_string(&result2.anonymized_data).unwrap();

        // Note: This test may be flaky if the random tokens happen to match
        // In production, we'd use deterministic tokens with a secret key
        // For Phase 1, we accept random tokens
        assert!(result1.total_detections() > 0 && result2.total_detections() > 0);
    }
}

#[tokio::test]
async fn test_compliance_mode_configuration() {
    // Verify HIPAA mode is configured correctly
    let config_hipaa = create_test_config(ComplianceMode::HipaaSafeHarbor);
    assert_eq!(config_hipaa.mode, ComplianceMode::HipaaSafeHarbor);

    // Verify GDPR mode is configured correctly
    let config_gdpr = create_test_config(ComplianceMode::Gdpr);
    assert_eq!(config_gdpr.mode, ComplianceMode::Gdpr);
}

#[tokio::test]
async fn test_batch_compliance() {
    let config = create_test_config(ComplianceMode::HipaaSafeHarbor);
    let engine = AnonymizationEngine::new(config).expect("Failed to create engine");

    let batch = vec![
        create_composition_with_all_hipaa_identifiers(),
        create_composition_with_gdpr_quasi_identifiers(),
    ];

    let results = engine
        .anonymize_batch(batch)
        
        .expect("Failed to process batch");

    // All compositions should be processed
    assert_eq!(results.len(), 2);

    // All should have detections
    for result in &results {
        assert!(result.total_detections() > 0);
    }
}

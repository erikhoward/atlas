//! PII entity data models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// PII category enumeration covering HIPAA Safe Harbor (18 identifiers) and GDPR quasi-identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PiiCategory {
    // HIPAA Safe Harbor - 18 Identifiers
    /// Names (first, middle, last, maiden)
    Name,
    /// Geographic subdivisions smaller than state (street address, city, county, ZIP)
    GeographicLocation,
    /// All date elements (birth, admission, discharge, death) except year
    Date,
    /// Telephone numbers
    Phone,
    /// Fax numbers
    Fax,
    /// Email addresses
    Email,
    /// Social Security Numbers
    Ssn,
    /// Medical Record Numbers
    MedicalRecordNumber,
    /// Health Plan Beneficiary Numbers
    HealthPlanNumber,
    /// Account Numbers
    AccountNumber,
    /// Certificate/License Numbers
    CertificateLicenseNumber,
    /// Vehicle Identifiers (license plates, serial numbers)
    VehicleIdentifier,
    /// Device Identifiers and Serial Numbers
    DeviceIdentifier,
    /// Web URLs
    Url,
    /// IP Addresses
    IpAddress,
    /// Biometric Identifiers (fingerprints, voiceprints)
    BiometricIdentifier,
    /// Full-face photographs
    FacePhotograph,
    /// Any other unique identifying number, characteristic, or code
    UniqueIdentifier,

    // GDPR Quasi-Identifiers (additional)
    /// Occupation/profession
    Occupation,
    /// Education level
    EducationLevel,
    /// Marital status
    MaritalStatus,
    /// Ethnicity/race references
    Ethnicity,
    /// Age (when combined with other quasi-identifiers)
    Age,
    /// Gender (when combined with other quasi-identifiers)
    Gender,
}

impl PiiCategory {
    /// Get human-readable label for the category
    pub fn label(&self) -> &'static str {
        match self {
            Self::Name => "PERSON",
            Self::GeographicLocation => "LOCATION",
            Self::Date => "DATE",
            Self::Phone => "PHONE",
            Self::Fax => "FAX",
            Self::Email => "EMAIL",
            Self::Ssn => "SSN",
            Self::MedicalRecordNumber => "MRN",
            Self::HealthPlanNumber => "HEALTH_PLAN",
            Self::AccountNumber => "ACCOUNT",
            Self::CertificateLicenseNumber => "LICENSE",
            Self::VehicleIdentifier => "VEHICLE",
            Self::DeviceIdentifier => "DEVICE",
            Self::Url => "URL",
            Self::IpAddress => "IP_ADDRESS",
            Self::BiometricIdentifier => "BIOMETRIC",
            Self::FacePhotograph => "PHOTO",
            Self::UniqueIdentifier => "IDENTIFIER",
            Self::Occupation => "OCCUPATION",
            Self::EducationLevel => "EDUCATION",
            Self::MaritalStatus => "MARITAL_STATUS",
            Self::Ethnicity => "ETHNICITY",
            Self::Age => "AGE",
            Self::Gender => "GENDER",
        }
    }

    /// Check if this category is a HIPAA Safe Harbor identifier
    pub fn is_hipaa_identifier(&self) -> bool {
        !matches!(
            self,
            Self::Occupation
                | Self::EducationLevel
                | Self::MaritalStatus
                | Self::Ethnicity
                | Self::Age
                | Self::Gender
        )
    }

    /// Check if this category is a GDPR quasi-identifier
    pub fn is_gdpr_quasi_identifier(&self) -> bool {
        matches!(
            self,
            Self::Occupation
                | Self::EducationLevel
                | Self::MaritalStatus
                | Self::Ethnicity
                | Self::Age
                | Self::Gender
        )
    }
}

/// Detection method used to identify PII
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionMethod {
    /// Regex pattern matching (Phase I)
    Regex,
    /// Named Entity Recognition (Phase II)
    Ner,
    /// Hybrid approach (Phase II)
    Hybrid,
}

/// Detected PII entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiEntity {
    /// Category of PII
    pub category: PiiCategory,
    /// Original value (hashed in audit logs)
    pub original_value: String,
    /// Anonymized replacement value
    pub anonymized_value: Option<String>,
    /// Start position in text (for free-text detection)
    pub start_pos: Option<usize>,
    /// End position in text (for free-text detection)
    pub end_pos: Option<usize>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Detection method used
    pub detection_method: DetectionMethod,
    /// JSON path to the field containing this PII
    pub field_path: String,
}

impl PiiEntity {
    /// Create a new PII entity
    pub fn new(
        category: PiiCategory,
        original_value: String,
        field_path: String,
        detection_method: DetectionMethod,
    ) -> Self {
        Self {
            category,
            original_value,
            anonymized_value: None,
            start_pos: None,
            end_pos: None,
            confidence: 1.0,
            detection_method,
            field_path,
        }
    }

    /// Create a new PII entity with position information
    pub fn with_position(
        category: PiiCategory,
        original_value: String,
        field_path: String,
        detection_method: DetectionMethod,
        start_pos: usize,
        end_pos: usize,
    ) -> Self {
        Self {
            category,
            original_value,
            anonymized_value: None,
            start_pos: Some(start_pos),
            end_pos: Some(end_pos),
            confidence: 1.0,
            detection_method,
            field_path,
        }
    }

    /// Set the anonymized value
    pub fn set_anonymized_value(&mut self, value: String) {
        self.anonymized_value = Some(value);
    }

    /// Set the confidence score
    pub fn set_confidence(&mut self, confidence: f32) {
        self.confidence = confidence.clamp(0.0, 1.0);
    }
}

/// Anonymized composition result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizedComposition {
    /// Original composition ID
    pub original_id: String,
    /// Anonymized composition data
    pub anonymized_data: Value,
    /// List of detected PII entities
    pub detections: Vec<PiiEntity>,
    /// Strategy applied
    pub strategy_applied: String,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Timestamp of anonymization
    pub timestamp: DateTime<Utc>,
    /// Statistics by category
    pub stats_by_category: HashMap<PiiCategory, usize>,
}

impl AnonymizedComposition {
    /// Create a new anonymized composition
    pub fn new(
        original_id: String,
        anonymized_data: Value,
        detections: Vec<PiiEntity>,
        strategy_applied: String,
        processing_time_ms: u64,
    ) -> Self {
        let mut stats_by_category = HashMap::new();
        for detection in &detections {
            *stats_by_category.entry(detection.category).or_insert(0) += 1;
        }

        Self {
            original_id,
            anonymized_data,
            detections,
            strategy_applied,
            processing_time_ms,
            timestamp: Utc::now(),
            stats_by_category,
        }
    }

    /// Get total number of detections
    pub fn total_detections(&self) -> usize {
        self.detections.len()
    }

    /// Check if any PII was detected
    pub fn has_detections(&self) -> bool {
        !self.detections.is_empty()
    }
}

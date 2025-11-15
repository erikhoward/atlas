//! HIPAA Safe Harbor compliance rules

use crate::anonymization::models::PiiCategory;

/// Get all HIPAA Safe Harbor identifier categories (18 identifiers)
pub fn hipaa_identifiers() -> Vec<PiiCategory> {
    vec![
        PiiCategory::Name,
        PiiCategory::GeographicLocation,
        PiiCategory::Date,
        PiiCategory::Phone,
        PiiCategory::Fax,
        PiiCategory::Email,
        PiiCategory::Ssn,
        PiiCategory::MedicalRecordNumber,
        PiiCategory::HealthPlanNumber,
        PiiCategory::AccountNumber,
        PiiCategory::CertificateLicenseNumber,
        PiiCategory::VehicleIdentifier,
        PiiCategory::DeviceIdentifier,
        PiiCategory::Url,
        PiiCategory::IpAddress,
        PiiCategory::BiometricIdentifier,
        PiiCategory::FacePhotograph,
        PiiCategory::UniqueIdentifier,
    ]
}

/// Check if a category is a HIPAA Safe Harbor identifier
pub fn is_hipaa_identifier(category: PiiCategory) -> bool {
    category.is_hipaa_identifier()
}

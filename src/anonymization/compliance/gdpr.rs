//! GDPR compliance rules

use crate::anonymization::models::PiiCategory;

/// Get all GDPR identifier categories (HIPAA + quasi-identifiers)
pub fn gdpr_identifiers() -> Vec<PiiCategory> {
    let mut identifiers = super::hipaa::hipaa_identifiers();
    
    // Add GDPR quasi-identifiers
    identifiers.extend(vec![
        PiiCategory::Occupation,
        PiiCategory::EducationLevel,
        PiiCategory::MaritalStatus,
        PiiCategory::Ethnicity,
        PiiCategory::Age,
        PiiCategory::Gender,
    ]);
    
    identifiers
}

/// Check if a category is a GDPR quasi-identifier
pub fn is_gdpr_quasi_identifier(category: PiiCategory) -> bool {
    category.is_gdpr_quasi_identifier()
}


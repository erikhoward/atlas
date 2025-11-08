//! Verification logic for post-export validation
//!
//! This module implements the verification logic that validates exported
//! compositions by recalculating checksums and comparing them with stored values.

use crate::adapters::cosmosdb::CosmosDbClient;
use crate::core::export::ExportSummary;
use crate::core::verification::calculate_checksum;
use crate::core::verification::report::{VerificationFailure, VerificationReport};
use crate::domain::ids::{CompositionUid, EhrId, TemplateId};
use crate::domain::Result;
use std::sync::Arc;
use std::time::Instant;

/// Verifier for post-export validation
pub struct Verifier {
    cosmos_client: Arc<CosmosDbClient>,
}

impl Verifier {
    /// Create a new verifier
    pub fn new(cosmos_client: Arc<CosmosDbClient>) -> Self {
        Self { cosmos_client }
    }

    /// Verify exported compositions
    ///
    /// # Arguments
    ///
    /// * `summary` - The export summary containing information about exported compositions
    ///
    /// # Returns
    ///
    /// Returns a verification report with pass/fail counts and details.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use atlas::core::verification::verify::Verifier;
    /// use atlas::core::export::ExportSummary;
    /// use atlas::adapters::cosmosdb::CosmosDbClient;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// # let cosmos_client = Arc::new(CosmosDbClient::new(Default::default()).await?);
    /// let verifier = Verifier::new(cosmos_client);
    /// let summary = ExportSummary::new();
    /// let report = verifier.verify_export(&summary).await?;
    /// println!("{}", report.format_summary());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn verify_export(&self, summary: &ExportSummary) -> Result<VerificationReport> {
        let start = Instant::now();
        let mut report = VerificationReport::new();

        let total_to_verify = summary.exported_compositions.len();

        tracing::info!(
            total_compositions = total_to_verify,
            "Starting post-export verification"
        );

        // If no compositions were exported with checksums, skip verification
        if total_to_verify == 0 {
            tracing::info!("No compositions to verify (checksums not enabled or no exports)");
            let duration = start.elapsed();
            report.set_duration(duration.as_millis() as u64);
            return Ok(report);
        }

        // Verify each exported composition
        for exported_comp in &summary.exported_compositions {
            match self
                .verify_composition(
                    &exported_comp.composition_uid,
                    &exported_comp.ehr_id,
                    &exported_comp.template_id,
                    &exported_comp.checksum,
                )
                .await
            {
                Ok(()) => {
                    report.record_pass();
                }
                Err(failure) => {
                    report.record_failure(failure);
                }
            }
        }

        let duration = start.elapsed();
        report.set_duration(duration.as_millis() as u64);

        tracing::info!(
            total_verified = report.total_verified,
            passed = report.passed,
            failed = report.failed,
            skipped = report.skipped,
            duration_ms = report.duration_ms,
            success_rate = format!("{:.2}%", report.success_rate()),
            "Verification completed"
        );

        Ok(report)
    }

    /// Verify a single composition
    ///
    /// # Arguments
    ///
    /// * `composition_uid` - The composition UID
    /// * `ehr_id` - The EHR ID (partition key)
    /// * `template_id` - The template ID
    /// * `expected_checksum` - The expected checksum from export
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if verification passes, or a VerificationFailure if it fails.
    async fn verify_composition(
        &self,
        composition_uid: &CompositionUid,
        ehr_id: &EhrId,
        template_id: &TemplateId,
        expected_checksum: &str,
    ) -> std::result::Result<(), VerificationFailure> {
        tracing::debug!(
            composition_uid = %composition_uid.as_str(),
            ehr_id = %ehr_id.as_str(),
            template_id = %template_id.as_str(),
            "Verifying composition"
        );

        // Fetch the composition document from Cosmos DB
        let document = match self
            .cosmos_client
            .fetch_composition(template_id, ehr_id, composition_uid)
            .await
        {
            Ok(doc) => doc,
            Err(e) => {
                return Err(VerificationFailure {
                    composition_uid: composition_uid.clone(),
                    ehr_id: ehr_id.clone(),
                    template_id: template_id.clone(),
                    expected_checksum: expected_checksum.to_string(),
                    actual_checksum: "N/A".to_string(),
                    reason: format!("Failed to fetch composition from Cosmos DB: {e}"),
                });
            }
        };

        // Extract the content field from the document
        // The document structure depends on the export format (preserve or flatten)
        let content = if let Some(content_field) = document.get("content") {
            // Preserve format - content is in a dedicated field
            content_field.clone()
        } else {
            // Flatten format - the entire document is the content (minus metadata fields)
            // We need to exclude atlas_metadata and system fields
            let mut content_map = document.as_object().cloned().unwrap_or_default();
            content_map.remove("id");
            content_map.remove("ehr_id");
            content_map.remove("composition_uid");
            content_map.remove("template_id");
            content_map.remove("time_committed");
            content_map.remove("atlas_metadata");
            serde_json::Value::Object(content_map)
        };

        // Recalculate the checksum
        tracing::debug!(
            composition_uid = %composition_uid.as_str(),
            content_sample = ?serde_json::to_string(&content).unwrap_or_default().chars().take(200).collect::<String>(),
            "Content extracted for verification"
        );

        let actual_checksum = match calculate_checksum(&content) {
            Ok(checksum) => {
                tracing::debug!(
                    composition_uid = %composition_uid.as_str(),
                    checksum = %checksum,
                    expected = %expected_checksum,
                    "Calculated checksum during verification"
                );
                checksum
            }
            Err(e) => {
                return Err(VerificationFailure {
                    composition_uid: composition_uid.clone(),
                    ehr_id: ehr_id.clone(),
                    template_id: template_id.clone(),
                    expected_checksum: expected_checksum.to_string(),
                    actual_checksum: "N/A".to_string(),
                    reason: format!("Failed to calculate checksum: {e}"),
                });
            }
        };

        // Compare checksums
        if actual_checksum == expected_checksum {
            tracing::debug!(
                composition_uid = %composition_uid.as_str(),
                "Composition verification passed"
            );
            Ok(())
        } else {
            tracing::warn!(
                composition_uid = %composition_uid.as_str(),
                expected = %expected_checksum,
                actual = %actual_checksum,
                "Composition verification failed - checksum mismatch"
            );
            Err(VerificationFailure {
                composition_uid: composition_uid.clone(),
                ehr_id: ehr_id.clone(),
                template_id: template_id.clone(),
                expected_checksum: expected_checksum.to_string(),
                actual_checksum,
                reason: "Checksum mismatch".to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_creation() {
        // This is a placeholder test since we can't easily create a real CosmosDbClient in tests
        // In a real implementation, we would use mocks or integration tests
        // Just verify the module compiles
    }

    #[test]
    fn test_verification_report_structure() {
        let report = VerificationReport::new();
        assert_eq!(report.total_verified, 0);
        assert_eq!(report.passed, 0);
        assert_eq!(report.failed, 0);
        assert!(report.is_success());
    }
}

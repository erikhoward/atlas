//! Verification report structures
//!
//! This module defines the structures for reporting verification results.

use crate::domain::ids::{CompositionUid, EhrId, TemplateId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Verification report containing results of post-export validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    /// When the verification was performed
    pub verified_at: DateTime<Utc>,

    /// Total number of compositions verified
    pub total_verified: usize,

    /// Number of compositions that passed verification
    pub passed: usize,

    /// Number of compositions that failed verification
    pub failed: usize,

    /// Number of compositions that were skipped (e.g., no checksum)
    pub skipped: usize,

    /// List of failed verifications with details
    pub failures: Vec<VerificationFailure>,

    /// Duration of verification in milliseconds
    pub duration_ms: u64,
}

/// Details of a failed verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationFailure {
    /// Composition UID
    pub composition_uid: CompositionUid,

    /// EHR ID
    pub ehr_id: EhrId,

    /// Template ID
    pub template_id: TemplateId,

    /// Expected checksum (from metadata)
    pub expected_checksum: String,

    /// Actual checksum (recalculated)
    pub actual_checksum: String,

    /// Reason for failure
    pub reason: String,
}

impl VerificationReport {
    /// Create a new verification report
    pub fn new() -> Self {
        Self {
            verified_at: Utc::now(),
            total_verified: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            failures: Vec::new(),
            duration_ms: 0,
        }
    }

    /// Record a successful verification
    pub fn record_pass(&mut self) {
        self.total_verified += 1;
        self.passed += 1;
    }

    /// Record a failed verification
    pub fn record_failure(&mut self, failure: VerificationFailure) {
        self.total_verified += 1;
        self.failed += 1;
        self.failures.push(failure);
    }

    /// Record a skipped verification
    pub fn record_skip(&mut self) {
        self.total_verified += 1;
        self.skipped += 1;
    }

    /// Set the duration of verification
    pub fn set_duration(&mut self, duration_ms: u64) {
        self.duration_ms = duration_ms;
    }

    /// Check if all verifications passed
    pub fn is_success(&self) -> bool {
        self.failed == 0
    }

    /// Get the success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_verified == 0 {
            return 100.0;
        }
        (self.passed as f64 / self.total_verified as f64) * 100.0
    }

    /// Format the report as a human-readable string
    pub fn format_summary(&self) -> String {
        let mut summary = String::new();
        summary.push_str("📊 Verification Report\n");
        summary.push_str(&format!("  Verified at: {}\n", self.verified_at));
        summary.push_str(&format!("  Duration: {} ms\n", self.duration_ms));
        summary.push_str(&format!("  Total verified: {}\n", self.total_verified));
        summary.push_str(&format!("  ✅ Passed: {}\n", self.passed));
        summary.push_str(&format!("  ❌ Failed: {}\n", self.failed));
        summary.push_str(&format!("  ⏭️  Skipped: {}\n", self.skipped));
        summary.push_str(&format!("  Success rate: {:.2}%\n", self.success_rate()));

        if !self.failures.is_empty() {
            summary.push_str("\n❌ Failures:\n");
            for (i, failure) in self.failures.iter().enumerate() {
                summary.push_str(&format!(
                    "  {}. Composition: {}\n",
                    i + 1,
                    failure.composition_uid
                ));
                summary.push_str(&format!("     EHR: {}\n", failure.ehr_id));
                summary.push_str(&format!("     Template: {}\n", failure.template_id));
                summary.push_str(&format!("     Reason: {}\n", failure.reason));
            }
        }

        summary
    }
}

impl Default for VerificationReport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_verification_report_new() {
        let report = VerificationReport::new();
        assert_eq!(report.total_verified, 0);
        assert_eq!(report.passed, 0);
        assert_eq!(report.failed, 0);
        assert_eq!(report.skipped, 0);
        assert!(report.failures.is_empty());
        assert!(report.is_success());
    }

    #[test]
    fn test_record_pass() {
        let mut report = VerificationReport::new();
        report.record_pass();
        report.record_pass();

        assert_eq!(report.total_verified, 2);
        assert_eq!(report.passed, 2);
        assert_eq!(report.failed, 0);
        assert!(report.is_success());
    }

    #[test]
    fn test_record_failure() {
        let mut report = VerificationReport::new();
        let failure = VerificationFailure {
            composition_uid: CompositionUid::from_str("84d7c3f5::local.ehrbase.org::1").unwrap(),
            ehr_id: EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap(),
            template_id: TemplateId::from_str("vital_signs.v1").unwrap(),
            expected_checksum: "abc123".to_string(),
            actual_checksum: "def456".to_string(),
            reason: "Checksum mismatch".to_string(),
        };

        report.record_failure(failure);

        assert_eq!(report.total_verified, 1);
        assert_eq!(report.passed, 0);
        assert_eq!(report.failed, 1);
        assert_eq!(report.failures.len(), 1);
        assert!(!report.is_success());
    }

    #[test]
    fn test_record_skip() {
        let mut report = VerificationReport::new();
        report.record_skip();

        assert_eq!(report.total_verified, 1);
        assert_eq!(report.skipped, 1);
        assert!(report.is_success());
    }

    #[test]
    fn test_success_rate() {
        let mut report = VerificationReport::new();
        report.record_pass();
        report.record_pass();
        report.record_pass();
        report.record_skip();

        assert_eq!(report.success_rate(), 75.0); // 3 passed out of 4 total
    }

    #[test]
    fn test_success_rate_empty() {
        let report = VerificationReport::new();
        assert_eq!(report.success_rate(), 100.0);
    }

    #[test]
    fn test_format_summary() {
        let mut report = VerificationReport::new();
        report.record_pass();
        report.record_pass();
        report.set_duration(1500);

        let summary = report.format_summary();
        assert!(summary.contains("Total verified: 2"));
        assert!(summary.contains("Passed: 2"));
        assert!(summary.contains("Failed: 0"));
        assert!(summary.contains("Duration: 1500 ms"));
    }
}

//! Watermark model for tracking export state
//!
//! This module defines the watermark structure used to track the state of
//! incremental exports per {template_id, ehr_id} combination.

use crate::domain::ids::{CompositionUid, EhrId, TemplateId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Export status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportStatus {
    /// Export is in progress
    InProgress,
    /// Export completed successfully
    Completed,
    /// Export failed with an error
    Failed,
    /// Export was never started
    NotStarted,
}

impl Default for ExportStatus {
    fn default() -> Self {
        Self::NotStarted
    }
}

/// Watermark for tracking export state per {template_id, ehr_id}
///
/// This structure is stored in the Cosmos DB control container to track
/// the progress of incremental exports. It enables resuming from the last
/// successful checkpoint and detecting which compositions need to be exported.
///
/// # Examples
///
/// ```
/// use atlas::core::state::watermark::{Watermark, WatermarkBuilder, ExportStatus};
/// use atlas::domain::ids::{TemplateId, EhrId};
/// use std::str::FromStr;
///
/// let template_id = TemplateId::from_str("vital_signs.v1").unwrap();
/// let ehr_id = EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap();
///
/// let watermark = WatermarkBuilder::new(template_id, ehr_id)
///     .compositions_exported_count(100)
///     .build();
///
/// assert_eq!(watermark.compositions_exported_count, 100);
/// assert_eq!(watermark.last_export_status, ExportStatus::NotStarted);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watermark {
    /// Unique identifier for this watermark document
    /// Format: "{template_id}_{ehr_id}"
    pub id: String,

    /// Template ID this watermark tracks
    pub template_id: TemplateId,

    /// EHR ID this watermark tracks
    pub ehr_id: EhrId,

    /// Timestamp of the last successfully exported composition
    pub last_exported_timestamp: DateTime<Utc>,

    /// UID of the last successfully exported composition
    pub last_exported_composition_uid: Option<CompositionUid>,

    /// Total count of compositions exported for this {template_id, ehr_id}
    pub compositions_exported_count: u64,

    /// Timestamp when the last export batch started
    pub last_export_started_at: DateTime<Utc>,

    /// Timestamp when the last export batch completed (None if still in progress)
    pub last_export_completed_at: Option<DateTime<Utc>>,

    /// Status of the last export operation
    pub last_export_status: ExportStatus,
}

impl Watermark {
    /// Generate the document ID for a watermark
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID
    /// * `ehr_id` - EHR ID
    ///
    /// # Returns
    ///
    /// A string in the format "{template_id}_{ehr_id}"
    pub fn generate_id(template_id: &TemplateId, ehr_id: &EhrId) -> String {
        format!("{}_{}", template_id.as_str(), ehr_id.as_str())
    }

    /// Check if this watermark indicates an export is currently in progress
    pub fn is_in_progress(&self) -> bool {
        self.last_export_status == ExportStatus::InProgress
    }

    /// Check if the last export completed successfully
    pub fn is_completed(&self) -> bool {
        self.last_export_status == ExportStatus::Completed
    }

    /// Check if the last export failed
    pub fn is_failed(&self) -> bool {
        self.last_export_status == ExportStatus::Failed
    }

    /// Get the duration of the last export if it completed
    pub fn last_export_duration(&self) -> Option<chrono::Duration> {
        self.last_export_completed_at
            .map(|completed| completed - self.last_export_started_at)
    }

    /// Mark the export as started
    pub fn mark_started(&mut self) {
        self.last_export_started_at = Utc::now();
        self.last_export_status = ExportStatus::InProgress;
        self.last_export_completed_at = None;
    }

    /// Mark the export as completed
    pub fn mark_completed(&mut self) {
        self.last_export_completed_at = Some(Utc::now());
        self.last_export_status = ExportStatus::Completed;
    }

    /// Mark the export as failed
    pub fn mark_failed(&mut self) {
        self.last_export_completed_at = Some(Utc::now());
        self.last_export_status = ExportStatus::Failed;
    }

    /// Update the watermark after successfully exporting a composition
    ///
    /// # Arguments
    ///
    /// * `composition_uid` - UID of the exported composition
    /// * `timestamp` - Timestamp of the composition
    pub fn update_after_export(
        &mut self,
        composition_uid: CompositionUid,
        timestamp: DateTime<Utc>,
    ) {
        self.last_exported_composition_uid = Some(composition_uid);
        self.last_exported_timestamp = timestamp;
        self.compositions_exported_count += 1;
    }
}

/// Builder for creating Watermark instances
pub struct WatermarkBuilder {
    template_id: TemplateId,
    ehr_id: EhrId,
    last_exported_timestamp: Option<DateTime<Utc>>,
    last_exported_composition_uid: Option<CompositionUid>,
    compositions_exported_count: u64,
    last_export_started_at: Option<DateTime<Utc>>,
    last_export_completed_at: Option<DateTime<Utc>>,
    last_export_status: ExportStatus,
}

impl WatermarkBuilder {
    /// Create a new WatermarkBuilder
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID to track
    /// * `ehr_id` - EHR ID to track
    pub fn new(template_id: TemplateId, ehr_id: EhrId) -> Self {
        Self {
            template_id,
            ehr_id,
            last_exported_timestamp: None,
            last_exported_composition_uid: None,
            compositions_exported_count: 0,
            last_export_started_at: None,
            last_export_completed_at: None,
            last_export_status: ExportStatus::NotStarted,
        }
    }

    /// Set the last exported timestamp
    pub fn last_exported_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.last_exported_timestamp = Some(timestamp);
        self
    }

    /// Set the last exported composition UID
    pub fn last_exported_composition_uid(mut self, uid: CompositionUid) -> Self {
        self.last_exported_composition_uid = Some(uid);
        self
    }

    /// Set the compositions exported count
    pub fn compositions_exported_count(mut self, count: u64) -> Self {
        self.compositions_exported_count = count;
        self
    }

    /// Set the last export started timestamp
    pub fn last_export_started_at(mut self, timestamp: DateTime<Utc>) -> Self {
        self.last_export_started_at = Some(timestamp);
        self
    }

    /// Set the last export completed timestamp
    pub fn last_export_completed_at(mut self, timestamp: DateTime<Utc>) -> Self {
        self.last_export_completed_at = Some(timestamp);
        self
    }

    /// Set the last export status
    pub fn last_export_status(mut self, status: ExportStatus) -> Self {
        self.last_export_status = status;
        self
    }

    /// Build the Watermark instance
    pub fn build(self) -> Watermark {
        let id = Watermark::generate_id(&self.template_id, &self.ehr_id);
        let now = Utc::now();

        Watermark {
            id,
            template_id: self.template_id,
            ehr_id: self.ehr_id,
            last_exported_timestamp: self.last_exported_timestamp.unwrap_or(now),
            last_exported_composition_uid: self.last_exported_composition_uid,
            compositions_exported_count: self.compositions_exported_count,
            last_export_started_at: self.last_export_started_at.unwrap_or(now),
            last_export_completed_at: self.last_export_completed_at,
            last_export_status: self.last_export_status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_watermark_builder() {
        let template_id = TemplateId::from_str("vital_signs.v1").unwrap();
        let ehr_id = EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap();

        let watermark = WatermarkBuilder::new(template_id.clone(), ehr_id.clone())
            .compositions_exported_count(50)
            .build();

        assert_eq!(watermark.template_id, template_id);
        assert_eq!(watermark.ehr_id, ehr_id);
        assert_eq!(watermark.compositions_exported_count, 50);
        assert_eq!(watermark.last_export_status, ExportStatus::NotStarted);
    }

    #[test]
    fn test_generate_id() {
        let template_id = TemplateId::from_str("vital_signs.v1").unwrap();
        let ehr_id = EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap();

        let id = Watermark::generate_id(&template_id, &ehr_id);
        assert_eq!(id, "vital_signs.v1_7d44b88c-4199-4bad-97dc-d78268e01398");
    }

    #[test]
    fn test_mark_started() {
        let template_id = TemplateId::from_str("vital_signs.v1").unwrap();
        let ehr_id = EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap();

        let mut watermark = WatermarkBuilder::new(template_id, ehr_id).build();
        watermark.mark_started();

        assert!(watermark.is_in_progress());
        assert!(!watermark.is_completed());
        assert!(!watermark.is_failed());
        assert!(watermark.last_export_completed_at.is_none());
    }

    #[test]
    fn test_mark_completed() {
        let template_id = TemplateId::from_str("vital_signs.v1").unwrap();
        let ehr_id = EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap();

        let mut watermark = WatermarkBuilder::new(template_id, ehr_id).build();
        watermark.mark_started();
        watermark.mark_completed();

        assert!(!watermark.is_in_progress());
        assert!(watermark.is_completed());
        assert!(!watermark.is_failed());
        assert!(watermark.last_export_completed_at.is_some());
    }

    #[test]
    fn test_mark_failed() {
        let template_id = TemplateId::from_str("vital_signs.v1").unwrap();
        let ehr_id = EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap();

        let mut watermark = WatermarkBuilder::new(template_id, ehr_id).build();
        watermark.mark_started();
        watermark.mark_failed();

        assert!(!watermark.is_in_progress());
        assert!(!watermark.is_completed());
        assert!(watermark.is_failed());
        assert!(watermark.last_export_completed_at.is_some());
    }

    #[test]
    fn test_update_after_export() {
        let template_id = TemplateId::from_str("vital_signs.v1").unwrap();
        let ehr_id = EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap();
        let composition_uid = CompositionUid::from_str("84d7c3f5::local.ehrbase.org::1").unwrap();

        let mut watermark = WatermarkBuilder::new(template_id, ehr_id).build();
        let initial_count = watermark.compositions_exported_count;

        let timestamp = Utc::now();
        watermark.update_after_export(composition_uid.clone(), timestamp);

        assert_eq!(watermark.compositions_exported_count, initial_count + 1);
        assert_eq!(
            watermark.last_exported_composition_uid,
            Some(composition_uid)
        );
        assert_eq!(watermark.last_exported_timestamp, timestamp);
    }

    #[test]
    fn test_export_duration() {
        let template_id = TemplateId::from_str("vital_signs.v1").unwrap();
        let ehr_id = EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap();

        let mut watermark = WatermarkBuilder::new(template_id, ehr_id).build();

        // No duration before completion
        assert!(watermark.last_export_duration().is_none());

        watermark.mark_started();
        std::thread::sleep(std::time::Duration::from_millis(10));
        watermark.mark_completed();

        // Should have a duration after completion
        let duration = watermark.last_export_duration();
        assert!(duration.is_some());
        assert!(duration.unwrap().num_milliseconds() >= 0);
    }

    #[test]
    fn test_watermark_serialization() {
        let template_id = TemplateId::from_str("vital_signs.v1").unwrap();
        let ehr_id = EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap();

        let watermark = WatermarkBuilder::new(template_id, ehr_id)
            .compositions_exported_count(100)
            .last_export_status(ExportStatus::Completed)
            .build();

        // Serialize to JSON
        let json = serde_json::to_string(&watermark).unwrap();
        assert!(json.contains("vital_signs.v1"));
        assert!(json.contains("7d44b88c-4199-4bad-97dc-d78268e01398"));

        // Deserialize back
        let deserialized: Watermark = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.compositions_exported_count, 100);
        assert_eq!(deserialized.last_export_status, ExportStatus::Completed);
    }
}

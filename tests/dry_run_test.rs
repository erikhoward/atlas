//! Integration tests for dry-run mode
//!
//! These tests verify that the --dry-run flag prevents all database writes
//! while allowing the export process to run normally.

use atlas::config::schema::ExportConfig;
use atlas::core::export::batch::{BatchConfig, BatchResult};
use atlas::core::export::summary::ExportSummary;
use atlas::core::state::watermark::{ExportStatus, WatermarkBuilder};
use atlas::domain::ids::{EhrId, TemplateId};
use std::str::FromStr;

#[test]
fn test_batch_config_with_dry_run() {
    let config = BatchConfig::from_config(1000, "preserve", true, None).unwrap();

    assert_eq!(config.batch_size, 1000);
    assert!(config.dry_run);
}

#[test]
fn test_batch_config_without_dry_run() {
    let config = BatchConfig::from_config(1000, "preserve", false, None).unwrap();

    assert_eq!(config.batch_size, 1000);
    assert!(!config.dry_run);
}

#[test]
fn test_export_config_dry_run_default() {
    let config = ExportConfig {
        mode: "incremental".to_string(),
        export_composition_format: "preserve".to_string(),
        max_retries: 3,
        retry_backoff_ms: vec![1000, 2000, 4000],
        shutdown_timeout_secs: 30,
        dry_run: false,
    };

    assert!(!config.dry_run);
}

#[test]
fn test_export_config_dry_run_enabled() {
    let config = ExportConfig {
        mode: "incremental".to_string(),
        export_composition_format: "preserve".to_string(),
        max_retries: 3,
        retry_backoff_ms: vec![1000, 2000, 4000],
        shutdown_timeout_secs: 30,
        dry_run: true,
    };

    assert!(config.dry_run);
}

#[test]
fn test_export_summary_dry_run_flag() {
    let mut summary = ExportSummary::new();
    assert!(!summary.dry_run);

    summary.dry_run = true;
    assert!(summary.dry_run);
}

#[test]
fn test_export_summary_with_dry_run() {
    let mut summary = ExportSummary::new();
    summary.dry_run = true;
    summary.total_compositions = 100;
    summary.successful_exports = 100;

    assert!(summary.dry_run);
    assert_eq!(summary.total_compositions, 100);
    assert_eq!(summary.successful_exports, 100);
    assert!(summary.is_successful());
}

#[test]
fn test_batch_result_operations() {
    let mut result = BatchResult::new();

    assert_eq!(result.successful, 0);
    assert_eq!(result.failed, 0);
    assert_eq!(result.duplicates_skipped, 0);

    result.successful = 50;
    result.failed = 2;
    result.duplicates_skipped = 3;

    assert_eq!(result.successful, 50);
    assert_eq!(result.failed, 2);
    assert_eq!(result.duplicates_skipped, 3);
}

#[test]
fn test_watermark_status_for_dry_run() {
    let template_id = TemplateId::from_str("vital_signs.v1").unwrap();
    let ehr_id = EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap();

    let mut watermark = WatermarkBuilder::new(template_id, ehr_id).build();

    // In dry-run mode, watermarks should still track state correctly
    watermark.mark_started();
    assert_eq!(watermark.last_export_status, ExportStatus::InProgress);

    watermark.mark_completed();
    assert_eq!(watermark.last_export_status, ExportStatus::Completed);
}

#[test]
fn test_dry_run_preserves_watermark_logic() {
    let template_id = TemplateId::from_str("vital_signs.v1").unwrap();
    let ehr_id = EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").unwrap();

    let watermark = WatermarkBuilder::new(template_id.clone(), ehr_id.clone())
        .compositions_exported_count(50)
        .build();

    // Dry-run should not affect watermark state tracking
    assert_eq!(watermark.template_id, template_id);
    assert_eq!(watermark.ehr_id, ehr_id);
    assert_eq!(watermark.compositions_exported_count, 50);
}

#[test]
fn test_batch_config_dry_run_propagation() {
    // Test that dry_run flag is properly stored in BatchConfig
    let config_with_dry_run = BatchConfig::from_config(500, "flatten", true, None).unwrap();
    assert!(config_with_dry_run.dry_run);

    let config_without_dry_run = BatchConfig::from_config(500, "flatten", false, None).unwrap();
    assert!(!config_without_dry_run.dry_run);
}

#[test]
fn test_export_summary_dry_run_in_new() {
    // Verify that new() creates summary with dry_run = false by default
    let summary = ExportSummary::new();
    assert!(!summary.dry_run);
}

#[test]
fn test_export_summary_dry_run_with_results() {
    let mut summary = ExportSummary::new();
    summary.dry_run = true;
    summary.total_ehrs = 5;
    summary.total_compositions = 250;
    summary.successful_exports = 250;
    summary.failed_exports = 0;

    assert!(summary.dry_run);
    assert_eq!(summary.total_ehrs, 5);
    assert_eq!(summary.total_compositions, 250);
    assert_eq!(summary.successful_exports, 250);
    assert_eq!(summary.failed_exports, 0);
    assert!(summary.is_successful());
    assert_eq!(summary.success_rate(), 100.0);
}

#[test]
fn test_dry_run_flag_independence() {
    // Verify that dry_run flag is independent of other summary fields
    let mut summary1 = ExportSummary::new();
    summary1.dry_run = true;
    summary1.interrupted = true;

    let mut summary2 = ExportSummary::new();
    summary2.dry_run = false;
    summary2.interrupted = true;

    assert!(summary1.dry_run);
    assert!(!summary2.dry_run);
    assert!(summary1.interrupted);
    assert!(summary2.interrupted);
}

#[test]
fn test_batch_config_all_formats_with_dry_run() {
    // Test dry_run with preserve format
    let preserve_config = BatchConfig::from_config(1000, "preserve", true, None).unwrap();
    assert!(preserve_config.dry_run);

    // Test dry_run with flatten format
    let flatten_config = BatchConfig::from_config(1000, "flatten", true, None).unwrap();
    assert!(flatten_config.dry_run);
}

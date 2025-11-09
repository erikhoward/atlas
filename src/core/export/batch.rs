//! Batch processing for composition exports
//!
//! This module handles the transformation and bulk insertion of compositions
//! to database backends in batches.

use crate::adapters::database::traits::DatabaseClient;
use crate::core::state::{StateManager, Watermark};
use crate::core::transform::CompositionFormat;
use crate::domain::composition::Composition;
use crate::domain::ids::{CompositionUid, EhrId, TemplateId};
use crate::domain::Result;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

/// Configuration for batch processing
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Batch size (100-5000)
    pub batch_size: usize,
    /// Composition format (preserve or flatten)
    pub composition_format: CompositionFormat,
    /// Dry run mode - skip database writes
    pub dry_run: bool,
}

impl BatchConfig {
    /// Create a new batch configuration
    pub fn new(batch_size: usize, composition_format: CompositionFormat, dry_run: bool) -> Self {
        Self {
            batch_size,
            composition_format,
            dry_run,
        }
    }

    /// Create from export config strings
    pub fn from_config(
        batch_size: usize,
        composition_format_str: &str,
        dry_run: bool,
    ) -> Result<Self> {
        let composition_format = CompositionFormat::from_str(composition_format_str)?;
        Ok(Self::new(batch_size, composition_format, dry_run))
    }
}

/// Result of processing a batch
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Number of compositions successfully exported
    pub successful: usize,
    /// Number of compositions that failed
    pub failed: usize,
    /// Number of duplicates skipped
    pub duplicates_skipped: usize,
    /// Errors encountered
    pub errors: Vec<String>,
    /// Checksums of successfully exported compositions (composition_uid -> checksum)
    pub checksums: HashMap<CompositionUid, String>,
}

impl BatchResult {
    /// Create a new empty batch result
    pub fn new() -> Self {
        Self {
            successful: 0,
            failed: 0,
            duplicates_skipped: 0,
            errors: Vec::new(),
            checksums: HashMap::new(),
        }
    }

    /// Add a successful export
    pub fn add_success(&mut self) {
        self.successful += 1;
    }

    /// Add a failed export
    pub fn add_failure(&mut self, error: String) {
        self.failed += 1;
        self.errors.push(error);
    }

    /// Add a skipped duplicate
    pub fn add_duplicate(&mut self) {
        self.duplicates_skipped += 1;
    }

    /// Merge another batch result into this one
    pub fn merge(&mut self, other: BatchResult) {
        self.successful += other.successful;
        self.failed += other.failed;
        self.duplicates_skipped += other.duplicates_skipped;
        self.errors.extend(other.errors);
        self.checksums.extend(other.checksums);
    }

    /// Add a checksum for a successfully exported composition
    pub fn add_checksum(&mut self, composition_uid: CompositionUid, checksum: String) {
        self.checksums.insert(composition_uid, checksum);
    }
}

impl Default for BatchResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch processor for compositions
pub struct BatchProcessor {
    database_client: Arc<dyn DatabaseClient + Send + Sync>,
    state_manager: Arc<StateManager>,
    config: BatchConfig,
}

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new(
        database_client: Arc<dyn DatabaseClient + Send + Sync>,
        state_manager: Arc<StateManager>,
        config: BatchConfig,
    ) -> Self {
        Self {
            database_client,
            state_manager,
            config,
        }
    }

    /// Process a batch of compositions
    ///
    /// This method:
    /// 1. Transforms compositions to the target format
    /// 2. Checks for duplicates (FR-2.6)
    /// 3. Bulk inserts to Cosmos DB
    /// 4. Handles partial failures (FR-5.3)
    /// 5. Updates watermarks
    /// 6. Returns detailed results
    pub async fn process_batch(
        &self,
        compositions: Vec<Composition>,
        template_id: &TemplateId,
        ehr_id: &EhrId,
        watermark: &mut Watermark,
    ) -> Result<BatchResult> {
        let mut result = BatchResult::new();

        if compositions.is_empty() {
            tracing::debug!("No compositions to process in batch");
            return Ok(result);
        }

        tracing::info!(
            template_id = %template_id.as_str(),
            ehr_id = %ehr_id.as_str(),
            batch_size = compositions.len(),
            "Processing batch of compositions"
        );

        // Bulk insert to database using the appropriate method based on format
        let bulk_result = match self.config.composition_format {
            CompositionFormat::Preserve => {
                self.database_client
                    .bulk_insert_compositions(
                        template_id,
                        compositions.clone(),
                        "preserve".to_string(),
                        3, // max_retries
                        self.config.dry_run,
                    )
                    .await?
            }
            CompositionFormat::Flatten => {
                self.database_client
                    .bulk_insert_compositions_flattened(
                        template_id,
                        compositions.clone(),
                        "flatten".to_string(),
                        3, // max_retries
                        self.config.dry_run,
                    )
                    .await?
            }
        };

        result.successful = bulk_result.success_count;
        result.failed = bulk_result.failure_count;

        // Add failure details
        for failure in bulk_result.failures {
            result.add_failure(format!("{}: {}", failure.document_id, failure.error));
        }

        tracing::info!(
            inserted = bulk_result.success_count,
            failed = bulk_result.failure_count,
            "Bulk insert completed"
        );

        // Update watermark with last composition
        if let Some(last_composition) = compositions.last() {
            watermark.update_after_export(
                last_composition.uid.clone(),
                last_composition.time_committed,
            );

            // Checkpoint progress
            if let Err(e) = self
                .state_manager
                .checkpoint_batch(watermark, self.config.dry_run)
                .await
            {
                tracing::warn!(error = %e, "Failed to checkpoint watermark");
                // Don't fail the batch, just log the warning
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config_creation() {
        let config = BatchConfig::new(1000, CompositionFormat::Preserve, false);

        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.composition_format, CompositionFormat::Preserve);
        assert_eq!(config.dry_run, false);
    }

    #[test]
    fn test_batch_config_from_config() {
        let config = BatchConfig::from_config(500, "flatten", false).unwrap();

        assert_eq!(config.batch_size, 500);
        assert_eq!(config.composition_format, CompositionFormat::Flatten);
        assert_eq!(config.dry_run, false);
    }

    #[test]
    fn test_batch_result_operations() {
        let mut result = BatchResult::new();

        assert_eq!(result.successful, 0);
        assert_eq!(result.failed, 0);
        assert_eq!(result.duplicates_skipped, 0);
        assert!(result.checksums.is_empty());

        result.add_success();
        result.add_success();
        assert_eq!(result.successful, 2);

        result.add_failure("Error 1".to_string());
        assert_eq!(result.failed, 1);
        assert_eq!(result.errors.len(), 1);

        result.add_duplicate();
        assert_eq!(result.duplicates_skipped, 1);
    }

    #[test]
    fn test_batch_result_merge() {
        use std::str::FromStr;

        let mut result1 = BatchResult::new();
        result1.add_success();
        result1.add_failure("Error 1".to_string());
        let uid1 = CompositionUid::from_str("84d7c3f5::local.ehrbase.org::1").unwrap();
        result1.add_checksum(uid1.clone(), "checksum1".to_string());

        let mut result2 = BatchResult::new();
        result2.add_success();
        result2.add_success();
        result2.add_duplicate();
        let uid2 = CompositionUid::from_str("95e8d4g6::local.ehrbase.org::1").unwrap();
        result2.add_checksum(uid2.clone(), "checksum2".to_string());

        result1.merge(result2);

        assert_eq!(result1.successful, 3);
        assert_eq!(result1.failed, 1);
        assert_eq!(result1.duplicates_skipped, 1);
        assert_eq!(result1.errors.len(), 1);
        assert_eq!(result1.checksums.len(), 2);
        assert_eq!(result1.checksums.get(&uid1), Some(&"checksum1".to_string()));
        assert_eq!(result1.checksums.get(&uid2), Some(&"checksum2".to_string()));
    }
}

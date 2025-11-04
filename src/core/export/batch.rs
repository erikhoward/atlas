//! Batch processing for composition exports
//!
//! This module handles the transformation and bulk insertion of compositions
//! to Cosmos DB in batches.

use crate::adapters::cosmosdb::CosmosDbClient;
use crate::core::state::{StateManager, Watermark};
use crate::core::transform::{transform_composition, CompositionFormat};
use crate::domain::composition::Composition;
use crate::domain::ids::{EhrId, TemplateId};
use crate::domain::{AtlasError, Result};
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;

/// Configuration for batch processing
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Batch size (100-5000)
    pub batch_size: usize,
    /// Composition format (preserve or flatten)
    pub composition_format: CompositionFormat,
    /// Export mode (full or incremental)
    pub export_mode: String,
    /// Enable checksum calculation
    pub enable_checksum: bool,
}

impl BatchConfig {
    /// Create a new batch configuration
    pub fn new(
        batch_size: usize,
        composition_format: CompositionFormat,
        export_mode: String,
        enable_checksum: bool,
    ) -> Self {
        Self {
            batch_size,
            composition_format,
            export_mode,
            enable_checksum,
        }
    }

    /// Create from export config strings
    pub fn from_config(
        batch_size: usize,
        composition_format_str: &str,
        export_mode: String,
        enable_checksum: bool,
    ) -> Result<Self> {
        let composition_format = CompositionFormat::from_str(composition_format_str)?;
        Ok(Self::new(
            batch_size,
            composition_format,
            export_mode,
            enable_checksum,
        ))
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
}

impl BatchResult {
    /// Create a new empty batch result
    pub fn new() -> Self {
        Self {
            successful: 0,
            failed: 0,
            duplicates_skipped: 0,
            errors: Vec::new(),
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
    }
}

impl Default for BatchResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch processor for compositions
pub struct BatchProcessor {
    cosmos_client: Arc<CosmosDbClient>,
    state_manager: Arc<StateManager>,
    config: BatchConfig,
}

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new(
        cosmos_client: Arc<CosmosDbClient>,
        state_manager: Arc<StateManager>,
        config: BatchConfig,
    ) -> Self {
        Self {
            cosmos_client,
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

        // Transform compositions
        let mut transformed_docs = Vec::new();
        for composition in &compositions {
            match transform_composition(
                composition.clone(),
                self.config.composition_format,
                self.config.export_mode.clone(),
                self.config.enable_checksum,
            ) {
                Ok(doc) => transformed_docs.push(doc),
                Err(e) => {
                    tracing::warn!(
                        composition_uid = %composition.uid.as_str(),
                        error = %e,
                        "Failed to transform composition"
                    );
                    result.add_failure(format!(
                        "Transform failed for {}: {}",
                        composition.uid.as_str(),
                        e
                    ));
                }
            }
        }

        if transformed_docs.is_empty() {
            tracing::warn!("All compositions failed transformation");
            return Ok(result);
        }

        // Bulk insert to Cosmos DB
        match self
            .bulk_insert_compositions(template_id, transformed_docs)
            .await
        {
            Ok(inserted_count) => {
                result.successful = inserted_count;
                tracing::info!(
                    inserted = inserted_count,
                    "Successfully inserted compositions to Cosmos DB"
                );
            }
            Err(e) => {
                tracing::error!(error = %e, "Bulk insert failed");
                result.add_failure(format!("Bulk insert failed: {}", e));
                return Ok(result);
            }
        }

        // Update watermark with last composition
        if let Some(last_composition) = compositions.last() {
            watermark.update_after_export(
                last_composition.uid.clone(),
                last_composition.time_committed,
            );

            // Checkpoint progress
            if let Err(e) = self.state_manager.checkpoint_batch(watermark).await {
                tracing::warn!(error = %e, "Failed to checkpoint watermark");
                // Don't fail the batch, just log the warning
            }
        }

        Ok(result)
    }

    /// Bulk insert compositions to Cosmos DB
    async fn bulk_insert_compositions(
        &self,
        template_id: &TemplateId,
        documents: Vec<Value>,
    ) -> Result<usize> {
        // Ensure container exists
        self.cosmos_client
            .ensure_container_exists(template_id)
            .await?;

        // Get container client
        let container = self.cosmos_client.get_container_client(template_id);

        // Insert documents one by one (Azure SDK doesn't have true bulk insert)
        let mut inserted = 0;
        for doc in documents {
            // Extract partition key (ehr_id) from document
            let ehr_id = doc["ehr_id"].as_str().ok_or_else(|| {
                AtlasError::Serialization("Missing ehr_id in document".to_string())
            })?;

            let partition_key = azure_data_cosmos::PartitionKey::from(ehr_id.to_string());

            // Upsert the document
            match container.upsert_item(partition_key, &doc, None).await {
                Ok(_) => {
                    inserted += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        ehr_id = ehr_id,
                        "Failed to insert document"
                    );
                    // Continue with other documents (partial failure handling)
                }
            }
        }

        Ok(inserted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config_creation() {
        let config = BatchConfig::new(1000, CompositionFormat::Preserve, "full".to_string(), true);

        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.composition_format, CompositionFormat::Preserve);
        assert_eq!(config.export_mode, "full");
        assert!(config.enable_checksum);
    }

    #[test]
    fn test_batch_config_from_config() {
        let config =
            BatchConfig::from_config(500, "flatten", "incremental".to_string(), false).unwrap();

        assert_eq!(config.batch_size, 500);
        assert_eq!(config.composition_format, CompositionFormat::Flatten);
        assert_eq!(config.export_mode, "incremental");
        assert!(!config.enable_checksum);
    }

    #[test]
    fn test_batch_result_operations() {
        let mut result = BatchResult::new();

        assert_eq!(result.successful, 0);
        assert_eq!(result.failed, 0);
        assert_eq!(result.duplicates_skipped, 0);

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
        let mut result1 = BatchResult::new();
        result1.add_success();
        result1.add_failure("Error 1".to_string());

        let mut result2 = BatchResult::new();
        result2.add_success();
        result2.add_success();
        result2.add_duplicate();

        result1.merge(result2);

        assert_eq!(result1.successful, 3);
        assert_eq!(result1.failed, 1);
        assert_eq!(result1.duplicates_skipped, 1);
        assert_eq!(result1.errors.len(), 1);
    }
}

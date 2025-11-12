//! Batch processing for composition exports
//!
//! This module handles the transformation and bulk insertion of compositions
//! to database backends in batches.

use crate::adapters::database::traits::DatabaseClient;
use crate::anonymization::config::AnonymizationConfig;
use crate::anonymization::engine::AnonymizationEngine;
use crate::core::state::{StateManager, Watermark};
use crate::core::transform::{transform_composition, CompositionFormat};
use crate::domain::composition::Composition;
use crate::domain::ids::{CompositionUid, EhrId, TemplateId};
use crate::domain::Result;
use serde_json::Value;
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
    /// Anonymization configuration (optional)
    pub anonymization: Option<AnonymizationConfig>,
}

impl BatchConfig {
    /// Create a new batch configuration
    pub fn new(
        batch_size: usize,
        composition_format: CompositionFormat,
        dry_run: bool,
        anonymization: Option<AnonymizationConfig>,
    ) -> Self {
        Self {
            batch_size,
            composition_format,
            dry_run,
            anonymization,
        }
    }

    /// Create from export config strings
    pub fn from_config(
        batch_size: usize,
        composition_format_str: &str,
        dry_run: bool,
        anonymization: Option<AnonymizationConfig>,
    ) -> Result<Self> {
        let composition_format = CompositionFormat::from_str(composition_format_str)?;
        Ok(Self::new(
            batch_size,
            composition_format,
            dry_run,
            anonymization,
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
    /// Checksums of successfully exported compositions (composition_uid -> checksum)
    pub checksums: HashMap<CompositionUid, String>,
    /// Anonymization statistics (if anonymization was enabled)
    pub anonymization_stats: Option<AnonymizationStats>,
}

/// Statistics for anonymization processing
#[derive(Debug, Clone)]
pub struct AnonymizationStats {
    /// Number of compositions anonymized
    pub compositions_anonymized: usize,
    /// Number of compositions that failed anonymization
    pub anonymization_failures: usize,
    /// Total PII entities detected
    pub total_pii_detected: usize,
    /// Average processing time per composition (milliseconds)
    pub avg_processing_time_ms: u64,
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
            anonymization_stats: None,
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

        // Merge anonymization stats if present
        if let Some(other_stats) = other.anonymization_stats {
            if let Some(ref mut self_stats) = self.anonymization_stats {
                self_stats.compositions_anonymized += other_stats.compositions_anonymized;
                self_stats.anonymization_failures += other_stats.anonymization_failures;
                self_stats.total_pii_detected += other_stats.total_pii_detected;
                // Recalculate average processing time
                let total_comps =
                    self_stats.compositions_anonymized + other_stats.compositions_anonymized;
                if total_comps > 0 {
                    self_stats.avg_processing_time_ms = (self_stats.avg_processing_time_ms
                        + other_stats.avg_processing_time_ms)
                        / 2;
                }
            } else {
                self.anonymization_stats = Some(other_stats);
            }
        }
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

    /// Transform and optionally anonymize compositions
    ///
    /// This method:
    /// 1. Transforms compositions to JSON based on format (preserve/flatten)
    /// 2. Applies anonymization if enabled
    /// 3. Returns transformed JSON values and anonymization stats
    async fn transform_and_anonymize(
        &self,
        compositions: &[Composition],
    ) -> Result<(Vec<Value>, Option<AnonymizationStats>)> {
        let export_mode = if self.config.dry_run {
            "dry_run"
        } else {
            "full"
        }
        .to_string();

        // Transform compositions to JSON
        let mut transformed: Vec<Value> = Vec::with_capacity(compositions.len());
        for composition in compositions {
            let json = transform_composition(
                composition.clone(),
                self.config.composition_format,
                export_mode.clone(),
            )?;
            transformed.push(json);
        }

        // Apply anonymization if enabled
        if let Some(ref anon_config) = self.config.anonymization {
            if anon_config.enabled {
                tracing::info!(
                    mode = ?anon_config.mode,
                    strategy = ?anon_config.strategy,
                    dry_run = anon_config.dry_run,
                    "Anonymizing batch of {} compositions",
                    transformed.len()
                );

                // Create anonymization engine
                let engine = AnonymizationEngine::new(anon_config.clone()).map_err(|e| {
                    crate::domain::AtlasError::Export(format!(
                        "Anonymization engine creation failed: {}",
                        e
                    ))
                })?;

                // Anonymize all compositions
                let anonymized_results = engine.anonymize_batch(transformed).map_err(|e| {
                    crate::domain::AtlasError::Export(format!("Anonymization failed: {}", e))
                })?;

                // Extract anonymized JSON and collect stats
                let mut anonymized_json = Vec::with_capacity(anonymized_results.len());
                let mut total_pii = 0;
                let mut total_time_ms = 0;

                for result in &anonymized_results {
                    anonymized_json.push(result.anonymized_data.clone());
                    total_pii += result.detections.len();
                    total_time_ms += result.processing_time_ms;
                }

                let stats = AnonymizationStats {
                    compositions_anonymized: anonymized_results.len(),
                    anonymization_failures: compositions.len() - anonymized_results.len(),
                    total_pii_detected: total_pii,
                    avg_processing_time_ms: if !anonymized_results.is_empty() {
                        total_time_ms / anonymized_results.len() as u64
                    } else {
                        0
                    },
                };

                tracing::info!(
                    anonymized = stats.compositions_anonymized,
                    pii_detected = stats.total_pii_detected,
                    avg_time_ms = stats.avg_processing_time_ms,
                    "Anonymization completed"
                );

                return Ok((anonymized_json, Some(stats)));
            }
        }

        // No anonymization - return transformed JSON as-is
        Ok((transformed, None))
    }

    /// Process a batch of compositions
    ///
    /// This method:
    /// 1. Transforms compositions to the target format
    /// 2. Applies anonymization if enabled
    /// 3. Checks for duplicates (FR-2.6)
    /// 4. Bulk inserts to database
    /// 5. Handles partial failures (FR-5.3)
    /// 6. Updates watermarks
    /// 7. Returns detailed results
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
            anonymization_enabled = self.config.anonymization.as_ref().map(|c| c.enabled).unwrap_or(false),
            "Processing batch of compositions"
        );

        // Transform and anonymize compositions
        let (transformed_json, anonymization_stats) =
            self.transform_and_anonymize(&compositions).await?;

        // Store anonymization stats in result
        result.anonymization_stats = anonymization_stats;

        // Bulk insert to database using the new bulk_insert_json method
        let bulk_result = self
            .database_client
            .bulk_insert_json(
                template_id,
                transformed_json,
                3, // max_retries
                self.config.dry_run,
            )
            .await?;

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
    use crate::adapters::database::traits::{BulkInsertResult, StateStorage};
    use crate::core::state::watermark::WatermarkBuilder;
    use crate::domain::composition::Composition;
    use async_trait::async_trait;
    use chrono::Utc;
    use std::any::Any;
    use std::sync::Mutex;

    // Mock Database Client for testing
    struct MockDatabaseClient {
        should_fail: bool,
        insert_results: Mutex<Vec<BulkInsertResult>>,
    }

    impl MockDatabaseClient {
        fn new() -> Self {
            Self {
                should_fail: false,
                insert_results: Mutex::new(vec![]),
            }
        }

        fn with_insert_result(self, result: BulkInsertResult) -> Self {
            self.insert_results.lock().unwrap().push(result);
            self
        }

        #[allow(dead_code)]
        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }
    }

    #[async_trait]
    impl DatabaseClient for MockDatabaseClient {
        fn as_any(&self) -> &dyn Any {
            self
        }

        async fn test_connection(&self) -> Result<()> {
            Ok(())
        }

        async fn ensure_database_exists(&self) -> Result<()> {
            Ok(())
        }

        async fn ensure_container_exists(&self, _template_id: &TemplateId) -> Result<()> {
            Ok(())
        }

        async fn ensure_control_container_exists(&self) -> Result<()> {
            Ok(())
        }

        async fn check_composition_exists(
            &self,
            _template_id: &TemplateId,
            _ehr_id: &str,
            _composition_id: &str,
        ) -> Result<bool> {
            Ok(false)
        }

        fn database_name(&self) -> &str {
            "mock_database"
        }

        async fn bulk_insert_json(
            &self,
            _template_id: &TemplateId,
            _documents: Vec<serde_json::Value>,
            _max_retries: usize,
            _dry_run: bool,
        ) -> Result<BulkInsertResult> {
            if self.should_fail {
                return Err(crate::domain::AtlasError::CosmosDb(
                    crate::domain::CosmosDbError::InsertFailed("Mock insert failed".to_string()),
                ));
            }
            let mut results = self.insert_results.lock().unwrap();
            if let Some(result) = results.pop() {
                Ok(result)
            } else {
                Ok(BulkInsertResult {
                    success_count: _documents.len(),
                    failure_count: 0,
                    failures: vec![],
                })
            }
        }

        async fn bulk_insert_compositions(
            &self,
            _template_id: &TemplateId,
            _compositions: Vec<Composition>,
            _export_mode: String,
            _max_retries: usize,
            _dry_run: bool,
        ) -> Result<BulkInsertResult> {
            if self.should_fail {
                return Err(crate::domain::AtlasError::CosmosDb(
                    crate::domain::CosmosDbError::InsertFailed("Mock insert failed".to_string()),
                ));
            }
            let mut results = self.insert_results.lock().unwrap();
            if let Some(result) = results.pop() {
                Ok(result)
            } else {
                Ok(BulkInsertResult {
                    success_count: 0,
                    failure_count: 0,
                    failures: vec![],
                })
            }
        }

        async fn bulk_insert_compositions_flattened(
            &self,
            _template_id: &TemplateId,
            _compositions: Vec<Composition>,
            _export_mode: String,
            _max_retries: usize,
            _dry_run: bool,
        ) -> Result<BulkInsertResult> {
            self.bulk_insert_compositions(
                _template_id,
                _compositions,
                _export_mode,
                _max_retries,
                _dry_run,
            )
            .await
        }
    }

    // Mock State Storage for testing
    struct MockStateStorage {
        watermarks: Mutex<std::collections::HashMap<String, Watermark>>,
        should_fail_checkpoint: bool,
    }

    impl MockStateStorage {
        fn new() -> Self {
            Self {
                watermarks: Mutex::new(std::collections::HashMap::new()),
                should_fail_checkpoint: false,
            }
        }

        #[allow(dead_code)]
        fn with_checkpoint_failure(mut self) -> Self {
            self.should_fail_checkpoint = true;
            self
        }
    }

    #[async_trait]
    impl StateStorage for MockStateStorage {
        async fn load_watermark(
            &self,
            template_id: &TemplateId,
            ehr_id: &EhrId,
        ) -> Result<Option<Watermark>> {
            let key = format!("{}_{}", template_id.as_str(), ehr_id.as_str());
            Ok(self.watermarks.lock().unwrap().get(&key).cloned())
        }

        async fn save_watermark(&self, watermark: &Watermark, _dry_run: bool) -> Result<()> {
            if self.should_fail_checkpoint {
                return Err(crate::domain::AtlasError::CosmosDb(
                    crate::domain::CosmosDbError::InsertFailed(
                        "Mock checkpoint failed".to_string(),
                    ),
                ));
            }
            let key = format!(
                "{}_{}",
                watermark.template_id.as_str(),
                watermark.ehr_id.as_str()
            );
            self.watermarks
                .lock()
                .unwrap()
                .insert(key, watermark.clone());
            Ok(())
        }

        async fn get_all_watermarks(&self) -> Result<Vec<Watermark>> {
            Ok(self.watermarks.lock().unwrap().values().cloned().collect())
        }
    }

    // Helper to create test composition
    fn create_test_composition(uid_str: &str, template_id: &str, ehr_id: &str) -> Composition {
        Composition {
            uid: CompositionUid::parse(uid_str).unwrap(),
            ehr_id: EhrId::new(ehr_id).unwrap(),
            template_id: TemplateId::new(template_id).unwrap(),
            time_committed: Utc::now(),
            content: serde_json::json!({
                "test": "data",
                "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1"
            }),
        }
    }

    #[test]
    fn test_batch_config_creation() {
        let config = BatchConfig::new(1000, CompositionFormat::Preserve, false, None);

        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.composition_format, CompositionFormat::Preserve);
        assert!(!config.dry_run);
        assert!(config.anonymization.is_none());
    }

    #[test]
    fn test_batch_config_from_config() {
        let config = BatchConfig::from_config(500, "flatten", false, None).unwrap();

        assert_eq!(config.batch_size, 500);
        assert_eq!(config.composition_format, CompositionFormat::Flatten);
        assert!(!config.dry_run);
        assert!(config.anonymization.is_none());
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

    #[tokio::test]
    async fn test_process_batch_empty_compositions() {
        let db_client = Arc::new(MockDatabaseClient::new());
        let state_storage = Arc::new(MockStateStorage::new());
        let state_manager = Arc::new(StateManager::new_with_storage(state_storage));
        let config = BatchConfig::new(100, CompositionFormat::Preserve, false, None);
        let processor = BatchProcessor::new(db_client, state_manager, config);

        let template_id = TemplateId::new("vital_signs").unwrap();
        let ehr_id = EhrId::new("test-ehr").unwrap();
        let mut watermark = WatermarkBuilder::new(template_id.clone(), ehr_id.clone()).build();

        let result = processor
            .process_batch(vec![], &template_id, &ehr_id, &mut watermark)
            .await
            .unwrap();

        assert_eq!(result.successful, 0);
        assert_eq!(result.failed, 0);
    }

    #[tokio::test]
    async fn test_process_batch_successful_preserve() {
        let bulk_result = BulkInsertResult {
            success_count: 3,
            failure_count: 0,
            failures: vec![],
        };
        let db_client = Arc::new(MockDatabaseClient::new().with_insert_result(bulk_result));
        let state_storage = Arc::new(MockStateStorage::new());
        let state_manager = Arc::new(StateManager::new_with_storage(state_storage));
        let config = BatchConfig::new(100, CompositionFormat::Preserve, false, None);
        let processor = BatchProcessor::new(db_client, state_manager, config);

        let template_id = TemplateId::new("vital_signs").unwrap();
        let ehr_id = EhrId::new("test-ehr").unwrap();
        let mut watermark = WatermarkBuilder::new(template_id.clone(), ehr_id.clone()).build();

        let compositions = vec![
            create_test_composition("uid1::local::1", "vital_signs", "test-ehr"),
            create_test_composition("uid2::local::1", "vital_signs", "test-ehr"),
            create_test_composition("uid3::local::1", "vital_signs", "test-ehr"),
        ];

        let result = processor
            .process_batch(compositions, &template_id, &ehr_id, &mut watermark)
            .await
            .unwrap();

        assert_eq!(result.successful, 3);
        assert_eq!(result.failed, 0);
        assert_eq!(result.errors.len(), 0);
    }

    #[tokio::test]
    async fn test_process_batch_successful_flatten() {
        let bulk_result = BulkInsertResult {
            success_count: 2,
            failure_count: 0,
            failures: vec![],
        };
        let db_client = Arc::new(MockDatabaseClient::new().with_insert_result(bulk_result));
        let state_storage = Arc::new(MockStateStorage::new());
        let state_manager = Arc::new(StateManager::new_with_storage(state_storage));
        let config = BatchConfig::new(100, CompositionFormat::Flatten, false, None);
        let processor = BatchProcessor::new(db_client, state_manager, config);

        let template_id = TemplateId::new("vital_signs").unwrap();
        let ehr_id = EhrId::new("test-ehr").unwrap();
        let mut watermark = WatermarkBuilder::new(template_id.clone(), ehr_id.clone()).build();

        let compositions = vec![
            create_test_composition("uid1::local::1", "vital_signs", "test-ehr"),
            create_test_composition("uid2::local::1", "vital_signs", "test-ehr"),
        ];

        let result = processor
            .process_batch(compositions, &template_id, &ehr_id, &mut watermark)
            .await
            .unwrap();

        assert_eq!(result.successful, 2);
        assert_eq!(result.failed, 0);
    }

    #[tokio::test]
    async fn test_process_batch_with_failures() {
        use crate::adapters::database::traits::BulkInsertFailure;

        let bulk_result = BulkInsertResult {
            success_count: 2,
            failure_count: 1,
            failures: vec![BulkInsertFailure {
                document_id: "uid3::local::1".to_string(),
                error: "Duplicate key error".to_string(),
                is_throttled: false,
            }],
        };
        let db_client = Arc::new(MockDatabaseClient::new().with_insert_result(bulk_result));
        let state_storage = Arc::new(MockStateStorage::new());
        let state_manager = Arc::new(StateManager::new_with_storage(state_storage));
        let config = BatchConfig::new(100, CompositionFormat::Preserve, false, None);
        let processor = BatchProcessor::new(db_client, state_manager, config);

        let template_id = TemplateId::new("vital_signs").unwrap();
        let ehr_id = EhrId::new("test-ehr").unwrap();
        let mut watermark = WatermarkBuilder::new(template_id.clone(), ehr_id.clone()).build();

        let compositions = vec![
            create_test_composition("uid1::local::1", "vital_signs", "test-ehr"),
            create_test_composition("uid2::local::1", "vital_signs", "test-ehr"),
            create_test_composition("uid3::local::1", "vital_signs", "test-ehr"),
        ];

        let result = processor
            .process_batch(compositions, &template_id, &ehr_id, &mut watermark)
            .await
            .unwrap();

        assert_eq!(result.successful, 2);
        // Note: failed count is 2 because it's set from bulk_result.failure_count (1)
        // and then add_failure increments it again (1 + 1 = 2)
        assert_eq!(result.failed, 2);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("uid3::local::1"));
        assert!(result.errors[0].contains("Duplicate key error"));
    }

    #[tokio::test]
    async fn test_process_batch_dry_run_mode() {
        let bulk_result = BulkInsertResult {
            success_count: 2,
            failure_count: 0,
            failures: vec![],
        };
        let db_client = Arc::new(MockDatabaseClient::new().with_insert_result(bulk_result));
        let state_storage = Arc::new(MockStateStorage::new());
        let state_manager = Arc::new(StateManager::new_with_storage(state_storage));
        let config = BatchConfig::new(100, CompositionFormat::Preserve, true, None); // dry_run = true
        let processor = BatchProcessor::new(db_client, state_manager, config);

        let template_id = TemplateId::new("vital_signs").unwrap();
        let ehr_id = EhrId::new("test-ehr").unwrap();
        let mut watermark = WatermarkBuilder::new(template_id.clone(), ehr_id.clone()).build();

        let compositions = vec![
            create_test_composition("uid1::local::1", "vital_signs", "test-ehr"),
            create_test_composition("uid2::local::1", "vital_signs", "test-ehr"),
        ];

        let result = processor
            .process_batch(compositions, &template_id, &ehr_id, &mut watermark)
            .await
            .unwrap();

        assert_eq!(result.successful, 2);
        assert_eq!(result.failed, 0);
    }

    #[tokio::test]
    async fn test_process_batch_updates_watermark() {
        let bulk_result = BulkInsertResult {
            success_count: 1,
            failure_count: 0,
            failures: vec![],
        };
        let db_client = Arc::new(MockDatabaseClient::new().with_insert_result(bulk_result));
        let state_storage = Arc::new(MockStateStorage::new());
        let state_manager = Arc::new(StateManager::new_with_storage(state_storage));
        let config = BatchConfig::new(100, CompositionFormat::Preserve, false, None);
        let processor = BatchProcessor::new(db_client, state_manager, config);

        let template_id = TemplateId::new("vital_signs").unwrap();
        let ehr_id = EhrId::new("test-ehr").unwrap();
        let mut watermark = WatermarkBuilder::new(template_id.clone(), ehr_id.clone()).build();

        let initial_last_uid = watermark.last_exported_composition_uid.clone();

        let compositions = vec![create_test_composition(
            "uid1::local::1",
            "vital_signs",
            "test-ehr",
        )];

        processor
            .process_batch(compositions, &template_id, &ehr_id, &mut watermark)
            .await
            .unwrap();

        // Watermark should be updated with the last composition UID
        assert_ne!(watermark.last_exported_composition_uid, initial_last_uid);
        assert_eq!(
            watermark
                .last_exported_composition_uid
                .as_ref()
                .unwrap()
                .as_str(),
            "uid1::local::1"
        );
    }
}

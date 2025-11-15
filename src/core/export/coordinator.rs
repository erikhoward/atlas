//! Export coordinator - main orchestrator for the export process
//!
//! This module coordinates the entire export workflow, managing the interaction
//! between openEHR, database backends, state management, and batch processing.

use crate::adapters::cosmosdb::{CosmosDbAdapter, CosmosDbClient};
use crate::adapters::database::create_database_and_state;
use crate::adapters::database::traits::DatabaseClient;
use crate::adapters::openehr::OpenEhrClient;
use crate::config::schema::DatabaseTarget;
use crate::config::AtlasConfig;
use crate::core::export::batch::{BatchConfig, BatchProcessor};
use crate::core::export::summary::{ExportError, ExportErrorType, ExportSummary};
use crate::core::state::{StateManager, Watermark, WatermarkBuilder};
use crate::core::verification::Verifier;
use crate::domain::ids::{EhrId, TemplateId};
use crate::domain::Result;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::watch;

/// Export coordinator
pub struct ExportCoordinator {
    config: AtlasConfig,
    #[allow(dead_code)] // Will be used in future phases
    openehr_client: Arc<OpenEhrClient>,
    database_client: Arc<dyn DatabaseClient + Send + Sync>,
    state_manager: Arc<StateManager>,
    #[allow(dead_code)] // Will be used in future phases
    batch_processor: Arc<BatchProcessor>,
    /// Cosmos DB client for verification (only available when using CosmosDB)
    cosmos_client: Option<Arc<CosmosDbClient>>,
    /// Shutdown signal receiver for graceful shutdown
    shutdown_signal: watch::Receiver<bool>,
}

impl ExportCoordinator {
    /// Create a new export coordinator
    ///
    /// # Arguments
    ///
    /// * `config` - Atlas configuration
    /// * `shutdown_signal` - Receiver for shutdown signal (true = shutdown requested)
    pub async fn new(config: AtlasConfig, shutdown_signal: watch::Receiver<bool>) -> Result<Self> {
        // Create openEHR client
        let openehr_client = Arc::new(OpenEhrClient::new(config.openehr.clone()).await?);

        // Create database client and state storage using factory
        let (database_client, state_storage) = create_database_and_state(&config).await?;

        // Ensure database exists
        database_client.ensure_database_exists().await?;

        // Ensure control container exists for state management
        database_client.ensure_control_container_exists().await?;

        // Create state manager with state storage
        let state_manager = Arc::new(StateManager::new_with_storage(state_storage));

        // Create batch configuration
        let batch_config = BatchConfig::from_config(
            config.openehr.query.batch_size,
            &config.export.export_composition_format,
            config.export.dry_run,
            config.anonymization.clone(),
        )?;

        // Create batch processor
        let batch_processor = Arc::new(BatchProcessor::new(
            database_client.clone(),
            state_manager.clone(),
            batch_config,
        ));

        // Get Cosmos DB client for verification if using CosmosDB
        // We reuse the existing client from the adapter instead of creating a new one
        let cosmos_client = if config.database_target == DatabaseTarget::CosmosDB {
            // Downcast the trait object to get the concrete CosmosDbAdapter
            let adapter = database_client
                .as_any()
                .downcast_ref::<CosmosDbAdapter>()
                .expect("database_client should be CosmosDbAdapter when using CosmosDB");
            let client = adapter.client().clone();
            tracing::debug!(
                endpoint = %client.endpoint(),
                database = %client.database_name(),
                "Reusing existing CosmosDbClient for verification"
            );
            Some(client)
        } else {
            None
        };

        Ok(Self {
            config,
            openehr_client,
            database_client,
            state_manager,
            batch_processor,
            cosmos_client,
            shutdown_signal,
        })
    }

    /// Check if shutdown has been requested
    fn is_shutdown_requested(&self) -> bool {
        *self.shutdown_signal.borrow()
    }

    /// Validate configuration and parse template IDs
    ///
    /// # Arguments
    ///
    /// * `summary` - Export summary to update with errors if validation fails
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(template_ids))` if validation succeeds and template IDs are valid,
    /// `Ok(None)` if validation fails (error added to summary), or `Err` for unexpected errors
    fn validate_and_prepare_export(
        &self,
        summary: &mut ExportSummary,
    ) -> Result<Option<Vec<TemplateId>>> {
        // Validate configuration
        if let Err(e) = self.config.validate() {
            let error = ExportError::new(ExportErrorType::Configuration, e);
            summary.add_error(error);
            return Ok(None);
        }

        // Get template IDs to process
        let template_ids: Vec<TemplateId> = self
            .config
            .openehr
            .query
            .template_ids
            .iter()
            .filter_map(|id| TemplateId::from_str(id).ok())
            .collect();

        if template_ids.is_empty() {
            tracing::warn!("No valid template IDs to process");
            return Ok(None);
        }

        Ok(Some(template_ids))
    }

    /// Process all templates for all EHRs
    ///
    /// # Arguments
    ///
    /// * `template_ids` - List of template IDs to process
    /// * `ehr_ids` - List of EHR IDs to process
    /// * `summary` - Export summary to update with results
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if processing completed normally,
    /// `Ok(false)` if shutdown was requested, or `Err` for unexpected errors
    async fn process_templates(
        &self,
        template_ids: &[TemplateId],
        ehr_ids: &[EhrId],
        summary: &mut ExportSummary,
    ) -> Result<bool> {
        for template_id in template_ids {
            // Check for shutdown signal before processing each template
            if self.is_shutdown_requested() {
                tracing::info!("Shutdown signal received, stopping export");
                summary.interrupted = true;
                summary.shutdown_reason = Some("User signal (SIGTERM/SIGINT)".to_string());
                return Ok(false);
            }

            tracing::info!(
                template_id = %template_id.as_str(),
                "Processing template"
            );

            // Ensure container exists for this template
            if let Err(e) = self
                .database_client
                .ensure_container_exists(template_id)
                .await
            {
                tracing::error!(
                    template_id = %template_id.as_str(),
                    error = %e,
                    "Failed to create container"
                );
                summary.add_error(
                    ExportError::new(
                        ExportErrorType::Storage,
                        format!("Failed to create container: {e}"),
                    )
                    .with_context(format!("template_id={}", template_id.as_str())),
                );
                continue;
            }

            // Process each EHR for this template
            if !self
                .process_ehrs_for_template(template_id, ehr_ids, summary)
                .await?
            {
                return Ok(false); // Shutdown requested
            }
        }

        Ok(true)
    }

    /// Process all EHRs for a single template
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID to process
    /// * `ehr_ids` - List of EHR IDs to process
    /// * `summary` - Export summary to update with results
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if processing completed normally,
    /// `Ok(false)` if shutdown was requested, or `Err` for unexpected errors
    async fn process_ehrs_for_template(
        &self,
        template_id: &TemplateId,
        ehr_ids: &[EhrId],
        summary: &mut ExportSummary,
    ) -> Result<bool> {
        for ehr_id in ehr_ids {
            // Check for shutdown signal before processing each EHR
            if self.is_shutdown_requested() {
                tracing::info!("Shutdown signal received, stopping export");
                summary.interrupted = true;
                summary.shutdown_reason = Some("User signal (SIGTERM/SIGINT)".to_string());
                return Ok(false);
            }

            match self
                .process_ehr_for_template(template_id, ehr_id, summary)
                .await
            {
                Ok(_) => {
                    tracing::debug!(
                        template_id = %template_id.as_str(),
                        ehr_id = %ehr_id.as_str(),
                        "Completed processing EHR"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        template_id = %template_id.as_str(),
                        ehr_id = %ehr_id.as_str(),
                        error = %e,
                        "Failed to process EHR"
                    );
                    summary.add_error(
                        ExportError::new(
                            ExportErrorType::Unknown,
                            format!("Failed to process EHR: {e}"),
                        )
                        .with_context(format!(
                            "template_id={}, ehr_id={}",
                            template_id.as_str(),
                            ehr_id.as_str()
                        )),
                    );
                }
            }
        }

        Ok(true)
    }

    /// Run post-export verification if enabled
    ///
    /// # Arguments
    ///
    /// * `summary` - Export summary containing exported compositions to verify
    async fn run_post_export_verification(&self, summary: &mut ExportSummary) {
        if !self.config.verification.enable_verification {
            return;
        }

        let Some(cosmos_client) = &self.cosmos_client else {
            tracing::warn!(
                "Verification is enabled but not available for the current database target"
            );
            return;
        };

        tracing::info!("Running post-export verification");
        let verifier = Verifier::new(cosmos_client.clone());

        match verifier.verify_export(summary).await {
            Ok(verification_report) => {
                tracing::info!(
                    total_verified = verification_report.total_verified,
                    passed = verification_report.passed,
                    failed = verification_report.failed,
                    success_rate = format!("{:.2}%", verification_report.success_rate()),
                    "Verification completed"
                );

                // Log verification failures
                if !verification_report.is_success() {
                    tracing::warn!(
                        failed_count = verification_report.failed,
                        "Verification found {} composition(s) that could not be found in the database",
                        verification_report.failed
                    );
                    for failure in &verification_report.failures {
                        tracing::warn!(
                            composition_uid = %failure.composition_uid.as_str(),
                            ehr_id = %failure.ehr_id.as_str(),
                            template_id = %failure.template_id.as_str(),
                            reason = %failure.reason,
                            "Verification failure"
                        );
                    }
                }

                // Store verification report in summary
                summary.set_verification_report(verification_report);
            }
            Err(e) => {
                tracing::error!(error = %e, "Verification failed");
                summary.add_error(ExportError::new(
                    ExportErrorType::Unknown,
                    format!("Verification failed: {e}"),
                ));
            }
        }
    }

    /// Execute the export
    ///
    /// This is the main entry point for the export process. It:
    /// 1. Validates configuration
    /// 2. Connects to openEHR and Cosmos DB
    /// 3. Loads or creates state
    /// 4. Determines EHRs to process
    /// 5. For each template_id:
    ///    - For each EHR:
    ///      - Determines compositions to export (incremental logic)
    ///      - Fetches compositions in batches
    ///      - Transforms and loads
    ///      - Checkpoints progress
    /// 6. Generates summary report
    pub async fn execute_export(&self) -> Result<ExportSummary> {
        let start_time = Instant::now();
        let mut summary = ExportSummary::new();
        summary.dry_run = self.config.export.dry_run;

        tracing::info!("Starting export process");

        // Validate configuration and get template IDs
        let template_ids = match self.validate_and_prepare_export(&mut summary)? {
            Some(ids) => ids,
            None => return Ok(summary.with_duration(start_time.elapsed())),
        };

        // Get EHR IDs to process
        let ehr_ids = self.get_ehr_ids_to_process().await?;
        summary.total_ehrs = ehr_ids.len();

        tracing::info!(
            template_count = template_ids.len(),
            ehr_count = ehr_ids.len(),
            "Processing templates and EHRs"
        );

        // Process all templates
        if !self
            .process_templates(&template_ids, &ehr_ids, &mut summary)
            .await?
        {
            return Ok(summary.with_duration(start_time.elapsed()));
        }

        // Run post-export verification
        self.run_post_export_verification(&mut summary).await;

        let duration = start_time.elapsed();
        summary = summary.with_duration(duration);
        summary.log_summary();

        Ok(summary)
    }

    /// Load or create watermark for a template and EHR
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID
    /// * `ehr_id` - EHR ID
    ///
    /// # Returns
    ///
    /// Returns the watermark (either loaded from state or newly created)
    async fn load_or_create_watermark(
        &self,
        template_id: &TemplateId,
        ehr_id: &EhrId,
    ) -> Result<Watermark> {
        let watermark = match self
            .state_manager
            .load_watermark(template_id, ehr_id)
            .await?
        {
            Some(wm) => {
                tracing::info!(
                    template_id = %template_id.as_str(),
                    ehr_id = %ehr_id.as_str(),
                    last_exported = %wm.last_exported_timestamp,
                    "Loaded existing watermark - incremental export"
                );
                wm
            }
            None => {
                tracing::info!(
                    template_id = %template_id.as_str(),
                    ehr_id = %ehr_id.as_str(),
                    "No watermark found - full export"
                );
                WatermarkBuilder::new(template_id.clone(), ehr_id.clone()).build()
            }
        };

        Ok(watermark)
    }

    /// Fetch compositions for an EHR and template
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID
    /// * `ehr_id` - EHR ID
    /// * `watermark` - Watermark to determine incremental query timestamp
    ///
    /// # Returns
    ///
    /// Returns a vector of compositions
    async fn fetch_compositions_for_ehr(
        &self,
        template_id: &TemplateId,
        ehr_id: &EhrId,
        watermark: &Watermark,
    ) -> Result<Vec<crate::domain::Composition>> {
        // Determine the timestamp to query from (for incremental exports)
        let since = if self.config.export.mode == "incremental" {
            Some(watermark.last_exported_timestamp)
        } else {
            None
        };

        // Fetch composition metadata from openEHR
        let compositions_metadata = self
            .openehr_client
            .vendor()
            .get_compositions_for_ehr(ehr_id, template_id, since)
            .await?;

        tracing::info!(
            template_id = %template_id.as_str(),
            ehr_id = %ehr_id.as_str(),
            count = compositions_metadata.len(),
            "Found compositions for EHR"
        );

        // Fetch full composition data
        let mut compositions = Vec::new();
        for metadata in compositions_metadata {
            match self
                .openehr_client
                .vendor()
                .fetch_composition(&metadata)
                .await
            {
                Ok(composition) => compositions.push(composition),
                Err(e) => {
                    tracing::warn!(
                        composition_uid = %metadata.uid,
                        error = %e,
                        "Failed to fetch composition, skipping"
                    );
                }
            }
        }

        Ok(compositions)
    }

    /// Process compositions and update summary
    ///
    /// # Arguments
    ///
    /// * `compositions` - Compositions to process
    /// * `template_id` - Template ID
    /// * `ehr_id` - EHR ID
    /// * `watermark` - Watermark to update
    /// * `summary` - Export summary to update
    async fn process_and_update_summary(
        &self,
        compositions: Vec<crate::domain::Composition>,
        template_id: &TemplateId,
        ehr_id: &EhrId,
        watermark: &mut Watermark,
        summary: &mut ExportSummary,
    ) -> Result<()> {
        if compositions.is_empty() {
            return Ok(());
        }

        let batch_result = self
            .batch_processor
            .process_batch(compositions.clone(), template_id, ehr_id, watermark)
            .await?;

        // Update summary with batch results
        summary.total_compositions += batch_result.successful + batch_result.failed;
        summary.successful_exports += batch_result.successful;
        summary.failed_exports += batch_result.failed;
        summary.duplicates_skipped += batch_result.duplicates_skipped;

        // Add batch errors to summary
        for error_msg in batch_result.errors {
            summary.add_error(
                ExportError::new(ExportErrorType::Unknown, error_msg).with_context(format!(
                    "template_id={}, ehr_id={}",
                    template_id.as_str(),
                    ehr_id.as_str()
                )),
            );
        }

        // Add compositions to summary for verification
        for composition in &compositions {
            summary.add_exported_composition(
                composition.uid.clone(),
                ehr_id.clone(),
                template_id.clone(),
            );
        }

        Ok(())
    }

    /// Get EHR IDs to process
    async fn get_ehr_ids_to_process(&self) -> Result<Vec<EhrId>> {
        // If specific EHR IDs are configured, use those
        if !self.config.openehr.query.ehr_ids.is_empty() {
            tracing::info!(
                count = self.config.openehr.query.ehr_ids.len(),
                "Using configured EHR IDs"
            );
            return Ok(self
                .config
                .openehr
                .query
                .ehr_ids
                .iter()
                .filter_map(|id| EhrId::from_str(id).ok())
                .collect());
        }

        // Otherwise, fetch all EHR IDs from openEHR vendor
        tracing::info!("No EHR IDs configured - fetching all EHR IDs from openEHR server");
        let ehr_ids = self.openehr_client.vendor().get_ehr_ids().await?;

        tracing::info!(count = ehr_ids.len(), "Fetched EHR IDs from openEHR server");

        Ok(ehr_ids)
    }

    /// Process a single EHR for a template
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID to process
    /// * `ehr_id` - EHR ID to process
    /// * `summary` - Export summary to update with results
    async fn process_ehr_for_template(
        &self,
        template_id: &TemplateId,
        ehr_id: &EhrId,
        summary: &mut ExportSummary,
    ) -> Result<()> {
        tracing::debug!(
            template_id = %template_id.as_str(),
            ehr_id = %ehr_id.as_str(),
            "Processing EHR for template"
        );

        // Load or create watermark
        let mut watermark = self.load_or_create_watermark(template_id, ehr_id).await?;

        // Mark export as started
        watermark.mark_started();
        self.state_manager
            .save_watermark(&watermark, self.config.export.dry_run)
            .await?;

        // Fetch compositions for this EHR and template
        let compositions = self
            .fetch_compositions_for_ehr(template_id, ehr_id, &watermark)
            .await?;

        // If no compositions found, mark as completed and return
        if compositions.is_empty() {
            watermark.mark_completed();
            self.state_manager
                .save_watermark(&watermark, self.config.export.dry_run)
                .await?;
            return Ok(());
        }

        // Process compositions and update summary
        self.process_and_update_summary(compositions, template_id, ehr_id, &mut watermark, summary)
            .await?;

        // Mark export as completed and save watermark
        watermark.mark_completed();
        self.state_manager
            .save_watermark(&watermark, self.config.export.dry_run)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::database::traits::{BulkInsertResult, StateStorage};
    use crate::adapters::openehr::vendor::{CompositionMetadata, OpenEhrVendor};
    use crate::core::state::watermark::Watermark;
    use crate::domain::composition::Composition;
    use crate::domain::ids::CompositionUid;
    use async_trait::async_trait;
    use chrono::Utc;
    use std::any::Any;
    use std::sync::Mutex;

    // Mock openEHR Vendor
    struct MockOpenEhrVendor {
        ehr_ids: Vec<EhrId>,
        compositions_metadata: Vec<CompositionMetadata>,
        compositions: Vec<Composition>,
        should_fail: bool,
    }

    impl MockOpenEhrVendor {
        fn new() -> Self {
            Self {
                ehr_ids: vec![],
                compositions_metadata: vec![],
                compositions: vec![],
                should_fail: false,
            }
        }

        fn with_ehr_ids(mut self, ehr_ids: Vec<EhrId>) -> Self {
            self.ehr_ids = ehr_ids;
            self
        }

        #[allow(dead_code)]
        fn with_compositions_metadata(mut self, metadata: Vec<CompositionMetadata>) -> Self {
            self.compositions_metadata = metadata;
            self
        }

        #[allow(dead_code)]
        fn with_compositions(mut self, compositions: Vec<Composition>) -> Self {
            self.compositions = compositions;
            self
        }

        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }
    }

    #[async_trait]
    impl OpenEhrVendor for MockOpenEhrVendor {
        async fn authenticate(&mut self) -> Result<()> {
            if self.should_fail {
                return Err(crate::domain::AtlasError::OpenEhr(
                    crate::domain::OpenEhrError::AuthenticationFailed(
                        "Mock auth failed".to_string(),
                    ),
                ));
            }
            Ok(())
        }

        async fn get_ehr_ids(&self) -> Result<Vec<EhrId>> {
            if self.should_fail {
                return Err(crate::domain::AtlasError::OpenEhr(
                    crate::domain::OpenEhrError::QueryFailed("Mock query failed".to_string()),
                ));
            }
            Ok(self.ehr_ids.clone())
        }

        async fn get_compositions_for_ehr(
            &self,
            _ehr_id: &EhrId,
            _template_id: &TemplateId,
            _since: Option<chrono::DateTime<Utc>>,
        ) -> Result<Vec<CompositionMetadata>> {
            if self.should_fail {
                return Err(crate::domain::AtlasError::OpenEhr(
                    crate::domain::OpenEhrError::QueryFailed("Mock query failed".to_string()),
                ));
            }
            Ok(self.compositions_metadata.clone())
        }

        async fn fetch_composition(&self, _metadata: &CompositionMetadata) -> Result<Composition> {
            if self.should_fail {
                return Err(crate::domain::AtlasError::OpenEhr(
                    crate::domain::OpenEhrError::QueryFailed("Mock fetch failed".to_string()),
                ));
            }
            if let Some(comp) = self.compositions.first() {
                Ok(comp.clone())
            } else {
                Err(crate::domain::AtlasError::OpenEhr(
                    crate::domain::OpenEhrError::CompositionNotFound("No compositions".to_string()),
                ))
            }
        }

        fn is_authenticated(&self) -> bool {
            !self.should_fail
        }

        fn base_url(&self) -> &str {
            "http://mock.ehrbase.org"
        }
    }

    // Mock Database Client
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

        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }

        fn with_insert_result(self, result: BulkInsertResult) -> Self {
            self.insert_results.lock().unwrap().push(result);
            self
        }
    }

    #[async_trait]
    impl DatabaseClient for MockDatabaseClient {
        fn as_any(&self) -> &dyn Any {
            self
        }

        async fn test_connection(&self) -> Result<()> {
            if self.should_fail {
                return Err(crate::domain::AtlasError::CosmosDb(
                    crate::domain::CosmosDbError::ConnectionFailed(
                        "Mock connection failed".to_string(),
                    ),
                ));
            }
            Ok(())
        }

        async fn ensure_database_exists(&self) -> Result<()> {
            if self.should_fail {
                return Err(crate::domain::AtlasError::CosmosDb(
                    crate::domain::CosmosDbError::ConnectionFailed(
                        "Mock database creation failed".to_string(),
                    ),
                ));
            }
            Ok(())
        }

        async fn ensure_container_exists(&self, _template_id: &TemplateId) -> Result<()> {
            if self.should_fail {
                return Err(crate::domain::AtlasError::CosmosDb(
                    crate::domain::CosmosDbError::ContainerCreationFailed(
                        "Mock container creation failed".to_string(),
                    ),
                ));
            }
            Ok(())
        }

        async fn ensure_control_container_exists(&self) -> Result<()> {
            if self.should_fail {
                return Err(crate::domain::AtlasError::CosmosDb(
                    crate::domain::CosmosDbError::ContainerCreationFailed(
                        "Mock control container creation failed".to_string(),
                    ),
                ));
            }
            Ok(())
        }

        async fn check_composition_exists(
            &self,
            _template_id: &TemplateId,
            _ehr_id: &str,
            _composition_id: &str,
        ) -> Result<bool> {
            if self.should_fail {
                return Err(crate::domain::AtlasError::CosmosDb(
                    crate::domain::CosmosDbError::QueryFailed("Mock query failed".to_string()),
                ));
            }
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
            Ok(BulkInsertResult {
                success_count: _documents.len(),
                failure_count: 0,
                failures: vec![],
            })
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

    // Mock State Storage
    struct MockStateStorage {
        watermarks: Mutex<std::collections::HashMap<String, Watermark>>,
        should_fail: bool,
    }

    impl MockStateStorage {
        fn new() -> Self {
            Self {
                watermarks: Mutex::new(std::collections::HashMap::new()),
                should_fail: false,
            }
        }

        #[allow(dead_code)]
        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }

        #[allow(dead_code)]
        fn with_watermark(
            self,
            template_id: &TemplateId,
            ehr_id: &EhrId,
            watermark: Watermark,
        ) -> Self {
            let key = format!("{}_{}", template_id.as_str(), ehr_id.as_str());
            self.watermarks.lock().unwrap().insert(key, watermark);
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
            if self.should_fail {
                return Err(crate::domain::AtlasError::CosmosDb(
                    crate::domain::CosmosDbError::QueryFailed("Mock query failed".to_string()),
                ));
            }
            let key = format!("{}_{}", template_id.as_str(), ehr_id.as_str());
            Ok(self.watermarks.lock().unwrap().get(&key).cloned())
        }

        async fn save_watermark(&self, watermark: &Watermark, _dry_run: bool) -> Result<()> {
            if self.should_fail {
                return Err(crate::domain::AtlasError::CosmosDb(
                    crate::domain::CosmosDbError::InsertFailed("Mock save failed".to_string()),
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
            if self.should_fail {
                return Err(crate::domain::AtlasError::CosmosDb(
                    crate::domain::CosmosDbError::QueryFailed("Mock query failed".to_string()),
                ));
            }
            Ok(self.watermarks.lock().unwrap().values().cloned().collect())
        }
    }

    #[test]
    fn test_is_shutdown_requested_false() {
        let (_tx, rx) = watch::channel(false);

        // Test that shutdown signal is initially false
        assert!(!*rx.borrow());
    }

    #[test]
    fn test_is_shutdown_requested_true() {
        let (tx, rx) = watch::channel(false);

        // Send shutdown signal
        tx.send(true).unwrap();

        assert!(*rx.borrow());
    }

    // Helper to create test composition metadata
    #[allow(dead_code)]
    fn create_test_metadata(uid_str: &str, template_id: &str, ehr_id: &str) -> CompositionMetadata {
        CompositionMetadata::new(
            CompositionUid::parse(uid_str).unwrap(),
            TemplateId::new(template_id).unwrap(),
            EhrId::new(ehr_id).unwrap(),
            Utc::now(),
        )
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
    fn test_mock_openehr_vendor_creation() {
        let vendor = MockOpenEhrVendor::new();
        assert!(vendor.is_authenticated());
        assert_eq!(vendor.base_url(), "http://mock.ehrbase.org");
    }

    #[tokio::test]
    async fn test_mock_openehr_vendor_get_ehr_ids() {
        let ehr_id = EhrId::new("test-ehr-123").unwrap();
        let vendor = MockOpenEhrVendor::new().with_ehr_ids(vec![ehr_id.clone()]);

        let result = vendor.get_ehr_ids().await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], ehr_id);
    }

    #[tokio::test]
    async fn test_mock_openehr_vendor_failure() {
        let vendor = MockOpenEhrVendor::new().with_failure();

        let result = vendor.get_ehr_ids().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_database_client_connection() {
        let client = MockDatabaseClient::new();
        assert!(client.test_connection().await.is_ok());

        let failing_client = MockDatabaseClient::new().with_failure();
        assert!(failing_client.test_connection().await.is_err());
    }

    #[tokio::test]
    async fn test_mock_state_storage_load_save() {
        let storage = MockStateStorage::new();
        let template_id = TemplateId::new("vital_signs").unwrap();
        let ehr_id = EhrId::new("test-ehr").unwrap();

        // Load non-existent watermark
        let result = storage.load_watermark(&template_id, &ehr_id).await.unwrap();
        assert!(result.is_none());

        // Save watermark
        let watermark = WatermarkBuilder::new(template_id.clone(), ehr_id.clone()).build();
        storage.save_watermark(&watermark, false).await.unwrap();

        // Load saved watermark
        let loaded = storage.load_watermark(&template_id, &ehr_id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().template_id, template_id);
    }

    #[tokio::test]
    async fn test_mock_database_client_bulk_insert() {
        let result = BulkInsertResult {
            success_count: 5,
            failure_count: 0,
            failures: vec![],
        };
        let client = MockDatabaseClient::new().with_insert_result(result);

        let template_id = TemplateId::new("vital_signs").unwrap();
        let compositions = vec![create_test_composition(
            "uid1::local::1",
            "vital_signs",
            "ehr1",
        )];

        let insert_result = client
            .bulk_insert_compositions(&template_id, compositions, "preserve".to_string(), 3, false)
            .await
            .unwrap();

        assert_eq!(insert_result.success_count, 5);
        assert_eq!(insert_result.failure_count, 0);
    }

    #[tokio::test]
    async fn test_mock_database_client_ensure_container() {
        let client = MockDatabaseClient::new();
        let template_id = TemplateId::new("vital_signs").unwrap();

        assert!(client.ensure_container_exists(&template_id).await.is_ok());

        let failing_client = MockDatabaseClient::new().with_failure();
        assert!(failing_client
            .ensure_container_exists(&template_id)
            .await
            .is_err());
    }
}

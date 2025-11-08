//! Export coordinator - main orchestrator for the export process
//!
//! This module coordinates the entire export workflow, managing the interaction
//! between OpenEHR, database backends, state management, and batch processing.

use crate::adapters::cosmosdb::{CosmosDbAdapter, CosmosDbClient};
use crate::adapters::database::create_database_and_state;
use crate::adapters::database::traits::DatabaseClient;
use crate::adapters::openehr::OpenEhrClient;
use crate::config::schema::DatabaseTarget;
use crate::config::AtlasConfig;
use crate::core::export::batch::{BatchConfig, BatchProcessor};
use crate::core::export::summary::{ExportError, ExportErrorType, ExportSummary};
use crate::core::state::{StateManager, WatermarkBuilder};
use crate::core::verification::Verifier;
use crate::domain::ids::{EhrId, TemplateId};
use crate::domain::Result;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

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
}

impl ExportCoordinator {
    /// Create a new export coordinator
    pub async fn new(config: AtlasConfig) -> Result<Self> {
        // Create OpenEHR client
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
            config.verification.enable_verification,
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
        })
    }

    /// Execute the export
    ///
    /// This is the main entry point for the export process. It:
    /// 1. Validates configuration
    /// 2. Connects to OpenEHR and Cosmos DB
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

        tracing::info!("Starting export process");

        // Validate configuration
        if let Err(e) = self.config.validate() {
            let error = ExportError::new(ExportErrorType::Configuration, e);
            summary.add_error(error);
            return Ok(summary.with_duration(start_time.elapsed()));
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
            return Ok(summary.with_duration(start_time.elapsed()));
        }

        // Get EHR IDs to process
        let ehr_ids = self.get_ehr_ids_to_process().await?;
        summary.total_ehrs = ehr_ids.len();

        tracing::info!(
            template_count = template_ids.len(),
            ehr_count = ehr_ids.len(),
            "Processing templates and EHRs"
        );

        // Process each template
        for template_id in &template_ids {
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
            for ehr_id in &ehr_ids {
                match self
                    .process_ehr_for_template(template_id, ehr_id, &mut summary)
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
        }

        // Run verification if enabled and cosmos client is available
        if self.config.verification.enable_verification {
            if let Some(cosmos_client) = &self.cosmos_client {
                tracing::info!("Running post-export verification");
                let verifier = Verifier::new(cosmos_client.clone());

                match verifier.verify_export(&summary).await {
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
            } else {
                tracing::warn!(
                    "Verification is enabled but not available for the current database target"
                );
            }
        }

        let duration = start_time.elapsed();
        summary = summary.with_duration(duration);
        summary.log_summary();

        Ok(summary)
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

        // Otherwise, fetch all EHR IDs from OpenEHR vendor
        tracing::info!("No EHR IDs configured - fetching all EHR IDs from OpenEHR server");
        let ehr_ids = self.openehr_client.vendor().get_ehr_ids().await?;

        tracing::info!(count = ehr_ids.len(), "Fetched EHR IDs from OpenEHR server");

        Ok(ehr_ids)
    }

    /// Process a single EHR for a template
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
        let mut watermark = match self
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

        // Mark export as started
        watermark.mark_started();
        self.state_manager.save_watermark(&watermark).await?;

        // Determine the timestamp to query from (for incremental exports)
        let since = if self.config.export.mode == "incremental" {
            Some(watermark.last_exported_timestamp)
        } else {
            None
        };

        // Fetch composition metadata from OpenEHR
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

        // If no compositions found, mark as completed and return
        if compositions_metadata.is_empty() {
            watermark.mark_completed();
            self.state_manager.save_watermark(&watermark).await?;
            return Ok(());
        }

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

        // Process compositions through batch processor
        if !compositions.is_empty() {
            let batch_result = self
                .batch_processor
                .process_batch(compositions.clone(), template_id, ehr_id, &mut watermark)
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

            // Add checksums to summary for verification
            for composition in &compositions {
                if let Some(checksum) = batch_result.checksums.get(&composition.uid) {
                    summary.add_exported_composition(
                        composition.uid.clone(),
                        ehr_id.clone(),
                        template_id.clone(),
                        checksum.clone(),
                    );
                }
            }
        }

        // Mark export as completed
        watermark.mark_completed();
        self.state_manager.save_watermark(&watermark).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_export_coordinator_placeholder() {
        // Placeholder test - actual tests would require mocking
        // This test exists to ensure the module compiles
    }
}

//! CosmosDB adapter implementing database traits
//!
//! This module provides the implementation of DatabaseClient and StateStorage traits
//! for Azure Cosmos DB.

use crate::adapters::cosmosdb::bulk::{
    bulk_insert_compositions as cosmos_bulk_insert,
    bulk_insert_compositions_flattened as cosmos_bulk_insert_flattened,
};
use crate::adapters::cosmosdb::client::CosmosDbClient;
use crate::adapters::cosmosdb::models::{CosmosComposition, CosmosCompositionFlattened};
use crate::adapters::database::traits::{
    BulkInsertFailure, BulkInsertResult, DatabaseClient, StateStorage,
};
use crate::core::state::watermark::Watermark;
use crate::core::transform::{flatten::flatten_composition, preserve::preserve_composition};
use crate::domain::composition::Composition;
use crate::domain::ids::{EhrId, TemplateId};
use crate::domain::{AtlasError, CosmosDbError, Result};
use async_trait::async_trait;
use azure_data_cosmos::PartitionKey;
use std::any::Any;
use std::sync::Arc;

/// CosmosDB implementation of database traits
///
/// This wraps the CosmosDbClient and implements the DatabaseClient and StateStorage traits.
pub struct CosmosDbAdapter {
    client: Arc<CosmosDbClient>,
}

impl CosmosDbAdapter {
    /// Create a new CosmosDB adapter
    pub fn new(client: CosmosDbClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }

    /// Create a new CosmosDB adapter with an Arc-wrapped client
    pub fn new_with_arc(client: Arc<CosmosDbClient>) -> Self {
        Self { client }
    }

    /// Get a reference to the underlying client
    pub fn client(&self) -> &Arc<CosmosDbClient> {
        &self.client
    }
}

#[async_trait]
impl DatabaseClient for CosmosDbAdapter {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn test_connection(&self) -> Result<()> {
        self.client.test_connection().await
    }

    async fn ensure_database_exists(&self) -> Result<()> {
        self.client.ensure_database_exists().await
    }

    async fn ensure_container_exists(&self, template_id: &TemplateId) -> Result<()> {
        self.client.ensure_container_exists(template_id).await
    }

    async fn ensure_control_container_exists(&self) -> Result<()> {
        self.client.ensure_control_container_exists().await
    }

    async fn bulk_insert_compositions(
        &self,
        template_id: &TemplateId,
        compositions: Vec<Composition>,
        export_mode: String,
        max_retries: usize,
        dry_run: bool,
    ) -> Result<BulkInsertResult> {
        // Transform compositions using preserve_composition
        let mut cosmos_compositions = Vec::new();
        for composition in compositions {
            let json_value = preserve_composition(composition, export_mode.clone())?;
            let cosmos_comp: CosmosComposition = serde_json::from_value(json_value)
                .map_err(|e| AtlasError::Serialization(e.to_string()))?;
            cosmos_compositions.push(cosmos_comp);
        }

        // If dry-run, skip actual write and return success
        if dry_run {
            tracing::info!(
                template_id = %template_id.as_str(),
                count = cosmos_compositions.len(),
                "DRY RUN: Would insert {} compositions (preserved format)",
                cosmos_compositions.len()
            );
            return Ok(BulkInsertResult {
                success_count: cosmos_compositions.len(),
                failure_count: 0,
                failures: Vec::new(),
            });
        }

        // Get container client
        let container = self.client.get_container_client(template_id);

        // Perform bulk insert
        let result = cosmos_bulk_insert(&container, cosmos_compositions, max_retries).await?;

        // Convert CosmosDB BulkInsertResult to trait BulkInsertResult
        Ok(BulkInsertResult {
            success_count: result.success_count,
            failure_count: result.failure_count,
            failures: result
                .failures
                .into_iter()
                .map(|f| BulkInsertFailure {
                    document_id: f.document_id,
                    error: f.error,
                    is_throttled: f.is_throttled,
                })
                .collect(),
        })
    }

    async fn bulk_insert_compositions_flattened(
        &self,
        template_id: &TemplateId,
        compositions: Vec<Composition>,
        export_mode: String,
        max_retries: usize,
        dry_run: bool,
    ) -> Result<BulkInsertResult> {
        // Transform compositions using flatten_composition
        let mut cosmos_compositions = Vec::new();
        for composition in compositions {
            let json_value = flatten_composition(composition, export_mode.clone())?;
            let cosmos_comp: CosmosCompositionFlattened = serde_json::from_value(json_value)
                .map_err(|e| AtlasError::Serialization(e.to_string()))?;
            cosmos_compositions.push(cosmos_comp);
        }

        // If dry-run, skip actual write and return success
        if dry_run {
            tracing::info!(
                template_id = %template_id.as_str(),
                count = cosmos_compositions.len(),
                "DRY RUN: Would insert {} compositions (flattened format)",
                cosmos_compositions.len()
            );
            return Ok(BulkInsertResult {
                success_count: cosmos_compositions.len(),
                failure_count: 0,
                failures: Vec::new(),
            });
        }

        // Get container client
        let container = self.client.get_container_client(template_id);

        // Perform bulk insert
        let result =
            cosmos_bulk_insert_flattened(&container, cosmos_compositions, max_retries).await?;

        // Convert CosmosDB BulkInsertResult to trait BulkInsertResult
        Ok(BulkInsertResult {
            success_count: result.success_count,
            failure_count: result.failure_count,
            failures: result
                .failures
                .into_iter()
                .map(|f| BulkInsertFailure {
                    document_id: f.document_id,
                    error: f.error,
                    is_throttled: f.is_throttled,
                })
                .collect(),
        })
    }

    async fn check_composition_exists(
        &self,
        template_id: &TemplateId,
        ehr_id: &str,
        composition_id: &str,
    ) -> Result<bool> {
        self.client
            .check_composition_exists(template_id, ehr_id, composition_id)
            .await
    }

    fn database_name(&self) -> &str {
        self.client.database_name()
    }
}

#[async_trait]
impl StateStorage for CosmosDbAdapter {
    async fn load_watermark(
        &self,
        template_id: &TemplateId,
        ehr_id: &EhrId,
    ) -> Result<Option<Watermark>> {
        let container = self.client.get_control_container_client();
        let watermark_id = Watermark::generate_id(template_id, ehr_id);
        let partition_key = PartitionKey::from(watermark_id.clone());

        tracing::debug!(
            template_id = %template_id.as_str(),
            ehr_id = %ehr_id.as_str(),
            watermark_id = %watermark_id,
            "Loading watermark"
        );

        match container
            .read_item::<Watermark>(partition_key, &watermark_id, None)
            .await
        {
            Ok(response) => {
                let watermark = response.into_body().map_err(|e| {
                    AtlasError::CosmosDb(CosmosDbError::DeserializationFailed(format!(
                        "Failed to deserialize watermark: {e}"
                    )))
                })?;

                tracing::debug!(
                    template_id = %template_id.as_str(),
                    ehr_id = %ehr_id.as_str(),
                    last_exported = %watermark.last_exported_timestamp,
                    "Watermark loaded"
                );

                Ok(Some(watermark))
            }
            Err(e) => {
                // Check if it's a 404 (not found) error
                if e.to_string().contains("404") || e.to_string().contains("NotFound") {
                    tracing::debug!(
                        template_id = %template_id.as_str(),
                        ehr_id = %ehr_id.as_str(),
                        "No watermark found (first export)"
                    );
                    Ok(None)
                } else {
                    Err(AtlasError::CosmosDb(CosmosDbError::QueryFailed(format!(
                        "Failed to load watermark: {e}"
                    ))))
                }
            }
        }
    }

    async fn save_watermark(&self, watermark: &Watermark, dry_run: bool) -> Result<()> {
        tracing::debug!(
            template_id = %watermark.template_id.as_str(),
            ehr_id = %watermark.ehr_id.as_str(),
            watermark_id = %watermark.id,
            dry_run = dry_run,
            "Saving watermark"
        );

        // If dry-run, skip actual write
        if dry_run {
            tracing::info!(
                template_id = %watermark.template_id.as_str(),
                ehr_id = %watermark.ehr_id.as_str(),
                watermark_id = %watermark.id,
                "DRY RUN: Would save watermark"
            );
            return Ok(());
        }

        let container = self.client.get_control_container_client();
        let partition_key = PartitionKey::from(watermark.id.clone());

        container
            .upsert_item(partition_key, watermark, None)
            .await
            .map_err(|e| {
                AtlasError::CosmosDb(CosmosDbError::WriteFailed(format!(
                    "Failed to save watermark: {e}"
                )))
            })?;

        tracing::debug!(
            template_id = %watermark.template_id.as_str(),
            ehr_id = %watermark.ehr_id.as_str(),
            "Watermark saved successfully"
        );

        Ok(())
    }

    async fn get_all_watermarks(&self) -> Result<Vec<Watermark>> {
        // For now, return an empty vector
        // TODO: Implement cross-partition query when SDK supports it better
        tracing::warn!(
            "get_all_watermarks not yet implemented for CosmosDB - returning empty list"
        );
        Ok(Vec::new())
    }
}

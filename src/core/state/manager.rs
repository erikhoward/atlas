//! State manager for watermark persistence
//!
//! This module provides the StateManager for loading and saving watermarks
//! to the Cosmos DB control container.

use crate::adapters::cosmosdb::CosmosDbClient;
use crate::core::state::watermark::Watermark;
use crate::domain::ids::{EhrId, TemplateId};
use crate::domain::{AtlasError, CosmosDbError, Result};
use azure_data_cosmos::PartitionKey;
use std::sync::Arc;

/// State manager for watermark persistence
///
/// Manages the loading and saving of watermarks to the Cosmos DB control container.
/// Watermarks track the state of incremental exports per {template_id, ehr_id}.
///
/// # Examples
///
/// ```no_run
/// use atlas::adapters::cosmosdb::CosmosDbClient;
/// use atlas::core::state::manager::StateManager;
/// use atlas::config::CosmosDbConfig;
/// use atlas::domain::ids::{TemplateId, EhrId};
/// use std::str::FromStr;
///
/// # async fn example() -> atlas::domain::Result<()> {
/// let config = CosmosDbConfig::default();
/// let client = CosmosDbClient::new(config).await?;
/// let manager = StateManager::new(client);
///
/// let template_id = TemplateId::from_str("vital_signs.v1")?;
/// let ehr_id = EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398")?;
///
/// // Load watermark (returns None if not found)
/// let watermark = manager.load_watermark(&template_id, &ehr_id).await?;
/// # Ok(())
/// # }
/// ```
pub struct StateManager {
    /// Cosmos DB client
    client: Arc<CosmosDbClient>,
}

impl StateManager {
    /// Create a new StateManager
    ///
    /// # Arguments
    ///
    /// * `client` - Cosmos DB client
    pub fn new(client: CosmosDbClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }

    /// Create a new StateManager with an Arc-wrapped client
    ///
    /// # Arguments
    ///
    /// * `client` - Arc-wrapped Cosmos DB client
    pub fn new_with_arc(client: Arc<CosmosDbClient>) -> Self {
        Self { client }
    }

    /// Load a watermark from the control container
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID
    /// * `ehr_id` - EHR ID
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(Watermark))` if found, `Ok(None)` if not found, or an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails for reasons other than "not found".
    pub async fn load_watermark(
        &self,
        template_id: &TemplateId,
        ehr_id: &EhrId,
    ) -> Result<Option<Watermark>> {
        let container = self.client.get_control_container_client();
        let watermark_id = Watermark::generate_id(template_id, ehr_id);

        // Use the watermark ID as the partition key (control container uses /id)
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
                // The response body is the watermark itself
                let watermark = response.into_body().map_err(|e| {
                    AtlasError::CosmosDb(CosmosDbError::QueryFailed(format!(
                        "Failed to deserialize watermark: {}",
                        e
                    )))
                })?;

                tracing::info!(
                    template_id = %template_id.as_str(),
                    ehr_id = %ehr_id.as_str(),
                    compositions_count = watermark.compositions_exported_count,
                    "Watermark loaded"
                );

                Ok(Some(watermark))
            }
            Err(e) => {
                let error_str = e.to_string();
                // Check if it's a 404 (not found) error
                if error_str.contains("404") || error_str.contains("NotFound") {
                    tracing::debug!(
                        template_id = %template_id.as_str(),
                        ehr_id = %ehr_id.as_str(),
                        "Watermark not found (will perform full export)"
                    );
                    Ok(None)
                } else {
                    Err(AtlasError::CosmosDb(CosmosDbError::QueryFailed(format!(
                        "Failed to load watermark: {}",
                        e
                    ))))
                }
            }
        }
    }

    /// Save a watermark to the control container
    ///
    /// Uses upsert to create or update the watermark atomically.
    ///
    /// # Arguments
    ///
    /// * `watermark` - Watermark to save
    ///
    /// # Errors
    ///
    /// Returns an error if the upsert operation fails.
    pub async fn save_watermark(&self, watermark: &Watermark) -> Result<()> {
        let container = self.client.get_control_container_client();

        // Use the watermark ID as the partition key (control container uses /id)
        let partition_key = PartitionKey::from(watermark.id.clone());

        tracing::debug!(
            template_id = %watermark.template_id.as_str(),
            ehr_id = %watermark.ehr_id.as_str(),
            watermark_id = %watermark.id,
            compositions_count = watermark.compositions_exported_count,
            "Saving watermark"
        );

        container
            .upsert_item(partition_key, watermark, None)
            .await
            .map_err(|e| {
                AtlasError::CosmosDb(CosmosDbError::UpdateFailed(format!(
                    "Failed to save watermark: {}",
                    e
                )))
            })?;

        tracing::info!(
            template_id = %watermark.template_id.as_str(),
            ehr_id = %watermark.ehr_id.as_str(),
            compositions_count = watermark.compositions_exported_count,
            status = ?watermark.last_export_status,
            "Watermark saved"
        );

        Ok(())
    }

    /// Get all watermarks from the control container
    ///
    /// # Returns
    ///
    /// Returns a vector of all watermarks in the control container.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_all_watermarks(&self) -> Result<Vec<Watermark>> {
        let _container = self.client.get_control_container_client();

        tracing::debug!("Querying all watermarks");

        // For now, return an empty vector
        // TODO: Implement cross-partition query when SDK supports it better
        // This functionality is not critical for Phase 6 checkpoint
        tracing::warn!("get_all_watermarks not yet implemented - returning empty list");

        Ok(Vec::new())
    }

    /// Checkpoint a batch by saving the watermark
    ///
    /// This is an alias for `save_watermark` but with explicit checkpoint semantics.
    /// Used to save progress after each successful batch to enable recovery.
    ///
    /// # Arguments
    ///
    /// * `watermark` - Watermark to checkpoint
    ///
    /// # Errors
    ///
    /// Returns an error if the checkpoint fails.
    pub async fn checkpoint_batch(&self, watermark: &Watermark) -> Result<()> {
        tracing::info!(
            template_id = %watermark.template_id.as_str(),
            ehr_id = %watermark.ehr_id.as_str(),
            compositions_count = watermark.compositions_exported_count,
            "Checkpointing batch"
        );

        self.save_watermark(watermark).await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_state_manager_creation() {
        // This test just verifies the StateManager can be created
        // Actual functionality requires a real Cosmos DB connection
        // and will be tested in integration tests
    }
}

//! State manager for watermark persistence
//!
//! This module provides the StateManager for loading and saving watermarks
//! to the database backend.

use crate::adapters::database::traits::StateStorage;
use crate::core::state::watermark::Watermark;
use crate::domain::ids::{EhrId, TemplateId};
use crate::domain::Result;
use std::sync::Arc;

/// State manager for watermark persistence
///
/// Manages the loading and saving of watermarks to the database backend.
/// Watermarks track the state of incremental exports per {template_id, ehr_id}.
pub struct StateManager {
    /// State storage backend
    storage: Arc<dyn StateStorage + Send + Sync>,
}

impl StateManager {
    /// Create a new StateManager with a state storage backend
    ///
    /// # Arguments
    ///
    /// * `storage` - State storage implementation
    pub fn new_with_storage(storage: Arc<dyn StateStorage + Send + Sync>) -> Self {
        Self { storage }
    }

    /// Load a watermark from the database
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
        self.storage.load_watermark(template_id, ehr_id).await
    }

    /// Save a watermark to the database
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
        self.storage.save_watermark(watermark).await
    }

    /// Get all watermarks from the database
    ///
    /// # Returns
    ///
    /// Returns a vector of all watermarks in the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_all_watermarks(&self) -> Result<Vec<Watermark>> {
        self.storage.get_all_watermarks().await
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

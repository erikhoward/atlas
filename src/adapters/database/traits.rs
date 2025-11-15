//! Database abstraction traits
//!
//! This module defines the traits that database adapters must implement
//! to work with Atlas.

use crate::core::state::watermark::Watermark;
use crate::domain::composition::Composition;
use crate::domain::ids::{EhrId, TemplateId};
use crate::domain::Result;
use async_trait::async_trait;
use std::any::Any;

/// Result of a bulk insert operation
#[derive(Debug, Clone)]
pub struct BulkInsertResult {
    /// Number of items successfully inserted
    pub success_count: usize,

    /// Number of items that failed to insert
    pub failure_count: usize,

    /// Details of failed items
    pub failures: Vec<BulkInsertFailure>,
}

/// Details of a failed bulk insert item
#[derive(Debug, Clone)]
pub struct BulkInsertFailure {
    /// Document/row ID that failed
    pub document_id: String,

    /// Error message
    pub error: String,

    /// Whether the failure was due to throttling
    pub is_throttled: bool,
}

/// Database client trait for composition storage
///
/// This trait defines the interface that all database adapters must implement
/// for storing and managing OpenEHR compositions.
#[async_trait]
pub trait DatabaseClient: Send + Sync {
    /// Downcast to Any for type-specific operations
    ///
    /// This allows downcasting the trait object to concrete types when needed
    /// (e.g., for verification operations that require the underlying client).
    fn as_any(&self) -> &dyn Any;

    /// Test the database connection
    ///
    /// # Errors
    ///
    /// Returns an error if the connection test fails.
    async fn test_connection(&self) -> Result<()>;

    /// Ensure the database exists, creating it if necessary
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be created or accessed.
    async fn ensure_database_exists(&self) -> Result<()>;

    /// Ensure a container/table exists for a specific template
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID to create storage for
    ///
    /// # Errors
    ///
    /// Returns an error if the container/table cannot be created.
    async fn ensure_container_exists(&self, template_id: &TemplateId) -> Result<()>;

    /// Ensure the control container/table exists for state management
    ///
    /// # Errors
    ///
    /// Returns an error if the control container/table cannot be created.
    async fn ensure_control_container_exists(&self) -> Result<()>;

    /// Bulk insert pre-transformed JSON documents
    ///
    /// This is the preferred method for inserting compositions as it allows
    /// anonymization to be applied to the transformed JSON before database insertion.
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID for the compositions
    /// * `documents` - Pre-transformed JSON documents to insert
    /// * `max_retries` - Maximum number of retries for transient failures
    /// * `dry_run` - If true, skip actual database writes (for testing)
    ///
    /// # Returns
    ///
    /// Returns a `BulkInsertResult` with success/failure counts.
    async fn bulk_insert_json(
        &self,
        template_id: &TemplateId,
        documents: Vec<serde_json::Value>,
        max_retries: usize,
        dry_run: bool,
    ) -> Result<BulkInsertResult>;

    /// Bulk insert compositions in preserved format
    ///
    /// # Deprecated
    ///
    /// This method performs transformation internally, preventing anonymization
    /// from being applied between transformation and database insertion.
    /// Use `bulk_insert_json` with pre-transformed compositions instead.
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID for the compositions
    /// * `compositions` - Compositions to insert
    /// * `export_mode` - Export mode (full or incremental)
    /// * `max_retries` - Maximum number of retries for transient failures
    /// * `dry_run` - If true, skip actual database writes (for testing)
    ///
    /// # Returns
    ///
    /// Returns a `BulkInsertResult` with success/failure counts.
    async fn bulk_insert_compositions(
        &self,
        template_id: &TemplateId,
        compositions: Vec<Composition>,
        export_mode: String,
        max_retries: usize,
        dry_run: bool,
    ) -> Result<BulkInsertResult>;

    /// Bulk insert compositions in flattened format
    ///
    /// # Deprecated
    ///
    /// This method performs transformation internally, preventing anonymization
    /// from being applied between transformation and database insertion.
    /// Use `bulk_insert_json` with pre-transformed compositions instead.
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID for the compositions
    /// * `compositions` - Compositions to insert (will be flattened)
    /// * `export_mode` - Export mode (full or incremental)
    /// * `max_retries` - Maximum number of retries for transient failures
    /// * `dry_run` - If true, skip actual database writes (for testing)
    ///
    /// # Returns
    ///
    /// Returns a `BulkInsertResult` with success/failure counts.
    async fn bulk_insert_compositions_flattened(
        &self,
        template_id: &TemplateId,
        compositions: Vec<Composition>,
        export_mode: String,
        max_retries: usize,
        dry_run: bool,
    ) -> Result<BulkInsertResult>;

    /// Check if a composition exists
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID
    /// * `ehr_id` - EHR ID
    /// * `composition_id` - Composition ID
    ///
    /// # Returns
    ///
    /// Returns `true` if the composition exists, `false` otherwise.
    async fn check_composition_exists(
        &self,
        template_id: &TemplateId,
        ehr_id: &str,
        composition_id: &str,
    ) -> Result<bool>;

    /// Get the database name
    fn database_name(&self) -> &str;
}

/// State storage trait for watermark persistence
///
/// This trait defines the interface for storing and retrieving watermarks
/// that track the state of incremental exports.
#[async_trait]
pub trait StateStorage: Send + Sync {
    /// Load a watermark from storage
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID
    /// * `ehr_id` - EHR ID
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(Watermark))` if found, `Ok(None)` if not found.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails for reasons other than "not found".
    async fn load_watermark(
        &self,
        template_id: &TemplateId,
        ehr_id: &EhrId,
    ) -> Result<Option<Watermark>>;

    /// Save a watermark to storage
    ///
    /// # Arguments
    ///
    /// * `watermark` - Watermark to save
    /// * `dry_run` - If true, skip actual database writes (for testing)
    ///
    /// # Errors
    ///
    /// Returns an error if the save operation fails.
    async fn save_watermark(&self, watermark: &Watermark, dry_run: bool) -> Result<()>;

    /// Checkpoint a batch by saving the watermark
    ///
    /// This is an alias for `save_watermark` but with explicit checkpoint semantics.
    ///
    /// # Arguments
    ///
    /// * `watermark` - Watermark to checkpoint
    /// * `dry_run` - If true, skip actual database writes (for testing)
    ///
    /// # Errors
    ///
    /// Returns an error if the checkpoint fails.
    async fn checkpoint_batch(&self, watermark: &Watermark, dry_run: bool) -> Result<()> {
        tracing::info!(
            template_id = %watermark.template_id.as_str(),
            ehr_id = %watermark.ehr_id.as_str(),
            compositions_count = watermark.compositions_exported_count,
            "Checkpointing batch"
        );

        self.save_watermark(watermark, dry_run).await
    }

    /// Get all watermarks from storage
    ///
    /// # Returns
    ///
    /// Returns a vector of all watermarks.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    async fn get_all_watermarks(&self) -> Result<Vec<Watermark>>;
}

//! PostgreSQL adapter implementing database traits
//!
//! This module provides the implementation of DatabaseClient and StateStorage traits
//! for PostgreSQL.

use crate::adapters::database::traits::{
    BulkInsertFailure, BulkInsertResult, DatabaseClient, StateStorage,
};
use crate::adapters::postgresql::client::PostgreSQLClient;
use crate::adapters::postgresql::models::{PostgreSQLComposition, PostgreSQLWatermark};
use crate::core::state::watermark::Watermark;
use crate::domain::composition::Composition;
use crate::domain::ids::{EhrId, TemplateId};
use crate::domain::{AtlasError, Result};
use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;

/// PostgreSQL implementation of database traits
///
/// This wraps the PostgreSQLClient and implements the DatabaseClient and StateStorage traits.
pub struct PostgreSQLAdapter {
    client: Arc<PostgreSQLClient>,
}

impl PostgreSQLAdapter {
    /// Create a new PostgreSQL adapter
    pub fn new(client: PostgreSQLClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }

    /// Create a new PostgreSQL adapter with an Arc-wrapped client
    pub fn new_with_arc(client: Arc<PostgreSQLClient>) -> Self {
        Self { client }
    }

    /// Get a reference to the underlying client
    pub fn client(&self) -> &Arc<PostgreSQLClient> {
        &self.client
    }
}

#[async_trait]
impl DatabaseClient for PostgreSQLAdapter {
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
        // PostgreSQL uses a single table for all compositions
        self.client.ensure_table_exists(template_id.as_str()).await
    }

    async fn ensure_control_container_exists(&self) -> Result<()> {
        self.client.ensure_watermarks_table_exists().await
    }

    async fn bulk_insert_json(
        &self,
        _template_id: &TemplateId,
        documents: Vec<serde_json::Value>,
        _max_retries: usize,
        dry_run: bool,
    ) -> Result<BulkInsertResult> {
        // If dry-run, skip actual write and return success
        if dry_run {
            tracing::info!(
                count = documents.len(),
                "DRY RUN: Would insert {} compositions into PostgreSQL",
                documents.len()
            );
            return Ok(BulkInsertResult {
                success_count: documents.len(),
                failure_count: 0,
                failures: Vec::new(),
            });
        }

        let mut success_count = 0;
        let mut failures = Vec::new();

        // Determine format from first document (check if it has "fields" key for flattened format)
        let is_flattened = documents
            .first()
            .and_then(|doc| doc.get("fields"))
            .is_some();

        for doc in documents {
            let doc_id = doc
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            // Convert JSON to PostgreSQLComposition
            let pg_comp = if is_flattened {
                match PostgreSQLComposition::from_json_flattened(doc) {
                    Ok(comp) => comp,
                    Err(e) => {
                        failures.push(BulkInsertFailure {
                            document_id: doc_id,
                            error: format!("Failed to convert composition: {e}"),
                            is_throttled: false,
                        });
                        continue;
                    }
                }
            } else {
                match PostgreSQLComposition::from_json_preserved(doc) {
                    Ok(comp) => comp,
                    Err(e) => {
                        failures.push(BulkInsertFailure {
                            document_id: doc_id,
                            error: format!("Failed to convert composition: {e}"),
                            is_throttled: false,
                        });
                        continue;
                    }
                }
            };

            // Insert into database
            let insert_query = r#"
                INSERT INTO compositions (
                    id, ehr_id, composition_uid, template_id, time_committed,
                    content, export_mode, exported_at, atlas_version, checksum
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                ON CONFLICT (id) DO UPDATE SET
                    time_committed = EXCLUDED.time_committed,
                    content = EXCLUDED.content,
                    exported_at = EXCLUDED.exported_at,
                    checksum = EXCLUDED.checksum
            "#;

            // Convert content to serde_json::Value for ToSql
            let content_json = serde_json::to_value(&pg_comp.content)
                .map_err(|e| AtlasError::Serialization(e.to_string()))?;

            match self
                .client
                .execute(
                    insert_query,
                    &[
                        &pg_comp.id,
                        &pg_comp.ehr_id,
                        &pg_comp.composition_uid,
                        &pg_comp.template_id,
                        &pg_comp.time_committed,
                        &content_json,
                        &pg_comp.export_mode,
                        &pg_comp.exported_at,
                        &pg_comp.atlas_version,
                        &pg_comp.checksum,
                    ],
                )
                .await
            {
                Ok(_) => {
                    success_count += 1;
                }
                Err(e) => {
                    failures.push(BulkInsertFailure {
                        document_id: doc_id,
                        error: format!("Database insert failed: {e}"),
                        is_throttled: false,
                    });
                }
            }
        }

        Ok(BulkInsertResult {
            success_count,
            failure_count: failures.len(),
            failures,
        })
    }

    async fn bulk_insert_compositions(
        &self,
        _template_id: &TemplateId,
        compositions: Vec<Composition>,
        export_mode: String,
        _max_retries: usize,
        dry_run: bool,
    ) -> Result<BulkInsertResult> {
        // If dry-run, skip actual write and return success
        if dry_run {
            tracing::info!(
                count = compositions.len(),
                "DRY RUN: Would insert {} compositions (preserved format) into PostgreSQL",
                compositions.len()
            );
            return Ok(BulkInsertResult {
                success_count: compositions.len(),
                failure_count: 0,
                failures: Vec::new(),
            });
        }

        let mut success_count = 0;
        let mut failures = Vec::new();

        for composition in compositions {
            let doc_id = composition.uid.to_string();

            // Convert to PostgreSQL document
            let pg_comp = match PostgreSQLComposition::from_domain_preserved(
                composition,
                export_mode.clone(),
            ) {
                Ok(comp) => comp,
                Err(e) => {
                    failures.push(BulkInsertFailure {
                        document_id: doc_id,
                        error: format!("Failed to convert composition: {e}"),
                        is_throttled: false,
                    });
                    continue;
                }
            };

            // Insert into database
            let insert_query = r#"
                INSERT INTO compositions (
                    id, ehr_id, composition_uid, template_id, time_committed,
                    content, export_mode, exported_at, atlas_version, checksum
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                ON CONFLICT (id) DO UPDATE SET
                    time_committed = EXCLUDED.time_committed,
                    content = EXCLUDED.content,
                    exported_at = EXCLUDED.exported_at
            "#;

            let content_json = serde_json::to_value(&pg_comp.content).map_err(|e| {
                AtlasError::Serialization(format!("Failed to serialize content: {e}"))
            })?;

            match self
                .client
                .execute(
                    insert_query,
                    &[
                        &pg_comp.id,
                        &pg_comp.ehr_id,
                        &pg_comp.composition_uid,
                        &pg_comp.template_id,
                        &pg_comp.time_committed,
                        &content_json,
                        &pg_comp.export_mode,
                        &pg_comp.exported_at,
                        &pg_comp.atlas_version,
                        &pg_comp.checksum,
                    ],
                )
                .await
            {
                Ok(_) => {
                    success_count += 1;
                }
                Err(e) => {
                    tracing::error!(
                        composition_id = %doc_id,
                        error = %e,
                        "Failed to insert composition into PostgreSQL"
                    );
                    failures.push(BulkInsertFailure {
                        document_id: doc_id,
                        error: e.to_string(),
                        is_throttled: false,
                    });
                }
            }
        }

        Ok(BulkInsertResult {
            success_count,
            failure_count: failures.len(),
            failures,
        })
    }

    async fn bulk_insert_compositions_flattened(
        &self,
        _template_id: &TemplateId,
        compositions: Vec<Composition>,
        export_mode: String,
        _max_retries: usize,
        dry_run: bool,
    ) -> Result<BulkInsertResult> {
        // If dry-run, skip actual write and return success
        if dry_run {
            tracing::info!(
                count = compositions.len(),
                "DRY RUN: Would insert {} compositions (flattened format) into PostgreSQL",
                compositions.len()
            );
            return Ok(BulkInsertResult {
                success_count: compositions.len(),
                failure_count: 0,
                failures: Vec::new(),
            });
        }

        let mut success_count = 0;
        let mut failures = Vec::new();

        for composition in compositions {
            let doc_id = composition.uid.to_string();

            // Convert to flattened PostgreSQL document
            let pg_comp = match PostgreSQLComposition::from_domain_flattened(
                composition,
                export_mode.clone(),
            ) {
                Ok(comp) => comp,
                Err(e) => {
                    failures.push(BulkInsertFailure {
                        document_id: doc_id,
                        error: format!("Failed to convert composition: {e}"),
                        is_throttled: false,
                    });
                    continue;
                }
            };

            // Insert into database
            let insert_query = r#"
                INSERT INTO compositions (
                    id, ehr_id, composition_uid, template_id, time_committed,
                    content, export_mode, exported_at, atlas_version, checksum
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                ON CONFLICT (id) DO UPDATE SET
                    time_committed = EXCLUDED.time_committed,
                    content = EXCLUDED.content,
                    exported_at = EXCLUDED.exported_at
            "#;

            let content_json = serde_json::to_value(&pg_comp.content).map_err(|e| {
                AtlasError::Serialization(format!("Failed to serialize content: {e}"))
            })?;

            match self
                .client
                .execute(
                    insert_query,
                    &[
                        &pg_comp.id,
                        &pg_comp.ehr_id,
                        &pg_comp.composition_uid,
                        &pg_comp.template_id,
                        &pg_comp.time_committed,
                        &content_json,
                        &pg_comp.export_mode,
                        &pg_comp.exported_at,
                        &pg_comp.atlas_version,
                        &pg_comp.checksum,
                    ],
                )
                .await
            {
                Ok(_) => {
                    success_count += 1;
                }
                Err(e) => {
                    tracing::error!(
                        composition_id = %doc_id,
                        error = %e,
                        "Failed to insert composition into PostgreSQL (flattened)"
                    );
                    failures.push(BulkInsertFailure {
                        document_id: doc_id,
                        error: e.to_string(),
                        is_throttled: false,
                    });
                }
            }
        }

        Ok(BulkInsertResult {
            success_count,
            failure_count: failures.len(),
            failures,
        })
    }

    async fn check_composition_exists(
        &self,
        _template_id: &TemplateId,
        ehr_id: &str,
        composition_id: &str,
    ) -> Result<bool> {
        let query = "SELECT EXISTS(SELECT 1 FROM compositions WHERE id = $1 AND ehr_id = $2)";

        let rows = self
            .client
            .query(query, &[&composition_id, &ehr_id])
            .await?;

        if let Some(row) = rows.first() {
            let exists: bool = row.get(0);
            Ok(exists)
        } else {
            Ok(false)
        }
    }

    fn database_name(&self) -> &str {
        "postgresql"
    }
}

#[async_trait]
impl StateStorage for PostgreSQLAdapter {
    async fn load_watermark(
        &self,
        template_id: &TemplateId,
        ehr_id: &EhrId,
    ) -> Result<Option<Watermark>> {
        let watermark_id = Watermark::generate_id(template_id, ehr_id);

        tracing::debug!(
            template_id = %template_id.as_str(),
            ehr_id = %ehr_id.as_str(),
            watermark_id = %watermark_id,
            "Loading watermark from PostgreSQL"
        );

        let query = "SELECT * FROM watermarks WHERE id = $1";

        let rows = self.client.query(query, &[&watermark_id]).await?;

        if let Some(row) = rows.first() {
            let pg_watermark = PostgreSQLWatermark {
                id: row.get("id"),
                template_id: row.get("template_id"),
                ehr_id: row.get("ehr_id"),
                last_exported_timestamp: row.get("last_exported_timestamp"),
                last_exported_composition_uid: row.get("last_exported_composition_uid"),
                compositions_exported_count: row.get("compositions_exported_count"),
                last_export_started_at: row.get("last_export_started_at"),
                last_export_completed_at: row.get("last_export_completed_at"),
                last_export_status: row.get("last_export_status"),
            };

            let watermark = pg_watermark.to_domain()?;

            tracing::debug!(
                template_id = %template_id.as_str(),
                ehr_id = %ehr_id.as_str(),
                last_exported = %watermark.last_exported_timestamp,
                "Watermark loaded from PostgreSQL"
            );

            Ok(Some(watermark))
        } else {
            tracing::debug!(
                template_id = %template_id.as_str(),
                ehr_id = %ehr_id.as_str(),
                "No watermark found in PostgreSQL (first export)"
            );
            Ok(None)
        }
    }

    async fn save_watermark(&self, watermark: &Watermark, dry_run: bool) -> Result<()> {
        tracing::debug!(
            template_id = %watermark.template_id.as_str(),
            ehr_id = %watermark.ehr_id.as_str(),
            watermark_id = %watermark.id,
            dry_run = dry_run,
            "Saving watermark to PostgreSQL"
        );

        // If dry-run, skip actual write
        if dry_run {
            tracing::info!(
                template_id = %watermark.template_id.as_str(),
                ehr_id = %watermark.ehr_id.as_str(),
                watermark_id = %watermark.id,
                "DRY RUN: Would save watermark to PostgreSQL"
            );
            return Ok(());
        }

        let pg_watermark = PostgreSQLWatermark::from_domain(watermark);

        let upsert_query = r#"
            INSERT INTO watermarks (
                id, template_id, ehr_id, last_exported_timestamp,
                last_exported_composition_uid, compositions_exported_count,
                last_export_started_at, last_export_completed_at, last_export_status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO UPDATE SET
                last_exported_timestamp = EXCLUDED.last_exported_timestamp,
                last_exported_composition_uid = EXCLUDED.last_exported_composition_uid,
                compositions_exported_count = EXCLUDED.compositions_exported_count,
                last_export_started_at = EXCLUDED.last_export_started_at,
                last_export_completed_at = EXCLUDED.last_export_completed_at,
                last_export_status = EXCLUDED.last_export_status
        "#;

        self.client
            .execute(
                upsert_query,
                &[
                    &pg_watermark.id,
                    &pg_watermark.template_id,
                    &pg_watermark.ehr_id,
                    &pg_watermark.last_exported_timestamp,
                    &pg_watermark.last_exported_composition_uid,
                    &pg_watermark.compositions_exported_count,
                    &pg_watermark.last_export_started_at,
                    &pg_watermark.last_export_completed_at,
                    &pg_watermark.last_export_status,
                ],
            )
            .await?;

        tracing::debug!(
            template_id = %watermark.template_id.as_str(),
            ehr_id = %watermark.ehr_id.as_str(),
            "Watermark saved to PostgreSQL successfully"
        );

        Ok(())
    }

    async fn get_all_watermarks(&self) -> Result<Vec<Watermark>> {
        tracing::debug!("Querying all watermarks from PostgreSQL");

        let query = "SELECT * FROM watermarks ORDER BY template_id, ehr_id";

        let rows = self.client.query(query, &[]).await?;

        let mut watermarks = Vec::new();

        for row in rows {
            let pg_watermark = PostgreSQLWatermark {
                id: row.get("id"),
                template_id: row.get("template_id"),
                ehr_id: row.get("ehr_id"),
                last_exported_timestamp: row.get("last_exported_timestamp"),
                last_exported_composition_uid: row.get("last_exported_composition_uid"),
                compositions_exported_count: row.get("compositions_exported_count"),
                last_export_started_at: row.get("last_export_started_at"),
                last_export_completed_at: row.get("last_export_completed_at"),
                last_export_status: row.get("last_export_status"),
            };

            watermarks.push(pg_watermark.to_domain()?);
        }

        tracing::debug!(
            count = watermarks.len(),
            "Loaded watermarks from PostgreSQL"
        );

        Ok(watermarks)
    }
}

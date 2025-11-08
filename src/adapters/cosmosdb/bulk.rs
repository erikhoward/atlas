//! Bulk operations for Cosmos DB
//!
//! This module provides batch insert functionality with retry logic
//! for handling throttling errors.

use crate::adapters::cosmosdb::models::{CosmosComposition, CosmosCompositionFlattened};
use crate::domain::{AtlasError, CosmosDbError, Result};
use azure_data_cosmos::clients::ContainerClient;
use azure_data_cosmos::PartitionKey;
use serde::Serialize;
use std::time::Duration;
use tokio::time::sleep;

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
    /// Document ID that failed
    pub document_id: String,

    /// Error message
    pub error: String,

    /// Whether the failure was due to throttling (429)
    pub is_throttled: bool,
}

/// Bulk insert compositions into Cosmos DB
///
/// This function inserts multiple compositions in batch, with retry logic
/// for handling throttling errors (429).
///
/// # Arguments
///
/// * `container` - Container client to insert into
/// * `compositions` - Compositions to insert
/// * `max_retries` - Maximum number of retries for throttled requests
///
/// # Returns
///
/// Returns a `BulkInsertResult` with success/failure counts and details.
pub async fn bulk_insert_compositions(
    container: &ContainerClient,
    compositions: Vec<CosmosComposition>,
    max_retries: usize,
) -> Result<BulkInsertResult> {
    let mut success_count = 0;
    let mut failures = Vec::new();

    for composition in compositions {
        let ehr_id = composition.ehr_id.clone();
        let document_id = composition.id.clone();
        let partition_key = PartitionKey::from(ehr_id);

        match insert_with_retry(container, partition_key, composition, max_retries).await {
            Ok(_) => {
                success_count += 1;
            }
            Err(e) => {
                let is_throttled = e.to_string().contains("429")
                    || e.to_string().contains("TooManyRequests")
                    || e.to_string().contains("Request rate is large");

                failures.push(BulkInsertFailure {
                    document_id,
                    error: e.to_string(),
                    is_throttled,
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

/// Bulk insert flattened compositions into Cosmos DB
///
/// This function inserts multiple flattened compositions in batch, with retry logic
/// for handling throttling errors (429).
///
/// # Arguments
///
/// * `container` - Container client to insert into
/// * `compositions` - Flattened compositions to insert
/// * `max_retries` - Maximum number of retries for throttled requests
///
/// # Returns
///
/// Returns a `BulkInsertResult` with success/failure counts and details.
pub async fn bulk_insert_compositions_flattened(
    container: &ContainerClient,
    compositions: Vec<CosmosCompositionFlattened>,
    max_retries: usize,
) -> Result<BulkInsertResult> {
    let mut success_count = 0;
    let mut failures = Vec::new();

    for composition in compositions {
        let ehr_id = composition.ehr_id.clone();
        let document_id = composition.id.clone();
        let partition_key = PartitionKey::from(ehr_id);

        match insert_with_retry(container, partition_key, composition, max_retries).await {
            Ok(_) => {
                success_count += 1;
            }
            Err(e) => {
                let is_throttled = e.to_string().contains("429")
                    || e.to_string().contains("TooManyRequests")
                    || e.to_string().contains("Request rate is large");

                failures.push(BulkInsertFailure {
                    document_id,
                    error: e.to_string(),
                    is_throttled,
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

/// Insert a document with exponential backoff retry for throttling errors
async fn insert_with_retry<T: Serialize + Clone>(
    container: &ContainerClient,
    partition_key: PartitionKey,
    document: T,
    max_retries: usize,
) -> Result<()> {
    let mut retry_count = 0;
    let mut delay_ms = 1000; // Start with 1 second

    loop {
        match container
            .create_item(partition_key.clone(), document.clone(), None)
            .await
        {
            Ok(_) => return Ok(()),
            Err(e) => {
                let is_throttled = e.to_string().contains("429")
                    || e.to_string().contains("TooManyRequests")
                    || e.to_string().contains("Request rate is large");

                if is_throttled && retry_count < max_retries {
                    tracing::warn!(
                        retry_count = retry_count,
                        delay_ms = delay_ms,
                        "Throttled by Cosmos DB, retrying after delay"
                    );

                    sleep(Duration::from_millis(delay_ms)).await;

                    retry_count += 1;
                    delay_ms *= 2; // Exponential backoff
                    delay_ms = delay_ms.min(30000); // Cap at 30 seconds
                } else {
                    return Err(AtlasError::CosmosDb(CosmosDbError::InsertFailed(format!(
                        "Failed to insert document after {retry_count} retries: {e}"
                    ))));
                }
            }
        }
    }
}

/// Upsert a composition into Cosmos DB
///
/// This function upserts a composition, creating it if it doesn't exist
/// or replacing it if it does.
///
/// # Arguments
///
/// * `container` - Container client to upsert into
/// * `composition` - Composition to upsert
/// * `max_retries` - Maximum number of retries for throttled requests
pub async fn upsert_composition(
    container: &ContainerClient,
    composition: CosmosComposition,
    max_retries: usize,
) -> Result<()> {
    let ehr_id = composition.ehr_id.clone();
    let partition_key = PartitionKey::from(ehr_id);

    upsert_with_retry(container, partition_key, composition, max_retries).await
}

/// Upsert a flattened composition into Cosmos DB
///
/// This function upserts a flattened composition, creating it if it doesn't exist
/// or replacing it if it does.
///
/// # Arguments
///
/// * `container` - Container client to upsert into
/// * `composition` - Flattened composition to upsert
/// * `max_retries` - Maximum number of retries for throttled requests
pub async fn upsert_composition_flattened(
    container: &ContainerClient,
    composition: CosmosCompositionFlattened,
    max_retries: usize,
) -> Result<()> {
    let ehr_id = composition.ehr_id.clone();
    let partition_key = PartitionKey::from(ehr_id);

    upsert_with_retry(container, partition_key, composition, max_retries).await
}

/// Upsert a document with exponential backoff retry for throttling errors
async fn upsert_with_retry<T: Serialize + Clone>(
    container: &ContainerClient,
    partition_key: PartitionKey,
    document: T,
    max_retries: usize,
) -> Result<()> {
    let mut retry_count = 0;
    let mut delay_ms = 1000; // Start with 1 second

    loop {
        match container
            .upsert_item(partition_key.clone(), document.clone(), None)
            .await
        {
            Ok(_) => return Ok(()),
            Err(e) => {
                let is_throttled = e.to_string().contains("429")
                    || e.to_string().contains("TooManyRequests")
                    || e.to_string().contains("Request rate is large");

                if is_throttled && retry_count < max_retries {
                    tracing::warn!(
                        retry_count = retry_count,
                        delay_ms = delay_ms,
                        "Throttled by Cosmos DB, retrying after delay"
                    );

                    sleep(Duration::from_millis(delay_ms)).await;

                    retry_count += 1;
                    delay_ms *= 2; // Exponential backoff
                    delay_ms = delay_ms.min(30000); // Cap at 30 seconds
                } else {
                    return Err(AtlasError::CosmosDb(CosmosDbError::UpdateFailed(format!(
                        "Failed to upsert document after {retry_count} retries: {e}"
                    ))));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bulk_insert_result_creation() {
        let result = BulkInsertResult {
            success_count: 10,
            failure_count: 2,
            failures: vec![
                BulkInsertFailure {
                    document_id: "doc1".to_string(),
                    error: "Error 1".to_string(),
                    is_throttled: true,
                },
                BulkInsertFailure {
                    document_id: "doc2".to_string(),
                    error: "Error 2".to_string(),
                    is_throttled: false,
                },
            ],
        };

        assert_eq!(result.success_count, 10);
        assert_eq!(result.failure_count, 2);
        assert_eq!(result.failures.len(), 2);
        assert!(result.failures[0].is_throttled);
        assert!(!result.failures[1].is_throttled);
    }
}

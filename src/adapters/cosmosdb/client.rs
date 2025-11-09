//! Cosmos DB client implementation
//!
//! This module provides the client for interacting with Azure Cosmos DB.

use crate::config::CosmosDbConfig;
use crate::domain::ids::{CompositionUid, EhrId, TemplateId};
use crate::domain::{AtlasError, CosmosDbError, Result};
use azure_core::credentials::Secret;
use azure_data_cosmos::clients::{ContainerClient, DatabaseClient};
use azure_data_cosmos::models::{ContainerProperties, IndexingPolicy, PartitionKeyDefinition};
use azure_data_cosmos::{CosmosClient, CosmosClientOptions, PartitionKey};
use futures::stream::StreamExt;
use serde_json::Value;
use std::borrow::Cow;

/// Cosmos DB client for Atlas
///
/// Provides methods for connecting to Azure Cosmos DB, managing containers,
/// and performing document operations.
pub struct CosmosDbClient {
    /// Cosmos DB client
    client: CosmosClient,

    /// Database client
    database: DatabaseClient,

    /// Configuration
    config: CosmosDbConfig,
}

impl CosmosDbClient {
    /// Create a new Cosmos DB client
    ///
    /// # Arguments
    ///
    /// * `config` - Cosmos DB configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the client cannot be created or the connection fails.
    pub async fn new(config: CosmosDbConfig) -> Result<Self> {
        use secrecy::ExposeSecret;

        // Create Cosmos client with key authentication
        // Convert our SecretString to Azure's Secret type
        let key_str: String = config.key.expose_secret().clone().into();
        let key = Secret::new(key_str);
        let options = Some(CosmosClientOptions::default());

        let client = CosmosClient::with_key(&config.endpoint, key, options).map_err(|e| {
            AtlasError::CosmosDb(CosmosDbError::ConnectionFailed(format!(
                "Failed to create Cosmos client: {e}"
            )))
        })?;

        let database = client.database_client(&config.database_name);

        Ok(Self {
            client,
            database,
            config,
        })
    }

    /// Test the connection to Cosmos DB
    ///
    /// Attempts to read the database to verify connectivity.
    pub async fn test_connection(&self) -> Result<()> {
        self.database.read(None).await.map_err(|e| {
            AtlasError::CosmosDb(CosmosDbError::ConnectionFailed(format!(
                "Connection test failed: {e}"
            )))
        })?;

        Ok(())
    }

    /// Ensure the database exists, creating it if necessary
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be created.
    pub async fn ensure_database_exists(&self) -> Result<()> {
        // Try to read the database first
        match self.database.read(None).await {
            Ok(_) => {
                tracing::info!(database = %self.config.database_name, "Database already exists");
                Ok(())
            }
            Err(_) => {
                // Database doesn't exist, create it
                tracing::info!(database = %self.config.database_name, "Creating database");

                self.client
                    .create_database(&self.config.database_name, None)
                    .await
                    .map_err(|e| {
                        AtlasError::CosmosDb(CosmosDbError::DatabaseCreationFailed(format!(
                            "Failed to create database: {e}"
                        )))
                    })?;

                tracing::info!(database = %self.config.database_name, "Database created successfully");
                Ok(())
            }
        }
    }

    /// Ensure a container exists for a template, creating it if necessary
    ///
    /// Container name format: `{prefix}_{template_id}`
    /// Partition key: `/ehr_id`
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID to create container for
    ///
    /// # Errors
    ///
    /// Returns an error if the container cannot be created.
    pub async fn ensure_container_exists(&self, template_id: &TemplateId) -> Result<()> {
        let container_name = self.get_container_name(template_id);
        let container = self.database.container_client(&container_name);

        // Try to read the container first
        match container.read(None).await {
            Ok(_) => {
                tracing::info!(container = %container_name, "Container already exists");
                Ok(())
            }
            Err(_) => {
                // Container doesn't exist, create it
                tracing::info!(container = %container_name, "Creating container");

                let partition_key_def = PartitionKeyDefinition {
                    paths: vec!["/ehr_id".to_string()],
                    kind: azure_data_cosmos::models::PartitionKeyKind::Hash,
                    version: None,
                };

                let properties = ContainerProperties {
                    id: Cow::Owned(container_name.clone()),
                    partition_key: partition_key_def,
                    indexing_policy: Some(IndexingPolicy::default()),
                    ..Default::default()
                };

                self.database
                    .create_container(properties, None)
                    .await
                    .map_err(|e| {
                        AtlasError::CosmosDb(CosmosDbError::ContainerCreationFailed(format!(
                            "Failed to create container {container_name}: {e}"
                        )))
                    })?;

                tracing::info!(container = %container_name, "Container created successfully");
                Ok(())
            }
        }
    }

    /// Ensure the control container exists for state management
    ///
    /// The control container stores watermarks and other state information.
    pub async fn ensure_control_container_exists(&self) -> Result<()> {
        let container_name = &self.config.control_container;
        let container = self.database.container_client(container_name);

        // Try to read the container first
        match container.read(None).await {
            Ok(_) => {
                tracing::info!(container = %container_name, "Control container already exists");
                Ok(())
            }
            Err(_) => {
                // Container doesn't exist, create it
                tracing::info!(container = %container_name, "Creating control container");

                let partition_key_def = PartitionKeyDefinition {
                    paths: vec!["/id".to_string()],
                    kind: azure_data_cosmos::models::PartitionKeyKind::Hash,
                    version: None,
                };

                let properties = ContainerProperties {
                    id: Cow::Owned(container_name.clone()),
                    partition_key: partition_key_def,
                    indexing_policy: Some(IndexingPolicy::default()),
                    ..Default::default()
                };

                self.database
                    .create_container(properties, None)
                    .await
                    .map_err(|e| {
                        AtlasError::CosmosDb(CosmosDbError::ContainerCreationFailed(format!(
                            "Failed to create control container {container_name}: {e}"
                        )))
                    })?;

                tracing::info!(container = %container_name, "Control container created successfully");
                Ok(())
            }
        }
    }

    /// Get the container name for a template
    ///
    /// Format: `{prefix}_{template_id}`
    pub fn get_container_name(&self, template_id: &TemplateId) -> String {
        template_id.to_container_name(&self.config.data_container_prefix)
    }

    /// Get a container client for a template
    pub fn get_container_client(&self, template_id: &TemplateId) -> ContainerClient {
        let container_name = self.get_container_name(template_id);
        self.database.container_client(&container_name)
    }

    /// Get the control container client
    pub fn get_control_container_client(&self) -> ContainerClient {
        self.database
            .container_client(&self.config.control_container)
    }

    /// Check if a composition exists in the container
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID
    /// * `ehr_id` - EHR ID (partition key)
    /// * `composition_id` - Composition ID (document ID)
    pub async fn check_composition_exists(
        &self,
        template_id: &TemplateId,
        ehr_id: &str,
        composition_id: &str,
    ) -> Result<bool> {
        let container = self.get_container_client(template_id);
        let partition_key = PartitionKey::from(ehr_id.to_string());

        match container
            .read_item::<serde_json::Value>(partition_key, composition_id, None)
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                // Check if it's a 404 (not found) error
                if e.to_string().contains("404") || e.to_string().contains("NotFound") {
                    Ok(false)
                } else {
                    Err(AtlasError::CosmosDb(CosmosDbError::QueryFailed(format!(
                        "Failed to check if composition exists: {e}"
                    ))))
                }
            }
        }
    }

    /// Fetch a composition document from Cosmos DB
    ///
    /// # Arguments
    ///
    /// * `template_id` - Template ID
    /// * `ehr_id` - EHR ID (partition key)
    /// * `composition_uid` - Composition UID (document ID)
    ///
    /// # Returns
    ///
    /// Returns the composition document as a JSON Value
    pub async fn fetch_composition(
        &self,
        template_id: &TemplateId,
        ehr_id: &EhrId,
        composition_uid: &CompositionUid,
    ) -> Result<Value> {
        let container = self.get_container_client(template_id);
        let partition_key = PartitionKey::from(ehr_id.as_str().to_string());

        tracing::debug!(
            template_id = %template_id.as_str(),
            ehr_id = %ehr_id.as_str(),
            composition_uid = %composition_uid.as_str(),
            "Fetching composition from Cosmos DB using query"
        );

        // Use query instead of read_item to avoid potential issues with special characters in document IDs
        let query = format!(
            "SELECT * FROM c WHERE c.id = '{}'",
            composition_uid.as_str().replace('\'', "''") // Escape single quotes
        );

        let mut query_response = container
            .query_items::<Value>(query, partition_key, None)
            .map_err(|e| {
                AtlasError::CosmosDb(CosmosDbError::QueryFailed(format!(
                    "Failed to create query: {e}"
                )))
            })?;

        // Collect all results
        let mut documents = Vec::new();
        while let Some(item) = query_response.next().await {
            match item {
                Ok(doc) => documents.push(doc),
                Err(e) => {
                    return Err(AtlasError::CosmosDb(CosmosDbError::QueryFailed(format!(
                        "Failed to fetch composition: {e}"
                    ))));
                }
            }
        }

        if documents.is_empty() {
            Err(AtlasError::CosmosDb(CosmosDbError::QueryFailed(format!(
                "Composition not found: {}",
                composition_uid.as_str()
            ))))
        } else if documents.len() > 1 {
            tracing::warn!(
                composition_uid = %composition_uid.as_str(),
                count = documents.len(),
                "Multiple documents found with same ID, using first one"
            );
            Ok(documents.into_iter().next().unwrap())
        } else {
            Ok(documents.into_iter().next().unwrap())
        }
    }

    /// Get the database name
    pub fn database_name(&self) -> &str {
        &self.config.database_name
    }

    /// Get the endpoint URL
    pub fn endpoint(&self) -> &str {
        &self.config.endpoint
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::secret::SecretValue;
    use secrecy::Secret;

    #[test]
    fn test_get_container_name() {
        let config = CosmosDbConfig {
            endpoint: "https://test.documents.azure.com:443/".to_string(),
            key: Secret::new(SecretValue::from("test-key".to_string())),
            database_name: "test_db".to_string(),
            data_container_prefix: "compositions".to_string(),
            control_container: "atlas_control".to_string(),
            partition_key: "/ehr_id".to_string(),
            max_concurrency: 10,
            request_timeout_seconds: 30,
        };

        use secrecy::ExposeSecret;
        let key_str: String = config.key.expose_secret().clone().into();
        let azure_secret = azure_core::credentials::Secret::new(key_str);

        let client = CosmosDbClient {
            client: CosmosClient::with_key(
                &config.endpoint,
                azure_secret.clone(),
                Some(CosmosClientOptions::default()),
            )
            .unwrap(),
            database: CosmosClient::with_key(
                &config.endpoint,
                azure_secret,
                Some(CosmosClientOptions::default()),
            )
            .unwrap()
            .database_client(&config.database_name),
            config,
        };

        let template_id = TemplateId::new("vital_signs").unwrap();
        assert_eq!(
            client.get_container_name(&template_id),
            "compositions_vital_signs"
        );
    }
}

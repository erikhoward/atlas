//! Database client factory
//!
//! This module provides factory functions to create database clients based on configuration.

use crate::adapters::cosmosdb::adapter::CosmosDbAdapter;
use crate::adapters::cosmosdb::client::CosmosDbClient;
use crate::adapters::database::traits::{DatabaseClient, StateStorage};
use crate::adapters::postgresql::adapter::PostgreSQLAdapter;
use crate::adapters::postgresql::client::PostgreSQLClient;
use crate::config::schema::{AtlasConfig, DatabaseTarget};
use crate::domain::Result;
use std::sync::Arc;

/// Create a database client based on the configuration
///
/// This factory function examines the `database_target` in the configuration
/// and creates the appropriate database client implementation.
///
/// # Arguments
///
/// * `config` - The Atlas configuration
///
/// # Returns
///
/// Returns an Arc-wrapped trait object that implements DatabaseClient
///
/// # Errors
///
/// Returns an error if the database client cannot be created
pub async fn create_database_client(
    config: &AtlasConfig,
) -> Result<Arc<dyn DatabaseClient + Send + Sync>> {
    match config.database_target {
        DatabaseTarget::CosmosDB => {
            let cosmos_config = config
                .cosmosdb
                .as_ref()
                .expect("CosmosDB config should be validated");

            tracing::info!("Creating CosmosDB client");
            let client = CosmosDbClient::new(cosmos_config.clone()).await?;
            let adapter = CosmosDbAdapter::new(client);

            Ok(Arc::new(adapter) as Arc<dyn DatabaseClient + Send + Sync>)
        }
        DatabaseTarget::PostgreSQL => {
            let pg_config = config
                .postgresql
                .as_ref()
                .expect("PostgreSQL config should be validated");

            tracing::info!("Creating PostgreSQL client");
            let client = PostgreSQLClient::new(pg_config.clone()).await?;
            let adapter = PostgreSQLAdapter::new(client);

            Ok(Arc::new(adapter) as Arc<dyn DatabaseClient + Send + Sync>)
        }
    }
}

/// Create a state storage client based on the configuration
///
/// This factory function examines the `database_target` in the configuration
/// and creates the appropriate state storage implementation.
///
/// # Arguments
///
/// * `config` - The Atlas configuration
///
/// # Returns
///
/// Returns an Arc-wrapped trait object that implements StateStorage
///
/// # Errors
///
/// Returns an error if the state storage client cannot be created
pub async fn create_state_storage(
    config: &AtlasConfig,
) -> Result<Arc<dyn StateStorage + Send + Sync>> {
    match config.database_target {
        DatabaseTarget::CosmosDB => {
            let cosmos_config = config
                .cosmosdb
                .as_ref()
                .expect("CosmosDB config should be validated");

            tracing::info!("Creating CosmosDB state storage");
            let client = CosmosDbClient::new(cosmos_config.clone()).await?;
            let adapter = CosmosDbAdapter::new(client);

            Ok(Arc::new(adapter) as Arc<dyn StateStorage + Send + Sync>)
        }
        DatabaseTarget::PostgreSQL => {
            let pg_config = config
                .postgresql
                .as_ref()
                .expect("PostgreSQL config should be validated");

            tracing::info!("Creating PostgreSQL state storage");
            let client = PostgreSQLClient::new(pg_config.clone()).await?;
            let adapter = PostgreSQLAdapter::new(client);

            Ok(Arc::new(adapter) as Arc<dyn StateStorage + Send + Sync>)
        }
    }
}

/// Create both database client and state storage from the same underlying client
///
/// This is more efficient than creating two separate clients as it reuses
/// the same connection pool.
///
/// # Arguments
///
/// * `config` - The Atlas configuration
///
/// # Returns
///
/// Returns a tuple of (DatabaseClient, StateStorage) trait objects
///
/// # Errors
///
/// Returns an error if the clients cannot be created
pub async fn create_database_and_state(
    config: &AtlasConfig,
) -> Result<(
    Arc<dyn DatabaseClient + Send + Sync>,
    Arc<dyn StateStorage + Send + Sync>,
)> {
    match config.database_target {
        DatabaseTarget::CosmosDB => {
            let cosmos_config = config
                .cosmosdb
                .as_ref()
                .expect("CosmosDB config should be validated");

            tracing::info!("Creating CosmosDB client and state storage");
            let client = Arc::new(CosmosDbClient::new(cosmos_config.clone()).await?);
            let adapter = Arc::new(CosmosDbAdapter::new_with_arc(client));

            Ok((
                adapter.clone() as Arc<dyn DatabaseClient + Send + Sync>,
                adapter as Arc<dyn StateStorage + Send + Sync>,
            ))
        }
        DatabaseTarget::PostgreSQL => {
            let pg_config = config
                .postgresql
                .as_ref()
                .expect("PostgreSQL config should be validated");

            tracing::info!("Creating PostgreSQL client and state storage");
            let client = Arc::new(PostgreSQLClient::new(pg_config.clone()).await?);
            let adapter = Arc::new(PostgreSQLAdapter::new_with_arc(client));

            Ok((
                adapter.clone() as Arc<dyn DatabaseClient + Send + Sync>,
                adapter as Arc<dyn StateStorage + Send + Sync>,
            ))
        }
    }
}


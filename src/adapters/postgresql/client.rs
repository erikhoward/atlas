//! PostgreSQL client implementation
//!
//! This module provides the client for interacting with PostgreSQL.

use crate::config::schema::PostgreSQLConfig;
use crate::domain::{AtlasError, Result};
use deadpool_postgres::{Config as PoolConfig, Manager, ManagerConfig, Pool, RecyclingMethod};
use std::time::Duration;
use tokio_postgres::{NoTls, Row};

/// PostgreSQL client for Atlas
///
/// Provides methods for connecting to PostgreSQL, managing tables,
/// and performing document operations using connection pooling.
pub struct PostgreSQLClient {
    /// Connection pool
    pool: Pool,

    /// Configuration
    config: PostgreSQLConfig,
}

impl PostgreSQLClient {
    /// Create a new PostgreSQL client
    ///
    /// # Arguments
    ///
    /// * `config` - PostgreSQL configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the client cannot be created or the connection fails.
    pub async fn new(config: PostgreSQLConfig) -> Result<Self> {
        // Parse connection string
        let pg_config: tokio_postgres::Config = config.connection_string.parse().map_err(|e| {
            AtlasError::Configuration(format!("Invalid PostgreSQL connection string: {}", e))
        })?;

        // Create pool configuration
        let mut pool_config = PoolConfig::new();
        pool_config.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        // Create manager
        let manager = Manager::from_config(pg_config, NoTls, pool_config.manager.unwrap());

        // Create pool
        let pool = Pool::builder(manager)
            .max_size(config.max_connections)
            .wait_timeout(Some(Duration::from_secs(config.connection_timeout_seconds)))
            .create_timeout(Some(Duration::from_secs(config.connection_timeout_seconds)))
            .recycle_timeout(Some(Duration::from_secs(config.connection_timeout_seconds)))
            .build()
            .map_err(|e| {
                AtlasError::Database(format!("Failed to create connection pool: {}", e))
            })?;

        Ok(Self { pool, config })
    }

    /// Test the connection to PostgreSQL
    ///
    /// Attempts to get a connection from the pool and execute a simple query.
    pub async fn test_connection(&self) -> Result<()> {
        let client = self.pool.get().await.map_err(|e| {
            AtlasError::Database(format!("Failed to get connection from pool: {}", e))
        })?;

        client
            .query_one("SELECT 1", &[])
            .await
            .map_err(|e| AtlasError::Database(format!("Connection test failed: {}", e)))?;

        tracing::info!("PostgreSQL connection test successful");
        Ok(())
    }

    /// Ensure the database schema exists
    ///
    /// This runs the migration SQL to create tables and indexes if they don't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the schema cannot be created.
    pub async fn ensure_database_exists(&self) -> Result<()> {
        let client = self.pool.get().await.map_err(|e| {
            AtlasError::Database(format!("Failed to get connection from pool: {}", e))
        })?;

        // Read migration SQL
        let migration_sql = include_str!("../../../migrations/001_initial_schema.sql");

        // Execute migration
        client
            .batch_execute(migration_sql)
            .await
            .map_err(|e| AtlasError::Database(format!("Failed to execute migration: {}", e)))?;

        tracing::info!("PostgreSQL schema initialized successfully");
        Ok(())
    }

    /// Ensure a table exists for compositions
    ///
    /// In PostgreSQL, we use a single table for all compositions,
    /// so this is a no-op (the table is created in ensure_database_exists).
    ///
    /// # Arguments
    ///
    /// * `_template_id` - Template ID (unused in PostgreSQL implementation)
    pub async fn ensure_table_exists(&self, _template_id: &str) -> Result<()> {
        // No-op: PostgreSQL uses a single table for all compositions
        Ok(())
    }

    /// Ensure the watermarks table exists
    ///
    /// This is a no-op since the table is created in ensure_database_exists.
    pub async fn ensure_watermarks_table_exists(&self) -> Result<()> {
        // No-op: Table is created in ensure_database_exists
        Ok(())
    }

    /// Get a connection from the pool
    ///
    /// # Errors
    ///
    /// Returns an error if a connection cannot be obtained.
    pub async fn get_connection(&self) -> Result<deadpool_postgres::Object> {
        self.pool
            .get()
            .await
            .map_err(|e| AtlasError::Database(format!("Failed to get connection from pool: {}", e)))
    }

    /// Execute a query and return rows
    ///
    /// # Arguments
    ///
    /// * `query` - SQL query
    /// * `params` - Query parameters
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn query(
        &self,
        query: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<Vec<Row>> {
        let client = self.get_connection().await?;

        // Set statement timeout
        let timeout_query = format!(
            "SET statement_timeout = {}",
            self.config.statement_timeout_seconds * 1000
        );
        client
            .execute(&timeout_query, &[])
            .await
            .map_err(|e| AtlasError::Database(format!("Failed to set statement timeout: {}", e)))?;

        client
            .query(query, params)
            .await
            .map_err(|e| AtlasError::Database(format!("Query failed: {}", e)))
    }

    /// Execute a statement and return the number of affected rows
    ///
    /// # Arguments
    ///
    /// * `statement` - SQL statement
    /// * `params` - Statement parameters
    ///
    /// # Errors
    ///
    /// Returns an error if the statement fails.
    pub async fn execute(
        &self,
        statement: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<u64> {
        let client = self.get_connection().await?;

        // Set statement timeout
        let timeout_query = format!(
            "SET statement_timeout = {}",
            self.config.statement_timeout_seconds * 1000
        );
        client
            .execute(&timeout_query, &[])
            .await
            .map_err(|e| AtlasError::Database(format!("Failed to set statement timeout: {}", e)))?;

        client
            .execute(statement, params)
            .await
            .map_err(|e| AtlasError::Database(format!("Statement execution failed: {}", e)))
    }

    /// Get the connection string (without password)
    pub fn connection_string_safe(&self) -> String {
        // Redact password from connection string
        self.config
            .connection_string
            .split('@')
            .last()
            .map(|s| format!("postgresql://***@{}", s))
            .unwrap_or_else(|| "postgresql://***".to_string())
    }

    /// Get the pool statistics
    pub fn pool_status(&self) -> deadpool_postgres::Status {
        self.pool.status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_string_safe() {
        let config = PostgreSQLConfig {
            connection_string: "postgresql://user:password@localhost:5432/atlas".to_string(),
            max_connections: 10,
            connection_timeout_seconds: 30,
            statement_timeout_seconds: 60,
            ssl_mode: "prefer".to_string(),
        };

        let client = PostgreSQLClient {
            pool: Pool::builder(Manager::from_config(
                config.connection_string.parse().unwrap(),
                NoTls,
                ManagerConfig {
                    recycling_method: RecyclingMethod::Fast,
                },
            ))
            .max_size(10)
            .build()
            .unwrap(),
            config: config.clone(),
        };

        let safe_str = client.connection_string_safe();
        assert!(!safe_str.contains("password"));
        assert!(safe_str.contains("localhost:5432/atlas"));
    }
}

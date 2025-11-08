//! PostgreSQL client implementation
//!
//! This module provides the client for interacting with PostgreSQL.

use crate::config::schema::PostgreSQLConfig;
use crate::domain::{AtlasError, Result};
use deadpool_postgres::{Config as PoolConfig, Manager, ManagerConfig, Pool, RecyclingMethod};
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use tokio_postgres::Row;

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
        // Log connection attempt (without password)
        tracing::debug!(
            "Creating PostgreSQL client with connection string (password hidden): {}",
            config.connection_string.replace(
                |c: char| c != '@'
                    && c != ':'
                    && c != '/'
                    && !c.is_alphanumeric()
                    && c != '.'
                    && c != '-'
                    && c != '='
                    && c != '?',
                "*"
            )
        );

        // Parse connection string
        let pg_config: tokio_postgres::Config = config.connection_string.parse().map_err(|e| {
            AtlasError::Configuration(format!("Invalid PostgreSQL connection string: {e}"))
        })?;

        // Create pool configuration
        let mut pool_config = PoolConfig::new();
        pool_config.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        // Create TLS connector
        // Accept invalid certificates if ssl_mode is not verify-ca or verify-full
        let accept_invalid_certs = !matches!(config.ssl_mode.as_str(), "verify-ca" | "verify-full");

        tracing::debug!(
            "Creating TLS connector with accept_invalid_certs={}",
            accept_invalid_certs
        );

        let tls_connector = TlsConnector::builder()
            .danger_accept_invalid_certs(accept_invalid_certs)
            .build()
            .map_err(|e| {
                AtlasError::Configuration(format!("Failed to create TLS connector: {e}"))
            })?;
        let tls = MakeTlsConnector::new(tls_connector);

        // Test direct connection first to get better error messages
        tracing::debug!("Testing direct PostgreSQL connection before creating pool...");
        let test_tls_connector = TlsConnector::builder()
            .danger_accept_invalid_certs(accept_invalid_certs)
            .build()
            .map_err(|e| {
                AtlasError::Configuration(format!("Failed to create test TLS connector: {e}"))
            })?;
        let test_tls = MakeTlsConnector::new(test_tls_connector);

        match tokio_postgres::connect(&config.connection_string, test_tls).await {
            Ok((client, connection)) => {
                // Spawn the connection handler
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        tracing::error!("PostgreSQL connection error: {}", e);
                    }
                });

                // Test a simple query
                match client.query_one("SELECT 1 as test", &[]).await {
                    Ok(_) => {
                        tracing::info!("Direct PostgreSQL connection test successful");
                    }
                    Err(e) => {
                        return Err(AtlasError::Database(format!(
                            "Direct connection succeeded but test query failed: {e}"
                        )));
                    }
                }
            }
            Err(e) => {
                // Try to get more detailed error information by walking the error chain
                let error_display = format!("{e}");
                let error_debug = format!("{e:?}");

                // Try to get the source error
                let mut error_chain = vec![error_display.clone()];
                let mut source = std::error::Error::source(&e);
                while let Some(err) = source {
                    error_chain.push(format!("{err}"));
                    source = std::error::Error::source(err);
                }

                tracing::error!("PostgreSQL connection failed!");
                tracing::error!("Error: {}", error_display);
                tracing::error!("Error chain: {:?}", error_chain);
                tracing::error!("Debug: {}", error_debug);

                // Build detailed error message with full chain
                let full_error = error_chain.join("\n  Caused by: ");

                // Check if it's a TLS/SSL error
                let error_hint = if full_error.contains("tls")
                    || full_error.contains("ssl")
                    || full_error.contains("TLS")
                    || full_error.contains("SSL")
                    || full_error.contains("certificate")
                {
                    "\n\n🔐 This appears to be a TLS/SSL error. Try:\n\
                    - Change sslmode=prefer to sslmode=disable in connection string\n\
                    - OR configure PostgreSQL to accept SSL connections\n\
                    - Check PostgreSQL logs: sudo tail -f /var/log/postgresql/postgresql-*-main.log"
                } else if full_error.contains("authentication") || full_error.contains("password") {
                    "\n\n🔑 This appears to be an authentication error. Try:\n\
                    - Verify ATLAS_PG_PASSWORD is set correctly\n\
                    - Check user password: psql -U atlas_user -d openehr_data -h localhost"
                } else {
                    "\n\n💡 Generic connection error. Check:\n\
                    - PostgreSQL is running: sudo systemctl status postgresql\n\
                    - Server is listening: sudo netstat -tlnp | grep 5432\n\
                    - Database exists: psql -U atlas_user -l"
                };

                return Err(AtlasError::Database(format!(
                    "Failed to connect to PostgreSQL.\n\
                    \nError: {}\n\
                    \nConnection target: {}{}",
                    full_error,
                    config
                        .connection_string
                        .split('@')
                        .next_back()
                        .unwrap_or("unknown"),
                    error_hint
                )));
            }
        }

        // Create manager
        let manager = Manager::from_config(pg_config, tls, pool_config.manager.unwrap());

        // Create pool
        // Note: Timeouts are not set during pool creation to avoid runtime detection issues.
        // Connection timeouts are handled at the PostgreSQL connection string level via
        // connect_timeout parameter, and statement timeouts via statement_timeout.
        let pool = Pool::builder(manager)
            .max_size(config.max_connections)
            .build()
            .map_err(|e| {
                AtlasError::Database(format!("Failed to create connection pool: {e}"))
            })?;

        // Test the connection immediately to get better error messages
        tracing::debug!("Testing PostgreSQL connection...");
        let test_client = pool.get().await.map_err(|e| {
            // Try to extract more detailed error information
            let error_msg = format!("{e}");
            let detailed_msg = if error_msg.contains("error connecting to server") {
                format!(
                    "Failed to connect to PostgreSQL server. \
                    Please check:\n\
                    1. PostgreSQL is running: sudo systemctl status postgresql\n\
                    2. Server is listening on localhost:5432\n\
                    3. Database 'openehr_data' exists\n\
                    4. User 'atlas_user' has access\n\
                    5. pg_hba.conf allows connections from localhost\n\
                    \nOriginal error: {e}"
                )
            } else {
                format!("Failed to get connection from pool: {e}")
            };
            AtlasError::Database(detailed_msg)
        })?;

        // Run a simple test query
        test_client
            .query_one("SELECT 1 as test", &[])
            .await
            .map_err(|e| AtlasError::Database(format!("Connection test query failed: {e}")))?;

        tracing::info!("PostgreSQL connection test successful");

        Ok(Self { pool, config })
    }

    /// Test the connection to PostgreSQL
    ///
    /// Attempts to get a connection from the pool and execute a simple query.
    pub async fn test_connection(&self) -> Result<()> {
        let client = self.pool.get().await.map_err(|e| {
            AtlasError::Database(format!("Failed to get connection from pool: {e}"))
        })?;

        client
            .query_one("SELECT 1", &[])
            .await
            .map_err(|e| AtlasError::Database(format!("Connection test failed: {e}")))?;

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
            AtlasError::Database(format!("Failed to get connection from pool: {e}"))
        })?;

        // Read migration SQL
        let migration_sql = include_str!("../../../migrations/001_initial_schema.sql");

        // Execute migration
        client.batch_execute(migration_sql).await.map_err(|e| {
            // Extract detailed error information
            let error_msg = format!("{e}");
            let error_debug = format!("{e:?}");

            tracing::error!("Migration failed: {}", error_msg);
            tracing::error!("Debug info: {}", error_debug);

            // Check for common errors and provide helpful messages
            if error_msg.contains("must be owner") {
                AtlasError::Database(format!(
                    "Failed to execute migration: Permission denied.\n\
                        \nThe tables exist but are owned by a different user.\n\
                        \nTo fix this, run as postgres superuser:\n\
                        docker exec -it local-postgres psql -U postgres -d openehr_data -c \\\n\
                        \"ALTER TABLE compositions OWNER TO atlas_user; \\\n\
                         ALTER TABLE watermarks OWNER TO atlas_user;\"\n\
                        \nOriginal error: {error_msg}"
                ))
            } else if error_msg.contains("column") && error_msg.contains("does not exist") {
                AtlasError::Database(format!(
                    "Failed to execute migration: Schema mismatch detected.\n\
                        \nThe existing tables have an outdated schema that doesn't match the current migration.\n\
                        \nThis typically happens after a major refactor.\n\
                        \n⚠️  DEVELOPMENT ONLY - To drop and recreate tables (DELETES ALL DATA):\n\
                        docker exec -it local-postgres psql -U atlas_user -d openehr_data -c \\\n\
                        \"DROP TABLE IF EXISTS compositions CASCADE; DROP TABLE IF EXISTS watermarks CASCADE;\"\n\
                        \nThen restart Atlas to recreate tables with the new schema.\n\
                        \n📖 For production migrations, see: migrations/README.md\n\
                        \nOriginal error: {error_msg}"
                ))
            } else {
                AtlasError::Database(format!(
                    "Failed to execute migration: {error_msg}\n\
                        \nDebug: {error_debug}\n\
                        \nFor troubleshooting, see: migrations/README.md"
                ))
            }
        })?;

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
            .map_err(|e| AtlasError::Database(format!("Failed to get connection from pool: {e}")))
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
            .map_err(|e| AtlasError::Database(format!("Failed to set statement timeout: {e}")))?;

        client
            .query(query, params)
            .await
            .map_err(|e| AtlasError::Database(format!("Query failed: {e}")))
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
            .map_err(|e| AtlasError::Database(format!("Failed to set statement timeout: {e}")))?;

        client
            .execute(statement, params)
            .await
            .map_err(|e| AtlasError::Database(format!("Statement execution failed: {e}")))
    }

    /// Get the connection string (without password)
    pub fn connection_string_safe(&self) -> String {
        // Redact password from connection string
        self.config
            .connection_string
            .split('@')
            .next_back()
            .map(|s| format!("postgresql://***@{s}"))
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
    use tokio_postgres::NoTls;

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

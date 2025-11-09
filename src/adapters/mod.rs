//! External system integrations for Atlas.
//!
//! This module provides adapters for integrating with external systems:
//!
//! - [`openehr`] - OpenEHR server integration (EHRBase, Better Platform, etc.)
//! - [`database`] - Database abstraction layer (trait-based)
//! - [`cosmosdb`] - Azure Cosmos DB implementation
//! - [`postgresql`] - PostgreSQL implementation (coming soon)
//!
//! # Design Pattern
//!
//! Adapters follow the **Adapter Pattern** to isolate external dependencies and
//! enable testing with mock implementations. The database layer uses trait-based
//! abstraction to support multiple database backends.
//!
//! # OpenEHR Adapter
//!
//! The OpenEHR adapter uses a trait-based design for vendor abstraction:
//!
//! ```rust,no_run
//! use atlas::adapters::openehr::OpenEhrClient;
//! use atlas::config::{OpenEhrConfig, SecretString, SecretValue};
//! use secrecy::Secret;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = OpenEhrConfig {
//!     base_url: "https://ehrbase.example.com/ehrbase/rest/openehr/v1".to_string(),
//!     vendor: "ehrbase".to_string(),
//!     vendor_type: "ehrbase".to_string(),
//!     auth_type: "basic".to_string(),
//!     username: Some("user".to_string()),
//!     password: Some(Secret::new(SecretValue::from("pass".to_string()))),
//!     tls_verify: true,
//!     tls_verify_certificates: true,
//!     tls_ca_cert: None,
//!     timeout_seconds: 30,
//!     retry: Default::default(),
//!     query: Default::default(),
//! };
//!
//! let client = OpenEhrClient::new(config).await?;
//! // Use client for operations
//! # Ok(())
//! # }
//! ```
//!
//! # Cosmos DB Adapter
//!
//! The Cosmos DB adapter provides bulk operations and state management:
//!
//! ```rust,no_run
//! use atlas::adapters::cosmosdb::CosmosDbClient;
//! use atlas::config::{CosmosDbConfig, SecretString, SecretValue};
//! use secrecy::Secret;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = CosmosDbConfig {
//!     endpoint: "https://account.documents.azure.com:443/".to_string(),
//!     key: Secret::new(SecretValue::from("key".to_string())),
//!     database_name: "openehr_data".to_string(),
//!     control_container: "atlas_control".to_string(),
//!     data_container_prefix: "compositions".to_string(),
//!     partition_key: "/ehr_id".to_string(),
//!     max_concurrency: 10,
//!     request_timeout_seconds: 30,
//! };
//!
//! let client = CosmosDbClient::new(config).await?;
//! // Use client for bulk operations
//! # Ok(())
//! # }
//! ```

pub mod cosmosdb;
pub mod database;
pub mod openehr;
pub mod postgresql;

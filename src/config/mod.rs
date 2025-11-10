//! Configuration management for Atlas.
//!
//! This module provides TOML-based configuration loading, parsing, and validation
//! following Microsoft Rust Guidelines TR-4.1 through TR-4.4.
//!
//! # Overview
//!
//! Atlas uses TOML configuration files with support for:
//! - Environment variable substitution (`${VAR_NAME}`)
//! - Default values for optional settings
//! - Comprehensive validation
//! - Type-safe configuration structs
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use atlas::config::load_config;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Load configuration from file
//! let config = load_config("atlas.toml")?;
//!
//! // Access configuration sections
//! println!("OpenEHR URL: {}", config.openehr.base_url);
//! if let Some(cosmosdb) = &config.cosmosdb {
//!     println!("Cosmos DB: {}", cosmosdb.database_name);
//! }
//! println!("Export mode: {}", config.export.mode);
//! # Ok(())
//! # }
//! ```
//!
//! # Configuration Structure
//!
//! The configuration is organized into sections:
//!
//! - [`ApplicationConfig`] - Application settings (name, version, log level)
//! - [`OpenEhrConfig`] - OpenEHR connection and authentication
//! - [`QueryConfig`] - Query parameters (templates, EHRs, batch size)
//! - [`ExportConfig`] - Export settings (mode, format, retries)
//! - [`CosmosDbConfig`] - Cosmos DB connection and settings
//! - [`StateConfig`] - State management and checkpointing
//! - [`VerificationConfig`] - Data verification settings
//! - [`LoggingConfig`] - Logging configuration
//!
//! # Example Configuration
//!
//! ```toml
//! [application]
//! log_level = "info"
//!
//! [openehr]
//! base_url = "https://ehrbase.example.com/ehrbase/rest/openehr/v1"
//! username = "atlas_user"
//! password = "${ATLAS_OPENEHR_PASSWORD}"
//!
//! [openehr.query]
//! template_ids = ["IDCR - Vital Signs.v1"]
//! batch_size = 1000
//!
//! [cosmosdb]
//! endpoint = "https://your-account.documents.azure.com:443/"
//! key = "${ATLAS_COSMOS_KEY}"
//! database_name = "openehr_data"
//!
//! [export]
//! mode = "incremental"
//! export_composition_format = "preserve"
//! ```
//!
//! # Environment Variables
//!
//! Atlas supports comprehensive environment variable overrides for 12-factor app compliance.
//!
//! ## Substitution Syntax
//!
//! Use `${VAR_NAME}` in TOML files for environment variable substitution:
//!
//! ```bash
//! export OPENEHR_PASSWORD="secret-password"
//! export COSMOS_KEY="secret-key"
//! ```
//!
//! ```toml
//! [openehr]
//! password = "${OPENEHR_PASSWORD}"
//!
//! [cosmosdb]
//! key = "${COSMOS_KEY}"
//! ```
//!
//! ## Override Syntax
//!
//! Use `ATLAS_<SECTION>_<KEY>` environment variables to override any configuration value:
//!
//! ```bash
//! # Database selection
//! export ATLAS_DATABASE_TARGET=postgresql
//!
//! # Application settings
//! export ATLAS_APPLICATION_LOG_LEVEL=debug
//! export ATLAS_APPLICATION_DRY_RUN=true
//!
//! # OpenEHR connection
//! export ATLAS_OPENEHR_BASE_URL=https://prod-ehrbase.com
//! export ATLAS_OPENEHR_USERNAME=atlas_user
//! export ATLAS_OPENEHR_PASSWORD=secret
//! export ATLAS_OPENEHR_TIMEOUT_SECONDS=120
//!
//! # Query settings (arrays support JSON or comma-separated)
//! export ATLAS_OPENEHR_QUERY_BATCH_SIZE=2000
//! export ATLAS_OPENEHR_QUERY_TEMPLATE_IDS='["IDCR - Vital Signs.v1","IDCR - Lab Report.v1"]'
//! export ATLAS_OPENEHR_QUERY_EHR_IDS="ehr-123,ehr-456,ehr-789"
//!
//! # Export settings
//! export ATLAS_EXPORT_MODE=full
//! export ATLAS_EXPORT_DRY_RUN=false
//! export ATLAS_EXPORT_RETRY_BACKOFF_MS="1000,2000,4000"
//!
//! # PostgreSQL
//! export ATLAS_POSTGRESQL_CONNECTION_STRING="postgresql://user:pass@localhost/db"
//! export ATLAS_POSTGRESQL_MAX_CONNECTIONS=20
//! export ATLAS_POSTGRESQL_SSL_MODE=require
//!
//! # Cosmos DB
//! export ATLAS_COSMOSDB_ENDPOINT=https://myaccount.documents.azure.com:443/
//! export ATLAS_COSMOSDB_KEY=secret-key
//! export ATLAS_COSMOSDB_DATABASE_NAME=prod_openehr
//!
//! # Logging
//! export ATLAS_LOGGING_LOCAL_ENABLED=true
//! export ATLAS_LOGGING_LOCAL_PATH=/var/log/atlas
//! export ATLAS_LOGGING_LOCAL_ROTATION=daily
//! ```
//!
//! Environment variable overrides take precedence over TOML file values, enabling
//! containerized deployments without modifying configuration files.
//!
//! See [`loader`] module documentation for complete list of supported variables.
//!
//! # Validation
//!
//! Configuration is validated on load:
//!
//! ```rust,no_run
//! use atlas::config::load_config;
//!
//! # fn example() {
//! match load_config("atlas.toml") {
//!     Ok(config) => println!("Configuration valid"),
//!     Err(e) => eprintln!("Configuration error: {}", e),
//! }
//! # }
//! ```
//!
//! # See Also
//!
//! - [Configuration Guide](https://github.com/erikhoward/atlas/blob/main/docs/configuration.md)
//! - [Example Configurations](https://github.com/erikhoward/atlas/tree/main/examples)

pub mod loader;
pub mod schema;
pub mod secret;

// Re-export commonly used types
pub use loader::load_config;
pub use schema::{
    ApplicationConfig, AtlasConfig, CosmosDbConfig, Environment, ExportConfig, LoggingConfig,
    OpenEhrConfig, QueryConfig, StateConfig, VerificationConfig,
};
pub use secret::{secret_string, secret_string_opt, SecretString, SecretValue};

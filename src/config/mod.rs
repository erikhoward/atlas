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
//! name = "atlas"
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
//! Use `${VAR_NAME}` syntax for environment variable substitution:
//!
//! ```bash
//! export ATLAS_OPENEHR_PASSWORD="secret-password"
//! export ATLAS_COSMOS_KEY="secret-key"
//! ```
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
    ApplicationConfig, AtlasConfig, CosmosDbConfig, ExportConfig, LoggingConfig, OpenEhrConfig,
    QueryConfig, StateConfig, VerificationConfig,
};
pub use secret::{secret_string, secret_string_opt, SecretString, SecretValue};

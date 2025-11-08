// Atlas - OpenEHR to Azure Cosmos DB ETL Tool
// Copyright (c) 2025 Atlas Contributors
// Licensed under the MIT License

//! # Atlas - OpenEHR to Azure Cosmos DB ETL
//!
//! Atlas is a high-performance ETL tool built in Rust that exports OpenEHR clinical data
//! from EHRBase servers to Azure Cosmos DB for analytics, machine learning, and research.
//!
//! ## Overview
//!
//! This library provides the core functionality for:
//! - **Extracting** compositions from OpenEHR servers via REST API v1.1
//! - **Transforming** data with preserve or flatten modes
//! - **Loading** data into Azure Cosmos DB with batch processing
//! - **Managing** export state with high-watermark tracking for incremental sync
//!
//! ## Architecture
//!
//! Atlas follows a layered architecture:
//!
//! - [`cli`] - Command-line interface and argument parsing
//! - [`core`] - Business logic (export, transform, state, verification)
//! - [`adapters`] - External integrations (OpenEHR, Cosmos DB)
//! - [`domain`] - Core domain types and models
//! - [`config`] - Configuration management
//! - [`logging`] - Structured logging and observability
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use atlas::config::{AtlasConfig, load_config};
//! use atlas::core::export::ExportCoordinator;
//! use atlas::domain::AtlasError;
//! use std::sync::Arc;
//! use atlas::adapters::openehr::rest::OpenEHRRestAdapter;
//! use atlas::adapters::cosmos::CosmosRepository;
//! use atlas::logging::{init_logging, LogLevel, LoggerType};
//! use tracing::info;
//! use cosmos_sdk::cosmos::CosmosClient; // Assuming cosmos_sdk is in Cargo.toml
//!
//! #[tokio::main]
//! async fn main() -> Result<(), AtlasError> {
//!     // Initialize logging for visibility
//!     init_logging(LogLevel::Info, LoggerType::Console)?;
//!     info!("Starting Atlas quick start example...");
//!
//!     // Load configuration from "atlas.toml". This file must exist and be valid.
//!     let config = load_config("atlas.toml").await?;
//!
//!     // Create HTTP client for the OpenEHR adapter
//!     let http_client = Arc::new(reqwest::Client::builder()
//!         .timeout(config.openehr.http_timeout)
//!         .build()
//!         .map_err(|e| AtlasError::ConfigurationError(format!("Failed to build HTTP client: {}", e)))?);
//!
//!     // Create the OpenEHR adapter
//!     let openehr_adapter = Arc::new(OpenEHRRestAdapter::new(
//!         config.openehr.server_url.clone(),
//!         config.openehr.auth_token.clone(),
//!         http_client,
//!     ));
//!
//!     // Create Cosmos DB client
//!     let cosmos_client = Arc::new(CosmosClient::new(
//!         config.database_target.clone(), // New required field `database_target`
//!         config.cosmos.database_name.clone(),
//!         config.cosmos.account_key.clone(),
//!         config.cosmos.account_endpoint.clone(),
//!         config.cosmos.partition_key_path.clone(),
//!         config.cosmos.consistency_level.clone(),
//!         config.cosmos.connection_mode.clone(),
//!         config.tls_ca_cert.clone(), // New required field `tls_ca_cert` (Option<String>)
//!     ).map_err(|e| AtlasError::CosmosDbError(format!("Failed to create Cosmos DB client: {}", e)))?);
//!
//!     // Create Cosmos repository
//!     let cosmos_repository = Arc::new(CosmosRepository::new(cosmos_client));
//!
//!     // Create export coordinator, passing in dependencies
//!     let coordinator = ExportCoordinator::new(
//!         cosmos_repository,
//!         openehr_adapter,
//!         config.export.clone(),
//!     ).await?;
//!
//!     // Execute export
//!     let summary = coordinator.execute_export().await?;
//!
//!     info!("Exported {} compositions", summary.total_compositions);
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! ### Incremental Sync
//!
//! Atlas tracks the last exported composition per {template_id, ehr_id} combination
//! using watermarks stored in Cosmos DB. This enables efficient incremental exports:
//!
//! ```rust,no_run
//! use atlas::core::state::StateManager;
//! use atlas::domain::{TemplateId, EhrId, AtlasError};
//! use atlas::adapters::cosmos::CosmosRepository;
//! use std::sync::Arc;
//! use cosmos_sdk::cosmos::CosmosClient;
//! use atlas::config::CosmosConfig; // Used for creating a dummy config
//!
//! # async fn example() -> Result<(), AtlasError> {
//! // Create a dummy CosmosConfig for example purposes.
//! // In a real application, this would come from a loaded AtlasConfig.
//! let dummy_cosmos_config = CosmosConfig {
//!     database_name: "test_db".to_string(),
//!     account_key: "dummy_key".to_string(),
//!     account_endpoint: "https://example.documents.azure.com:443/".to_string(),
//!     partition_key_path: "/id".to_string(),
//!     consistency_level: "Session".to_string(),
//!     connection_mode: "Gateway".to_string(),
//! };
//!
//! // Create a dummy Cosmos DB client for StateManager.
//! // This client would ideally connect to a test instance.
//! let cosmos_client = Arc::new(CosmosClient::new(
//!     "https://example.documents.azure.com:443/".to_string(), // dummy database_target
//!     dummy_cosmos_config.database_name.clone(),
//!     dummy_cosmos_config.account_key.clone(),
//!     dummy_cosmos_config.account_endpoint.clone(),
//!     dummy_cosmos_config.partition_key_path.clone(),
//!     dummy_cosmos_config.consistency_level.clone(),
//!     dummy_cosmos_config.connection_mode.clone(),
//!     None, // tls_ca_cert can be None for example
//! ).map_err(|e| AtlasError::CosmosDbError(format!("Failed to create dummy Cosmos DB client: {}", e)))?);
//!
//! // StateManager now likely takes Arc<CosmosRepository>
//! let state_manager = StateManager::new(Arc::new(CosmosRepository::new(cosmos_client)));
//!
//! // Load watermark for a specific template and EHR
//! let template_id = TemplateId::new("IDCR - Vital Signs.v1")?;
//! let ehr_id = EhrId::new("ehr-123")?;
//! let watermark = state_manager.load_watermark(&template_id, &ehr_id).await?;
//!
//! // Use watermark timestamp to query only new compositions
//! # Ok(())
//! # }
//! ```
//!
//! ### Data Transformation
//!
//! Atlas supports two transformation modes:
//!
//! - **Preserve**: Maintains exact FLAT JSON structure from OpenEHR
//! - **Flatten**: Converts nested paths to flat field names for analytics
//!
//! ```rust,no_run
//! use atlas::core::transform;
//! use atlas::domain::{Composition, AtlasError};
//! use serde_json::json; // For creating a mock Composition
//!
//! # fn example() -> Result<(), AtlasError> {
//! // Mock a Composition (assuming `Composition` is `serde_json::Value` or can be created from it)
//! let composition_value: Composition = json!({
//!     "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
//!     "uid": { "value": "a1d2e3f4-g5h6-i7j8-k9l0-m1n2o3p4q5r6" },
//!     "name": { "value": "Consultation" },
//!     "content": [
//!         {
//!             "_archetype_id": "openEHR-EHR-OBSERVATION.blood_pressure.v1",
//!             "systolic": 120,
//!             "diastolic": 80
//!         }
//!     ]
//! });
//!
//! // Preserve mode - maintains exact structure
//! let preserved = transform::preserve_composition(&composition_value, true)?;
//!
//! // Flatten mode - converts to flat field names
//! let flattened = transform::flatten_composition(&composition_value, true)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Batch Processing
//!
//! Atlas processes compositions in configurable batches for optimal performance:
//!
//! ```rust,no_run
//! use atlas::core::export::{BatchProcessor, BatchConfig};
//! use atlas::domain::{Composition, AtlasError};
//! use atlas::adapters::cosmos::CosmosRepository;
//! use atlas::core::state::StateManager;
//! use std::sync::Arc;
//! use cosmos_sdk::cosmos::CosmosClient;
//! use atlas::config::CosmosConfig;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), AtlasError> {
//! // Create a dummy CosmosConfig for example purposes.
//! let dummy_cosmos_config = CosmosConfig {
//!     database_name: "test_db".to_string(),
//!     account_key: "dummy_key".to_string(),
//!     account_endpoint: "https://example.documents.azure.com:443/".to_string(),
//!     partition_key_path: "/id".to_string(),
//!     consistency_level: "Session".to_string(),
//!     connection_mode: "Gateway".to_string(),
//! };
//!
//! // Create dummy Cosmos DB client and repository
//! let cosmos_client = Arc::new(CosmosClient::new(
//!     "https://example.documents.azure.com:443/".to_string(), // dummy database_target
//!     dummy_cosmos_config.database_name.clone(),
//!     dummy_cosmos_config.account_key.clone(),
//!     dummy_cosmos_config.account_endpoint.clone(),
//!     dummy_cosmos_config.partition_key_path.clone(),
//!     dummy_cosmos_config.consistency_level.clone(),
//!     dummy_cosmos_config.connection_mode.clone(),
//!     None,
//! ).map_err(|e| AtlasError::CosmosDbError(format!("Failed to create dummy Cosmos DB client: {}", e)))?);
//! let cosmos_repository = Arc::new(CosmosRepository::new(cosmos_client));
//!
//! // Create dummy StateManager (dependency for BatchProcessor)
//! let state_manager = Arc::new(StateManager::new(cosmos_repository.clone()));
//!
//! // Create batch processor, passing in dependencies
//! // Assuming BatchProcessor::new is sync.
//! let batch_processor = BatchProcessor::new(cosmos_repository, state_manager);
//!
//! // Create dummy compositions (assuming `Composition` is `serde_json::Value`)
//! let compositions: Vec<Composition> = vec![
//!     json!({"_ehr_id": "ehr-1", "_template_id": "t1", "id": "c1"}).into(),
//!     json!({"_ehr_id": "ehr-2", "_template_id": "t2", "id": "c2"}).into(),
//! ];
//!
//! // Create dummy batch configuration
//! let batch_config = BatchConfig {
//!     batch_size: 100,
//!     max_retries: 3,
//! };
//!
//! // Process batch of compositions
//! let result = batch_processor.process_batch(
//!     compositions,
//!     &batch_config,
//! ).await?;
//!
//! println!("Processed: {}, Failed: {}", result.successful, result.failed);
//! # Ok(())
//! # }
//! ```
//!
//! ## Error Handling
//!
//! Atlas uses the [`domain::AtlasError`] type for all errors, following Rust best practices:
//!
//! ```rust,no_run
//! use atlas::domain::AtlasError;
//! use atlas::config::load_config;
//! use tracing::info;
//! use atlas::logging::{init_logging, LogLevel, LoggerType};
//!
//! #[tokio::main]
//! async fn example() -> Result<(), AtlasError> {
//!     // Initialize logging for better error visibility in the example.
//!     init_logging(LogLevel::Info, LoggerType::Console)?;
//!     info!("Attempting to demonstrate error handling...");
//!
//!     // Errors are automatically converted using the ? operator.
//!     // This will fail because "non_existent_atlas.toml" does not exist,
//!     // demonstrating how `?` propagates `AtlasError`.
//!     let config = load_config("non_existent_atlas.toml").await?;
//!
//!     info!("Config loaded successfully (this line should not print in this example)");
//!     Ok(())
//! }
//! ```
//!
//! ## Logging
//!
//! Atlas uses structured logging with the `tracing` crate:
//!
//! ```rust,no_run
//! use tracing::{info, warn, error};
//! use atlas::logging::{init_logging, LogLevel, LoggerType};
//! use atlas::domain::AtlasError; // For creating a dummy error
//!
//! fn example() -> Result<(), AtlasError> {
//!     // Initialize logging to see the output from tracing macros
//!     init_logging(LogLevel::Info, LoggerType::Console)?;
//!
//!     info!("Starting export");
//!     warn!(template_id = "IDCR - Vital Signs.v1", "No compositions found");
//!
//!     // Define a dummy error for demonstration purposes
//!     let err = AtlasError::InternalError("Simulated export failure for logging example".to_string());
//!     error!(error = ?err, "Export failed"); // `?err` implies AtlasError implements Debug
//!     Ok(())
//! }
//! ```
//!
//! ## See Also
//!
//! - [User Guide](https://github.com/erikhoward/atlas/blob/main/docs/user-guide.md)
//! - [Configuration Guide](https://github.com/erikhoward/atlas/blob/main/docs/configuration.md)
//! - [Architecture Documentation](https://github.com/erikhoward/atlas/blob/main/docs/architecture.md)

pub mod adapters;
pub mod cli;
pub mod config;
pub mod core;
pub mod domain;
pub mod logging;
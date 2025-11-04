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
//! use atlas::config::AtlasConfig;
//! use atlas::core::export::ExportCoordinator;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load configuration
//!     let config = AtlasConfig::from_file("atlas.toml")?;
//!
//!     // Create export coordinator
//!     let coordinator = ExportCoordinator::new(&config).await?;
//!
//!     // Execute export
//!     let summary = coordinator.execute_export().await?;
//!
//!     println!("Exported {} compositions", summary.total_compositions);
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
//! use atlas::domain::{TemplateId, EhrId};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let state_manager = StateManager::new(/* cosmos_client */);
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
//! use atlas::domain::Composition;
//!
//! # fn example(composition: &Composition) -> Result<(), Box<dyn std::error::Error>> {
//! // Preserve mode - maintains exact structure
//! let preserved = transform::preserve_composition(composition, true)?;
//!
//! // Flatten mode - converts to flat field names
//! let flattened = transform::flatten_composition(composition, true)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Batch Processing
//!
//! Atlas processes compositions in configurable batches for optimal performance:
//!
//! ```rust,no_run
//! use atlas::core::export::BatchProcessor;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let batch_processor = BatchProcessor::new(/* cosmos_client, state_manager */);
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
//!
//! fn example() -> Result<(), AtlasError> {
//!     // Errors are automatically converted using the ? operator
//!     let config = atlas::config::AtlasConfig::from_file("atlas.toml")?;
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
//!
//! info!("Starting export");
//! warn!(template_id = "IDCR - Vital Signs.v1", "No compositions found");
//! error!(error = ?err, "Export failed");
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

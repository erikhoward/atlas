//! Core business logic for Atlas.
//!
//! This module contains the core business logic and orchestration for Atlas exports.
//!
//! # Modules
//!
//! - [`export`] - Export orchestration, batch processing, and coordination
//! - [`state`] - State management with watermarks for incremental exports
//! - [`transform`] - Data transformation (preserve and flatten modes)
//! - [`verification`] - Data verification with checksums
//!
//! # Export Workflow
//!
//! The typical export workflow:
//!
//! 1. **Load State**: Read watermarks from Cosmos DB control container
//! 2. **Query OpenEHR**: Fetch compositions since last watermark
//! 3. **Transform**: Convert to preserve or flatten format
//! 4. **Batch Process**: Group compositions and bulk insert to Cosmos DB
//! 5. **Checkpoint**: Update watermarks after successful batches
//! 6. **Verify** (optional): Validate data integrity with checksums
//! 7. **Report**: Generate export summary
//!
//! # Example
//!
//! ```rust,no_run
//! use atlas::config::load_config;
//! use atlas::core::export::ExportCoordinator;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Load configuration
//! let config = load_config("atlas.toml")?;
//!
//! // Create shutdown signal
//! let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
//!
//! // Create export coordinator
//! let coordinator = ExportCoordinator::new(config, shutdown_rx).await?;
//!
//! // Execute export
//! let summary = coordinator.execute_export().await?;
//!
//! println!("Total: {}", summary.total_compositions);
//! println!("Successful: {}", summary.successful_exports);
//! println!("Failed: {}", summary.failed_exports);
//! # Ok(())
//! # }
//! ```

pub mod export;
pub mod state;
pub mod transform;
pub mod verification;

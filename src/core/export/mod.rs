//! Export orchestration and batch processing
//!
//! This module provides the core export logic for Atlas, including:
//! - Batch processing of compositions
//! - Export coordination and orchestration
//! - Summary and reporting

pub mod batch;
pub mod coordinator;
pub mod summary;

pub use batch::{BatchConfig, BatchProcessor, BatchResult};
pub use coordinator::ExportCoordinator;
pub use summary::{ExportError, ExportErrorType, ExportSummary};

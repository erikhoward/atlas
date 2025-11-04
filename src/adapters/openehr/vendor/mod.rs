//! OpenEHR vendor implementations
//!
//! This module provides vendor-specific implementations of the OpenEHR REST API.
//! The `OpenEhrVendor` trait defines the common interface, and vendor-specific
//! implementations (e.g., EHRBase) provide the concrete functionality.

pub mod ehrbase;
mod r#trait;

pub use ehrbase::EhrBaseVendor;
pub use r#trait::{CompositionMetadata, OpenEhrVendor};

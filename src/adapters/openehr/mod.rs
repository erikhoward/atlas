//! OpenEHR adapter implementation
//!
//! This module provides the integration with OpenEHR servers, including
//! vendor-specific implementations, client factory, and API models.

pub mod client;
pub mod models;
pub mod vendor;

pub use client::OpenEhrClient;
pub use models::{AqlQueryRequest, AqlQueryResponse, FlatComposition};
pub use vendor::{CompositionMetadata, EhrBaseVendor, OpenEhrVendor};

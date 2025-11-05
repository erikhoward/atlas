//! Azure Cosmos DB integration
//!
//! This module provides integration with Azure Cosmos DB for storing
//! OpenEHR compositions.

pub mod adapter;
pub mod bulk;
pub mod client;
pub mod models;

pub use adapter::CosmosDbAdapter;
pub use bulk::{
    bulk_insert_compositions, bulk_insert_compositions_flattened, upsert_composition,
    upsert_composition_flattened, BulkInsertFailure, BulkInsertResult,
};
pub use client::CosmosDbClient;
pub use models::{AtlasMetadata, CosmosComposition, CosmosCompositionFlattened};

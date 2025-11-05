//! Database abstraction layer
//!
//! This module provides a trait-based abstraction for database operations,
//! allowing Atlas to work with different database backends (CosmosDB, PostgreSQL).

pub mod traits;

pub use traits::{DatabaseClient, StateStorage};


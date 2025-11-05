//! Database abstraction layer
//!
//! This module provides a trait-based abstraction for database operations,
//! allowing Atlas to work with different database backends (CosmosDB, PostgreSQL).

pub mod factory;
pub mod traits;

pub use factory::{create_database_and_state, create_database_client, create_state_storage};
pub use traits::{DatabaseClient, StateStorage};

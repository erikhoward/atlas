//! PostgreSQL database integration
//!
//! This module provides integration with PostgreSQL for storing
//! openEHR compositions.

pub mod adapter;
pub mod client;
pub mod models;

pub use adapter::PostgreSQLAdapter;
pub use client::PostgreSQLClient;
pub use models::{PostgreSQLComposition, PostgreSQLWatermark};

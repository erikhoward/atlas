//! Data verification for post-export validation
//!
//! This module provides functionality for verifying exported data integrity
//! by checking that compositions exist in the database.

pub mod report;
pub mod verify;

pub use report::{VerificationFailure, VerificationReport};
pub use verify::Verifier;

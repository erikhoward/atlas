//! Data verification and checksum validation
//!
//! This module provides functionality for verifying exported data integrity
//! through checksum calculation and validation.

pub mod checksum;
pub mod report;
pub mod verify;

pub use checksum::calculate_checksum;
pub use report::{VerificationFailure, VerificationReport};
pub use verify::Verifier;

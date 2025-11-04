//! Domain models and types for Atlas.
//!
//! This module contains the core domain models, types, and business rules for Atlas.
//! All types follow the Microsoft Rust Guidelines (TR-6.1 - TR-6.10) for type safety,
//! error handling, and API design.
//!
//! # Overview
//!
//! The domain layer provides:
//! - **Strongly-typed identifiers** ([`EhrId`], [`CompositionUid`], [`TemplateId`])
//! - **Domain models** ([`Composition`], [`Ehr`], [`Template`])
//! - **Error types** ([`AtlasError`], [`OpenEhrError`], [`CosmosDbError`])
//! - **Result type alias** ([`Result`])
//!
//! # Type Safety
//!
//! Atlas uses the newtype pattern for identifiers to prevent mixing different ID types:
//!
//! ```rust
//! use atlas::domain::{EhrId, CompositionUid};
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let ehr_id = EhrId::new("ehr-123")?;
//! let composition_uid = CompositionUid::new("composition-456::server.com::1")?;
//!
//! // This won't compile - type safety prevents mixing IDs
//! // let wrong: EhrId = composition_uid;  // Compile error!
//! # Ok(())
//! # }
//! ```
//!
//! # Error Handling
//!
//! All fallible operations return [`Result<T, AtlasError>`]:
//!
//! ```rust
//! use atlas::domain::{AtlasError, Result};
//!
//! fn example() -> Result<()> {
//!     // Errors are automatically converted using the ? operator
//!     let config = atlas::config::AtlasConfig::from_file("atlas.toml")?;
//!     Ok(())
//! }
//! ```
//!
//! # Builder Pattern
//!
//! Complex domain models use the builder pattern for construction:
//!
//! ```rust
//! use atlas::domain::{CompositionBuilder, EhrId, TemplateId};
//! use chrono::Utc;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let composition = CompositionBuilder::new()
//!     .uid("composition-123::server.com::1")?
//!     .ehr_id("ehr-456")?
//!     .template_id("IDCR - Vital Signs.v1")?
//!     .time_committed(Utc::now())
//!     .content(serde_json::json!({"vital_signs": {}}))
//!     .build()?;
//! # Ok(())
//! # }
//! ```

pub mod composition;
pub mod ehr;
pub mod errors;
pub mod ids;
pub mod result;
pub mod template;

// Re-export commonly used types for convenience
pub use composition::{Composition, CompositionBuilder, CompositionMetadata};
pub use ehr::{Ehr, EhrBuilder};
pub use errors::{AtlasError, CosmosDbError, ExportErrorDetail, OpenEhrError};
pub use ids::{CompositionUid, EhrId, TemplateId};
pub use result::Result;
pub use template::{Template, TemplateBuilder};

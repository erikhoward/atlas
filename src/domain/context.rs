//! Error context extension trait
//!
//! This module provides a context extension trait similar to `anyhow::Context`
//! that works with `Result<T, AtlasError>`. This allows adding rich context
//! to errors throughout the library code while maintaining type safety.
//!
//! # Examples
//!
//! ```rust
//! use atlas::domain::{AtlasError, Result};
//! use atlas::domain::context::ResultExt;
//!
//! fn read_file(path: &str) -> Result<String> {
//!     std::fs::read_to_string(path)
//!         .context(format!("Failed to read file: {}", path))
//! }
//!
//! fn process_data(id: &str) -> Result<()> {
//!     fetch_data(id)
//!         .with_context(|| format!("Failed to process data for ID: {}", id))?;
//!     Ok(())
//! }
//! # fn fetch_data(id: &str) -> Result<()> { Ok(()) }
//! ```

use crate::domain::errors::AtlasError;
use crate::domain::result::Result;

/// Extension trait for adding context to `Result` types
///
/// This trait provides `.context()` and `.with_context()` methods
/// for adding contextual information to errors, similar to `anyhow::Context`.
///
/// The key difference from anyhow is that this maintains the `AtlasError` type
/// throughout the library code, ensuring type safety and domain-specific errors.
pub trait ResultExt<T> {
    /// Add context to an error
    ///
    /// This method adds contextual information to an error. The context
    /// is evaluated eagerly, so use `.with_context()` if the context
    /// string is expensive to compute.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atlas::domain::{AtlasError, Result};
    /// use atlas::domain::context::ResultExt;
    ///
    /// fn load_config(path: &str) -> Result<String> {
    ///     std::fs::read_to_string(path)
    ///         .context(format!("Failed to load configuration from: {}", path))
    /// }
    /// ```
    fn context<C>(self, context: C) -> Result<T>
    where
        C: std::fmt::Display + Send + Sync + 'static;

    /// Add context to an error using a closure (lazy evaluation)
    ///
    /// This method is similar to `.context()` but the context is computed
    /// lazily only if an error occurs. This is more efficient when the
    /// context string is expensive to compute.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atlas::domain::{AtlasError, Result};
    /// use atlas::domain::context::ResultExt;
    ///
    /// fn fetch_composition(uid: &str, ehr_id: &str) -> Result<String> {
    ///     make_request(uid)
    ///         .with_context(|| format!(
    ///             "Failed to fetch composition {} from EHR {}",
    ///             uid, ehr_id
    ///         ))
    /// }
    /// # fn make_request(uid: &str) -> Result<String> { Ok(String::new()) }
    /// ```
    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: std::fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

/// Implementation for `Result<T, E>` where `E` can be converted to `AtlasError`
///
/// This allows `.context()` and `.with_context()` to work with any error type
/// that implements `Into<AtlasError>`, including `AtlasError` itself and
/// all the specialized error types like `OpenEhrError` and `CosmosDbError`.
impl<T, E> ResultExt<T> for std::result::Result<T, E>
where
    E: Into<AtlasError>,
{
    fn context<C>(self, context: C) -> Result<T>
    where
        C: std::fmt::Display + Send + Sync + 'static,
    {
        self.map_err(|e| {
            let base_error = e.into();
            // Wrap the error with context using the Other variant
            // This preserves the original error message and adds context
            AtlasError::Other(format!("{context}: {base_error}"))
        })
    }

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: std::fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(|e| {
            let base_error = e.into();
            // Lazy evaluation: only call f() if there's an error
            let context = f();
            AtlasError::Other(format!("{context}: {base_error}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::errors::{CosmosDbError, OpenEhrError};

    #[test]
    fn test_context_with_atlas_error() {
        let result: Result<()> = Err(AtlasError::Configuration("Invalid config".to_string()));
        let with_context = result.context("Failed to load configuration");

        assert!(with_context.is_err());
        let err_msg = with_context.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to load configuration"));
        assert!(err_msg.contains("Invalid config"));
    }

    #[test]
    fn test_with_context_lazy_evaluation() {
        let expensive_context_called =
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let expensive_context_called_clone = expensive_context_called.clone();

        let result: Result<i32> = Ok(42);
        let with_context = result.with_context(|| {
            expensive_context_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            "Expensive context"
        });

        // Context should NOT be evaluated for Ok results
        assert!(with_context.is_ok());
        assert!(!expensive_context_called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_with_context_error_evaluation() {
        let expensive_context_called =
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let expensive_context_called_clone = expensive_context_called.clone();

        let result: Result<()> = Err(AtlasError::Export("Export failed".to_string()));
        let with_context = result.with_context(|| {
            expensive_context_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            "Processing batch 5"
        });

        // Context SHOULD be evaluated for Err results
        assert!(with_context.is_err());
        assert!(expensive_context_called.load(std::sync::atomic::Ordering::SeqCst));

        let err_msg = with_context.unwrap_err().to_string();
        assert!(err_msg.contains("Processing batch 5"));
        assert!(err_msg.contains("Export failed"));
    }

    #[test]
    fn test_context_with_openehr_error() {
        let result: Result<()> =
            Err(OpenEhrError::ConnectionFailed("Network timeout".to_string()).into());
        let with_context = result.context("Failed to fetch composition abc-123");

        assert!(with_context.is_err());
        let err_msg = with_context.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to fetch composition abc-123"));
        assert!(err_msg.contains("Network timeout"));
    }

    #[test]
    fn test_context_with_cosmosdb_error() {
        let result: Result<()> = Err(CosmosDbError::Throttled("5 seconds".to_string()).into());
        let with_context =
            result.context("Failed to insert document into container 'compositions'");

        assert!(with_context.is_err());
        let err_msg = with_context.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to insert document"));
        assert!(err_msg.contains("Request rate too large") || err_msg.contains("429"));
    }

    #[test]
    fn test_context_chaining() {
        let result: Result<()> = Err(AtlasError::Database("Connection failed".to_string()));
        let with_context = result
            .context("Failed to execute query")
            .context("Failed to fetch watermark");

        assert!(with_context.is_err());
        let err_msg = with_context.unwrap_err().to_string();
        // Both contexts should be present
        assert!(err_msg.contains("Failed to fetch watermark"));
        assert!(err_msg.contains("Failed to execute query"));
        assert!(err_msg.contains("Connection failed"));
    }

    #[test]
    fn test_io_error_with_context() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let result: Result<()> = Err(io_error.into());
        let with_context = result.context("Failed to read configuration file 'atlas.toml'");

        assert!(with_context.is_err());
        let err_msg = with_context.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to read configuration file"));
        assert!(err_msg.contains("File not found"));
    }
}

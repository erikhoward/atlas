//! Domain error types
//!
//! This module defines the error hierarchy for Atlas following TR-6.4, TR-6.5, and TR-6.6.
//! All errors are domain-specific and don't expose third-party types.

use thiserror::Error;

/// Main Atlas error type
///
/// This is the primary error type used throughout the application.
/// It wraps specific error types and provides context for error handling.
#[derive(Debug, Error)]
pub enum AtlasError {
    /// Configuration-related errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// OpenEHR-related errors
    #[error("OpenEHR error: {0}")]
    OpenEhr(#[from] OpenEhrError),

    /// Cosmos DB-related errors
    #[error("Cosmos DB error: {0}")]
    CosmosDb(#[from] CosmosDbError),

    /// Database-related errors (generic)
    #[error("Database error: {0}")]
    Database(String),

    /// Export process errors
    #[error("Export error: {0}")]
    Export(String),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// Authentication errors
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Network/connection errors
    #[error("Connection error: {0}")]
    Connection(String),

    /// State management errors
    #[error("State management error: {0}")]
    State(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(String),

    /// Azure logging errors
    #[error("Azure logging error: {0}")]
    AzureLogging(String),

    /// Generic errors with context
    #[error("{0}")]
    Other(String),
}

/// OpenEHR-specific errors
///
/// Errors that occur when interacting with OpenEHR servers.
/// These errors don't expose third-party HTTP client types (TR-6.6).
#[derive(Debug, Error)]
pub enum OpenEhrError {
    /// Failed to connect to OpenEHR server
    #[error("Failed to connect to OpenEHR server: {0}")]
    ConnectionFailed(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Invalid response from server
    #[error("Invalid response from server: {0}")]
    InvalidResponse(String),

    /// Composition not found
    #[error("Composition not found: {0}")]
    CompositionNotFound(String),

    /// EHR not found
    #[error("EHR not found: {0}")]
    EhrNotFound(String),

    /// Template not found
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    /// API version not supported
    #[error("API version not supported: {0}")]
    UnsupportedApiVersion(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded, retry after: {0}")]
    RateLimitExceeded(String),

    /// Query failed
    #[error("Query failed: {0}")]
    QueryFailed(String),

    /// Server error (5xx)
    #[error("Server error: {status} - {message}")]
    ServerError { status: u16, message: String },

    /// Client error (4xx)
    #[error("Client error: {status} - {message}")]
    ClientError { status: u16, message: String },

    /// Timeout
    #[error("Request timeout: {0}")]
    Timeout(String),

    /// Invalid data format
    #[error("Invalid data format: {0}")]
    InvalidFormat(String),
}

/// Cosmos DB-specific errors
///
/// Errors that occur when interacting with Azure Cosmos DB.
/// These errors don't expose third-party SDK types (TR-6.6).
#[derive(Debug, Error)]
pub enum CosmosDbError {
    /// Failed to connect to Cosmos DB
    #[error("Failed to connect to Cosmos DB: {0}")]
    ConnectionFailed(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Database not found
    #[error("Database not found: {0}")]
    DatabaseNotFound(String),

    /// Container not found
    #[error("Container not found: {0}")]
    ContainerNotFound(String),

    /// Document not found
    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    /// Failed to create database
    #[error("Failed to create database: {0}")]
    DatabaseCreationFailed(String),

    /// Failed to create container
    #[error("Failed to create container: {0}")]
    ContainerCreationFailed(String),

    /// Failed to insert document
    #[error("Failed to insert document: {0}")]
    InsertFailed(String),

    /// Failed to update document
    #[error("Failed to update document: {0}")]
    UpdateFailed(String),

    /// Failed to query documents
    #[error("Failed to query documents: {0}")]
    QueryFailed(String),

    /// Throttling error (429)
    #[error("Request rate too large (429), retry after: {0}")]
    Throttled(String),

    /// Conflict error (409)
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Partition key mismatch
    #[error("Partition key mismatch: {0}")]
    PartitionKeyMismatch(String),

    /// Bulk operation failed
    #[error("Bulk operation failed: {successful}/{total} succeeded")]
    BulkOperationFailed { successful: usize, total: usize },

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Timeout
    #[error("Request timeout: {0}")]
    Timeout(String),

    /// Failed to write document
    #[error("Failed to write document: {0}")]
    WriteFailed(String),

    /// Failed to deserialize response
    #[error("Failed to deserialize response: {0}")]
    DeserializationFailed(String),
}

/// Export-specific error details
///
/// Provides additional context for export failures
#[derive(Debug, Clone)]
pub struct ExportErrorDetail {
    /// EHR ID associated with the error
    pub ehr_id: Option<String>,

    /// Composition UID associated with the error
    pub composition_uid: Option<String>,

    /// Template ID associated with the error
    pub template_id: Option<String>,

    /// Error message
    pub message: String,

    /// Whether the error is retryable
    pub retryable: bool,
}

impl ExportErrorDetail {
    /// Creates a new export error detail
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            ehr_id: None,
            composition_uid: None,
            template_id: None,
            message: message.into(),
            retryable: false,
        }
    }

    /// Sets the EHR ID
    pub fn with_ehr_id(mut self, ehr_id: impl Into<String>) -> Self {
        self.ehr_id = Some(ehr_id.into());
        self
    }

    /// Sets the composition UID
    pub fn with_composition_uid(mut self, uid: impl Into<String>) -> Self {
        self.composition_uid = Some(uid.into());
        self
    }

    /// Sets the template ID
    pub fn with_template_id(mut self, template_id: impl Into<String>) -> Self {
        self.template_id = Some(template_id.into());
        self
    }

    /// Marks the error as retryable
    pub fn retryable(mut self) -> Self {
        self.retryable = true;
        self
    }
}

// Conversion from std::io::Error
impl From<std::io::Error> for AtlasError {
    fn from(err: std::io::Error) -> Self {
        AtlasError::Io(err.to_string())
    }
}

// Conversion from serde_json::Error
impl From<serde_json::Error> for AtlasError {
    fn from(err: serde_json::Error) -> Self {
        AtlasError::Serialization(err.to_string())
    }
}

// Conversion from toml parse errors
impl From<toml::de::Error> for AtlasError {
    fn from(err: toml::de::Error) -> Self {
        AtlasError::Configuration(format!("TOML parse error: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atlas_error_display() {
        let err = AtlasError::Configuration("Invalid config".to_string());
        assert_eq!(err.to_string(), "Configuration error: Invalid config");
    }

    #[test]
    fn test_openehr_error_conversion() {
        let openehr_err = OpenEhrError::ConnectionFailed("Network error".to_string());
        let atlas_err: AtlasError = openehr_err.into();
        assert!(matches!(atlas_err, AtlasError::OpenEhr(_)));
    }

    #[test]
    fn test_cosmosdb_error_conversion() {
        let cosmos_err = CosmosDbError::Throttled("5 seconds".to_string());
        let atlas_err: AtlasError = cosmos_err.into();
        assert!(matches!(atlas_err, AtlasError::CosmosDb(_)));
    }

    #[test]
    fn test_export_error_detail_builder() {
        let detail = ExportErrorDetail::new("Test error")
            .with_ehr_id("ehr-123")
            .with_composition_uid("comp-456")
            .with_template_id("template-789")
            .retryable();

        assert_eq!(detail.ehr_id, Some("ehr-123".to_string()));
        assert_eq!(detail.composition_uid, Some("comp-456".to_string()));
        assert_eq!(detail.template_id, Some("template-789".to_string()));
        assert!(detail.retryable);
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let atlas_err: AtlasError = io_err.into();
        assert!(matches!(atlas_err, AtlasError::Io(_)));
    }

    #[test]
    fn test_serde_json_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let atlas_err: AtlasError = json_err.into();
        assert!(matches!(atlas_err, AtlasError::Serialization(_)));
    }

    #[test]
    fn test_toml_error_conversion() {
        let toml_err = toml::from_str::<toml::Value>("invalid = toml = syntax").unwrap_err();
        let atlas_err: AtlasError = toml_err.into();
        assert!(matches!(atlas_err, AtlasError::Configuration(_)));
        assert!(atlas_err.to_string().contains("TOML parse error"));
    }

    #[test]
    fn test_atlas_error_implements_std_error() {
        let err = AtlasError::Validation("Test error".to_string());
        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_openehr_error_implements_std_error() {
        let err = OpenEhrError::ConnectionFailed("Test error".to_string());
        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_cosmosdb_error_implements_std_error() {
        let err = CosmosDbError::Throttled("5 seconds".to_string());
        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &err;
    }
}

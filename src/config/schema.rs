//! Configuration schema types
//!
//! This module defines the configuration structure for Atlas following TR-4.1.

use crate::config::SecretString;
use serde::{Deserialize, Serialize};

/// Database target selection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseTarget {
    /// PostgreSQL database
    PostgreSQL,
    /// Azure Cosmos DB
    CosmosDB,
}

/// Runtime environment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// Development environment
    #[default]
    Development,
    /// Staging environment
    Staging,
    /// Production environment
    Production,
}

/// Main Atlas configuration
///
/// This is the root configuration structure that maps to the TOML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasConfig {
    /// Application-level settings
    pub application: ApplicationConfig,

    /// Runtime environment (development, staging, production)
    #[serde(default)]
    pub environment: Environment,

    /// OpenEHR server configuration
    pub openehr: OpenEhrConfig,

    /// Export settings
    pub export: ExportConfig,

    /// Database target (postgresql or cosmosdb)
    pub database_target: DatabaseTarget,

    /// Azure Cosmos DB configuration (required if database_target = cosmosdb)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cosmosdb: Option<CosmosDbConfig>,

    /// PostgreSQL configuration (required if database_target = postgresql)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postgresql: Option<PostgreSQLConfig>,

    /// State management configuration
    pub state: StateConfig,

    /// Data verification configuration
    #[serde(default)]
    pub verification: VerificationConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
}

impl AtlasConfig {
    /// Validates the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if any configuration values are invalid
    pub fn validate(&self) -> Result<(), String> {
        self.application.validate()?;
        self.openehr.validate(&self.environment)?;
        self.export.validate()?;

        // Validate that the correct database config is present and valid
        // Note: Both database configurations can be present in the TOML file for 12-factor app compliance,
        // but only the active one (based on database_target) is validated
        match self.database_target {
            DatabaseTarget::CosmosDB => {
                if let Some(ref config) = self.cosmosdb {
                    config.validate()?;
                } else {
                    return Err(
                        "cosmosdb configuration is required when database_target = 'cosmosdb'"
                            .to_string(),
                    );
                }
            }
            DatabaseTarget::PostgreSQL => {
                if let Some(ref config) = self.postgresql {
                    config.validate()?;
                } else {
                    return Err(
                        "postgresql configuration is required when database_target = 'postgresql'"
                            .to_string(),
                    );
                }
            }
        }

        self.state.validate()?;
        self.verification.validate()?;
        self.logging.validate()?;
        Ok(())
    }
}

/// Application-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Dry run mode (don't write to Cosmos DB)
    #[serde(default)]
    pub dry_run: bool,
}

impl ApplicationConfig {
    fn validate(&self) -> Result<(), String> {
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.log_level.as_str()) {
            return Err(format!(
                "Invalid log_level '{}'. Must be one of: {}",
                self.log_level,
                valid_levels.join(", ")
            ));
        }
        Ok(())
    }
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,

    /// Initial delay in milliseconds
    #[serde(default = "default_initial_delay_ms")]
    pub initial_delay_ms: u64,

    /// Maximum delay in milliseconds
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,

    /// Backoff multiplier
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            initial_delay_ms: default_initial_delay_ms(),
            max_delay_ms: default_max_delay_ms(),
            backoff_multiplier: default_backoff_multiplier(),
        }
    }
}

/// OpenEHR server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenEhrConfig {
    /// Base URL of the OpenEHR server
    pub base_url: String,

    /// Vendor implementation (e.g., "ehrbase")
    #[serde(default = "default_vendor")]
    pub vendor: String,

    /// Vendor type (alias for vendor, for compatibility)
    #[serde(default = "default_vendor")]
    pub vendor_type: String,

    /// Authentication type
    #[serde(default = "default_auth_type")]
    pub auth_type: String,

    /// Username for authentication (optional)
    #[serde(default)]
    pub username: Option<String>,

    /// Password for authentication (optional)
    /// Stored securely in memory and automatically zeroized on drop
    #[serde(default)]
    pub password: Option<SecretString>,

    /// TLS certificate verification enabled
    ///
    /// **SECURITY WARNING**: Disabling TLS verification (setting to `false`) exposes the application
    /// to man-in-the-middle attacks and should ONLY be used in development/testing environments.
    ///
    /// - In **production** environments, this MUST be set to `true` (enforced by validation)
    /// - For self-signed certificates, use `tls_ca_cert` to specify a custom CA certificate
    /// - Default: `true`
    #[serde(default = "default_true")]
    pub tls_verify: bool,

    /// TLS certificate verification (alias for tls_verify)
    ///
    /// **SECURITY WARNING**: Disabling TLS verification (setting to `false`) exposes the application
    /// to man-in-the-middle attacks and should ONLY be used in development/testing environments.
    ///
    /// - In **production** environments, this MUST be set to `true` (enforced by validation)
    /// - For self-signed certificates, use `tls_ca_cert` to specify a custom CA certificate
    /// - Default: `true`
    #[serde(default = "default_true")]
    pub tls_verify_certificates: bool,

    /// Timeout in seconds
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,

    /// Optional TLS CA certificate path for custom/self-signed certificates
    ///
    /// Use this to specify a custom CA certificate file when connecting to OpenEHR servers
    /// with self-signed certificates or certificates from a private CA. This is the recommended
    /// approach for production environments instead of disabling TLS verification.
    #[serde(default)]
    pub tls_ca_cert: Option<String>,

    /// Retry configuration
    #[serde(default)]
    pub retry: RetryConfig,

    /// Query configuration
    pub query: QueryConfig,
}

impl OpenEhrConfig {
    fn validate(&self, environment: &Environment) -> Result<(), String> {
        use secrecy::ExposeSecret;

        if self.base_url.is_empty() {
            return Err("openehr.base_url cannot be empty".to_string());
        }

        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err("openehr.base_url must start with http:// or https://".to_string());
        }

        // Validate username and password if auth_type is basic
        if self.auth_type == "basic" {
            if self.username.is_none()
                || self.username.as_ref().map(|s| s.is_empty()).unwrap_or(true)
            {
                return Err(
                    "openehr.username cannot be empty when auth_type is 'basic'".to_string()
                );
            }

            if self.password.is_none()
                || self
                    .password
                    .as_ref()
                    .map(|s| s.expose_secret().is_empty())
                    .unwrap_or(true)
            {
                return Err(
                    "openehr.password cannot be empty when auth_type is 'basic'".to_string()
                );
            }
        }

        let valid_auth_types = ["basic", "openid"];
        if !valid_auth_types.contains(&self.auth_type.as_str()) {
            return Err(format!(
                "Invalid auth_type '{}'. Must be one of: {}",
                self.auth_type,
                valid_auth_types.join(", ")
            ));
        }

        // Security: Enforce TLS verification in production environments
        // Disabling TLS verification exposes the application to man-in-the-middle attacks
        if *environment == Environment::Production
            && (!self.tls_verify || !self.tls_verify_certificates)
        {
            return Err(
                "TLS certificate verification cannot be disabled in production environments. \
                This is a critical security requirement to prevent man-in-the-middle attacks. \
                Either set 'tls_verify = true' or provide a custom CA certificate using 'tls_ca_cert'. \
                For development/testing environments, set 'environment = \"development\"' or 'environment = \"staging\"'.".to_string()
            );
        }

        self.query.validate()?;
        Ok(())
    }
}

impl Default for OpenEhrConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080/ehrbase".to_string(),
            vendor: default_vendor(),
            vendor_type: default_vendor(),
            auth_type: default_auth_type(),
            username: None,
            password: None,
            tls_verify: true,
            tls_verify_certificates: true,
            timeout_seconds: default_timeout_seconds(),
            tls_ca_cert: None,
            retry: RetryConfig::default(),
            query: QueryConfig::default(),
        }
    }
}

/// Query configuration for OpenEHR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryConfig {
    /// Template IDs to export
    pub template_ids: Vec<String>,

    /// EHR IDs to export (empty = all)
    #[serde(default)]
    pub ehr_ids: Vec<String>,

    /// Start of time range (ISO 8601)
    #[serde(default)]
    pub time_range_start: Option<String>,

    /// End of time range (ISO 8601)
    #[serde(default)]
    pub time_range_end: Option<String>,

    /// Batch size for processing
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Number of parallel EHR processors
    #[serde(default = "default_parallel_ehrs")]
    pub parallel_ehrs: usize,
}

impl QueryConfig {
    fn validate(&self) -> Result<(), String> {
        if self.template_ids.is_empty() {
            return Err("openehr.query.template_ids cannot be empty".to_string());
        }

        if !(100..=5000).contains(&self.batch_size) {
            return Err(format!(
                "openehr.query.batch_size must be between 100 and 5000, got {}",
                self.batch_size
            ));
        }

        if self.parallel_ehrs == 0 || self.parallel_ehrs > 100 {
            return Err(format!(
                "openehr.query.parallel_ehrs must be between 1 and 100, got {}",
                self.parallel_ehrs
            ));
        }

        Ok(())
    }
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            template_ids: vec![],
            ehr_ids: vec![],
            time_range_start: None,
            time_range_end: None,
            batch_size: default_batch_size(),
            parallel_ehrs: default_parallel_ehrs(),
        }
    }
}

/// Export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Export mode (full or incremental)
    #[serde(default = "default_export_mode")]
    pub mode: String,

    /// Composition format for export (preserve or flatten)
    #[serde(default = "default_composition_format")]
    pub export_composition_format: String,

    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,

    /// Retry backoff intervals in milliseconds
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: Vec<u64>,

    /// Graceful shutdown timeout in seconds (default: 30)
    /// This is the maximum time to wait for the current batch to complete
    /// before forcing shutdown. Should align with container orchestration
    /// grace periods (e.g., Kubernetes default is 30s).
    #[serde(default = "default_shutdown_timeout_secs")]
    pub shutdown_timeout_secs: u64,

    /// Dry run mode - simulate export without writing to database (default: false)
    /// When enabled, all database write operations are skipped but the export
    /// process runs normally. Useful for testing configuration and previewing
    /// what would be exported without modifying data.
    #[serde(default)]
    pub dry_run: bool,
}

impl ExportConfig {
    fn validate(&self) -> Result<(), String> {
        let valid_modes = ["full", "incremental"];
        if !valid_modes.contains(&self.mode.as_str()) {
            return Err(format!(
                "Invalid export.mode '{}'. Must be one of: {}",
                self.mode,
                valid_modes.join(", ")
            ));
        }

        let valid_formats = ["preserve", "flatten"];
        if !valid_formats.contains(&self.export_composition_format.as_str()) {
            return Err(format!(
                "Invalid export.export_composition_format '{}'. Must be one of: {}",
                self.export_composition_format,
                valid_formats.join(", ")
            ));
        }

        if self.max_retries > 10 {
            return Err(format!(
                "export.max_retries must be <= 10, got {}",
                self.max_retries
            ));
        }

        Ok(())
    }
}

/// Azure Cosmos DB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosDbConfig {
    /// Cosmos DB endpoint URL
    pub endpoint: String,

    /// Cosmos DB access key
    /// Stored securely in memory and automatically zeroized on drop
    pub key: SecretString,

    /// Database name
    pub database_name: String,

    /// Control container name
    #[serde(default = "default_control_container")]
    pub control_container: String,

    /// Data container prefix
    #[serde(default = "default_data_container_prefix")]
    pub data_container_prefix: String,

    /// Partition key path
    #[serde(default = "default_partition_key")]
    pub partition_key: String,

    /// Maximum concurrency for operations
    #[serde(default = "default_max_concurrency")]
    pub max_concurrency: usize,

    /// Request timeout in seconds
    #[serde(default = "default_request_timeout_seconds")]
    pub request_timeout_seconds: u64,
}

impl CosmosDbConfig {
    fn validate(&self) -> Result<(), String> {
        use secrecy::ExposeSecret;

        if self.endpoint.is_empty() {
            return Err("cosmosdb.endpoint cannot be empty".to_string());
        }

        if !self.endpoint.starts_with("https://") {
            return Err("cosmosdb.endpoint must start with https://".to_string());
        }

        if self.key.expose_secret().is_empty() {
            return Err("cosmosdb.key cannot be empty".to_string());
        }

        if self.database_name.is_empty() {
            return Err("cosmosdb.database_name cannot be empty".to_string());
        }

        if self.max_concurrency == 0 || self.max_concurrency > 100 {
            return Err(format!(
                "cosmosdb.max_concurrency must be between 1 and 100, got {}",
                self.max_concurrency
            ));
        }

        Ok(())
    }
}

/// PostgreSQL database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgreSQLConfig {
    /// PostgreSQL connection string
    /// Format: postgresql://user:password@host:port/database
    /// Stored securely in memory and automatically zeroized on drop
    pub connection_string: SecretString,

    /// Maximum number of connections in the pool
    #[serde(default = "default_pg_max_connections")]
    pub max_connections: usize,

    /// Connection timeout in seconds
    #[serde(default = "default_pg_connection_timeout_seconds")]
    pub connection_timeout_seconds: u64,

    /// Statement timeout in seconds
    #[serde(default = "default_pg_statement_timeout_seconds")]
    pub statement_timeout_seconds: u64,

    /// Enable SSL/TLS for connections
    #[serde(default = "default_pg_ssl_mode")]
    pub ssl_mode: String,
}

impl PostgreSQLConfig {
    fn validate(&self) -> Result<(), String> {
        use secrecy::ExposeSecret;

        let conn_str = self.connection_string.expose_secret();

        if conn_str.is_empty() {
            return Err("postgresql.connection_string cannot be empty".to_string());
        }

        if !conn_str.starts_with("postgresql://") && !conn_str.starts_with("postgres://") {
            return Err(
                "postgresql.connection_string must start with postgresql:// or postgres://"
                    .to_string(),
            );
        }

        if self.max_connections == 0 || self.max_connections > 100 {
            return Err(format!(
                "postgresql.max_connections must be between 1 and 100, got {}",
                self.max_connections
            ));
        }

        let valid_ssl_modes = [
            "disable",
            "allow",
            "prefer",
            "require",
            "verify-ca",
            "verify-full",
        ];
        if !valid_ssl_modes.contains(&self.ssl_mode.as_str()) {
            return Err(format!(
                "postgresql.ssl_mode must be one of: {}, got '{}'",
                valid_ssl_modes.join(", "),
                self.ssl_mode
            ));
        }

        Ok(())
    }
}

/// State management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConfig {
    /// Enable checkpointing
    #[serde(default = "default_true")]
    pub enable_checkpointing: bool,

    /// Checkpoint interval in seconds
    #[serde(default = "default_checkpoint_interval_seconds")]
    pub checkpoint_interval_seconds: u64,
}

impl StateConfig {
    fn validate(&self) -> Result<(), String> {
        if self.checkpoint_interval_seconds == 0 {
            return Err("state.checkpoint_interval_seconds must be > 0".to_string());
        }
        Ok(())
    }
}

/// Data verification configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VerificationConfig {
    /// Enable verification
    #[serde(default)]
    pub enable_verification: bool,
}

impl VerificationConfig {
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Enable local file logging
    #[serde(default = "default_true")]
    pub local_enabled: bool,

    /// Local log file path
    #[serde(default = "default_local_path")]
    pub local_path: String,

    /// Log rotation strategy
    #[serde(default = "default_local_rotation")]
    pub local_rotation: String,

    /// Maximum log file size in MB
    #[serde(default = "default_local_max_size_mb")]
    pub local_max_size_mb: usize,

    /// Enable Azure Log Analytics
    #[serde(default)]
    pub azure_enabled: bool,

    /// Azure AD tenant ID
    #[serde(default)]
    pub azure_tenant_id: Option<String>,

    /// Azure AD client ID (from App Registration)
    #[serde(default)]
    pub azure_client_id: Option<String>,

    /// Azure AD client secret (from App Registration)
    /// Stored securely in memory and automatically zeroized on drop
    #[serde(default)]
    pub azure_client_secret: Option<SecretString>,

    /// Log Analytics workspace ID
    #[serde(default)]
    pub azure_log_analytics_workspace_id: Option<String>,

    /// Data Collection Rule (DCR) immutable ID
    #[serde(default)]
    pub azure_dcr_immutable_id: Option<String>,

    /// Data Collection Endpoint (DCE) URL
    #[serde(default)]
    pub azure_dce_endpoint: Option<String>,

    /// Stream name for custom logs (e.g., "Custom-AtlasExport_CL")
    #[serde(default)]
    pub azure_stream_name: Option<String>,
}

impl LoggingConfig {
    fn validate(&self) -> Result<(), String> {
        let valid_rotations = ["daily", "size"];
        if !valid_rotations.contains(&self.local_rotation.as_str()) {
            return Err(format!(
                "Invalid logging.local_rotation '{}'. Must be one of: {}",
                self.local_rotation,
                valid_rotations.join(", ")
            ));
        }

        if self.local_max_size_mb == 0 {
            return Err("logging.local_max_size_mb must be > 0".to_string());
        }

        if self.azure_enabled {
            // Validate Azure AD credentials
            if self.azure_tenant_id.is_none() {
                return Err("Azure logging enabled but azure_tenant_id not provided".to_string());
            }
            if self.azure_client_id.is_none() {
                return Err("Azure logging enabled but azure_client_id not provided".to_string());
            }
            if self.azure_client_secret.is_none() {
                return Err(
                    "Azure logging enabled but azure_client_secret not provided".to_string()
                );
            }

            // Validate Log Analytics configuration
            if self.azure_log_analytics_workspace_id.is_none() {
                return Err(
                    "Azure logging enabled but azure_log_analytics_workspace_id not provided"
                        .to_string(),
                );
            }
            if self.azure_dcr_immutable_id.is_none() {
                return Err(
                    "Azure logging enabled but azure_dcr_immutable_id not provided".to_string(),
                );
            }
            if self.azure_dce_endpoint.is_none() {
                return Err("Azure logging enabled but azure_dce_endpoint not provided".to_string());
            }
            if self.azure_stream_name.is_none() {
                return Err("Azure logging enabled but azure_stream_name not provided".to_string());
            }
        }

        Ok(())
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            local_enabled: true,
            local_path: default_local_path(),
            local_rotation: default_local_rotation(),
            local_max_size_mb: default_local_max_size_mb(),
            azure_enabled: false,
            azure_tenant_id: None,
            azure_client_id: None,
            azure_client_secret: None,
            azure_log_analytics_workspace_id: None,
            azure_dcr_immutable_id: None,
            azure_dce_endpoint: None,
            azure_stream_name: None,
        }
    }
}

// Default value functions
fn default_log_level() -> String {
    "info".to_string()
}

fn default_vendor() -> String {
    "ehrbase".to_string()
}

fn default_auth_type() -> String {
    "basic".to_string()
}

fn default_true() -> bool {
    true
}

fn default_timeout_seconds() -> u64 {
    60
}

fn default_initial_delay_ms() -> u64 {
    1000
}

fn default_max_delay_ms() -> u64 {
    30000
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

fn default_batch_size() -> usize {
    1000
}

fn default_parallel_ehrs() -> usize {
    8
}

fn default_export_mode() -> String {
    "incremental".to_string()
}

fn default_composition_format() -> String {
    "preserve".to_string()
}

fn default_max_retries() -> usize {
    3
}

fn default_retry_backoff_ms() -> Vec<u64> {
    vec![1000, 2000, 4000]
}

fn default_shutdown_timeout_secs() -> u64 {
    30
}

fn default_control_container() -> String {
    "atlas_control".to_string()
}

fn default_data_container_prefix() -> String {
    "compositions".to_string()
}

fn default_partition_key() -> String {
    "/ehr_id".to_string()
}

fn default_max_concurrency() -> usize {
    10
}

fn default_request_timeout_seconds() -> u64 {
    60
}

fn default_checkpoint_interval_seconds() -> u64 {
    30
}

fn default_local_path() -> String {
    "/var/log/atlas".to_string()
}

fn default_local_rotation() -> String {
    "daily".to_string()
}

fn default_local_max_size_mb() -> usize {
    100
}

fn default_pg_max_connections() -> usize {
    10
}

fn default_pg_connection_timeout_seconds() -> u64 {
    30
}

fn default_pg_statement_timeout_seconds() -> u64 {
    60
}

fn default_pg_ssl_mode() -> String {
    "prefer".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::secret::SecretValue;
    use secrecy::Secret;

    #[test]
    fn test_application_config_validation() {
        let mut config = ApplicationConfig {
            log_level: "info".to_string(),
            dry_run: false,
        };

        assert!(config.validate().is_ok());

        config.log_level = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_query_config_validation() {
        let mut config = QueryConfig {
            template_ids: vec!["template1".to_string()],
            ehr_ids: vec![],
            time_range_start: None,
            time_range_end: None,
            batch_size: 1000,
            parallel_ehrs: 8,
        };

        assert!(config.validate().is_ok());

        // Test empty template_ids
        config.template_ids = vec![];
        assert!(config.validate().is_err());

        // Test invalid batch_size
        config.template_ids = vec!["template1".to_string()];
        config.batch_size = 50;
        assert!(config.validate().is_err());

        config.batch_size = 6000;
        assert!(config.validate().is_err());

        // Test invalid parallel_ehrs
        config.batch_size = 1000;
        config.parallel_ehrs = 0;
        assert!(config.validate().is_err());

        config.parallel_ehrs = 101;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_openehr_config_validation() {
        let config = OpenEhrConfig {
            base_url: "https://ehrbase.example.com".to_string(),
            vendor: "ehrbase".to_string(),
            vendor_type: "ehrbase".to_string(),
            auth_type: "basic".to_string(),
            username: Some("user".to_string()),
            password: Some(Secret::new(SecretValue::from("pass".to_string()))),
            tls_verify: true,
            tls_verify_certificates: true,
            timeout_seconds: 60,
            tls_ca_cert: None,
            retry: RetryConfig::default(),
            query: QueryConfig {
                template_ids: vec!["template1".to_string()],
                ehr_ids: vec![],
                time_range_start: None,
                time_range_end: None,
                batch_size: 1000,
                parallel_ehrs: 8,
            },
        };

        // Test with development environment
        assert!(config.validate(&Environment::Development).is_ok());

        // Test with production environment
        assert!(config.validate(&Environment::Production).is_ok());
    }

    #[test]
    fn test_openehr_tls_verification_in_production() {
        // Test that TLS verification cannot be disabled in production
        let mut config = OpenEhrConfig {
            base_url: "https://ehrbase.example.com".to_string(),
            vendor: "ehrbase".to_string(),
            vendor_type: "ehrbase".to_string(),
            auth_type: "basic".to_string(),
            username: Some("user".to_string()),
            password: Some(Secret::new(SecretValue::from("pass".to_string()))),
            tls_verify: false, // Disabled
            tls_verify_certificates: true,
            timeout_seconds: 60,
            tls_ca_cert: None,
            retry: RetryConfig::default(),
            query: QueryConfig {
                template_ids: vec!["template1".to_string()],
                ehr_ids: vec![],
                time_range_start: None,
                time_range_end: None,
                batch_size: 1000,
                parallel_ehrs: 8,
            },
        };

        // Should fail in production environment
        let result = config.validate(&Environment::Production);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("TLS certificate verification cannot be disabled in production"));

        // Should succeed in development environment
        assert!(config.validate(&Environment::Development).is_ok());

        // Should succeed in staging environment
        assert!(config.validate(&Environment::Staging).is_ok());

        // Test with tls_verify_certificates disabled
        config.tls_verify = true;
        config.tls_verify_certificates = false;

        // Should fail in production
        let result = config.validate(&Environment::Production);
        assert!(result.is_err());

        // Should succeed in development
        assert!(config.validate(&Environment::Development).is_ok());
    }

    #[test]
    fn test_openehr_tls_verification_both_disabled() {
        // Test that validation fails when both TLS flags are disabled in production
        let config = OpenEhrConfig {
            base_url: "https://ehrbase.example.com".to_string(),
            vendor: "ehrbase".to_string(),
            vendor_type: "ehrbase".to_string(),
            auth_type: "basic".to_string(),
            username: Some("user".to_string()),
            password: Some(Secret::new(SecretValue::from("pass".to_string()))),
            tls_verify: false,
            tls_verify_certificates: false,
            timeout_seconds: 60,
            tls_ca_cert: None,
            retry: RetryConfig::default(),
            query: QueryConfig {
                template_ids: vec!["template1".to_string()],
                ehr_ids: vec![],
                time_range_start: None,
                time_range_end: None,
                batch_size: 1000,
                parallel_ehrs: 8,
            },
        };

        // Should fail in production
        assert!(config.validate(&Environment::Production).is_err());

        // Should succeed in development
        assert!(config.validate(&Environment::Development).is_ok());
    }

    #[test]
    fn test_export_config_validation() {
        let mut config = ExportConfig {
            mode: "incremental".to_string(),
            export_composition_format: "preserve".to_string(),
            max_retries: 3,
            retry_backoff_ms: vec![1000, 2000, 4000],
            shutdown_timeout_secs: 30,
            dry_run: false,
        };

        assert!(config.validate().is_ok());

        config.mode = "invalid".to_string();
        assert!(config.validate().is_err());

        config.mode = "full".to_string();
        config.export_composition_format = "invalid".to_string();
        assert!(config.validate().is_err());

        config.export_composition_format = "flatten".to_string();
        config.max_retries = 11;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_cosmosdb_config_validation() {
        let config = CosmosDbConfig {
            endpoint: "https://myaccount.documents.azure.com:443/".to_string(),
            key: Secret::new(SecretValue::from("test-key".to_string())),
            database_name: "openehr_data".to_string(),
            control_container: "atlas_control".to_string(),
            data_container_prefix: "compositions".to_string(),
            partition_key: "/ehr_id".to_string(),
            max_concurrency: 10,
            request_timeout_seconds: 60,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_verification_config_default() {
        let config = VerificationConfig::default();
        assert!(!config.enable_verification);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert!(config.local_enabled);
        assert_eq!(config.local_path, "/var/log/atlas");
        assert_eq!(config.local_rotation, "daily");
        assert_eq!(config.local_max_size_mb, 100);
        assert!(!config.azure_enabled);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_default_values() {
        assert_eq!(default_log_level(), "info");
        assert_eq!(default_vendor(), "ehrbase");
        assert_eq!(default_auth_type(), "basic");
        assert_eq!(default_batch_size(), 1000);
        assert_eq!(default_parallel_ehrs(), 8);
        assert_eq!(default_export_mode(), "incremental");
        assert_eq!(default_composition_format(), "preserve");
        assert_eq!(default_max_retries(), 3);
        assert_eq!(default_retry_backoff_ms(), vec![1000, 2000, 4000]);
    }
}

//! Configuration loader with TOML parsing and environment variable overrides
//!
//! This module implements configuration loading following TR-4.2 and 12-factor app principles.
//!
//! # Environment Variable Support
//!
//! Atlas supports comprehensive environment variable overrides for all configuration options,
//! enabling 12-factor app compliance and containerized deployments.
//!
//! ## Two Types of Environment Variable Support
//!
//! ### 1. Substitution Syntax (`${VAR}`)
//!
//! Use `${VAR_NAME}` in TOML files for environment variable substitution:
//!
//! ```toml
//! [openehr]
//! password = "${OPENEHR_PASSWORD}"
//!
//! [cosmosdb]
//! key = "${COSMOS_KEY}"
//! ```
//!
//! ### 2. Override Syntax (`ATLAS_*`)
//!
//! Use `ATLAS_<SECTION>_<KEY>` environment variables to override any configuration value:
//!
//! ```bash
//! # Database selection
//! ATLAS_DATABASE_TARGET=postgresql
//!
//! # Application settings
//! ATLAS_APPLICATION_LOG_LEVEL=debug
//! ATLAS_APPLICATION_DRY_RUN=true
//!
//! # OpenEHR connection
//! ATLAS_OPENEHR_BASE_URL=https://prod-ehrbase.com
//! ATLAS_OPENEHR_USERNAME=atlas_user
//! ATLAS_OPENEHR_PASSWORD=secret
//!
//! # Query settings
//! ATLAS_OPENEHR_QUERY_BATCH_SIZE=2000
//! ATLAS_OPENEHR_QUERY_TEMPLATE_IDS='["IDCR - Vital Signs.v1","IDCR - Lab Report.v1"]'
//!
//! # Export settings
//! ATLAS_EXPORT_MODE=full
//! ATLAS_EXPORT_DRY_RUN=false
//!
//! # PostgreSQL
//! ATLAS_POSTGRESQL_CONNECTION_STRING="postgresql://user:pass@localhost/db"
//! ATLAS_POSTGRESQL_MAX_CONNECTIONS=20
//!
//! # Cosmos DB
//! ATLAS_COSMOSDB_ENDPOINT=https://myaccount.documents.azure.com:443/
//! ATLAS_COSMOSDB_KEY=secret-key
//! ```
//!
//! ## Array Format Support
//!
//! Array fields support both JSON and comma-separated formats:
//!
//! ```bash
//! # JSON format (recommended for complex values)
//! ATLAS_OPENEHR_QUERY_TEMPLATE_IDS='["IDCR - Vital Signs.v1","IDCR - Lab Report.v1"]'
//!
//! # Comma-separated format (simpler)
//! ATLAS_OPENEHR_QUERY_EHR_IDS="ehr-123,ehr-456,ehr-789"
//!
//! # Numeric arrays
//! ATLAS_EXPORT_RETRY_BACKOFF_MS="1000,2000,4000"
//! ATLAS_EXPORT_RETRY_BACKOFF_MS='[1000,2000,4000]'
//! ```
//!
//! Empty string clears the array:
//! ```bash
//! ATLAS_OPENEHR_QUERY_EHR_IDS=""  # Clears the EHR IDs list
//! ```
//!
//! See the `apply_env_overrides` function documentation for a complete list of supported variables.

use super::schema::AtlasConfig;
use crate::domain::errors::AtlasError;
use crate::domain::result::Result;
use regex::Regex;
use std::fs;
use std::path::Path;

/// Loads configuration from a TOML file
///
/// This function:
/// 1. Reads the TOML file
/// 2. Performs environment variable substitution (${VAR} syntax)
/// 3. Parses the TOML into AtlasConfig
/// 4. Applies environment variable overrides (ATLAS_* prefix)
/// 5. Validates the configuration
///
/// # Arguments
///
/// * `path` - Path to the TOML configuration file
///
/// # Errors
///
/// Returns an error if:
/// - File cannot be read
/// - TOML parsing fails
/// - Environment variable substitution fails
/// - Configuration validation fails
///
/// # Examples
///
/// ```no_run
/// use atlas::config::loader::load_config;
///
/// let config = load_config("atlas.toml").expect("Failed to load config");
/// ```
pub fn load_config(path: impl AsRef<Path>) -> Result<AtlasConfig> {
    let path = path.as_ref();

    // Check if file exists
    if !path.exists() {
        return Err(AtlasError::Configuration(format!(
            "Configuration file not found: {}",
            path.display()
        )));
    }

    // Read file contents
    let contents = fs::read_to_string(path).map_err(|e| {
        AtlasError::Configuration(format!(
            "Failed to read configuration file {}: {}",
            path.display(),
            e
        ))
    })?;

    // Perform environment variable substitution
    let contents = substitute_env_vars(&contents)?;

    // Parse TOML
    let mut config: AtlasConfig = toml::from_str(&contents)
        .map_err(|e| AtlasError::Configuration(format!("Failed to parse TOML: {e}")))?;

    // Apply environment variable overrides
    apply_env_overrides(&mut config)?;

    // Validate configuration
    config
        .validate()
        .map_err(|e| AtlasError::Configuration(format!("Configuration validation failed: {e}")))?;

    Ok(config)
}

/// Substitutes environment variables in the format ${VAR_NAME}
///
/// # Arguments
///
/// * `input` - String containing ${VAR} placeholders
///
/// # Errors
///
/// Returns an error if a referenced environment variable is not set
fn substitute_env_vars(input: &str) -> Result<String> {
    let re = Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").unwrap();
    let mut result = String::new();
    let mut missing_vars = Vec::new();

    // Process line by line to skip comments
    for line in input.lines() {
        let trimmed = line.trim_start();

        // Skip comment lines - don't process env vars in comments
        if trimmed.starts_with('#') {
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Process non-comment lines for env var substitution
        let mut processed_line = line.to_string();
        for cap in re.captures_iter(line) {
            let var_name = &cap[1];
            match std::env::var(var_name) {
                Ok(value) => {
                    let placeholder = format!("${{{var_name}}}");
                    processed_line = processed_line.replace(&placeholder, &value);
                }
                Err(_) => {
                    if !missing_vars.contains(&var_name.to_string()) {
                        missing_vars.push(var_name.to_string());
                    }
                }
            }
        }
        result.push_str(&processed_line);
        result.push('\n');
    }

    if !missing_vars.is_empty() {
        return Err(AtlasError::Configuration(format!(
            "Missing required environment variables: {}",
            missing_vars.join(", ")
        )));
    }

    // Remove trailing newline if present
    if result.ends_with('\n') {
        result.pop();
    }

    Ok(result)
}

/// Parses a string array from environment variable
///
/// Supports two formats:
/// 1. JSON array: `["item1", "item2"]`
/// 2. Comma-separated: `item1,item2`
///
/// Empty string returns an empty vector.
fn parse_string_array(value: &str) -> Vec<String> {
    let trimmed = value.trim();

    // Empty string clears the array
    if trimmed.is_empty() {
        return vec![];
    }

    // Try JSON format first
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(trimmed) {
        return arr;
    }

    // Fallback to comma-separated
    trimmed
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parses a u64 array from environment variable
///
/// Supports two formats:
/// 1. JSON array: `[1000, 2000, 4000]`
/// 2. Comma-separated: `1000,2000,4000`
///
/// Empty string returns an empty vector.
fn parse_u64_array(value: &str) -> Option<Vec<u64>> {
    let trimmed = value.trim();

    // Empty string clears the array
    if trimmed.is_empty() {
        return Some(vec![]);
    }

    // Try JSON format first
    if let Ok(arr) = serde_json::from_str::<Vec<u64>>(trimmed) {
        return Some(arr);
    }

    // Fallback to comma-separated
    let result: std::result::Result<Vec<u64>, _> = trimmed
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<u64>())
        .collect();

    result.ok()
}

/// Applies environment variable overrides using ATLAS_* prefix
///
/// Environment variables follow the pattern: ATLAS_<SECTION>_<KEY>
/// For example: ATLAS_OPENEHR_BASE_URL, ATLAS_EXPORT_MODE
///
/// Supported environment variables:
/// - ATLAS_ENVIRONMENT: Runtime environment (development, staging, production)
/// - ATLAS_DATABASE_TARGET: Database target (cosmosdb or postgresql)
/// - ATLAS_APPLICATION_LOG_LEVEL: Log level
/// - ATLAS_APPLICATION_DRY_RUN: Dry run mode (true/false)
/// - ATLAS_OPENEHR_BASE_URL: OpenEHR server base URL
/// - ATLAS_OPENEHR_USERNAME: OpenEHR username
/// - ATLAS_OPENEHR_PASSWORD: OpenEHR password
/// - ATLAS_OPENEHR_VENDOR: OpenEHR vendor
/// - ATLAS_OPENEHR_AUTH_TYPE: Authentication type
/// - ATLAS_OPENEHR_TLS_VERIFY: TLS verification (true/false)
/// - ATLAS_OPENEHR_TLS_VERIFY_CERTIFICATES: TLS certificate verification (true/false)
/// - ATLAS_OPENEHR_TLS_CA_CERT: TLS CA certificate path
/// - ATLAS_OPENEHR_TIMEOUT_SECONDS: Request timeout in seconds
/// - ATLAS_OPENEHR_RETRY_MAX_RETRIES: Maximum retry attempts
/// - ATLAS_OPENEHR_RETRY_INITIAL_DELAY_MS: Initial retry delay in milliseconds
/// - ATLAS_OPENEHR_RETRY_MAX_DELAY_MS: Maximum retry delay in milliseconds
/// - ATLAS_OPENEHR_RETRY_BACKOFF_MULTIPLIER: Retry backoff multiplier
/// - ATLAS_OPENEHR_QUERY_TEMPLATE_IDS: Template IDs (JSON array or comma-separated)
/// - ATLAS_OPENEHR_QUERY_EHR_IDS: EHR IDs (JSON array or comma-separated)
/// - ATLAS_OPENEHR_QUERY_TIME_RANGE_START: Query time range start (ISO 8601)
/// - ATLAS_OPENEHR_QUERY_TIME_RANGE_END: Query time range end (ISO 8601)
/// - ATLAS_OPENEHR_QUERY_BATCH_SIZE: Query batch size
/// - ATLAS_OPENEHR_QUERY_PARALLEL_EHRS: Parallel EHR processing count
/// - ATLAS_EXPORT_MODE: Export mode (full or incremental)
/// - ATLAS_EXPORT_COMPOSITION_FORMAT: Composition format (preserve or flatten)
/// - ATLAS_EXPORT_MAX_RETRIES: Maximum export retries
/// - ATLAS_EXPORT_RETRY_BACKOFF_MS: Retry backoff delays (JSON array or comma-separated)
/// - ATLAS_EXPORT_SHUTDOWN_TIMEOUT_SECS: Shutdown timeout in seconds
/// - ATLAS_EXPORT_DRY_RUN: Export dry run mode (true/false)
/// - ATLAS_COSMOSDB_ENDPOINT: Cosmos DB endpoint URL
/// - ATLAS_COSMOSDB_KEY: Cosmos DB access key
/// - ATLAS_COSMOSDB_DATABASE_NAME: Cosmos DB database name
/// - ATLAS_COSMOSDB_CONTROL_CONTAINER: Cosmos DB control container name
/// - ATLAS_COSMOSDB_DATA_CONTAINER_PREFIX: Cosmos DB data container prefix
/// - ATLAS_COSMOSDB_PARTITION_KEY: Cosmos DB partition key
/// - ATLAS_COSMOSDB_MAX_CONCURRENCY: Cosmos DB max concurrency
/// - ATLAS_COSMOSDB_REQUEST_TIMEOUT_SECONDS: Cosmos DB request timeout
/// - ATLAS_POSTGRESQL_CONNECTION_STRING: PostgreSQL connection string
/// - ATLAS_POSTGRESQL_MAX_CONNECTIONS: PostgreSQL max connections
/// - ATLAS_POSTGRESQL_CONNECTION_TIMEOUT_SECONDS: PostgreSQL connection timeout
/// - ATLAS_POSTGRESQL_STATEMENT_TIMEOUT_SECONDS: PostgreSQL statement timeout
/// - ATLAS_POSTGRESQL_SSL_MODE: PostgreSQL SSL mode
/// - ATLAS_STATE_BACKEND: State backend (file or database)
/// - ATLAS_STATE_FILE_PATH: State file path
/// - ATLAS_STATE_ENABLE_CHECKPOINTING: Enable checkpointing (true/false)
/// - ATLAS_STATE_CHECKPOINT_INTERVAL_SECONDS: Checkpoint interval in seconds
/// - ATLAS_VERIFICATION_ENABLE_VERIFICATION: Enable verification (true/false)
/// - ATLAS_LOGGING_LOCAL_ENABLED: Enable local logging (true/false)
/// - ATLAS_LOGGING_LOCAL_PATH: Local log file path
/// - ATLAS_LOGGING_LOCAL_ROTATION: Log rotation strategy (daily or size)
/// - ATLAS_LOGGING_LOCAL_MAX_SIZE_MB: Maximum log file size in MB
/// - ATLAS_LOGGING_AZURE_ENABLED: Enable Azure logging (true/false)
/// - ATLAS_LOGGING_AZURE_TENANT_ID: Azure tenant ID
/// - ATLAS_LOGGING_AZURE_CLIENT_ID: Azure client ID
/// - ATLAS_LOGGING_AZURE_CLIENT_SECRET: Azure client secret
/// - ATLAS_LOGGING_AZURE_LOG_ANALYTICS_WORKSPACE_ID: Log Analytics workspace ID
/// - ATLAS_LOGGING_AZURE_DCR_IMMUTABLE_ID: Data Collection Rule immutable ID
/// - ATLAS_LOGGING_AZURE_DCE_ENDPOINT: Data Collection Endpoint URL
/// - ATLAS_LOGGING_AZURE_STREAM_NAME: Azure stream name
///
/// # Arguments
///
/// * `config` - Mutable reference to the configuration to update
///
/// # Errors
///
/// Returns an error if critical environment variable values are invalid
fn apply_env_overrides(config: &mut AtlasConfig) -> Result<()> {
    use crate::config::schema::{DatabaseTarget, Environment};

    // Environment override
    if let Ok(val) = std::env::var("ATLAS_ENVIRONMENT") {
        match val.to_lowercase().as_str() {
            "development" => config.environment = Environment::Development,
            "staging" => config.environment = Environment::Staging,
            "production" => config.environment = Environment::Production,
            _ => {
                return Err(AtlasError::Configuration(format!(
                    "Invalid ATLAS_ENVIRONMENT value '{val}'. Must be 'development', 'staging', or 'production'"
                )));
            }
        }
    }

    // Database target override (must be first)
    if let Ok(val) = std::env::var("ATLAS_DATABASE_TARGET") {
        match val.to_lowercase().as_str() {
            "cosmosdb" => config.database_target = DatabaseTarget::CosmosDB,
            "postgresql" => config.database_target = DatabaseTarget::PostgreSQL,
            _ => {
                return Err(AtlasError::Configuration(format!(
                    "Invalid ATLAS_DATABASE_TARGET value '{val}'. Must be 'cosmosdb' or 'postgresql'"
                )));
            }
        }
    }
    // Application overrides
    if let Ok(val) = std::env::var("ATLAS_APPLICATION_LOG_LEVEL") {
        config.application.log_level = val;
    }
    if let Ok(val) = std::env::var("ATLAS_APPLICATION_DRY_RUN") {
        config.application.dry_run = val.parse().unwrap_or(false);
    }

    // OpenEHR overrides
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_BASE_URL") {
        config.openehr.base_url = val;
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_USERNAME") {
        config.openehr.username = Some(val);
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_PASSWORD") {
        use crate::config::secret::SecretValue;
        use secrecy::Secret;
        config.openehr.password = Some(Secret::new(SecretValue::from(val)));
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_VENDOR") {
        config.openehr.vendor = val.clone();
        config.openehr.vendor_type = val;
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_AUTH_TYPE") {
        config.openehr.auth_type = val;
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_TLS_VERIFY") {
        config.openehr.tls_verify = val.parse().unwrap_or(true);
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_TLS_VERIFY_CERTIFICATES") {
        config.openehr.tls_verify_certificates = val.parse().unwrap_or(true);
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_TLS_CA_CERT") {
        config.openehr.tls_ca_cert = Some(val);
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_OIDC_TOKEN_URL") {
        config.openehr.oidc_token_url = Some(val);
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_CLIENT_ID") {
        config.openehr.client_id = Some(val);
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_TIMEOUT_SECONDS") {
        if let Ok(timeout) = val.parse() {
            config.openehr.timeout_seconds = timeout;
        }
    }

    // OpenEHR Retry overrides
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_RETRY_MAX_RETRIES") {
        if let Ok(retries) = val.parse() {
            config.openehr.retry.max_retries = retries;
        }
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_RETRY_INITIAL_DELAY_MS") {
        if let Ok(delay) = val.parse() {
            config.openehr.retry.initial_delay_ms = delay;
        }
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_RETRY_MAX_DELAY_MS") {
        if let Ok(delay) = val.parse() {
            config.openehr.retry.max_delay_ms = delay;
        }
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_RETRY_BACKOFF_MULTIPLIER") {
        if let Ok(multiplier) = val.parse() {
            config.openehr.retry.backoff_multiplier = multiplier;
        }
    }

    // Query overrides
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_QUERY_TEMPLATE_IDS") {
        config.openehr.query.template_ids = parse_string_array(&val);
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_QUERY_EHR_IDS") {
        config.openehr.query.ehr_ids = parse_string_array(&val);
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_QUERY_TIME_RANGE_START") {
        config.openehr.query.time_range_start = Some(val);
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_QUERY_TIME_RANGE_END") {
        config.openehr.query.time_range_end = Some(val);
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_QUERY_BATCH_SIZE") {
        if let Ok(size) = val.parse() {
            config.openehr.query.batch_size = size;
        }
    }
    if let Ok(val) = std::env::var("ATLAS_OPENEHR_QUERY_PARALLEL_EHRS") {
        if let Ok(parallel) = val.parse() {
            config.openehr.query.parallel_ehrs = parallel;
        }
    }

    // Export overrides
    if let Ok(val) = std::env::var("ATLAS_EXPORT_MODE") {
        config.export.mode = val;
    }
    if let Ok(val) = std::env::var("ATLAS_EXPORT_COMPOSITION_FORMAT") {
        config.export.export_composition_format = val;
    }
    if let Ok(val) = std::env::var("ATLAS_EXPORT_MAX_RETRIES") {
        if let Ok(retries) = val.parse() {
            config.export.max_retries = retries;
        }
    }
    if let Ok(val) = std::env::var("ATLAS_EXPORT_RETRY_BACKOFF_MS") {
        if let Some(backoff) = parse_u64_array(&val) {
            config.export.retry_backoff_ms = backoff;
        } else {
            eprintln!("Warning: Invalid ATLAS_EXPORT_RETRY_BACKOFF_MS value '{val}', keeping existing value");
        }
    }
    if let Ok(val) = std::env::var("ATLAS_EXPORT_SHUTDOWN_TIMEOUT_SECS") {
        if let Ok(timeout) = val.parse() {
            config.export.shutdown_timeout_secs = timeout;
        }
    }
    if let Ok(val) = std::env::var("ATLAS_EXPORT_DRY_RUN") {
        config.export.dry_run = val.parse().unwrap_or(false);
    }

    // Cosmos DB overrides (only if CosmosDB is configured)
    if let Some(ref mut cosmos_config) = config.cosmosdb {
        if let Ok(val) = std::env::var("ATLAS_COSMOSDB_ENDPOINT") {
            cosmos_config.endpoint = val;
        }
        if let Ok(val) = std::env::var("ATLAS_COSMOSDB_KEY") {
            use crate::config::secret::SecretValue;
            use secrecy::Secret;
            cosmos_config.key = Secret::new(SecretValue::from(val));
        }
        if let Ok(val) = std::env::var("ATLAS_COSMOSDB_DATABASE_NAME") {
            cosmos_config.database_name = val;
        }
        if let Ok(val) = std::env::var("ATLAS_COSMOSDB_CONTROL_CONTAINER") {
            cosmos_config.control_container = val;
        }
        if let Ok(val) = std::env::var("ATLAS_COSMOSDB_DATA_CONTAINER_PREFIX") {
            cosmos_config.data_container_prefix = val;
        }
        if let Ok(val) = std::env::var("ATLAS_COSMOSDB_PARTITION_KEY") {
            cosmos_config.partition_key = val;
        }
        if let Ok(val) = std::env::var("ATLAS_COSMOSDB_MAX_CONCURRENCY") {
            if let Ok(concurrency) = val.parse() {
                cosmos_config.max_concurrency = concurrency;
            }
        }
        if let Ok(val) = std::env::var("ATLAS_COSMOSDB_REQUEST_TIMEOUT_SECONDS") {
            if let Ok(timeout) = val.parse() {
                cosmos_config.request_timeout_seconds = timeout;
            }
        }
    }

    // PostgreSQL overrides (only if PostgreSQL is configured)
    if let Some(ref mut pg_config) = config.postgresql {
        if let Ok(val) = std::env::var("ATLAS_POSTGRESQL_CONNECTION_STRING") {
            use crate::config::secret::SecretValue;
            use secrecy::Secret;
            pg_config.connection_string = Secret::new(SecretValue::from(val));
        }
        if let Ok(val) = std::env::var("ATLAS_POSTGRESQL_MAX_CONNECTIONS") {
            if let Ok(max_conn) = val.parse() {
                pg_config.max_connections = max_conn;
            }
        }
        if let Ok(val) = std::env::var("ATLAS_POSTGRESQL_CONNECTION_TIMEOUT_SECONDS") {
            if let Ok(timeout) = val.parse() {
                pg_config.connection_timeout_seconds = timeout;
            }
        }
        if let Ok(val) = std::env::var("ATLAS_POSTGRESQL_STATEMENT_TIMEOUT_SECONDS") {
            if let Ok(timeout) = val.parse() {
                pg_config.statement_timeout_seconds = timeout;
            }
        }
        if let Ok(val) = std::env::var("ATLAS_POSTGRESQL_SSL_MODE") {
            pg_config.ssl_mode = val;
        }
    }

    // State overrides
    if let Ok(val) = std::env::var("ATLAS_STATE_ENABLE_CHECKPOINTING") {
        config.state.enable_checkpointing = val.parse().unwrap_or(true);
    }
    if let Ok(val) = std::env::var("ATLAS_STATE_CHECKPOINT_INTERVAL_SECONDS") {
        if let Ok(interval) = val.parse() {
            config.state.checkpoint_interval_seconds = interval;
        }
    }

    // Verification overrides
    if let Ok(val) = std::env::var("ATLAS_VERIFICATION_ENABLE_VERIFICATION") {
        config.verification.enable_verification = val.parse().unwrap_or(false);
    }

    // Logging overrides
    if let Ok(val) = std::env::var("ATLAS_LOGGING_LOCAL_ENABLED") {
        config.logging.local_enabled = val.parse().unwrap_or(true);
    }
    if let Ok(val) = std::env::var("ATLAS_LOGGING_LOCAL_PATH") {
        config.logging.local_path = val;
    }
    if let Ok(val) = std::env::var("ATLAS_LOGGING_LOCAL_ROTATION") {
        config.logging.local_rotation = val;
    }
    if let Ok(val) = std::env::var("ATLAS_LOGGING_LOCAL_MAX_SIZE_MB") {
        if let Ok(size) = val.parse() {
            config.logging.local_max_size_mb = size;
        }
    }
    if let Ok(val) = std::env::var("ATLAS_LOGGING_AZURE_ENABLED") {
        config.logging.azure_enabled = val.parse().unwrap_or(false);
    }
    if let Ok(val) = std::env::var("ATLAS_LOGGING_AZURE_TENANT_ID") {
        config.logging.azure_tenant_id = Some(val);
    }
    if let Ok(val) = std::env::var("ATLAS_LOGGING_AZURE_CLIENT_ID") {
        config.logging.azure_client_id = Some(val);
    }
    if let Ok(val) = std::env::var("ATLAS_LOGGING_AZURE_CLIENT_SECRET") {
        use crate::config::secret::SecretValue;
        use secrecy::Secret;
        config.logging.azure_client_secret = Some(Secret::new(SecretValue::from(val)));
    }
    if let Ok(val) = std::env::var("ATLAS_LOGGING_AZURE_LOG_ANALYTICS_WORKSPACE_ID") {
        config.logging.azure_log_analytics_workspace_id = Some(val);
    }
    if let Ok(val) = std::env::var("ATLAS_LOGGING_AZURE_DCR_IMMUTABLE_ID") {
        config.logging.azure_dcr_immutable_id = Some(val);
    }
    if let Ok(val) = std::env::var("ATLAS_LOGGING_AZURE_DCE_ENDPOINT") {
        config.logging.azure_dce_endpoint = Some(val);
    }
    if let Ok(val) = std::env::var("ATLAS_LOGGING_AZURE_STREAM_NAME") {
        config.logging.azure_stream_name = Some(val);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::DatabaseTarget;
    use std::io::Write;
    use std::sync::Mutex;
    use tempfile::NamedTempFile;

    // Mutex to serialize tests that modify environment variables
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_substitute_env_vars() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::set_var("TEST_VAR", "test_value");
        let input = "password = \"${TEST_VAR}\"";
        let result = substitute_env_vars(input).unwrap();
        assert_eq!(result, "password = \"test_value\"");
        std::env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_substitute_env_vars_missing() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("MISSING_VAR");
        let input = "password = \"${MISSING_VAR}\"";
        let result = substitute_env_vars(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_missing_file() {
        let result = load_config("nonexistent.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_valid() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // Clean up any environment variables from other tests
        std::env::remove_var("ATLAS_APPLICATION_NAME");
        std::env::remove_var("ATLAS_DATABASE_TARGET");
        std::env::remove_var("ATLAS_ENVIRONMENT");
        std::env::remove_var("ATLAS_OPENEHR_BASE_URL");
        std::env::remove_var("ATLAS_OPENEHR_USERNAME");
        std::env::remove_var("ATLAS_OPENEHR_PASSWORD");

        let toml_content = r#"database_target = "cosmosdb"
environment = "development"

[application]
log_level = "info"

[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"

[openehr.query]
template_ids = ["template1"]

[export]
mode = "incremental"

[cosmosdb]
endpoint = "https://test.documents.azure.com:443/"
key = "test-key"
database_name = "test_db"

[state]
enable_checkpointing = true
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = load_config(temp_file.path());
        if let Err(e) = &result {
            eprintln!("Config load error: {e}");
        }
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.openehr.base_url, "https://ehrbase.example.com");
    }

    #[test]
    fn test_parse_string_array_json() {
        let result = parse_string_array(r#"["item1","item2","item3"]"#);
        assert_eq!(result, vec!["item1", "item2", "item3"]);
    }

    #[test]
    fn test_parse_string_array_csv() {
        let result = parse_string_array("item1,item2,item3");
        assert_eq!(result, vec!["item1", "item2", "item3"]);
    }

    #[test]
    fn test_parse_string_array_csv_with_spaces() {
        let result = parse_string_array("item1, item2 , item3");
        assert_eq!(result, vec!["item1", "item2", "item3"]);
    }

    #[test]
    fn test_parse_string_array_empty_string() {
        let result = parse_string_array("");
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_parse_string_array_whitespace() {
        let result = parse_string_array("   ");
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_parse_u64_array_json() {
        let result = parse_u64_array("[1000,2000,4000]");
        assert_eq!(result, Some(vec![1000, 2000, 4000]));
    }

    #[test]
    fn test_parse_u64_array_csv() {
        let result = parse_u64_array("1000,2000,4000");
        assert_eq!(result, Some(vec![1000, 2000, 4000]));
    }

    #[test]
    fn test_parse_u64_array_empty_string() {
        let result = parse_u64_array("");
        assert_eq!(result, Some(vec![]));
    }

    #[test]
    fn test_parse_u64_array_invalid() {
        let result = parse_u64_array("not,a,number");
        assert_eq!(result, None);
    }

    #[test]
    fn test_env_override_database_target() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // Clean up any environment variables from other tests
        std::env::remove_var("ATLAS_APPLICATION_NAME");
        std::env::remove_var("ATLAS_APPLICATION_VERSION");
        std::env::remove_var("ATLAS_APPLICATION_LOG_LEVEL");
        std::env::remove_var("ATLAS_APPLICATION_DRY_RUN");
        std::env::remove_var("ATLAS_ENVIRONMENT");

        std::env::set_var("ATLAS_DATABASE_TARGET", "postgresql");

        let toml_content = r#"database_target = "cosmosdb"
environment = "development"
[application]
[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"
[openehr.query]
template_ids = ["template1"]
[export]
mode = "incremental"
[postgresql]
connection_string = "postgresql://localhost/test"
[state]
enable_checkpointing = true
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = load_config(temp_file.path());
        if let Err(e) = &result {
            eprintln!("Config load error: {e}");
        }
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.database_target, DatabaseTarget::PostgreSQL);

        std::env::remove_var("ATLAS_DATABASE_TARGET");
    }

    #[test]
    fn test_env_override_application_fields() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // Clean up any environment variables from other tests
        std::env::remove_var("ATLAS_DATABASE_TARGET");
        std::env::remove_var("ATLAS_ENVIRONMENT");
        std::env::remove_var("ATLAS_APPLICATION_LOG_LEVEL");
        std::env::remove_var("ATLAS_APPLICATION_DRY_RUN");

        std::env::set_var("ATLAS_APPLICATION_LOG_LEVEL", "debug");
        std::env::set_var("ATLAS_APPLICATION_DRY_RUN", "true");

        let toml_content = r#"database_target = "cosmosdb"
environment = "development"
[application]
log_level = "info"
[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"
[openehr.query]
template_ids = ["template1"]
[export]
mode = "incremental"
[cosmosdb]
endpoint = "https://test.documents.azure.com:443/"
key = "test-key"
database_name = "test_db"
[state]
enable_checkpointing = true
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = load_config(temp_file.path());
        if let Err(e) = &result {
            eprintln!("Config load error: {e}");
        }
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.application.log_level, "debug");
        assert!(config.application.dry_run);

        std::env::remove_var("ATLAS_APPLICATION_LOG_LEVEL");
        std::env::remove_var("ATLAS_APPLICATION_DRY_RUN");
    }

    #[test]
    fn test_env_override_openehr_fields() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::set_var("ATLAS_OPENEHR_BASE_URL", "https://prod-ehrbase.com");
        std::env::set_var("ATLAS_OPENEHR_USERNAME", "prod_user");
        std::env::set_var("ATLAS_OPENEHR_PASSWORD", "prod_pass");
        std::env::set_var("ATLAS_OPENEHR_TIMEOUT_SECONDS", "120");
        std::env::set_var("ATLAS_OPENEHR_TLS_VERIFY_CERTIFICATES", "false");

        let toml_content = r#"database_target = "cosmosdb"
environment = "development"
[application]
[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"
timeout_seconds = 60
[openehr.query]
template_ids = ["template1"]
[export]
mode = "incremental"
[cosmosdb]
endpoint = "https://test.documents.azure.com:443/"
key = "test-key"
database_name = "test_db"
[state]
enable_checkpointing = true
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = load_config(temp_file.path());
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.openehr.base_url, "https://prod-ehrbase.com");
        assert_eq!(config.openehr.username, Some("prod_user".to_string()));
        assert_eq!(config.openehr.timeout_seconds, 120);
        assert!(!config.openehr.tls_verify_certificates);

        std::env::remove_var("ATLAS_OPENEHR_BASE_URL");
        std::env::remove_var("ATLAS_OPENEHR_USERNAME");
        std::env::remove_var("ATLAS_OPENEHR_PASSWORD");
        std::env::remove_var("ATLAS_OPENEHR_TIMEOUT_SECONDS");
        std::env::remove_var("ATLAS_OPENEHR_TLS_VERIFY_CERTIFICATES");
    }

    #[test]
    fn test_env_override_query_arrays() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::set_var(
            "ATLAS_OPENEHR_QUERY_TEMPLATE_IDS",
            r#"["IDCR - Vital Signs.v1","IDCR - Lab Report.v1"]"#,
        );
        std::env::set_var("ATLAS_OPENEHR_QUERY_EHR_IDS", "ehr-123,ehr-456,ehr-789");
        std::env::set_var("ATLAS_OPENEHR_QUERY_BATCH_SIZE", "2000");

        let toml_content = r#"database_target = "cosmosdb"
environment = "development"
[application]
[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"
[openehr.query]
template_ids = ["template1"]
batch_size = 1000
[export]
mode = "incremental"
[cosmosdb]
endpoint = "https://test.documents.azure.com:443/"
key = "test-key"
database_name = "test_db"
[state]
enable_checkpointing = true
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = load_config(temp_file.path());
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(
            config.openehr.query.template_ids,
            vec!["IDCR - Vital Signs.v1", "IDCR - Lab Report.v1"]
        );
        assert_eq!(
            config.openehr.query.ehr_ids,
            vec![
                "ehr-123".to_string(),
                "ehr-456".to_string(),
                "ehr-789".to_string()
            ]
        );
        assert_eq!(config.openehr.query.batch_size, 2000);

        std::env::remove_var("ATLAS_OPENEHR_QUERY_TEMPLATE_IDS");
        std::env::remove_var("ATLAS_OPENEHR_QUERY_EHR_IDS");
        std::env::remove_var("ATLAS_OPENEHR_QUERY_BATCH_SIZE");
    }

    #[test]
    fn test_env_override_export_retry_backoff() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // Clean up any environment variables from other tests
        std::env::remove_var("ATLAS_DATABASE_TARGET");
        std::env::remove_var("ATLAS_ENVIRONMENT");

        std::env::set_var("ATLAS_EXPORT_RETRY_BACKOFF_MS", "1000,2000,4000,8000");
        std::env::set_var("ATLAS_EXPORT_MODE", "full");
        std::env::set_var("ATLAS_EXPORT_DRY_RUN", "true");

        let toml_content = r#"database_target = "cosmosdb"
environment = "development"
[application]
[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"
[openehr.query]
template_ids = ["template1"]
[export]
mode = "incremental"
[cosmosdb]
endpoint = "https://test.documents.azure.com:443/"
key = "test-key"
database_name = "test_db"
[state]
enable_checkpointing = true
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = load_config(temp_file.path());
        if let Err(e) = &result {
            eprintln!("Config load error: {e}");
        }
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.export.retry_backoff_ms, vec![1000, 2000, 4000, 8000]);
        assert_eq!(config.export.mode, "full");
        assert!(config.export.dry_run);

        std::env::remove_var("ATLAS_EXPORT_RETRY_BACKOFF_MS");
        std::env::remove_var("ATLAS_EXPORT_MODE");
        std::env::remove_var("ATLAS_EXPORT_DRY_RUN");
    }

    #[test]
    fn test_env_override_postgresql_fields() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::set_var(
            "ATLAS_POSTGRESQL_CONNECTION_STRING",
            "postgresql://prod:pass@prod-db:5432/openehr",
        );
        std::env::set_var("ATLAS_POSTGRESQL_MAX_CONNECTIONS", "50");
        std::env::set_var("ATLAS_POSTGRESQL_SSL_MODE", "verify-full");

        let toml_content = r#"database_target = "postgresql"
[application]
[openehr]
base_url = "https://ehrbase.example.com"
username = "user"
password = "pass"
[openehr.query]
template_ids = ["template1"]
[export]
mode = "incremental"
[postgresql]
connection_string = "postgresql://localhost/test"
max_connections = 10
[state]
enable_checkpointing = true
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = load_config(temp_file.path());
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.postgresql.as_ref().unwrap().max_connections, 50);
        assert_eq!(
            config.postgresql.as_ref().unwrap().ssl_mode,
            "verify-full".to_string()
        );

        std::env::remove_var("ATLAS_POSTGRESQL_CONNECTION_STRING");
        std::env::remove_var("ATLAS_POSTGRESQL_MAX_CONNECTIONS");
        std::env::remove_var("ATLAS_POSTGRESQL_SSL_MODE");
    }
}

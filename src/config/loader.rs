//! Configuration loader with TOML parsing and environment variable overrides
//!
//! This module implements configuration loading following TR-4.2.

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
        .map_err(|e| AtlasError::Configuration(format!("Failed to parse TOML: {}", e)))?;

    // Apply environment variable overrides
    apply_env_overrides(&mut config)?;

    // Validate configuration
    config.validate().map_err(|e| {
        AtlasError::Configuration(format!("Configuration validation failed: {}", e))
    })?;

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
                    let placeholder = format!("${{{}}}", var_name);
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

    Ok(result)
}

/// Applies environment variable overrides using ATLAS_* prefix
///
/// Environment variables follow the pattern: ATLAS_<SECTION>_<KEY>
/// For example: ATLAS_OPENEHR_BASE_URL, ATLAS_EXPORT_MODE
///
/// # Arguments
///
/// * `config` - Mutable reference to the configuration to update
fn apply_env_overrides(config: &mut AtlasConfig) -> Result<()> {
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
        config.openehr.password = Some(val);
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

    // Query overrides
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

    // Cosmos DB overrides (only if CosmosDB is configured)
    if let Some(ref mut cosmos_config) = config.cosmosdb {
        if let Ok(val) = std::env::var("ATLAS_COSMOSDB_ENDPOINT") {
            cosmos_config.endpoint = val;
        }
        if let Ok(val) = std::env::var("ATLAS_COSMOSDB_KEY") {
            cosmos_config.key = val;
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
        if let Ok(val) = std::env::var("ATLAS_COSMOSDB_MAX_CONCURRENCY") {
            if let Ok(concurrency) = val.parse() {
                cosmos_config.max_concurrency = concurrency;
            }
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
    if let Ok(val) = std::env::var("ATLAS_VERIFICATION_CHECKSUM_ALGORITHM") {
        config.verification.checksum_algorithm = val;
    }

    // Logging overrides
    if let Ok(val) = std::env::var("ATLAS_LOGGING_LOCAL_ENABLED") {
        config.logging.local_enabled = val.parse().unwrap_or(true);
    }
    if let Ok(val) = std::env::var("ATLAS_LOGGING_LOCAL_PATH") {
        config.logging.local_path = val;
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
        config.logging.azure_client_secret = Some(val);
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
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_substitute_env_vars() {
        std::env::set_var("TEST_VAR", "test_value");
        let input = "password = \"${TEST_VAR}\"";
        let result = substitute_env_vars(input).unwrap();
        assert_eq!(result, "password = \"test_value\"");
        std::env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_substitute_env_vars_missing() {
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
        let toml_content = r#"
[application]
name = "atlas"
version = "1.0.0"
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
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.application.name, "atlas");
        assert_eq!(config.openehr.base_url, "https://ehrbase.example.com");
    }
}

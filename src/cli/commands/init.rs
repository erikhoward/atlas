//! Init command implementation
//!
//! This module implements the `init` command for generating a sample
//! configuration file.

use clap::Args;
use std::fs;
use std::path::Path;

/// Arguments for the init command
#[derive(Args, Debug)]
pub struct InitArgs {
    /// Path where to create the configuration file
    #[arg(short, long, default_value = "atlas.toml")]
    pub output: String,

    /// Include example values and comments
    #[arg(long)]
    pub with_examples: bool,

    /// Overwrite existing file
    #[arg(long)]
    pub force: bool,
}

impl InitArgs {
    /// Execute the init command
    pub async fn execute(&self) -> anyhow::Result<i32> {
        tracing::info!(output = %self.output, "Initializing configuration file");

        println!("📝 Initializing Atlas configuration");
        println!();

        // Check if file already exists
        if Path::new(&self.output).exists() && !self.force {
            println!("❌ Configuration file already exists: {}", self.output);
            println!("   Use --force to overwrite");
            return Ok(2); // Configuration error exit code
        }

        // Generate configuration content
        let config_content = if self.with_examples {
            Self::generate_config_with_examples()
        } else {
            Self::generate_minimal_config()
        };

        // Write to file
        match fs::write(&self.output, config_content) {
            Ok(_) => {
                println!("✅ Configuration file created: {}", self.output);
                println!();
                println!("Next steps:");
                println!("  1. Edit {} with your settings", self.output);
                println!("  2. Set required environment variables:");
                println!("     - ATLAS_OPENEHR_USERNAME");
                println!("     - ATLAS_OPENEHR_PASSWORD");
                println!("     - ATLAS_COSMOSDB_KEY");
                println!("  3. Validate configuration: atlas validate-config");
                println!("  4. Run export: atlas export");
                println!();
                Ok(0)
            }
            Err(e) => {
                println!("❌ Failed to write configuration file");
                println!("   Error: {}", e);
                Ok(5) // Fatal error exit code
            }
        }
    }

    /// Generate minimal configuration
    fn generate_minimal_config() -> String {
        r#"# Atlas Configuration File
# OpenEHR to Azure Cosmos DB ETL Tool

[application]
name = "atlas"
version = "1.0.0"
log_level = "info"

[openehr]
base_url = "https://ehrbase.example.com"
vendor = "ehrbase"

[openehr.auth]
username = "${ATLAS_OPENEHR_USERNAME}"
password = "${ATLAS_OPENEHR_PASSWORD}"

[openehr.query]
template_ids = ["vital_signs.v1"]
batch_size = 1000
parallel_ehrs = 8

[export]
mode = "incremental"
cosmos_composition_format = "preserve"

[cosmosdb]
endpoint = "https://your-account.documents.azure.com:443/"
key = "${ATLAS_COSMOSDB_KEY}"
database_name = "openehr_data"

[state]
enable_state_management = true

[verification]
enable_verification = false

[logging]
local_enabled = true
local_path = "/var/log/atlas"
azure_enabled = false
"#
        .to_string()
    }

    /// Generate configuration with examples and comments
    fn generate_config_with_examples() -> String {
        r#"# Atlas Configuration File
# OpenEHR to Azure Cosmos DB ETL Tool
# 
# This file contains all configuration options with examples and explanations.

# ============================================================================
# Application Settings
# ============================================================================
[application]
# Application name (used in logging and telemetry)
name = "atlas"

# Application version
version = "1.0.0"

# Log level (trace, debug, info, warn, error)
log_level = "info"

# Dry run mode (don't write to Cosmos DB)
dry_run = false

# ============================================================================
# OpenEHR Server Configuration
# ============================================================================
[openehr]
# Base URL of the OpenEHR server
base_url = "https://ehrbase.example.com"

# Vendor type (currently only "ehrbase" is supported)
vendor = "ehrbase"

# Authentication configuration
[openehr.auth]
# Username for Basic Authentication (use environment variable)
username = "${ATLAS_OPENEHR_USERNAME}"

# Password for Basic Authentication (use environment variable)
password = "${ATLAS_OPENEHR_PASSWORD}"

# Query configuration
[openehr.query]
# Template IDs to export (required)
template_ids = [
    "vital_signs.v1",
    "lab_results.v1",
]

# EHR IDs to export (empty = all EHRs)
ehr_ids = []

# Batch size for processing (100-5000)
batch_size = 1000

# Number of parallel EHR processors (1-100)
parallel_ehrs = 8

# ============================================================================
# Export Configuration
# ============================================================================
[export]
# Export mode: "full" or "incremental"
# - full: Export all compositions
# - incremental: Only export new compositions since last run
mode = "incremental"

# Composition format in Cosmos DB: "preserve" or "flatten"
# - preserve: Keep original FLAT JSON structure
# - flatten: Convert paths to simple field names
cosmos_composition_format = "preserve"

# Maximum retry attempts for transient failures
max_retries = 3

# Retry backoff delays in milliseconds
retry_backoff_ms = [1000, 2000, 4000]

# ============================================================================
# Azure Cosmos DB Configuration
# ============================================================================
[cosmosdb]
# Cosmos DB endpoint URL
endpoint = "https://your-account.documents.azure.com:443/"

# Cosmos DB primary key (use environment variable)
key = "${ATLAS_COSMOSDB_KEY}"

# Database name
database_name = "openehr_data"

# Control container name (for state management)
control_container = "atlas_control"

# Data container prefix (containers will be named: prefix_templateid)
data_container_prefix = "compositions"

# Partition key path (should be /ehr_id for optimal patient queries)
partition_key = "/ehr_id"

# Maximum concurrent operations
max_concurrency = 10

# Request timeout in seconds
request_timeout_seconds = 60

# ============================================================================
# State Management Configuration
# ============================================================================
[state]
# Enable state management for incremental exports
enable_state_management = true

# ============================================================================
# Data Verification Configuration
# ============================================================================
[verification]
# Enable checksum calculation for data integrity
enable_verification = false

# Checksum algorithm (sha256 or sha512)
checksum_algorithm = "sha256"

# ============================================================================
# Logging Configuration
# ============================================================================
[logging]
# Enable local file logging
local_enabled = true

# Local log file path
local_path = "/var/log/atlas"

# Log rotation (daily, hourly, never)
local_rotation = "daily"

# Maximum log file size in MB
local_max_size_mb = 100

# Enable Azure Application Insights logging
azure_enabled = false

# Application Insights instrumentation key (if azure_enabled = true)
# azure_instrumentation_key = "${AZURE_INSTRUMENTATION_KEY}"

# Enable Azure Log Analytics logging
# azure_log_analytics_enabled = false

# Log Analytics workspace ID (if azure_log_analytics_enabled = true)
# azure_log_analytics_workspace_id = "${AZURE_LOG_ANALYTICS_WORKSPACE_ID}"

# Log Analytics shared key (if azure_log_analytics_enabled = true)
# azure_log_analytics_shared_key = "${AZURE_LOG_ANALYTICS_SHARED_KEY}"
"#
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_args_defaults() {
        let args = InitArgs {
            output: "atlas.toml".to_string(),
            with_examples: false,
            force: false,
        };

        assert_eq!(args.output, "atlas.toml");
        assert!(!args.with_examples);
        assert!(!args.force);
    }

    #[test]
    fn test_generate_minimal_config() {
        let config = InitArgs::generate_minimal_config();
        assert!(config.contains("[application]"));
        assert!(config.contains("[openehr]"));
        assert!(config.contains("[cosmosdb]"));
    }

    #[test]
    fn test_generate_config_with_examples() {
        let config = InitArgs::generate_config_with_examples();
        assert!(config.contains("# Atlas Configuration File"));
        assert!(config.contains("template_ids"));
        assert!(config.contains("batch_size"));
    }
}

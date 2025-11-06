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
                println!("  2. Set database_target to 'cosmosdb' or 'postgresql'");
                println!("  3. Create a .env file with your credentials:");
                println!("     - Copy .env.example to .env");
                println!("     - Set ATLAS_OPENEHR_USERNAME and ATLAS_OPENEHR_PASSWORD");
                println!("     - Set ATLAS_COSMOSDB_KEY (if using CosmosDB)");
                println!("     - Set ATLAS_PG_PASSWORD (if using PostgreSQL)");
                println!(
                    "  4. For PostgreSQL: Run schema migration (see docs/postgresql-setup.md)"
                );
                println!("  5. Validate configuration: atlas validate-config");
                println!("  6. Run export: atlas export");
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
# OpenEHR to Database ETL Tool
# Supports: Azure Cosmos DB or PostgreSQL

# Database target (postgresql or cosmosdb)
database_target = "cosmosdb"  # cosmosdb | postgresql

[application]
name = "atlas"
version = "1.5.0"
log_level = "info"
dry_run = false

[openehr]
base_url = "https://ehrbase.example.com/ehrbase/rest/openehr/v1"
vendor = "ehrbase"

# Authentication
auth_type = "basic"
username = "${ATLAS_OPENEHR_USERNAME}"
password = "${ATLAS_OPENEHR_PASSWORD}"

# TLS settings
tls_verify = true

[openehr.query]
template_ids = ["vital_signs.v1"]
batch_size = 1000
parallel_ehrs = 8

[export]
mode = "incremental"
export_composition_format = "preserve"
max_retries = 3
retry_backoff_ms = [1000, 2000, 4000]

# Choose ONE database backend based on database_target above

[cosmosdb]
endpoint = "https://your-account.documents.azure.com:443/"
key = "${ATLAS_COSMOSDB_KEY}"
database_name = "openehr_data"
control_container = "atlas_control"
data_container_prefix = "compositions"
partition_key = "/ehr_id"
max_concurrency = 10
request_timeout_seconds = 60

# [postgresql]
# connection_string = "postgresql://atlas_user:${ATLAS_PG_PASSWORD}@localhost:5432/openehr_data?sslmode=require"
# max_connections = 20
# connection_timeout_seconds = 30
# statement_timeout_seconds = 60
# ssl_mode = "require"

[state]
enable_checkpointing = true
checkpoint_interval_seconds = 30

[verification]
enable_verification = false
checksum_algorithm = "sha256"

[logging]
local_enabled = true
local_path = "/var/log/atlas"
local_rotation = "daily"
local_max_size_mb = 100
azure_enabled = false
"#
        .to_string()
    }

    /// Generate configuration with examples and comments
    fn generate_config_with_examples() -> String {
        r#"# Atlas Configuration File
# OpenEHR to Database ETL Tool
#
# This file contains all configuration options with examples and explanations.
#
# Atlas supports two database backends:
#   - Azure Cosmos DB (NoSQL, globally distributed)
#   - PostgreSQL 14+ (Relational, JSONB support)
#
# Choose your backend by setting database_target below.

# ============================================================================
# Database Target Selection
# ============================================================================
# Database target (postgresql or cosmosdb)
database_target = "cosmosdb"  # cosmosdb | postgresql

# ============================================================================
# Application Settings
# ============================================================================
[application]
# Application name (used in logging and telemetry)
name = "atlas"

# Application version
version = "1.5.0"

# Log level (trace, debug, info, warn, error)
log_level = "info"

# Dry run mode (don't write to database)
dry_run = false

# ============================================================================
# OpenEHR Server Configuration
# ============================================================================
[openehr]
# Base URL of the OpenEHR server
base_url = "https://ehrbase.example.com/ehrbase/rest/openehr/v1"

# Vendor type (currently only "ehrbase" is supported)
vendor = "ehrbase"

# Authentication type (currently only "basic" is supported)
auth_type = "basic"

# Username for Basic Authentication (use environment variable)
username = "${ATLAS_OPENEHR_USERNAME}"

# Password for Basic Authentication (use environment variable)
password = "${ATLAS_OPENEHR_PASSWORD}"

# TLS/SSL verification
tls_verify = true

# Optional: Custom CA certificate path
# tls_ca_cert = "/path/to/ca.crt"

# Query configuration
[openehr.query]
# Template IDs to export (required)
template_ids = [
    "vital_signs.v1",
    "lab_results.v1",
]

# EHR IDs to export (empty = all EHRs)
ehr_ids = []

# Optional: Time range filters (ISO 8601 format)
# time_range_start = "2024-01-01T00:00:00Z"
# time_range_end = null  # null = now

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

# Composition format for export: "preserve" or "flatten"
# - preserve: Keep original FLAT JSON structure
# - flatten: Convert paths to simple field names
export_composition_format = "preserve"

# Maximum retry attempts for transient failures
max_retries = 3

# Retry backoff delays in milliseconds
retry_backoff_ms = [1000, 2000, 4000]

# ============================================================================
# Database Configuration
# Choose ONE database backend based on database_target above
# ============================================================================

# ----------------------------------------------------------------------------
# Option 1: Azure Cosmos DB
# ----------------------------------------------------------------------------
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

# ----------------------------------------------------------------------------
# Option 2: PostgreSQL
# ----------------------------------------------------------------------------
# Uncomment this section if using PostgreSQL (database_target = "postgresql")
#
# [postgresql]
# # Connection string format: postgresql://[user[:password]@][host][:port][/dbname][?params]
# connection_string = "postgresql://atlas_user:${ATLAS_PG_PASSWORD}@localhost:5432/openehr_data?sslmode=require"
#
# # Connection pool settings
# max_connections = 20                # Maximum connections in pool (1-100)
# connection_timeout_seconds = 30     # Timeout for acquiring connection
# statement_timeout_seconds = 60      # Timeout for SQL statement execution
#
# # SSL/TLS mode: disable | allow | prefer | require | verify-ca | verify-full
# ssl_mode = "require"                # Use 'require' or higher for production
#
# # Note: Before using PostgreSQL, run the schema migration:
# #   psql -U atlas_user -d openehr_data -f migrations/001_initial_schema.sql
# # See docs/postgresql-setup.md for detailed setup instructions

# ============================================================================
# State Management Configuration
# ============================================================================
[state]
# Enable checkpointing for incremental exports
enable_checkpointing = true

# Checkpoint interval in seconds
checkpoint_interval_seconds = 30

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

# Log rotation (daily or size)
local_rotation = "daily"

# Maximum log file size in MB
local_max_size_mb = 100

# Azure Log Analytics (optional)
# Requires Azure AD App Registration and Data Collection Rule (DCR)
azure_enabled = false
# azure_tenant_id = "${AZURE_TENANT_ID}"
# azure_client_id = "${AZURE_CLIENT_ID}"
# azure_client_secret = "${AZURE_CLIENT_SECRET}"
# azure_log_analytics_workspace_id = "${AZURE_LOG_ANALYTICS_WORKSPACE_ID}"
# azure_dcr_immutable_id = "${AZURE_DCR_IMMUTABLE_ID}"
# azure_dce_endpoint = "${AZURE_DCE_ENDPOINT}"
# azure_stream_name = "Custom-AtlasExport_CL"
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

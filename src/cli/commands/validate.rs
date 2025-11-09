//! Validate config command implementation
//!
//! This module implements the `validate-config` command for validating
//! the Atlas configuration file.

use crate::config::load_config;
use crate::config::schema::DatabaseTarget;
use clap::Args;

/// Arguments for the validate-config command
#[derive(Args, Debug)]
pub struct ValidateArgs {}

impl ValidateArgs {
    /// Execute the validate-config command
    pub async fn execute(&self, config_path: &str) -> anyhow::Result<i32> {
        tracing::info!(config_path = %config_path, "Validating configuration");

        println!("🔍 Validating configuration file: {config_path}");
        println!();

        // Load configuration
        let config = match load_config(config_path) {
            Ok(c) => {
                println!("✅ Configuration file loaded successfully");
                c
            }
            Err(e) => {
                println!("❌ Failed to load configuration file");
                println!("   Error: {e}");
                return Ok(2); // Configuration error exit code
            }
        };

        // Validate configuration
        match config.validate() {
            Ok(_) => {
                println!("✅ Configuration is valid");
                println!();
                println!("Configuration Summary:");
                println!("  Application: {}", config.application.name);
                println!("  Version: {}", config.application.version);
                println!("  Log Level: {}", config.application.log_level);
                println!("  OpenEHR Server: {}", config.openehr.base_url);
                println!("  OpenEHR Vendor: {}", config.openehr.vendor);

                // Display database-specific configuration
                match config.database_target {
                    DatabaseTarget::CosmosDB => {
                        if let Some(ref cosmos_config) = config.cosmosdb {
                            println!("  Database Target: CosmosDB");
                            println!("  Cosmos DB Endpoint: {}", cosmos_config.endpoint);
                            println!("  Cosmos DB Database: {}", cosmos_config.database_name);
                        }
                    }
                    DatabaseTarget::PostgreSQL => {
                        if let Some(ref pg_config) = config.postgresql {
                            use secrecy::ExposeSecret;
                            println!("  Database Target: PostgreSQL");
                            println!(
                                "  PostgreSQL Connection: {}",
                                pg_config
                                    .connection_string
                                    .expose_secret()
                                    .split('@')
                                    .next_back()
                                    .unwrap_or("***")
                            );
                            println!("  Max Connections: {}", pg_config.max_connections);
                        }
                    }
                }

                println!("  Export Mode: {}", config.export.mode);
                println!(
                    "  Composition Format: {}",
                    config.export.export_composition_format
                );
                println!("  Batch Size: {}", config.openehr.query.batch_size);
                println!("  Parallel EHRs: {}", config.openehr.query.parallel_ehrs);
                println!("  Template IDs: {:?}", config.openehr.query.template_ids);
                println!();
                Ok(0)
            }
            Err(e) => {
                println!("❌ Configuration validation failed");
                println!("   Error: {e}");
                println!();
                Ok(2) // Configuration error exit code
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_args_creation() {
        let args = ValidateArgs {};
        // Just ensure it compiles and can be created
        let _ = format!("{args:?}");
    }
}

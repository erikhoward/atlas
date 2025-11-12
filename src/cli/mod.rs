//! CLI interface and argument parsing
//!
//! This module provides the command-line interface for Atlas using clap.

pub mod commands;

use clap::{Parser, Subcommand};

/// Atlas - OpenEHR ETL Tool
#[derive(Parser, Debug)]
#[command(name = "atlas")]
#[command(version, about, long_about = None)]
#[command(author = "Atlas Contributors")]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "atlas.toml", env = "ATLAS_CONFIG")]
    pub config: String,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, env = "ATLAS_LOG_LEVEL")]
    pub log_level: Option<String>,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Export compositions from OpenEHR to configured database
    Export(commands::export::ExportArgs),

    /// Validate configuration file
    ValidateConfig(commands::validate::ValidateArgs),

    /// Show export status and watermarks
    Status(commands::status::StatusArgs),

    /// Initialize a new configuration file
    Init(commands::init::InitArgs),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_export() {
        let cli = Cli::parse_from(["atlas", "export"]);
        assert_eq!(cli.config, "atlas.toml");
        assert!(matches!(cli.command, Commands::Export(_)));
    }

    #[test]
    fn test_cli_parse_with_config() {
        let cli = Cli::parse_from(["atlas", "--config", "custom.toml", "export"]);
        assert_eq!(cli.config, "custom.toml");
    }

    #[test]
    fn test_cli_parse_with_log_level() {
        let cli = Cli::parse_from(["atlas", "--log-level", "debug", "export"]);
        assert_eq!(cli.log_level, Some("debug".to_string()));
    }

    #[test]
    fn test_cli_parse_validate_config() {
        let cli = Cli::parse_from(["atlas", "validate-config"]);
        assert!(matches!(cli.command, Commands::ValidateConfig(_)));
    }

    #[test]
    fn test_cli_parse_status() {
        let cli = Cli::parse_from(["atlas", "status"]);
        assert!(matches!(cli.command, Commands::Status(_)));
    }

    #[test]
    fn test_cli_parse_init() {
        let cli = Cli::parse_from(["atlas", "init"]);
        assert!(matches!(cli.command, Commands::Init(_)));
    }
}

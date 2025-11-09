// Atlas - OpenEHR to Azure Cosmos DB ETL Tool
// Copyright (c) 2025 Atlas Contributors
// Licensed under the MIT License

use atlas::cli::{Cli, Commands};
use atlas::config::LoggingConfig;
use atlas::logging::init_logging;
use clap::Parser;
use std::process;
use tokio::sync::watch;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file if present
    // This is optional - if .env doesn't exist, it's silently ignored
    let _ = dotenvy::dotenv();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logging with console-only config (no file logging for CLI)
    let log_level = cli.log_level.as_deref().unwrap_or("info");
    let logging_config = LoggingConfig {
        local_enabled: false, // Disable file logging for CLI
        local_path: String::new(),
        local_rotation: "daily".to_string(),
        local_max_size_mb: 100,
        azure_enabled: false,
        azure_tenant_id: None,
        azure_client_id: None,
        azure_client_secret: None,
        azure_log_analytics_workspace_id: None,
        azure_dcr_immutable_id: None,
        azure_dce_endpoint: None,
        azure_stream_name: None,
    };
    if let Err(e) = init_logging(log_level, &logging_config) {
        eprintln!("Failed to initialize logging: {e}");
        process::exit(5);
    }

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "Atlas - OpenEHR to Azure Cosmos DB ETL Tool"
    );

    // Create shutdown signal channel for graceful shutdown
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Spawn signal handler task
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to create SIGTERM handler");

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Received SIGINT (Ctrl+C), initiating graceful shutdown...");
                    println!("\n⚠️  Shutdown signal received, completing current batch...");
                    let _ = shutdown_tx_clone.send(true);
                }
                _ = sigterm.recv() => {
                    tracing::info!("Received SIGTERM, initiating graceful shutdown...");
                    println!("\n⚠️  Shutdown signal received, completing current batch...");
                    let _ = shutdown_tx_clone.send(true);
                }
            }
        }

        #[cfg(not(unix))]
        {
            if let Err(e) = tokio::signal::ctrl_c().await {
                tracing::error!(error = %e, "Failed to listen for Ctrl+C");
            } else {
                tracing::info!("Received SIGINT (Ctrl+C), initiating graceful shutdown...");
                println!("\n⚠️  Shutdown signal received, completing current batch...");
                let _ = shutdown_tx_clone.send(true);
            }
        }
    });

    // Execute command and get exit code
    let exit_code = match execute_command(&cli, shutdown_rx).await {
        Ok(code) => code,
        Err(e) => {
            tracing::error!(error = %e, "Command execution failed");
            eprintln!("Error: {e}");
            5 // Fatal error exit code
        }
    };

    // Exit with appropriate code
    process::exit(exit_code);
}

/// Execute the CLI command
async fn execute_command(cli: &Cli, shutdown_signal: watch::Receiver<bool>) -> anyhow::Result<i32> {
    match &cli.command {
        Commands::Export(args) => args.execute(&cli.config, shutdown_signal).await,
        Commands::ValidateConfig(args) => args.execute(&cli.config).await,
        Commands::Status(args) => args.execute(&cli.config).await,
        Commands::Init(args) => args.execute().await,
    }
}

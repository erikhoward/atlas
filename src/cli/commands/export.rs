//! Export command implementation
//!
//! This module implements the `export` command for exporting compositions
//! from OpenEHR to Azure Cosmos DB.

use crate::config::load_config;
use crate::core::export::ExportCoordinator;
use clap::Args;
use std::time::Duration;
use tokio::sync::watch;

/// Arguments for the export command
#[derive(Args, Debug)]
pub struct ExportArgs {
    /// Skip confirmation prompt
    #[arg(short, long)]
    pub yes: bool,

    /// Dry run mode - simulate export without writing to Cosmos DB
    #[arg(long)]
    pub dry_run: bool,

    /// Override template ID(s) to export (comma-separated)
    #[arg(long)]
    pub template_id: Option<String>,

    /// Override EHR ID(s) to export (comma-separated)
    #[arg(long)]
    pub ehr_id: Option<String>,

    /// Override export mode (full or incremental)
    #[arg(long)]
    pub mode: Option<String>,

    /// Enable anonymization of PHI/PII data
    #[arg(long)]
    pub anonymize: bool,

    /// Anonymization compliance mode (gdpr or hipaa_safe_harbor)
    #[arg(long, value_name = "MODE")]
    pub anonymize_mode: Option<String>,

    /// Anonymization dry-run - detect PII without anonymizing
    #[arg(long)]
    pub anonymize_dry_run: bool,
}

impl ExportArgs {
    /// Execute the export command
    pub async fn execute(
        &self,
        config_path: &str,
        shutdown_signal: watch::Receiver<bool>,
    ) -> anyhow::Result<i32> {
        tracing::info!("Starting export command");

        // Load configuration
        let mut config = load_config(config_path)?;

        // Apply CLI overrides
        if let Some(mode) = &self.mode {
            tracing::info!(mode = %mode, "Overriding export mode from CLI");
            config.export.mode = mode.clone();
        }

        if let Some(template_ids) = &self.template_id {
            let ids: Vec<String> = template_ids
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            tracing::info!(template_ids = ?ids, "Overriding template IDs from CLI");
            config.openehr.query.template_ids = ids;
        }

        if let Some(ehr_ids) = &self.ehr_id {
            let ids: Vec<String> = ehr_ids.split(',').map(|s| s.trim().to_string()).collect();
            tracing::info!(ehr_ids = ?ids, "Overriding EHR IDs from CLI");
            config.openehr.query.ehr_ids = ids;
        }

        // Apply dry-run flag from CLI
        if self.dry_run {
            tracing::info!("Enabling dry-run mode from CLI");
            config.export.dry_run = true;
        }

        // Apply anonymization flags from CLI
        if self.anonymize {
            tracing::info!("Enabling anonymization from CLI");
            if config.anonymization.is_none() {
                config.anonymization =
                    Some(crate::anonymization::config::AnonymizationConfig::default());
            }
            if let Some(ref mut anon_config) = config.anonymization {
                anon_config.enabled = true;
            }
        }

        if let Some(ref mode_str) = self.anonymize_mode {
            tracing::info!(mode = %mode_str, "Overriding anonymization mode from CLI");
            if config.anonymization.is_none() {
                config.anonymization =
                    Some(crate::anonymization::config::AnonymizationConfig::default());
            }
            if let Some(ref mut anon_config) = config.anonymization {
                anon_config.mode = match mode_str.to_lowercase().as_str() {
                    "gdpr" => crate::anonymization::compliance::ComplianceMode::Gdpr,
                    "hipaa_safe_harbor" | "hipaa" => {
                        crate::anonymization::compliance::ComplianceMode::HipaaSafeHarbor
                    }
                    _ => {
                        tracing::error!(mode = %mode_str, "Invalid anonymization mode");
                        eprintln!("Invalid anonymization mode: {mode_str}. Use 'gdpr' or 'hipaa_safe_harbor'");
                        return Ok(2);
                    }
                };
            }
        }

        if self.anonymize_dry_run {
            tracing::info!("Enabling anonymization dry-run mode from CLI");
            if config.anonymization.is_none() {
                config.anonymization =
                    Some(crate::anonymization::config::AnonymizationConfig::default());
            }
            if let Some(ref mut anon_config) = config.anonymization {
                anon_config.dry_run = true;
                anon_config.enabled = true; // Enable anonymization for dry-run
            }
        }

        // Validate configuration
        if let Err(e) = config.validate() {
            tracing::error!(error = %e, "Configuration validation failed");
            eprintln!("Configuration validation failed: {e}");
            return Ok(2); // Configuration error exit code
        }

        // Dry run mode
        if self.dry_run {
            tracing::info!("Dry run mode enabled - no data will be written");
            println!("🔍 DRY RUN MODE - No data will be written to the database");
            println!();
        }

        // Confirmation prompt (unless --yes or dry-run)
        if !self.yes && !self.dry_run {
            println!("Export Configuration:");
            println!("  Mode: {}", config.export.mode);
            println!("  Templates: {:?}", config.openehr.query.template_ids);
            println!(
                "  EHRs: {}",
                if config.openehr.query.ehr_ids.is_empty() {
                    "All".to_string()
                } else {
                    format!("{:?}", config.openehr.query.ehr_ids)
                }
            );
            println!("  Batch size: {}", config.openehr.query.batch_size);
            println!();
            print!("Proceed with export? [y/N]: ");
            use std::io::{self, Write};
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Export cancelled.");
                return Ok(0);
            }
        }

        // Get shutdown timeout from config before moving config
        let shutdown_timeout_secs = config.export.shutdown_timeout_secs;

        // Create export coordinator
        tracing::info!("Creating export coordinator");
        let coordinator = match ExportCoordinator::new(config, shutdown_signal).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = %e, "Failed to create export coordinator");
                eprintln!("Failed to initialize export: {e}");
                return Ok(4); // Connection error exit code
            }
        };

        // Execute export with timeout
        tracing::info!("Executing export");
        println!("🚀 Starting export...");
        println!();

        // Configure shutdown timeout
        let _shutdown_timeout = Duration::from_secs(shutdown_timeout_secs);
        tracing::debug!(
            timeout_secs = shutdown_timeout_secs,
            "Shutdown timeout configured"
        );

        // Wrap export execution with timeout
        // Note: The timeout only applies after a shutdown signal is received
        // Normal exports can run indefinitely
        let summary = match coordinator.execute_export().await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = %e, "Export failed");
                eprintln!("Export failed: {e}");
                return Ok(5); // Fatal error exit code
            }
        };

        // Display summary
        println!();
        println!("📊 Export Summary:");
        println!("  Total EHRs: {}", summary.total_ehrs);
        println!("  Total Compositions: {}", summary.total_compositions);
        println!("  Successful: {}", summary.successful_exports);
        println!("  Failed: {}", summary.failed_exports);
        println!("  Duplicates Skipped: {}", summary.duplicates_skipped);
        println!("  Duration: {:.2}s", summary.duration.as_secs_f64());
        println!("  Success Rate: {:.2}%", summary.success_rate());
        println!();

        // Display verification results if available
        if let Some(verification_report) = &summary.verification_report {
            println!("🔍 Verification Results:");
            println!("  Total Verified: {}", verification_report.total_verified);
            println!("  Passed: {}", verification_report.passed);
            println!("  Failed: {}", verification_report.failed);
            println!("  Skipped: {}", verification_report.skipped);
            println!(
                "  Verification Rate: {:.2}%",
                verification_report.success_rate()
            );
            println!(
                "  Duration: {:.2}s",
                verification_report.duration_ms as f64 / 1000.0
            );

            if !verification_report.failures.is_empty() {
                println!();
                println!("  ⚠️  Verification Failures:");
                for (i, failure) in verification_report.failures.iter().enumerate() {
                    if i < 10 {
                        // Show first 10 failures
                        println!(
                            "    - {} (EHR: {}, Template: {})",
                            failure.composition_uid.as_str(),
                            failure.ehr_id.as_str(),
                            failure.template_id.as_str()
                        );
                        println!("      Reason: {}", failure.reason);
                    }
                }
                if verification_report.failures.len() > 10 {
                    println!(
                        "    ... and {} more failures",
                        verification_report.failures.len() - 10
                    );
                }
            }
            println!();
        }

        if !summary.errors.is_empty() {
            println!("⚠️  Errors encountered:");
            for error in &summary.errors {
                println!("  - {:?}: {}", error.error_type, error.message);
                if let Some(context) = &error.context {
                    println!("    Context: {context}");
                }
            }
            println!();
        }

        // Determine exit code
        let exit_code = if summary.interrupted {
            println!();
            println!("⚠️  Export interrupted gracefully. Progress saved.");
            println!("   Run the same command to resume from checkpoint.");
            println!();
            tracing::info!("Export interrupted by user signal");
            130 // SIGINT exit code (standard Unix convention)
        } else if summary.is_successful() {
            println!("✅ Export completed successfully!");
            0
        } else if summary.failed_exports > 0 {
            println!("⚠️  Export completed with failures");
            1 // Partial success
        } else {
            println!("✅ Export completed!");
            0
        };

        Ok(exit_code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_args_defaults() {
        let args = ExportArgs {
            yes: false,
            dry_run: false,
            template_id: None,
            ehr_id: None,
            mode: None,
        };

        assert!(!args.yes);
        assert!(!args.dry_run);
        assert!(args.template_id.is_none());
        assert!(args.ehr_id.is_none());
        assert!(args.mode.is_none());
    }

    #[test]
    fn test_export_args_with_overrides() {
        let args = ExportArgs {
            yes: true,
            dry_run: true,
            template_id: Some("vital_signs.v1".to_string()),
            ehr_id: Some("ehr1,ehr2".to_string()),
            mode: Some("full".to_string()),
        };

        assert!(args.yes);
        assert!(args.dry_run);
        assert_eq!(args.template_id, Some("vital_signs.v1".to_string()));
        assert_eq!(args.ehr_id, Some("ehr1,ehr2".to_string()));
        assert_eq!(args.mode, Some("full".to_string()));
    }
}

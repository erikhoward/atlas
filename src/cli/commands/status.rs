//! Status command implementation
//!
//! This module implements the `status` command for displaying export
//! status and watermarks.

use crate::adapters::database::create_state_storage;
use crate::config::load_config;
use crate::core::state::StateManager;
use clap::Args;

/// Arguments for the status command
#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Filter by template ID
    #[arg(long)]
    pub template_id: Option<String>,

    /// Filter by EHR ID
    #[arg(long)]
    pub ehr_id: Option<String>,
}

impl StatusArgs {
    /// Execute the status command
    pub async fn execute(&self, config_path: &str) -> anyhow::Result<i32> {
        tracing::info!("Checking export status");

        println!("📊 Export Status");
        println!();

        // Load configuration
        let config = match load_config(config_path) {
            Ok(c) => c,
            Err(e) => {
                println!("❌ Failed to load configuration file");
                println!("   Error: {}", e);
                return Ok(2); // Configuration error exit code
            }
        };

        // Create state storage client
        let state_storage = match create_state_storage(&config).await {
            Ok(s) => s,
            Err(e) => {
                println!("❌ Failed to connect to database");
                println!("   Error: {}", e);
                return Ok(4); // Connection error exit code
            }
        };

        // Create state manager
        let state_manager = StateManager::new_with_storage(state_storage);

        // Load all watermarks
        let watermarks = match state_manager.get_all_watermarks().await {
            Ok(w) => w,
            Err(e) => {
                println!("❌ Failed to load watermarks");
                println!("   Error: {}", e);
                return Ok(5); // Fatal error exit code
            }
        };

        if watermarks.is_empty() {
            println!("No export history found.");
            println!("Run 'atlas export' to start exporting data.");
            return Ok(0);
        }

        // Filter watermarks if requested
        let filtered_watermarks: Vec<_> = watermarks
            .iter()
            .filter(|w| {
                if let Some(ref tid) = self.template_id {
                    if w.template_id.as_str() != tid {
                        return false;
                    }
                }
                if let Some(ref eid) = self.ehr_id {
                    if w.ehr_id.as_str() != eid {
                        return false;
                    }
                }
                true
            })
            .collect();

        if filtered_watermarks.is_empty() {
            println!("No watermarks match the specified filters.");
            return Ok(0);
        }

        // Display watermarks in table format
        println!("Found {} watermark(s):", filtered_watermarks.len());
        println!();
        println!(
            "{:<30} {:<40} {:<15} {:<10} {:<25}",
            "Template ID", "EHR ID", "Status", "Count", "Last Export"
        );
        println!("{}", "-".repeat(120));

        for watermark in filtered_watermarks {
            let status = if watermark.is_completed() {
                "✅ Completed"
            } else if watermark.is_in_progress() {
                "🔄 In Progress"
            } else if watermark.is_failed() {
                "❌ Failed"
            } else {
                "⏸️  Not Started"
            };

            let last_export = if let Some(completed_at) = watermark.last_export_completed_at {
                completed_at.format("%Y-%m-%d %H:%M:%S").to_string()
            } else {
                "Never".to_string()
            };

            println!(
                "{:<30} {:<40} {:<15} {:<10} {:<25}",
                watermark.template_id.as_str(),
                watermark.ehr_id.as_str(),
                status,
                watermark.compositions_exported_count,
                last_export
            );
        }

        println!();
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_args_defaults() {
        let args = StatusArgs {
            template_id: None,
            ehr_id: None,
        };

        assert!(args.template_id.is_none());
        assert!(args.ehr_id.is_none());
    }

    #[test]
    fn test_status_args_with_filters() {
        let args = StatusArgs {
            template_id: Some("vital_signs.v1".to_string()),
            ehr_id: Some("ehr123".to_string()),
        };

        assert_eq!(args.template_id, Some("vital_signs.v1".to_string()));
        assert_eq!(args.ehr_id, Some("ehr123".to_string()));
    }
}

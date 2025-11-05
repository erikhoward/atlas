//! Example demonstrating the Atlas logging system
//!
//! This example shows how to:
//! - Initialize structured logging
//! - Use logging macros
//! - Configure Azure integration (optional)
//!
//! Run with:
//! ```bash
//! cargo run --example logging_example
//! ```

use atlas::config::LoggingConfig;
use atlas::domain::ids::{EhrId, TemplateId};
use atlas::logging::init_logging;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a logging configuration
    // Note: Azure logging is disabled in this example. To enable it, set azure_enabled to true
    // and provide the required Azure AD and Log Analytics configuration.
    let config = LoggingConfig {
        local_enabled: true,
        local_path: "/tmp/atlas_example".to_string(),
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

    // Initialize logging (keep the guard alive for the duration of the program)
    let _guard = init_logging("info", &config)?;

    // Log some basic messages
    tracing::info!("Atlas logging example started");
    tracing::debug!("This is a debug message");
    tracing::warn!("This is a warning message");

    // Use structured logging with fields
    tracing::info!(
        version = "1.0.0",
        environment = "development",
        "Application initialized"
    );

    // Demonstrate export logging macros
    let ehr_id = EhrId::new("ehr-12345")?;
    let template_id = TemplateId::new("vital_signs")?;

    atlas::log_export_start!(&ehr_id, &template_id);

    // Simulate some work
    std::thread::sleep(Duration::from_millis(100));

    // Log batch processing
    atlas::log_batch_processing!(50, 100);

    // Simulate more work
    std::thread::sleep(Duration::from_millis(100));

    atlas::log_batch_processing!(100, 100);

    // Log completion
    let duration = Duration::from_millis(200);
    atlas::log_export_complete!(100, duration);

    // Demonstrate retry logging
    atlas::log_retry_attempt!(1, 3, "Connection timeout");

    // Demonstrate error logging
    let error = atlas::domain::AtlasError::Configuration("Example error".to_string());
    atlas::log_error_with_context!(&error, "Demonstrating error logging");

    // Log with correlation ID
    let correlation_id = uuid::Uuid::new_v4();
    tracing::info!(
        correlation_id = %correlation_id,
        operation = "export",
        "Operation completed with correlation ID"
    );

    tracing::info!("Atlas logging example completed");

    println!("\n✅ Logging example completed successfully!");
    println!("📁 Check logs in: /tmp/atlas_example/atlas.log");
    println!("💡 Logs are in JSON format for production use");

    Ok(())
}

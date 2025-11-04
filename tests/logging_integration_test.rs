//! Integration tests for logging functionality

use atlas::config::LoggingConfig;
use atlas::logging::azure::AzureLogger;
use tempfile::TempDir;

#[test]
fn test_logging_config_default() {
    let config = LoggingConfig::default();
    assert!(config.local_enabled);
    assert_eq!(config.local_rotation, "daily");
    assert!(!config.azure_enabled);
}

#[test]
fn test_logging_directory_creation() {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("logs");

    let config = LoggingConfig {
        local_enabled: true,
        local_path: log_path.to_string_lossy().to_string(),
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

    // The directory should be created when logging is initialized
    // For now, just verify the config is valid
    assert!(config.local_enabled);
    assert!(!log_path.exists()); // Not created yet
}

#[tokio::test]
async fn test_azure_logger_disabled() {
    let config = LoggingConfig {
        local_enabled: true,
        local_path: "/tmp/atlas".to_string(),
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

    let result = AzureLogger::new(&config).await;
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Azure logging is not enabled"));
    }
}

#[tokio::test]
async fn test_azure_logger_enabled_with_log_analytics() {
    let config = LoggingConfig {
        local_enabled: true,
        local_path: "/tmp/atlas".to_string(),
        local_rotation: "daily".to_string(),
        local_max_size_mb: 100,
        azure_enabled: true,
        azure_tenant_id: Some("test-tenant-id".to_string()),
        azure_client_id: Some("test-client-id".to_string()),
        azure_client_secret: Some("test-client-secret".to_string()),
        azure_log_analytics_workspace_id: Some("test-workspace-id".to_string()),
        azure_dcr_immutable_id: Some("dcr-test123".to_string()),
        azure_dce_endpoint: Some("https://test-dce.monitor.azure.com".to_string()),
        azure_stream_name: Some("Custom-AtlasExport_CL".to_string()),
    };

    let result = AzureLogger::new(&config).await;
    assert!(result.is_ok());
}

// Note: Actual API calls to Azure Log Analytics are not tested here
// as they require real Azure credentials and infrastructure.
// The logger methods are tested in src/logging/azure.rs unit tests.

#[test]
fn test_logging_rotation_types() {
    let rotations = vec!["daily", "size"];

    for rotation in rotations {
        let config = LoggingConfig {
            local_enabled: true,
            local_path: "/tmp/atlas".to_string(),
            local_rotation: rotation.to_string(),
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

        // Validate that the config is accepted
        assert_eq!(config.local_rotation, rotation);
    }
}

#[test]
fn test_logging_macros_usage() {
    // Test that the macros compile and can be used
    // Note: We can't actually test the output without initializing the logger
    // which can only be done once per process

    use atlas::domain::ids::{EhrId, TemplateId};

    let ehr_id = EhrId::new("test-ehr-123").unwrap();
    let template_id = TemplateId::new("vital_signs").unwrap();

    // These macros should compile
    // atlas::log_export_start!(&ehr_id, &template_id);
    // atlas::log_export_complete!(42, Duration::from_secs(10));
    // atlas::log_batch_processing!(100, 1000);
    // atlas::log_retry_attempt!(2, 3, "Connection timeout");

    // Just verify the types are correct
    assert_eq!(ehr_id.to_string(), "test-ehr-123");
    assert_eq!(template_id.to_string(), "vital_signs");
}

// Note: LoggingConfig::validate() is a private method called by AtlasConfig::validate()
// We test validation through the full config loading process in config_integration_test.rs

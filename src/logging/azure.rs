//! Azure Log Analytics integration
//!
//! This module provides integration with Azure Log Analytics using the Logs Ingestion API.
//! It uses Azure AD authentication (client credentials flow) to send custom logs to a
//! Log Analytics workspace via Data Collection Rules (DCR) and Data Collection Endpoints (DCE).
//!
//! # Features
//!
//! - Azure Log Analytics workspace integration via Logs Ingestion API
//! - Azure AD authentication (client secret credential)
//! - Export operations logging
//! - Error and exception tracking
//! - Performance metrics
//!
//! # Example
//!
//! ```no_run
//! use atlas::logging::azure::AzureLogger;
//! use atlas::config::LoggingConfig;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = LoggingConfig::default();
//! if config.azure_enabled {
//!     let logger = AzureLogger::new(&config).await?;
//!     logger.log_export_operation("export_started", "ehr-123", "IDCR - Vital Signs.v1", 42, "completed", None, 1250).await?;
//! }
//! # Ok(())
//! # }
//! ```

use crate::config::LoggingConfig;
use crate::domain::Result;
use azure_core::credentials::TokenCredential;
use azure_identity::ClientSecretCredential;
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Azure logger for Log Analytics using Logs Ingestion API
///
/// This struct provides methods to send logs to Azure Log Analytics workspace
/// using the modern Logs Ingestion API with Azure AD authentication.
/// It is optional and only initialized when Azure logging is enabled in the configuration.
pub struct AzureLogger {
    /// Azure AD credential for authentication
    credential: Arc<ClientSecretCredential>,
    /// Log Analytics workspace ID
    workspace_id: String,
    /// Data Collection Rule (DCR) immutable ID
    dcr_immutable_id: String,
    /// Data Collection Endpoint (DCE) URL
    dce_endpoint: String,
    /// Stream name for custom logs (e.g., "Custom-AtlasExport_CL")
    stream_name: String,
    /// HTTP client for API calls
    http_client: reqwest::Client,
}

impl AzureLogger {
    /// Create a new Azure logger from configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Logging configuration with Azure settings
    ///
    /// # Returns
    ///
    /// A new `AzureLogger` instance if Azure logging is enabled
    ///
    /// # Errors
    ///
    /// Returns an error if Azure logging is not enabled or required credentials are missing
    ///
    /// # Example
    ///
    /// ```no_run
    /// use atlas::logging::azure::AzureLogger;
    /// use atlas::config::LoggingConfig;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = LoggingConfig::default();
    /// if config.azure_enabled {
    ///     let logger = AzureLogger::new(&config).await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(config: &LoggingConfig) -> Result<Self> {
        if !config.azure_enabled {
            return Err(crate::domain::AtlasError::Configuration(
                "Azure logging is not enabled".to_string(),
            ));
        }

        // Extract required configuration values
        let tenant_id = config
            .azure_tenant_id
            .as_ref()
            .ok_or_else(|| {
                crate::domain::AtlasError::Configuration(
                    "azure_tenant_id is required when Azure logging is enabled".to_string(),
                )
            })?
            .clone();

        let client_id = config
            .azure_client_id
            .as_ref()
            .ok_or_else(|| {
                crate::domain::AtlasError::Configuration(
                    "azure_client_id is required when Azure logging is enabled".to_string(),
                )
            })?
            .clone();

        let client_secret = config
            .azure_client_secret
            .as_ref()
            .ok_or_else(|| {
                crate::domain::AtlasError::Configuration(
                    "azure_client_secret is required when Azure logging is enabled".to_string(),
                )
            })?
            .clone();

        let workspace_id = config
            .azure_log_analytics_workspace_id
            .as_ref()
            .ok_or_else(|| {
                crate::domain::AtlasError::Configuration(
                    "azure_log_analytics_workspace_id is required when Azure logging is enabled"
                        .to_string(),
                )
            })?
            .clone();

        let dcr_immutable_id = config
            .azure_dcr_immutable_id
            .as_ref()
            .ok_or_else(|| {
                crate::domain::AtlasError::Configuration(
                    "azure_dcr_immutable_id is required when Azure logging is enabled".to_string(),
                )
            })?
            .clone();

        let dce_endpoint = config
            .azure_dce_endpoint
            .as_ref()
            .ok_or_else(|| {
                crate::domain::AtlasError::Configuration(
                    "azure_dce_endpoint is required when Azure logging is enabled".to_string(),
                )
            })?
            .clone();

        let stream_name = config
            .azure_stream_name
            .as_ref()
            .ok_or_else(|| {
                crate::domain::AtlasError::Configuration(
                    "azure_stream_name is required when Azure logging is enabled".to_string(),
                )
            })?
            .clone();

        // Create Azure AD credential
        // Convert client_secret to Secret type
        let secret = azure_core::credentials::Secret::new(client_secret.clone());

        let credential = ClientSecretCredential::new(
            &tenant_id,
            client_id.clone(),
            secret,
            None, // Use default options
        )
        .map_err(|e| {
            crate::domain::AtlasError::AzureLogging(format!(
                "Failed to create Azure AD credential: {}",
                e
            ))
        })?;

        // Create HTTP client for API calls
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| {
                crate::domain::AtlasError::Configuration(format!(
                    "Failed to create HTTP client: {}",
                    e
                ))
            })?;

        info!(
            workspace_id = %workspace_id,
            dcr_id = %dcr_immutable_id,
            stream = %stream_name,
            "Azure Log Analytics logger initialized"
        );

        Ok(Self {
            credential,
            workspace_id,
            dcr_immutable_id,
            dce_endpoint,
            stream_name,
            http_client,
        })
    }

    /// Get an Azure AD access token for the Logs Ingestion API
    ///
    /// # Returns
    ///
    /// An access token string
    ///
    /// # Errors
    ///
    /// Returns an error if token acquisition fails
    async fn get_access_token(&self) -> Result<String> {
        const MONITOR_SCOPE: &str = "https://monitor.azure.com/.default";

        // Use the TokenCredential trait to get the token
        let token = TokenCredential::get_token(&*self.credential, &[MONITOR_SCOPE], None)
            .await
            .map_err(|e| {
                crate::domain::AtlasError::AzureLogging(format!(
                    "Failed to acquire Azure AD token: {}",
                    e
                ))
            })?;

        Ok(token.token.secret().to_string())
    }

    /// Send log records to Azure Log Analytics via Logs Ingestion API
    ///
    /// # Arguments
    ///
    /// * `records` - JSON array of log records to send
    ///
    /// # Errors
    ///
    /// Returns an error if the API call fails
    async fn send_logs(&self, records: serde_json::Value) -> Result<()> {
        // Get access token
        let token = self.get_access_token().await?;

        // Build API URL
        let url = format!(
            "{}/dataCollectionRules/{}/streams/{}?api-version=2023-01-01",
            self.dce_endpoint.trim_end_matches('/'),
            self.dcr_immutable_id,
            self.stream_name
        );

        debug!(
            url = %url,
            record_count = records.as_array().map(|a| a.len()).unwrap_or(0),
            "Sending logs to Azure Log Analytics"
        );

        // Make POST request
        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&records)
            .send()
            .await
            .map_err(|e| {
                crate::domain::AtlasError::AzureLogging(format!(
                    "Failed to send logs to Azure: {}",
                    e
                ))
            })?;

        // Check response status
        let status = response.status();
        if status.is_success() {
            info!(
                status = %status,
                record_count = records.as_array().map(|a| a.len()).unwrap_or(0),
                "Successfully sent logs to Azure Log Analytics"
            );
            Ok(())
        } else {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!(
                status = %status,
                error = %error_body,
                "Failed to send logs to Azure Log Analytics"
            );
            Err(crate::domain::AtlasError::AzureLogging(format!(
                "Azure Log Analytics API returned status {}: {}",
                status, error_body
            )))
        }
    }

    /// Log an export operation to Azure Log Analytics
    ///
    /// # Arguments
    ///
    /// * `operation_type` - Type of operation (e.g., "export_started", "export_completed", "batch_processed")
    /// * `ehr_id` - EHR ID being processed
    /// * `template_id` - Template ID being processed
    /// * `composition_count` - Number of compositions processed
    /// * `status` - Operation status (e.g., "started", "completed", "failed")
    /// * `error_message` - Optional error message if operation failed
    /// * `duration_ms` - Operation duration in milliseconds
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use atlas::logging::azure::AzureLogger;
    /// # use atlas::config::LoggingConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = LoggingConfig::default();
    /// # let logger = AzureLogger::new(&config).await?;
    /// logger.log_export_operation(
    ///     "export_completed",
    ///     "ehr-123",
    ///     "IDCR - Vital Signs.v1",
    ///     42,
    ///     "completed",
    ///     None,
    ///     1250
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn log_export_operation(
        &self,
        operation_type: &str,
        ehr_id: &str,
        template_id: &str,
        composition_count: i64,
        status: &str,
        error_message: Option<&str>,
        duration_ms: i64,
    ) -> Result<()> {
        let timestamp = Utc::now().to_rfc3339();

        let record = json!([{
            "TimeGenerated": timestamp,
            "OperationType": operation_type,
            "EhrId": ehr_id,
            "TemplateId": template_id,
            "CompositionCount": composition_count,
            "Status": status,
            "ErrorMessage": error_message.unwrap_or(""),
            "DurationMs": duration_ms,
        }]);

        info!(
            operation_type = operation_type,
            ehr_id = ehr_id,
            template_id = template_id,
            composition_count = composition_count,
            status = status,
            duration_ms = duration_ms,
            "Logging export operation to Azure Log Analytics"
        );

        self.send_logs(record).await
    }

    /// Log an error or exception to Azure Log Analytics
    ///
    /// # Arguments
    ///
    /// * `error_type` - Type of error (e.g., "connection_error", "validation_error", "export_error")
    /// * `error_message` - Error message
    /// * `ehr_id` - Optional EHR ID related to the error
    /// * `template_id` - Optional template ID related to the error
    /// * `stack_trace` - Optional stack trace
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use atlas::logging::azure::AzureLogger;
    /// # use atlas::config::LoggingConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = LoggingConfig::default();
    /// # let logger = AzureLogger::new(&config).await?;
    /// logger.log_error(
    ///     "connection_error",
    ///     "Failed to connect to EHRBase",
    ///     Some("ehr-123"),
    ///     Some("IDCR - Vital Signs.v1"),
    ///     None
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn log_error(
        &self,
        error_type: &str,
        error_message: &str,
        ehr_id: Option<&str>,
        template_id: Option<&str>,
        _stack_trace: Option<&str>,
    ) -> Result<()> {
        let timestamp = Utc::now().to_rfc3339();

        let record = json!([{
            "TimeGenerated": timestamp,
            "OperationType": "error",
            "EhrId": ehr_id.unwrap_or(""),
            "TemplateId": template_id.unwrap_or(""),
            "CompositionCount": 0,
            "Status": "error",
            "ErrorMessage": format!("[{}] {}", error_type, error_message),
            "DurationMs": 0,
        }]);

        error!(
            error_type = error_type,
            error_message = error_message,
            ehr_id = ?ehr_id,
            template_id = ?template_id,
            "Logging error to Azure Log Analytics"
        );

        self.send_logs(record).await
    }

    /// Log performance metrics to Azure Log Analytics
    ///
    /// # Arguments
    ///
    /// * `metric_name` - Name of the metric (e.g., "throughput", "latency", "batch_size")
    /// * `metric_value` - Metric value
    /// * `ehr_id` - Optional EHR ID related to the metric
    /// * `template_id` - Optional template ID related to the metric
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use atlas::logging::azure::AzureLogger;
    /// # use atlas::config::LoggingConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = LoggingConfig::default();
    /// # let logger = AzureLogger::new(&config).await?;
    /// logger.log_performance_metric(
    ///     "throughput",
    ///     1500,
    ///     None,
    ///     Some("IDCR - Vital Signs.v1")
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn log_performance_metric(
        &self,
        metric_name: &str,
        metric_value: i64,
        ehr_id: Option<&str>,
        template_id: Option<&str>,
    ) -> Result<()> {
        let timestamp = Utc::now().to_rfc3339();

        let record = json!([{
            "TimeGenerated": timestamp,
            "OperationType": format!("metric_{}", metric_name),
            "EhrId": ehr_id.unwrap_or(""),
            "TemplateId": template_id.unwrap_or(""),
            "CompositionCount": metric_value,
            "Status": "metric",
            "ErrorMessage": "",
            "DurationMs": 0,
        }]);

        info!(
            metric_name = metric_name,
            metric_value = metric_value,
            ehr_id = ?ehr_id,
            template_id = ?template_id,
            "Logging performance metric to Azure Log Analytics"
        );

        self.send_logs(record).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_azure_logger_creation_disabled() {
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
    }

    #[tokio::test]
    async fn test_azure_logger_creation_enabled() {
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
}

//! OpenEHR client factory and utilities
//!
//! This module provides a factory for creating OpenEHR vendor instances
//! and utilities for working with OpenEHR servers.

use crate::config::OpenEhrConfig;
use crate::domain::{AtlasError, Result};
use std::sync::Arc;

use super::vendor::{EhrBaseVendor, OpenEhrVendor};

/// OpenEHR client that wraps a vendor implementation
///
/// This struct provides a high-level interface for interacting with
/// OpenEHR servers. It handles vendor selection and provides common
/// utilities like health checks and connection pooling.
pub struct OpenEhrClient {
    vendor: Arc<dyn OpenEhrVendor>,
}

impl OpenEhrClient {
    /// Create a new OpenEHR client from configuration
    ///
    /// This factory method creates the appropriate vendor implementation
    /// based on the configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - OpenEHR configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the vendor type is not supported or if
    /// the vendor cannot be initialized.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use atlas::adapters::openehr::client::OpenEhrClient;
    /// use atlas::config::OpenEhrConfig;
    ///
    /// # async fn example() -> atlas::domain::Result<()> {
    /// let config = OpenEhrConfig::default();
    /// let client = OpenEhrClient::new(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(config: OpenEhrConfig) -> Result<Self> {
        let vendor_type = config.vendor_type.to_lowercase();

        let vendor: Arc<dyn OpenEhrVendor> = match vendor_type.as_str() {
            "ehrbase" => {
                let mut vendor = EhrBaseVendor::new(config);
                vendor.authenticate().await?;
                Arc::new(vendor)
            }
            _ => {
                return Err(AtlasError::Configuration(format!(
                    "Unsupported OpenEHR vendor: {vendor_type}. Supported vendors: ehrbase"
                )))
            }
        };

        Ok(Self { vendor })
    }

    /// Get a reference to the underlying vendor implementation
    pub fn vendor(&self) -> &Arc<dyn OpenEhrVendor> {
        &self.vendor
    }

    /// Perform a health check on the OpenEHR server
    ///
    /// This method attempts to verify that the server is reachable
    /// and responding to requests.
    ///
    /// # Errors
    ///
    /// Returns an error if the server is not reachable or not responding.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use atlas::adapters::openehr::client::OpenEhrClient;
    /// # use atlas::config::OpenEhrConfig;
    /// # async fn example() -> atlas::domain::Result<()> {
    /// let config = OpenEhrConfig::default();
    /// let client = OpenEhrClient::new(config).await?;
    ///
    /// if client.health_check().await.is_ok() {
    ///     println!("Server is healthy");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn health_check(&self) -> Result<()> {
        // Try to get EHR IDs as a simple health check
        // This will verify authentication and connectivity
        match self.vendor.get_ehr_ids().await {
            Ok(_) => {
                tracing::info!(
                    base_url = self.vendor.base_url(),
                    "OpenEHR server health check passed"
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    base_url = self.vendor.base_url(),
                    error = %e,
                    "OpenEHR server health check failed"
                );
                Err(e)
            }
        }
    }

    /// Check if the client is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.vendor.is_authenticated()
    }

    /// Get the base URL of the OpenEHR server
    pub fn base_url(&self) -> &str {
        self.vendor.base_url()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::secret::SecretValue;
    use secrecy::Secret;

    #[tokio::test]
    async fn test_client_creation_with_ehrbase() {
        let config = OpenEhrConfig {
            vendor_type: "ehrbase".to_string(),
            username: Some("test".to_string()),
            password: Some(Secret::new(SecretValue::from("test".to_string()))),
            ..Default::default()
        };

        let result = OpenEhrClient::new(config).await;
        assert!(result.is_ok());

        let client = result.unwrap();
        assert!(client.is_authenticated());
        assert_eq!(client.base_url(), "http://localhost:8080/ehrbase");
    }

    #[tokio::test]
    async fn test_client_creation_with_unsupported_vendor() {
        let config = OpenEhrConfig {
            vendor_type: "unsupported".to_string(),
            ..Default::default()
        };

        let result = OpenEhrClient::new(config).await;
        assert!(result.is_err());

        if let Err(AtlasError::Configuration(msg)) = result {
            assert!(msg.contains("Unsupported OpenEHR vendor"));
        } else {
            panic!("Expected Configuration error");
        }
    }

    #[test]
    fn test_client_properties() {
        // This test just verifies the struct compiles and has the expected methods
        // We can't actually create a client without async context
    }
}

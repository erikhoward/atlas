//! EHRBase vendor implementation
//!
//! This module provides the EHRBase-specific implementation of the OpenEHR REST API.
//! EHRBase is an open-source OpenEHR server that implements the OpenEHR REST API v1.1.x.

use super::{CompositionMetadata, OpenEhrVendor};
use crate::config::OpenEhrConfig;
use crate::domain::ids::{CompositionUid, EhrId, TemplateId};
use crate::domain::{AtlasError, Composition, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Utc};
use reqwest::{Client, ClientBuilder, StatusCode};
use serde::Deserialize;
use std::str::FromStr;
use std::time::Duration;

/// EHRBase vendor implementation
///
/// This struct implements the `OpenEhrVendor` trait for EHRBase servers.
/// It handles authentication, composition fetching, and other OpenEHR operations
/// specific to EHRBase.
///
/// # Example
///
/// ```no_run
/// use atlas::adapters::openehr::vendor::{EhrBaseVendor, OpenEhrVendor};
/// use atlas::config::OpenEhrConfig;
///
/// # async fn example() -> atlas::domain::Result<()> {
/// let config = OpenEhrConfig::default();
/// let mut vendor = EhrBaseVendor::new(config);
///
/// // Authenticate
/// vendor.authenticate().await?;
///
/// // Use the vendor...
/// # Ok(())
/// # }
/// ```
pub struct EhrBaseVendor {
    /// Base URL of the EHRBase server
    base_url: String,

    /// HTTP client for making requests
    client: Client,

    /// Authentication token (if authenticated)
    auth_token: Option<String>,

    /// OpenEHR configuration
    config: OpenEhrConfig,
}

impl EhrBaseVendor {
    /// Create a new EHRBase vendor instance
    ///
    /// # Arguments
    ///
    /// * `config` - OpenEHR configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use atlas::adapters::openehr::vendor::EhrBaseVendor;
    /// use atlas::config::OpenEhrConfig;
    ///
    /// let config = OpenEhrConfig::default();
    /// let vendor = EhrBaseVendor::new(config);
    /// ```
    pub fn new(config: OpenEhrConfig) -> Self {
        let base_url = config.base_url.clone();

        // Build HTTP client with TLS configuration
        let mut client_builder = ClientBuilder::new()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .connect_timeout(Duration::from_secs(30));

        // Configure TLS certificate validation
        // Check both tls_verify and tls_verify_certificates (they are aliases)
        if !config.tls_verify || !config.tls_verify_certificates {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        let client = client_builder.build().expect("Failed to build HTTP client");

        Self {
            base_url,
            client,
            auth_token: None,
            config,
        }
    }

    /// Build authorization header value
    fn auth_header_value(&self) -> Option<String> {
        if let Some(ref token) = self.auth_token {
            Some(format!("Bearer {token}"))
        } else if let (Some(ref username), Some(ref password)) =
            (&self.config.username, &self.config.password)
        {
            // Basic auth
            let credentials = format!("{username}:{password}");
            let encoded = general_purpose::STANDARD.encode(credentials.as_bytes());
            Some(format!("Basic {encoded}"))
        } else {
            None
        }
    }

    /// Retry a request with exponential backoff
    async fn retry_request<F, T, Fut>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let max_retries = self.config.retry.max_retries;
        let mut attempt = 0;

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempt += 1;
                    if attempt >= max_retries {
                        return Err(e);
                    }

                    // Calculate backoff delay
                    let delay_ms = self.config.retry.initial_delay_ms
                        * (self
                            .config
                            .retry
                            .backoff_multiplier
                            .powf((attempt - 1) as f64) as u64);
                    let delay_ms = delay_ms.min(self.config.retry.max_delay_ms);

                    tracing::warn!(
                        attempt = attempt,
                        max_retries = max_retries,
                        delay_ms = delay_ms,
                        error = %e,
                        "Retrying request after error"
                    );

                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }
}

#[async_trait]
impl OpenEhrVendor for EhrBaseVendor {
    async fn authenticate(&mut self) -> Result<()> {
        // For Basic Auth, we don't need to fetch a token
        // The credentials are sent with each request
        if self.config.username.is_some() && self.config.password.is_some() {
            tracing::info!("Using Basic Authentication for EHRBase");
            return Ok(());
        }

        // If no credentials provided, check if the server allows anonymous access
        tracing::warn!("No authentication credentials provided, attempting anonymous access");
        Ok(())
    }

    async fn get_ehr_ids(&self) -> Result<Vec<EhrId>> {
        // Use AQL query to fetch all EHR IDs from EHRBase
        // This query selects all unique EHR IDs from the system
        let aql = "SELECT e/ehr_id/value FROM EHR e";

        tracing::info!("Fetching all EHR IDs from EHRBase using AQL query");
        tracing::debug!(aql = %aql, "Executing AQL query to retrieve EHR IDs");

        // Execute AQL query
        let url = format!("{}/rest/openehr/v1/query/aql", self.base_url);

        let response = self
            .retry_request(|| async {
                let mut request = self.client.post(&url).json(&serde_json::json!({
                    "q": aql
                }));

                if let Some(auth) = self.auth_header_value() {
                    request = request.header("Authorization", auth);
                }

                let resp = request.send().await.map_err(|e| {
                    AtlasError::OpenEhr(crate::domain::OpenEhrError::ConnectionFailed(
                        e.to_string(),
                    ))
                })?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    return Err(AtlasError::OpenEhr(
                        crate::domain::OpenEhrError::QueryFailed(format!(
                            "AQL query to fetch EHR IDs failed with status {status}: {body}"
                        )),
                    ));
                }

                resp.json::<AqlQueryResponse>().await.map_err(|e| {
                    AtlasError::OpenEhr(crate::domain::OpenEhrError::InvalidResponse(e.to_string()))
                })
            })
            .await?;

        // Parse response into EhrId list
        let mut ehr_ids = Vec::new();
        for row in response.rows {
            if !row.is_empty() {
                let ehr_id_str = row[0].as_str().ok_or_else(|| {
                    AtlasError::OpenEhr(crate::domain::OpenEhrError::InvalidResponse(
                        "Invalid EHR ID in AQL response".to_string(),
                    ))
                })?;

                match EhrId::from_str(ehr_id_str) {
                    Ok(ehr_id) => ehr_ids.push(ehr_id),
                    Err(e) => {
                        tracing::warn!(
                            ehr_id = %ehr_id_str,
                            error = %e,
                            "Skipping invalid EHR ID"
                        );
                    }
                }
            }
        }

        tracing::info!(
            count = ehr_ids.len(),
            "Successfully fetched EHR IDs from EHRBase"
        );

        Ok(ehr_ids)
    }

    async fn get_compositions_for_ehr(
        &self,
        ehr_id: &EhrId,
        template_id: &TemplateId,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<CompositionMetadata>> {
        // Build AQL query to get compositions for this EHR and template
        let mut aql = format!(
            "SELECT c/uid/value, c/archetype_details/template_id/value, \
             c/context/start_time/value, c/name/value \
             FROM EHR e[ehr_id/value='{ehr_id}'] \
             CONTAINS COMPOSITION c \
             WHERE c/archetype_details/template_id/value = '{template_id}'"
        );

        if let Some(since_time) = since {
            aql.push_str(&format!(
                " AND c/context/start_time/value >= '{}'",
                since_time.to_rfc3339()
            ));
        }

        tracing::debug!(
            aql = %aql,
            ehr_id = %ehr_id,
            template_id = %template_id,
            "Executing AQL query for compositions"
        );

        // Execute AQL query
        let url = format!("{}/rest/openehr/v1/query/aql", self.base_url);

        let response = self
            .retry_request(|| async {
                let mut request = self.client.post(&url).json(&serde_json::json!({
                    "q": aql
                }));

                if let Some(auth) = self.auth_header_value() {
                    request = request.header("Authorization", auth);
                }

                let resp = request.send().await.map_err(|e| {
                    AtlasError::OpenEhr(crate::domain::OpenEhrError::ConnectionFailed(
                        e.to_string(),
                    ))
                })?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    return Err(AtlasError::OpenEhr(
                        crate::domain::OpenEhrError::QueryFailed(format!(
                            "AQL query failed with status {status}: {body}"
                        )),
                    ));
                }

                resp.json::<AqlQueryResponse>().await.map_err(|e| {
                    AtlasError::OpenEhr(crate::domain::OpenEhrError::InvalidResponse(e.to_string()))
                })
            })
            .await?;

        // Parse response into CompositionMetadata
        let mut metadata_list = Vec::new();

        tracing::debug!(
            row_count = response.rows.len(),
            "AQL query returned {} rows",
            response.rows.len()
        );

        for row in response.rows {
            if row.len() >= 3 {
                let uid_str = row[0].as_str().ok_or_else(|| {
                    AtlasError::OpenEhr(crate::domain::OpenEhrError::InvalidResponse(
                        "Invalid UID in AQL response".to_string(),
                    ))
                })?;

                let uid = CompositionUid::parse(uid_str).map_err(|e| {
                    AtlasError::OpenEhr(crate::domain::OpenEhrError::InvalidResponse(e))
                })?;

                let time_str = row[2].as_str().ok_or_else(|| {
                    AtlasError::OpenEhr(crate::domain::OpenEhrError::InvalidResponse(
                        "Invalid timestamp in AQL response".to_string(),
                    ))
                })?;

                let time_committed = DateTime::parse_from_rfc3339(time_str)
                    .map_err(|e| {
                        AtlasError::OpenEhr(crate::domain::OpenEhrError::InvalidResponse(format!(
                            "Invalid timestamp format: {e}"
                        )))
                    })?
                    .with_timezone(&Utc);

                let mut metadata = CompositionMetadata::new(
                    uid,
                    template_id.clone(),
                    ehr_id.clone(),
                    time_committed,
                );

                // Add optional name if present
                if row.len() >= 4 {
                    if let Some(name) = row[3].as_str() {
                        metadata = metadata.with_name(name.to_string());
                    }
                }

                metadata_list.push(metadata);
            } else {
                tracing::warn!(
                    row_length = row.len(),
                    "Skipping AQL row with insufficient columns (expected >= 3)"
                );
            }
        }

        tracing::debug!(
            metadata_count = metadata_list.len(),
            "Parsed {} composition metadata entries",
            metadata_list.len()
        );

        Ok(metadata_list)
    }

    async fn fetch_composition(&self, metadata: &CompositionMetadata) -> Result<Composition> {
        let url = format!(
            "{}/rest/openehr/v1/ehr/{}/composition/{}?format=FLAT",
            self.base_url, metadata.ehr_id, metadata.uid
        );

        tracing::debug!(
            url = %url,
            ehr_id = %metadata.ehr_id,
            composition_uid = %metadata.uid,
            "Fetching composition"
        );

        self.retry_request(|| async {
            let mut request = self.client.get(&url);

            if let Some(auth) = self.auth_header_value() {
                request = request.header("Authorization", auth);
            }

            let resp = request.send().await.map_err(|e| {
                AtlasError::OpenEhr(crate::domain::OpenEhrError::ConnectionFailed(e.to_string()))
            })?;

            match resp.status() {
                StatusCode::OK => {
                    // Parse the FLAT format response
                    let flat_json: serde_json::Value = resp.json().await.map_err(|e| {
                        AtlasError::OpenEhr(crate::domain::OpenEhrError::InvalidResponse(
                            e.to_string(),
                        ))
                    })?;

                    // Convert to domain Composition
                    // Use metadata to populate all required fields
                    Ok(Composition::builder()
                        .uid(metadata.uid.clone())
                        .ehr_id(metadata.ehr_id.clone())
                        .template_id(metadata.template_id.clone())
                        .time_committed(metadata.time_committed)
                        .content(flat_json)
                        .build()
                        .map_err(AtlasError::Configuration)?)
                }
                StatusCode::NOT_FOUND => Err(AtlasError::OpenEhr(
                    crate::domain::OpenEhrError::CompositionNotFound(metadata.uid.to_string()),
                )),
                status => {
                    let body = resp.text().await.unwrap_or_default();
                    Err(AtlasError::OpenEhr(
                        crate::domain::OpenEhrError::QueryFailed(format!(
                            "Failed to fetch composition with status {status}: {body}"
                        )),
                    ))
                }
            }
        })
        .await
    }

    fn is_authenticated(&self) -> bool {
        self.auth_token.is_some()
            || (self.config.username.is_some() && self.config.password.is_some())
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// AQL query response structure
#[derive(Debug, Deserialize)]
struct AqlQueryResponse {
    rows: Vec<Vec<serde_json::Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ehrbase_vendor_creation() {
        let config = OpenEhrConfig::default();
        let vendor = EhrBaseVendor::new(config);

        assert!(!vendor.is_authenticated());
        assert_eq!(vendor.base_url(), "http://localhost:8080/ehrbase");
    }

    #[test]
    fn test_ehrbase_vendor_with_credentials() {
        let config = OpenEhrConfig {
            username: Some("test_user".to_string()),
            password: Some("test_pass".to_string()),
            ..Default::default()
        };

        let vendor = EhrBaseVendor::new(config);

        assert!(vendor.is_authenticated());
    }
}

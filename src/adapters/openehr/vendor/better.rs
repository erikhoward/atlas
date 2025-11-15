//! Better Platform vendor implementation
//!
//! This module provides the Better Platform-specific implementation of the openEHR REST API.
//! Better Platform uses OIDC (OpenID Connect) authentication with OAuth2 password grant flow.

use super::{CompositionMetadata, OpenEhrVendor};
use crate::config::OpenEhrConfig;
use crate::domain::ids::{CompositionUid, EhrId, TemplateId};
use crate::domain::{AtlasError, Composition, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::{Client, ClientBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Better Platform vendor implementation
///
/// This struct implements the `OpenEhrVendor` trait for Better Platform servers.
/// It handles OIDC authentication, token refresh, composition fetching, and other
/// openEHR operations specific to Better Platform.
///
/// # Authentication
///
/// Better Platform uses OIDC with OAuth2 password grant flow:
/// - Initial authentication: POST to token endpoint with username/password
/// - Returns: access_token, refresh_token, expires_in
/// - Token refresh: POST to token endpoint with refresh_token before expiration
///
/// # Example
///
/// ```no_run
/// use atlas::adapters::openehr::vendor::{BetterVendor, OpenEhrVendor};
/// use atlas::config::OpenEhrConfig;
///
/// # async fn example() -> atlas::domain::Result<()> {
/// let config = OpenEhrConfig::default();
/// let mut vendor = BetterVendor::new(config);
///
/// // Authenticate
/// vendor.authenticate().await?;
///
/// // Use the vendor...
/// # Ok(())
/// # }
/// ```
/// Token state for OIDC authentication
#[derive(Debug, Clone)]
struct TokenState {
    access_token: Option<String>,
    refresh_token: Option<String>,
    token_expiry: Option<DateTime<Utc>>,
}

pub struct BetterVendor {
    /// Base URL of the Better Platform server
    base_url: String,

    /// HTTP client for making requests
    client: Client,

    /// Token state (protected by mutex for interior mutability)
    token_state: Arc<Mutex<TokenState>>,

    /// openEHR configuration
    config: OpenEhrConfig,
}

/// OIDC token request for password grant
#[derive(Debug, Serialize)]
struct OidcPasswordRequest {
    grant_type: String,
    client_id: String,
    username: String,
    password: String,
}

/// OIDC token request for refresh grant
#[derive(Debug, Serialize)]
struct OidcRefreshRequest {
    grant_type: String,
    client_id: String,
    refresh_token: String,
}

/// OIDC token response
#[derive(Debug, Deserialize)]
struct OidcTokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    expires_in: u64,
    #[serde(default)]
    #[allow(dead_code)]
    token_type: String,
}

impl BetterVendor {
    /// Create a new Better Platform vendor instance
    ///
    /// # Arguments
    ///
    /// * `config` - openEHR configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use atlas::adapters::openehr::vendor::BetterVendor;
    /// use atlas::config::OpenEhrConfig;
    ///
    /// let config = OpenEhrConfig::default();
    /// let vendor = BetterVendor::new(config);
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
            // Security Warning: TLS verification is disabled
            // This exposes the application to man-in-the-middle attacks
            tracing::warn!(
                "⚠️  SECURITY WARNING: TLS certificate verification is DISABLED for openEHR server at {}. \
                This configuration is INSECURE and should only be used in development/testing environments. \
                The application is vulnerable to man-in-the-middle attacks. \
                For production use, either enable TLS verification (tls_verify = true) or provide a custom CA certificate (tls_ca_cert).",
                config.base_url
            );
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        let client = client_builder.build().expect("Failed to build HTTP client");

        Self {
            base_url,
            client,
            token_state: Arc::new(Mutex::new(TokenState {
                access_token: None,
                refresh_token: None,
                token_expiry: None,
            })),
            config,
        }
    }

    /// Build authorization header value with Bearer token
    async fn auth_header_value(&self) -> Option<String> {
        let state = self.token_state.lock().await;
        state
            .access_token
            .as_ref()
            .map(|token| format!("Bearer {token}"))
    }

    /// Acquire OIDC tokens using password grant flow
    async fn acquire_token(&mut self) -> Result<()> {
        use secrecy::ExposeSecret;

        let oidc_token_url = self.config.oidc_token_url.as_ref().ok_or_else(|| {
            AtlasError::Configuration(
                "oidc_token_url is required for Better Platform authentication".to_string(),
            )
        })?;

        let client_id = self.config.client_id.as_ref().ok_or_else(|| {
            AtlasError::Configuration(
                "client_id is required for Better Platform authentication".to_string(),
            )
        })?;

        let username = self.config.username.as_ref().ok_or_else(|| {
            AtlasError::Configuration(
                "username is required for Better Platform authentication".to_string(),
            )
        })?;

        let password = self.config.password.as_ref().ok_or_else(|| {
            AtlasError::Configuration(
                "password is required for Better Platform authentication".to_string(),
            )
        })?;

        tracing::debug!(
            token_url = %oidc_token_url,
            client_id = %client_id,
            username = %username,
            "Acquiring OIDC token with password grant"
        );

        let request_body = OidcPasswordRequest {
            grant_type: "password".to_string(),
            client_id: client_id.clone(),
            username: username.clone(),
            password: password.expose_secret().to_string(),
        };

        let response = self
            .client
            .post(oidc_token_url)
            .form(&request_body)
            .send()
            .await
            .map_err(|e| {
                AtlasError::OpenEhr(crate::domain::OpenEhrError::ConnectionFailed(format!(
                    "Failed to request OIDC token: {e}"
                )))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AtlasError::OpenEhr(
                crate::domain::OpenEhrError::AuthenticationFailed(format!(
                    "OIDC token request failed with status {status}: {error_text}"
                )),
            ));
        }

        let token_response: OidcTokenResponse = response.json().await.map_err(|e| {
            AtlasError::OpenEhr(crate::domain::OpenEhrError::InvalidResponse(format!(
                "Failed to parse OIDC token response: {e}"
            )))
        })?;

        // Calculate token expiry (current time + expires_in seconds)
        let token_expiry = Utc::now() + chrono::Duration::seconds(token_response.expires_in as i64);

        // Store tokens
        let mut state = self.token_state.lock().await;
        state.access_token = Some(token_response.access_token);
        state.refresh_token = token_response.refresh_token.clone();
        state.token_expiry = Some(token_expiry);

        tracing::info!(
            expires_at = %token_expiry,
            has_refresh_token = token_response.refresh_token.is_some(),
            "Successfully acquired OIDC access token"
        );

        Ok(())
    }

    /// Refresh the access token using the refresh token
    async fn refresh_token_impl(&self) -> Result<()> {
        let oidc_token_url = self.config.oidc_token_url.as_ref().ok_or_else(|| {
            AtlasError::Configuration(
                "oidc_token_url is required for Better Platform authentication".to_string(),
            )
        })?;

        let client_id = self.config.client_id.as_ref().ok_or_else(|| {
            AtlasError::Configuration(
                "client_id is required for Better Platform authentication".to_string(),
            )
        })?;

        let refresh_token = {
            let state = self.token_state.lock().await;
            state.refresh_token.clone().ok_or_else(|| {
                AtlasError::OpenEhr(crate::domain::OpenEhrError::AuthenticationFailed(
                    "No refresh token available for token refresh".to_string(),
                ))
            })?
        };

        tracing::debug!(
            token_url = %oidc_token_url,
            client_id = %client_id,
            "Refreshing OIDC token"
        );

        let request_body = OidcRefreshRequest {
            grant_type: "refresh_token".to_string(),
            client_id: client_id.clone(),
            refresh_token,
        };

        let response = self
            .client
            .post(oidc_token_url)
            .form(&request_body)
            .send()
            .await
            .map_err(|e| {
                AtlasError::OpenEhr(crate::domain::OpenEhrError::ConnectionFailed(format!(
                    "Failed to refresh OIDC token: {e}"
                )))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AtlasError::OpenEhr(
                crate::domain::OpenEhrError::AuthenticationFailed(format!(
                    "OIDC token refresh failed with status {status}: {error_text}"
                )),
            ));
        }

        let token_response: OidcTokenResponse = response.json().await.map_err(|e| {
            AtlasError::OpenEhr(crate::domain::OpenEhrError::InvalidResponse(format!(
                "Failed to parse OIDC token refresh response: {e}"
            )))
        })?;

        // Calculate token expiry (current time + expires_in seconds)
        let token_expiry = Utc::now() + chrono::Duration::seconds(token_response.expires_in as i64);

        // Store new tokens
        let mut state = self.token_state.lock().await;
        state.access_token = Some(token_response.access_token);
        // Update refresh token if a new one was provided
        if let Some(new_refresh_token) = token_response.refresh_token {
            state.refresh_token = Some(new_refresh_token);
        }
        state.token_expiry = Some(token_expiry);

        tracing::info!(
            expires_at = %token_expiry,
            "Successfully refreshed OIDC access token"
        );

        Ok(())
    }

    /// Check if token needs refresh and refresh if necessary
    /// Tokens are refreshed if they expire within 60 seconds
    async fn check_and_refresh_token(&self) -> Result<()> {
        let should_refresh = {
            let state = self.token_state.lock().await;

            // If no token, nothing to refresh
            if state.access_token.is_none() {
                return Ok(());
            }

            // Check if token is expiring soon (within 60 seconds)
            if let Some(expiry) = state.token_expiry {
                let now = Utc::now();
                let time_until_expiry = expiry - now;

                if time_until_expiry.num_seconds() < 60 {
                    tracing::debug!(
                        seconds_until_expiry = time_until_expiry.num_seconds(),
                        "Token expiring soon, refreshing"
                    );
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };

        if should_refresh {
            self.refresh_token_impl().await?;
        }

        Ok(())
    }

    /// Ensure the client is authenticated and token is valid
    /// This checks and refreshes the token if needed
    async fn ensure_authenticated(&self) -> Result<()> {
        {
            let state = self.token_state.lock().await;
            if state.access_token.is_none() {
                return Err(AtlasError::OpenEhr(
                    crate::domain::OpenEhrError::AuthenticationFailed(
                        "Not authenticated. Call authenticate() first.".to_string(),
                    ),
                ));
            }
        }

        self.check_and_refresh_token().await
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
                        "Request failed, retrying with exponential backoff"
                    );

                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }

    /// Get all EHR IDs from the Better Platform server using AQL
    async fn get_ehr_ids_impl(&self) -> Result<Vec<EhrId>> {
        use crate::adapters::openehr::models::AqlQueryResponse;

        self.ensure_authenticated().await?;

        // Use AQL query to fetch all EHR IDs
        let aql = "SELECT e/ehr_id/value FROM EHR e";

        tracing::info!("Fetching all EHR IDs from Better Platform using AQL query");
        tracing::debug!(aql = %aql, "Executing AQL query to retrieve EHR IDs");

        // Execute AQL query
        let url = format!("{}/rest/openehr/v1/query/aql", self.base_url);

        let response = self
            .retry_request(|| async {
                let mut request = self.client.post(&url).json(&serde_json::json!({
                    "q": aql
                }));

                if let Some(auth) = self.auth_header_value().await {
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
            "Successfully fetched EHR IDs from Better Platform"
        );

        Ok(ehr_ids)
    }

    /// Get compositions for a specific EHR and template using AQL
    async fn get_compositions_for_ehr_impl(
        &self,
        ehr_id: &EhrId,
        template_id: &TemplateId,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<CompositionMetadata>> {
        use crate::adapters::openehr::models::AqlQueryResponse;

        self.ensure_authenticated().await?;

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

                if let Some(auth) = self.auth_header_value().await {
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

    /// Fetch a composition in FLAT format
    async fn fetch_composition_impl(&self, metadata: &CompositionMetadata) -> Result<Composition> {
        self.ensure_authenticated().await?;

        // Better Platform uses Accept header instead of query parameter for format
        // Try FLAT format first with proper Accept header
        let url = format!(
            "{}/rest/openehr/v1/ehr/{}/composition/{}",
            self.base_url, metadata.ehr_id, metadata.uid
        );

        tracing::debug!(
            url = %url,
            ehr_id = %metadata.ehr_id,
            composition_uid = %metadata.uid,
            "Fetching composition"
        );

        self.retry_request(|| async {
            // Better Platform uses custom Accept header for FLAT format
            let mut request = self
                .client
                .get(&url)
                .header("Accept", "application/openehr.wt.flat+json");

            if let Some(auth) = self.auth_header_value().await {
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

    /// Check if the client is authenticated
    async fn is_authenticated_impl(&self) -> bool {
        let state = self.token_state.lock().await;
        if let Some(expiry) = state.token_expiry {
            // Check if we have a token and it's not expired
            state.access_token.is_some() && Utc::now() < expiry
        } else {
            false
        }
    }

    /// Get the base URL of the Better Platform server
    fn base_url_impl(&self) -> &str {
        &self.base_url
    }

    /// Authenticate with the Better Platform server using OIDC
    async fn authenticate_impl(&mut self) -> Result<()> {
        self.acquire_token().await
    }
}

#[async_trait]
impl OpenEhrVendor for BetterVendor {
    async fn authenticate(&mut self) -> Result<()> {
        self.authenticate_impl().await
    }

    async fn get_ehr_ids(&self) -> Result<Vec<EhrId>> {
        self.get_ehr_ids_impl().await
    }

    async fn get_compositions_for_ehr(
        &self,
        ehr_id: &EhrId,
        template_id: &TemplateId,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<CompositionMetadata>> {
        self.get_compositions_for_ehr_impl(ehr_id, template_id, since)
            .await
    }

    async fn fetch_composition(&self, metadata: &CompositionMetadata) -> Result<Composition> {
        self.fetch_composition_impl(metadata).await
    }

    fn is_authenticated(&self) -> bool {
        // We need to block on the async method
        // This is safe because we're just reading the state
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.is_authenticated_impl())
        })
    }

    fn base_url(&self) -> &str {
        self.base_url_impl()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OpenEhrConfig;

    fn create_test_config() -> OpenEhrConfig {
        use crate::config::secret::SecretValue;
        use secrecy::Secret;

        OpenEhrConfig {
            base_url: "https://test.better.care/ehr".to_string(),
            vendor: "better".to_string(),
            vendor_type: "better".to_string(),
            auth_type: "oidc".to_string(),
            username: Some("test_user".to_string()),
            password: Some(Secret::new(SecretValue::from("test_pass".to_string()))),
            oidc_token_url: Some(
                "https://test.better.care/auth/realms/portal/protocol/openid-connect/token"
                    .to_string(),
            ),
            client_id: Some("portal".to_string()),
            tls_verify: true,
            tls_verify_certificates: true,
            timeout_seconds: 30,
            tls_ca_cert: None,
            retry: crate::config::schema::RetryConfig::default(),
            query: crate::config::schema::QueryConfig::default(),
        }
    }

    #[test]
    fn test_better_vendor_new() {
        let config = create_test_config();
        let vendor = BetterVendor::new(config.clone());

        assert_eq!(vendor.base_url, config.base_url);
        assert_eq!(vendor.config.vendor_type, "better");
    }

    #[tokio::test]
    async fn test_auth_header_value_with_no_token() {
        let config = create_test_config();
        let vendor = BetterVendor::new(config);

        let header = vendor.auth_header_value().await;
        assert!(header.is_none());
    }

    #[tokio::test]
    async fn test_auth_header_value_with_token() {
        let config = create_test_config();
        let vendor = BetterVendor::new(config);

        // Manually set a token
        {
            let mut state = vendor.token_state.lock().await;
            state.access_token = Some("test_access_token".to_string());
        }

        let header = vendor.auth_header_value().await;
        assert_eq!(header, Some("Bearer test_access_token".to_string()));
    }

    #[tokio::test]
    async fn test_is_authenticated_with_no_token() {
        let config = create_test_config();
        let vendor = BetterVendor::new(config);

        let is_auth = vendor.is_authenticated_impl().await;
        assert!(!is_auth);
    }

    #[tokio::test]
    async fn test_is_authenticated_with_valid_token() {
        let config = create_test_config();
        let vendor = BetterVendor::new(config);

        // Set a token that expires in the future
        {
            let mut state = vendor.token_state.lock().await;
            state.access_token = Some("test_token".to_string());
            state.token_expiry = Some(Utc::now() + chrono::Duration::hours(1));
        }

        let is_auth = vendor.is_authenticated_impl().await;
        assert!(is_auth);
    }

    #[tokio::test]
    async fn test_is_authenticated_with_expired_token() {
        let config = create_test_config();
        let vendor = BetterVendor::new(config);

        // Set a token that expired in the past
        {
            let mut state = vendor.token_state.lock().await;
            state.access_token = Some("test_token".to_string());
            state.token_expiry = Some(Utc::now() - chrono::Duration::hours(1));
        }

        let is_auth = vendor.is_authenticated_impl().await;
        assert!(!is_auth);
    }

    #[test]
    fn test_base_url() {
        let config = create_test_config();
        let vendor = BetterVendor::new(config.clone());

        assert_eq!(vendor.base_url(), config.base_url);
    }

    #[test]
    fn test_oidc_password_request_serialization() {
        let request = OidcPasswordRequest {
            grant_type: "password".to_string(),
            client_id: "test_client".to_string(),
            username: "test_user".to_string(),
            password: "test_pass".to_string(),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["grant_type"], "password");
        assert_eq!(json["client_id"], "test_client");
        assert_eq!(json["username"], "test_user");
        assert_eq!(json["password"], "test_pass");
    }

    #[test]
    fn test_oidc_refresh_request_serialization() {
        let request = OidcRefreshRequest {
            grant_type: "refresh_token".to_string(),
            client_id: "test_client".to_string(),
            refresh_token: "test_refresh_token".to_string(),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["grant_type"], "refresh_token");
        assert_eq!(json["client_id"], "test_client");
        assert_eq!(json["refresh_token"], "test_refresh_token");
    }

    #[test]
    fn test_oidc_token_response_deserialization() {
        let json = serde_json::json!({
            "access_token": "test_access_token",
            "refresh_token": "test_refresh_token",
            "expires_in": 3600,
            "token_type": "Bearer"
        });

        let response: OidcTokenResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.access_token, "test_access_token");
        assert_eq!(
            response.refresh_token,
            Some("test_refresh_token".to_string())
        );
        assert_eq!(response.expires_in, 3600);
        assert_eq!(response.token_type, "Bearer");
    }

    #[test]
    fn test_oidc_token_response_deserialization_without_refresh_token() {
        let json = serde_json::json!({
            "access_token": "test_access_token",
            "expires_in": 3600
        });

        let response: OidcTokenResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.access_token, "test_access_token");
        assert!(response.refresh_token.is_none());
        assert_eq!(response.expires_in, 3600);
    }
}

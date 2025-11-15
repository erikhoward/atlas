//! openEHR vendor trait definition
//!
//! This module defines the `OpenEhrVendor` trait that abstracts vendor-specific
//! implementations of openEHR REST API servers. This allows Atlas to support
//! multiple openEHR vendors (EHRBase, Better, etc.) through a common interface.

use crate::domain::ids::{CompositionUid, EhrId, TemplateId};
use crate::domain::{Composition, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Metadata about a composition without the full content
///
/// This is used to list compositions for an EHR without fetching
/// the full composition data, which can be large.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompositionMetadata {
    /// Unique identifier for the composition (including version)
    pub uid: CompositionUid,

    /// Template ID used for this composition
    pub template_id: TemplateId,

    /// EHR ID this composition belongs to
    pub ehr_id: EhrId,

    /// Timestamp when the composition was committed
    pub time_committed: DateTime<Utc>,

    /// Archetype node ID (optional)
    pub archetype_node_id: Option<String>,

    /// Composition name (optional)
    pub name: Option<String>,
}

impl CompositionMetadata {
    /// Create a new composition metadata
    pub fn new(
        uid: CompositionUid,
        template_id: TemplateId,
        ehr_id: EhrId,
        time_committed: DateTime<Utc>,
    ) -> Self {
        Self {
            uid,
            template_id,
            ehr_id,
            time_committed,
            archetype_node_id: None,
            name: None,
        }
    }

    /// Set the archetype node ID
    pub fn with_archetype_node_id(mut self, archetype_node_id: String) -> Self {
        self.archetype_node_id = Some(archetype_node_id);
        self
    }

    /// Set the composition name
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
}

/// Trait for openEHR vendor implementations
///
/// This trait defines the interface that all openEHR vendor implementations
/// must provide. It abstracts the vendor-specific REST API details and provides
/// a common interface for Atlas to interact with different openEHR servers.
///
/// # Example
///
/// ```no_run
/// use atlas::adapters::openehr::vendor::{OpenEhrVendor, EhrBaseVendor};
/// use atlas::config::OpenEhrConfig;
/// use atlas::domain::ids::{EhrId, TemplateId};
/// use atlas::domain::AtlasError;
/// use std::str::FromStr;
///
/// # async fn example() -> atlas::domain::Result<()> {
/// // Create a vendor instance
/// let config = OpenEhrConfig::default();
/// let mut vendor = EhrBaseVendor::new(config);
///
/// // Authenticate
/// vendor.authenticate().await?;
///
/// // Get all EHR IDs
/// let ehr_ids = vendor.get_ehr_ids().await?;
///
/// // Get compositions for a specific EHR and template
/// let ehr_id = EhrId::from_str("ehr-123").map_err(|e| AtlasError::Validation(e))?;
/// let template_id = TemplateId::from_str("vital_signs").map_err(|e| AtlasError::Validation(e))?;
/// let compositions = vendor.get_compositions_for_ehr(&ehr_id, &template_id, None).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait OpenEhrVendor: Send + Sync {
    /// Authenticate with the openEHR server
    ///
    /// This method should establish authentication with the server and store
    /// any necessary tokens or credentials for subsequent requests.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails due to invalid credentials,
    /// network issues, or server errors.
    async fn authenticate(&mut self) -> Result<()>;

    /// Get all EHR IDs from the server
    ///
    /// This method retrieves a list of all EHR IDs available on the server.
    /// The implementation may use vendor-specific queries or endpoints.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the server returns an error.
    ///
    /// # Notes
    ///
    /// Some vendors may not support listing all EHRs. In such cases, this
    /// method may return an empty list or an error.
    async fn get_ehr_ids(&self) -> Result<Vec<EhrId>>;

    /// Get composition metadata for a specific EHR and template
    ///
    /// This method retrieves metadata about compositions for a given EHR ID
    /// and template ID. It does not fetch the full composition content.
    ///
    /// # Arguments
    ///
    /// * `ehr_id` - The EHR ID to query
    /// * `template_id` - The template ID to filter by
    /// * `since` - Optional timestamp to filter compositions modified after this time
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the server returns an error.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use atlas::adapters::openehr::vendor::OpenEhrVendor;
    /// # use atlas::domain::ids::{EhrId, TemplateId};
    /// # use atlas::domain::AtlasError;
    /// # use chrono::Utc;
    /// # use std::str::FromStr;
    /// # async fn example(vendor: &impl OpenEhrVendor) -> atlas::domain::Result<()> {
    /// let ehr_id = EhrId::from_str("ehr-123").map_err(|e| AtlasError::Validation(e))?;
    /// let template_id = TemplateId::from_str("vital_signs").map_err(|e| AtlasError::Validation(e))?;
    /// let since = Some(Utc::now() - chrono::Duration::days(7));
    ///
    /// let compositions = vendor.get_compositions_for_ehr(&ehr_id, &template_id, since).await?;
    /// println!("Found {} compositions", compositions.len());
    /// # Ok(())
    /// # }
    /// ```
    async fn get_compositions_for_ehr(
        &self,
        ehr_id: &EhrId,
        template_id: &TemplateId,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<CompositionMetadata>>;

    /// Fetch the full composition content
    ///
    /// This method retrieves the complete composition data in FLAT format.
    ///
    /// # Arguments
    ///
    /// * `metadata` - The composition metadata containing UID, EHR ID, template ID, etc.
    ///
    /// # Errors
    ///
    /// Returns an error if the composition is not found, if the request fails,
    /// or if the server returns an error.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use atlas::adapters::openehr::vendor::{OpenEhrVendor, CompositionMetadata};
    /// # use atlas::domain::ids::{EhrId, CompositionUid, TemplateId};
    /// # use atlas::domain::AtlasError;
    /// # use chrono::Utc;
    /// # use std::str::FromStr;
    /// # async fn example(vendor: &impl OpenEhrVendor) -> atlas::domain::Result<()> {
    /// let metadata = CompositionMetadata::new(
    ///     CompositionUid::from_str("550e8400-e29b-41d4-a716-446655440000::local.ehrbase.org::1")
    ///         .map_err(|e| AtlasError::Validation(e))?,
    ///     TemplateId::from_str("vital_signs").map_err(|e| AtlasError::Validation(e))?,
    ///     EhrId::from_str("7d44b88c-4199-4bad-97dc-d78268e01398").map_err(|e| AtlasError::Validation(e))?,
    ///     Utc::now(),
    /// );
    /// let composition = vendor.fetch_composition(&metadata).await?;
    /// println!("Fetched composition: {}", composition.uid);
    /// # Ok(())
    /// # }
    /// ```
    async fn fetch_composition(&self, metadata: &CompositionMetadata) -> Result<Composition>;

    /// Check if the vendor is authenticated
    ///
    /// This method returns true if the vendor has valid authentication credentials.
    fn is_authenticated(&self) -> bool;

    /// Get the base URL of the openEHR server
    fn base_url(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composition_metadata_creation() {
        let uid =
            CompositionUid::parse("550e8400-e29b-41d4-a716-446655440000::local.ehrbase.org::1")
                .unwrap();
        let template_id = TemplateId::new("vital_signs").unwrap();
        let ehr_id = EhrId::new("ehr-123").unwrap();
        let time_committed = Utc::now();

        let metadata = CompositionMetadata::new(uid.clone(), template_id, ehr_id, time_committed);

        assert_eq!(metadata.uid, uid);
        assert_eq!(metadata.archetype_node_id, None);
        assert_eq!(metadata.name, None);
    }

    #[test]
    fn test_composition_metadata_with_optional_fields() {
        let uid =
            CompositionUid::parse("550e8400-e29b-41d4-a716-446655440000::local.ehrbase.org::1")
                .unwrap();
        let template_id = TemplateId::new("vital_signs").unwrap();
        let ehr_id = EhrId::new("ehr-123").unwrap();
        let time_committed = Utc::now();

        let metadata = CompositionMetadata::new(uid, template_id, ehr_id, time_committed)
            .with_archetype_node_id("openEHR-EHR-COMPOSITION.encounter.v1".to_string())
            .with_name("Vital Signs".to_string());

        assert_eq!(
            metadata.archetype_node_id,
            Some("openEHR-EHR-COMPOSITION.encounter.v1".to_string())
        );
        assert_eq!(metadata.name, Some("Vital Signs".to_string()));
    }

    #[test]
    fn test_composition_metadata_serialization() {
        let uid =
            CompositionUid::parse("550e8400-e29b-41d4-a716-446655440000::local.ehrbase.org::1")
                .unwrap();
        let template_id = TemplateId::new("vital_signs").unwrap();
        let ehr_id = EhrId::new("ehr-123").unwrap();
        let time_committed = Utc::now();

        let metadata = CompositionMetadata::new(uid, template_id, ehr_id, time_committed);

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: CompositionMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(metadata, deserialized);
    }
}

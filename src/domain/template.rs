//! Template domain model
//!
//! This module defines the Template type representing OpenEHR operational templates.

use super::ids::TemplateId;
use serde::{Deserialize, Serialize};

/// Represents an OpenEHR operational template
///
/// A template defines the structure and constraints for compositions.
/// This type holds the template identifier and metadata.
///
/// # Examples
///
/// ```
/// use atlas::domain::template::Template;
/// use atlas::domain::ids::TemplateId;
///
/// let template = Template::new(
///     TemplateId::new("IDCR - Lab Report.v1").unwrap()
/// );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Template {
    /// Unique identifier for this template
    pub id: TemplateId,

    /// Human-readable name (optional)
    pub name: Option<String>,

    /// Template version (optional)
    pub version: Option<String>,

    /// Description of the template (optional)
    pub description: Option<String>,
}

impl Template {
    /// Creates a new Template with the given ID
    ///
    /// # Arguments
    ///
    /// * `id` - The template identifier
    ///
    /// # Examples
    ///
    /// ```
    /// use atlas::domain::template::Template;
    /// use atlas::domain::ids::TemplateId;
    ///
    /// let template = Template::new(
    ///     TemplateId::new("IDCR - Lab Report.v1").unwrap()
    /// );
    /// ```
    pub fn new(id: TemplateId) -> Self {
        Self {
            id,
            name: None,
            version: None,
            description: None,
        }
    }

    /// Returns a builder for constructing a Template
    pub fn builder() -> TemplateBuilder {
        TemplateBuilder::default()
    }

    /// Generates a container name for this template
    ///
    /// # Arguments
    ///
    /// * `prefix` - Prefix to prepend to the container name
    ///
    /// # Examples
    ///
    /// ```
    /// use atlas::domain::template::Template;
    /// use atlas::domain::ids::TemplateId;
    ///
    /// let template = Template::new(
    ///     TemplateId::new("IDCR - Lab Report.v1").unwrap()
    /// );
    /// let container_name = template.container_name("compositions");
    /// assert_eq!(container_name, "compositions_idcr_lab_report_v1");
    /// ```
    pub fn container_name(&self, prefix: &str) -> String {
        self.id.to_container_name(prefix)
    }
}

/// Builder for constructing Template instances
///
/// Follows the builder pattern (TR-6.2) for ergonomic construction.
#[derive(Debug, Default)]
pub struct TemplateBuilder {
    id: Option<TemplateId>,
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
}

impl TemplateBuilder {
    /// Creates a new TemplateBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the template ID
    pub fn id(mut self, id: TemplateId) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the template name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the template version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Sets the template description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Builds the Template
    ///
    /// # Errors
    ///
    /// Returns an error if the ID is missing
    pub fn build(self) -> Result<Template, String> {
        Ok(Template {
            id: self.id.ok_or("id is required")?,
            name: self.name,
            version: self.version,
            description: self.description,
        })
    }
}

impl Default for Template {
    /// Creates a default Template with a placeholder ID
    ///
    /// Note: This is primarily for testing. Production code should use the builder.
    fn default() -> Self {
        Self {
            id: TemplateId::new("default-template-id").unwrap(),
            name: None,
            version: None,
            description: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_creation() {
        let template_id = TemplateId::new("IDCR - Lab Report.v1").unwrap();
        let template = Template::new(template_id.clone());

        assert_eq!(template.id, template_id);
        assert_eq!(template.name, None);
        assert_eq!(template.version, None);
    }

    #[test]
    fn test_template_builder() {
        let template_id = TemplateId::new("IDCR - Lab Report.v1").unwrap();

        let template = Template::builder()
            .id(template_id.clone())
            .name("Lab Report Template")
            .version("1.0")
            .description("Template for laboratory reports")
            .build()
            .unwrap();

        assert_eq!(template.id, template_id);
        assert_eq!(template.name, Some("Lab Report Template".to_string()));
        assert_eq!(template.version, Some("1.0".to_string()));
        assert_eq!(
            template.description,
            Some("Template for laboratory reports".to_string())
        );
    }

    #[test]
    fn test_template_builder_missing_id() {
        let result = Template::builder().name("Test Template").build();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("id is required"));
    }

    #[test]
    fn test_template_container_name() {
        let template = Template::new(TemplateId::new("IDCR - Lab Report.v1").unwrap());

        assert_eq!(
            template.container_name("compositions"),
            "compositions_idcr_lab_report_v1"
        );
    }

    #[test]
    fn test_template_serialization() {
        let template = Template::builder()
            .id(TemplateId::new("IDCR - Lab Report.v1").unwrap())
            .name("Lab Report")
            .build()
            .unwrap();

        let json = serde_json::to_string(&template).unwrap();
        let deserialized: Template = serde_json::from_str(&json).unwrap();

        assert_eq!(template.id, deserialized.id);
        assert_eq!(template.name, deserialized.name);
    }

    #[test]
    fn test_template_default() {
        let template = Template::default();
        assert_eq!(template.id.as_str(), "default-template-id");
        assert!(template.name.is_none());
    }

    #[test]
    fn test_template_container_name_no_prefix() {
        let template = Template::new(TemplateId::new("IDCR - Vital Signs.v1").unwrap());

        assert_eq!(template.container_name(""), "idcr_vital_signs_v1");
    }
}

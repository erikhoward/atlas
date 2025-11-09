//! Secure credential handling using the secrecy crate
//!
//! This module provides type aliases and utilities for handling sensitive
//! credentials in memory. It uses the `secrecy` crate which automatically
//! zeros memory when secrets are dropped, preventing exposure in memory dumps
//! or crash reports.
//!
//! # Security Features
//!
//! - **Automatic Zeroization**: Memory is zeroed when `Secret<T>` is dropped
//! - **Debug Protection**: Custom Debug implementation prevents logging
//! - **Explicit Access**: Must call `expose_secret()` to access the value
//!
//! # Example
//!
//! ```rust
//! use atlas::config::{SecretString, SecretValue};
//! use secrecy::{Secret, ExposeSecret};
//!
//! // Create a secret
//! let password: SecretString = Secret::new(SecretValue::from("my-password".to_string()));
//!
//! // Access the secret (only when needed)
//! let password_str = password.expose_secret();
//!
//! // Debug output is redacted
//! println!("{:?}", password); // Prints: Secret([REDACTED])
//! ```

use secrecy::{CloneableSecret, DebugSecret, Secret, SerializableSecret};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroize;

/// Newtype wrapper for String that implements the required traits for Secret
#[derive(Clone, Debug, Zeroize)]
#[zeroize(drop)]
pub struct SecretValue(String);

impl CloneableSecret for SecretValue {}
impl DebugSecret for SecretValue {}
impl SerializableSecret for SecretValue {}

impl From<String> for SecretValue {
    fn from(s: String) -> Self {
        SecretValue(s)
    }
}

impl From<SecretValue> for String {
    fn from(mut s: SecretValue) -> Self {
        std::mem::take(&mut s.0)
    }
}

impl PartialEq<str> for SecretValue {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl std::fmt::Display for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SecretValue {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl SecretValue {
    /// Check if the secret value is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Check if the secret value starts with a prefix
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }

    /// Split the secret value by a delimiter
    pub fn split(&self, delimiter: char) -> std::str::Split<'_, char> {
        self.0.split(delimiter)
    }

    /// Parse the secret value into another type
    pub fn parse<F: std::str::FromStr>(&self) -> Result<F, F::Err> {
        self.0.parse()
    }
}

impl Serialize for SecretValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SecretValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(SecretValue)
    }
}

/// Type alias for a secret string
///
/// This wraps a `SecretValue` in a `Secret` container that:
/// - Zeros the memory when dropped
/// - Prevents accidental logging via Debug
/// - Requires explicit `expose_secret()` to access
pub type SecretString = Secret<SecretValue>;

/// Helper function to create a SecretString from a String
///
/// # Arguments
///
/// * `value` - The string value to protect
///
/// # Example
///
/// ```rust
/// use atlas::config::secret_string;
///
/// let password = secret_string("my-password".to_string());
/// ```
#[inline]
pub fn secret_string(value: String) -> SecretString {
    Secret::new(SecretValue::from(value))
}

/// Helper function to create an optional SecretString from an optional String
///
/// # Arguments
///
/// * `value` - The optional string value to protect
///
/// # Example
///
/// ```rust
/// use atlas::config::secret_string_opt;
///
/// let password = secret_string_opt(Some("my-password".to_string()));
/// assert!(password.is_some());
///
/// let no_password = secret_string_opt(None);
/// assert!(no_password.is_none());
/// ```
#[inline]
pub fn secret_string_opt(value: Option<String>) -> Option<SecretString> {
    value.map(|s| Secret::new(SecretValue::from(s)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    #[test]
    fn test_secret_string_creation() {
        let secret = secret_string("test-password".to_string());
        assert_eq!(secret.expose_secret(), "test-password");
    }

    #[test]
    fn test_secret_string_opt_some() {
        let secret = secret_string_opt(Some("test-password".to_string()));
        assert!(secret.is_some());
        assert_eq!(secret.unwrap().expose_secret(), "test-password");
    }

    #[test]
    fn test_secret_string_opt_none() {
        let secret = secret_string_opt(None);
        assert!(secret.is_none());
    }

    #[test]
    fn test_secret_debug_redacted() {
        let secret = secret_string("sensitive-data".to_string());
        let debug_output = format!("{secret:?}");

        // Should not contain the actual secret
        assert!(!debug_output.contains("sensitive-data"));
        // Should contain redaction indicator
        assert!(debug_output.contains("REDACTED") || debug_output.contains("Secret"));
    }

    #[test]
    fn test_secret_serde() {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        struct TestConfig {
            password: SecretString,
        }

        let config = TestConfig {
            password: secret_string("test123".to_string()),
        };

        // Serialize
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test123"));

        // Deserialize
        let deserialized: TestConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.password.expose_secret(), "test123");
    }
}

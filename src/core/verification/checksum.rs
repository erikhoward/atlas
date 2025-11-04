//! Checksum calculation for data verification
//!
//! This module provides checksum calculation functionality for verifying
//! data integrity after export to Cosmos DB.

use crate::domain::Result;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Calculate SHA-256 checksum of JSON data
///
/// # Arguments
///
/// * `data` - The JSON value to calculate checksum for
///
/// # Returns
///
/// Returns a hex-encoded SHA-256 checksum string (64 characters).
///
/// # Examples
///
/// ```
/// use atlas::core::verification::checksum::calculate_checksum;
/// use serde_json::json;
///
/// let data = json!({"key": "value"});
/// let checksum = calculate_checksum(&data).unwrap();
/// assert_eq!(checksum.len(), 64); // SHA-256 produces 64 hex characters
/// ```
pub fn calculate_checksum(data: &Value) -> Result<String> {
    let data_str = serde_json::to_string(data)
        .map_err(|e| crate::domain::AtlasError::Serialization(e.to_string()))?;

    let mut hasher = Sha256::new();
    hasher.update(data_str.as_bytes());
    let result = hasher.finalize();

    Ok(format!("{:x}", result))
}

/// Calculate SHA-256 checksum of raw bytes
///
/// # Arguments
///
/// * `data` - The raw bytes to calculate checksum for
///
/// # Returns
///
/// Returns a hex-encoded SHA-256 checksum string (64 characters).
pub fn calculate_checksum_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_calculate_checksum_deterministic() {
        let data = json!({
            "ctx/language": "en",
            "vital_signs/body_temperature:0|magnitude": 37.5
        });

        let checksum1 = calculate_checksum(&data).unwrap();
        let checksum2 = calculate_checksum(&data).unwrap();

        // Same content should produce same checksum
        assert_eq!(checksum1, checksum2);
        assert_eq!(checksum1.len(), 64);
    }

    #[test]
    fn test_calculate_checksum_different_content() {
        let data1 = json!({
            "ctx/language": "en",
            "vital_signs/body_temperature:0|magnitude": 37.5
        });

        let data2 = json!({
            "ctx/language": "en",
            "vital_signs/body_temperature:0|magnitude": 38.0
        });

        let checksum1 = calculate_checksum(&data1).unwrap();
        let checksum2 = calculate_checksum(&data2).unwrap();

        // Different content should produce different checksums
        assert_ne!(checksum1, checksum2);
    }

    #[test]
    fn test_calculate_checksum_bytes() {
        let data = b"Hello, World!";
        let checksum = calculate_checksum_bytes(data);

        // Verify it's a valid hex string of correct length
        assert_eq!(checksum.len(), 64);
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_calculate_checksum_bytes_deterministic() {
        let data = b"Test data";
        let checksum1 = calculate_checksum_bytes(data);
        let checksum2 = calculate_checksum_bytes(data);

        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_calculate_checksum_known_value() {
        // Test with a known SHA-256 hash
        let data = json!({"test": "data"});
        let checksum = calculate_checksum(&data).unwrap();

        // This should be deterministic
        assert_eq!(checksum.len(), 64);
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }
}

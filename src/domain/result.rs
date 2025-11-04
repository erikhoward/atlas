//! Result type alias for Atlas
//!
//! This module provides a convenient Result type alias that uses AtlasError
//! as the error type, following TR-6.4.

use super::errors::AtlasError;

/// Result type alias for Atlas operations
///
/// This is a convenience type alias that uses `AtlasError` as the error type.
/// Use this throughout the codebase for fallible operations.
///
/// # Examples
///
/// ```
/// use atlas::domain::result::Result;
/// use atlas::domain::errors::AtlasError;
///
/// fn example_function() -> Result<String> {
///     Ok("success".to_string())
/// }
///
/// fn failing_function() -> Result<()> {
///     Err(AtlasError::Validation("Invalid input".to_string()))
/// }
/// ```
pub type Result<T> = std::result::Result<T, AtlasError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::errors::AtlasError;

    #[test]
    fn test_result_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        if let Ok(value) = result {
            assert_eq!(value, 42);
        }
    }

    #[test]
    fn test_result_err() {
        let result: Result<i32> = Err(AtlasError::Validation("test error".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_result_with_question_mark() -> Result<()> {
        fn inner() -> Result<i32> {
            Ok(42)
        }

        let value = inner()?;
        assert_eq!(value, 42);
        Ok(())
    }
}

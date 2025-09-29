/*!
Request and response validation for provider formats.

This module provides validation functions that use serde deserialization
to ensure JSON strings conform to provider-specific schemas.
*/

#[cfg(feature = "openai")]
pub mod openai;

#[cfg(feature = "anthropic")]
pub mod anthropic;

#[cfg(feature = "google")]
pub mod google;

#[cfg(feature = "bedrock")]
pub mod bedrock;

use serde::Deserialize;
use thiserror::Error;

/// Validation error type
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("JSON parsing failed: {0}")]
    JsonParseFailed(String),

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
}

/// Generic validation function using serde
pub fn validate_json<'a, T>(json: &'a str) -> Result<T, ValidationError>
where
    T: Deserialize<'a>,
{
    serde_json::from_str::<T>(json)
        .map_err(|e| ValidationError::DeserializationFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        name: String,
        value: i32,
    }

    #[test]
    fn test_validate_json_success() {
        let json = r#"{"name": "test", "value": 42}"#;
        let result: Result<TestStruct, _> = validate_json(json);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.name, "test");
        assert_eq!(parsed.value, 42);
    }

    #[test]
    fn test_validate_json_invalid() {
        let json = r#"{"name": "test"}"#; // missing required field
        let result: Result<TestStruct, _> = validate_json(json);
        assert!(result.is_err());
    }
}

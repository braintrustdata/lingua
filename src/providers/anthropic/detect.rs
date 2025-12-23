/*!
Anthropic format detection.

This module provides functions to detect if a payload is already in
Anthropic-compatible format by attempting to deserialize into the
Anthropic struct types.
*/

use crate::providers::anthropic::generated::CreateMessageParams;
use crate::serde_json::{self, Value};
use thiserror::Error;

/// Attempt to parse a JSON Value as Anthropic CreateMessageParams.
///
/// Returns the parsed struct if successful, or an error if the payload
/// is not valid Anthropic format.
pub fn try_parse_anthropic(payload: &Value) -> Result<CreateMessageParams, DetectionError> {
    serde_json::from_value(payload.clone())
        .map_err(|e| DetectionError::DeserializationFailed(e.to_string()))
}

/// Error type for payload detection
#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_try_parse_anthropic_valid() {
        let payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 1024
        });

        assert!(try_parse_anthropic(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_anthropic_missing_required() {
        // Missing max_tokens (required for Anthropic)
        let payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ]
        });

        assert!(try_parse_anthropic(&payload).is_err());
    }
}

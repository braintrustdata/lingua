/*!
Anthropic format detection.

This module provides functions to detect if a payload is already in
Anthropic-compatible format by attempting to deserialize into the
Anthropic struct types. This replaces heuristic-based detection with
actual struct validation.
*/

use crate::capabilities::ProviderFormat;
use crate::processing::FormatDetector;
use crate::providers::anthropic::generated::CreateMessageParams;
use crate::serde_json::{self, Value};
use thiserror::Error;

/// Detector for Anthropic Messages API format.
///
/// Detection is performed by attempting to deserialize the payload
/// into `CreateMessageParams`. If deserialization succeeds, the payload
/// is valid Anthropic format.
///
/// Anthropic has distinctive features:
/// - `max_tokens` is required (optional in OpenAI)
/// - System prompt is a top-level field, not in messages
/// - Content blocks use snake_case types: `tool_use`, `tool_result`
/// - Image blocks use `source` object (OpenAI uses `image_url`)
#[derive(Debug, Clone, Copy)]
pub struct AnthropicDetector;

impl FormatDetector for AnthropicDetector {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::Anthropic
    }

    fn detect(&self, payload: &Value) -> bool {
        // Attempt to deserialize into Anthropic struct - if it works, it's valid Anthropic format
        try_parse_anthropic(payload).is_ok()
    }

    fn priority(&self) -> u8 {
        80 // High priority - distinctive format
    }
}

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
    fn test_try_parse_anthropic_with_tool_use() {
        let payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "tool_use",
                            "id": "toolu_123",
                            "name": "get_weather",
                            "input": {"location": "SF"}
                        }
                    ]
                }
            ],
            "max_tokens": 1024
        });

        assert!(try_parse_anthropic(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_anthropic_with_tool_result() {
        let payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": "toolu_123",
                            "content": "72°F"
                        }
                    ]
                }
            ],
            "max_tokens": 1024
        });

        assert!(try_parse_anthropic(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_anthropic_with_image() {
        let payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "data": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
                                "media_type": "image/png"
                            }
                        }
                    ]
                }
            ],
            "max_tokens": 1024
        });

        assert!(try_parse_anthropic(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_anthropic_missing_max_tokens() {
        // max_tokens is required in CreateMessageParams
        let payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ]
        });

        // Deserialization fails because max_tokens is required
        assert!(try_parse_anthropic(&payload).is_err());
    }

    #[test]
    fn test_try_parse_anthropic_success() {
        let payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        assert!(try_parse_anthropic(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_anthropic_fails_for_openai_format() {
        // OpenAI-style payload with system role in messages - won't parse as Anthropic
        // because Anthropic's MessageRole enum only has User and Assistant
        let payload = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are helpful"},
                {"role": "user", "content": "Hello"}
            ]
        });

        // This should fail because "system" is not a valid Anthropic role
        assert!(try_parse_anthropic(&payload).is_err());
    }

    #[test]
    fn test_try_parse_anthropic_with_system_field() {
        // Valid Anthropic payload with system as top-level field
        let payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "system": "You are helpful",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        assert!(try_parse_anthropic(&payload).is_ok());

        // Invalid - missing max_tokens
        let invalid_payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        assert!(try_parse_anthropic(&invalid_payload).is_err());
    }

    #[test]
    fn test_detector_uses_struct_validation() {
        let detector = AnthropicDetector;

        // Valid Anthropic format
        let valid = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(detector.detect(&valid));

        // Invalid - no max_tokens
        let invalid = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(!detector.detect(&invalid));
    }
}

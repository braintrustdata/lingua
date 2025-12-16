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
    #[error("JSON parsing failed: {0}")]
    JsonParseFailed(String),

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
}

/// Detects if a JSON payload string is in Anthropic format.
///
/// This function attempts to deserialize the payload as an Anthropic
/// `CreateMessageParams`. If deserialization succeeds, the payload is valid.
///
/// # Arguments
///
/// * `payload` - JSON string to check
///
/// # Returns
///
/// * `Ok(true)` - Payload is valid Anthropic format
/// * `Ok(false)` - Payload is not Anthropic format (deserialization failed)
/// * `Err(DetectionError)` - JSON parsing error (not deserialization failure)
///
/// # Examples
///
/// ```rust
/// use lingua::providers::anthropic::is_anthropic_format;
///
/// let anthropic_payload = r#"{
///     "model": "claude-3-5-sonnet-20241022",
///     "messages": [{"role": "user", "content": "Hello"}],
///     "max_tokens": 1024
/// }"#;
///
/// assert!(is_anthropic_format(anthropic_payload).unwrap());
/// ```
pub fn is_anthropic_format(payload: &str) -> Result<bool, DetectionError> {
    // Parse JSON string first
    let value: Value = serde_json::from_str(payload)
        .map_err(|e| DetectionError::JsonParseFailed(e.to_string()))?;

    // Try to deserialize as Anthropic format
    Ok(try_parse_anthropic(&value).is_ok())
}

/// Check if a JSON Value is valid Anthropic format by attempting deserialization.
///
/// This is the primary detection function - if the payload can be deserialized
/// into `CreateMessageParams`, it IS valid Anthropic format.
pub fn is_anthropic_format_value(payload: &Value) -> bool {
    try_parse_anthropic(payload).is_ok()
}

/// More detailed format detection that distinguishes between Anthropic and OpenAI formats.
#[derive(Debug, Clone, PartialEq)]
pub enum PayloadFormat {
    /// Payload is valid Anthropic format
    Anthropic,
    /// Payload is valid OpenAI format
    OpenAI,
    /// Payload format is unknown or invalid
    Unknown,
}

/// Detects the payload format by attempting deserialization.
///
/// This function attempts to deserialize the payload as both Anthropic and OpenAI
/// formats to determine which format it matches.
///
/// # Arguments
///
/// * `payload` - JSON string to check
///
/// # Returns
///
/// * `Ok(PayloadFormat::Anthropic)` - Payload is Anthropic format
/// * `Ok(PayloadFormat::OpenAI)` - Payload is OpenAI format  
/// * `Ok(PayloadFormat::Unknown)` - Payload doesn't match either format
/// * `Err(DetectionError)` - JSON parsing error
pub fn detect_payload_format(payload: &str) -> Result<PayloadFormat, DetectionError> {
    // Parse to Value first
    let value: Value = serde_json::from_str(payload)
        .map_err(|e| DetectionError::JsonParseFailed(e.to_string()))?;

    // Try Anthropic first (it's more specific due to required max_tokens)
    if try_parse_anthropic(&value).is_ok() {
        return Ok(PayloadFormat::Anthropic);
    }

    // Try OpenAI format
    #[cfg(feature = "openai")]
    {
        use crate::providers::openai::generated::CreateChatCompletionRequestClass;

        if serde_json::from_value::<CreateChatCompletionRequestClass>(value).is_ok() {
            return Ok(PayloadFormat::OpenAI);
        }
    }

    Ok(PayloadFormat::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_anthropic_format_valid() {
        let payload = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 1024
        }"#;

        assert!(is_anthropic_format(payload).unwrap());
    }

    #[test]
    fn test_is_anthropic_format_with_tool_use() {
        let payload = r#"{
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
        }"#;

        assert!(is_anthropic_format(payload).unwrap());
    }

    #[test]
    fn test_is_anthropic_format_with_tool_result() {
        let payload = r#"{
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
        }"#;

        assert!(is_anthropic_format(payload).unwrap());
    }

    #[test]
    fn test_is_anthropic_format_with_image() {
        let payload = r#"{
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
        }"#;

        assert!(is_anthropic_format(payload).unwrap());
    }

    #[test]
    fn test_is_anthropic_format_invalid_json() {
        let payload = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
        }"#; // Invalid JSON

        assert!(is_anthropic_format(payload).is_err());
    }

    #[test]
    fn test_is_anthropic_format_missing_max_tokens() {
        // max_tokens is required in CreateMessageParams
        // This should return Ok(false) - deserialization fails but JSON is valid
        let payload = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ]
        }"#;

        // Deserialization fails because max_tokens is required, so returns Ok(false)
        assert!(!is_anthropic_format(payload).unwrap());
    }

    #[test]
    fn test_try_parse_anthropic_success() {
        use crate::serde_json::json;

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
        use crate::serde_json::json;

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
    fn test_is_anthropic_format_value() {
        use crate::serde_json::json;

        // Valid Anthropic payload
        let anthropic_payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "system": "You are helpful",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        assert!(is_anthropic_format_value(&anthropic_payload));

        // Invalid - missing max_tokens
        let invalid_payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        assert!(!is_anthropic_format_value(&invalid_payload));
    }

    #[test]
    fn test_detector_uses_struct_validation() {
        use crate::serde_json::json;

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

/*!
Anthropic format detection.

This module provides functions to detect if a payload is already in
Anthropic-compatible format by attempting to deserialize into the
Anthropic struct types. This replaces heuristic-based detection with
actual struct validation.
*/

use crate::providers::anthropic::generated::CreateMessageParams;
use crate::serde_json::{self, Value};
use thiserror::Error;

/// Fields that only exist in OpenAI format, never in Anthropic.
/// If any of these are present, the request is definitely not Anthropic format.
const OPENAI_ONLY_FIELDS: &[&str] = &[
    "stream_options",
    "n",
    "logprobs",
    "top_logprobs",
    "logit_bias",
    "response_format",
    "seed",
    "presence_penalty",
    "frequency_penalty",
    "service_tier",
    "store",
    "parallel_tool_calls",
    // OpenAI uses `stop`, Anthropic uses `stop_sequences`
    "stop",
    // OpenAI reasoning parameter
    "reasoning_effort",
    // Braintrust extension for disabling reasoning
    "reasoning_enabled",
    // OpenAI alias for max_tokens
    "max_completion_tokens",
];

/// Attempt to parse a JSON Value as Anthropic CreateMessageParams.
///
/// Returns the parsed struct if successful, or an error if the payload
/// is not valid Anthropic format. Also rejects payloads containing
/// OpenAI-specific fields to prevent misdetection.
pub fn try_parse_anthropic(payload: &Value) -> Result<CreateMessageParams, DetectionError> {
    // First, reject if any OpenAI-only fields are present
    if let Some(obj) = payload.as_object() {
        for field in OPENAI_ONLY_FIELDS {
            if obj.contains_key(*field) {
                return Err(DetectionError::OpenAIFieldPresent((*field).to_string()));
            }
        }
    }

    serde_json::from_value(payload.clone())
        .map_err(|e| DetectionError::DeserializationFailed(e.to_string()))
}

/// Error type for payload detection
#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
    #[error("OpenAI-specific field present: {0}")]
    OpenAIFieldPresent(String),
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
    fn test_try_parse_anthropic_rejects_openai_fields() {
        // Request with stream_options (OpenAI-specific) should be rejected
        let payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "stream_options": {"include_usage": true}
        });

        let result = try_parse_anthropic(&payload);
        assert!(result.is_err());
        assert!(matches!(result, Err(DetectionError::OpenAIFieldPresent(_))));

        // Request with other OpenAI-only fields
        let payload_with_n = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}],
            "n": 2
        });
        assert!(try_parse_anthropic(&payload_with_n).is_err());

        let payload_with_logprobs = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}],
            "logprobs": true
        });
        assert!(try_parse_anthropic(&payload_with_logprobs).is_err());
    }
}

/*!
Anthropic format detection.
*/

use crate::providers::anthropic::generated::CreateMessageParams;
use crate::providers::anthropic::params::first_openai_only_field;
use crate::serde_json::Value;
use serde::Deserialize;
use thiserror::Error;

pub(crate) fn is_anthropic_request(payload: &Value) -> bool {
    let Value::Object(fields) = payload else {
        return false;
    };
    let Value::Array(messages) = &payload["messages"] else {
        return false;
    };
    if !matches!(payload["model"], Value::String(_))
        || !matches!(payload["max_tokens"], Value::Number(_))
        || first_openai_only_field(payload).is_some()
        || !payload["tools"][0]["function"].is_null()
    {
        return false;
    }
    if [
        "cache_control",
        "system",
        "thinking",
        "output_config",
        "top_k",
        "stop_sequences",
    ]
    .iter()
    .any(|field| fields.contains_key(*field))
        || matches!(payload["tools"][0]["name"], Value::String(_))
    {
        return true;
    }
    if messages
        .first()
        .is_some_and(|message| message["role"] == "system" && message["output_config"].is_null())
    {
        return false;
    }
    !messages.iter().any(|message| {
        ["developer", "tool", "function"]
            .iter()
            .any(|role| message["role"] == *role)
            || ["tool_calls", "tool_call_id", "function_call", "refusal"]
                .iter()
                .any(|field| !message[*field].is_null())
            || matches!(&message["content"], Value::Array(parts) if parts.iter().any(|part| {
                ["image_url", "input_audio", "file", "refusal"]
                    .iter()
                    .any(|kind| part["type"] == *kind)
            }))
    })
}

pub fn try_parse_anthropic(payload: &Value) -> Result<CreateMessageParams, DetectionError> {
    if let Some(field) = first_openai_only_field(payload) {
        return Err(DetectionError::OpenAIFieldPresent(field.to_string()));
    }
    if !is_anthropic_request(payload) {
        return Err(DetectionError::DeserializationFailed(
            "Not an Anthropic Messages request".to_string(),
        ));
    }
    CreateMessageParams::deserialize(payload)
        .map_err(|e| DetectionError::DeserializationFailed(e.to_string()))
}

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

        let payload_with_suffix_messages = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Shared prefix"}],
            "suffix_messages": [
                [{ "role": "user", "content": "Does this mention billing?" }]
            ]
        });
        assert!(try_parse_anthropic(&payload_with_suffix_messages).is_err());

        let payload_with_functions = json!({
            "model": "gpt-4o",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}],
            "functions": [
                {
                    "name": "get_weather",
                    "parameters": {"type": "object"}
                }
            ],
            "function_call": "auto"
        });
        assert!(matches!(
            try_parse_anthropic(&payload_with_functions),
            Err(DetectionError::OpenAIFieldPresent(_))
        ));

        let payload_with_null_response_format = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}],
            "response_format": null
        });
        assert!(matches!(
            try_parse_anthropic(&payload_with_null_response_format),
            Err(DetectionError::OpenAIFieldPresent(_))
        ));
    }

    #[test]
    fn test_try_parse_anthropic_with_service_tier() {
        let payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "service_tier": "auto",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        assert!(try_parse_anthropic(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_anthropic_with_tool_search_tool() {
        let payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "tools": [
                {
                    "name": "tool_search_tool_regex",
                    "type": "tool_search_tool_regex_20251119"
                }
            ]
        });

        assert!(try_parse_anthropic(&payload).is_ok());
    }
}

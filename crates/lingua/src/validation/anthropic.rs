/*!
Anthropic format validation.
*/

use crate::providers::anthropic::generated::{CreateMessageParams, Message};
use crate::providers::anthropic::params::first_openai_only_field;
use crate::validation::{validate_json, ValidationError};

/// Validates a JSON string as an Anthropic messages request
pub fn validate_anthropic_request(json: &str) -> Result<CreateMessageParams, ValidationError> {
    let value: crate::serde_json::Value = validate_json(json)?;
    if let Some(field) =
        first_openai_only_field(&value).map_err(ValidationError::DeserializationFailed)?
    {
        return Err(ValidationError::DeserializationFailed(format!(
            "OpenAI-only field `{}` is not valid Anthropic request syntax",
            field
        )));
    }
    crate::serde_json::from_value(value)
        .map_err(|e| ValidationError::DeserializationFailed(e.to_string()))
}

/// Validates a JSON string as an Anthropic messages response
pub fn validate_anthropic_response(json: &str) -> Result<Message, ValidationError> {
    validate_json(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_anthropic_request_minimal() {
        let json = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 1024
        }"#;

        let result = validate_anthropic_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_anthropic_request_invalid() {
        let json = r#"{
            "model": "claude-3-5-sonnet-20241022"
        }"#; // missing messages and max_tokens

        let result = validate_anthropic_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_anthropic_request_rejects_openai_only_fields() {
        let json = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 1024,
            "frequency_penalty": 0.5
        }"#;

        let result = validate_anthropic_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_anthropic_request_requires_max_tokens() {
        let json = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ]
        }"#;

        let result = validate_anthropic_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_anthropic_request_rejects_gateway_openai_only_fields() {
        for field in [
            "reasoning_enabled",
            "suffix_messages",
            "chat_template_kwargs",
            "functions",
            "function_call",
        ] {
            let json = format!(
                r#"{{
                    "model": "claude-3-5-sonnet-20241022",
                    "messages": [
                        {{
                            "role": "user",
                            "content": "Hello"
                        }}
                    ],
                    "max_tokens": 1024,
                    "{}": {{}}
                }}"#,
                field
            );

            let result = validate_anthropic_request(&json);
            assert!(result.is_err(), "field should be rejected: {field}");
        }
    }

    #[test]
    fn test_validate_anthropic_request_rejects_null_openai_only_fields() {
        let json = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 1024,
            "response_format": null
        }"#;

        let result = validate_anthropic_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_anthropic_request_rejects_invalid_known_fields() {
        for (field, value) in [
            ("cache_control", r#""bad""#),
            ("container", r#"{"id":"container_123"}"#),
            ("inference_geo", r#"{"region":"us"}"#),
            ("service_tier", r#""priority""#),
        ] {
            let json = format!(
                r#"{{
                    "model": "claude-3-5-sonnet-20241022",
                    "messages": [
                        {{
                            "role": "user",
                            "content": "Hello"
                        }}
                    ],
                    "max_tokens": 1024,
                    "{}": {}
                }}"#,
                field, value
            );

            let result = validate_anthropic_request(&json);
            assert!(result.is_err(), "field should be typed: {field}");
        }
    }

    #[test]
    fn test_validate_anthropic_request_rejects_bare_tool_search_tool() {
        // tool_search_tool_* variants were removed from the generated Tool enum.
        // Bare tool_search entries (no input_schema) fail CustomTool fallback.
        let json = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 1024,
            "tools": [
                {
                    "name": "tool_search_tool_regex",
                    "type": "tool_search_tool_regex_20251119"
                }
            ]
        }"#;

        let result = validate_anthropic_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_anthropic_request_accepts_web_search_tool() {
        let json = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 1024,
            "tools": [
                {
                    "name": "web_search",
                    "type": "web_search_20260318"
                }
            ]
        }"#;

        let result = validate_anthropic_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_anthropic_request_accepts_web_fetch_tool() {
        let json = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 1024,
            "tools": [
                {
                    "name": "web_fetch",
                    "type": "web_fetch_20260318"
                }
            ]
        }"#;

        let result = validate_anthropic_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_anthropic_response_minimal() {
        let json = r#"{
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "Hello!"
                }
            ],
            "model": "claude-3-5-sonnet-20241022",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 20
            }
        }"#;

        let result = validate_anthropic_response(json);
        assert!(result.is_ok());
    }
}

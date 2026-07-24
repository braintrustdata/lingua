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
    fn test_validate_anthropic_request_accepts_tool_search_tool() {
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

    #[test]
    fn test_validate_anthropic_response_model_context_window_exceeded() {
        // Regression: the `model_context_window_exceeded` stop reason was added to
        // the Anthropic spec (StopReason enum). A response carrying it must remain a
        // valid Anthropic response and deserialize into the typed StopReason variant.
        use crate::providers::anthropic::generated::StopReason;

        let json = r#"{
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "partial"
                }
            ],
            "model": "claude-3-5-sonnet-20241022",
            "stop_reason": "model_context_window_exceeded",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 20
            }
        }"#;

        let message = validate_anthropic_response(json).expect("valid Anthropic response");
        assert_eq!(
            message.stop_reason,
            Some(StopReason::ModelContextWindowExceeded)
        );

        // Round-trip: re-serializing preserves the wire value exactly.
        let reserialized = crate::serde_json::to_value(&message).unwrap();
        assert_eq!(
            reserialized.get("stop_reason").and_then(|v| v.as_str()),
            Some("model_context_window_exceeded")
        );
    }

    #[test]
    fn test_validate_anthropic_response_refusal_general_harms() {
        // Regression: the `general_harms` refusal category was added to the Anthropic
        // spec (RefusalCategory enum). A refusal response naming it must remain valid
        // and deserialize into the typed RefusalCategory variant.
        use crate::providers::anthropic::generated::{RefusalCategory, StopReason};

        let json = r#"{
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [],
            "model": "claude-3-5-sonnet-20241022",
            "stop_reason": "refusal",
            "stop_details": {
                "type": "refusal",
                "category": "general_harms",
                "explanation": "declined"
            },
            "usage": {
                "input_tokens": 10,
                "output_tokens": 0
            }
        }"#;

        let message = validate_anthropic_response(json).expect("valid Anthropic response");
        assert_eq!(message.stop_reason, Some(StopReason::Refusal));
        let details = message.stop_details.expect("stop_details present");
        assert_eq!(details.category, Some(RefusalCategory::GeneralHarms));
    }

    #[test]
    fn test_validate_anthropic_response_all_stop_reasons_still_accepted() {
        // Regression: widening StopReason must not drop any previously-valid wire value.
        for reason in [
            "end_turn",
            "max_tokens",
            "stop_sequence",
            "tool_use",
            "pause_turn",
            "refusal",
            "model_context_window_exceeded",
        ] {
            let json = format!(
                r#"{{
                    "id": "msg_123",
                    "type": "message",
                    "role": "assistant",
                    "content": [],
                    "model": "claude-3-5-sonnet-20241022",
                    "stop_reason": "{reason}",
                    "usage": {{ "input_tokens": 1, "output_tokens": 1 }}
                }}"#
            );
            assert!(
                validate_anthropic_response(&json).is_ok(),
                "stop_reason {reason} should remain a valid Anthropic response"
            );
        }
    }
}

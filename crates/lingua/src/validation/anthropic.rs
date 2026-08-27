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
    use crate::providers::anthropic::generated;

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
            // `container` accepts a bare id string or a ContainerParams object; an object whose
            // `id` is not a string matches neither arm.
            ("container", r#"{"id":5}"#),
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
    fn test_validate_anthropic_request_accepts_container_id_string() {
        let json = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 1024,
            "container": "container_123"
        }"#;

        let request = validate_anthropic_request(json).expect("bare container id is valid");
        assert_eq!(
            request.container,
            Some(generated::ContainerUnion::ContainerId(
                "container_123".to_string()
            ))
        );
    }

    #[test]
    fn test_validate_anthropic_request_accepts_container_params_with_skills() {
        let json = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 1024,
            "container": {
                "id": "container_123",
                "skills": [
                    { "skill_id": "pdf", "type": "anthropic", "version": "latest" }
                ]
            }
        }"#;

        let request = validate_anthropic_request(json).expect("container params are valid");
        let Some(generated::ContainerUnion::ContainerParams(params)) = request.container else {
            panic!(
                "expected typed container params, got {:?}",
                request.container
            );
        };
        assert_eq!(params.id.as_deref(), Some("container_123"));
        let skills = params.skills.expect("skills should be preserved");
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].skill_id, "pdf");
        assert_eq!(skills[0].skill_params_type, generated::SkillType::Anthropic);
        assert_eq!(skills[0].version.as_deref(), Some("latest"));
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
    fn test_validate_anthropic_request_rejects_scalar_toolset_configs() {
        for toolset_type in ["browser_toolset_20260801", "computer_toolset_20260801"] {
            let json = format!(
                r#"{{
                    "model": "claude-opus-4-1",
                    "messages": [{{"role": "user", "content": "Hello"}}],
                    "max_tokens": 16,
                    "tools": [{{"type": "{toolset_type}", "configs": 5}}]
                }}"#
            );

            assert!(
                validate_anthropic_request(&json).is_err(),
                "scalar configs should be rejected for {toolset_type}"
            );
        }
    }

    #[test]
    fn test_validate_anthropic_request_accepts_typed_toolset_configs() {
        let json = r#"{
            "model": "claude-opus-4-1",
            "messages": [{"role": "user", "content": "Hello"}],
            "max_tokens": 16,
            "tools": [{
                "type": "browser_toolset_20260801",
                "configs": {
                    "navigate": {"enabled": false},
                    "screenshot": {"defer_loading": true}
                }
            }]
        }"#;

        assert!(validate_anthropic_request(json).is_ok());
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

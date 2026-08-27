/*!
Anthropic format validation.
*/

use crate::providers::anthropic::generated::{self, CreateMessageParams, Message};
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
    let request: CreateMessageParams = crate::serde_json::from_value(value)
        .map_err(|e| ValidationError::DeserializationFailed(e.to_string()))?;

    for message in &request.messages {
        let generated::MessageContent::InputContentBlockArray(blocks) = &message.content else {
            continue;
        };
        for block in blocks {
            if block.transformations.is_some()
                && block.input_content_block_type != generated::InputContentBlockType::Image
            {
                return Err(ValidationError::DeserializationFailed(format!(
                    "Anthropic image transformations are not valid on {:?} content blocks",
                    block.input_content_block_type
                )));
            }
            if block.toolset_name.is_some()
                && !matches!(
                    block.input_content_block_type,
                    generated::InputContentBlockType::ToolUse
                        | generated::InputContentBlockType::ToolResult
                )
            {
                return Err(ValidationError::DeserializationFailed(format!(
                    "Anthropic toolset_name is not valid on {:?} content blocks",
                    block.input_content_block_type
                )));
            }
            if let Some(generated::InputContentBlockContent::BlockArray(nested_blocks)) =
                &block.content
            {
                for nested in nested_blocks {
                    let (nested, allows_transformations) = match nested {
                        generated::Block::BrowserState(_) => continue,
                        generated::Block::Image(block) => (block, true),
                        generated::Block::Document(block)
                        | generated::Block::SearchResult(block)
                        | generated::Block::Text(block)
                        | generated::Block::ToolReference(block)
                        | generated::Block::WebSearchResult(block) => (block, false),
                    };
                    if nested.transformations.is_some() && !allows_transformations {
                        return Err(ValidationError::DeserializationFailed(
                            "Anthropic image transformations are not valid on this nested content block"
                                .to_string(),
                        ));
                    }
                }
            }
        }
    }

    Ok(request)
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
    fn test_validate_anthropic_request_requires_browser_state_tabs() {
        let json = r#"{
            "model": "claude-opus-4-1",
            "messages": [{
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": "toolu_123",
                    "content": [{"type": "browser_state"}]
                }]
            }],
            "max_tokens": 16
        }"#;

        assert!(validate_anthropic_request(json).is_err());
    }

    #[test]
    fn test_validate_anthropic_request_requires_file_source_ids() {
        for block_type in ["image", "document"] {
            let json = format!(
                r#"{{
                    "model": "claude-opus-4-1",
                    "messages": [{{
                        "role": "user",
                        "content": [{{
                            "type": "{block_type}",
                            "source": {{"type": "file"}}
                        }}]
                    }}],
                    "max_tokens": 16
                }}"#
            );

            assert!(
                validate_anthropic_request(&json).is_err(),
                "{block_type} file sources require file_id"
            );
        }
    }

    #[test]
    fn test_validate_anthropic_request_restricts_transformations_to_images() {
        for block in [
            r#"{"type":"text","text":"Hello","transformations":{}}"#,
            r#"{
                "type":"document",
                "source":{
                    "type":"base64",
                    "media_type":"application/pdf",
                    "data":"cGRm"
                },
                "transformations":{}
            }"#,
        ] {
            let json = format!(
                r#"{{
                    "model":"claude-opus-4-1",
                    "messages":[{{"role":"user","content":[{block}]}}],
                    "max_tokens":16
                }}"#
            );
            assert!(validate_anthropic_request(&json).is_err());
        }

        let valid_image = r#"{
            "model":"claude-opus-4-1",
            "messages":[{
                "role":"user",
                "content":[{
                    "type":"image",
                    "source":{
                        "type":"base64",
                        "media_type":"image/png",
                        "data":"aW1hZ2U="
                    },
                    "transformations":{"oversized_image":"downsize"}
                }]
            }],
            "max_tokens":16
        }"#;
        assert!(validate_anthropic_request(valid_image).is_ok());
    }

    #[test]
    fn test_validate_anthropic_request_restricts_toolset_name_to_tool_blocks() {
        let invalid = r#"{
            "model":"claude-opus-4-1",
            "messages":[{
                "role":"user",
                "content":[{"type":"text","text":"Hello","toolset_name":"browser"}]
            }],
            "max_tokens":16
        }"#;
        assert!(validate_anthropic_request(invalid).is_err());

        let valid = r#"{
            "model":"claude-opus-4-1",
            "messages":[{
                "role":"assistant",
                "content":[{
                    "type":"tool_use",
                    "id":"toolu_123",
                    "name":"navigate",
                    "toolset_name":"browser",
                    "input":{"url":"https://example.com"}
                }]
            }],
            "max_tokens":16
        }"#;
        assert!(validate_anthropic_request(valid).is_ok());
    }

    #[test]
    fn test_validate_anthropic_request_restricts_nested_transformations_to_images() {
        let invalid = r#"{
            "model":"claude-opus-4-1",
            "messages":[{
                "role":"user",
                "content":[{
                    "type":"tool_result",
                    "tool_use_id":"toolu_123",
                    "content":[{
                        "type":"text",
                        "text":"done",
                        "transformations":{"oversized_image":"error"}
                    }]
                }]
            }],
            "max_tokens":16
        }"#;
        assert!(validate_anthropic_request(invalid).is_err());

        let valid = r#"{
            "model":"claude-opus-4-1",
            "messages":[{
                "role":"user",
                "content":[{
                    "type":"tool_result",
                    "tool_use_id":"toolu_123",
                    "content":[{
                        "type":"image",
                        "source":{
                            "type":"base64",
                            "media_type":"image/png",
                            "data":"aW1hZ2U="
                        },
                        "transformations":{"oversized_image":"error"}
                    }]
                }]
            }],
            "max_tokens":16
        }"#;
        assert!(validate_anthropic_request(valid).is_ok());
    }

    #[test]
    fn test_validate_anthropic_request_rejects_unknown_toolset_fields() {
        for tool in [
            r#"{"type":"browser_toolset_20260801","name":"browser"}"#,
            r#"{"type":"computer_toolset_20260801","display_number":1}"#,
        ] {
            let json = format!(
                r#"{{
                    "model":"claude-opus-4-1",
                    "messages":[{{"role":"user","content":"Hello"}}],
                    "tools":[{tool}],
                    "max_tokens":16
                }}"#
            );
            assert!(validate_anthropic_request(&json).is_err());
        }
    }

    #[test]
    fn test_validate_anthropic_request_requires_browser_state_change_fields() {
        let json = r#"{
            "model": "claude-opus-4-1",
            "messages": [{
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": "toolu_123",
                    "content": [{
                        "type": "browser_state",
                        "tabs": [],
                        "state_changes": [{
                            "type": "download_started",
                            "download_id": "download_123"
                        }]
                    }]
                }]
            }],
            "max_tokens": 16
        }"#;

        assert!(validate_anthropic_request(json).is_err());
    }

    #[test]
    fn test_validate_anthropic_request_accepts_typed_browser_state() {
        let json = r#"{
            "model": "claude-opus-4-1",
            "messages": [{
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": "toolu_123",
                    "content": [{
                        "type": "browser_state",
                        "tabs": [{
                            "tab_id": "tab_123",
                            "title": "Example",
                            "url": "https://example.com",
                            "active": true
                        }],
                        "state_changes": [{
                            "type": "tab_opened",
                            "tab_id": "tab_123"
                        }]
                    }]
                }]
            }],
            "max_tokens": 16
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

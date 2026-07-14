/*!
Anthropic format detection.

This module provides functions to detect if a payload is already in
Anthropic-compatible format by attempting to deserialize into the
Anthropic struct types. This replaces heuristic-based detection with
actual struct validation.
*/

use crate::providers::anthropic::capabilities;
use crate::providers::anthropic::generated::{
    CreateMessageParams, InputContentBlockType, InputMessage, MessageContent, MessageRole,
};
use crate::providers::anthropic::params::first_openai_only_field;
use crate::serde_json::{self, Value};
use thiserror::Error;

/// Attempt to parse a JSON Value as Anthropic CreateMessageParams.
///
/// Returns the parsed struct if successful, or an error if the payload
/// is not valid Anthropic format. Also rejects payloads containing
/// OpenAI-specific fields to prevent misdetection. This also validates the
/// model-gated `messages[].role = "system"` feature and its placement rules.
pub fn try_parse_anthropic(payload: &Value) -> Result<CreateMessageParams, DetectionError> {
    reject_openai_only_fields(payload)?;

    let request: CreateMessageParams = serde_json::from_value(payload.clone())
        .map_err(|e| DetectionError::DeserializationFailed(e.to_string()))?;
    validate_system_message_support_and_placement(&request.model, &request.messages)?;
    Ok(request)
}

/// Attempt to parse a JSON Value as an Anthropic Messages-shaped source request.
///
/// This is intentionally less strict than `try_parse_anthropic`: it accepts
/// Claude Code-style Messages payloads with system-role entries even when the
/// requested model belongs to another provider. Native Anthropic passthrough
/// still uses `try_parse_anthropic`.
pub fn try_parse_anthropic_source(payload: &Value) -> Result<CreateMessageParams, DetectionError> {
    reject_openai_only_fields(payload)?;

    let request: CreateMessageParams = serde_json::from_value(payload.clone())
        .map_err(|e| DetectionError::DeserializationFailed(e.to_string()))?;

    validate_system_message_placement(&request.messages)?;

    if system_messages_present(&request.messages)
        && !capabilities::supports_mid_conversation_system_messages(&request.model)
        && !has_anthropic_source_markers(&request)
    {
        return Err(DetectionError::UnsupportedSystemRoleMessages {
            model: request.model.clone(),
        });
    }

    Ok(request)
}

fn reject_openai_only_fields(payload: &Value) -> Result<(), DetectionError> {
    if let Some(field) =
        first_openai_only_field(payload).map_err(DetectionError::DeserializationFailed)?
    {
        return Err(DetectionError::OpenAIFieldPresent(field.to_string()));
    }

    Ok(())
}

fn has_anthropic_source_markers(request: &CreateMessageParams) -> bool {
    request.cache_control.is_some()
        || request.system.is_some()
        || request.thinking.is_some()
        || request.output_config.is_some()
        || request.top_k.is_some()
        || request
            .stop_sequences
            .as_ref()
            .is_some_and(|stop_sequences| !stop_sequences.is_empty())
        || request
            .tools
            .as_ref()
            .is_some_and(|tools| !tools.is_empty())
}

pub(crate) fn system_messages_are_supported_and_well_placed(
    model: &str,
    raw_messages: &Value,
) -> Result<bool, DetectionError> {
    let messages: Vec<InputMessage> = serde_json::from_value(raw_messages.clone())
        .map_err(|e| DetectionError::DeserializationFailed(e.to_string()))?;
    Ok(validate_system_message_support_and_placement(model, &messages).is_ok())
}

fn validate_system_message_support_and_placement(
    model: &str,
    messages: &[InputMessage],
) -> Result<(), DetectionError> {
    if !system_messages_present(messages) {
        return Ok(());
    }

    if !capabilities::supports_mid_conversation_system_messages(model) {
        return Err(DetectionError::UnsupportedSystemRoleMessages {
            model: model.to_string(),
        });
    }

    validate_system_message_placement(messages)?;

    Ok(())
}

fn system_messages_present(messages: &[InputMessage]) -> bool {
    messages
        .iter()
        .any(|message| matches!(message.role, MessageRole::System))
}

fn validate_system_message_placement(messages: &[InputMessage]) -> Result<(), DetectionError> {
    if mid_conversation_system_messages_have_valid_placement(messages) {
        Ok(())
    } else {
        Err(DetectionError::InvalidSystemRolePlacement)
    }
}

fn assistant_message_ends_with_server_tool_use(message: &InputMessage) -> bool {
    if !matches!(message.role, MessageRole::Assistant) {
        return false;
    }

    let MessageContent::InputContentBlockArray(blocks) = &message.content else {
        return false;
    };

    blocks.last().is_some_and(|block| {
        matches!(
            block.input_content_block_type,
            InputContentBlockType::ServerToolUse
        )
    })
}

fn mid_conversation_system_messages_have_valid_placement(messages: &[InputMessage]) -> bool {
    for (index, message) in messages.iter().enumerate() {
        if !matches!(message.role, MessageRole::System) {
            continue;
        }

        let Some(previous) = index.checked_sub(1).and_then(|prev| messages.get(prev)) else {
            return false;
        };
        let previous_allows_system = matches!(previous.role, MessageRole::User)
            || assistant_message_ends_with_server_tool_use(previous);
        if !previous_allows_system {
            return false;
        }

        let next_allows_system = messages
            .get(index + 1)
            .is_none_or(|next| matches!(next.role, MessageRole::Assistant));
        if !next_allows_system {
            return false;
        }
    }

    true
}

/// Error type for payload detection
#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
    #[error("OpenAI-specific field present: {0}")]
    OpenAIFieldPresent(String),
    #[error("system-role messages are not supported for model: {model}")]
    UnsupportedSystemRoleMessages { model: String },
    #[error("system-role message placement is invalid")]
    InvalidSystemRolePlacement,
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
        // OpenAI-style leading system role in messages is not supported or
        // correctly placed for non-Opus 4.8 Anthropic requests.
        let payload = json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 1024,
            "messages": [
                {"role": "system", "content": "You are helpful"},
                {"role": "user", "content": "Hello"}
            ]
        });

        assert!(matches!(
            try_parse_anthropic(&payload),
            Err(DetectionError::UnsupportedSystemRoleMessages { .. })
        ));
    }

    #[test]
    fn test_try_parse_anthropic_allows_opus_4_8_mid_conversation_system() {
        let payload = json!({
            "model": "claude-opus-4-8",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Review this function."},
                {"role": "system", "content": "From now on, include type annotations."}
            ]
        });

        assert!(try_parse_anthropic(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_anthropic_rejects_invalid_opus_4_8_system_placement() {
        let payload = json!({
            "model": "claude-opus-4-8",
            "max_tokens": 1024,
            "messages": [
                {"role": "system", "content": "You are helpful"},
                {"role": "user", "content": "Hello"}
            ]
        });

        assert!(matches!(
            try_parse_anthropic(&payload),
            Err(DetectionError::InvalidSystemRolePlacement)
        ));
    }

    #[test]
    fn test_try_parse_anthropic_source_allows_claude_code_system_messages_for_other_models() {
        let payload = json!({
            "model": "gpt-5.5",
            "max_tokens": 1024,
            "system": [
                {"type": "text", "text": "You are running inside Claude Code."}
            ],
            "messages": [
                {"role": "user", "content": "Hello"},
                {"role": "system", "content": "Use the provided tools exactly."}
            ],
            "tools": [
                {
                    "name": "Read",
                    "description": "Read a file.",
                    "input_schema": {
                        "type": "object",
                        "properties": {"file_path": {"type": "string"}},
                        "required": ["file_path"]
                    }
                }
            ],
            "thinking": {"type": "adaptive"},
            "output_config": {"effort": "high"}
        });

        assert!(try_parse_anthropic(&payload).is_err());
        assert!(try_parse_anthropic_source(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_anthropic_source_rejects_plain_openai_system_messages() {
        let payload = json!({
            "model": "gpt-5.5",
            "max_tokens": 1024,
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello"}
            ]
        });

        assert!(try_parse_anthropic_source(&payload).is_err());
    }

    #[test]
    fn test_try_parse_anthropic_source_rejects_invalid_leading_system_message_with_marker() {
        let payload = json!({
            "model": "gpt-5.5",
            "max_tokens": 1024,
            "system": "Top-level instructions.",
            "messages": [
                {"role": "system", "content": "Leading system instructions that must not be dropped."},
                {"role": "user", "content": "Hello"}
            ]
        });

        assert!(matches!(
            try_parse_anthropic_source(&payload),
            Err(DetectionError::InvalidSystemRolePlacement)
        ));
    }

    #[test]
    fn test_try_parse_anthropic_source_rejects_empty_tools_as_marker() {
        let payload = json!({
            "model": "gpt-5.5",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Hello"},
                {"role": "system", "content": "Use the provided tools exactly."}
            ],
            "tools": []
        });

        assert!(matches!(
            try_parse_anthropic_source(&payload),
            Err(DetectionError::UnsupportedSystemRoleMessages { .. })
        ));
    }

    #[test]
    fn test_try_parse_anthropic_source_allows_cache_control_marker() {
        let payload = json!({
            "model": "gpt-5.5",
            "max_tokens": 1024,
            "cache_control": {"type": "ephemeral"},
            "messages": [
                {"role": "user", "content": "Hello"},
                {"role": "system", "content": "Use the provided context."}
            ]
        });

        assert!(try_parse_anthropic(&payload).is_err());
        assert!(try_parse_anthropic_source(&payload).is_ok());
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
    fn test_try_parse_anthropic_with_tool_search_tool_no_longer_in_tool_enum() {
        // tool_search_tool_* variants were removed from the generated Tool enum
        // in the 2026-07 spec update. Requests carrying them in the tools array
        // fail deserialization because the schema no longer defines them.
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

        assert!(try_parse_anthropic(&payload).is_err());
    }
}

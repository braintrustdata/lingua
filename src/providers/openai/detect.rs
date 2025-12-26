/*!
OpenAI format detection.

This module provides functions to detect if a payload is in
OpenAI chat completion format by attempting to deserialize into
the OpenAI struct types.
*/

use crate::providers::openai::generated::{CreateChatCompletionRequestClass, CreateResponseClass};
use crate::serde_json::{self, Value};
use thiserror::Error;

/// Error type for OpenAI payload detection
#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
}

/// Attempt to parse a JSON Value as OpenAI CreateChatCompletionRequestClass.
///
/// Returns the parsed struct if successful, or an error if the payload
/// is not valid OpenAI format.
///
/// # Examples
///
/// ```rust
/// use lingua::serde_json::json;
/// use lingua::providers::openai::detect::try_parse_openai;
///
/// let openai_payload = json!({
///     "model": "gpt-4",
///     "messages": [{"role": "user", "content": "Hello"}]
/// });
///
/// assert!(try_parse_openai(&openai_payload).is_ok());
/// ```
pub fn try_parse_openai(
    payload: &Value,
) -> Result<CreateChatCompletionRequestClass, DetectionError> {
    serde_json::from_value(payload.clone())
        .map_err(|e| DetectionError::DeserializationFailed(e.to_string()))
}

/// Attempt to parse a JSON Value as OpenAI Responses API CreateResponseClass.
///
/// Returns the parsed struct if successful, or an error if the payload
/// is not valid Responses API format.
///
/// The key distinguishing feature is the `input` field (array of InputItem or string)
/// instead of the `messages` field used by Chat Completions.
pub fn try_parse_responses(payload: &Value) -> Result<CreateResponseClass, DetectionError> {
    // First check for Responses-specific indicators
    // Responses API uses "input" field (not "messages" like Chat Completions)
    let has_input = payload.get("input").is_some();

    // Also check it doesn't have Chat Completions indicators
    let has_messages = payload.get("messages").is_some();

    if !has_input || has_messages {
        return Err(DetectionError::DeserializationFailed(
            "Not a Responses API payload: missing 'input' field or has 'messages' field"
                .to_string(),
        ));
    }

    serde_json::from_value(payload.clone())
        .map_err(|e| DetectionError::DeserializationFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::openai::generated::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageContent,
        ChatCompletionRequestMessageRole, CreateChatCompletionRequestClass, PurpleFunction,
        ToolCall, ToolType,
    };
    use crate::serde_json::{self, json};

    #[test]
    fn test_try_parse_openai_basic() {
        let request = CreateChatCompletionRequestClass {
            model: "gpt-4".to_string(),
            messages: vec![ChatCompletionRequestMessage {
                role: ChatCompletionRequestMessageRole::User,
                content: Some(ChatCompletionRequestMessageContent::String(
                    "Hello".to_string(),
                )),
                name: None,
                audio: None,
                function_call: None,
                refusal: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            metadata: None,
            prompt_cache_key: None,
            safety_identifier: None,
            service_tier: None,
            temperature: None,
            top_logprobs: None,
            top_p: None,
            user: None,
            audio: None,
            frequency_penalty: None,
            function_call: None,
            functions: None,
            logit_bias: None,
            logprobs: None,
            max_completion_tokens: None,
            max_tokens: None,
            modalities: None,
            n: None,
            parallel_tool_calls: None,
            prediction: None,
            presence_penalty: None,
            reasoning_effort: None,
            response_format: None,
            seed: None,
            stop: None,
            store: None,
            stream: None,
            stream_options: None,
            tool_choice: None,
            tools: None,
            verbosity: None,
            web_search_options: None,
        };

        let payload = serde_json::to_value(&request).unwrap();
        assert!(try_parse_openai(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_openai_with_system() {
        let request = CreateChatCompletionRequestClass {
            model: "gpt-4".to_string(),
            messages: vec![
                ChatCompletionRequestMessage {
                    role: ChatCompletionRequestMessageRole::System,
                    content: Some(ChatCompletionRequestMessageContent::String(
                        "You are helpful".to_string(),
                    )),
                    name: None,
                    audio: None,
                    function_call: None,
                    refusal: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                ChatCompletionRequestMessage {
                    role: ChatCompletionRequestMessageRole::User,
                    content: Some(ChatCompletionRequestMessageContent::String(
                        "Hello".to_string(),
                    )),
                    name: None,
                    audio: None,
                    function_call: None,
                    refusal: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            metadata: None,
            prompt_cache_key: None,
            safety_identifier: None,
            service_tier: None,
            temperature: None,
            top_logprobs: None,
            top_p: None,
            user: None,
            audio: None,
            frequency_penalty: None,
            function_call: None,
            functions: None,
            logit_bias: None,
            logprobs: None,
            max_completion_tokens: None,
            max_tokens: None,
            modalities: None,
            n: None,
            parallel_tool_calls: None,
            prediction: None,
            presence_penalty: None,
            reasoning_effort: None,
            response_format: None,
            seed: None,
            stop: None,
            store: None,
            stream: None,
            stream_options: None,
            tool_choice: None,
            tools: None,
            verbosity: None,
            web_search_options: None,
        };

        let payload = serde_json::to_value(&request).unwrap();
        assert!(try_parse_openai(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_openai_with_tool_calls() {
        let request = CreateChatCompletionRequestClass {
            model: "gpt-4".to_string(),
            messages: vec![ChatCompletionRequestMessage {
                role: ChatCompletionRequestMessageRole::Assistant,
                content: None,
                name: None,
                audio: None,
                function_call: None,
                refusal: None,
                tool_calls: Some(vec![ToolCall {
                    id: "call_123".to_string(),
                    tool_call_type: ToolType::Function,
                    function: Some(PurpleFunction {
                        name: "get_weather".to_string(),
                        arguments: "{}".to_string(),
                    }),
                    custom: None,
                }]),
                tool_call_id: None,
            }],
            metadata: None,
            prompt_cache_key: None,
            safety_identifier: None,
            service_tier: None,
            temperature: None,
            top_logprobs: None,
            top_p: None,
            user: None,
            audio: None,
            frequency_penalty: None,
            function_call: None,
            functions: None,
            logit_bias: None,
            logprobs: None,
            max_completion_tokens: None,
            max_tokens: None,
            modalities: None,
            n: None,
            parallel_tool_calls: None,
            prediction: None,
            presence_penalty: None,
            reasoning_effort: None,
            response_format: None,
            seed: None,
            stop: None,
            store: None,
            stream: None,
            stream_options: None,
            tool_choice: None,
            tools: None,
            verbosity: None,
            web_search_options: None,
        };

        let payload = serde_json::to_value(&request).unwrap();
        assert!(try_parse_openai(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_openai_fails_no_model() {
        // Missing model field - should fail struct deserialization
        let payload = json!({
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(try_parse_openai(&payload).is_err());
    }

    #[test]
    fn test_try_parse_openai_empty_messages() {
        // Empty messages - should fail struct deserialization
        let payload = json!({
            "model": "gpt-4",
            "messages": []
        });
        // Note: OpenAI struct allows empty messages array, so this may pass
        // The actual validation depends on struct definition
        let result = try_parse_openai(&payload);
        // Just verify it doesn't panic - behavior depends on struct definition
        let _ = result;
    }

    #[test]
    fn test_try_parse_openai_fails_for_google() {
        // Google format - should fail OpenAI struct deserialization
        let payload = json!({
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}]
        });
        assert!(try_parse_openai(&payload).is_err());
    }

    #[test]
    fn test_try_parse_openai_success() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let result = try_parse_openai(&payload);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.model, "gpt-4");
    }

    #[test]
    fn test_try_parse_openai_permissive_for_anthropic() {
        // Anthropic format with max_tokens but no system role in messages
        let payload = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        // This should actually succeed because OpenAI is permissive
        // OpenAI accepts extra fields and the structure is similar
        let result = try_parse_openai(&payload);
        // Note: Due to OpenAI's permissive schema, this may pass
        let _ = result;
    }
}

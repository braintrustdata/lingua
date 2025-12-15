/*!
OpenAI format detection.

This module provides functions to detect if a payload is in
OpenAI chat completion format.
*/

use crate::capabilities::ProviderFormat;
use crate::processing::FormatDetector;
use crate::serde_json::Value;

/// Detector for OpenAI Chat Completions API format.
///
/// OpenAI format is the most permissive and serves as a fallback.
/// It detects payloads with:
/// - `messages` array with `role` and `content`/`tool_calls`
/// - `model` field
#[derive(Debug, Clone, Copy)]
pub struct OpenAIDetector;

impl FormatDetector for OpenAIDetector {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::OpenAI
    }

    fn detect(&self, payload: &Value) -> bool {
        is_openai_format(payload)
    }

    fn priority(&self) -> u8 {
        50 // Lowest priority - fallback format
    }
}

/// Check if payload is in OpenAI format.
///
/// This is the most permissive check and serves as a fallback.
///
/// Indicators:
/// - Has "messages" array
/// - Has "model" field
/// - Messages have "role" and "content" fields
///
/// # Examples
///
/// ```rust
/// use lingua::serde_json::json;
/// use lingua::providers::openai::detect::is_openai_format;
///
/// let openai_payload = json!({
///     "model": "gpt-4",
///     "messages": [{"role": "user", "content": "Hello"}]
/// });
///
/// assert!(is_openai_format(&openai_payload));
/// ```
pub fn is_openai_format(payload: &Value) -> bool {
    // Must have messages array
    let has_messages = payload
        .get("messages")
        .and_then(|v| v.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false);

    // Should have model field
    let has_model = payload.get("model").is_some();

    // Basic validation: messages have role and content
    if has_messages {
        if let Some(messages) = payload.get("messages").and_then(|v| v.as_array()) {
            let valid_messages = messages.iter().all(|msg| {
                msg.get("role").is_some()
                    && (msg.get("content").is_some() || msg.get("tool_calls").is_some())
            });
            return valid_messages && has_model;
        }
    }

    false
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
    fn test_openai_format_basic() {
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
        assert!(is_openai_format(&payload));
    }

    #[test]
    fn test_openai_format_with_system() {
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
        assert!(is_openai_format(&payload));
    }

    #[test]
    fn test_openai_format_with_tool_calls() {
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
        assert!(is_openai_format(&payload));
    }

    #[test]
    fn test_not_openai_format_no_model() {
        // Raw JSON for invalid payloads to ensure detection rejects them
        let payload = json!({
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(!is_openai_format(&payload));
    }

    #[test]
    fn test_not_openai_format_empty_messages() {
        // Raw JSON for invalid payloads to ensure detection rejects them
        let payload = json!({
            "model": "gpt-4",
            "messages": []
        });
        assert!(!is_openai_format(&payload));
    }

    #[test]
    fn test_not_openai_format_google() {
        // Raw JSON for non-OpenAI formats to ensure detection rejects them
        let payload = json!({
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}]
        });
        assert!(!is_openai_format(&payload));
    }
}

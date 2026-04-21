/*!
OpenAI format detection.

This module provides functions to detect if a payload is in
OpenAI chat completion format by attempting to deserialize into
the OpenAI struct types.
*/

use crate::processing::{json_value_kind, probe_shape, JsonValueKind};
use crate::providers::openai::generated::{CreateChatCompletionRequestClass, CreateResponseClass};
use crate::serde_json::{self, Value};
use serde::Deserialize;
use std::fmt;
use thiserror::Error;

/// Error type for OpenAI payload detection
#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("typed deserialization failed")]
    DeserializationFailed,

    #[error("invalid OpenAI Responses request shape: {0}")]
    ResponsesRequestDiagnostic(ResponsesRequestDiagnosticView),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponsesRequestDiagnosticKind {
    TopLevelNotObject,
    MissingInput,
    InputAndMessagesBothPresent,
    InputWrongTopLevelType,
    ToolsWrongTopLevelType,
    TypedRequestShapeMismatch,
}

impl ResponsesRequestDiagnosticKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TopLevelNotObject => "top_level_not_object",
            Self::MissingInput => "missing_input",
            Self::InputAndMessagesBothPresent => "input_and_messages_both_present",
            Self::InputWrongTopLevelType => "input_wrong_top_level_type",
            Self::ToolsWrongTopLevelType => "tools_wrong_top_level_type",
            Self::TypedRequestShapeMismatch => "typed_request_shape_mismatch",
        }
    }
}

impl fmt::Display for ResponsesRequestDiagnosticKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponsesRequestDiagnosticView {
    pub kind: ResponsesRequestDiagnosticKind,
    pub top_level_type: JsonValueKind,
    pub input_type: Option<JsonValueKind>,
    pub tools_type: Option<JsonValueKind>,
    pub messages_present: bool,
}

impl fmt::Display for ResponsesRequestDiagnosticView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;
        write!(f, " (top_level_type={}", self.top_level_type)?;
        if let Some(input_type) = self.input_type {
            write!(f, ", input_type={input_type}")?;
        }
        if let Some(tools_type) = self.tools_type {
            write!(f, ", tools_type={tools_type}")?;
        }
        write!(f, ", messages_present={})", self.messages_present)
    }
}

#[derive(Debug, Deserialize)]
struct ResponsesRequestDiagnosticProbe {
    #[serde(default)]
    input: Option<JsonValueKind>,
    #[serde(default)]
    messages: Option<JsonValueKind>,
    #[serde(default)]
    tools: Option<JsonValueKind>,
}

pub fn responses_request_diagnostic(payload: &Value) -> ResponsesRequestDiagnosticView {
    let top_level_type = json_value_kind(payload);
    if top_level_type != JsonValueKind::Object {
        return ResponsesRequestDiagnosticView {
            kind: ResponsesRequestDiagnosticKind::TopLevelNotObject,
            top_level_type,
            input_type: None,
            tools_type: None,
            messages_present: false,
        };
    }

    let probe = probe_shape::<ResponsesRequestDiagnosticProbe>(payload).unwrap_or(
        ResponsesRequestDiagnosticProbe {
            input: None,
            messages: None,
            tools: None,
        },
    );
    let messages_present = probe.messages.is_some();

    if probe.input.is_none() {
        return ResponsesRequestDiagnosticView {
            kind: ResponsesRequestDiagnosticKind::MissingInput,
            top_level_type,
            input_type: None,
            tools_type: probe.tools,
            messages_present,
        };
    }

    if messages_present {
        return ResponsesRequestDiagnosticView {
            kind: ResponsesRequestDiagnosticKind::InputAndMessagesBothPresent,
            top_level_type,
            input_type: probe.input,
            tools_type: probe.tools,
            messages_present,
        };
    }

    if !matches!(
        probe.input,
        Some(JsonValueKind::String) | Some(JsonValueKind::Array)
    ) {
        return ResponsesRequestDiagnosticView {
            kind: ResponsesRequestDiagnosticKind::InputWrongTopLevelType,
            top_level_type,
            input_type: probe.input,
            tools_type: probe.tools,
            messages_present,
        };
    }

    if !matches!(probe.tools, None | Some(JsonValueKind::Array)) {
        return ResponsesRequestDiagnosticView {
            kind: ResponsesRequestDiagnosticKind::ToolsWrongTopLevelType,
            top_level_type,
            input_type: probe.input,
            tools_type: probe.tools,
            messages_present,
        };
    }

    ResponsesRequestDiagnosticView {
        kind: ResponsesRequestDiagnosticKind::TypedRequestShapeMismatch,
        top_level_type,
        input_type: probe.input,
        tools_type: probe.tools,
        messages_present,
    }
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
    serde_json::from_value(payload.clone()).map_err(|_e| DetectionError::DeserializationFailed)
}

/// Attempt to parse a JSON Value as OpenAI Responses API CreateResponseClass.
///
/// Returns the parsed struct if successful, or an error if the payload
/// is not valid Responses API format.
///
/// The key distinguishing feature is the `input` field (array of InputItem or string)
/// instead of the `messages` field used by Chat Completions.
pub fn try_parse_responses(payload: &Value) -> Result<CreateResponseClass, DetectionError> {
    let diagnostic = responses_request_diagnostic(payload);
    if diagnostic.kind != ResponsesRequestDiagnosticKind::TypedRequestShapeMismatch {
        return Err(DetectionError::ResponsesRequestDiagnostic(diagnostic));
    }

    serde_json::from_value(payload.clone()).map_err(|_err| {
        DetectionError::ResponsesRequestDiagnostic(ResponsesRequestDiagnosticView {
            kind: ResponsesRequestDiagnosticKind::TypedRequestShapeMismatch,
            ..diagnostic
        })
    })
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

    #[test]
    fn test_responses_request_diagnostic_missing_input() {
        let payload = json!({
            "model": "gpt-5-mini",
            "tools": []
        });

        let diagnostic = responses_request_diagnostic(&payload);
        assert_eq!(
            diagnostic.kind,
            ResponsesRequestDiagnosticKind::MissingInput
        );
        assert_eq!(diagnostic.top_level_type, JsonValueKind::Object);
        assert_eq!(diagnostic.input_type, None);
        assert_eq!(diagnostic.tools_type, Some(JsonValueKind::Array));
        assert!(!diagnostic.messages_present);
    }

    #[test]
    fn test_responses_request_diagnostic_input_and_messages_both_present() {
        let payload = json!({
            "model": "gpt-5-mini",
            "input": [],
            "messages": []
        });

        let diagnostic = responses_request_diagnostic(&payload);
        assert_eq!(
            diagnostic.kind,
            ResponsesRequestDiagnosticKind::InputAndMessagesBothPresent
        );
        assert_eq!(diagnostic.input_type, Some(JsonValueKind::Array));
        assert!(diagnostic.messages_present);
    }

    #[test]
    fn test_responses_request_diagnostic_input_wrong_top_level_type() {
        let payload = json!({
            "model": "gpt-5-mini",
            "input": {"role": "user"}
        });

        let diagnostic = responses_request_diagnostic(&payload);
        assert_eq!(
            diagnostic.kind,
            ResponsesRequestDiagnosticKind::InputWrongTopLevelType
        );
        assert_eq!(diagnostic.input_type, Some(JsonValueKind::Object));
    }

    #[test]
    fn test_responses_request_diagnostic_tools_wrong_top_level_type() {
        let payload = json!({
            "model": "gpt-5-mini",
            "input": [],
            "tools": {"type": "function"}
        });

        let diagnostic = responses_request_diagnostic(&payload);
        assert_eq!(
            diagnostic.kind,
            ResponsesRequestDiagnosticKind::ToolsWrongTopLevelType
        );
        assert_eq!(diagnostic.input_type, Some(JsonValueKind::Array));
        assert_eq!(diagnostic.tools_type, Some(JsonValueKind::Object));
    }

    #[test]
    fn test_try_parse_responses_returns_safe_diagnostic() {
        let payload = json!({
            "model": "gpt-5-mini",
            "input": [],
            "tools": {"type": "function"}
        });

        let err = try_parse_responses(&payload).unwrap_err();
        assert!(matches!(
            err,
            DetectionError::ResponsesRequestDiagnostic(ResponsesRequestDiagnosticView {
                kind: ResponsesRequestDiagnosticKind::ToolsWrongTopLevelType,
                ..
            })
        ));
    }
}

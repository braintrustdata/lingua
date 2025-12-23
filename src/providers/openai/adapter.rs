/*!
OpenAI provider adapters for chat completions and responses API.

This module provides two adapters:
- `OpenAIAdapter` for the standard Chat Completions API
- `ResponsesAdapter` for the Responses API (used by reasoning models like o1)
*/

use crate::capabilities::ProviderFormat;
use crate::processing::adapters::{
    collect_extras, insert_opt_bool, insert_opt_f64, insert_opt_i64, insert_opt_value,
    ProviderAdapter,
};
use crate::processing::transform::TransformError;
use crate::providers::openai::generated::{
    ChatCompletionRequestMessage, ChatCompletionResponseMessage, CreateChatCompletionRequestClass,
    CreateResponseClass, InputItem, InputItemContent, InputItemRole, InputItemType, Instructions,
};
use crate::providers::openai::{try_parse_openai, try_parse_responses, universal_to_responses_input};
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{AssistantContent, Message, UserContent};
use crate::universal::{
    FinishReason, UniversalParams, UniversalRequest, UniversalResponse, UniversalStreamChunk,
    UniversalStreamChoice, UniversalUsage,
};

/// Known request fields for OpenAI Chat Completions API.
/// These are fields extracted into UniversalRequest/UniversalParams.
/// Fields not in this list go into `extras` for passthrough.
const OPENAI_KNOWN_KEYS: &[&str] = &[
    "model",
    "messages",
    "temperature",
    "top_p",
    "max_tokens",
    "max_completion_tokens",
    "stop",
    "tools",
    "tool_choice",
    "response_format",
    "seed",
    "presence_penalty",
    "frequency_penalty",
    "stream",
    // OpenAI-specific fields (not in UniversalParams) go to extras:
    // stream_options, n, logprobs, top_logprobs, logit_bias,
    // user, store, metadata, parallel_tool_calls, service_tier
];

/// Known request fields for OpenAI Responses API.
/// These are fields extracted into UniversalRequest/UniversalParams.
/// Fields not in this list go into `extras` for passthrough.
const RESPONSES_KNOWN_KEYS: &[&str] = &[
    "model",
    "input",
    "temperature",
    "top_p",
    "max_output_tokens",
    "tools",
    "tool_choice",
    "stream",
    // Responses-specific fields (not in UniversalParams) go to extras:
    // instructions, stop, response_format, seed, presence_penalty,
    // frequency_penalty, reasoning, truncation, user, store,
    // metadata, parallel_tool_calls
];

/// Adapter for OpenAI Chat Completions API.
pub struct OpenAIAdapter;

impl ProviderAdapter for OpenAIAdapter {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::OpenAI
    }

    fn directory_name(&self) -> &'static str {
        "chat-completions"
    }

    fn display_name(&self) -> &'static str {
        "ChatCompletions"
    }

    fn detect_request(&self, payload: &Value) -> bool {
        try_parse_openai(payload).is_ok()
    }

    fn request_to_universal(&self, payload: &Value) -> Result<UniversalRequest, TransformError> {
        let request: CreateChatCompletionRequestClass = serde_json::from_value(payload.clone())
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let messages = <Vec<Message> as TryFromLLM<Vec<_>>>::try_from(request.messages)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let params = UniversalParams {
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: None, // OpenAI doesn't support top_k
            max_tokens: request.max_tokens.or(request.max_completion_tokens),
            stop: request.stop.and_then(|s| serde_json::to_value(s).ok()),
            tools: request.tools.and_then(|t| serde_json::to_value(t).ok()),
            tool_choice: request.tool_choice.and_then(|t| serde_json::to_value(t).ok()),
            response_format: request.response_format.and_then(|r| serde_json::to_value(r).ok()),
            seed: request.seed,
            presence_penalty: request.presence_penalty,
            frequency_penalty: request.frequency_penalty,
            stream: request.stream,
        };

        Ok(UniversalRequest {
            model: Some(request.model),
            messages,
            params,
            extras: collect_extras(payload, OPENAI_KNOWN_KEYS),
        })
    }

    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
        let model = req.model.as_ref().ok_or(TransformError::ValidationFailed {
            target: ProviderFormat::OpenAI,
            reason: "missing model".to_string(),
        })?;

        let openai_messages: Vec<ChatCompletionRequestMessage> =
            <Vec<ChatCompletionRequestMessage> as TryFromLLM<Vec<Message>>>::try_from(
                req.messages.clone(),
            )
            .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

        let mut obj = Map::new();
        obj.insert("model".into(), Value::String(model.clone()));
        obj.insert(
            "messages".into(),
            serde_json::to_value(openai_messages)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
        );

        // Insert params
        insert_opt_f64(&mut obj, "temperature", req.params.temperature);
        insert_opt_f64(&mut obj, "top_p", req.params.top_p);
        insert_opt_i64(&mut obj, "max_tokens", req.params.max_tokens);
        insert_opt_value(&mut obj, "stop", req.params.stop.clone());
        insert_opt_value(&mut obj, "tools", req.params.tools.clone());
        insert_opt_value(&mut obj, "tool_choice", req.params.tool_choice.clone());
        insert_opt_value(&mut obj, "response_format", req.params.response_format.clone());
        insert_opt_i64(&mut obj, "seed", req.params.seed);
        insert_opt_f64(&mut obj, "presence_penalty", req.params.presence_penalty);
        insert_opt_f64(&mut obj, "frequency_penalty", req.params.frequency_penalty);
        insert_opt_bool(&mut obj, "stream", req.params.stream);

        // Merge extras (provider-specific fields)
        for (k, v) in &req.extras {
            obj.insert(k.clone(), v.clone());
        }

        Ok(Value::Object(obj))
    }

    fn apply_defaults(&self, _req: &mut UniversalRequest) {
        // OpenAI doesn't require any specific defaults
    }

    fn detect_response(&self, payload: &Value) -> bool {
        // OpenAI chat completion response has choices[].message and object="chat.completion"
        payload.get("choices").and_then(Value::as_array).is_some()
            && payload
                .get("object")
                .and_then(Value::as_str)
                .is_some_and(|o| o == "chat.completion")
    }

    fn response_to_universal(&self, payload: &Value) -> Result<UniversalResponse, TransformError> {
        let choices = payload
            .get("choices")
            .and_then(Value::as_array)
            .ok_or_else(|| TransformError::ToUniversalFailed("missing choices".to_string()))?;

        let mut messages = Vec::new();
        let mut finish_reason = None;

        for choice in choices {
            if let Some(msg_val) = choice.get("message") {
                let response_msg: ChatCompletionResponseMessage =
                    serde_json::from_value(msg_val.clone())
                        .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                let universal =
                    <Message as TryFromLLM<&ChatCompletionResponseMessage>>::try_from(&response_msg)
                        .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                messages.push(universal);
            }

            // Get finish_reason from first choice
            if finish_reason.is_none() {
                if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
                    finish_reason = Some(FinishReason::from_str(reason));
                }
            }
        }

        let usage = payload.get("usage").map(|u| UniversalUsage {
            input_tokens: u.get("prompt_tokens").and_then(Value::as_i64),
            output_tokens: u.get("completion_tokens").and_then(Value::as_i64),
        });

        Ok(UniversalResponse {
            model: payload
                .get("model")
                .and_then(Value::as_str)
                .map(String::from),
            messages,
            usage,
            finish_reason,
            extras: Map::new(), // TODO: preserve extras if needed
        })
    }

    fn response_from_universal(&self, resp: &UniversalResponse) -> Result<Value, TransformError> {
        let finish_reason = self
            .map_finish_reason(resp.finish_reason.as_ref())
            .unwrap_or_else(|| "stop".to_string());

        let choices: Vec<Value> = resp
            .messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let response_msg =
                    <ChatCompletionResponseMessage as TryFromLLM<&Message>>::try_from(msg)
                        .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

                let message_value = serde_json::to_value(&response_msg)
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "index": i,
                    "message": message_value,
                    "finish_reason": finish_reason
                }))
            })
            .collect::<Result<Vec<_>, TransformError>>()?;

        let usage = resp.usage.as_ref().map(|u| {
            let input = u.input_tokens.unwrap_or(0);
            let output = u.output_tokens.unwrap_or(0);
            serde_json::json!({
                "prompt_tokens": input,
                "completion_tokens": output,
                "total_tokens": input + output
            })
        });

        let mut obj = serde_json::json!({
            "id": resp.extras.get("id").and_then(Value::as_str).unwrap_or("transformed"),
            "object": "chat.completion",
            "created": resp.extras.get("created").and_then(Value::as_i64).unwrap_or(0),
            "model": resp.model.as_deref().unwrap_or("transformed"),
            "choices": choices
        });

        if let Some(usage_val) = usage {
            obj.as_object_mut().unwrap().insert("usage".into(), usage_val);
        }

        Ok(obj)
    }

    fn map_finish_reason(&self, reason: Option<&FinishReason>) -> Option<String> {
        reason.map(|r| match r {
            FinishReason::Stop => "stop".to_string(),
            FinishReason::Length => "length".to_string(),
            FinishReason::ToolCalls => "tool_calls".to_string(),
            FinishReason::ContentFilter => "content_filter".to_string(),
            FinishReason::Other(s) => s.clone(),
        })
    }

    // =========================================================================
    // Streaming response handling
    // =========================================================================

    fn detect_stream_response(&self, payload: &Value) -> bool {
        // OpenAI streaming chunk has object="chat.completion.chunk"
        // or has choices array with delta field
        if let Some(obj) = payload.get("object").and_then(Value::as_str) {
            if obj == "chat.completion.chunk" {
                return true;
            }
        }

        // Fallback: check for choices with delta
        if let Some(choices) = payload.get("choices").and_then(Value::as_array) {
            if choices.iter().any(|c| c.get("delta").is_some()) {
                return true;
            }
        }

        false
    }

    fn stream_to_universal(
        &self,
        payload: &Value,
    ) -> Result<Option<UniversalStreamChunk>, TransformError> {
        // OpenAI is the canonical format, so this is mostly direct mapping
        let choices = payload
            .get("choices")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| {
                        let index = c.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;
                        let delta = c.get("delta").cloned();
                        let finish_reason = c
                            .get("finish_reason")
                            .and_then(Value::as_str)
                            .map(String::from);
                        Some(UniversalStreamChoice {
                            index,
                            delta,
                            finish_reason,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Extract usage if present (usually only on final chunk)
        let usage = payload.get("usage").map(|u| UniversalUsage {
            input_tokens: u.get("prompt_tokens").and_then(Value::as_i64),
            output_tokens: u.get("completion_tokens").and_then(Value::as_i64),
        });

        Ok(Some(UniversalStreamChunk::new(
            payload.get("id").and_then(Value::as_str).map(String::from),
            payload
                .get("model")
                .and_then(Value::as_str)
                .map(String::from),
            choices,
            payload.get("created").and_then(Value::as_u64),
            usage,
        )))
    }

    fn stream_from_universal(
        &self,
        chunk: &UniversalStreamChunk,
    ) -> Result<Value, TransformError> {
        // Convert back to OpenAI streaming format
        if chunk.is_keep_alive() {
            // Return empty chunk for keep-alive
            return Ok(serde_json::json!({
                "object": "chat.completion.chunk",
                "choices": []
            }));
        }

        let choices: Vec<Value> = chunk
            .choices
            .iter()
            .map(|c| {
                let mut choice = serde_json::json!({
                    "index": c.index,
                    "delta": c.delta.clone().unwrap_or(Value::Object(Map::new()))
                });
                if let Some(ref reason) = c.finish_reason {
                    choice
                        .as_object_mut()
                        .unwrap()
                        .insert("finish_reason".into(), Value::String(reason.clone()));
                } else {
                    choice
                        .as_object_mut()
                        .unwrap()
                        .insert("finish_reason".into(), Value::Null);
                }
                choice
            })
            .collect();

        let mut obj = serde_json::json!({
            "object": "chat.completion.chunk",
            "choices": choices
        });

        let obj_map = obj.as_object_mut().unwrap();
        if let Some(ref id) = chunk.id {
            obj_map.insert("id".into(), Value::String(id.clone()));
        }
        if let Some(ref model) = chunk.model {
            obj_map.insert("model".into(), Value::String(model.clone()));
        }
        if let Some(created) = chunk.created {
            obj_map.insert("created".into(), Value::Number(created.into()));
        }
        if let Some(ref usage) = chunk.usage {
            let input = usage.input_tokens.unwrap_or(0);
            let output = usage.output_tokens.unwrap_or(0);
            obj_map.insert(
                "usage".into(),
                serde_json::json!({
                    "prompt_tokens": input,
                    "completion_tokens": output,
                    "total_tokens": input + output
                }),
            );
        }

        Ok(obj)
    }
}

/// Adapter for OpenAI Responses API (used by reasoning models like o1).
pub struct ResponsesAdapter;

impl ProviderAdapter for ResponsesAdapter {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::Responses
    }

    fn directory_name(&self) -> &'static str {
        "responses"
    }

    fn display_name(&self) -> &'static str {
        "Responses"
    }

    fn detect_request(&self, payload: &Value) -> bool {
        try_parse_responses(payload).is_ok()
    }

    fn request_to_universal(&self, payload: &Value) -> Result<UniversalRequest, TransformError> {
        let request: CreateResponseClass = serde_json::from_value(payload.clone())
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Extract input items from the request
        let input_items: Vec<InputItem> = match request.input {
            Some(Instructions::InputItemArray(items)) => items,
            Some(Instructions::String(s)) => {
                // Single string input - create a user message InputItem
                vec![InputItem {
                    input_item_type: Some(InputItemType::Message),
                    role: Some(InputItemRole::User),
                    content: Some(InputItemContent::String(s)),
                    ..Default::default()
                }]
            }
            None => vec![],
        };

        let messages = <Vec<Message> as TryFromLLM<Vec<InputItem>>>::try_from(input_items)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let params = UniversalParams {
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: None,
            max_tokens: request.max_output_tokens,
            stop: None, // Responses API doesn't use stop
            tools: request.tools.and_then(|t| serde_json::to_value(t).ok()),
            tool_choice: request.tool_choice.and_then(|t| serde_json::to_value(t).ok()),
            response_format: None, // Different structure in Responses API
            seed: None, // Responses API uses different randomness control
            presence_penalty: None, // Responses API doesn't support penalties
            frequency_penalty: None,
            stream: request.stream,
        };

        Ok(UniversalRequest {
            model: request.model,  // Already Option<String> in CreateResponseClass
            messages,
            params,
            extras: collect_extras(payload, RESPONSES_KNOWN_KEYS),
        })
    }

    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
        let model = req.model.as_ref().ok_or(TransformError::ValidationFailed {
            target: ProviderFormat::Responses,
            reason: "missing model".to_string(),
        })?;

        // Use existing conversion with 1:N Tool message expansion
        let input_items = universal_to_responses_input(&req.messages)
            .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

        let mut obj = Map::new();
        obj.insert("model".into(), Value::String(model.clone()));
        obj.insert(
            "input".into(),
            serde_json::to_value(input_items)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
        );

        // Insert params (note: some params like temperature are not supported by reasoning models)
        // We still include them for passthrough, the API will validate
        insert_opt_f64(&mut obj, "temperature", req.params.temperature);
        insert_opt_f64(&mut obj, "top_p", req.params.top_p);
        insert_opt_i64(&mut obj, "max_output_tokens", req.params.max_tokens);
        insert_opt_value(&mut obj, "tools", req.params.tools.clone());
        insert_opt_value(&mut obj, "tool_choice", req.params.tool_choice.clone());
        insert_opt_f64(&mut obj, "presence_penalty", req.params.presence_penalty);
        insert_opt_f64(&mut obj, "frequency_penalty", req.params.frequency_penalty);
        insert_opt_bool(&mut obj, "stream", req.params.stream);

        // Merge extras
        for (k, v) in &req.extras {
            obj.insert(k.clone(), v.clone());
        }

        Ok(Value::Object(obj))
    }

    fn apply_defaults(&self, _req: &mut UniversalRequest) {
        // Responses API doesn't require any specific defaults
    }

    fn detect_response(&self, payload: &Value) -> bool {
        // Responses API response has output[] array and object="response"
        payload.get("output").and_then(Value::as_array).is_some()
            && payload
                .get("object")
                .and_then(Value::as_str)
                .is_some_and(|o| o == "response")
    }

    fn response_to_universal(&self, payload: &Value) -> Result<UniversalResponse, TransformError> {
        let output = payload
            .get("output")
            .and_then(Value::as_array)
            .ok_or_else(|| TransformError::ToUniversalFailed("missing output".to_string()))?;

        // Convert output items to messages
        // Responses API has multiple output types: message, function_call, reasoning, etc.
        let mut messages: Vec<Message> = Vec::new();
        let mut tool_calls: Vec<Value> = Vec::new();

        for item in output {
            let item_type = item.get("type").and_then(Value::as_str);

            match item_type {
                Some("message") => {
                    // Message type - extract text content
                    if let Some(content) = item.get("content") {
                        if let Some(content_arr) = content.as_array() {
                            let text: String = content_arr
                                .iter()
                                .filter_map(|c| {
                                    if c.get("type").and_then(Value::as_str) == Some("output_text") {
                                        c.get("text").and_then(Value::as_str).map(String::from)
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("");
                            if !text.is_empty() {
                                messages.push(Message::Assistant {
                                    content: AssistantContent::String(text),
                                    id: None,
                                });
                            }
                        }
                    }
                }
                Some("function_call") => {
                    // Function call - collect for later conversion to tool calls
                    tool_calls.push(item.clone());
                }
                _ => {
                    // Skip reasoning and other types for now
                }
            }
        }

        // If we have tool calls but no messages, create an assistant message with tool calls
        if !tool_calls.is_empty() && messages.is_empty() {
            // Convert function_call items to tool call format
            use crate::universal::message::{AssistantContentPart, ToolCallArguments};
            let parts: Vec<AssistantContentPart> = tool_calls
                .iter()
                .filter_map(|tc| {
                    let name = tc.get("name").and_then(Value::as_str)?;
                    let call_id = tc.get("call_id").and_then(Value::as_str)?;
                    let arguments = tc.get("arguments").and_then(Value::as_str)?;

                    // Try to parse arguments as JSON, fall back to invalid string
                    let args = serde_json::from_str::<Map<String, Value>>(arguments)
                        .map(ToolCallArguments::Valid)
                        .unwrap_or_else(|_| ToolCallArguments::Invalid(arguments.to_string()));

                    Some(AssistantContentPart::ToolCall {
                        tool_call_id: call_id.to_string(),
                        tool_name: name.to_string(),
                        arguments: args,
                        provider_options: None,
                        provider_executed: None,
                    })
                })
                .collect();

            if !parts.is_empty() {
                messages.push(Message::Assistant {
                    content: AssistantContent::Array(parts),
                    id: None,
                });
            }
        }

        // If still no messages, try output_text field as fallback
        if messages.is_empty() {
            if let Some(text) = payload.get("output_text").and_then(Value::as_str) {
                if !text.is_empty() {
                    messages.push(Message::Assistant {
                        content: AssistantContent::String(text.to_string()),
                        id: None,
                    });
                }
            }
        }

        // Map status to finish_reason
        let finish_reason = payload
            .get("status")
            .and_then(Value::as_str)
            .map(FinishReason::from_str);

        let usage = payload.get("usage").map(|u| UniversalUsage {
            input_tokens: u.get("input_tokens").and_then(Value::as_i64),
            output_tokens: u.get("output_tokens").and_then(Value::as_i64),
        });

        Ok(UniversalResponse {
            model: payload
                .get("model")
                .and_then(Value::as_str)
                .map(String::from),
            messages,
            usage,
            finish_reason,
            extras: Map::new(),
        })
    }

    fn response_from_universal(&self, resp: &UniversalResponse) -> Result<Value, TransformError> {
        // Build Responses API response format
        let output: Vec<Value> = resp
            .messages
            .iter()
            .map(|msg| {
                let text = match msg {
                    Message::Assistant { content, .. } => match content {
                        AssistantContent::String(s) => s.clone(),
                        AssistantContent::Array(_) => String::new(), // TODO: extract text from parts
                    },
                    Message::User { content } => match content {
                        UserContent::String(s) => s.clone(),
                        UserContent::Array(_) => String::new(),
                    },
                    _ => String::new(),
                };

                serde_json::json!({
                    "type": "message",
                    "role": "assistant",
                    "content": [{
                        "type": "output_text",
                        "text": text
                    }]
                })
            })
            .collect();

        let status = self
            .map_finish_reason(resp.finish_reason.as_ref())
            .unwrap_or_else(|| "completed".to_string());

        // Build response with all required fields for TheResponseObject
        let mut obj = serde_json::json!({
            "id": resp.extras.get("id").and_then(Value::as_str).unwrap_or("resp_transformed"),
            "object": "response",
            "model": resp.model.as_deref().unwrap_or("transformed"),
            "output": output,
            "status": status,
            "created_at": 0.0,
            "tool_choice": "none",
            "tools": [],
            "parallel_tool_calls": false
        });

        if let Some(usage) = &resp.usage {
            let input = usage.input_tokens.unwrap_or(0);
            let output = usage.output_tokens.unwrap_or(0);
            obj.as_object_mut().unwrap().insert(
                "usage".into(),
                serde_json::json!({
                    "input_tokens": input,
                    "output_tokens": output,
                    "total_tokens": input + output,
                    "input_tokens_details": {
                        "cached_tokens": 0
                    },
                    "output_tokens_details": {
                        "reasoning_tokens": 0
                    }
                }),
            );
        }

        Ok(obj)
    }

    fn map_finish_reason(&self, reason: Option<&FinishReason>) -> Option<String> {
        reason.map(|r| match r {
            FinishReason::Stop => "completed".to_string(),
            FinishReason::Length => "incomplete".to_string(),
            FinishReason::ToolCalls => "completed".to_string(), // Tool calls also complete
            FinishReason::ContentFilter => "incomplete".to_string(),
            FinishReason::Other(s) => s.clone(),
        })
    }

    // =========================================================================
    // Streaming response handling
    // =========================================================================

    fn detect_stream_response(&self, payload: &Value) -> bool {
        // Responses API streaming has type field starting with "response."
        payload
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|t| t.starts_with("response."))
    }

    fn stream_to_universal(
        &self,
        payload: &Value,
    ) -> Result<Option<UniversalStreamChunk>, TransformError> {
        let event_type = payload
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| TransformError::ToUniversalFailed("missing type field".to_string()))?;

        match event_type {
            "response.output_text.delta" => {
                // Text delta - extract from delta field
                let text = payload.get("delta").and_then(Value::as_str).unwrap_or("");
                let output_index = payload
                    .get("output_index")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32;

                Ok(Some(UniversalStreamChunk::new(
                    None,
                    None,
                    vec![UniversalStreamChoice {
                        index: output_index,
                        delta: Some(serde_json::json!({
                            "role": "assistant",
                            "content": text
                        })),
                        finish_reason: None,
                    }],
                    None,
                    None,
                )))
            }

            "response.completed" => {
                // Final event with usage
                let response = payload.get("response");
                let usage = response.and_then(|r| r.get("usage")).map(|u| UniversalUsage {
                    input_tokens: u.get("input_tokens").and_then(Value::as_i64),
                    output_tokens: u.get("output_tokens").and_then(Value::as_i64),
                });

                let model = response
                    .and_then(|r| r.get("model"))
                    .and_then(Value::as_str)
                    .map(String::from);

                let id = response
                    .and_then(|r| r.get("id"))
                    .and_then(Value::as_str)
                    .map(String::from);

                Ok(Some(UniversalStreamChunk::new(
                    id,
                    model,
                    vec![UniversalStreamChoice {
                        index: 0,
                        delta: Some(serde_json::json!({})),
                        finish_reason: Some("stop".to_string()),
                    }],
                    None,
                    usage,
                )))
            }

            "response.incomplete" => {
                // Incomplete response - typically due to length
                let response = payload.get("response");
                let usage = response.and_then(|r| r.get("usage")).map(|u| UniversalUsage {
                    input_tokens: u.get("input_tokens").and_then(Value::as_i64),
                    output_tokens: u.get("output_tokens").and_then(Value::as_i64),
                });

                Ok(Some(UniversalStreamChunk::new(
                    None,
                    None,
                    vec![UniversalStreamChoice {
                        index: 0,
                        delta: Some(serde_json::json!({})),
                        finish_reason: Some("length".to_string()),
                    }],
                    None,
                    usage,
                )))
            }

            "response.created" | "response.in_progress" => {
                // Initial metadata events - extract model/id
                let response = payload.get("response");
                let model = response
                    .and_then(|r| r.get("model"))
                    .and_then(Value::as_str)
                    .map(String::from);
                let id = response
                    .and_then(|r| r.get("id"))
                    .and_then(Value::as_str)
                    .map(String::from);

                Ok(Some(UniversalStreamChunk::new(
                    id,
                    model,
                    vec![UniversalStreamChoice {
                        index: 0,
                        delta: Some(serde_json::json!({"role": "assistant", "content": ""})),
                        finish_reason: None,
                    }],
                    None,
                    None,
                )))
            }

            // All other events are metadata/keep-alive
            _ => Ok(Some(UniversalStreamChunk::keep_alive())),
        }
    }

    fn stream_from_universal(
        &self,
        chunk: &UniversalStreamChunk,
    ) -> Result<Value, TransformError> {
        if chunk.is_keep_alive() {
            // Return a generic in_progress event
            return Ok(serde_json::json!({
                "type": "response.in_progress",
                "sequence_number": 0
            }));
        }

        // Check for finish chunk
        let has_finish = chunk
            .choices
            .first()
            .and_then(|c| c.finish_reason.as_ref())
            .is_some();

        if has_finish {
            let finish_reason = chunk.choices.first().and_then(|c| c.finish_reason.as_ref());
            let status = match finish_reason.map(|r| r.as_str()) {
                Some("stop") => "completed",
                Some("length") => "incomplete",
                _ => "completed",
            };

            let mut response = serde_json::json!({
                "id": chunk.id.as_deref().unwrap_or("resp_transformed"),
                "object": "response",
                "model": chunk.model.as_deref().unwrap_or("transformed"),
                "status": status,
                "output": []
            });

            if let Some(usage) = &chunk.usage {
                response.as_object_mut().unwrap().insert(
                    "usage".into(),
                    serde_json::json!({
                        "input_tokens": usage.input_tokens.unwrap_or(0),
                        "output_tokens": usage.output_tokens.unwrap_or(0),
                        "total_tokens": usage.input_tokens.unwrap_or(0) + usage.output_tokens.unwrap_or(0)
                    }),
                );
            }

            return Ok(serde_json::json!({
                "type": if status == "completed" { "response.completed" } else { "response.incomplete" },
                "response": response
            }));
        }

        // Check for content delta
        if let Some(choice) = chunk.choices.first() {
            if let Some(delta) = &choice.delta {
                if let Some(content) = delta.get("content").and_then(Value::as_str) {
                    return Ok(serde_json::json!({
                        "type": "response.output_text.delta",
                        "output_index": choice.index,
                        "content_index": 0,
                        "delta": content
                    }));
                }
            }
        }

        // Fallback - return output_text.delta with empty content
        Ok(serde_json::json!({
            "type": "response.output_text.delta",
            "output_index": 0,
            "content_index": 0,
            "delta": ""
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_openai_detect_request() {
        let adapter = OpenAIAdapter;
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(adapter.detect_request(&payload));
    }

    #[test]
    fn test_openai_passthrough() {
        let adapter = OpenAIAdapter;
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let universal = adapter.request_to_universal(&payload).unwrap();
        assert_eq!(universal.model, Some("gpt-4".to_string()));
        assert_eq!(universal.messages.len(), 1);

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert_eq!(reconstructed.get("model").unwrap(), "gpt-4");
        assert!(reconstructed.get("messages").is_some());
    }

    #[test]
    fn test_openai_preserves_extras() {
        let adapter = OpenAIAdapter;
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}],
            "user": "test-user-123",
            "custom_field": "should_be_preserved"
        });

        let universal = adapter.request_to_universal(&payload).unwrap();
        assert!(universal.extras.contains_key("user"));
        assert!(universal.extras.contains_key("custom_field"));

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert_eq!(reconstructed.get("user").unwrap(), "test-user-123");
        assert_eq!(reconstructed.get("custom_field").unwrap(), "should_be_preserved");
    }

    #[test]
    fn test_responses_detect_request() {
        let adapter = ResponsesAdapter;
        let payload = json!({
            "model": "o1",
            "input": [{"role": "user", "content": "Hello"}]
        });
        assert!(adapter.detect_request(&payload));
    }
}

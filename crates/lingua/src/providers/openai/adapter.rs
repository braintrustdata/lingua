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
use crate::providers::openai::capabilities::{OpenAICapabilities, TargetProvider};
use crate::providers::openai::generated::{
    AllowedToolsFunction, ChatCompletionRequestMessage, ChatCompletionRequestMessageContent,
    ChatCompletionRequestMessageContentPart, ChatCompletionRequestMessageRole,
    ChatCompletionResponseMessage, ChatCompletionToolChoiceOption,
    CreateChatCompletionRequestClass, CreateResponseClass, File, FunctionObject,
    FunctionToolChoiceClass, FunctionToolChoiceType, InputItem, InputItemContent, InputItemRole,
    InputItemType, Instructions, PurpleType, ResponseFormatType, ToolElement, ToolType,
};
use crate::providers::openai::{
    try_parse_openai, try_parse_responses, universal_to_responses_input,
};
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{AssistantContent, Message};
use crate::universal::{
    FinishReason, UniversalParams, UniversalRequest, UniversalResponse, UniversalStreamChoice,
    UniversalStreamChunk, UniversalUsage, PLACEHOLDER_ID, PLACEHOLDER_MODEL,
};
use crate::util::media::parse_base64_data_url;

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

    fn request_to_universal(&self, payload: Value) -> Result<UniversalRequest, TransformError> {
        let extras = collect_extras(&payload, OPENAI_KNOWN_KEYS);
        let request: CreateChatCompletionRequestClass = serde_json::from_value(payload)
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
            tool_choice: request
                .tool_choice
                .and_then(|t| serde_json::to_value(t).ok()),
            response_format: request
                .response_format
                .and_then(|r| serde_json::to_value(r).ok()),
            seed: request.seed,
            presence_penalty: request.presence_penalty,
            frequency_penalty: request.frequency_penalty,
            stream: request.stream,
        };

        Ok(UniversalRequest {
            model: Some(request.model),
            messages,
            params,
            extras,
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
        insert_opt_i64(&mut obj, "max_completion_tokens", req.params.max_tokens);
        insert_opt_value(&mut obj, "stop", req.params.stop.clone());
        insert_opt_value(&mut obj, "tools", req.params.tools.clone());
        insert_opt_value(&mut obj, "tool_choice", req.params.tool_choice.clone());
        insert_opt_value(
            &mut obj,
            "response_format",
            req.params.response_format.clone(),
        );
        insert_opt_i64(&mut obj, "seed", req.params.seed);
        insert_opt_f64(&mut obj, "presence_penalty", req.params.presence_penalty);
        insert_opt_f64(&mut obj, "frequency_penalty", req.params.frequency_penalty);
        insert_opt_bool(&mut obj, "stream", req.params.stream);

        // If streaming, ensure stream_options.include_usage is set for usage reporting
        if req.params.stream == Some(true) {
            let stream_options = obj
                .entry("stream_options")
                .or_insert_with(|| serde_json::json!({}));
            if let Value::Object(opts) = stream_options {
                opts.insert("include_usage".into(), Value::Bool(true));
            }
        }

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

    fn response_to_universal(&self, payload: Value) -> Result<UniversalResponse, TransformError> {
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
                let universal = <Message as TryFromLLM<&ChatCompletionResponseMessage>>::try_from(
                    &response_msg,
                )
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                messages.push(universal);
            }

            // Get finish_reason from first choice
            if finish_reason.is_none() {
                if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
                    finish_reason = Some(reason.parse().unwrap());
                }
            }
        }

        let usage = payload.get("usage").map(|u| UniversalUsage {
            prompt_tokens: u.get("prompt_tokens").and_then(Value::as_i64),
            completion_tokens: u.get("completion_tokens").and_then(Value::as_i64),
            prompt_cached_tokens: u
                .get("prompt_tokens_details")
                .and_then(|d| d.get("cached_tokens"))
                .and_then(Value::as_i64),
            prompt_cache_creation_tokens: None, // OpenAI doesn't report cache creation tokens
            completion_reasoning_tokens: u
                .get("completion_tokens_details")
                .and_then(|d| d.get("reasoning_tokens"))
                .and_then(Value::as_i64),
        });

        Ok(UniversalResponse {
            model: payload
                .get("model")
                .and_then(Value::as_str)
                .map(String::from),
            messages,
            usage,
            finish_reason,
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
            let input = u.prompt_tokens.unwrap_or(0);
            let output = u.completion_tokens.unwrap_or(0);
            serde_json::json!({
                "prompt_tokens": input,
                "completion_tokens": output,
                "total_tokens": input + output
            })
        });

        let mut obj = serde_json::json!({
            "id": format!("chatcmpl-{}", PLACEHOLDER_ID),
            "object": "chat.completion",
            "created": 0,
            "model": resp.model.as_deref().unwrap_or(PLACEHOLDER_MODEL),
            "choices": choices
        });

        if let Some(usage_val) = usage {
            obj.as_object_mut()
                .unwrap()
                .insert("usage".into(), usage_val);
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
        payload: Value,
    ) -> Result<Option<UniversalStreamChunk>, TransformError> {
        // OpenAI is the canonical format, so this is mostly direct mapping
        let choices = payload
            .get("choices")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .map(|c| {
                        let index = c.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;
                        let delta = c.get("delta").cloned();
                        let finish_reason = c
                            .get("finish_reason")
                            .and_then(Value::as_str)
                            .map(String::from);
                        UniversalStreamChoice {
                            index,
                            delta,
                            finish_reason,
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Extract usage if present (usually only on final chunk)
        let usage = payload.get("usage").map(|u| UniversalUsage {
            prompt_tokens: u.get("prompt_tokens").and_then(Value::as_i64),
            completion_tokens: u.get("completion_tokens").and_then(Value::as_i64),
            prompt_cached_tokens: u
                .get("prompt_tokens_details")
                .and_then(|d| d.get("cached_tokens"))
                .and_then(Value::as_i64),
            prompt_cache_creation_tokens: None,
            completion_reasoning_tokens: u
                .get("completion_tokens_details")
                .and_then(|d| d.get("reasoning_tokens"))
                .and_then(Value::as_i64),
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

    fn stream_from_universal(&self, chunk: &UniversalStreamChunk) -> Result<Value, TransformError> {
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
            let prompt = usage.prompt_tokens.unwrap_or(0);
            let completion = usage.completion_tokens.unwrap_or(0);
            obj_map.insert(
                "usage".into(),
                serde_json::json!({
                    "prompt_tokens": prompt,
                    "completion_tokens": completion,
                    "total_tokens": prompt + completion
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

    fn request_to_universal(&self, payload: Value) -> Result<UniversalRequest, TransformError> {
        let extras = collect_extras(&payload, RESPONSES_KNOWN_KEYS);
        let request: CreateResponseClass = serde_json::from_value(payload)
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
            tool_choice: request
                .tool_choice
                .and_then(|t| serde_json::to_value(t).ok()),
            response_format: None,  // Different structure in Responses API
            seed: None,             // Responses API uses different randomness control
            presence_penalty: None, // Responses API doesn't support penalties
            frequency_penalty: None,
            stream: request.stream,
        };

        Ok(UniversalRequest {
            model: request.model, // Already Option<String> in CreateResponseClass
            messages,
            params,
            extras,
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

        // Note: temperature is intentionally NOT included for Responses API
        // as reasoning models (o1, o3) don't support it
        insert_opt_f64(&mut obj, "top_p", req.params.top_p);
        insert_opt_i64(&mut obj, "max_output_tokens", req.params.max_tokens);
        insert_opt_f64(&mut obj, "presence_penalty", req.params.presence_penalty);
        insert_opt_f64(&mut obj, "frequency_penalty", req.params.frequency_penalty);
        insert_opt_bool(&mut obj, "stream", req.params.stream);

        // Transform tools from OpenAI Chat format to Responses API format
        // {type: "function", function: {name, description, parameters}}
        // → {type: "function", name, description, parameters, strict: false}
        // Tools can come from params.tools or extras.tools depending on how the request was built
        let tools_value = req
            .params
            .tools
            .as_ref()
            .or_else(|| req.extras.get("tools"));
        if let Some(Value::Array(tools)) = tools_value {
            let response_tools: Vec<Value> = tools
                .iter()
                .filter_map(|tool| {
                    if tool.get("type").and_then(Value::as_str) == Some("function") {
                        let func = tool.get("function")?;
                        Some(serde_json::json!({
                            "type": "function",
                            "name": func.get("name")?,
                            "description": func.get("description"),
                            "parameters": func.get("parameters").cloned().unwrap_or(serde_json::json!({})),
                            "strict": false
                        }))
                    } else {
                        None
                    }
                })
                .collect();
            if !response_tools.is_empty() {
                obj.insert("tools".into(), Value::Array(response_tools));
            }
        }

        // Transform tool_choice from OpenAI Chat format to Responses API format
        // {function: {name: "foo"}} → {type: "function", name: "foo"}
        // tool_choice can come from params or extras depending on how the request was built
        let tool_choice_value = req
            .params
            .tool_choice
            .as_ref()
            .or_else(|| req.extras.get("tool_choice"));
        if let Some(tool_choice) = tool_choice_value {
            let converted = match tool_choice {
                Value::String(s) if s == "none" || s == "auto" || s == "required" => {
                    Value::String(s.clone())
                }
                Value::Object(obj_tc) if obj_tc.contains_key("function") => {
                    if let Some(func) = obj_tc.get("function") {
                        if let Some(name) = func.get("name").and_then(Value::as_str) {
                            serde_json::json!({ "type": "function", "name": name })
                        } else {
                            Value::String("auto".into())
                        }
                    } else {
                        Value::String("auto".into())
                    }
                }
                _ => Value::String("auto".into()),
            };
            obj.insert("tool_choice".into(), converted);
        }

        // Transform response_format to nested text.format structure for Responses API
        if let Some(response_format) = req.extras.get("response_format") {
            let text_format = match response_format.get("type").and_then(Value::as_str) {
                Some("text") | Some("json_object") => {
                    Some(serde_json::json!({ "format": response_format }))
                }
                Some("json_schema") => response_format.get("json_schema").map(|json_schema| {
                    serde_json::json!({
                        "format": {
                            "type": "json_schema",
                            "schema": json_schema.get("schema").cloned().unwrap_or(serde_json::json!({})),
                            "name": json_schema.get("name"),
                            "description": json_schema.get("description"),
                            "strict": json_schema.get("strict")
                        }
                    })
                }),
                _ => None,
            };
            if let Some(tf) = text_format {
                obj.insert("text".into(), tf);
            }
        }

        // Transform reasoning_effort to nested reasoning.effort structure
        if let Some(effort) = req.extras.get("reasoning_effort") {
            obj.insert(
                "reasoning".into(),
                serde_json::json!({ "effort": effort.clone() }),
            );
        }

        // Pass through parallel_tool_calls
        if let Some(Value::Bool(parallel)) = req.extras.get("parallel_tool_calls") {
            obj.insert("parallel_tool_calls".into(), Value::Bool(*parallel));
        }

        // Merge remaining extras (except those we handled specially)
        for (k, v) in &req.extras {
            if !matches!(
                k.as_str(),
                "tools"
                    | "tool_choice"
                    | "response_format"
                    | "reasoning_effort"
                    | "parallel_tool_calls"
            ) {
                obj.insert(k.clone(), v.clone());
            }
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

    fn response_to_universal(&self, payload: Value) -> Result<UniversalResponse, TransformError> {
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
                                    if c.get("type").and_then(Value::as_str) == Some("output_text")
                                    {
                                        c.get("text").and_then(Value::as_str).map(String::from)
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("");
                            if !text.is_empty() {
                                messages.push(Message::Assistant {
                                    content: AssistantContent::from(text),
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
                    content: AssistantContent::from(parts),
                    id: None,
                });
            }
        }

        // If still no messages, try output_text field as fallback
        if messages.is_empty() {
            if let Some(text) = payload.get("output_text").and_then(Value::as_str) {
                if !text.is_empty() {
                    messages.push(Message::Assistant {
                        content: AssistantContent::from(text.to_string()),
                        id: None,
                    });
                }
            }
        }

        // Map status to finish_reason
        let finish_reason = payload
            .get("status")
            .and_then(Value::as_str)
            .map(|s| s.parse().unwrap());

        let usage = payload.get("usage").map(|u| UniversalUsage {
            prompt_tokens: u.get("input_tokens").and_then(Value::as_i64),
            completion_tokens: u.get("output_tokens").and_then(Value::as_i64),
            prompt_cached_tokens: u
                .get("input_tokens_details")
                .and_then(|d| d.get("cached_tokens"))
                .and_then(Value::as_i64),
            prompt_cache_creation_tokens: None,
            completion_reasoning_tokens: u
                .get("output_tokens_details")
                .and_then(|d| d.get("reasoning_tokens"))
                .and_then(Value::as_i64),
        });

        Ok(UniversalResponse {
            model: payload
                .get("model")
                .and_then(Value::as_str)
                .map(String::from),
            messages,
            usage,
            finish_reason,
        })
    }

    fn response_from_universal(&self, resp: &UniversalResponse) -> Result<Value, TransformError> {
        // Build Responses API response format
        let output: Vec<Value> = resp
            .messages
            .iter()
            .map(|msg| {
                let text = match msg {
                    Message::Assistant { content, .. } => {
                        content.as_text().map(|s| s.to_string()).unwrap_or_default()
                    }
                    Message::User { content } => {
                        content.as_text().map(|s| s.to_string()).unwrap_or_default()
                    }
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
            "id": format!("resp_{}", PLACEHOLDER_ID),
            "object": "response",
            "model": resp.model.as_deref().unwrap_or(PLACEHOLDER_MODEL),
            "output": output,
            "status": status,
            "created_at": 0.0,
            "tool_choice": "none",
            "tools": [],
            "parallel_tool_calls": false
        });

        if let Some(usage) = &resp.usage {
            let input = usage.prompt_tokens.unwrap_or(0);
            let output = usage.completion_tokens.unwrap_or(0);
            obj.as_object_mut().unwrap().insert(
                "usage".into(),
                serde_json::json!({
                    "input_tokens": input,
                    "output_tokens": output,
                    "total_tokens": input + output,
                    "input_tokens_details": {
                        "cached_tokens": usage.prompt_cached_tokens.unwrap_or(0)
                    },
                    "output_tokens_details": {
                        "reasoning_tokens": usage.completion_reasoning_tokens.unwrap_or(0)
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
        payload: Value,
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
                let usage = response
                    .and_then(|r| r.get("usage"))
                    .map(|u| UniversalUsage {
                        prompt_tokens: u.get("input_tokens").and_then(Value::as_i64),
                        completion_tokens: u.get("output_tokens").and_then(Value::as_i64),
                        prompt_cached_tokens: u
                            .get("input_tokens_details")
                            .and_then(|d| d.get("cached_tokens"))
                            .and_then(Value::as_i64),
                        prompt_cache_creation_tokens: None,
                        completion_reasoning_tokens: u
                            .get("output_tokens_details")
                            .and_then(|d| d.get("reasoning_tokens"))
                            .and_then(Value::as_i64),
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
                let usage = response
                    .and_then(|r| r.get("usage"))
                    .map(|u| UniversalUsage {
                        prompt_tokens: u.get("input_tokens").and_then(Value::as_i64),
                        completion_tokens: u.get("output_tokens").and_then(Value::as_i64),
                        prompt_cached_tokens: u
                            .get("input_tokens_details")
                            .and_then(|d| d.get("cached_tokens"))
                            .and_then(Value::as_i64),
                        prompt_cache_creation_tokens: None,
                        completion_reasoning_tokens: u
                            .get("output_tokens_details")
                            .and_then(|d| d.get("reasoning_tokens"))
                            .and_then(Value::as_i64),
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

    fn stream_from_universal(&self, chunk: &UniversalStreamChunk) -> Result<Value, TransformError> {
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

            let id = chunk
                .id
                .clone()
                .unwrap_or_else(|| format!("resp_{}", PLACEHOLDER_ID));
            let mut response = serde_json::json!({
                "id": id,
                "object": "response",
                "model": chunk.model.as_deref().unwrap_or(PLACEHOLDER_MODEL),
                "status": status,
                "output": []
            });

            if let Some(usage) = &chunk.usage {
                response.as_object_mut().unwrap().insert(
                    "usage".into(),
                    serde_json::json!({
                        "input_tokens": usage.prompt_tokens.unwrap_or(0),
                        "output_tokens": usage.completion_tokens.unwrap_or(0),
                        "total_tokens": usage.prompt_tokens.unwrap_or(0) + usage.completion_tokens.unwrap_or(0)
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

// =============================================================================
// OpenAI Target-Specific Transformations
// =============================================================================

/// Error type for transformation operations.
#[derive(Debug, thiserror::Error)]
pub enum OpenAITransformError {
    #[error("missing required field: {field}")]
    MissingField { field: &'static str },
    #[error("invalid value: {message}")]
    InvalidValue { message: String },
    #[error("unsupported feature: {feature}")]
    Unsupported { feature: String },
    #[error("serialization failed: {0}")]
    SerializationFailed(String),
}

/// Apply target-specific transformations to an OpenAI-format request payload.
///
/// This function applies transformations needed to make an OpenAI-format request
/// work with different target providers (Azure, Vertex, Mistral, etc.).
///
/// # Arguments
///
/// * `payload` - The OpenAI-format request payload (modified in place)
/// * `target_provider` - The target provider that will receive the request
/// * `provider_metadata` - Optional provider-specific metadata (e.g., api_version for Azure)
///
/// # Returns
///
/// The transformed payload, or an error if transformation fails.
pub fn apply_target_transforms(
    payload: &Value,
    target_provider: TargetProvider,
    provider_metadata: Option<&Map<String, Value>>,
) -> Result<Value, OpenAITransformError> {
    // Parse as OpenAI request
    let mut request: CreateChatCompletionRequestClass = serde_json::from_value(payload.clone())
        .map_err(|e| OpenAITransformError::SerializationFailed(e.to_string()))?;

    // Detect capabilities based on request and target
    let capabilities = OpenAICapabilities::detect(&request, target_provider);

    // Apply reasoning model transformations
    if capabilities.requires_reasoning_transforms() {
        apply_reasoning_transforms(&mut request, &capabilities);
    }

    // Apply provider-specific field sanitization
    apply_provider_sanitization(
        &mut request,
        &capabilities,
        target_provider,
        provider_metadata,
    );

    // Apply model name normalization if needed
    if capabilities.requires_model_normalization {
        apply_model_normalization(&mut request, target_provider);
    }

    // Normalize user messages (handle non-image base64 content)
    normalize_user_messages(&mut request)?;

    // Apply response format transformations
    apply_response_format(&mut request, &capabilities)?;

    // Serialize back to Value
    serde_json::to_value(&request)
        .map_err(|e| OpenAITransformError::SerializationFailed(e.to_string()))
}

fn apply_reasoning_transforms(
    request: &mut CreateChatCompletionRequestClass,
    capabilities: &OpenAICapabilities,
) {
    // Remove unsupported fields for reasoning models
    request.temperature = None;
    request.parallel_tool_calls = None;

    // For legacy o1 models, convert system messages to user messages
    if capabilities.is_legacy_o1_model {
        for message in &mut request.messages {
            if matches!(message.role, ChatCompletionRequestMessageRole::System) {
                message.role = ChatCompletionRequestMessageRole::User;
            }
        }
    }
}

fn apply_provider_sanitization(
    request: &mut CreateChatCompletionRequestClass,
    capabilities: &OpenAICapabilities,
    target_provider: TargetProvider,
    provider_metadata: Option<&Map<String, Value>>,
) {
    // Remove stream_options for providers that don't support it
    if !capabilities.supports_stream_options {
        request.stream_options = None;
    }

    // Remove parallel_tool_calls for providers that don't support it
    if !capabilities.supports_parallel_tools {
        request.parallel_tool_calls = None;
    }

    // Remove seed field for Azure with API version
    let has_api_version = provider_metadata
        .and_then(|meta| meta.get("api_version"))
        .is_some();

    if capabilities.should_remove_seed_for_azure(target_provider, has_api_version) {
        request.seed = None;
    }
}

fn apply_model_normalization(
    request: &mut CreateChatCompletionRequestClass,
    target_provider: TargetProvider,
) {
    // Normalize Vertex model names
    if target_provider == TargetProvider::Vertex {
        if request.model.starts_with("publishers/meta/models/") {
            // Strip to "meta/..." format
            request.model = request
                .model
                .strip_prefix("publishers/")
                .and_then(|s| s.strip_prefix("meta/models/"))
                .map(|s| format!("meta/{}", s))
                .unwrap_or_else(|| request.model.clone());
        } else if let Some(stripped) = request.model.strip_prefix("publishers/") {
            // Strip "publishers/X/models/Y" to "Y"
            if let Some(model_part) = stripped.split("/models/").nth(1) {
                request.model = model_part.to_string();
            }
        }
    }
}

fn normalize_user_messages(
    request: &mut CreateChatCompletionRequestClass,
) -> Result<(), OpenAITransformError> {
    for message in &mut request.messages {
        if matches!(message.role, ChatCompletionRequestMessageRole::User) {
            if let Some(
                ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(
                    parts,
                ),
            ) = message.content.as_mut()
            {
                for part in parts.iter_mut() {
                    normalize_content_part(part)?;
                }
            }
        }
    }
    Ok(())
}

fn normalize_content_part(
    part: &mut ChatCompletionRequestMessageContentPart,
) -> Result<(), OpenAITransformError> {
    if !matches!(
        part.chat_completion_request_message_content_part_type,
        PurpleType::ImageUrl
    ) {
        return Ok(());
    }

    let Some(image_url_value) = part
        .image_url
        .as_ref()
        .map(|image_url| image_url.url.clone())
    else {
        return Ok(());
    };

    // Handle base64 data URLs - convert non-images to file type
    if let Some(data_url) = parse_base64_data_url(&image_url_value) {
        if !data_url.media_type.starts_with("image/") {
            part.chat_completion_request_message_content_part_type = PurpleType::File;
            part.image_url = None;
            part.file = Some(File {
                file_data: Some(image_url_value),
                file_id: None,
                filename: Some(if data_url.media_type == "application/pdf" {
                    "file_from_base64.pdf".to_string()
                } else {
                    "file_from_base64".to_string()
                }),
            });
        }
    }

    Ok(())
}

fn apply_response_format(
    request: &mut CreateChatCompletionRequestClass,
    capabilities: &OpenAICapabilities,
) -> Result<(), OpenAITransformError> {
    let Some(response_format) = request.response_format.take() else {
        return Ok(());
    };

    match response_format.text_type {
        ResponseFormatType::Text => Ok(()),
        ResponseFormatType::JsonSchema => {
            if capabilities.supports_native_structured_output {
                request.response_format = Some(response_format);
                return Ok(());
            }

            // Check if tools are already being used
            if request
                .tools
                .as_ref()
                .is_some_and(|tools| !tools.is_empty())
                || request.function_call.is_some()
                || request.tool_choice.is_some()
            {
                return Err(OpenAITransformError::Unsupported {
                    feature: "tools_with_structured_output".to_string(),
                });
            }

            // Convert json_schema to a tool call
            match response_format.json_schema {
                Some(schema) => {
                    request.tools = Some(vec![ToolElement {
                        function: Some(FunctionObject {
                            description: Some("Output the result in JSON format".to_string()),
                            name: "json".to_string(),
                            parameters: schema.schema.clone(),
                            strict: schema.strict,
                        }),
                        tool_type: ToolType::Function,
                        custom: None,
                    }]);

                    request.tool_choice =
                        Some(ChatCompletionToolChoiceOption::FunctionToolChoiceClass(
                            FunctionToolChoiceClass {
                                allowed_tools: None,
                                allowed_tools_type: FunctionToolChoiceType::Function,
                                function: Some(AllowedToolsFunction {
                                    name: "json".to_string(),
                                }),
                                custom: None,
                            },
                        ));

                    Ok(())
                }
                None => Err(OpenAITransformError::InvalidValue {
                    message: "json_schema response_format is missing schema".to_string(),
                }),
            }
        }
        ResponseFormatType::JsonObject => {
            request.response_format = Some(response_format);
            Ok(())
        }
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

        let universal = adapter.request_to_universal(payload).unwrap();
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

        let universal = adapter.request_to_universal(payload).unwrap();
        assert!(universal.extras.contains_key("user"));
        assert!(universal.extras.contains_key("custom_field"));

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert_eq!(reconstructed.get("user").unwrap(), "test-user-123");
        assert_eq!(
            reconstructed.get("custom_field").unwrap(),
            "should_be_preserved"
        );
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

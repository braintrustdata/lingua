/*!
OpenAI Responses API adapter.

This module provides the `ResponsesAdapter` for the Responses API,
which is used by reasoning models like o1 and o3.
*/

use crate::capabilities::ProviderFormat;
use std::collections::HashMap;

use crate::processing::adapters::{
    insert_opt_bool, insert_opt_f64, insert_opt_i64, ProviderAdapter,
};
use crate::processing::transform::TransformError;
use crate::providers::openai::generated::{
    InputItem, InputItemContent, InputItemRole, InputItemType, Instructions,
};
use crate::providers::openai::params::OpenAIResponsesParams;
use crate::providers::openai::{try_parse_responses, universal_to_responses_input};
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{AssistantContent, Message, UserContent};
use std::convert::TryInto;
use crate::universal::tools::is_responses_tool_format;
use crate::universal::{
    FinishReason, UniversalParams, UniversalRequest, UniversalResponse, UniversalStreamChoice,
    UniversalStreamChunk, UniversalUsage, PLACEHOLDER_ID, PLACEHOLDER_MODEL,
};

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
        // Parse into typed params - extras are automatically captured via #[serde(flatten)]
        let typed_params: OpenAIResponsesParams = serde_json::from_value(payload.clone())
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Parse again for strongly-typed message conversion
        let request: crate::providers::openai::generated::CreateResponseClass =
            serde_json::from_value(payload)
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

        // Extract response_format from nested text.format structure and convert to typed config
        let response_format = typed_params
            .text
            .as_ref()
            .and_then(|t| t.get("format"))
            .and_then(|v| (ProviderFormat::Responses, v).try_into().ok());

        let params = UniversalParams {
            temperature: typed_params.temperature,
            top_p: typed_params.top_p,
            top_k: None,
            max_tokens: typed_params.max_output_tokens,
            stop: None, // Responses API doesn't use stop
            tools: typed_params.tools,
            tool_choice: typed_params
                .tool_choice
                .as_ref()
                .and_then(|v| (ProviderFormat::Responses, v).try_into().ok()),
            response_format,
            seed: None, // Responses API uses different randomness control
            presence_penalty: None, // Responses API doesn't support penalties
            frequency_penalty: None,
            stream: typed_params.stream,
            // New canonical fields
            parallel_tool_calls: typed_params.parallel_tool_calls,
            reasoning: typed_params
                .reasoning
                .as_ref()
                .and_then(|v| (ProviderFormat::Responses, v).try_into().ok()),
            metadata: typed_params.metadata,
            store: typed_params.store,
            service_tier: typed_params.service_tier,
            logprobs: None,     // Responses API doesn't support logprobs
            top_logprobs: None, // Responses API doesn't support top_logprobs
        };

        // Collect provider-specific extras for round-trip preservation
        // This includes both unknown fields (from serde flatten) and known Responses API fields
        // that aren't part of UniversalParams
        let mut extras_map: Map<String, Value> = typed_params.extras.into_iter().collect();

        // Add Responses API specific known fields that aren't in UniversalParams
        if let Some(instructions) = typed_params.instructions {
            extras_map.insert("instructions".into(), Value::String(instructions));
        }
        if let Some(text) = typed_params.text {
            extras_map.insert("text".into(), text);
        }
        if let Some(truncation) = typed_params.truncation {
            extras_map.insert("truncation".into(), truncation);
        }
        if let Some(user) = typed_params.user {
            extras_map.insert("user".into(), Value::String(user));
        }
        if let Some(safety_identifier) = typed_params.safety_identifier {
            extras_map.insert("safety_identifier".into(), Value::String(safety_identifier));
        }
        if let Some(prompt_cache_key) = typed_params.prompt_cache_key {
            extras_map.insert("prompt_cache_key".into(), Value::String(prompt_cache_key));
        }

        let mut provider_extras = HashMap::new();
        if !extras_map.is_empty() {
            provider_extras.insert(ProviderFormat::Responses, extras_map);
        }

        Ok(UniversalRequest {
            model: typed_params.model,
            messages,
            params,
            provider_extras,
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

        // Get provider-specific extras for Responses API
        let responses_extras = req.provider_extras.get(&ProviderFormat::Responses);

        // Transform tools - but if already in Responses format, pass through unchanged
        if let Some(tools) = req.params.tools.as_ref() {
            if is_responses_tool_format(tools) {
                // Already in Responses format - pass through
                obj.insert("tools".into(), tools.clone());
            } else if let Value::Array(tools_arr) = tools {
                // Convert from OpenAI Chat format to Responses API format
                // {type: "function", function: {name, description, parameters}}
                // → {type: "function", name, description, parameters, strict: false}
                let response_tools: Vec<Value> = tools_arr
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
        }

        // Convert tool_choice from canonical ToolChoiceConfig to Responses API format
        // Responses API doesn't use parallel_tool_calls in tool_choice, pass None
        if let Some(tool_choice_val) = req
            .params
            .tool_choice
            .as_ref()
            .and_then(|tc| tc.to_provider(ProviderFormat::Responses, None).ok())
            .flatten()
        {
            obj.insert("tool_choice".into(), tool_choice_val);
        }

        // Convert response_format from canonical ResponseFormatConfig to Responses API text format
        if let Some(text_val) = req
            .params
            .response_format
            .as_ref()
            .and_then(|rf| rf.to_provider(ProviderFormat::Responses).ok())
            .flatten()
        {
            obj.insert("text".into(), text_val);
        }

        // Add reasoning from canonical params - convert ReasoningConfig to Responses API format
        // max_tokens is passed explicitly for budget→effort conversion
        if let Some(reasoning_val) = req
            .params
            .reasoning
            .as_ref()
            .and_then(|r| r.to_provider(ProviderFormat::Responses, req.params.max_tokens).ok())
            .flatten()
        {
            obj.insert("reasoning".into(), reasoning_val);
        }

        // Add parallel_tool_calls from canonical params
        if let Some(parallel) = req.params.parallel_tool_calls {
            obj.insert("parallel_tool_calls".into(), Value::Bool(parallel));
        }

        // Add metadata from canonical params
        if let Some(metadata) = req.params.metadata.as_ref() {
            obj.insert("metadata".into(), metadata.clone());
        }

        // Add store from canonical params
        if let Some(store) = req.params.store {
            obj.insert("store".into(), Value::Bool(store));
        }

        // Add service_tier from canonical params
        if let Some(ref service_tier) = req.params.service_tier {
            obj.insert("service_tier".into(), Value::String(service_tier.clone()));
        }

        // Merge back provider-specific extras (only for Responses API)
        if let Some(extras) = responses_extras {
            for (k, v) in extras {
                // Don't overwrite canonical fields we already handled
                if !obj.contains_key(k) {
                    obj.insert(k.clone(), v.clone());
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

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

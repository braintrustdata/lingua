/*!
OpenAI Responses API adapter.

This module provides the `ResponsesAdapter` for the Responses API,
which is used by reasoning models like o1 and o3.
*/

use crate::capabilities::ProviderFormat;

use crate::error::ConvertError;
use crate::processing::adapters::{
    insert_opt_bool, insert_opt_f64, insert_opt_i64, ProviderAdapter,
};
use crate::processing::transform::TransformError;
use crate::providers::openai::capabilities::apply_model_transforms;
use crate::providers::openai::generated::{
    InputItem, InputItemContent, InputItemRole, InputItemType, Instructions, OutputItemType,
};
use crate::providers::openai::params::OpenAIResponsesParams;
use crate::providers::openai::{try_parse_responses, universal_to_responses_input};
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{
    AssistantContent, Message, TextContentPart, UserContent, UserContentPart,
};
use crate::universal::tools::{tools_to_responses_value, UniversalTool};
use crate::universal::{
    FinishReason, UniversalParams, UniversalRequest, UniversalResponse, UniversalStreamChoice,
    UniversalStreamChunk, UniversalUsage, PLACEHOLDER_ID, PLACEHOLDER_MODEL,
};
use std::convert::TryInto;

fn system_text(message: &Message) -> Option<&str> {
    match message {
        Message::System { content } | Message::Developer { content } => match content {
            UserContent::String(text) => Some(text.as_str()),
            UserContent::Array(parts) => {
                if parts.len() != 1 {
                    return None;
                }
                match &parts[0] {
                    UserContentPart::Text(TextContentPart { text, .. }) => Some(text.as_str()),
                    _ => None,
                }
            }
        },
        _ => None,
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
        // Single parse: typed params now includes typed input via #[serde(flatten)]
        let typed_params: OpenAIResponsesParams = serde_json::from_value(payload)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Extract input items from typed_params.input (partial move - other fields remain accessible)
        let input_items: Vec<InputItem> = match typed_params.input {
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
            None => {
                return Err(TransformError::ToUniversalFailed(
                    "OpenAI Responses: missing 'input' field".to_string(),
                ))
            }
        };

        let mut messages = <Vec<Message> as TryFromLLM<Vec<InputItem>>>::try_from(input_items)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        if let Some(instructions) = typed_params.instructions.as_ref().filter(|s| !s.is_empty()) {
            messages.insert(
                0,
                Message::System {
                    content: UserContent::String(instructions.clone()),
                },
            );
        }

        // Extract response_format from nested text.format structure and convert to typed config
        let response_format = typed_params
            .text
            .as_ref()
            .and_then(|t| t.get("format"))
            .and_then(|v| (ProviderFormat::Responses, v).try_into().ok());

        // Extract max_tokens first - needed for reasoning budget computation
        let max_tokens = typed_params.max_output_tokens;

        // Convert reasoning to ReasoningConfig, computing budget_tokens with max_tokens context
        let reasoning = typed_params
            .reasoning
            .as_ref()
            .map(|r| (r, max_tokens).into());

        let mut params = UniversalParams {
            temperature: typed_params.temperature,
            top_p: typed_params.top_p,
            top_k: None,
            max_tokens,
            stop: None, // Responses API doesn't use stop
            tools: typed_params
                .tools
                .as_ref()
                .map(UniversalTool::from_value_array),
            tool_choice: typed_params
                .tool_choice
                .as_ref()
                .and_then(|v| (ProviderFormat::Responses, v).try_into().ok()),
            response_format,
            seed: None,             // Responses API uses different randomness control
            presence_penalty: None, // Responses API doesn't support penalties
            frequency_penalty: None,
            stream: typed_params.stream,
            // New canonical fields
            parallel_tool_calls: typed_params.parallel_tool_calls,
            reasoning,
            metadata: typed_params.metadata,
            store: typed_params.store,
            service_tier: typed_params.service_tier,
            logprobs: None, // Responses API doesn't support logprobs boolean
            top_logprobs: typed_params.top_logprobs,
            extras: Default::default(),
        };

        // Sync parallel_tool_calls with tool_choice.disable_parallel for roundtrip fidelity
        // OpenAI uses parallel_tool_calls at params level, Anthropic uses tool_choice.disable_parallel
        if params.parallel_tool_calls == Some(false) {
            if let Some(ref mut tc) = params.tool_choice {
                if tc.disable_parallel.is_none() {
                    tc.disable_parallel = Some(true);
                }
            }
        }

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

        if !extras_map.is_empty() {
            params.extras.insert(ProviderFormat::Responses, extras_map);
        }

        Ok(UniversalRequest {
            model: typed_params.model,
            messages,
            params,
        })
    }

    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
        let model = req.model.as_ref().ok_or(TransformError::ValidationFailed {
            target: ProviderFormat::Responses,
            reason: "missing model".to_string(),
        })?;

        let responses_extras = req.params.extras.get(&ProviderFormat::Responses);
        let mut messages_for_input = req.messages.clone();
        if let Some(extras) = responses_extras {
            if let Some(instructions) = extras.get("instructions").and_then(Value::as_str) {
                if let Some(first_text) = messages_for_input.first().and_then(system_text) {
                    if first_text == instructions {
                        messages_for_input.remove(0);
                    }
                }
            }
        }

        // Use existing conversion with 1:N Tool message expansion
        let input_items = universal_to_responses_input(&messages_for_input)
            .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

        let mut obj = Map::new();
        obj.insert("model".into(), Value::String(model.clone()));
        obj.insert(
            "input".into(),
            serde_json::to_value(input_items)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
        );

        insert_opt_f64(&mut obj, "temperature", req.params.temperature);
        insert_opt_f64(&mut obj, "top_p", req.params.top_p);
        insert_opt_i64(&mut obj, "max_output_tokens", req.params.max_tokens);
        insert_opt_i64(&mut obj, "top_logprobs", req.params.top_logprobs);
        // Note: presence_penalty, frequency_penalty, seed, logprobs (bool) are NOT supported by Responses API
        insert_opt_bool(&mut obj, "stream", req.params.stream);

        // Get provider-specific extras for Responses API
        let responses_extras = req.params.extras.get(&ProviderFormat::Responses);

        // Convert tools to Responses API format
        if let Some(tools) = req.params.tools.as_ref() {
            if let Some(tools_value) = tools_to_responses_value(tools)? {
                obj.insert("tools".into(), tools_value);
            }
        }

        // Convert tool_choice using helper method
        if let Some(tool_choice_val) = req.params.tool_choice_for(ProviderFormat::Responses) {
            obj.insert("tool_choice".into(), tool_choice_val);
        }

        // Convert response_format to Responses API text format using helper method
        if let Some(text_val) = req.params.response_format_for(ProviderFormat::Responses) {
            obj.insert("text".into(), text_val);
        }

        // Add reasoning from canonical params
        if let Some(reasoning_val) = req.params.reasoning_for(ProviderFormat::Responses) {
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

        // Apply capability-based transforms (e.g., strip temperature for reasoning models)
        apply_model_transforms(model, &mut obj);

        Ok(Value::Object(obj))
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
        use crate::providers::openai::generated::OutputItem;

        let output_items: Vec<OutputItem> = payload
            .get("output")
            .and_then(Value::as_array)
            .map(|arr| serde_json::from_value(Value::Array(arr.clone())))
            .transpose()
            .map_err(|e| {
                TransformError::ToUniversalFailed(format!("Failed to parse output items: {}", e))
            })?
            .ok_or_else(|| TransformError::ToUniversalFailed("missing output".to_string()))?;

        let messages: Vec<Message> = TryFromLLM::try_from(output_items)
            .map_err(|e: ConvertError| TransformError::ToUniversalFailed(e.to_string()))?;

        let has_tool_calls = messages.iter().any(|m| {
            if let Message::Assistant {
                content: AssistantContent::Array(parts),
                ..
            } = m
            {
                parts.iter().any(|p| {
                    matches!(
                        p,
                        crate::universal::message::AssistantContentPart::ToolCall { .. }
                    )
                })
            } else {
                false
            }
        });

        let finish_reason = if has_tool_calls {
            Some(FinishReason::ToolCalls)
        } else {
            match payload.get("status").and_then(Value::as_str) {
                Some(s) => Some(s.parse().map_err(|_| ConvertError::InvalidEnumValue {
                    type_name: "FinishReason",
                    value: s.to_string(),
                })?),
                None => None,
            }
        };

        let usage = UniversalUsage::extract_from_response(&payload, self.format());

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
        use crate::providers::openai::generated::OutputItem;

        let output_items: Vec<OutputItem> = TryFromLLM::try_from(resp.messages.clone())
            .map_err(|e: ConvertError| TransformError::FromUniversalFailed(e.to_string()))?;

        // Serialize OutputItems to JSON values
        let output: Vec<Value> = output_items
            .iter()
            .map(serde_json::to_value)
            .collect::<Result<_, _>>()
            .map_err(|e| {
                TransformError::SerializationFailed(format!(
                    "Failed to serialize output item: {}",
                    e
                ))
            })?;

        // Calculate output_text (concatenate text from all message-type items)
        let output_text = output_items
            .iter()
            .filter(|item| item.output_item_type == Some(OutputItemType::Message))
            .filter_map(|item| item.content.as_ref())
            .flat_map(|content| content.iter())
            .filter_map(|c| c.text.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("");

        let status = resp
            .finish_reason
            .as_ref()
            .map(|r| r.to_provider_string(self.format()).to_string())
            .unwrap_or_else(|| "completed".to_string());

        // Build response with all required fields for TheResponseObject
        let mut map = serde_json::Map::new();
        map.insert(
            "id".into(),
            Value::String(format!("resp_{}", PLACEHOLDER_ID)),
        );
        map.insert("object".into(), Value::String("response".into()));
        map.insert(
            "model".into(),
            Value::String(resp.model.as_deref().unwrap_or(PLACEHOLDER_MODEL).into()),
        );
        map.insert("output".into(), Value::Array(output));
        map.insert("output_text".into(), Value::String(output_text));
        map.insert("status".into(), Value::String(status));
        map.insert("created_at".into(), serde_json::json!(0.0));
        map.insert("tool_choice".into(), Value::String("none".into()));
        map.insert("tools".into(), Value::Array(vec![]));
        map.insert("parallel_tool_calls".into(), Value::Bool(false));

        if let Some(usage) = &resp.usage {
            map.insert("usage".into(), usage.to_provider_value(self.format()));
        }

        Ok(Value::Object(map))
    }

    // =========================================================================
    // Streaming response handling
    // =========================================================================

    fn detect_stream_response(&self, payload: &Value) -> bool {
        // Responses API streaming has two formats:
        // 1. type field starting with "response." at top level
        // 2. object="response.delta" at top level with delta.type nested
        payload
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|t| t.starts_with("response."))
            || payload
                .get("object")
                .and_then(Value::as_str)
                .is_some_and(|o| o == "response.delta")
    }

    fn stream_to_universal(
        &self,
        payload: Value,
    ) -> Result<Option<UniversalStreamChunk>, TransformError> {
        // Handle two streaming formats:
        // 1. Standard: type field at top level (e.g., "response.created")
        // 2. Alternate: object="response.delta" with delta.type nested (e.g., delta.type="response.start")
        let event_type = if let Some(t) = payload.get("type").and_then(Value::as_str) {
            t.to_string()
        } else if payload.get("object").and_then(Value::as_str) == Some("response.delta") {
            // Alternate format - get type from delta
            let delta_type = payload
                .get("delta")
                .and_then(|d| d.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            // Map alternate type names to standard ones
            match delta_type {
                "response.start" => "response.created".to_string(),
                "response.done" => "response.completed".to_string(),
                "content_part.delta" => "response.output_text.delta".to_string(),
                "content_part.start" | "content_part.done" | "output_item.start"
                | "output_item.done" => {
                    return Ok(Some(UniversalStreamChunk::keep_alive()));
                }
                other => format!("response.{}", other),
            }
        } else {
            return Err(TransformError::ToUniversalFailed(
                "missing type field".to_string(),
            ));
        };

        // For alternate format, extract data from delta instead of top level
        let is_alternate_format =
            payload.get("object").and_then(Value::as_str) == Some("response.delta");
        let delta_obj = payload.get("delta");

        match event_type.as_str() {
            "response.output_text.delta" => {
                // Text delta - extract from delta field
                // Standard format: payload.delta is the text string
                // Alternate format: payload.delta.text is the text string
                let text = if is_alternate_format {
                    delta_obj
                        .and_then(|d| d.get("text"))
                        .and_then(Value::as_str)
                } else {
                    payload.get("delta").and_then(Value::as_str)
                };

                // Use null for empty/missing text, preserving semantic equivalence with source
                let content_value = match text {
                    Some(t) if !t.is_empty() => Value::String(t.to_string()),
                    _ => Value::Null, // Empty or missing text becomes null
                };

                let output_index = payload
                    .get("output_index")
                    .or_else(|| delta_obj.and_then(|d| d.get("index")))
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32;

                Ok(Some(UniversalStreamChunk::new(
                    None,
                    None,
                    vec![UniversalStreamChoice {
                        index: output_index,
                        delta: Some(serde_json::json!({
                            "role": "assistant",
                            "content": content_value
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
                    .filter(|u| !u.is_null())
                    .map(|u| UniversalUsage::from_provider_value(u, self.format()));

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
                    .filter(|u| !u.is_null())
                    .map(|u| UniversalUsage::from_provider_value(u, self.format()));

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
                // Initial metadata events - extract model/id/usage
                // Standard format: payload.response contains the data
                // Alternate format: payload.delta.response contains the data
                let response = if is_alternate_format {
                    delta_obj.and_then(|d| d.get("response"))
                } else {
                    payload.get("response")
                };

                // If no response data, this is a keep-alive roundtrip
                if response.is_none() {
                    return Ok(Some(UniversalStreamChunk::keep_alive()));
                }

                let model = response
                    .and_then(|r| r.get("model"))
                    .and_then(Value::as_str)
                    .map(String::from);
                let id = response
                    .and_then(|r| r.get("id"))
                    .and_then(Value::as_str)
                    .map(String::from);
                let usage = response
                    .and_then(|r| r.get("usage"))
                    .filter(|u| !u.is_null())
                    .map(|u| UniversalUsage::from_provider_value(u, self.format()));

                Ok(Some(UniversalStreamChunk::new(
                    id,
                    model,
                    vec![UniversalStreamChoice {
                        index: 0,
                        delta: Some(serde_json::json!({"role": "assistant", "content": ""})),
                        finish_reason: None,
                    }],
                    None,
                    usage,
                )))
            }

            "response.output_item.added" => {
                // Tool call start - extract call_id, name, and output_index
                let item = payload.get("item");
                let item_type = item.and_then(|i| i.get("type")).and_then(Value::as_str);

                if item_type == Some("function_call") {
                    let call_id = item
                        .and_then(|i| i.get("call_id"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let name = item
                        .and_then(|i| i.get("name"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let output_index = payload
                        .get("output_index")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as u32;

                    return Ok(Some(UniversalStreamChunk::new(
                        None,
                        None,
                        vec![UniversalStreamChoice {
                            index: 0,
                            delta: Some(serde_json::json!({
                                "role": "assistant",
                                "content": Value::Null,
                                "tool_calls": [{
                                    "index": output_index,
                                    "id": call_id,
                                    "type": "function",
                                    "function": {
                                        "name": name,
                                        "arguments": ""
                                    }
                                }]
                            })),
                            finish_reason: None,
                        }],
                        None,
                        None,
                    )));
                }

                Ok(Some(UniversalStreamChunk::keep_alive()))
            }

            "response.function_call_arguments.delta" => {
                let arguments = payload.get("delta").and_then(Value::as_str).unwrap_or("");
                let output_index = payload
                    .get("output_index")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32;

                Ok(Some(UniversalStreamChunk::new(
                    None,
                    None,
                    vec![UniversalStreamChoice {
                        index: 0,
                        delta: Some(serde_json::json!({
                            "tool_calls": [{
                                "index": output_index,
                                "function": {
                                    "arguments": arguments
                                }
                            }]
                        })),
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

        // Check if delta has tool_calls
        let has_tool_calls = chunk
            .choices
            .first()
            .and_then(|c| c.delta.as_ref())
            .and_then(|d| d.get("tool_calls"))
            .and_then(Value::as_array)
            .is_some_and(|arr| !arr.is_empty());

        // Check if this is an initial metadata chunk (has model/id/usage but no content)
        // Exclude chunks with tool_calls - those must be handled by the tool call path
        let is_initial_metadata =
            (chunk.model.is_some() || chunk.id.is_some() || chunk.usage.is_some())
                && !has_finish
                && !has_tool_calls
                && chunk
                    .choices
                    .first()
                    .and_then(|c| c.delta.as_ref())
                    .is_none_or(|d| {
                        d.get("content")
                            .and_then(Value::as_str)
                            .is_none_or(|s| s.is_empty())
                    });

        if is_initial_metadata {
            // Return response.created with model/id/usage
            let id = chunk
                .id
                .clone()
                .unwrap_or_else(|| format!("resp_{}", PLACEHOLDER_ID));
            let mut response = serde_json::json!({
                "id": id,
                "object": "response",
                "model": chunk.model.as_deref().unwrap_or(PLACEHOLDER_MODEL),
                "status": "in_progress",
                "output": []
            });

            if let Some(usage) = &chunk.usage {
                if let Some(obj) = response.as_object_mut() {
                    obj.insert("usage".into(), usage.to_provider_value(self.format()));
                }
            }

            return Ok(serde_json::json!({
                "type": "response.created",
                "response": response
            }));
        }

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
                if let Some(obj) = response.as_object_mut() {
                    obj.insert("usage".into(), usage.to_provider_value(self.format()));
                }
            }

            return Ok(serde_json::json!({
                "type": if status == "completed" { "response.completed" } else { "response.incomplete" },
                "response": response
            }));
        }

        // Check for content delta
        if let Some(choice) = chunk.choices.first() {
            if let Some(delta) = &choice.delta {
                // Check for tool_calls in the delta
                if let Some(tool_calls) = delta.get("tool_calls").and_then(Value::as_array) {
                    if let Some(tc) = tool_calls.first() {
                        let output_index =
                            tc.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;

                        // Initial tool call chunk has an id field
                        if let Some(call_id) = tc.get("id").and_then(Value::as_str) {
                            let name = tc
                                .get("function")
                                .and_then(|f| f.get("name"))
                                .and_then(Value::as_str)
                                .unwrap_or("");

                            return Ok(serde_json::json!({
                                "type": "response.output_item.added",
                                "output_index": output_index,
                                "item": {
                                    "type": "function_call",
                                    "status": "in_progress",
                                    "call_id": call_id,
                                    "name": name,
                                    "arguments": ""
                                }
                            }));
                        }

                        // Subsequent chunks have only function.arguments
                        if let Some(arguments) = tc
                            .get("function")
                            .and_then(|f| f.get("arguments"))
                            .and_then(Value::as_str)
                        {
                            return Ok(serde_json::json!({
                                "type": "response.function_call_arguments.delta",
                                "output_index": output_index,
                                "delta": arguments
                            }));
                        }
                    }
                }

                if let Some(content) = delta.get("content").and_then(Value::as_str) {
                    return Ok(serde_json::json!({
                        "type": "response.output_text.delta",
                        "output_index": choice.index,
                        "content_index": 0,
                        "delta": content
                    }));
                }

                // If content is null or missing and no tool_calls, return empty text delta
                let content_is_missing_or_null =
                    delta.get("content").is_none() || delta.get("content") == Some(&Value::Null);

                if content_is_missing_or_null && !has_tool_calls {
                    return Ok(serde_json::json!({
                        "type": "response.output_text.delta",
                        "output_index": choice.index,
                        "content_index": 0,
                        "delta": ""
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

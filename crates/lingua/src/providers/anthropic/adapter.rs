/*!
Anthropic provider adapter for Messages API.

Anthropic's Messages API has some unique requirements:
- `max_tokens` is required (we use a default of 4096)
- System messages use a separate `system` parameter, not in `messages` array
*/

use crate::capabilities::ProviderFormat;
use crate::error::ConvertError;
use crate::processing::adapters::{
    insert_opt_bool, insert_opt_f64, insert_opt_i64, ProviderAdapter,
};
use crate::processing::transform::TransformError;
use crate::providers::anthropic::generated::{ContentBlock, InputMessage};
use crate::providers::anthropic::params::AnthropicParams;
use crate::providers::anthropic::try_parse_anthropic;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{Message, UserContent};
use crate::universal::tools::{tools_to_anthropic_value, UniversalTool};
use crate::universal::transform::extract_system_messages;
use crate::universal::{
    parse_stop_sequences, FinishReason, UniversalParams, UniversalRequest, UniversalResponse,
    UniversalStreamChoice, UniversalStreamChunk, UniversalUsage, PLACEHOLDER_ID, PLACEHOLDER_MODEL,
};
use std::convert::TryInto;

/// Default max_tokens for Anthropic requests (matches legacy proxy behavior).
pub const DEFAULT_MAX_TOKENS: i64 = 4096;

/// Adapter for Anthropic Messages API.
pub struct AnthropicAdapter;

impl ProviderAdapter for AnthropicAdapter {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::Anthropic
    }

    fn directory_name(&self) -> &'static str {
        "anthropic"
    }

    fn display_name(&self) -> &'static str {
        "Anthropic"
    }

    fn detect_request(&self, payload: &Value) -> bool {
        try_parse_anthropic(payload).is_ok()
    }

    fn request_to_universal(&self, payload: Value) -> Result<UniversalRequest, TransformError> {
        // Single parse: typed params now includes typed messages via #[serde(flatten)]
        let typed_params: AnthropicParams = serde_json::from_value(payload)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Extract typed messages (partial move - other fields remain accessible)
        let input_messages = typed_params.messages.ok_or_else(|| {
            TransformError::ToUniversalFailed("Anthropic: missing 'messages' field".to_string())
        })?;

        let messages = <Vec<Message> as TryFromLLM<Vec<_>>>::try_from(input_messages)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let mut params = UniversalParams {
            temperature: typed_params.temperature,
            top_p: typed_params.top_p,
            top_k: typed_params.top_k,
            max_tokens: typed_params.max_tokens,
            stop: typed_params
                .stop_sequences
                .as_ref()
                .and_then(parse_stop_sequences),
            tools: typed_params
                .tools
                .as_ref()
                .map(UniversalTool::from_value_array),
            tool_choice: typed_params
                .tool_choice
                .as_ref()
                .and_then(|v| (ProviderFormat::Anthropic, v).try_into().ok()),
            response_format: typed_params
                .output_format
                .as_ref()
                .and_then(|v| (ProviderFormat::Anthropic, v).try_into().ok()),
            seed: None,             // Anthropic doesn't support seed
            presence_penalty: None, // Anthropic doesn't support these
            frequency_penalty: None,
            stream: typed_params.stream,
            // Extract parallel_tool_calls from Anthropic's disable_parallel_tool_use in tool_choice
            parallel_tool_calls: typed_params
                .tool_choice
                .as_ref()
                .and_then(|tc| tc.get("disable_parallel_tool_use"))
                .and_then(Value::as_bool)
                .map(|disabled| !disabled), // disable_parallel_tool_use: true → parallel_tool_calls: false
            reasoning: typed_params
                .thinking
                .as_ref()
                .map(crate::universal::request::ReasoningConfig::from),
            metadata: typed_params.metadata,
            store: None, // Anthropic doesn't support store
            service_tier: typed_params.service_tier,
            logprobs: None,     // Anthropic doesn't support logprobs
            top_logprobs: None, // Anthropic doesn't support top_logprobs
            extras: Default::default(),
        };

        // Use extras captured automatically via #[serde(flatten)]
        if !typed_params.extras.is_empty() {
            params.extras.insert(
                ProviderFormat::Anthropic,
                typed_params.extras.into_iter().collect(),
            );
        }

        Ok(UniversalRequest {
            model: typed_params.model,
            messages,
            params,
        })
    }

    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
        let model = req.model.as_ref().ok_or(TransformError::ValidationFailed {
            target: ProviderFormat::Anthropic,
            reason: "missing model".to_string(),
        })?;

        // Clone messages and extract system messages (Anthropic uses separate `system` param)
        let mut msgs = req.messages.clone();
        let system_contents = extract_system_messages(&mut msgs);

        // Convert remaining messages
        let anthropic_messages: Vec<InputMessage> =
            <Vec<InputMessage> as TryFromLLM<Vec<Message>>>::try_from(msgs)
                .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

        let mut obj = Map::new();
        obj.insert("model".into(), Value::String(model.clone()));
        obj.insert(
            "messages".into(),
            serde_json::to_value(anthropic_messages)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
        );

        // Add system message if present
        if !system_contents.is_empty() {
            let system_text: String = system_contents
                .iter()
                .filter_map(|c| match c {
                    UserContent::String(s) => Some(s.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            obj.insert("system".into(), Value::String(system_text));
        }

        // max_tokens is required for Anthropic - use the value from params or default
        let max_tokens = req.params.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);
        obj.insert("max_tokens".into(), Value::Number(max_tokens.into()));

        // Check if reasoning/thinking is enabled
        // Note: thinking_val can be { type: "disabled" } or { type: "enabled", ... }
        let thinking_val = req.params.reasoning_for(ProviderFormat::Anthropic);
        let reasoning_enabled = thinking_val
            .as_ref()
            .and_then(|v| v.get("type"))
            .and_then(|t| t.as_str())
            .is_some_and(|t| t == "enabled");
        if !reasoning_enabled {
            insert_opt_f64(&mut obj, "temperature", req.params.temperature);
        }

        insert_opt_f64(&mut obj, "top_p", req.params.top_p);
        insert_opt_i64(&mut obj, "top_k", req.params.top_k);

        // Anthropic uses stop_sequences instead of stop
        if let Some(ref stop) = req.params.stop {
            if !stop.is_empty() {
                obj.insert(
                    "stop_sequences".into(),
                    Value::Array(stop.iter().map(|s| Value::String(s.clone())).collect()),
                );
            }
        }

        // Convert tools to Anthropic format
        if let Some(tools) = &req.params.tools {
            if let Some(tools_value) = tools_to_anthropic_value(tools)? {
                obj.insert("tools".into(), tools_value);
            }
        }

        // Convert tool_choice using helper method (handles parallel_tool_calls internally)
        if let Some(tool_choice_val) = req.params.tool_choice_for(ProviderFormat::Anthropic) {
            obj.insert("tool_choice".into(), tool_choice_val);
        }
        insert_opt_bool(&mut obj, "stream", req.params.stream);

        // Add reasoning as thinking if present (use pre-computed value from temperature override)
        if let Some(thinking) = thinking_val {
            obj.insert("thinking".into(), thinking);
        }

        // Add metadata from canonical params
        // Anthropic only accepts user_id in metadata, so filter out other fields
        if let Some(metadata) = req.params.metadata.as_ref() {
            if let Some(obj_map) = metadata.as_object() {
                if let Some(user_id) = obj_map.get("user_id") {
                    obj.insert("metadata".into(), serde_json::json!({ "user_id": user_id }));
                }
                // Skip metadata entirely if no user_id present
            }
        }

        // Add service_tier from canonical params
        // Map OpenAI's "default" to Anthropic's "auto" (Anthropic only accepts "auto" or "standard_only")
        if let Some(ref service_tier) = req.params.service_tier {
            let anthropic_tier = match service_tier.as_str() {
                "default" => "auto",
                other => other,
            };
            obj.insert(
                "service_tier".into(),
                Value::String(anthropic_tier.to_string()),
            );
        }

        // Add output_format for structured outputs (beta feature)
        if let Some(output_format_val) = req.params.response_format_for(ProviderFormat::Anthropic) {
            obj.insert("output_format".into(), output_format_val);
        }

        // Merge back provider-specific extras (only for Anthropic)
        if let Some(extras) = req.params.extras.get(&ProviderFormat::Anthropic) {
            for (k, v) in extras {
                // Don't overwrite canonical fields we already handled
                if !obj.contains_key(k) {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }

        Ok(Value::Object(obj))
    }

    fn apply_defaults(&self, req: &mut UniversalRequest) {
        // Anthropic requires max_tokens - set default if not provided
        if req.params.max_tokens.is_none() {
            req.params.max_tokens = Some(DEFAULT_MAX_TOKENS);
        }
    }

    fn detect_response(&self, payload: &Value) -> bool {
        // Anthropic response has content[] array and type="message"
        payload.get("content").and_then(Value::as_array).is_some()
            && payload
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|t| t == "message")
    }

    fn response_to_universal(&self, payload: Value) -> Result<UniversalResponse, TransformError> {
        let content = payload
            .get("content")
            .and_then(Value::as_array)
            .ok_or_else(|| TransformError::ToUniversalFailed("missing content".to_string()))?;

        let content_blocks: Vec<ContentBlock> = content
            .iter()
            .map(|v| {
                serde_json::from_value(v.clone())
                    .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let messages = <Vec<Message> as TryFromLLM<Vec<ContentBlock>>>::try_from(content_blocks)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let finish_reason = match payload.get("stop_reason").and_then(Value::as_str) {
            Some(s) => Some(s.parse().map_err(|_| ConvertError::InvalidEnumValue {
                type_name: "FinishReason",
                value: s.to_string(),
            })?),
            None => None,
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
        let content_blocks =
            <Vec<ContentBlock> as TryFromLLM<Vec<Message>>>::try_from(resp.messages.clone())
                .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

        let content_value = serde_json::to_value(&content_blocks)
            .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

        let stop_reason = resp
            .finish_reason
            .as_ref()
            .map(|r| r.to_provider_string(self.format()).to_string())
            .unwrap_or_else(|| "end_turn".to_string());

        let mut map = serde_json::Map::new();
        map.insert(
            "id".into(),
            Value::String(format!("msg_{}", PLACEHOLDER_ID)),
        );
        map.insert("type".into(), Value::String("message".into()));
        map.insert("role".into(), Value::String("assistant".into()));
        map.insert("content".into(), content_value);
        map.insert(
            "model".into(),
            Value::String(resp.model.as_deref().unwrap_or(PLACEHOLDER_MODEL).into()),
        );
        map.insert("stop_reason".into(), Value::String(stop_reason));

        if let Some(usage) = &resp.usage {
            map.insert("usage".into(), usage.to_provider_value(self.format()));
        }

        Ok(Value::Object(map))
    }

    // =========================================================================
    // Streaming response handling
    // =========================================================================

    fn detect_stream_response(&self, payload: &Value) -> bool {
        // Anthropic streaming has type field with specific event types
        if let Some(event_type) = payload.get("type").and_then(Value::as_str) {
            matches!(
                event_type,
                "message_start"
                    | "content_block_start"
                    | "content_block_delta"
                    | "content_block_stop"
                    | "message_delta"
                    | "message_stop"
                    | "ping"
            )
        } else {
            false
        }
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
            "content_block_delta" => {
                // Extract text delta - only handle text_delta type for basic text support
                let delta = payload.get("delta");
                let delta_type = delta.and_then(|d| d.get("type")).and_then(Value::as_str);

                if delta_type == Some("text_delta") {
                    let text = delta.and_then(|d| d.get("text")).and_then(Value::as_str);

                    // Use null for empty/missing text, preserving semantic equivalence with source
                    let content_value = match text {
                        Some(t) if !t.is_empty() => Value::String(t.to_string()),
                        _ => Value::Null, // Empty or missing text becomes null
                    };

                    let index = payload.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;

                    return Ok(Some(UniversalStreamChunk::new(
                        None,
                        None,
                        vec![UniversalStreamChoice {
                            index,
                            delta: Some(serde_json::json!({
                                "role": "assistant",
                                "content": content_value
                            })),
                            finish_reason: None,
                        }],
                        None,
                        None,
                    )));
                }

                // For non-text deltas (tool_use, etc.), return keep-alive
                Ok(Some(UniversalStreamChunk::keep_alive()))
            }

            "message_delta" => {
                // Contains stop_reason and final usage
                let stop_reason = payload
                    .get("delta")
                    .and_then(|d| d.get("stop_reason"))
                    .and_then(Value::as_str);

                let finish_reason = stop_reason
                    .map(|r| FinishReason::from_provider_string(r, self.format()).to_string());

                let usage = UniversalUsage::extract_from_response(&payload, self.format());

                if finish_reason.is_some() || usage.is_some() {
                    return Ok(Some(UniversalStreamChunk::new(
                        None,
                        None,
                        vec![UniversalStreamChoice {
                            index: 0,
                            delta: Some(serde_json::json!({})),
                            finish_reason,
                        }],
                        None,
                        usage,
                    )));
                }

                Ok(Some(UniversalStreamChunk::keep_alive()))
            }

            "message_start" => {
                // Extract initial usage and model info
                let message = payload.get("message");
                let model = message
                    .and_then(|m| m.get("model"))
                    .and_then(Value::as_str)
                    .map(String::from);
                let id = message
                    .and_then(|m| m.get("id"))
                    .and_then(Value::as_str)
                    .map(String::from);
                let usage = message
                    .and_then(|m| m.get("usage"))
                    .map(|u| UniversalUsage::from_provider_value(u, self.format()));

                // Return chunk with metadata but mark as role initialization
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

            "message_stop" => {
                // Terminal event - don't emit any chunk
                Ok(None)
            }

            "content_block_start" | "content_block_stop" | "ping" => {
                // Metadata events - return keep-alive
                Ok(Some(UniversalStreamChunk::keep_alive()))
            }

            _ => {
                // Unknown event type - return keep-alive
                Ok(Some(UniversalStreamChunk::keep_alive()))
            }
        }
    }

    fn stream_from_universal(&self, chunk: &UniversalStreamChunk) -> Result<Value, TransformError> {
        if chunk.is_keep_alive() {
            // Return a ping event for keep-alive
            return Ok(serde_json::json!({"type": "ping"}));
        }

        // Check if this is a finish chunk
        let has_finish = chunk
            .choices
            .first()
            .and_then(|c| c.finish_reason.as_ref())
            .is_some();

        // Check if this is an initial metadata chunk (has model/id/usage but no content)
        let is_initial_metadata =
            (chunk.model.is_some() || chunk.id.is_some() || chunk.usage.is_some())
                && !has_finish
                && chunk
                    .choices
                    .first()
                    .and_then(|c| c.delta.as_ref())
                    .is_none_or(|d| {
                        // Initial chunk has role but empty/no content
                        d.get("content")
                            .and_then(Value::as_str)
                            .is_none_or(|s| s.is_empty())
                    });

        if is_initial_metadata {
            // Return message_start with model/id/usage
            let id = chunk
                .id
                .clone()
                .unwrap_or_else(|| format!("msg_{}", PLACEHOLDER_ID));

            let mut message = serde_json::json!({
                "id": id,
                "type": "message",
                "role": "assistant",
                "model": chunk.model.as_deref().unwrap_or(PLACEHOLDER_MODEL),
                "content": [],
                "stop_reason": null,
                "stop_sequence": null
            });

            if let Some(usage) = &chunk.usage {
                if let Some(obj) = message.as_object_mut() {
                    obj.insert("usage".into(), usage.to_provider_value(self.format()));
                }
            }

            return Ok(serde_json::json!({
                "type": "message_start",
                "message": message
            }));
        }

        if has_finish {
            // Generate message_delta with stop_reason
            let finish_reason = chunk.choices.first().and_then(|c| c.finish_reason.as_ref());
            let stop_reason = finish_reason.map(|r| match r.as_str() {
                "stop" => "end_turn",
                "length" => "max_tokens",
                "tool_calls" => "tool_use",
                other => other,
            });

            let mut obj = serde_json::json!({
                "type": "message_delta",
                "delta": {
                    "stop_reason": stop_reason
                }
            });

            if let Some(usage) = &chunk.usage {
                if let Some(obj_map) = obj.as_object_mut() {
                    obj_map.insert("usage".into(), usage.to_provider_value(self.format()));
                }
            }

            return Ok(obj);
        }

        // Check if this is a content delta
        if let Some(choice) = chunk.choices.first() {
            if let Some(delta) = &choice.delta {
                if let Some(content) = delta.get("content").and_then(Value::as_str) {
                    return Ok(serde_json::json!({
                        "type": "content_block_delta",
                        "index": choice.index,
                        "delta": {
                            "type": "text_delta",
                            "text": content
                        }
                    }));
                }

                // Role-only delta or null content - return empty text_delta
                // Treat null content the same as missing content (semantically equivalent)
                // Using text_delta (instead of content_block_start) ensures proper roundtrip
                // since our stream_to_universal converts empty text back to null
                // Note: When tool_calls are present with null content, this will emit empty text
                // which is documented as an expected limitation in streaming_expected_differences.json
                let content_is_missing_or_null =
                    delta.get("content").is_none() || delta.get("content") == Some(&Value::Null);

                if delta.get("role").is_some() && content_is_missing_or_null {
                    return Ok(serde_json::json!({
                        "type": "content_block_delta",
                        "index": choice.index,
                        "delta": {
                            "type": "text_delta",
                            "text": ""
                        }
                    }));
                }
            }
        }

        // Fallback - return content_block_delta with empty text
        Ok(serde_json::json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "text_delta",
                "text": ""
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_anthropic_detect_request() {
        let adapter = AnthropicAdapter;
        let payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(adapter.detect_request(&payload));
    }

    #[test]
    fn test_anthropic_passthrough() {
        let adapter = AnthropicAdapter;
        let payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let universal = adapter.request_to_universal(payload).unwrap();
        assert_eq!(
            universal.model,
            Some("claude-3-5-sonnet-20241022".to_string())
        );
        assert_eq!(universal.params.max_tokens, Some(1024));

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert_eq!(
            reconstructed.get("model").unwrap(),
            "claude-3-5-sonnet-20241022"
        );
        assert_eq!(reconstructed.get("max_tokens").unwrap(), 1024);
    }

    #[test]
    fn test_anthropic_apply_defaults() {
        let adapter = AnthropicAdapter;
        let mut req = UniversalRequest {
            model: Some("claude-3-5-sonnet-20241022".to_string()),
            messages: vec![],
            params: UniversalParams::default(),
        };

        assert!(req.params.max_tokens.is_none());
        adapter.apply_defaults(&mut req);
        assert_eq!(req.params.max_tokens, Some(DEFAULT_MAX_TOKENS));
    }

    #[test]
    fn test_anthropic_preserves_existing_max_tokens() {
        let adapter = AnthropicAdapter;
        let mut req = UniversalRequest {
            model: Some("claude-3-5-sonnet-20241022".to_string()),
            messages: vec![],
            params: UniversalParams {
                max_tokens: Some(8192),
                ..Default::default()
            },
        };

        adapter.apply_defaults(&mut req);
        assert_eq!(req.params.max_tokens, Some(8192));
    }

    #[test]
    fn test_anthropic_omits_temperature_with_thinking() {
        use crate::universal::message::UserContent;
        use crate::universal::request::ReasoningConfig;

        let adapter = AnthropicAdapter;

        // Request with thinking enabled and user-specified temperature
        let req = UniversalRequest {
            model: Some("claude-sonnet-4-20250514".to_string()),
            messages: vec![Message::User {
                content: UserContent::String("Hello".to_string()),
            }],
            params: UniversalParams {
                temperature: Some(0.5), // User specified, but should be omitted
                reasoning: Some(ReasoningConfig {
                    enabled: Some(true),
                    budget_tokens: Some(2048),
                    ..Default::default()
                }),
                max_tokens: Some(4096),
                ..Default::default()
            },
        };

        let result = adapter.request_from_universal(&req).unwrap();

        assert!(
            result.get("temperature").is_none(),
            "Temperature should be omitted when thinking is enabled"
        );
        assert!(
            result.get("thinking").is_some(),
            "thinking field should be present"
        );
    }

    #[test]
    fn test_anthropic_preserves_temperature_without_thinking() {
        use crate::universal::message::UserContent;

        let adapter = AnthropicAdapter;

        // Request without thinking - temperature should be preserved
        let req = UniversalRequest {
            model: Some("claude-3-5-sonnet-20241022".to_string()),
            messages: vec![Message::User {
                content: UserContent::String("Hello".to_string()),
            }],
            params: UniversalParams {
                temperature: Some(0.7),
                max_tokens: Some(1024),
                ..Default::default()
            },
        };

        let result = adapter.request_from_universal(&req).unwrap();

        assert_eq!(
            result.get("temperature").unwrap().as_f64().unwrap(),
            0.7,
            "Temperature should be preserved when thinking is not enabled"
        );
        assert!(
            result.get("thinking").is_none(),
            "thinking field should not be present"
        );
    }

    #[test]
    fn test_anthropic_output_format_roundtrip() {
        let adapter = AnthropicAdapter;

        // Anthropic request with output_format (structured outputs)
        let payload = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Extract: John is 25."}],
            "output_format": {
                "type": "json_schema",
                "schema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "age": { "type": "number" }
                    },
                    "required": ["name", "age"],
                    "additionalProperties": false
                }
            }
        });

        // Parse to universal
        let universal = adapter.request_to_universal(payload.clone()).unwrap();

        // Verify response_format is parsed
        assert!(
            universal.params.response_format.is_some(),
            "response_format should be parsed from output_format"
        );

        // Convert back to Anthropic
        let reconstructed = adapter.request_from_universal(&universal).unwrap();

        // Verify output_format is preserved
        assert!(
            reconstructed.get("output_format").is_some(),
            "output_format should be present in reconstructed request"
        );
        let output_format = reconstructed.get("output_format").unwrap();
        assert_eq!(output_format.get("type").unwrap(), "json_schema");
        assert!(output_format.get("schema").is_some());
    }

    #[test]
    fn test_anthropic_cross_provider_output_format() {
        use crate::processing::adapters::ProviderAdapter;
        use crate::providers::openai::adapter::OpenAIAdapter;
        use crate::universal::request::ResponseFormatType;

        let openai_adapter = OpenAIAdapter;
        let anthropic_adapter = AnthropicAdapter;

        // OpenAI request with response_format
        let openai_payload = json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Extract: John is 25."}],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "person_info",
                    "schema": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "age": { "type": "number" }
                        },
                        "required": ["name", "age"]
                    },
                    "strict": true
                }
            }
        });

        // Parse OpenAI to universal
        let universal = openai_adapter.request_to_universal(openai_payload).unwrap();
        assert!(universal.params.response_format.is_some());
        assert_eq!(
            universal
                .params
                .response_format
                .as_ref()
                .unwrap()
                .format_type,
            Some(ResponseFormatType::JsonSchema)
        );

        // Convert to Anthropic
        let mut universal_for_anthropic = universal;
        universal_for_anthropic.model = Some("claude-sonnet-4-5-20250929".to_string());
        anthropic_adapter.apply_defaults(&mut universal_for_anthropic);

        let anthropic_request = anthropic_adapter
            .request_from_universal(&universal_for_anthropic)
            .unwrap();

        // Verify Anthropic output_format structure
        let output_format = anthropic_request.get("output_format").unwrap();
        assert_eq!(output_format.get("type").unwrap(), "json_schema");
        assert!(output_format.get("schema").is_some());
        // Name should NOT be included (Anthropic doesn't support it)
        assert!(output_format.get("name").is_none());
        // strict is NOT supported in Anthropic output_format (it's for tools only)
        assert!(output_format.get("strict").is_none());
        // Anthropic format doesn't have nested json_schema wrapper
        assert!(output_format.get("json_schema").is_none());
    }
}

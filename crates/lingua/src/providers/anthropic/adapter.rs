/*!
Anthropic provider adapter for Messages API.

Anthropic's Messages API has some unique requirements:
- `max_tokens` is required (we use a default of 4096)
- System messages use a separate `system` parameter, not in `messages` array
*/

use crate::capabilities::ProviderFormat;
use crate::processing::adapters::{
    collect_extras, insert_opt_bool, insert_opt_f64, insert_opt_i64, insert_opt_value,
    ProviderAdapter,
};
use crate::processing::transform::TransformError;
use crate::providers::anthropic::generated::{ContentBlock, CreateMessageParams, InputMessage};
use crate::providers::anthropic::try_parse_anthropic;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{Message, UserContent};
use crate::universal::transform::extract_system_messages;
use crate::universal::{
    FinishReason, UniversalParams, UniversalRequest, UniversalResponse, UniversalStreamChoice,
    UniversalStreamChunk, UniversalUsage, PLACEHOLDER_ID, PLACEHOLDER_MODEL,
};

/// Default max_tokens for Anthropic requests (matches legacy proxy behavior).
pub const DEFAULT_MAX_TOKENS: i64 = 4096;

/// Known request fields for Anthropic Messages API.
/// Fields not in this list go into `extras`.
const ANTHROPIC_KNOWN_KEYS: &[&str] = &[
    "model",
    "messages",
    "system",
    "max_tokens",
    "temperature",
    "top_p",
    "top_k",
    "stop_sequences",
    "stream",
    "metadata",
    "tools",
    "tool_choice",
];

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
        let extras = collect_extras(&payload, ANTHROPIC_KNOWN_KEYS);
        let stop = payload.get("stop_sequences").cloned();

        let request: CreateMessageParams = serde_json::from_value(payload)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let messages = <Vec<Message> as TryFromLLM<Vec<_>>>::try_from(request.messages)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let params = UniversalParams {
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: request.top_k,
            max_tokens: Some(request.max_tokens),
            stop,
            tools: request.tools.and_then(|t| serde_json::to_value(t).ok()),
            tool_choice: request
                .tool_choice
                .and_then(|t| serde_json::to_value(t).ok()),
            response_format: None,  // Anthropic doesn't use response_format
            seed: None,             // Anthropic doesn't support seed
            presence_penalty: None, // Anthropic doesn't support these
            frequency_penalty: None,
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

        // Insert other params
        insert_opt_f64(&mut obj, "temperature", req.params.temperature);
        insert_opt_f64(&mut obj, "top_p", req.params.top_p);
        insert_opt_i64(&mut obj, "top_k", req.params.top_k);

        // Anthropic uses stop_sequences instead of stop
        if let Some(stop) = &req.params.stop {
            obj.insert("stop_sequences".into(), stop.clone());
        }

        insert_opt_value(&mut obj, "tools", req.params.tools.clone());
        insert_opt_value(&mut obj, "tool_choice", req.params.tool_choice.clone());
        insert_opt_bool(&mut obj, "stream", req.params.stream);

        // Merge extras - only include Anthropic-known fields
        // This filters out OpenAI-specific fields like stream_options that would cause
        // Anthropic to reject the request with "extra inputs are not permitted"
        for (k, v) in &req.extras {
            if ANTHROPIC_KNOWN_KEYS.contains(&k.as_str()) {
                obj.insert(k.clone(), v.clone());
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

        let finish_reason = payload
            .get("stop_reason")
            .and_then(Value::as_str)
            .map(|s| s.parse().unwrap());

        let usage = payload.get("usage").map(|u| UniversalUsage {
            prompt_tokens: u.get("input_tokens").and_then(Value::as_i64),
            completion_tokens: u.get("output_tokens").and_then(Value::as_i64),
            prompt_cached_tokens: u.get("cache_read_input_tokens").and_then(Value::as_i64),
            prompt_cache_creation_tokens: u
                .get("cache_creation_input_tokens")
                .and_then(Value::as_i64),
            completion_reasoning_tokens: None, // Anthropic doesn't expose thinking tokens separately
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
        let content_blocks =
            <Vec<ContentBlock> as TryFromLLM<Vec<Message>>>::try_from(resp.messages.clone())
                .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

        let content_value = serde_json::to_value(&content_blocks)
            .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

        let stop_reason = self
            .map_finish_reason(resp.finish_reason.as_ref())
            .unwrap_or_else(|| "end_turn".to_string());

        let mut obj = serde_json::json!({
            "id": format!("msg_{}", PLACEHOLDER_ID),
            "type": "message",
            "role": "assistant",
            "content": content_value,
            "model": resp.model.as_deref().unwrap_or(PLACEHOLDER_MODEL),
            "stop_reason": stop_reason
        });

        if let Some(usage) = &resp.usage {
            obj.as_object_mut().unwrap().insert(
                "usage".into(),
                serde_json::json!({
                    "input_tokens": usage.prompt_tokens.unwrap_or(0),
                    "output_tokens": usage.completion_tokens.unwrap_or(0)
                }),
            );
        }

        Ok(obj)
    }

    fn map_finish_reason(&self, reason: Option<&FinishReason>) -> Option<String> {
        reason.map(|r| match r {
            FinishReason::Stop => "end_turn".to_string(),
            FinishReason::Length => "max_tokens".to_string(),
            FinishReason::ToolCalls => "tool_use".to_string(),
            FinishReason::ContentFilter => "content_filter".to_string(),
            FinishReason::Other(s) => s.clone(),
        })
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
                    let text = delta
                        .and_then(|d| d.get("text"))
                        .and_then(Value::as_str)
                        .unwrap_or("");

                    let index = payload.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;

                    return Ok(Some(UniversalStreamChunk::new(
                        None,
                        None,
                        vec![UniversalStreamChoice {
                            index,
                            delta: Some(serde_json::json!({
                                "role": "assistant",
                                "content": text
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

                let finish_reason = stop_reason.map(|r| match r {
                    "end_turn" | "stop_sequence" => "stop".to_string(),
                    "max_tokens" => "length".to_string(),
                    "tool_use" => "tool_calls".to_string(),
                    other => other.to_string(),
                });

                let usage = payload.get("usage").map(|u| UniversalUsage {
                    prompt_tokens: u.get("input_tokens").and_then(Value::as_i64),
                    completion_tokens: u.get("output_tokens").and_then(Value::as_i64),
                    prompt_cached_tokens: u.get("cache_read_input_tokens").and_then(Value::as_i64),
                    prompt_cache_creation_tokens: u
                        .get("cache_creation_input_tokens")
                        .and_then(Value::as_i64),
                    completion_reasoning_tokens: None,
                });

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
                    .map(|u| UniversalUsage {
                        prompt_tokens: u.get("input_tokens").and_then(Value::as_i64),
                        completion_tokens: u.get("output_tokens").and_then(Value::as_i64),
                        prompt_cached_tokens: u
                            .get("cache_read_input_tokens")
                            .and_then(Value::as_i64),
                        prompt_cache_creation_tokens: u
                            .get("cache_creation_input_tokens")
                            .and_then(Value::as_i64),
                        completion_reasoning_tokens: None,
                    });

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
                obj.as_object_mut().unwrap().insert(
                    "usage".into(),
                    serde_json::json!({
                        "input_tokens": usage.prompt_tokens.unwrap_or(0),
                        "output_tokens": usage.completion_tokens.unwrap_or(0)
                    }),
                );
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

                // Role-only delta (initial chunk) - return content_block_start
                if delta.get("role").is_some() && delta.get("content").is_none() {
                    return Ok(serde_json::json!({
                        "type": "content_block_start",
                        "index": choice.index,
                        "content_block": {
                            "type": "text",
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
            extras: Map::new(),
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
            extras: Map::new(),
        };

        adapter.apply_defaults(&mut req);
        assert_eq!(req.params.max_tokens, Some(8192));
    }
}

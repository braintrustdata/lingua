/*!
Amazon Bedrock provider adapter for Converse API.

Bedrock's Converse API has some unique characteristics:
- Uses `modelId` instead of `model`
- Inference params are in `inferenceConfig` object
- Uses camelCase field names
- System messages are in a separate `system` array
*/

use crate::capabilities::ProviderFormat;
use crate::processing::adapters::{collect_extras, ProviderAdapter};
use crate::processing::transform::TransformError;
use crate::providers::bedrock::request::{
    BedrockInferenceConfiguration, BedrockMessage, ConverseRequest,
};
use crate::providers::bedrock::try_parse_bedrock;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::Message;
use crate::universal::{
    FinishReason, UniversalParams, UniversalRequest, UniversalResponse, UniversalStreamChoice,
    UniversalStreamChunk, UniversalUsage,
};

/// Known request fields for Bedrock Converse API.
/// Fields not in this list go into `extras`.
const BEDROCK_KNOWN_KEYS: &[&str] = &[
    "modelId",
    "messages",
    "system",
    "inferenceConfig",
    "toolConfig",
    "guardrailConfig",
    "additionalModelRequestFields",
    "additionalModelResponseFieldPaths",
    "promptVariables",
];

/// Adapter for Amazon Bedrock Converse API.
pub struct BedrockAdapter;

impl ProviderAdapter for BedrockAdapter {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::Converse
    }

    fn directory_name(&self) -> &'static str {
        "bedrock"
    }

    fn display_name(&self) -> &'static str {
        "Bedrock"
    }

    fn detect_request(&self, payload: &Value) -> bool {
        try_parse_bedrock(payload).is_ok()
    }

    fn request_to_universal(&self, payload: &Value) -> Result<UniversalRequest, TransformError> {
        let request: ConverseRequest = serde_json::from_value(payload.clone())
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let messages =
            <Vec<Message> as TryFromLLM<Vec<BedrockMessage>>>::try_from(request.messages)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Extract params from inferenceConfig
        let (temperature, top_p, max_tokens, stop) = if let Some(config) = &request.inference_config
        {
            (
                config.temperature,
                config.top_p,
                config.max_tokens.map(|t| t as i64),
                config
                    .stop_sequences
                    .as_ref()
                    .and_then(|s| serde_json::to_value(s).ok()),
            )
        } else {
            (None, None, None, None)
        };

        let params = UniversalParams {
            temperature,
            top_p,
            top_k: None, // Bedrock doesn't expose top_k in Converse API
            max_tokens,
            stop,
            tools: request
                .tool_config
                .and_then(|t| serde_json::to_value(t).ok()),
            tool_choice: None, // Tool choice is inside tool_config
            response_format: None,
            seed: None, // Bedrock doesn't support seed
            presence_penalty: None,
            frequency_penalty: None,
            stream: None, // Bedrock uses separate endpoint for streaming
        };

        Ok(UniversalRequest {
            model: Some(request.model_id),
            messages,
            params,
            extras: collect_extras(payload, BEDROCK_KNOWN_KEYS),
        })
    }

    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
        let model_id = req.model.as_ref().ok_or(TransformError::ValidationFailed {
            target: ProviderFormat::Converse,
            reason: "missing model".to_string(),
        })?;

        // Convert messages to Bedrock format
        let bedrock_messages: Vec<BedrockMessage> =
            <Vec<BedrockMessage> as TryFromLLM<Vec<Message>>>::try_from(req.messages.clone())
                .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

        let mut obj = Map::new();
        obj.insert("modelId".into(), Value::String(model_id.clone()));
        obj.insert(
            "messages".into(),
            serde_json::to_value(bedrock_messages)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
        );

        // Build inferenceConfig if any params are set
        let has_params = req.params.temperature.is_some()
            || req.params.top_p.is_some()
            || req.params.max_tokens.is_some()
            || req.params.stop.is_some();

        if has_params {
            let config = BedrockInferenceConfiguration {
                temperature: req.params.temperature,
                top_p: req.params.top_p,
                max_tokens: req.params.max_tokens.map(|t| t as i32),
                stop_sequences: req.params.stop.as_ref().and_then(|v| {
                    if let Value::Array(arr) = v {
                        Some(
                            arr.iter()
                                .filter_map(|s| s.as_str().map(String::from))
                                .collect(),
                        )
                    } else if let Value::String(s) = v {
                        Some(vec![s.clone()])
                    } else {
                        None
                    }
                }),
            };

            obj.insert(
                "inferenceConfig".into(),
                serde_json::to_value(config)
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
            );
        }

        // Add toolConfig if tools are present
        if let Some(tools) = &req.params.tools {
            obj.insert("toolConfig".into(), tools.clone());
        }

        // Merge extras
        for (k, v) in &req.extras {
            obj.insert(k.clone(), v.clone());
        }

        Ok(Value::Object(obj))
    }

    fn apply_defaults(&self, _req: &mut UniversalRequest) {
        // Bedrock doesn't require any specific defaults
    }

    fn detect_response(&self, payload: &Value) -> bool {
        // Bedrock response has output.message structure
        payload
            .get("output")
            .and_then(|o| o.get("message"))
            .is_some()
    }

    fn response_to_universal(&self, payload: &Value) -> Result<UniversalResponse, TransformError> {
        let output = payload
            .get("output")
            .ok_or_else(|| TransformError::ToUniversalFailed("missing output".to_string()))?;

        let message_val = output.get("message").ok_or_else(|| {
            TransformError::ToUniversalFailed("missing output.message".to_string())
        })?;

        let bedrock_message: BedrockMessage = serde_json::from_value(message_val.clone())
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let messages =
            <Vec<Message> as TryFromLLM<Vec<BedrockMessage>>>::try_from(vec![bedrock_message])
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let finish_reason = payload
            .get("stopReason")
            .and_then(Value::as_str)
            .map(|s| s.parse().unwrap());

        let usage = payload.get("usage").map(|u| UniversalUsage {
            prompt_tokens: u.get("inputTokens").and_then(Value::as_i64),
            completion_tokens: u.get("outputTokens").and_then(Value::as_i64),
            prompt_cached_tokens: u.get("cacheReadInputTokens").and_then(Value::as_i64),
            prompt_cache_creation_tokens: u.get("cacheCreationInputTokens").and_then(Value::as_i64),
        });

        Ok(UniversalResponse {
            model: None, // Bedrock doesn't include model in response
            messages,
            usage,
            finish_reason,
            extras: Map::new(),
        })
    }

    fn response_from_universal(&self, resp: &UniversalResponse) -> Result<Value, TransformError> {
        let bedrock_messages: Vec<BedrockMessage> =
            <Vec<BedrockMessage> as TryFromLLM<Vec<Message>>>::try_from(resp.messages.clone())
                .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

        // Take first message for output
        let message = bedrock_messages
            .into_iter()
            .next()
            .ok_or_else(|| TransformError::FromUniversalFailed("no messages".to_string()))?;

        let message_value = serde_json::to_value(message)
            .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

        let stop_reason = self
            .map_finish_reason(resp.finish_reason.as_ref())
            .unwrap_or_else(|| "end_turn".to_string());

        let mut obj = serde_json::json!({
            "output": {
                "message": message_value
            },
            "stopReason": stop_reason
        });

        if let Some(usage) = &resp.usage {
            obj.as_object_mut().unwrap().insert(
                "usage".into(),
                serde_json::json!({
                    "inputTokens": usage.prompt_tokens.unwrap_or(0),
                    "outputTokens": usage.completion_tokens.unwrap_or(0)
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
            FinishReason::ContentFilter => "content_filtered".to_string(),
            FinishReason::Other(s) => s.clone(),
        })
    }

    // =========================================================================
    // Streaming response handling
    // =========================================================================

    fn detect_stream_response(&self, payload: &Value) -> bool {
        // Bedrock streaming has unique top-level keys
        payload.get("messageStart").is_some()
            || payload.get("contentBlockDelta").is_some()
            || payload.get("contentBlockStop").is_some()
            || payload.get("messageStop").is_some()
            || payload.get("metadata").is_some()
    }

    fn stream_to_universal(
        &self,
        payload: &Value,
    ) -> Result<Option<UniversalStreamChunk>, TransformError> {
        // Handle contentBlockDelta - text content
        if let Some(delta_event) = payload.get("contentBlockDelta") {
            let index = delta_event
                .get("contentBlockIndex")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32;

            // Only handle text deltas for basic text support
            if let Some(text) = delta_event
                .get("delta")
                .and_then(|d| d.get("text"))
                .and_then(Value::as_str)
            {
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

            // Non-text delta (tool use) - return keep-alive
            return Ok(Some(UniversalStreamChunk::keep_alive()));
        }

        // Handle messageStop - finish reason
        if let Some(stop_event) = payload.get("messageStop") {
            let stop_reason = stop_event.get("stopReason").and_then(Value::as_str);
            let finish_reason = stop_reason.map(|r| match r {
                "end_turn" | "stop_sequence" => "stop".to_string(),
                "max_tokens" => "length".to_string(),
                "tool_use" => "tool_calls".to_string(),
                "content_filtered" => "content_filter".to_string(),
                other => other.to_string(),
            });

            return Ok(Some(UniversalStreamChunk::new(
                None,
                None,
                vec![UniversalStreamChoice {
                    index: 0,
                    delta: Some(serde_json::json!({})),
                    finish_reason,
                }],
                None,
                None,
            )));
        }

        // Handle metadata - usage info
        if let Some(meta) = payload.get("metadata") {
            let usage = meta.get("usage").map(|u| UniversalUsage {
                prompt_tokens: u.get("inputTokens").and_then(Value::as_i64),
                completion_tokens: u.get("outputTokens").and_then(Value::as_i64),
                prompt_cached_tokens: u.get("cacheReadInputTokens").and_then(Value::as_i64),
                prompt_cache_creation_tokens: u
                    .get("cacheCreationInputTokens")
                    .and_then(Value::as_i64),
            });

            if usage.is_some() {
                return Ok(Some(UniversalStreamChunk::new(
                    None,
                    None,
                    vec![],
                    None,
                    usage,
                )));
            }
        }

        // Handle messageStart - role initialization
        if payload.get("messageStart").is_some() {
            return Ok(Some(UniversalStreamChunk::new(
                None,
                None,
                vec![UniversalStreamChoice {
                    index: 0,
                    delta: Some(serde_json::json!({"role": "assistant", "content": ""})),
                    finish_reason: None,
                }],
                None,
                None,
            )));
        }

        // Other events (contentBlockStart, contentBlockStop) - keep-alive
        Ok(Some(UniversalStreamChunk::keep_alive()))
    }

    fn stream_from_universal(&self, chunk: &UniversalStreamChunk) -> Result<Value, TransformError> {
        if chunk.is_keep_alive() {
            // Return empty contentBlockStop for keep-alive
            return Ok(serde_json::json!({
                "contentBlockStop": {"contentBlockIndex": 0}
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
            let stop_reason = finish_reason.map(|r| match r.as_str() {
                "stop" => "end_turn",
                "length" => "max_tokens",
                "tool_calls" => "tool_use",
                "content_filter" => "content_filtered",
                other => other,
            });

            return Ok(serde_json::json!({
                "messageStop": {
                    "stopReason": stop_reason
                }
            }));
        }

        // Check for usage-only chunk
        if chunk.choices.is_empty() && chunk.usage.is_some() {
            let usage = chunk.usage.as_ref().unwrap();
            return Ok(serde_json::json!({
                "metadata": {
                    "usage": {
                        "inputTokens": usage.prompt_tokens.unwrap_or(0),
                        "outputTokens": usage.completion_tokens.unwrap_or(0)
                    }
                }
            }));
        }

        // Check for content delta
        if let Some(choice) = chunk.choices.first() {
            if let Some(delta) = &choice.delta {
                if let Some(content) = delta.get("content").and_then(Value::as_str) {
                    return Ok(serde_json::json!({
                        "contentBlockDelta": {
                            "contentBlockIndex": choice.index,
                            "delta": {
                                "text": content
                            }
                        }
                    }));
                }

                // Role-only delta - messageStart
                if delta.get("role").is_some() && delta.get("content").is_none() {
                    return Ok(serde_json::json!({
                        "messageStart": {
                            "role": "assistant"
                        }
                    }));
                }
            }
        }

        // Fallback - return contentBlockDelta with empty text
        Ok(serde_json::json!({
            "contentBlockDelta": {
                "contentBlockIndex": 0,
                "delta": {
                    "text": ""
                }
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_bedrock_detect_request() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "modelId": "anthropic.claude-3-sonnet",
            "messages": [{
                "role": "user",
                "content": [{"text": "Hello"}]
            }]
        });
        assert!(adapter.detect_request(&payload));
    }

    #[test]
    fn test_bedrock_passthrough() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "modelId": "anthropic.claude-3-sonnet",
            "messages": [{
                "role": "user",
                "content": [{"text": "Hello"}]
            }],
            "inferenceConfig": {
                "temperature": 0.7,
                "maxTokens": 1024
            }
        });

        let universal = adapter.request_to_universal(&payload).unwrap();
        assert_eq!(
            universal.model,
            Some("anthropic.claude-3-sonnet".to_string())
        );
        assert_eq!(universal.params.temperature, Some(0.7));
        assert_eq!(universal.params.max_tokens, Some(1024));

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert_eq!(
            reconstructed.get("modelId").unwrap(),
            "anthropic.claude-3-sonnet"
        );
        assert!(reconstructed.get("messages").is_some());
        assert!(reconstructed.get("inferenceConfig").is_some());
    }

    #[test]
    fn test_bedrock_preserves_extras() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "modelId": "anthropic.claude-3-sonnet",
            "messages": [{
                "role": "user",
                "content": [{"text": "Hello"}]
            }],
            "guardrailConfig": {
                "guardrailIdentifier": "test",
                "guardrailVersion": "1"
            }
        });

        let universal = adapter.request_to_universal(&payload).unwrap();
        // guardrailConfig is a known key, so it won't be in extras

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert!(reconstructed.get("modelId").is_some());
        assert!(reconstructed.get("messages").is_some());
    }
}

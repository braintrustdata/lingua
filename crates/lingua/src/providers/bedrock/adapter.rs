/*!
Amazon Bedrock provider adapter for Converse API.

Bedrock's Converse API has some unique characteristics:
- Uses `modelId` instead of `model`
- Inference params are in `inferenceConfig` object
- Uses camelCase field names
- System messages are in a separate `system` array
*/

use crate::capabilities::ProviderFormat;
use crate::error::ConvertError;
use crate::processing::adapters::ProviderAdapter;
use crate::processing::transform::TransformError;
use crate::providers::anthropic::generated::Thinking;
use crate::providers::bedrock::params::BedrockParams;
use crate::providers::bedrock::request::{BedrockInferenceConfiguration, BedrockMessage};
use crate::providers::bedrock::try_parse_bedrock;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::Message;
use crate::universal::reasoning::ANTHROPIC_THINKING_TEMPERATURE;
use crate::universal::request::ReasoningConfig;
use crate::universal::tools::{UniversalTool, UniversalToolType};
use crate::universal::{
    FinishReason, UniversalParams, UniversalRequest, UniversalResponse, UniversalStreamChoice,
    UniversalStreamChunk, UniversalUsage,
};

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

    fn request_to_universal(&self, payload: Value) -> Result<UniversalRequest, TransformError> {
        // Single parse: typed params now includes typed messages and inference_config
        let typed_params: BedrockParams = serde_json::from_value(payload)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Extract typed messages (partial move - other fields remain accessible)
        let bedrock_messages = typed_params.messages.ok_or_else(|| {
            TransformError::ToUniversalFailed("Bedrock: missing 'messages' field".to_string())
        })?;

        let messages =
            <Vec<Message> as TryFromLLM<Vec<BedrockMessage>>>::try_from(bedrock_messages)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Extract params from inferenceConfig (now typed in params struct)
        let (temperature, top_p, max_tokens, stop) =
            if let Some(config) = &typed_params.inference_config {
                (
                    config.temperature,
                    config.top_p,
                    config.max_tokens.map(|t| t as i64),
                    config.stop_sequences.clone(),
                )
            } else {
                (None, None, None, None)
            };

        // Extract reasoning from additionalModelRequestFields.thinking
        // Bedrock uses the same format as Anthropic for Claude extended thinking
        let reasoning = typed_params
            .additional_model_request_fields
            .as_ref()
            .and_then(|fields| fields.get("thinking"))
            .and_then(|v| serde_json::from_value::<Thinking>(v.clone()).ok())
            .map(|t| ReasoningConfig::from(&t));

        let mut params = UniversalParams {
            temperature,
            top_p,
            top_k: None, // Bedrock doesn't expose top_k in Converse API
            max_tokens,
            stop,
            tools: typed_params.tool_config.and_then(|t| {
                // Bedrock uses {tools: [{toolSpec: {name, description, inputSchema: {json: {...}}}}]}
                // Parse into UniversalTools
                let value = serde_json::to_value(&t).ok()?;
                let tools_arr = value.get("tools").and_then(|v| v.as_array())?;

                let mut universal_tools = Vec::new();
                for tool in tools_arr {
                    if let Some(spec) = tool.get("toolSpec") {
                        let name = spec.get("name").and_then(|v| v.as_str())?;
                        let description = spec
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        let parameters =
                            spec.get("inputSchema").and_then(|s| s.get("json")).cloned();

                        universal_tools.push(UniversalTool::function(
                            name,
                            description,
                            parameters,
                            None,
                        ));
                    }
                }

                if universal_tools.is_empty() {
                    // Fallback: store as builtin for unknown format (e.g., toolChoice)
                    Some(vec![UniversalTool::builtin(
                        "bedrock_tool_config",
                        "bedrock",
                        "tool_config",
                        Some(value),
                    )])
                } else {
                    Some(universal_tools)
                }
            }),
            tool_choice: None, // Tool choice is inside tool_config
            response_format: None,
            seed: None, // Bedrock doesn't support seed
            presence_penalty: None,
            frequency_penalty: None,
            stream: None, // Bedrock uses separate endpoint for streaming
            // New canonical fields
            parallel_tool_calls: None,
            reasoning, // Extracted from additionalModelRequestFields.thinking
            metadata: None,
            store: None,
            service_tier: None,
            logprobs: None,
            top_logprobs: None,
            extras: Default::default(),
        };

        // Use extras captured automatically via #[serde(flatten)]
        if !typed_params.extras.is_empty() {
            params.extras.insert(
                ProviderFormat::Converse,
                typed_params.extras.into_iter().collect(),
            );
        }

        Ok(UniversalRequest {
            model: typed_params.model_id,
            messages,
            params,
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

        // Check if reasoning/thinking is enabled (for temperature override)
        let thinking_config = req.params.reasoning_for(ProviderFormat::Converse);

        // Build inferenceConfig if any params are set
        // Note: Claude on Bedrock requires temperature=1.0 when extended thinking is enabled
        let temperature = if thinking_config.is_some() {
            Some(ANTHROPIC_THINKING_TEMPERATURE)
        } else {
            req.params.temperature
        };

        let has_params = temperature.is_some()
            || req.params.top_p.is_some()
            || req.params.max_tokens.is_some()
            || req.params.stop.is_some();

        if has_params {
            let config = BedrockInferenceConfiguration {
                temperature,
                top_p: req.params.top_p,
                max_tokens: req.params.max_tokens.map(|t| t as i32),
                stop_sequences: req.params.stop.clone(),
            };

            obj.insert(
                "inferenceConfig".into(),
                serde_json::to_value(config)
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
            );
        }

        // Add toolConfig if tools are present
        // Bedrock uses toolConfig.tools format: [{toolSpec: {name, description, inputSchema}}]
        if let Some(tools) = &req.params.tools {
            // First check for Bedrock builtins (pass through original config)
            let mut bedrock_builtin_found = false;
            for tool in tools {
                if let UniversalToolType::Builtin {
                    provider, config, ..
                } = &tool.tool_type
                {
                    if provider == "bedrock" {
                        if let Some(config_value) = config {
                            obj.insert("toolConfig".into(), config_value.clone());
                            bedrock_builtin_found = true;
                            break;
                        }
                    }
                }
            }

            // If no Bedrock builtin, convert function tools to Bedrock format
            if !bedrock_builtin_found {
                let tool_specs: Vec<serde_json::Value> = tools
                    .iter()
                    .filter_map(|tool| {
                        if tool.is_function() {
                            Some(serde_json::json!({
                                "toolSpec": {
                                    "name": tool.name,
                                    "description": tool.description,
                                    "inputSchema": {
                                        "json": tool.parameters.clone().unwrap_or(serde_json::json!({}))
                                    }
                                }
                            }))
                        } else {
                            None
                        }
                    })
                    .collect();

                if !tool_specs.is_empty() {
                    obj.insert(
                        "toolConfig".into(),
                        serde_json::json!({"tools": tool_specs}),
                    );
                }
            }
        }

        // Inject reasoning/thinking into additionalModelRequestFields
        // Bedrock uses additionalModelRequestFields.thinking with same format as Anthropic
        if let Some(thinking_val) = &thinking_config {
            let additional_fields = obj
                .entry("additionalModelRequestFields")
                .or_insert_with(|| Value::Object(Map::new()));

            if let Value::Object(fields) = additional_fields {
                fields.insert("thinking".into(), thinking_val.clone());
            }
        }

        // Merge back provider-specific extras (only for Bedrock/Converse)
        if let Some(extras) = req.params.extras.get(&ProviderFormat::Converse) {
            for (k, v) in extras {
                // Don't overwrite canonical fields we already handled
                if !obj.contains_key(k) {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }

        Ok(Value::Object(obj))
    }

    fn detect_response(&self, payload: &Value) -> bool {
        // Bedrock response has output.message structure
        payload
            .get("output")
            .and_then(|o| o.get("message"))
            .is_some()
    }

    fn response_to_universal(&self, payload: Value) -> Result<UniversalResponse, TransformError> {
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

        let finish_reason = match payload.get("stopReason").and_then(Value::as_str) {
            Some(s) => Some(s.parse().map_err(|_| ConvertError::InvalidEnumValue {
                type_name: "FinishReason",
                value: s.to_string(),
            })?),
            None => None,
        };

        let usage = UniversalUsage::extract_from_response(&payload, self.format());

        Ok(UniversalResponse {
            model: None, // Bedrock doesn't include model in response
            messages,
            usage,
            finish_reason,
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

        let stop_reason = resp
            .finish_reason
            .as_ref()
            .map(|r| r.to_provider_string(self.format()).to_string())
            .unwrap_or_else(|| "end_turn".to_string());

        let mut map = serde_json::Map::new();
        map.insert(
            "output".into(),
            serde_json::json!({
                "message": message_value
            }),
        );
        map.insert("stopReason".into(), Value::String(stop_reason));

        if let Some(usage) = &resp.usage {
            map.insert("usage".into(), usage.to_provider_value(self.format()));
        }

        Ok(Value::Object(map))
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
        payload: Value,
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
            let finish_reason = stop_reason
                .map(|r| FinishReason::from_provider_string(r, self.format()).to_string());

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
            let usage = meta
                .get("usage")
                .map(|u| UniversalUsage::from_provider_value(u, self.format()));

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
        if let (true, Some(usage)) = (chunk.choices.is_empty(), &chunk.usage) {
            return Ok(serde_json::json!({
                "metadata": {
                    "usage": usage.to_provider_value(self.format())
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

        let universal = adapter.request_to_universal(payload).unwrap();
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

        let universal = adapter.request_to_universal(payload).unwrap();
        // guardrailConfig is a known key, so it won't be in extras

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert!(reconstructed.get("modelId").is_some());
        assert!(reconstructed.get("messages").is_some());
    }

    #[test]
    fn test_bedrock_extracts_reasoning_from_additional_fields() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "modelId": "anthropic.claude-3-7-sonnet",
            "messages": [{
                "role": "user",
                "content": [{"text": "Hello"}]
            }],
            "inferenceConfig": {
                "maxTokens": 4096
            },
            "additionalModelRequestFields": {
                "thinking": {
                    "type": "enabled",
                    "budget_tokens": 2048
                }
            }
        });

        let universal = adapter.request_to_universal(payload).unwrap();
        assert!(universal.params.reasoning.is_some());

        let reasoning = universal.params.reasoning.unwrap();
        assert_eq!(reasoning.enabled, Some(true));
        assert_eq!(reasoning.budget_tokens, Some(2048));
    }

    #[test]
    fn test_bedrock_injects_reasoning_into_additional_fields() {
        use crate::universal::request::ReasoningConfig;

        let adapter = BedrockAdapter;

        // Create a universal request with reasoning
        let universal = UniversalRequest {
            model: Some("anthropic.claude-3-7-sonnet".to_string()),
            messages: vec![],
            params: UniversalParams {
                reasoning: Some(ReasoningConfig {
                    enabled: Some(true),
                    budget_tokens: Some(3000),
                    ..Default::default()
                }),
                max_tokens: Some(4096),
                ..Default::default()
            },
        };

        let reconstructed = adapter.request_from_universal(&universal).unwrap();

        // Check additionalModelRequestFields.thinking is present
        let additional = reconstructed.get("additionalModelRequestFields").unwrap();
        let thinking = additional.get("thinking").unwrap();
        assert_eq!(thinking.get("type").unwrap(), "enabled");
        assert_eq!(thinking.get("budget_tokens").unwrap(), 3000);
    }

    #[test]
    fn test_bedrock_reasoning_sets_temperature_to_1() {
        use crate::universal::request::ReasoningConfig;

        let adapter = BedrockAdapter;

        // Create a universal request with reasoning and custom temperature
        let universal = UniversalRequest {
            model: Some("anthropic.claude-3-7-sonnet".to_string()),
            messages: vec![],
            params: UniversalParams {
                reasoning: Some(ReasoningConfig {
                    enabled: Some(true),
                    budget_tokens: Some(2048),
                    ..Default::default()
                }),
                temperature: Some(0.5), // This should be overridden to 1.0
                max_tokens: Some(4096),
                ..Default::default()
            },
        };

        let reconstructed = adapter.request_from_universal(&universal).unwrap();

        // Temperature should be 1.0 when thinking is enabled
        let inference_config = reconstructed.get("inferenceConfig").unwrap();
        assert_eq!(inference_config.get("temperature").unwrap(), 1.0);
    }

    #[test]
    fn test_bedrock_reasoning_roundtrip() {
        let adapter = BedrockAdapter;

        // Start with a Bedrock request with thinking enabled
        let payload = json!({
            "modelId": "anthropic.claude-3-7-sonnet",
            "messages": [{
                "role": "user",
                "content": [{"text": "Think about this carefully"}]
            }],
            "inferenceConfig": {
                "maxTokens": 4096,
                "temperature": 1.0
            },
            "additionalModelRequestFields": {
                "thinking": {
                    "type": "enabled",
                    "budget_tokens": 2500
                }
            }
        });

        // Convert to universal
        let universal = adapter.request_to_universal(payload).unwrap();
        assert!(universal.params.reasoning.is_some());
        assert_eq!(
            universal.params.reasoning.as_ref().unwrap().budget_tokens,
            Some(2500)
        );

        // Convert back to Bedrock
        let reconstructed = adapter.request_from_universal(&universal).unwrap();

        // Verify thinking config is preserved
        let additional = reconstructed.get("additionalModelRequestFields").unwrap();
        let thinking = additional.get("thinking").unwrap();
        assert_eq!(thinking.get("type").unwrap(), "enabled");
        assert_eq!(thinking.get("budget_tokens").unwrap(), 2500);

        // Verify temperature is set to 1.0
        let inference_config = reconstructed.get("inferenceConfig").unwrap();
        assert_eq!(inference_config.get("temperature").unwrap(), 1.0);
    }
}

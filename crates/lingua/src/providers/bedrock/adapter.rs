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
use crate::providers::bedrock::convert::universal_to_bedrock_messages;
use crate::providers::bedrock::params::BedrockParams;
use crate::providers::bedrock::request::{BedrockInferenceConfiguration, BedrockMessage};
use crate::providers::bedrock::response::{
    BedrockConverseStreamEvent, BedrockStopReason, BedrockStreamContentBlockDeltaValue,
    BedrockStreamContentBlockStartValue,
};
use crate::providers::bedrock::try_parse_bedrock;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::Message;
use crate::universal::request::ReasoningConfig;
use crate::universal::tools::{BuiltinToolProvider, UniversalTool, UniversalToolType};
use crate::universal::{
    TokenBudget, UniversalParams, UniversalReasoningDelta, UniversalRequest, UniversalResponse,
    UniversalStreamChoice, UniversalStreamChunk, UniversalStreamDelta, UniversalToolCallDelta,
    UniversalToolFunctionDelta, UniversalUsage,
};

/// Adapter for Amazon Bedrock Converse API.
pub struct BedrockAdapter;

fn bedrock_stop_reason_to_finish_reason(stop_reason: &BedrockStopReason) -> &'static str {
    match stop_reason {
        BedrockStopReason::EndTurn | BedrockStopReason::StopSequence => "stop",
        BedrockStopReason::MaxTokens => "length",
        BedrockStopReason::ToolUse => "tool_calls",
        BedrockStopReason::ContentFiltered
        | BedrockStopReason::GuardrailIntervened
        | BedrockStopReason::MalformedModelOutput
        | BedrockStopReason::MalformedToolUse => "content_filter",
        BedrockStopReason::ModelContextWindowExceeded => "length",
        BedrockStopReason::Other(_) => "stop",
    }
}

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
            token_budget: max_tokens.map(TokenBudget::OutputTokens),
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
                        BuiltinToolProvider::Converse,
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
            conversation_reference: None,
            service_tier: None,
            prompt_cache_key: None,
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
        let bedrock_messages: Vec<BedrockMessage> = universal_to_bedrock_messages(&req.messages)
            .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;
        if bedrock_messages.is_empty() && !req.messages.is_empty() {
            return Err(TransformError::ValidationFailed {
                target: ProviderFormat::Converse,
                reason: "Bedrock Converse does not support dynamic tool discovery history without at least one non-discovery content message.".to_string(),
            });
        }

        let mut obj = Map::new();
        obj.insert("modelId".into(), Value::String(model_id.clone()));
        obj.insert(
            "messages".into(),
            serde_json::to_value(bedrock_messages)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
        );

        // Check if reasoning/thinking is enabled
        // Note: thinking_config can be { type: "disabled" } or { type: "enabled", ... }
        let thinking_config = req.params.reasoning_for(ProviderFormat::Converse);
        let reasoning_enabled = thinking_config
            .as_ref()
            .and_then(|v| v.get("type"))
            .and_then(|t| t.as_str())
            .is_some_and(|t| t == "enabled");
        let temperature = if reasoning_enabled {
            None
        } else {
            req.params.temperature
        };

        let has_params = temperature.is_some()
            || req.params.top_p.is_some()
            || req.params.output_token_budget().is_some()
            || req.params.stop.is_some();

        if has_params {
            let config = BedrockInferenceConfiguration {
                temperature,
                top_p: req.params.top_p,
                max_tokens: req.params.output_token_budget().map(|t| t as i32),
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
                    if matches!(provider, BuiltinToolProvider::Converse) {
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
            id: UniversalResponse::extract_id_from_payload(&payload),
            id_format: Some(self.format()),
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
        serde_json::from_value::<BedrockConverseStreamEvent>(payload.clone()).is_ok()
    }

    fn stream_to_universal(
        &self,
        payload: Value,
    ) -> Result<Option<UniversalStreamChunk>, TransformError> {
        let event: BedrockConverseStreamEvent = serde_json::from_value(payload)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        match event {
            BedrockConverseStreamEvent::ContentBlockStart {
                content_block_start,
            } => match content_block_start.start {
                BedrockStreamContentBlockStartValue::ToolUse { tool_use } => {
                    Ok(Some(UniversalStreamChunk::new(
                        None,
                        None,
                        vec![UniversalStreamChoice {
                            index: 0,
                            delta: Some(Value::from(UniversalStreamDelta {
                                role: Some("assistant".to_string()),
                                tool_calls: vec![UniversalToolCallDelta {
                                    index: Some(content_block_start.content_block_index),
                                    id: Some(tool_use.tool_use_id),
                                    call_type: Some("function".to_string()),
                                    function: Some(UniversalToolFunctionDelta {
                                        name: Some(tool_use.name),
                                        arguments: Some(String::new()),
                                    }),
                                }],
                                ..Default::default()
                            })),
                            finish_reason: None,
                        }],
                        None,
                        None,
                    )))
                }
                BedrockStreamContentBlockStartValue::Other(_) => {
                    Ok(Some(UniversalStreamChunk::keep_alive()))
                }
            },
            BedrockConverseStreamEvent::ContentBlockDelta {
                content_block_delta,
            } => match content_block_delta.delta {
                BedrockStreamContentBlockDeltaValue::Text { text } => {
                    Ok(Some(UniversalStreamChunk::new(
                        None,
                        None,
                        vec![UniversalStreamChoice {
                            index: 0,
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
                BedrockStreamContentBlockDeltaValue::ReasoningContent { reasoning_content } => {
                    let is_empty_text = reasoning_content.text.as_deref().is_none_or(str::is_empty);
                    if is_empty_text && reasoning_content.signature.is_none() {
                        return Ok(Some(UniversalStreamChunk::keep_alive()));
                    }

                    Ok(Some(UniversalStreamChunk::new(
                        None,
                        None,
                        vec![UniversalStreamChoice {
                            index: 0,
                            delta: Some(Value::from(UniversalStreamDelta {
                                role: Some("assistant".to_string()),
                                reasoning: reasoning_content
                                    .text
                                    .filter(|text| !text.is_empty())
                                    .map(|text| {
                                        vec![UniversalReasoningDelta {
                                            content: Some(text),
                                        }]
                                    })
                                    .unwrap_or_default(),
                                reasoning_signature: reasoning_content.signature,
                                ..Default::default()
                            })),
                            finish_reason: None,
                        }],
                        None,
                        None,
                    )))
                }
                BedrockStreamContentBlockDeltaValue::ToolUse { tool_use } => {
                    Ok(Some(UniversalStreamChunk::new(
                        None,
                        None,
                        vec![UniversalStreamChoice {
                            index: 0,
                            delta: Some(Value::from(UniversalStreamDelta {
                                tool_calls: vec![UniversalToolCallDelta {
                                    index: Some(content_block_delta.content_block_index),
                                    function: Some(UniversalToolFunctionDelta {
                                        arguments: Some(tool_use.input),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }],
                                ..Default::default()
                            })),
                            finish_reason: None,
                        }],
                        None,
                        None,
                    )))
                }
                BedrockStreamContentBlockDeltaValue::Citation { .. }
                | BedrockStreamContentBlockDeltaValue::Image { .. }
                | BedrockStreamContentBlockDeltaValue::ToolResult { .. }
                | BedrockStreamContentBlockDeltaValue::Other(_) => {
                    Ok(Some(UniversalStreamChunk::keep_alive()))
                }
            },
            BedrockConverseStreamEvent::MessageStop { message_stop } => {
                let finish_reason = bedrock_stop_reason_to_finish_reason(&message_stop.stop_reason);
                Ok(Some(UniversalStreamChunk::new(
                    None,
                    None,
                    vec![UniversalStreamChoice {
                        index: 0,
                        delta: Some(serde_json::json!({})),
                        finish_reason: Some(finish_reason.to_string()),
                    }],
                    None,
                    None,
                )))
            }
            BedrockConverseStreamEvent::Metadata { metadata } => {
                let usage = metadata
                    .usage
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?
                    .map(|u| UniversalUsage::from_provider_value(&u, self.format()));

                if usage.is_some() {
                    Ok(Some(UniversalStreamChunk::new(
                        None,
                        None,
                        vec![],
                        None,
                        usage,
                    )))
                } else {
                    Ok(Some(UniversalStreamChunk::keep_alive()))
                }
            }
            BedrockConverseStreamEvent::MessageStart { .. }
            | BedrockConverseStreamEvent::ContentBlockStop { .. } => {
                Ok(Some(UniversalStreamChunk::keep_alive()))
            }
        }
    }

    fn stream_from_universal(&self, chunk: &UniversalStreamChunk) -> Result<Value, TransformError> {
        if chunk.is_keep_alive() {
            return Ok(serde_json::json!({
                "contentBlockStop": {"contentBlockIndex": 0}
            }));
        }

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

        if let (true, Some(usage)) = (chunk.choices.is_empty(), &chunk.usage) {
            return Ok(serde_json::json!({
                "metadata": {
                    "usage": usage.to_provider_value(self.format())
                }
            }));
        }

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

                if delta.get("role").is_some() && delta.get("content").is_none() {
                    return Ok(serde_json::json!({
                        "messageStart": {
                            "role": "assistant"
                        }
                    }));
                }
            }
        }

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
        assert_eq!(
            universal.params.token_budget,
            Some(TokenBudget::OutputTokens(1024))
        );

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert_eq!(
            reconstructed.get("modelId").unwrap(),
            "anthropic.claude-3-sonnet"
        );
        assert!(reconstructed.get("messages").is_some());
        assert!(reconstructed.get("inferenceConfig").is_some());
    }

    #[test]
    fn test_bedrock_rejects_discovery_only_history_after_filtering() {
        use crate::universal::message::{AssistantContent, AssistantContentPart, ToolContentPart};
        use crate::universal::{ToolDiscoveryResultContentPart, ToolDiscoveryResultItem};

        let adapter = BedrockAdapter;
        let request = UniversalRequest {
            model: Some("anthropic.claude-3-sonnet".to_string()),
            messages: vec![
                Message::Assistant {
                    content: AssistantContent::Array(vec![
                        AssistantContentPart::ToolDiscoveryCall {
                            tool_call_id: "call_tool_search_123".to_string(),
                            discovery_tool_name: "tool_search".to_string(),
                            query: Some("search_code".to_string()),
                            arguments: None,
                            status: Some("completed".to_string()),
                            execution: Some("client".to_string()),
                            provider_options: None,
                        },
                    ]),
                    id: None,
                },
                Message::Tool {
                    content: vec![ToolContentPart::ToolDiscoveryResult(
                        ToolDiscoveryResultContentPart {
                            tool_call_id: "call_tool_search_123".to_string(),
                            discovery_tool_name: "tool_search".to_string(),
                            tools: vec![ToolDiscoveryResultItem {
                                tool_name: "search_code".to_string(),
                                tool: None,
                                provider_options: None,
                            }],
                            status: Some("completed".to_string()),
                            execution: Some("client".to_string()),
                            provider_options: None,
                        },
                    )],
                },
            ],
            params: UniversalParams::default(),
        };

        let err = adapter.request_from_universal(&request).unwrap_err();
        match err {
            TransformError::ValidationFailed {
                target: ProviderFormat::Converse,
                reason,
            } => {
                assert!(reason.contains("dynamic tool discovery history"));
            }
            other => panic!("expected Bedrock validation error, got {other:?}"),
        }
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
                token_budget: Some(TokenBudget::OutputTokens(4096)),
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
    fn test_bedrock_reasoning_omits_temperature() {
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
                temperature: Some(0.5), // This should be omitted when thinking is enabled
                token_budget: Some(TokenBudget::OutputTokens(4096)),
                ..Default::default()
            },
        };

        let reconstructed = adapter.request_from_universal(&universal).unwrap();

        // Temperature should be omitted when thinking is enabled (let Bedrock default to 1.0)
        let inference_config = reconstructed.get("inferenceConfig").unwrap();
        assert!(
            inference_config.get("temperature").is_none(),
            "Temperature should be omitted when thinking is enabled"
        );
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

        // Temperature should be omitted when thinking is enabled
        let inference_config = reconstructed.get("inferenceConfig").unwrap();
        assert!(
            inference_config.get("temperature").is_none(),
            "Temperature should be omitted when thinking is enabled"
        );
    }

    #[test]
    fn test_bedrock_stream_detects_content_block_start() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "contentBlockStart": {
                "contentBlockIndex": 0,
                "start": {
                    "toolUse": {
                        "toolUseId": "tooluse_123",
                        "name": "list_campaigns",
                        "type": "tool_use"
                    }
                }
            }
        });

        assert!(adapter.detect_stream_response(&payload));
    }

    #[test]
    fn test_bedrock_stream_tool_start_to_universal_tool_call_delta() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "contentBlockStart": {
                "contentBlockIndex": 0,
                "start": {
                    "toolUse": {
                        "toolUseId": "tooluse_123",
                        "name": "list_campaigns",
                        "type": "tool_use"
                    }
                }
            }
        });

        let chunk = adapter.stream_to_universal(payload).unwrap().unwrap();
        let choice = chunk.choices.first().unwrap();
        let delta = choice.delta_view().unwrap();
        let tool_call = delta.tool_calls.first().unwrap();

        assert_eq!(choice.index, 0);
        assert_eq!(delta.role.as_deref(), Some("assistant"));
        assert_eq!(tool_call.index, Some(0));
        assert_eq!(tool_call.id.as_deref(), Some("tooluse_123"));
        assert_eq!(tool_call.call_type.as_deref(), Some("function"));
        assert_eq!(
            tool_call.function.as_ref().unwrap().name.as_deref(),
            Some("list_campaigns")
        );
        assert_eq!(
            tool_call.function.as_ref().unwrap().arguments.as_deref(),
            Some("")
        );
    }

    #[test]
    fn test_bedrock_stream_tool_input_to_universal_argument_delta() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "contentBlockDelta": {
                "contentBlockIndex": 0,
                "delta": {
                    "toolUse": {
                        "input": "{\"campaign"
                    }
                }
            }
        });

        let chunk = adapter.stream_to_universal(payload).unwrap().unwrap();
        let choice = chunk.choices.first().unwrap();
        let delta = choice.delta_view().unwrap();
        let tool_call = delta.tool_calls.first().unwrap();

        assert_eq!(choice.index, 0);
        assert_eq!(tool_call.index, Some(0));
        assert_eq!(
            tool_call.function.as_ref().unwrap().arguments.as_deref(),
            Some("{\"campaign")
        );
    }

    #[test]
    fn test_bedrock_stream_reasoning_content_to_universal_reasoning_delta() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "contentBlockDelta": {
                "contentBlockIndex": 0,
                "delta": {
                    "reasoningContent": {
                        "text": "Thinking"
                    }
                }
            }
        });

        let chunk = adapter.stream_to_universal(payload).unwrap().unwrap();
        let choice = chunk.choices.first().unwrap();
        let delta = choice.delta_view().unwrap();

        assert_eq!(choice.index, 0);
        assert_eq!(delta.role.as_deref(), Some("assistant"));
        assert_eq!(delta.reasoning.len(), 1);
        assert_eq!(delta.reasoning[0].content.as_deref(), Some("Thinking"));
    }

    #[test]
    fn test_bedrock_stream_reasoning_signature_to_universal_signature_delta() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "contentBlockDelta": {
                "contentBlockIndex": 0,
                "delta": {
                    "reasoningContent": {
                        "signature": "sig_123"
                    }
                }
            }
        });

        let chunk = adapter.stream_to_universal(payload).unwrap().unwrap();
        let choice = chunk.choices.first().unwrap();
        let delta = choice.delta_view().unwrap();

        assert_eq!(choice.index, 0);
        assert_eq!(delta.reasoning_signature.as_deref(), Some("sig_123"));
    }

    #[test]
    fn test_bedrock_stream_text_block_uses_single_openai_choice_index() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "contentBlockDelta": {
                "contentBlockIndex": 1,
                "delta": {
                    "text": "Final answer"
                }
            }
        });

        let chunk = adapter.stream_to_universal(payload).unwrap().unwrap();
        let choice = chunk.choices.first().unwrap();
        let delta = choice.delta_view().unwrap();

        assert_eq!(choice.index, 0);
        assert_eq!(delta.content.as_deref(), Some("Final answer"));
    }

    #[test]
    fn test_bedrock_stream_tool_block_keeps_tool_call_index_separate_from_choice_index() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "contentBlockStart": {
                "contentBlockIndex": 2,
                "start": {
                    "toolUse": {
                        "toolUseId": "tooluse_456",
                        "name": "list_campaigns"
                    }
                }
            }
        });

        let chunk = adapter.stream_to_universal(payload).unwrap().unwrap();
        let choice = chunk.choices.first().unwrap();
        let delta = choice.delta_view().unwrap();
        let tool_call = delta.tool_calls.first().unwrap();

        assert_eq!(choice.index, 0);
        assert_eq!(tool_call.index, Some(2));
        assert_eq!(tool_call.id.as_deref(), Some("tooluse_456"));
    }

    #[test]
    fn test_bedrock_stream_tool_stop_to_tool_calls_finish_reason() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "messageStop": {
                "stopReason": "tool_use"
            }
        });

        let chunk = adapter.stream_to_universal(payload).unwrap().unwrap();
        let choice = chunk.choices.first().unwrap();

        assert_eq!(choice.finish_reason.as_deref(), Some("tool_calls"));
    }

    #[test]
    fn test_bedrock_stream_ignores_citation_delta() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "contentBlockDelta": {
                "contentBlockIndex": 0,
                "delta": {
                    "citation": {
                        "generatedResponsePart": {
                            "textResponsePart": {
                                "text": "Paris",
                                "span": { "start": 0, "end": 5 }
                            }
                        }
                    }
                }
            }
        });

        let chunk = adapter.stream_to_universal(payload).unwrap().unwrap();

        assert!(chunk.is_keep_alive());
    }

    #[test]
    fn test_bedrock_stream_ignores_image_delta() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "contentBlockDelta": {
                "contentBlockIndex": 0,
                "delta": {
                    "image": {
                        "bytes": "AAAA"
                    }
                }
            }
        });

        let chunk = adapter.stream_to_universal(payload).unwrap().unwrap();

        assert!(chunk.is_keep_alive());
    }

    #[test]
    fn test_bedrock_stream_ignores_tool_result_delta() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "contentBlockDelta": {
                "contentBlockIndex": 0,
                "delta": {
                    "toolResult": [{
                        "content": [{ "text": "done" }]
                    }]
                }
            }
        });

        let chunk = adapter.stream_to_universal(payload).unwrap().unwrap();

        assert!(chunk.is_keep_alive());
    }

    #[test]
    fn test_bedrock_stream_guardrail_stop_to_content_filter_finish_reason() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "messageStop": {
                "stopReason": "guardrail_intervened"
            }
        });

        let chunk = adapter.stream_to_universal(payload).unwrap().unwrap();
        let choice = chunk.choices.first().unwrap();

        assert_eq!(choice.finish_reason.as_deref(), Some("content_filter"));
    }

    #[test]
    fn test_bedrock_stream_context_window_stop_to_length_finish_reason() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "messageStop": {
                "stopReason": "model_context_window_exceeded"
            }
        });

        let chunk = adapter.stream_to_universal(payload).unwrap().unwrap();
        let choice = chunk.choices.first().unwrap();

        assert_eq!(choice.finish_reason.as_deref(), Some("length"));
    }

    #[test]
    fn test_bedrock_stream_unknown_stop_reason_to_stop_finish_reason() {
        let adapter = BedrockAdapter;
        let payload = json!({
            "messageStop": {
                "stopReason": "future_stop_reason"
            }
        });

        let chunk = adapter.stream_to_universal(payload).unwrap().unwrap();
        let choice = chunk.choices.first().unwrap();

        assert_eq!(choice.finish_reason.as_deref(), Some("stop"));
    }
}

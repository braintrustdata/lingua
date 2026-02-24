/*!
Anthropic provider adapter for Messages API.

Anthropic's Messages API has some unique requirements:
- `max_tokens` is required (we use a default of 4096)
- System messages use a separate `system` parameter, not in `messages` array
*/

use std::convert::TryFrom;

use crate::capabilities::ProviderFormat;
use crate::error::ConvertError;
use crate::processing::adapters::{
    insert_opt_bool, insert_opt_f64, insert_opt_i64, ProviderAdapter,
};
use crate::processing::transform::TransformError;
use crate::providers::anthropic::capabilities;
use crate::providers::anthropic::convert::system_to_user_content;
use crate::providers::anthropic::generated::{
    ContentBlock, EffortLevel, InputMessage, OutputConfig, Thinking, ThinkingType, Tool,
    ToolChoice, ToolChoiceType,
};
use crate::providers::anthropic::params::{AnthropicExtrasView, AnthropicParams};
use crate::providers::anthropic::try_parse_anthropic;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{Message, UserContent, UserContentPart};
use crate::universal::request::{
    ReasoningCanonical, ReasoningEffort, ResponseFormatConfig, ToolChoiceConfig,
};
use crate::universal::tools::UniversalTool;
use crate::universal::transform::extract_system_messages;
use crate::universal::{
    FinishReason, TokenBudget, UniversalParams, UniversalReasoningDelta, UniversalRequest,
    UniversalResponse, UniversalStreamChoice, UniversalStreamChunk, UniversalStreamDelta,
    UniversalToolCallDelta, UniversalToolFunctionDelta, UniversalUsage, PLACEHOLDER_ID,
    PLACEHOLDER_MODEL,
};
use serde::Deserialize;

/// Default max_tokens for Anthropic requests (matches legacy proxy behavior).
pub const DEFAULT_MAX_TOKENS: i64 = 4096;

#[derive(Debug, Default, Deserialize)]
struct AnthropicMetadataView {
    user_id: Option<String>,
}

fn parse_anthropic_extras(
    extras: Option<&Map<String, Value>>,
) -> Result<AnthropicExtrasView, TransformError> {
    extras
        .map(|m| serde_json::from_value(Value::Object(m.clone())))
        .transpose()
        .map_err(|e| {
            TransformError::FromUniversalFailed(format!("invalid Anthropic extras shape: {}", e))
        })
        .map(|v: Option<AnthropicExtrasView>| v.unwrap_or_default())
}

fn is_forced_tool_choice(value: &Value) -> bool {
    let parsed: Result<ToolChoice, _> = serde_json::from_value(value.clone());
    parsed.ok().is_some_and(|tool_choice| {
        tool_choice.tool_choice_type == ToolChoiceType::Tool
            && tool_choice
                .name
                .as_ref()
                .is_some_and(|name| !name.is_empty())
    })
}

fn is_enabled_thinking(value: &Value) -> bool {
    let parsed: Result<Thinking, _> = serde_json::from_value(value.clone());
    parsed
        .ok()
        .is_some_and(|thinking| thinking.thinking_type == ThinkingType::Enabled)
}
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
        let raw_payload_obj: Map<String, Value> = serde_json::from_value(payload.clone())
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Single parse: typed params now includes typed messages via #[serde(flatten)]
        let typed_params: AnthropicParams = serde_json::from_value(payload)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Extract typed messages (partial move - other fields remain accessible)
        let input_messages = typed_params.messages.ok_or_else(|| {
            TransformError::ToUniversalFailed("Anthropic: missing 'messages' field".to_string())
        })?;

        let mut messages = <Vec<Message> as TryFromLLM<Vec<_>>>::try_from(input_messages)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        if let Some(system) = typed_params.system.clone() {
            messages.insert(
                0,
                Message::System {
                    content: system_to_user_content(system),
                },
            );
        }

        let mut params = UniversalParams {
            temperature: typed_params.temperature,
            top_p: typed_params.top_p,
            top_k: typed_params.top_k,
            token_budget: typed_params.max_tokens.map(TokenBudget::OutputTokens),
            stop: typed_params.stop_sequences.clone(),
            tools: typed_params.tools.map(|tools| {
                <Vec<UniversalTool> as TryFromLLM<Vec<_>>>::try_from(tools).unwrap_or_default()
            }),
            tool_choice: typed_params
                .tool_choice
                .as_ref()
                .map(ToolChoiceConfig::from),
            response_format: typed_params
                .output_config
                .as_ref()
                .and_then(|oc| oc.format.as_ref())
                .or(typed_params.output_format.as_ref())
                .map(ResponseFormatConfig::from),
            seed: None,
            presence_penalty: None,
            frequency_penalty: None,
            stream: typed_params.stream,
            parallel_tool_calls: typed_params
                .tool_choice
                .as_ref()
                .and_then(|tc| tc.disable_parallel_tool_use)
                .map(|disabled| !disabled),
            reasoning: typed_params
                .output_config
                .as_ref()
                .and_then(|oc| oc.effort.as_ref())
                .map(|effort| {
                    use crate::providers::anthropic::generated::EffortLevel;
                    use crate::universal::request::{
                        ReasoningCanonical, ReasoningConfig, ReasoningEffort,
                    };
                    let effort_level = match effort {
                        EffortLevel::Low => ReasoningEffort::Low,
                        EffortLevel::Medium => ReasoningEffort::Medium,
                        EffortLevel::High => ReasoningEffort::High,
                        EffortLevel::Max => ReasoningEffort::High,
                    };
                    ReasoningConfig {
                        enabled: Some(true),
                        effort: Some(effort_level),
                        canonical: Some(ReasoningCanonical::Effort),
                        ..Default::default()
                    }
                })
                .or_else(|| {
                    typed_params
                        .thinking
                        .as_ref()
                        .map(crate::universal::request::ReasoningConfig::from)
                }),
            metadata: typed_params
                .metadata
                .as_ref()
                .and_then(|m| serde_json::to_value(m).ok()),
            store: None,
            service_tier: typed_params.service_tier,
            logprobs: None,
            top_logprobs: None,
            extras: Default::default(),
        };

        // Use extras captured automatically via #[serde(flatten)]
        if !typed_params.extras.is_empty() {
            params.extras.insert(
                ProviderFormat::Anthropic,
                typed_params.extras.into_iter().collect(),
            );
        }

        let anthropic_extras = params.extras.entry(ProviderFormat::Anthropic).or_default();
        for (key, value) in raw_payload_obj {
            anthropic_extras.insert(key, value);
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

        let anthropic_extras = req.params.extras.get(&ProviderFormat::Anthropic);
        let anthropic_extras_view = parse_anthropic_extras(anthropic_extras)?;

        // Clone messages and extract system messages (Anthropic uses separate `system` param)
        let mut msgs = req.messages.clone();
        let system_contents = extract_system_messages(&mut msgs);

        let mut obj = Map::new();
        obj.insert("model".into(), Value::String(model.clone()));

        if let Some(raw_messages) = anthropic_extras_view.messages.as_ref() {
            obj.insert("messages".into(), raw_messages.clone());
        } else {
            // Convert remaining messages
            let anthropic_messages: Vec<InputMessage> =
                <Vec<InputMessage> as TryFromLLM<Vec<Message>>>::try_from(msgs)
                    .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;
            obj.insert(
                "messages".into(),
                serde_json::to_value(anthropic_messages)
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
            );
        }

        // Add system message if present
        if let Some(raw_system) = anthropic_extras_view.system.as_ref() {
            obj.insert("system".into(), raw_system.clone());
        } else if !system_contents.is_empty() {
            let system_text: String = system_contents
                .into_iter()
                .map(|c| match c {
                    UserContent::String(s) => s,
                    UserContent::Array(parts) => parts
                        .into_iter()
                        .filter_map(|p| {
                            if let UserContentPart::Text(t) = p {
                                Some(t.text)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            obj.insert("system".into(), Value::String(system_text));
        }

        // max_tokens is required for Anthropic - use the value from params or default
        let max_tokens = req
            .params
            .output_token_budget()
            .unwrap_or(DEFAULT_MAX_TOKENS);
        obj.insert("max_tokens".into(), Value::Number(max_tokens.into()));

        // Determine reasoning style based on model capability AND canonical source:
        // - Opus 4.5+ with effort canonical → output_config.effort (new API)
        // - All other cases → thinking object (legacy, broad model support)
        // Both branches use output_config.format for structured output (never output_format).
        let use_effort = capabilities::supports_output_config_effort(model)
            && req
                .params
                .reasoning
                .as_ref()
                .is_some_and(|r| r.canonical == Some(ReasoningCanonical::Effort));

        let thinking_val = if use_effort {
            None
        } else {
            req.params.reasoning_for(ProviderFormat::Anthropic)
        };

        let reasoning_enabled = use_effort
            || thinking_val
                .as_ref()
                .and_then(|v| v.get("type"))
                .and_then(|t| t.as_str())
                .is_some_and(|t| t == "enabled");
        if let Some(raw_temp) = anthropic_extras_view.temperature.as_ref() {
            obj.insert("temperature".into(), raw_temp.clone());
        } else if !reasoning_enabled {
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
        if let Some(raw_tools) = anthropic_extras_view.tools.as_ref() {
            obj.insert("tools".into(), raw_tools.clone());
        } else if let Some(tools) = &req.params.tools {
            if !tools.is_empty() {
                let anthropic_tools: Vec<Tool> = tools
                    .iter()
                    .map(Tool::try_from)
                    .collect::<Result<Vec<_>, _>>()?;
                obj.insert(
                    "tools".into(),
                    serde_json::to_value(&anthropic_tools)
                        .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
                );
            }
        }

        // Convert tool_choice using helper method (handles parallel_tool_calls internally)
        let tool_choice_value =
            if let Some(raw_tool_choice) = anthropic_extras_view.tool_choice.as_ref() {
                Some(raw_tool_choice.clone())
            } else {
                req.params.tool_choice_for(ProviderFormat::Anthropic)
            };
        let forced_tool_choice = tool_choice_value
            .as_ref()
            .is_some_and(is_forced_tool_choice);
        if let Some(tool_choice_val) = tool_choice_value {
            obj.insert("tool_choice".into(), tool_choice_val);
        }
        insert_opt_bool(&mut obj, "stream", req.params.stream);

        // Build output_config (always used for structured output format, and for effort on Opus 4.5+)
        let effort_level = if use_effort {
            req.params
                .reasoning
                .as_ref()
                .and_then(|r| r.effort)
                .map(|e| match e {
                    ReasoningEffort::Low => EffortLevel::Low,
                    ReasoningEffort::Medium => EffortLevel::Medium,
                    ReasoningEffort::High => EffortLevel::High,
                })
        } else {
            None
        };
        let format = req
            .params
            .response_format
            .as_ref()
            .and_then(|rf| rf.try_into().ok());

        let raw_output_config = anthropic_extras_view.output_config.as_ref();
        let raw_thinking = anthropic_extras_view.thinking.as_ref();

        if let Some(raw_output_config) = raw_output_config {
            obj.insert("output_config".into(), raw_output_config.clone());
        } else if effort_level.is_some() || format.is_some() {
            let output_config = OutputConfig {
                effort: effort_level,
                format,
            };
            obj.insert(
                "output_config".into(),
                serde_json::to_value(&output_config)
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
            );
        }

        // Add thinking for legacy reasoning (non-Opus models)
        if let Some(raw_thinking) = raw_thinking {
            if !(forced_tool_choice && is_enabled_thinking(raw_thinking)) {
                obj.insert("thinking".into(), raw_thinking.clone());
            }
        } else if raw_output_config.is_none() {
            if let Some(thinking) = thinking_val {
                if !(forced_tool_choice && is_enabled_thinking(&thinking)) {
                    obj.insert("thinking".into(), thinking);
                }
            }
        }

        // Add metadata from canonical params
        if let Some(raw_metadata) = anthropic_extras_view.metadata.as_ref() {
            obj.insert("metadata".into(), raw_metadata.clone());
        } else if let Some(metadata) = req.params.metadata.as_ref() {
            // Anthropic metadata only supports `user_id`.
            let metadata_view: AnthropicMetadataView =
                serde_json::from_value(metadata.clone()).unwrap_or_default();
            if let Some(user_id) = metadata_view.user_id {
                let mut anthropic_metadata = Map::new();
                anthropic_metadata.insert("user_id".into(), Value::String(user_id));
                obj.insert("metadata".into(), Value::Object(anthropic_metadata));
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

        // Merge back provider-specific extras (only for Anthropic)
        if let Some(extras) = req.params.extras.get(&ProviderFormat::Anthropic) {
            for (k, v) in extras {
                if k == "output_format" {
                    continue;
                }
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
        if req.params.output_token_budget().is_none() {
            req.params.token_budget = Some(TokenBudget::OutputTokens(DEFAULT_MAX_TOKENS));
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
                let delta = payload.get("delta");
                let delta_type = delta.and_then(|d| d.get("type")).and_then(Value::as_str);

                if delta_type == Some("text_delta") {
                    let text = delta.and_then(|d| d.get("text")).and_then(Value::as_str);

                    let delta = UniversalStreamDelta {
                        role: Some("assistant".to_string()),
                        content: match text {
                            Some(t) if !t.is_empty() => Some(t.to_string()),
                            _ => None,
                        },
                        ..Default::default()
                    };

                    let index = payload_index(&payload);

                    return Ok(Some(UniversalStreamChunk::new(
                        None,
                        None,
                        vec![UniversalStreamChoice {
                            index,
                            delta: Some(Value::from(delta)),
                            finish_reason: None,
                        }],
                        None,
                        None,
                    )));
                }

                if delta_type == Some("input_json_delta") {
                    let index = payload_index(&payload);
                    let partial_json = parse_content_block_delta_fields(delta)
                        .partial_json
                        .unwrap_or_default();

                    return Ok(Some(UniversalStreamChunk::new(
                        None,
                        None,
                        vec![UniversalStreamChoice {
                            index,
                            delta: Some(serde_json::json!({
                                "tool_calls": [{
                                    "index": index,
                                    "function": {
                                        "arguments": partial_json
                                    }
                                }]
                            })),
                            finish_reason: None,
                        }],
                        None,
                        None,
                    )));
                }

                if delta_type == Some("thinking_delta") {
                    let index = payload_index(&payload);
                    let thinking = parse_content_block_delta_fields(delta)
                        .thinking
                        .unwrap_or_default();
                    if thinking.is_empty() {
                        return Ok(Some(UniversalStreamChunk::keep_alive()));
                    }
                    let delta = UniversalStreamDelta {
                        role: Some("assistant".to_string()),
                        reasoning: vec![UniversalReasoningDelta {
                            content: Some(thinking),
                        }],
                        ..Default::default()
                    };

                    return Ok(Some(UniversalStreamChunk::new(
                        None,
                        None,
                        vec![UniversalStreamChoice {
                            index,
                            delta: Some(Value::from(delta)),
                            finish_reason: None,
                        }],
                        None,
                        None,
                    )));
                }

                if delta_type == Some("signature_delta") {
                    let index = payload_index(&payload);
                    let signature = parse_content_block_delta_fields(delta)
                        .signature
                        .unwrap_or_default();
                    if signature.is_empty() {
                        return Ok(Some(UniversalStreamChunk::keep_alive()));
                    }
                    let delta = UniversalStreamDelta {
                        role: Some("assistant".to_string()),
                        reasoning_signature: Some(signature.to_string()),
                        ..Default::default()
                    };

                    return Ok(Some(UniversalStreamChunk::new(
                        None,
                        None,
                        vec![UniversalStreamChoice {
                            index,
                            delta: Some(Value::from(delta)),
                            finish_reason: None,
                        }],
                        None,
                        None,
                    )));
                }

                // For unsupported non-text deltas, return keep-alive
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
                let tool_call_delta = message_start_tool_use_part(&payload)
                    .map(|part| {
                        let arguments = part
                            .input
                            .as_ref()
                            .map(tool_input_to_arguments)
                            .unwrap_or_default();
                        Value::from(UniversalStreamDelta {
                            role: Some("assistant".to_string()),
                            tool_calls: vec![UniversalToolCallDelta {
                                index: Some(0),
                                id: part.id,
                                call_type: Some("function".to_string()),
                                function: Some(UniversalToolFunctionDelta {
                                    name: part.name,
                                    arguments: Some(arguments),
                                }),
                            }],
                            ..Default::default()
                        })
                    })
                    .unwrap_or_else(|| {
                        Value::from(UniversalStreamDelta {
                            role: Some("assistant".to_string()),
                            content: Some(String::new()),
                            ..Default::default()
                        })
                    });

                // Return chunk with metadata but mark as role initialization
                Ok(Some(UniversalStreamChunk::new(
                    id,
                    model,
                    vec![UniversalStreamChoice {
                        index: 0,
                        delta: Some(tool_call_delta),
                        finish_reason: None,
                    }],
                    None,
                    usage,
                )))
            }

            "message_stop" => Ok(None),

            "content_block_start" => {
                let content_block_view = parse_content_block_start_event(&payload);
                let block_type = content_block_view
                    .content_block
                    .as_ref()
                    .and_then(|b| b.block_type.as_deref());

                if block_type == Some("tool_use") {
                    let id = content_block_view
                        .content_block
                        .as_ref()
                        .and_then(|b| b.id.as_deref())
                        .unwrap_or("");
                    let name = content_block_view
                        .content_block
                        .as_ref()
                        .and_then(|b| b.name.as_deref())
                        .unwrap_or("");
                    let block_index = content_block_view.index.unwrap_or(0);

                    return Ok(Some(UniversalStreamChunk::new(
                        None,
                        None,
                        vec![UniversalStreamChoice {
                            index: block_index,
                            delta: Some(Value::from(UniversalStreamDelta {
                                role: Some("assistant".to_string()),
                                tool_calls: vec![UniversalToolCallDelta {
                                    index: Some(block_index),
                                    id: Some(id.to_string()),
                                    call_type: Some("function".to_string()),
                                    function: Some(UniversalToolFunctionDelta {
                                        name: Some(name.to_string()),
                                        arguments: Some(String::new()),
                                    }),
                                }],
                                ..Default::default()
                            })),
                            finish_reason: None,
                        }],
                        None,
                        None,
                    )));
                }

                if block_type == Some("thinking") {
                    let block_index = content_block_view.index.unwrap_or(0);
                    let thinking = content_block_view
                        .content_block
                        .and_then(|b| b.thinking)
                        .unwrap_or_default();
                    if thinking.is_empty() {
                        return Ok(Some(UniversalStreamChunk::keep_alive()));
                    }
                    let delta = UniversalStreamDelta {
                        role: Some("assistant".to_string()),
                        reasoning: vec![UniversalReasoningDelta {
                            content: Some(thinking.to_string()),
                        }],
                        ..Default::default()
                    };

                    return Ok(Some(UniversalStreamChunk::new(
                        None,
                        None,
                        vec![UniversalStreamChoice {
                            index: block_index,
                            delta: Some(Value::from(delta)),
                            finish_reason: None,
                        }],
                        None,
                        None,
                    )));
                }
                Ok(Some(UniversalStreamChunk::keep_alive()))
            }

            "content_block_stop" | "ping" => Ok(Some(UniversalStreamChunk::keep_alive())),

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

        // Check if delta has tool_calls
        let has_tool_calls = chunk
            .choices
            .first()
            .and_then(|c| c.delta.as_ref())
            .and_then(|d| d.get("tool_calls"))
            .and_then(Value::as_array)
            .is_some_and(|arr| !arr.is_empty());

        // Check if this is an initial metadata chunk (has model/id/usage but no content).
        // Exclude chunks with tool_calls - those must be handled by the tool call path.
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
            if let Some(delta_view) = choice.delta_view() {
                if !delta_view.reasoning.is_empty() {
                    let thinking = delta_view
                        .reasoning
                        .iter()
                        .filter_map(|r| r.content.as_deref())
                        .collect::<String>();
                    if !thinking.is_empty() {
                        return Ok(serde_json::json!({
                            "type": "content_block_delta",
                            "index": choice.index,
                            "delta": {
                                "type": "thinking_delta",
                                "thinking": thinking
                            }
                        }));
                    }
                }

                if let Some(signature) = delta_view.reasoning_signature {
                    if !signature.is_empty() {
                        return Ok(serde_json::json!({
                            "type": "content_block_delta",
                            "index": choice.index,
                            "delta": {
                                "type": "signature_delta",
                                "signature": signature
                            }
                        }));
                    }
                }
            }

            if let Some(delta_view) = choice.delta_view() {
                if let Some(tool_call) = delta_view.tool_calls.first() {
                    let tool_index = tool_call.index.unwrap_or(choice.index);
                    let function = tool_call.function.clone().unwrap_or_default();
                    let tool_name = function.name.unwrap_or_default();
                    let tool_id = tool_call.id.clone().unwrap_or_default();
                    let arguments = function.arguments.unwrap_or_default();

                    if !tool_name.is_empty() || !tool_id.is_empty() {
                        let input = serde_json::from_str::<Value>(&arguments)
                            .ok()
                            .filter(Value::is_object)
                            .unwrap_or_else(|| serde_json::json!({}));
                        return Ok(serde_json::json!({
                            "type": "content_block_start",
                            "index": tool_index,
                            "content_block": {
                                "type": "tool_use",
                                "id": tool_id,
                                "name": tool_name,
                                "input": input
                            }
                        }));
                    }

                    return Ok(serde_json::json!({
                        "type": "content_block_delta",
                        "index": tool_index,
                        "delta": {
                            "type": "input_json_delta",
                            "partial_json": arguments
                        }
                    }));
                }
                if let Some(content) = delta_view.content.as_deref() {
                    return Ok(serde_json::json!({
                            "type": "content_block_delta",
                            "index": choice.index,
                        "delta": {
                            "type": "text_delta",
                            "text": content
                        }
                    }));
                }

                // Role-only delta or null content without tool_calls - return empty text_delta
                let content_is_missing_or_null = delta_view.content.is_none();
                let has_tool_calls = !delta_view.tool_calls.is_empty();

                if delta_view.role.is_some() && content_is_missing_or_null && !has_tool_calls {
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

fn tool_input_to_arguments(input: &Value) -> String {
    let parsed_object =
        serde_json::from_value::<serde_json::Map<String, Value>>(input.clone()).ok();
    if parsed_object.as_ref().is_some_and(|obj| obj.is_empty()) {
        return String::new();
    }
    serde_json::to_string(input).unwrap_or_else(|_| "{}".to_string())
}

#[derive(Debug, Deserialize, Default)]
struct IndexField {
    index: Option<u32>,
}

fn payload_index(payload: &Value) -> u32 {
    serde_json::from_value::<IndexField>(payload.clone())
        .ok()
        .and_then(|p| p.index)
        .unwrap_or(0)
}

#[derive(Debug, Deserialize, Default)]
struct ContentBlockDeltaFields {
    partial_json: Option<String>,
    thinking: Option<String>,
    signature: Option<String>,
}

fn parse_content_block_delta_fields(delta: Option<&Value>) -> ContentBlockDeltaFields {
    delta
        .cloned()
        .and_then(|d| serde_json::from_value::<ContentBlockDeltaFields>(d).ok())
        .unwrap_or_default()
}

#[derive(Debug, Deserialize, Default)]
struct MessageStartToolUseEvent {
    message: Option<MessageStartToolUseMessage>,
}

#[derive(Debug, Deserialize, Default)]
struct MessageStartToolUseMessage {
    #[serde(default)]
    content: Vec<MessageStartToolUsePart>,
}

#[derive(Debug, Deserialize, Default)]
struct MessageStartToolUsePart {
    #[serde(rename = "type")]
    part_type: Option<String>,
    id: Option<String>,
    name: Option<String>,
    input: Option<Value>,
}

fn message_start_tool_use_part(payload: &Value) -> Option<MessageStartToolUsePart> {
    let parsed = serde_json::from_value::<MessageStartToolUseEvent>(payload.clone()).ok()?;
    parsed
        .message?
        .content
        .into_iter()
        .find(|part| part.part_type.as_deref() == Some("tool_use"))
}

#[derive(Debug, Deserialize, Default)]
struct ContentBlockStartEventView {
    index: Option<u32>,
    content_block: Option<ContentBlockView>,
}

#[derive(Debug, Deserialize, Default)]
struct ContentBlockView {
    #[serde(rename = "type")]
    block_type: Option<String>,
    id: Option<String>,
    name: Option<String>,
    thinking: Option<String>,
}

fn parse_content_block_start_event(payload: &Value) -> ContentBlockStartEventView {
    serde_json::from_value::<ContentBlockStartEventView>(payload.clone()).unwrap_or_default()
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
        assert_eq!(
            universal.params.token_budget,
            Some(TokenBudget::OutputTokens(1024))
        );

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

        assert!(req.params.token_budget.is_none());
        adapter.apply_defaults(&mut req);
        assert_eq!(
            req.params.token_budget,
            Some(TokenBudget::OutputTokens(DEFAULT_MAX_TOKENS))
        );
    }

    #[test]
    fn test_anthropic_preserves_existing_max_tokens() {
        let adapter = AnthropicAdapter;
        let mut req = UniversalRequest {
            model: Some("claude-3-5-sonnet-20241022".to_string()),
            messages: vec![],
            params: UniversalParams {
                token_budget: Some(TokenBudget::OutputTokens(8192)),
                ..Default::default()
            },
        };

        adapter.apply_defaults(&mut req);
        assert_eq!(
            req.params.token_budget,
            Some(TokenBudget::OutputTokens(8192))
        );
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
                token_budget: Some(TokenBudget::OutputTokens(4096)),
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
                token_budget: Some(TokenBudget::OutputTokens(1024)),
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
    fn test_anthropic_omits_enabled_thinking_with_forced_tool_choice() {
        use crate::universal::message::UserContent;
        use crate::universal::request::{ReasoningConfig, ToolChoiceMode};

        let adapter = AnthropicAdapter;

        let req = UniversalRequest {
            model: Some("claude-sonnet-4-5-20250929".to_string()),
            messages: vec![Message::User {
                content: UserContent::String("Tokyo weather".to_string()),
            }],
            params: UniversalParams {
                token_budget: Some(TokenBudget::OutputTokens(4096)),
                reasoning: Some(ReasoningConfig {
                    enabled: Some(true),
                    budget_tokens: Some(2048),
                    ..Default::default()
                }),
                tool_choice: Some(ToolChoiceConfig {
                    mode: Some(ToolChoiceMode::Tool),
                    tool_name: Some("get_weather".to_string()),
                    disable_parallel: None,
                }),
                ..Default::default()
            },
        };

        let result = adapter.request_from_universal(&req).unwrap();

        assert!(result.get("tool_choice").is_some());
        assert!(
            result.get("thinking").is_none(),
            "Enabled thinking should be omitted when tool_choice forces tool use"
        );
    }

    #[test]
    fn test_anthropic_preserves_enabled_thinking_with_auto_tool_choice() {
        use crate::universal::message::UserContent;
        use crate::universal::request::{ReasoningConfig, ToolChoiceMode};

        let adapter = AnthropicAdapter;

        let req = UniversalRequest {
            model: Some("claude-sonnet-4-5-20250929".to_string()),
            messages: vec![Message::User {
                content: UserContent::String("Tokyo weather".to_string()),
            }],
            params: UniversalParams {
                token_budget: Some(TokenBudget::OutputTokens(4096)),
                reasoning: Some(ReasoningConfig {
                    enabled: Some(true),
                    budget_tokens: Some(2048),
                    ..Default::default()
                }),
                tool_choice: Some(ToolChoiceConfig {
                    mode: Some(ToolChoiceMode::Auto),
                    tool_name: None,
                    disable_parallel: None,
                }),
                ..Default::default()
            },
        };

        let result = adapter.request_from_universal(&req).unwrap();

        assert!(result.get("tool_choice").is_some());
        assert!(
            result.get("thinking").is_some(),
            "Enabled thinking should be preserved when tool_choice is not forced"
        );
    }

    #[test]
    fn test_anthropic_preserves_disabled_thinking_with_forced_tool_choice() {
        use crate::universal::message::UserContent;
        use crate::universal::request::{ReasoningConfig, ToolChoiceMode};

        let adapter = AnthropicAdapter;

        let req = UniversalRequest {
            model: Some("claude-sonnet-4-5-20250929".to_string()),
            messages: vec![Message::User {
                content: UserContent::String("Tokyo weather".to_string()),
            }],
            params: UniversalParams {
                token_budget: Some(TokenBudget::OutputTokens(4096)),
                reasoning: Some(ReasoningConfig {
                    enabled: Some(false),
                    ..Default::default()
                }),
                tool_choice: Some(ToolChoiceConfig {
                    mode: Some(ToolChoiceMode::Tool),
                    tool_name: Some("get_weather".to_string()),
                    disable_parallel: None,
                }),
                ..Default::default()
            },
        };

        let result = adapter.request_from_universal(&req).unwrap();

        assert!(result.get("tool_choice").is_some());
        assert_eq!(
            result
                .get("thinking")
                .and_then(|thinking| thinking.get("type"))
                .and_then(Value::as_str),
            Some("disabled"),
            "Disabled thinking should be preserved with forced tool_choice"
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

        let reconstructed = adapter.request_from_universal(&universal).unwrap();

        assert!(
            reconstructed.get("output_config").is_some(),
            "output_config should be present in reconstructed request"
        );
        let output_config = reconstructed.get("output_config").unwrap();
        let format = output_config.get("format").unwrap();
        assert_eq!(format.get("type").unwrap(), "json_schema");
        assert!(format.get("schema").is_some());
        assert!(
            reconstructed.get("output_format").is_none(),
            "legacy output_format should not be present"
        );
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

        // Verify Anthropic output_config.format structure (GA API)
        let output_config = anthropic_request.get("output_config").unwrap();
        let format = output_config.get("format").unwrap();
        assert_eq!(format.get("type").unwrap(), "json_schema");
        assert!(format.get("schema").is_some());
        // Name should NOT be included (Anthropic doesn't support it)
        assert!(format.get("name").is_none());
        // strict is NOT supported in Anthropic (it's for tools only)
        assert!(format.get("strict").is_none());
        // Anthropic format doesn't have nested json_schema wrapper
        assert!(format.get("json_schema").is_none());
        // Legacy output_format should NOT be present
        assert!(anthropic_request.get("output_format").is_none());
    }

    #[test]
    fn test_stream_to_universal_thinking_delta_semantic_chunk() {
        let adapter = AnthropicAdapter;
        let payload = json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "thinking_delta",
                "thinking": "chain of thought fragment"
            }
        });

        let chunk = adapter
            .stream_to_universal(payload)
            .expect("stream_to_universal should succeed")
            .expect("thinking_delta should emit a chunk");

        assert!(!chunk.is_keep_alive());
        let choice = chunk.choices.first().expect("choice must exist");
        let delta = choice.delta_view().expect("delta must exist");
        let first = delta.reasoning.first().expect("reasoning item must exist");
        assert_eq!(first.content.as_deref(), Some("chain of thought fragment"),);
    }

    #[test]
    fn test_stream_to_universal_signature_delta_semantic_chunk() {
        let adapter = AnthropicAdapter;
        let payload = json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "signature_delta",
                "signature": "sig_abc123"
            }
        });

        let chunk = adapter
            .stream_to_universal(payload)
            .expect("stream_to_universal should succeed")
            .expect("signature_delta should emit a chunk");

        assert!(!chunk.is_keep_alive());
        let choice = chunk.choices.first().expect("choice must exist");
        let delta = choice.delta_view().expect("delta must exist");
        assert_eq!(delta.reasoning_signature.as_deref(), Some("sig_abc123"));
    }

    #[test]
    fn test_stream_to_universal_content_block_start_thinking_semantic_chunk() {
        let adapter = AnthropicAdapter;
        let payload = json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {
                "type": "thinking",
                "thinking": "initial thought"
            }
        });

        let chunk = adapter
            .stream_to_universal(payload)
            .expect("stream_to_universal should succeed")
            .expect("thinking block start should emit a chunk");

        assert!(!chunk.is_keep_alive());
        let choice = chunk.choices.first().expect("choice must exist");
        let delta = choice.delta_view().expect("delta must exist");
        let first = delta.reasoning.first().expect("reasoning item must exist");
        assert_eq!(first.content.as_deref(), Some("initial thought"),);
    }

    #[test]
    fn test_stream_to_universal_message_stop_returns_none() {
        let adapter = AnthropicAdapter;
        let payload = json!({
            "type": "message_stop"
        });

        let result = adapter
            .stream_to_universal(payload)
            .expect("stream_to_universal should succeed");

        assert!(
            result.is_none(),
            "message_stop should return None (terminal event)"
        );
    }
}

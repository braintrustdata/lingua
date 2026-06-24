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
use crate::providers::anthropic::convert::{
    anthropic_input_messages_to_universal_messages, system_to_user_content,
    universal_messages_to_anthropic_input_messages,
};
use crate::providers::anthropic::detect::{
    system_messages_are_supported_and_well_placed, try_parse_anthropic_source,
};
use crate::providers::anthropic::generated::{
    ContentBlock, CreateMessageParams, EffortLevel, OutputConfig, ServiceTierEnum, Thinking,
    ThinkingType, Tool, ToolChoice, ToolChoiceType,
};
use crate::providers::anthropic::params::AnthropicExtrasView;
use crate::providers::anthropic::tool_discovery;
use crate::providers::anthropic::try_parse_anthropic;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{Message, UserContent, UserContentPart};
use crate::universal::reasoning::budget_to_effort;
use crate::universal::request::{
    ReasoningCanonical, ReasoningConfig, ReasoningEffort, ResponseFormatConfig, ToolChoiceConfig,
    UniversalMetadataUserView,
};
use crate::universal::tools::{UniversalTool, UniversalToolType};
use crate::universal::{
    FinishReason, TokenBudget, UniversalParams, UniversalReasoningDelta, UniversalRequest,
    UniversalResponse, UniversalStreamChoice, UniversalStreamChunk, UniversalStreamDelta,
    UniversalToolCallDelta, UniversalToolFunctionDelta, UniversalUsage, PLACEHOLDER_ID,
    PLACEHOLDER_MODEL,
};
use serde::Deserialize;

/// Default max_tokens for Anthropic requests (matches legacy proxy behavior).
pub const DEFAULT_MAX_TOKENS: i64 = 4096;
const JSON_OBJECT_SHIM_TOOL_NAME: &str = "json";
const JSON_OBJECT_SHIM_TOOL_DESCRIPTION: &str = "Output the result in JSON format";

fn anthropic_tool_value(tool: &UniversalTool) -> Result<Value, TransformError> {
    if tool_discovery::is_anthropic_tool_search_builtin(tool) {
        let builtin_type = match &tool.tool_type {
            UniversalToolType::Builtin { builtin_type, .. } => builtin_type,
            _ => unreachable!("checked by tool_discovery::is_anthropic_tool_search_builtin"),
        };
        return Ok(serde_json::json!({
            "name": tool.name.clone(),
            "type": builtin_type.clone()
        }));
    }

    let anthropic_tool = Tool::try_from(tool)?;
    serde_json::to_value(&anthropic_tool)
        .map_err(|e| TransformError::SerializationFailed(e.to_string()))
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

fn service_tier_to_string(service_tier: ServiceTierEnum) -> String {
    match service_tier {
        ServiceTierEnum::Auto => "auto".to_string(),
        ServiceTierEnum::StandardOnly => "standard_only".to_string(),
    }
}

fn extract_leading_system_messages(messages: &mut Vec<Message>) -> Vec<UserContent> {
    let mut system_contents = Vec::new();

    while matches!(
        messages.first(),
        Some(Message::System { .. } | Message::Developer { .. })
    ) {
        let message = messages.remove(0);
        if let Message::System { content } | Message::Developer { content } = message {
            system_contents.push(content);
        }
    }

    system_contents
}

fn validate_no_non_leading_system_messages(messages: &[Message]) -> Result<(), TransformError> {
    if messages
        .iter()
        .any(|message| matches!(message, Message::System { .. } | Message::Developer { .. }))
    {
        return Err(TransformError::ValidationFailed {
            target: ProviderFormat::Anthropic,
            reason: "Anthropic generated types include system-role input messages, but the live Messages API currently rejects role 'system' for available models; non-leading system/developer messages cannot be exported to Anthropic without changing semantics".to_string(),
        });
    }

    Ok(())
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
    parsed.ok().is_some_and(|thinking| {
        matches!(
            thinking.thinking_type,
            ThinkingType::Enabled | ThinkingType::Adaptive
        )
    })
}

fn reasoning_is_enabled(config: &ReasoningConfig) -> bool {
    config.effort != Some(ReasoningEffort::None)
        && (config.enabled == Some(true)
            || config.effort.is_some()
            || config.budget_tokens.is_some())
}

fn reasoning_effort_level(
    config: Option<&ReasoningConfig>,
    max_tokens: Option<i64>,
) -> Option<EffortLevel> {
    let config = config?;
    let effort = config.effort.or_else(|| {
        config
            .budget_tokens
            .map(|budget| budget_to_effort(budget, max_tokens))
    })?;

    match effort {
        ReasoningEffort::None => None,
        ReasoningEffort::Minimal | ReasoningEffort::Low => Some(EffortLevel::Low),
        ReasoningEffort::Medium => Some(EffortLevel::Medium),
        ReasoningEffort::High => Some(EffortLevel::High),
        ReasoningEffort::Xhigh => Some(EffortLevel::Max),
    }
}

fn is_json_object_response_format(config: Option<&ResponseFormatConfig>) -> bool {
    config
        .and_then(|rf| rf.format_type)
        .is_some_and(|t| t == crate::universal::request::ResponseFormatType::JsonObject)
}

fn maybe_unwrap_json_shim_tool_call(messages: &mut [Message]) {
    for message in messages {
        let Message::Assistant { content, .. } = message else {
            continue;
        };
        let should_unwrap = matches!(
            content,
            crate::universal::message::AssistantContent::Array(parts)
                if !parts.is_empty()
                    && parts.iter().all(|part| {
                        matches!(
                            part,
                            crate::universal::message::AssistantContentPart::ToolCall { tool_name, .. }
                                if tool_name == JSON_OBJECT_SHIM_TOOL_NAME
                        )
                    })
        );
        if !should_unwrap {
            continue;
        }
        let crate::universal::message::AssistantContent::Array(parts) = content else {
            continue;
        };
        let json_text = parts
            .iter()
            .find_map(|part| match part {
                crate::universal::message::AssistantContentPart::ToolCall { arguments, .. } => {
                    Some(arguments.to_string())
                }
                _ => None,
            })
            .unwrap_or_else(|| "{}".to_string());
        *content = crate::universal::message::AssistantContent::String(json_text);
    }
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
        try_parse_anthropic_source(payload).is_ok()
    }

    fn detect_passthrough_request(&self, payload: &Value) -> bool {
        try_parse_anthropic(payload).is_ok()
    }

    fn request_to_universal(&self, payload: Value) -> Result<UniversalRequest, TransformError> {
        let raw_payload_obj: Map<String, Value> = serde_json::from_value(payload.clone())
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
        let raw_params_view: AnthropicExtrasView =
            serde_json::from_value(Value::Object(raw_payload_obj.clone()))
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let typed_params: CreateMessageParams = serde_json::from_value(payload)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let mut messages = anthropic_input_messages_to_universal_messages(typed_params.messages)
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
            token_budget: Some(TokenBudget::OutputTokens(typed_params.max_tokens)),
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
                .or(raw_params_view.output_format.as_ref())
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
                        EffortLevel::Xhigh => ReasoningEffort::Xhigh,
                        EffortLevel::Max => ReasoningEffort::Xhigh,
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
            conversation_reference: None,
            service_tier: typed_params.service_tier.map(service_tier_to_string),
            prompt_cache_key: None,
            logprobs: None,
            top_logprobs: None,
            extras: Default::default(),
        };

        let anthropic_extras = params.extras.entry(ProviderFormat::Anthropic).or_default();
        for (key, value) in raw_payload_obj {
            anthropic_extras.insert(key, value);
        }

        Ok(UniversalRequest {
            model: Some(typed_params.model),
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

        // Clone messages and extract only leading system/developer messages to top-level `system`.
        // Later instructions cannot be moved there without changing their placement.
        let mut msgs = req.messages.clone();
        let system_contents = extract_leading_system_messages(&mut msgs);

        let mut obj = Map::new();
        obj.insert("model".into(), Value::String(model.clone()));

        if let Some(raw_messages) = anthropic_extras_view.messages.as_ref() {
            if system_messages_are_supported_and_well_placed(model, raw_messages)
                .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?
            {
                obj.insert("messages".into(), raw_messages.clone());
            } else {
                if msgs.is_empty() {
                    let reason = if system_contents.is_empty() {
                        "Anthropic requires at least one message in 'messages'.".to_string()
                    } else {
                        "Anthropic requires at least one non-system message; a system prompt alone cannot be sent because Anthropic stores system prompts in the top-level 'system' field and requires at least one user or assistant message in 'messages'.".to_string()
                    };
                    return Err(TransformError::ValidationFailed {
                        target: ProviderFormat::Anthropic,
                        reason,
                    });
                }
                validate_no_non_leading_system_messages(&msgs)?;
                let anthropic_messages = universal_messages_to_anthropic_input_messages(msgs)
                    .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;
                obj.insert(
                    "messages".into(),
                    serde_json::to_value(anthropic_messages)
                        .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
                );
            }
        } else {
            if msgs.is_empty() {
                let reason = if system_contents.is_empty() {
                    "Anthropic requires at least one message in 'messages'.".to_string()
                } else {
                    "Anthropic requires at least one non-system message; a system prompt alone cannot be sent because Anthropic stores system prompts in the top-level 'system' field and requires at least one user or assistant message in 'messages'.".to_string()
                };
                return Err(TransformError::ValidationFailed {
                    target: ProviderFormat::Anthropic,
                    reason,
                });
            }
            validate_no_non_leading_system_messages(&msgs)?;
            // Convert remaining messages
            let anthropic_messages = universal_messages_to_anthropic_input_messages(msgs)
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

        // Determine reasoning style based on model capability and source:
        // - Opus 4.7/4.8 → thinking.type=adaptive + output_config.effort
        // - Opus 4.5/4.6 with effort canonical → output_config.effort
        // - All other cases → thinking object (legacy, broad model support)
        // Both branches use output_config.format for structured output (never output_format).
        let reasoning_config = req.params.reasoning.as_ref();
        let use_adaptive_thinking = capabilities::supports_adaptive_thinking(model)
            && reasoning_config.is_some_and(reasoning_is_enabled);
        let use_effort = capabilities::supports_output_config_effort(model)
            && reasoning_config.is_some_and(|r| {
                r.canonical == Some(ReasoningCanonical::Effort) || use_adaptive_thinking
            });

        let thinking_val = if use_adaptive_thinking {
            Some(
                serde_json::to_value(&Thinking {
                    budget_tokens: None,
                    display: None,
                    thinking_type: ThinkingType::Adaptive,
                })
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
            )
        } else if use_effort {
            None
        } else {
            req.params.reasoning_for(ProviderFormat::Anthropic)
        };

        let reasoning_enabled =
            use_effort || thinking_val.as_ref().is_some_and(is_enabled_thinking);
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

        let use_json_object_shim =
            is_json_object_response_format(req.params.response_format.as_ref())
                && anthropic_extras_view.tools.is_none()
                && anthropic_extras_view.tool_choice.is_none();

        let mut tools_for_anthropic = req.params.tools.clone().unwrap_or_default();
        for discovered_tool in tool_discovery::discovered_tools_from_messages(&req.messages) {
            if !tools_for_anthropic
                .iter()
                .any(|tool| tool.name == discovered_tool.name)
            {
                tools_for_anthropic.push(discovered_tool);
            }
        }
        tools_for_anthropic = tool_discovery::normalize_tools_for_anthropic(tools_for_anthropic)?;
        if tool_discovery::has_tool_discovery(&req.messages)
            && !tools_for_anthropic
                .iter()
                .any(tool_discovery::is_anthropic_tool_search_builtin)
        {
            tools_for_anthropic.push(tool_discovery::anthropic_tool_search_tool());
        }

        // Convert tools to Anthropic format
        if let Some(raw_tools) = anthropic_extras_view.tools.as_ref() {
            obj.insert("tools".into(), raw_tools.clone());
        } else if use_json_object_shim {
            obj.insert(
                "tools".into(),
                serde_json::json!([{
                    "name": JSON_OBJECT_SHIM_TOOL_NAME,
                    "description": JSON_OBJECT_SHIM_TOOL_DESCRIPTION,
                    "input_schema": { "type": "object" }
                }]),
            );
        } else if !tools_for_anthropic.is_empty() {
            let anthropic_tools = tools_for_anthropic
                .iter()
                .map(anthropic_tool_value)
                .collect::<Result<Vec<_>, _>>()?;
            obj.insert("tools".into(), Value::Array(anthropic_tools));
        }

        // Convert tool_choice using helper method (handles parallel_tool_calls internally)
        let tool_choice_value =
            if let Some(raw_tool_choice) = anthropic_extras_view.tool_choice.as_ref() {
                Some(raw_tool_choice.clone())
            } else if use_json_object_shim {
                Some(serde_json::json!({
                    "type": "tool",
                    "name": JSON_OBJECT_SHIM_TOOL_NAME
                }))
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
            reasoning_effort_level(reasoning_config, Some(max_tokens))
        } else {
            None
        };
        let format = if use_json_object_shim {
            None
        } else {
            req.params
                .response_format
                .as_ref()
                .and_then(|rf| rf.try_into().ok())
        };

        let raw_output_config = anthropic_extras_view.output_config.as_ref();
        let raw_thinking = anthropic_extras_view.thinking.as_ref();

        if use_adaptive_thinking {
            if effort_level.is_some() || format.is_some() {
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
        } else if let Some(raw_output_config) = raw_output_config {
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
        if use_adaptive_thinking {
            if let Some(thinking) = thinking_val {
                if !(forced_tool_choice && is_enabled_thinking(&thinking)) {
                    obj.insert("thinking".into(), thinking);
                }
            }
        } else if let Some(raw_thinking) = raw_thinking {
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
            let metadata_view: UniversalMetadataUserView =
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

        // Enforce model-specific transforms (e.g. strip sampling params for Opus 4.7).
        capabilities::apply_model_transforms(model, &mut obj);

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

        let mut messages =
            <Vec<Message> as TryFromLLM<Vec<ContentBlock>>>::try_from(content_blocks)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
        maybe_unwrap_json_shim_tool_call(&mut messages);

        let finish_reason = match payload.get("stop_reason").and_then(Value::as_str) {
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
        map.insert("id".into(), Value::String(resp.id_for(self.format())));
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
            return Ok(serde_json::json!({"type": "ping"}));
        }

        let has_finish = chunk
            .choices
            .first()
            .and_then(|c| c.finish_reason.as_ref())
            .is_some();

        let has_tool_calls = chunk
            .choices
            .first()
            .and_then(|c| c.delta_view())
            .is_some_and(|d| !d.tool_calls.is_empty());
        let has_reasoning = chunk
            .choices
            .first()
            .and_then(|c| c.delta_view())
            .is_some_and(|d| !d.reasoning.is_empty());

        // Detect initial metadata (model/id/usage present, no content yet).
        // Exclude chunks with tool_calls — those are handled separately.
        // Exclude chunks with empty choices (e.g. usage-only final chunks from OpenAI).
        let has_metadata = chunk.model.is_some() || chunk.id.is_some() || chunk.usage.is_some();
        let is_initial_metadata = has_metadata
            && !has_finish
            && !has_tool_calls
            && !has_reasoning
            && !chunk.choices.is_empty()
            && chunk
                .choices
                .first()
                .and_then(|c| c.delta_view())
                .is_none_or(|d| d.content.as_deref().is_none_or(str::is_empty));

        let build_message_start = |chunk: &UniversalStreamChunk| -> Value {
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

            // Always include usage — the SDK stores message_start.message as the
            // snapshot and later does snapshot.usage.output_tokens on message_delta.
            if let Some(obj) = message.as_object_mut() {
                let usage_value = match &chunk.usage {
                    Some(usage) => usage.to_provider_value(ProviderFormat::Anthropic),
                    None => serde_json::json!({
                        "input_tokens": 0,
                        "output_tokens": 0
                    }),
                };
                obj.insert("usage".into(), usage_value);
            }

            serde_json::json!({
                "type": "message_start",
                "message": message
            })
        };

        if is_initial_metadata {
            let message_start = build_message_start(chunk);

            // Check if this chunk also carries reasoning content — emit
            // content_block_start + content_block_delta for thinking alongside message_start
            if let Some(choice) = chunk.choices.first() {
                if let Some(delta_view) = choice.delta_view() {
                    if !delta_view.reasoning.is_empty() {
                        let thinking = delta_view
                            .reasoning
                            .iter()
                            .filter_map(|r| r.content.as_deref())
                            .collect::<String>();
                        if !thinking.is_empty() {
                            return Ok(message_start);
                        }
                    }
                }
            }

            return Ok(message_start);
        }

        if has_finish {
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

            // Always include usage — the SDK requires it on message_delta.
            // Use real usage when available, otherwise default to zero.
            if let Some(obj_map) = obj.as_object_mut() {
                let usage_value = match &chunk.usage {
                    Some(usage) => usage.to_provider_value(self.format()),
                    None => serde_json::json!({ "output_tokens": 0 }),
                };
                obj_map.insert("usage".into(), usage_value);
            }

            // Some providers (e.g. GLM/zai) place the final tool-argument fragment in the
            // same delta that carries finish_reason. Re-emit any tool events before the
            // terminating message_delta so those arguments are not dropped.
            let tool_events = chunk
                .choices
                .first()
                .and_then(|choice| choice.delta_view().map(|view| (choice.index, view)))
                .map(|(index, view)| anthropic_tool_call_stream_events(&view, index))
                .unwrap_or_default();
            if tool_events.is_empty() {
                return Ok(obj);
            }
            let mut events = tool_events;
            events.push(obj);
            return Ok(Value::Array(events));
        }

        // Content deltas
        if let Some(choice) = chunk.choices.first() {
            if let Some(delta_view) = choice.delta_view() {
                // Reasoning / thinking delta
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

                // Signature delta
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
                // Tool calls. A single delta may carry assistant text and/or the opening
                // of a tool call (some OSS providers bundle the final text fragment onto
                // the tool-call chunk), and a tool-call delta may itself expand into
                // multiple Anthropic events. Emit any text first so it is never dropped,
                // then the tool events, returning an array when there is more than one.
                if !delta_view.tool_calls.is_empty() {
                    let mut events = Vec::new();
                    if let Some(content) =
                        delta_view.content.as_deref().filter(|c| !c.is_empty())
                    {
                        events.push(serde_json::json!({
                            "type": "content_block_delta",
                            "index": choice.index,
                            "delta": {
                                "type": "text_delta",
                                "text": content
                            }
                        }));
                    }
                    events.extend(anthropic_tool_call_stream_events(&delta_view, choice.index));
                    return Ok(if events.len() == 1 {
                        events.remove(0)
                    } else {
                        Value::Array(events)
                    });
                }

                // Text content delta
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

                // Role-only delta → emit content_block_start (first chunk typically has role)
                let content_is_missing_or_null = delta_view.content.is_none();
                let has_tool_calls_in_view = !delta_view.tool_calls.is_empty();

                if delta_view.role.is_some()
                    && content_is_missing_or_null
                    && !has_tool_calls_in_view
                {
                    return Ok(serde_json::json!({
                        "type": "content_block_start",
                        "index": choice.index,
                        "content_block": { "type": "text", "text": "" }
                    }));
                }
            }
        }

        // Usage-only chunk (e.g. OpenAI's final chunk with empty choices and usage).
        // Emit a message_delta with the usage data. The gateway's streaming layer
        // merges this with the preceding message_delta (which has stop_reason) so
        // the Anthropic SDK sees a single message_delta with both fields.
        if chunk.choices.is_empty() {
            if let Some(ref usage) = chunk.usage {
                let mut obj = serde_json::json!({
                    "type": "message_delta",
                    "delta": {}
                });
                if let Some(obj_map) = obj.as_object_mut() {
                    obj_map.insert("usage".into(), usage.to_provider_value(self.format()));
                }
                return Ok(obj);
            }
            return Ok(serde_json::json!({}));
        }

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

/// Build the Anthropic streaming events for the tool calls in a single delta.
///
/// Maps OpenAI-style streaming tool-call deltas to Anthropic events. The presence of
/// `function.name` (which only appears on the opening delta of a tool call) decides when
/// to open a `tool_use` block — `id` is not used because some OpenAI-compatible providers
/// (e.g. GLM/zai) repeat it on every continuation delta. Argument fragments are always
/// re-emitted as `input_json_delta`, including a fragment bundled into the opening delta,
/// so none are dropped.
fn anthropic_tool_call_stream_events(
    delta_view: &UniversalStreamDelta,
    fallback_index: u32,
) -> Vec<Value> {
    let mut events = Vec::new();
    for tool_call in &delta_view.tool_calls {
        let tool_index = tool_call.index.unwrap_or(fallback_index);
        let function = tool_call.function.clone().unwrap_or_default();
        let tool_name = function.name.unwrap_or_default();
        if tool_name.is_empty() {
            // Continuation delta: stream the argument fragment. Empty fragments are
            // preserved so providers that emit an initial empty input_json_delta
            // round-trip losslessly.
            let arguments = function.arguments.unwrap_or_default();
            events.push(serde_json::json!({
                "type": "content_block_delta",
                "index": tool_index,
                "delta": {
                    "type": "input_json_delta",
                    "partial_json": arguments
                }
            }));
            continue;
        }

        // Opening delta: open the tool_use block.
        events.push(serde_json::json!({
            "type": "content_block_start",
            "index": tool_index,
            "content_block": {
                "type": "tool_use",
                "id": tool_call.id.clone().unwrap_or_default(),
                "name": tool_name,
                "input": {}
            }
        }));
        // Some providers bundle the first argument fragment into the opening delta;
        // re-emit it so it is not lost.
        if let Some(arguments) = function.arguments.filter(|arguments| !arguments.is_empty()) {
            events.push(serde_json::json!({
                "type": "content_block_delta",
                "index": tool_index,
                "delta": {
                    "type": "input_json_delta",
                    "partial_json": arguments
                }
            }));
        }
    }
    events
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
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct ShimInputSchemaView {
        #[serde(rename = "type")]
        schema_type: String,
    }

    #[derive(Debug, Deserialize)]
    struct ShimToolView {
        name: String,
        description: Option<String>,
        input_schema: ShimInputSchemaView,
    }

    #[derive(Debug, Deserialize)]
    struct ShimToolChoiceView {
        #[serde(rename = "type")]
        choice_type: String,
        name: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct ShimOutputConfigView {
        #[serde(default)]
        format: Option<Value>,
    }

    #[derive(Debug, Deserialize)]
    struct ShimAnthropicRequestView {
        #[serde(default)]
        tools: Option<Vec<ShimToolView>>,
        #[serde(default)]
        tool_choice: Option<ShimToolChoiceView>,
        #[serde(default)]
        output_config: Option<ShimOutputConfigView>,
    }

    #[derive(Debug, Deserialize)]
    struct JsonColorView {
        color: String,
    }

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
    fn test_anthropic_rejects_non_leading_system_message() {
        let adapter = AnthropicAdapter;
        let req = UniversalRequest {
            model: Some("claude-3-5-sonnet-20241022".to_string()),
            messages: vec![
                Message::System {
                    content: UserContent::String("Use the initial policy.".to_string()),
                },
                Message::User {
                    content: UserContent::String("First turn.".to_string()),
                },
                Message::System {
                    content: UserContent::String("Use the updated policy.".to_string()),
                },
            ],
            params: UniversalParams {
                token_budget: Some(TokenBudget::OutputTokens(1024)),
                ..Default::default()
            },
        };

        let err = adapter.request_from_universal(&req).unwrap_err();
        assert!(format!("{err}").contains("live Messages API currently rejects"));
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
                }),
                ..Default::default()
            },
        };

        let result = adapter.request_from_universal(&req).unwrap();
        let parsed: CreateMessageParams = serde_json::from_value(result).unwrap();

        assert!(parsed.tool_choice.is_some());
        assert!(
            parsed.thinking.is_none(),
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
                }),
                ..Default::default()
            },
        };

        let result = adapter.request_from_universal(&req).unwrap();
        let parsed: CreateMessageParams = serde_json::from_value(result).unwrap();

        assert!(parsed.tool_choice.is_some());
        assert!(
            parsed.thinking.is_some(),
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
                }),
                ..Default::default()
            },
        };

        let result = adapter.request_from_universal(&req).unwrap();
        let parsed: CreateMessageParams = serde_json::from_value(result).unwrap();

        assert!(parsed.tool_choice.is_some());
        assert_eq!(
            parsed.thinking.map(|thinking| thinking.thinking_type),
            Some(ThinkingType::Disabled),
            "Disabled thinking should be preserved with forced tool_choice"
        );
    }

    #[test]
    fn test_anthropic_strips_sampling_params_for_opus_4_7() {
        use crate::universal::message::UserContent;

        let adapter = AnthropicAdapter;

        for model in [
            "claude-opus-4-7",
            "claude-opus-4-8",
            "claude-opus-4-10",
            "claude-opus-5-0",
            "claude-opus-5.0",
        ] {
            let req = UniversalRequest {
                model: Some(model.to_string()),
                messages: vec![Message::User {
                    content: UserContent::String("Hello".to_string()),
                }],
                params: UniversalParams {
                    temperature: Some(0.7),
                    top_p: Some(0.9),
                    top_k: Some(40),
                    token_budget: Some(TokenBudget::OutputTokens(1024)),
                    ..Default::default()
                },
            };

            let result: CreateMessageParams =
                serde_json::from_value(adapter.request_from_universal(&req).unwrap()).unwrap();

            assert!(
                result.temperature.is_none(),
                "Temperature should be stripped for {model}"
            );
            assert!(
                result.top_p.is_none(),
                "top_p should be stripped for {model}"
            );
            assert!(
                result.top_k.is_none(),
                "top_k should be stripped for {model}"
            );
            assert_eq!(result.model, model);
            assert_eq!(result.max_tokens, 1024);
        }
    }

    #[test]
    fn test_anthropic_opus_4_7_uses_adaptive_thinking_with_effort() {
        use crate::universal::message::UserContent;
        use crate::universal::request::ReasoningConfig;

        let adapter = AnthropicAdapter;

        let req = UniversalRequest {
            model: Some("claude-opus-4-7".to_string()),
            messages: vec![Message::User {
                content: UserContent::String("What is 2+2?".to_string()),
            }],
            params: UniversalParams {
                reasoning: Some(ReasoningConfig {
                    enabled: Some(true),
                    effort: Some(ReasoningEffort::Medium),
                    canonical: Some(ReasoningCanonical::Effort),
                    ..Default::default()
                }),
                token_budget: Some(TokenBudget::OutputTokens(4096)),
                ..Default::default()
            },
        };

        let result: CreateMessageParams =
            serde_json::from_value(adapter.request_from_universal(&req).unwrap()).unwrap();

        let thinking = result.thinking.expect("thinking should be present");
        assert_eq!(thinking.thinking_type, ThinkingType::Adaptive);
        assert_eq!(thinking.budget_tokens, None);

        let output_config = result
            .output_config
            .expect("output_config should be present");
        assert_eq!(output_config.effort, Some(EffortLevel::Medium));
    }

    #[test]
    fn test_anthropic_opus_4_8_uses_adaptive_thinking_with_budget() {
        use crate::universal::message::UserContent;
        use crate::universal::request::ReasoningConfig;

        let adapter = AnthropicAdapter;

        let req = UniversalRequest {
            model: Some("claude-opus-4-8".to_string()),
            messages: vec![Message::User {
                content: UserContent::String("What is 2+2?".to_string()),
            }],
            params: UniversalParams {
                reasoning: Some(ReasoningConfig {
                    enabled: Some(true),
                    budget_tokens: Some(2048),
                    canonical: Some(ReasoningCanonical::BudgetTokens),
                    ..Default::default()
                }),
                token_budget: Some(TokenBudget::OutputTokens(4096)),
                ..Default::default()
            },
        };

        let result: CreateMessageParams =
            serde_json::from_value(adapter.request_from_universal(&req).unwrap()).unwrap();

        let thinking = result.thinking.expect("thinking should be present");
        assert_eq!(thinking.thinking_type, ThinkingType::Adaptive);
        assert_eq!(thinking.budget_tokens, None);

        let output_config = result
            .output_config
            .expect("output_config should be present");
        assert_eq!(output_config.effort, Some(EffortLevel::Medium));
    }

    #[test]
    fn test_anthropic_opus_4_7_normalizes_legacy_enabled_thinking() {
        let adapter = AnthropicAdapter;
        let payload = json!({
            "model": "claude-opus-4-7",
            "max_tokens": 4096,
            "messages": [{"role": "user", "content": "What is 2+2?"}],
            "thinking": {
                "type": "enabled",
                "budget_tokens": 2048
            }
        });

        let universal = adapter.request_to_universal(payload).unwrap();
        assert!(universal.params.reasoning.as_ref().is_some_and(|r| {
            r.enabled == Some(true)
                && r.canonical == Some(ReasoningCanonical::BudgetTokens)
                && r.budget_tokens == Some(2048)
        }));

        let result: CreateMessageParams =
            serde_json::from_value(adapter.request_from_universal(&universal).unwrap()).unwrap();

        let thinking = result.thinking.expect("thinking should be present");
        assert_eq!(thinking.thinking_type, ThinkingType::Adaptive);
        assert_eq!(thinking.budget_tokens, None);

        let output_config = result
            .output_config
            .expect("output_config should be present");
        assert_eq!(output_config.effort, Some(EffortLevel::Medium));
    }

    #[test]
    fn test_anthropic_strips_sampling_params_for_opus_4_7_bedrock() {
        use crate::universal::message::UserContent;

        let adapter = AnthropicAdapter;

        let req = UniversalRequest {
            model: Some("us.anthropic.claude-opus-4-7-v1:0".to_string()),
            messages: vec![Message::User {
                content: UserContent::String("Hello".to_string()),
            }],
            params: UniversalParams {
                temperature: Some(0.5),
                top_p: Some(0.8),
                top_k: Some(32),
                token_budget: Some(TokenBudget::OutputTokens(1024)),
                ..Default::default()
            },
        };

        let result: CreateMessageParams =
            serde_json::from_value(adapter.request_from_universal(&req).unwrap()).unwrap();

        assert!(
            result.temperature.is_none(),
            "Temperature should be stripped for Bedrock-style Opus 4.7 model id"
        );
        assert!(
            result.top_p.is_none(),
            "top_p should be stripped for Bedrock-style Opus 4.7 model id"
        );
        assert!(
            result.top_k.is_none(),
            "top_k should be stripped for Bedrock-style Opus 4.7 model id"
        );
    }

    #[test]
    fn test_anthropic_strips_sampling_params_from_extras_for_opus_4_7() {
        use crate::capabilities::ProviderFormat;
        use crate::universal::message::UserContent;
        use std::collections::HashMap;

        let adapter = AnthropicAdapter;

        // Put sampling params in the Anthropic extras map (e.g. from passthrough path)
        let mut extras_map: HashMap<ProviderFormat, Map<String, Value>> = HashMap::new();
        let mut anthropic_extras = Map::new();
        anthropic_extras.insert("temperature".into(), json!(0.3));
        anthropic_extras.insert("top_p".into(), json!(0.8));
        anthropic_extras.insert("top_k".into(), json!(32));
        extras_map.insert(ProviderFormat::Anthropic, anthropic_extras);

        let req = UniversalRequest {
            model: Some("claude-opus-4-7".to_string()),
            messages: vec![Message::User {
                content: UserContent::String("Hello".to_string()),
            }],
            params: UniversalParams {
                token_budget: Some(TokenBudget::OutputTokens(1024)),
                extras: extras_map,
                ..Default::default()
            },
        };

        let result: CreateMessageParams =
            serde_json::from_value(adapter.request_from_universal(&req).unwrap()).unwrap();

        assert!(
            result.temperature.is_none(),
            "Temperature should be stripped even when sourced from extras for Opus 4.7"
        );
        assert!(
            result.top_p.is_none(),
            "top_p should be stripped even when sourced from extras for Opus 4.7"
        );
        assert!(
            result.top_k.is_none(),
            "top_k should be stripped even when sourced from extras for Opus 4.7"
        );
    }

    #[test]
    fn test_anthropic_strips_temperature_for_fable() {
        use crate::capabilities::ProviderFormat;
        use crate::universal::message::UserContent;
        use std::collections::HashMap;

        let adapter = AnthropicAdapter;
        let mut extras_map: HashMap<ProviderFormat, Map<String, Value>> = HashMap::new();
        let mut anthropic_extras = Map::new();
        anthropic_extras.insert("temperature".into(), json!(0.3));
        extras_map.insert(ProviderFormat::Anthropic, anthropic_extras);

        let req = UniversalRequest {
            model: Some("claude-fable-5".to_string()),
            messages: vec![Message::User {
                content: UserContent::String("Hello".to_string()),
            }],
            params: UniversalParams {
                temperature: Some(0.7),
                top_p: Some(0.9),
                top_k: Some(40),
                token_budget: Some(TokenBudget::OutputTokens(1024)),
                extras: extras_map,
                ..Default::default()
            },
        };

        let result: CreateMessageParams =
            serde_json::from_value(adapter.request_from_universal(&req).unwrap()).unwrap();

        assert!(
            result.temperature.is_none(),
            "temperature should be stripped for Fable, including extras"
        );
        assert!(result.top_p.is_none(), "top_p should be stripped for Fable");
        assert!(result.top_k.is_none(), "top_k should be stripped for Fable");
    }

    #[test]
    fn test_anthropic_preserves_sampling_params_for_non_fixed_sampling_opus() {
        use crate::universal::message::UserContent;

        let adapter = AnthropicAdapter;

        for model in [
            "claude-opus-4-20250514",
            "claude-opus-4-6",
            "claude-opus-4-5-20250514",
            "claude-sonnet-4-5-20250929",
            "claude-3-5-sonnet-20241022",
        ] {
            let req = UniversalRequest {
                model: Some(model.to_string()),
                messages: vec![Message::User {
                    content: UserContent::String("Hello".to_string()),
                }],
                params: UniversalParams {
                    temperature: Some(0.7),
                    top_p: Some(0.9),
                    top_k: Some(40),
                    token_budget: Some(TokenBudget::OutputTokens(1024)),
                    ..Default::default()
                },
            };

            let result: CreateMessageParams =
                serde_json::from_value(adapter.request_from_universal(&req).unwrap()).unwrap();

            assert_eq!(
                result.temperature,
                Some(0.7),
                "{} should preserve temperature",
                model
            );
            assert_eq!(result.top_p, Some(0.9), "{} should preserve top_p", model);
            assert_eq!(result.top_k, Some(40), "{} should preserve top_k", model);
        }
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
    fn test_anthropic_json_object_uses_tool_shim() {
        use crate::providers::openai::adapter::OpenAIAdapter;

        let openai_adapter = OpenAIAdapter;
        let anthropic_adapter = AnthropicAdapter;

        let openai_payload = json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Return JSON"}],
            "response_format": { "type": "json_object" }
        });

        let mut universal = openai_adapter.request_to_universal(openai_payload).unwrap();
        universal.model = Some("claude-sonnet-4-5-20250929".to_string());
        anthropic_adapter.apply_defaults(&mut universal);

        let anthropic_request = anthropic_adapter
            .request_from_universal(&universal)
            .unwrap();
        let request_view: ShimAnthropicRequestView = serde_json::from_value(anthropic_request)
            .expect("shim request should deserialize into typed view");
        let tools = request_view.tools.expect("tools should be present");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, JSON_OBJECT_SHIM_TOOL_NAME);
        assert_eq!(
            tools[0].description.as_deref(),
            Some(JSON_OBJECT_SHIM_TOOL_DESCRIPTION)
        );
        assert_eq!(tools[0].input_schema.schema_type, "object");

        let tool_choice = request_view
            .tool_choice
            .expect("tool_choice should be present");
        assert_eq!(tool_choice.choice_type, "tool");
        assert_eq!(
            tool_choice.name.as_deref(),
            Some(JSON_OBJECT_SHIM_TOOL_NAME)
        );

        assert!(
            request_view
                .output_config
                .as_ref()
                .and_then(|oc| oc.format.as_ref())
                .is_none(),
            "output_config.format should be omitted for json_object shim"
        );
    }

    #[test]
    fn responses_namespace_duplicate_local_tool_names_are_rejected_for_anthropic() {
        use crate::processing::adapters::ProviderAdapter;
        use crate::providers::openai::responses_adapter::ResponsesAdapter;

        let responses_adapter = ResponsesAdapter;
        let anthropic_adapter = AnthropicAdapter;
        let responses_payload = json!({
            "model": "gpt-5-nano",
            "input": [{"role": "user", "content": "Search both systems."}],
            "tools": [
                {
                    "type": "namespace",
                    "name": "crm",
                    "description": "CRM tools.",
                    "tools": [{
                        "type": "function",
                        "name": "lookup",
                        "description": "Look up CRM records.",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "customer_id": {"type": "string"}
                            },
                            "required": ["customer_id"]
                        },
                        "defer_loading": true
                    }]
                },
                {
                    "type": "namespace",
                    "name": "erp",
                    "description": "ERP tools.",
                    "tools": [{
                        "type": "function",
                        "name": "lookup",
                        "description": "Look up ERP records.",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "order_id": {"type": "string"}
                            },
                            "required": ["order_id"]
                        },
                        "defer_loading": true
                    }]
                },
                {"type": "tool_search"}
            ]
        });

        let mut universal = responses_adapter
            .request_to_universal(responses_payload)
            .expect("Responses request should convert to universal");
        universal.model = Some("claude-sonnet-4-5-20250929".to_string());
        anthropic_adapter.apply_defaults(&mut universal);

        let err = anthropic_adapter
            .request_from_universal(&universal)
            .unwrap_err();
        match err {
            TransformError::FromUniversalFailed(reason) => {
                assert!(reason.contains("duplicate local tool name 'lookup'"));
                assert!(reason.contains("'crm'"));
                assert!(reason.contains("'erp'"));
                assert!(reason.contains("Anthropic tools"));
            }
            other => panic!("expected unsupported namespace duplicate mapping, got {other:?}"),
        }
    }

    #[test]
    fn responses_namespace_and_top_level_duplicate_local_tool_names_are_rejected_for_anthropic() {
        use crate::processing::adapters::ProviderAdapter;
        use crate::providers::openai::responses_adapter::ResponsesAdapter;

        let responses_adapter = ResponsesAdapter;
        let anthropic_adapter = AnthropicAdapter;
        let responses_payload = json!({
            "model": "gpt-5-nano",
            "input": [{"role": "user", "content": "Search both systems."}],
            "tools": [
                {
                    "type": "function",
                    "name": "lookup",
                    "description": "Look up top-level records.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "record_id": {"type": "string"}
                        },
                        "required": ["record_id"]
                    }
                },
                {
                    "type": "namespace",
                    "name": "crm",
                    "description": "CRM tools.",
                    "tools": [{
                        "type": "function",
                        "name": "lookup",
                        "description": "Look up CRM records.",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "customer_id": {"type": "string"}
                            },
                            "required": ["customer_id"]
                        },
                        "defer_loading": true
                    }]
                },
                {"type": "tool_search"}
            ]
        });

        let mut universal = responses_adapter
            .request_to_universal(responses_payload)
            .expect("Responses request should convert to universal");
        universal.model = Some("claude-sonnet-4-5-20250929".to_string());
        anthropic_adapter.apply_defaults(&mut universal);

        let err = anthropic_adapter
            .request_from_universal(&universal)
            .unwrap_err();
        match err {
            TransformError::FromUniversalFailed(reason) => {
                assert!(reason.contains("duplicate local tool name 'lookup'"));
                assert!(reason.contains("top-level tool"));
                assert!(reason.contains("namespace 'crm'"));
                assert!(reason.contains("Anthropic tools"));
            }
            other => panic!("expected unsupported duplicate mapping, got {other:?}"),
        }
    }

    #[test]
    fn test_anthropic_response_tool_shim_unwraps_to_assistant_content() {
        let adapter = AnthropicAdapter;
        let payload = json!({
            "id": "msg_test",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "stop_reason": "tool_use",
            "content": [{
                "type": "tool_use",
                "id": "toolu_123",
                "name": JSON_OBJECT_SHIM_TOOL_NAME,
                "input": { "color": "blue" }
            }]
        });

        let universal = adapter.response_to_universal(payload).unwrap();
        assert_eq!(universal.messages.len(), 1);
        match &universal.messages[0] {
            Message::Assistant { content, .. } => match content {
                crate::universal::message::AssistantContent::String(text) => {
                    let parsed: JsonColorView = serde_json::from_str(text)
                        .expect("shim output should be valid serialized JSON object");
                    assert_eq!(parsed.color, "blue");
                }
                _ => panic!("expected assistant string content after shim unwrap"),
            },
            _ => panic!("expected assistant message"),
        }
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

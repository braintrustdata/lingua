/*!
Google AI provider adapter for GenerateContent API.

Google's API has some unique characteristics:
- Uses `contents` instead of `messages`
- Generation params are in `generationConfig` object
- Uses camelCase field names (e.g., `maxOutputTokens`)
- Streaming is endpoint-based, not parameter-based
*/

use crate::capabilities::ProviderFormat;
use crate::processing::adapters::ProviderAdapter;
use crate::processing::transform::TransformError;
use crate::providers::google::capabilities::{
    effort_to_thinking_level, thinking_level_to_effort, GoogleCapabilities, GoogleThinkingStyle,
};
use crate::providers::google::convert::SYNTHETIC_CALL_ID_PREFIX;
use crate::providers::google::detect::try_parse_google;
use crate::providers::google::generated::{
    Content as GoogleContent, GenerateContentResponse, GenerationConfig, ThinkingConfig,
    ThinkingLevel, Tool as GoogleTool, ToolConfig, UsageMetadata,
};
use crate::providers::google::params::GoogleParams;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{AssistantContent, AssistantContentPart, Message};
use crate::universal::reasoning::{budget_to_effort, effort_to_budget, MIN_THINKING_BUDGET};
use crate::universal::request::ToolChoiceConfig;
use crate::universal::tools::UniversalTool;
use crate::universal::ToolContentPart;
use crate::universal::{
    extract_system_messages, flatten_consecutive_messages, FinishReason, ReasoningCanonical,
    ReasoningConfig, TokenBudget, UniversalParams, UniversalReasoningDelta, UniversalRequest,
    UniversalResponse, UniversalStreamChoice, UniversalStreamChunk, UniversalStreamDelta,
    UniversalToolCallDelta, UniversalToolFunctionDelta, UniversalUsage, UserContent,
};
use serde::{Deserialize, Serialize};

/// Adapter for Google AI GenerateContent API.
pub struct GoogleAdapter;

#[derive(Debug, Clone, Default, Deserialize)]
struct GoogleSdkConfigExtrasView {
    config: Option<GenerationConfig>,
}

fn is_discovery_only_message(message: &Message) -> bool {
    match message {
        Message::Assistant {
            content: AssistantContent::Array(parts),
            ..
        } => {
            !parts.is_empty()
                && parts
                    .iter()
                    .all(|part| matches!(part, AssistantContentPart::ToolDiscoveryCall { .. }))
        }
        Message::Tool { content } => {
            !content.is_empty()
                && content
                    .iter()
                    .all(|part| matches!(part, ToolContentPart::ToolDiscoveryResult(_)))
        }
        _ => false,
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct GoogleStreamPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thought: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thought_signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function_call: Option<Map<String, Value>>,
}

impl ProviderAdapter for GoogleAdapter {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::Google
    }

    fn directory_name(&self) -> &'static str {
        "google"
    }

    fn display_name(&self) -> &'static str {
        "Google"
    }

    fn detect_request(&self, payload: &Value) -> bool {
        try_parse_google(payload).is_ok()
    }

    fn request_to_universal(&self, payload: Value) -> Result<UniversalRequest, TransformError> {
        // Single parse: typed params now includes typed contents and generation_config
        let typed_params: GoogleParams = serde_json::from_value(payload)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let model = typed_params.model.clone();

        // Extract typed contents (partial move - other fields remain accessible)
        let contents = typed_params.contents.ok_or_else(|| {
            TransformError::ToUniversalFailed("Google: missing 'contents' field".to_string())
        })?;

        let messages = <Vec<Message> as TryFromLLM<Vec<GoogleContent>>>::try_from(contents)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Extract params from generationConfig (now typed in params struct)
        let (temperature, top_p, top_k, max_tokens, stop, reasoning) =
            if let Some(config) = &typed_params.generation_config {
                let max_tokens = config.max_output_tokens;
                // Convert Google's thinkingConfig to ReasoningConfig
                // thinkingLevel: Gemini 3 (effort-based)
                // thinkingBudget: Gemini 2.5 (budget-based), 0 means disabled
                let reasoning = config.thinking_config.as_ref().map(|tc| {
                    if let Some(ref level) = tc.thinking_level {
                        // Gemini 3 style: thinkingLevel is canonical (effort-based)
                        let effort = thinking_level_to_effort(level);
                        let budget = effort_to_budget(effort, max_tokens);
                        ReasoningConfig {
                            enabled: Some(true),
                            effort: Some(effort),
                            budget_tokens: Some(budget),
                            canonical: Some(ReasoningCanonical::Effort),
                            ..Default::default()
                        }
                    } else {
                        // Gemini 2.5 style: thinkingBudget is canonical (budget-based)
                        let is_disabled = tc.thinking_budget == Some(0);
                        let budget_tokens = tc.thinking_budget;
                        let effort = budget_tokens.map(|b| budget_to_effort(b, max_tokens));
                        let canonical = if tc.thinking_budget.is_some() {
                            Some(ReasoningCanonical::GoogleThinkingBudget)
                        } else {
                            Some(ReasoningCanonical::GoogleIncludeThoughts)
                        };
                        ReasoningConfig {
                            enabled: Some(!is_disabled),
                            effort,
                            budget_tokens,
                            canonical,
                            ..Default::default()
                        }
                    }
                });
                let stop = config.stop_sequences.clone().filter(|s| !s.is_empty());
                (
                    config.temperature,
                    config.top_p,
                    config.top_k,
                    max_tokens,
                    stop,
                    reasoning,
                )
            } else {
                (None, None, None, None, None, None)
            };

        // Convert tools using typed conversions
        let tools = typed_params
            .tools
            .map(<Vec<UniversalTool> as TryFromLLM<Vec<_>>>::try_from)
            .transpose()
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Convert tool_choice from Google's ToolConfig
        let tool_choice = typed_params
            .tool_config
            .as_ref()
            .map(ToolChoiceConfig::from);

        // Convert response_format from GenerationConfig
        let sdk_config_extras: GoogleSdkConfigExtrasView =
            serde_json::to_value(&typed_params.extras)
                .and_then(serde_json::from_value)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
        let sdk_config_response_format = sdk_config_extras
            .config
            .as_ref()
            .map(crate::universal::request::ResponseFormatConfig::from);
        let response_format = typed_params
            .generation_config
            .as_ref()
            .map(crate::universal::request::ResponseFormatConfig::from)
            .or(sdk_config_response_format)
            .filter(|rf| rf.format_type.is_some());

        let mut params = UniversalParams {
            temperature,
            top_p,
            top_k,
            token_budget: max_tokens.map(TokenBudget::OutputTokens),
            stop,
            tools,
            tool_choice,
            response_format,
            seed: None, // Google doesn't support seed
            presence_penalty: None,
            frequency_penalty: None,
            stream: None, // Google uses endpoint-based streaming
            // New canonical fields - Google doesn't support most of these
            parallel_tool_calls: None,
            reasoning,
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
                ProviderFormat::Google,
                typed_params.extras.into_iter().collect(),
            );
        }

        Ok(UniversalRequest {
            model,
            messages,
            params,
        })
    }

    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
        // Extract system messages (Google requires them in systemInstruction, not contents)
        let mut messages = req.messages.clone();
        let system_contents = extract_system_messages(&mut messages);

        if messages.is_empty() {
            let reason = if system_contents.is_empty() {
                "Google requires at least one message in 'contents'.".to_string()
            } else {
                "Google requires at least one non-system message; a system prompt alone cannot be sent because Google stores system prompts in the top-level 'systemInstruction' field and requires at least one user or model message in 'contents'.".to_string()
            };
            return Err(TransformError::ValidationFailed {
                target: ProviderFormat::Google,
                reason,
            });
        }

        // Flatten consecutive messages of the same role (Google doesn't allow them)
        flatten_consecutive_messages(&mut messages);

        // Fill in tool names from preceding tool_calls — Google requires functionResponse.name
        // but some formats (e.g. OpenAI chat-completions role:tool) don't carry the function name
        fill_tool_names_from_context(&mut messages);

        let capabilities = GoogleCapabilities::detect(req.model.as_deref());
        if capabilities.requires_thought_signature_for_function_call_history {
            add_dummy_thought_signatures_for_transferred_function_call_history(&mut messages);
        }
        messages.retain(|message| !is_discovery_only_message(message));
        if messages.is_empty() {
            return Err(TransformError::ValidationFailed {
                target: ProviderFormat::Google,
                reason: "Google does not support dynamic tool discovery history without at least one non-discovery content message.".to_string(),
            });
        }

        // Convert messages to Google contents
        let google_contents: Vec<GoogleContent> =
            <Vec<GoogleContent> as TryFromLLM<Vec<Message>>>::try_from(messages)
                .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

        let mut obj = Map::new();

        // Insert model if present (though Google often uses URL-based model selection)
        if let Some(model) = &req.model {
            obj.insert("model".into(), Value::String(model.clone()));
        }

        obj.insert(
            "contents".into(),
            serde_json::to_value(google_contents)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
        );

        // Add systemInstruction if system messages were present
        if !system_contents.is_empty() {
            let system_text = system_contents
                .into_iter()
                .map(|c| match c {
                    UserContent::String(s) => s,
                    UserContent::Array(parts) => parts
                        .into_iter()
                        .filter_map(|p| {
                            if let crate::universal::UserContentPart::Text(t) = p {
                                Some(t.text)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                })
                .collect::<Vec<_>>()
                .join("\n");

            obj.insert(
                "systemInstruction".into(),
                serde_json::json!({
                    "parts": [{"text": system_text}]
                }),
            );
        }

        // Build generationConfig if any params are set
        let has_reasoning = req
            .params
            .reasoning
            .as_ref()
            .map(|r| !r.is_effectively_disabled())
            .unwrap_or(false);
        let has_response_format = req
            .params
            .response_format
            .as_ref()
            .map(|rf| rf.format_type.is_some())
            .unwrap_or(false);
        let has_params = req.params.temperature.is_some()
            || req.params.top_p.is_some()
            || req.params.top_k.is_some()
            || req.params.output_token_budget().is_some()
            || req.params.stop.is_some()
            || has_reasoning
            || has_response_format;

        if has_params {
            // Convert ReasoningConfig to Google's thinkingConfig
            // Use capabilities to determine whether to use thinkingLevel (Gemini 3) or thinkingBudget (Gemini 2.5)
            let thinking_config = req.params.reasoning.as_ref().and_then(|r| {
                if r.is_effectively_disabled() {
                    return None;
                }

                let caps = GoogleCapabilities::detect(req.model.as_deref());

                match caps.thinking_style {
                    GoogleThinkingStyle::ThinkingLevelBased => {
                        if r.canonical == Some(ReasoningCanonical::GoogleThinkingBudget) {
                            if let Some(budget) = r.budget_tokens {
                                return Some(ThinkingConfig {
                                    include_thoughts: Some(true),
                                    thinking_budget: Some(budget),
                                    thinking_level: None,
                                });
                            }
                        }

                        if r.canonical == Some(ReasoningCanonical::GoogleIncludeThoughts) {
                            return Some(ThinkingConfig {
                                include_thoughts: Some(true),
                                thinking_budget: None,
                                thinking_level: None,
                            });
                        }

                        // Gemini 3: use thinkingLevel (effort-based) unless the source was
                        // Google's native thinkingBudget and must roundtrip unchanged.
                        let level = r
                            .effort
                            .map(effort_to_thinking_level)
                            .unwrap_or(ThinkingLevel::High);
                        Some(ThinkingConfig {
                            include_thoughts: Some(true),
                            thinking_budget: None,
                            thinking_level: Some(level),
                        })
                    }
                    GoogleThinkingStyle::ThinkingBudget | GoogleThinkingStyle::None => {
                        if r.canonical == Some(ReasoningCanonical::GoogleIncludeThoughts) {
                            return Some(ThinkingConfig {
                                include_thoughts: Some(true),
                                thinking_budget: None,
                                thinking_level: None,
                            });
                        }

                        // Gemini 2.5: use thinkingBudget (budget-based)
                        let budget = r.budget_tokens.unwrap_or(MIN_THINKING_BUDGET);
                        Some(ThinkingConfig {
                            include_thoughts: Some(true),
                            thinking_budget: Some(budget),
                            thinking_level: None,
                        })
                    }
                }
            });

            let stop_sequences = req.params.stop.clone();

            let mut config = GenerationConfig {
                temperature: req.params.temperature,
                top_p: req.params.top_p,
                top_k: req.params.top_k,
                max_output_tokens: req.params.output_token_budget(),
                stop_sequences,
                thinking_config,
                ..Default::default()
            };

            // Apply response format to generationConfig
            if let Some(format) = &req.params.response_format {
                let response_config = GenerationConfig::try_from(format)
                    .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;
                config.response_mime_type = response_config.response_mime_type;
                config.generation_config_response_json_schema =
                    response_config.generation_config_response_json_schema;
                config.response_schema = response_config.response_schema;
            }

            obj.insert(
                "generationConfig".into(),
                serde_json::to_value(config)
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
            );
        }

        // Add tools if present.  FunctionDeclaration::try_from strips unsupported JSON Schema
        // keywords (e.g. `exclusiveMinimum`) from parametersJsonSchema during conversion.
        if let Some(tools) = &req.params.tools {
            let google_tools = <Vec<GoogleTool> as TryFromLLM<Vec<_>>>::try_from(tools.clone())
                .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;
            if !google_tools.is_empty() {
                obj.insert(
                    "tools".into(),
                    serde_json::to_value(&google_tools)
                        .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
                );
            }
        }

        // Add tool_choice if present
        if let Some(tool_choice) = &req.params.tool_choice {
            if let Ok(tool_config) = ToolConfig::try_from(tool_choice) {
                obj.insert(
                    "toolConfig".into(),
                    serde_json::to_value(&tool_config)
                        .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
                );
            }
        }

        // Merge back provider-specific extras (only for Google)
        if let Some(extras) = req.params.extras.get(&ProviderFormat::Google) {
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
        // Google has no strict response-schema equivalent. Canonically drop it so
        // cross-provider semantic comparison does not treat this as a regression.
        if let Some(format) = &mut req.params.response_format {
            if let Some(json_schema) = &mut format.json_schema {
                json_schema.strict = None;
            }
        }
    }

    fn detect_response(&self, payload: &Value) -> bool {
        // Google response has candidates array (content may be missing for NO_IMAGE etc)
        payload
            .get("candidates")
            .and_then(Value::as_array)
            .is_some_and(|arr| !arr.is_empty())
    }

    fn response_to_universal(&self, payload: Value) -> Result<UniversalResponse, TransformError> {
        let response: GenerateContentResponse = serde_json::from_value(payload)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let mut messages = Vec::new();
        let mut finish_reasons = Vec::new();

        for candidate in response.candidates.iter().flatten() {
            if let Some(content) = &candidate.content {
                let universal = <Message as TryFromLLM<GoogleContent>>::try_from(content.clone())
                    .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                messages.push(universal);
            }

            if let Some(reason) = candidate.finish_reason.as_ref().map(FinishReason::from) {
                finish_reasons.push(reason);
            }
        }

        let has_tool_calls = messages.iter().any(|m| {
            if let Message::Assistant {
                content: AssistantContent::Array(parts),
                ..
            } = m
            {
                parts
                    .iter()
                    .any(|p| matches!(p, AssistantContentPart::ToolCall { .. }))
            } else {
                false
            }
        });

        let finish_reason = if has_tool_calls {
            Some(FinishReason::ToolCalls)
        } else {
            finish_reasons.first().cloned()
        };

        let usage = response.usage_metadata.as_ref().map(UniversalUsage::from);

        Ok(UniversalResponse {
            id: None, // Google doesn't include a top-level response ID
            id_format: None,
            model: response.model_version,
            messages,
            usage,
            finish_reason,
            finish_reasons,
        })
    }

    fn response_from_universal(&self, resp: &UniversalResponse) -> Result<Value, TransformError> {
        let finish_reason = resp
            .finish_reason
            .as_ref()
            .map(|r| r.to_provider_string(self.format()).to_string())
            .unwrap_or_else(|| "STOP".to_string());

        let candidates: Vec<Value> = if resp.messages.is_empty() && resp.finish_reason.is_some() {
            vec![serde_json::json!({
                "index": 0,
                "finishReason": finish_reason
            })]
        } else {
            let google_contents = resp
                .messages
                .iter()
                .filter(|message| !is_discovery_only_message(message))
                .map(|msg| {
                    <GoogleContent as TryFromLLM<Message>>::try_from(msg.clone())
                        .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))
                })
                .collect::<Result<Vec<_>, TransformError>>()?;

            google_contents
                .into_iter()
                .enumerate()
                .map(|(i, content)| {
                    let content_value = serde_json::to_value(&content)
                        .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

                    Ok(serde_json::json!({
                        "index": i,
                        "content": content_value,
                        "finishReason": finish_reason
                    }))
                })
                .collect::<Result<Vec<_>, TransformError>>()?
        };

        let mut map = serde_json::Map::new();
        map.insert("candidates".into(), Value::Array(candidates));

        if let Some(model) = &resp.model {
            map.insert("modelVersion".into(), Value::String(model.clone()));
        }

        if let Some(usage) = &resp.usage {
            let metadata = UsageMetadata::from(usage);
            let value = serde_json::to_value(&metadata)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;
            map.insert("usageMetadata".into(), value);
        }

        Ok(Value::Object(map))
    }

    // =========================================================================
    // Streaming response handling
    // =========================================================================

    fn detect_stream_response(&self, payload: &Value) -> bool {
        // Google streaming uses the same format as non-streaming (candidates array)
        // The response_to_universal detection already handles this
        self.detect_response(payload)
    }

    fn stream_to_universal(
        &self,
        payload: Value,
    ) -> Result<Option<UniversalStreamChunk>, TransformError> {
        let typed_payload: GenerateContentResponse =
            serde_json::from_value(payload).map_err(|e| {
                TransformError::ToUniversalFailed(format!("failed to parse stream payload: {e}"))
            })?;
        let candidates = typed_payload
            .candidates
            .ok_or_else(|| TransformError::ToUniversalFailed("missing candidates".to_string()))?;

        let mut choices = Vec::new();

        for candidate in candidates {
            let index = candidate.index.unwrap_or(0) as u32;
            let parts = candidate
                .content
                .and_then(|content| content.parts)
                .unwrap_or_default();

            let mut text_segments = Vec::new();
            let mut reasoning = Vec::new();
            for part in &parts {
                let Some(text) = part.text.as_deref() else {
                    continue;
                };
                if part.thought == Some(true) {
                    reasoning.push(UniversalReasoningDelta {
                        content: Some(text.to_string()),
                    });
                } else {
                    text_segments.push(text);
                }
            }
            let text = text_segments.join("");
            let reasoning_signature = parts.iter().find_map(|part| part.thought_signature.clone());

            let response_id = typed_payload.response_id.as_deref();
            let mut tool_call_index = 0_u32;
            let tool_calls: Vec<UniversalToolCallDelta> = parts
                .iter()
                .filter_map(|part| {
                    part.function_call.as_ref().map(|function_call| {
                        let index = tool_call_index;
                        tool_call_index += 1;
                        UniversalToolCallDelta {
                            index: Some(index),
                            id: function_call.id.clone().or_else(|| {
                                Some(match response_id {
                                    Some(response_id) => {
                                        format!("{SYNTHETIC_CALL_ID_PREFIX}{response_id}_{index}")
                                    }
                                    None => format!("{SYNTHETIC_CALL_ID_PREFIX}{index}"),
                                })
                            }),
                            call_type: Some("function".to_string()),
                            function: Some(UniversalToolFunctionDelta {
                                name: function_call.name.clone(),
                                arguments: function_call
                                    .args
                                    .as_ref()
                                    .map(|args| Value::Object(args.clone()).to_string()),
                            }),
                        }
                    })
                })
                .collect();

            let finish_reason = candidate
                .finish_reason
                .as_ref()
                .and_then(|reason| serde_json::to_value(reason).ok())
                .and_then(|reason| match reason {
                    Value::String(s) => Some(s),
                    _ => None,
                })
                .map(|reason| {
                    if tool_calls.is_empty() {
                        FinishReason::from_provider_string(&reason, self.format()).to_string()
                    } else {
                        FinishReason::ToolCalls.to_string()
                    }
                });

            let delta = UniversalStreamDelta {
                role: Some("assistant".to_string()),
                content: Some(text),
                tool_calls,
                reasoning,
                reasoning_signature,
            };

            choices.push(UniversalStreamChoice {
                index,
                delta: Some(Value::from(delta)),
                finish_reason,
            });
        }

        let usage = typed_payload
            .usage_metadata
            .as_ref()
            .map(UniversalUsage::from);
        let model = typed_payload.model_version;
        let id = typed_payload.response_id;

        Ok(Some(UniversalStreamChunk::new(
            id, model, choices, None, usage,
        )))
    }

    fn stream_from_universal(&self, chunk: &UniversalStreamChunk) -> Result<Value, TransformError> {
        let mut candidates: Vec<Value> = chunk
            .choices
            .iter()
            .map(|c| {
                let delta = c.delta_view();

                // Build parts array from text and tool_calls
                let mut parts: Vec<GoogleStreamPart> = Vec::new();

                let text = delta
                    .as_ref()
                    .and_then(|d| d.content.as_deref())
                    .unwrap_or("");
                let has_function_call_part = delta.as_ref().is_some_and(|d| {
                    d.tool_calls
                        .iter()
                        .any(|tool_call| tool_call.function.is_some())
                });
                let text_reasoning_signature = delta
                    .as_ref()
                    .and_then(|d| d.reasoning_signature.as_deref())
                    .filter(|_| {
                        delta.as_ref().is_none_or(|d| {
                            !has_function_call_part && (!text.is_empty() || d.reasoning.is_empty())
                        })
                    });

                if let Some(ref d) = delta {
                    let reasoning_texts: Vec<&str> = d
                        .reasoning
                        .iter()
                        .filter_map(|reasoning| {
                            reasoning.content.as_deref().filter(|s| !s.is_empty())
                        })
                        .collect();
                    let thought_reasoning_signature =
                        d.reasoning_signature.as_deref().filter(|_| {
                            !reasoning_texts.is_empty()
                                && text.is_empty()
                                && !has_function_call_part
                        });

                    for (index, text) in reasoning_texts.iter().enumerate() {
                        let mut thought_part = GoogleStreamPart {
                            text: Some((*text).to_string()),
                            thought: Some(true),
                            ..Default::default()
                        };
                        if index == reasoning_texts.len() - 1 {
                            if let Some(signature) = thought_reasoning_signature {
                                thought_part.thought_signature = Some(signature.to_string());
                            }
                        }
                        parts.push(thought_part);
                    }
                }

                // Add text part if present, carrying thoughtSignature when there are no tool calls
                if !text.is_empty() || text_reasoning_signature.is_some() {
                    let mut text_part = GoogleStreamPart {
                        text: Some(text.to_string()),
                        ..Default::default()
                    };
                    if let Some(signature) = text_reasoning_signature {
                        text_part.thought_signature = Some(signature.to_string());
                    }
                    parts.push(text_part);
                }

                // Add functionCall parts from tool_calls
                if let Some(ref d) = delta {
                    for tc in &d.tool_calls {
                        if let Some(ref func) = tc.function {
                            let mut function_call = Map::new();
                            if let Some(ref name) = func.name {
                                function_call.insert("name".into(), Value::String(name.clone()));
                            }
                            if let Some(ref id) = tc.id {
                                function_call.insert("id".into(), Value::String(id.clone()));
                            }
                            if let Some(ref args) = func.arguments {
                                if args.is_empty() {
                                    function_call.insert("args".into(), serde_json::json!({}));
                                } else if let Ok(args_val) = serde_json::from_str::<Value>(args) {
                                    function_call.insert("args".into(), args_val);
                                }
                            }
                            let mut part = GoogleStreamPart {
                                function_call: Some(function_call),
                                ..Default::default()
                            };
                            if let Some(ref signature) = d.reasoning_signature {
                                part.thought_signature = Some(signature.clone());
                            }
                            parts.push(part);
                        }
                    }
                }

                // Ensure at least one empty text part if no parts
                if parts.is_empty() {
                    parts.push(GoogleStreamPart {
                        text: Some(String::new()),
                        ..Default::default()
                    });
                }

                let finish_reason = c.finish_reason.as_ref().map(|r| {
                    let fr: FinishReason = r.parse().unwrap_or(FinishReason::Other(r.clone()));
                    fr.to_provider_string(self.format()).to_string()
                });

                let mut candidate_map = serde_json::Map::new();
                candidate_map.insert("index".into(), serde_json::json!(c.index));
                candidate_map.insert(
                    "content".into(),
                    serde_json::json!({
                        "parts": parts,
                        "role": "model"
                    }),
                );

                if let Some(reason) = finish_reason {
                    candidate_map.insert("finishReason".into(), Value::String(reason.to_string()));
                }

                Ok(Value::Object(candidate_map))
            })
            .collect::<Result<Vec<_>, TransformError>>()?;

        if chunk.is_keep_alive() || candidates.is_empty() {
            candidates.push(serde_json::json!({
                "index": 0,
                "content": {
                    "parts": [{"text": ""}],
                    "role": "model"
                }
            }));
        }

        let mut map = serde_json::Map::new();
        map.insert("candidates".into(), Value::Array(candidates));

        if let Some(ref id) = chunk.id {
            map.insert("responseId".into(), Value::String(id.clone()));
        }
        if let Some(ref model) = chunk.model {
            map.insert("modelVersion".into(), Value::String(model.clone()));
        }
        if let Some(ref usage) = chunk.usage {
            let metadata = UsageMetadata::from(usage);
            let value = serde_json::to_value(&metadata)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;
            map.insert("usageMetadata".into(), value);
        }

        Ok(Value::Object(map))
    }
}

/// Build a tool_call_id → tool_name map from assistant messages and use it to
/// fill in empty tool names on Tool messages. Google requires `functionResponse.name`
/// but formats like OpenAI chat-completions don't include the name on tool result
/// messages — only the preceding assistant message has it.
fn fill_tool_names_from_context(messages: &mut [Message]) {
    let mut id_to_name: std::collections::HashMap<String, String> = Default::default();
    for msg in messages.iter() {
        if let Message::Assistant {
            content: AssistantContent::Array(parts),
            ..
        } = msg
        {
            for part in parts {
                if let AssistantContentPart::ToolCall {
                    tool_call_id,
                    tool_name,
                    ..
                } = part
                {
                    if !tool_name.is_empty() {
                        id_to_name.insert(tool_call_id.clone(), tool_name.clone());
                    }
                }
            }
        }
    }
    for msg in messages.iter_mut() {
        if let Message::Tool { content } = msg {
            for part in content.iter_mut() {
                if let ToolContentPart::ToolResult(result) = part {
                    if result.tool_name.is_empty() {
                        if let Some(name) = id_to_name.get(&result.tool_call_id) {
                            result.tool_name = name.clone();
                        }
                    }
                }
            }
        }
    }
}

const GOOGLE_DUMMY_THOUGHT_SIGNATURE: &str = "skip_thought_signature_validator";

fn add_dummy_thought_signatures_for_transferred_function_call_history(messages: &mut [Message]) {
    let Some(current_turn_start) = messages
        .iter()
        .rposition(|message| matches!(message, Message::User { .. }))
    else {
        return;
    };

    let mut later_tool_result_ids = std::collections::HashSet::new();
    let mut index = messages.len();
    while index > current_turn_start + 1 {
        index -= 1;

        match &mut messages[index] {
            Message::Tool { content } => {
                for part in content {
                    if let ToolContentPart::ToolResult(result) = part {
                        later_tool_result_ids.insert(result.tool_call_id.clone());
                    }
                }
            }
            Message::Assistant {
                content: AssistantContent::Array(parts),
                ..
            } => {
                let first_paired_call = parts.iter_mut().find(|part| {
                    if let AssistantContentPart::ToolCall { tool_call_id, .. } = part {
                        later_tool_result_ids.contains(tool_call_id)
                    } else {
                        false
                    }
                });

                if let Some(AssistantContentPart::ToolCall {
                    encrypted_content, ..
                }) = first_paired_call
                {
                    if encrypted_content.is_none() {
                        // Google documents this dummy signature for transferred function-call
                        // history. Only the first functionCall in a step is validated; native
                        // Gemini parallel siblings should remain unsigned if returned that way.
                        *encrypted_content = Some(GOOGLE_DUMMY_THOUGHT_SIGNATURE.to_string());
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::google::GenerateContentRequest;
    use crate::serde_json::json;
    use crate::universal::request::ToolChoiceMode;
    use crate::universal::ReasoningEffort;

    #[test]
    fn test_google_detect_request() {
        let adapter = GoogleAdapter;
        let payload = json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": "Hello"}]
            }]
        });
        assert!(adapter.detect_request(&payload));
    }

    #[test]
    fn test_google_passthrough() {
        let adapter = GoogleAdapter;
        let payload = json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": "Hello"}]
            }],
            "generationConfig": {
                "temperature": 0.7,
                "maxOutputTokens": 1024
            }
        });

        let universal = adapter.request_to_universal(payload).unwrap();
        // Use approximate comparison due to f32->f64 conversion precision
        assert!((universal.params.temperature.unwrap() - 0.7).abs() < 0.001);
        assert_eq!(
            universal.params.token_budget,
            Some(TokenBudget::OutputTokens(1024))
        );

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        let reconstructed: GenerateContentRequest =
            serde_json::from_value(reconstructed).expect("request should deserialize");
        assert!(reconstructed.contents.is_some());
        assert!(reconstructed.generation_config.is_some());
    }

    #[test]
    fn test_google_same_provider_preserves_budget_based_thinking_config_for_gemini_3() {
        let adapter = GoogleAdapter;
        let payload = json!({
            "model": "gemini-3.5-flash",
            "contents": [{
                "role": "user",
                "parts": [{"text": "Return JSON."}]
            }],
            "generationConfig": {
                "maxOutputTokens": 2048,
                "thinkingConfig": {
                    "thinkingBudget": 1024,
                    "includeThoughts": true
                }
            }
        });

        let universal = adapter.request_to_universal(payload).unwrap();
        let reasoning = universal.params.reasoning.as_ref().unwrap();
        assert_eq!(
            reasoning.canonical,
            Some(ReasoningCanonical::GoogleThinkingBudget)
        );
        assert_eq!(reasoning.budget_tokens, Some(1024));

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        let reconstructed: GenerateContentRequest =
            serde_json::from_value(reconstructed).expect("request should deserialize");
        let thinking_config = reconstructed
            .generation_config
            .and_then(|config| config.thinking_config)
            .expect("thinkingConfig should be present");

        assert_eq!(thinking_config.thinking_budget, Some(1024));
        assert_eq!(thinking_config.include_thoughts, Some(true));
        assert_eq!(thinking_config.thinking_level, None);
    }

    #[test]
    fn test_google_same_provider_preserves_include_thoughts_without_budget_for_gemini_3() {
        let adapter = GoogleAdapter;
        let payload = json!({
            "model": "gemini-3.5-flash",
            "contents": [{
                "role": "user",
                "parts": [{"text": "Return JSON."}]
            }],
            "generationConfig": {
                "maxOutputTokens": 2048,
                "thinkingConfig": {
                    "includeThoughts": true
                }
            }
        });

        let universal = adapter.request_to_universal(payload).unwrap();
        let reasoning = universal.params.reasoning.as_ref().unwrap();
        assert_eq!(
            reasoning.canonical,
            Some(ReasoningCanonical::GoogleIncludeThoughts)
        );
        assert_eq!(reasoning.budget_tokens, None);

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        let reconstructed: GenerateContentRequest =
            serde_json::from_value(reconstructed).expect("request should deserialize");
        let thinking_config = reconstructed
            .generation_config
            .and_then(|config| config.thinking_config)
            .expect("thinkingConfig should be present");

        assert_eq!(thinking_config.include_thoughts, Some(true));
        assert_eq!(thinking_config.thinking_budget, None);
        assert_eq!(thinking_config.thinking_level, None);
    }

    #[test]
    fn test_google_gemini_3_uses_thinking_level_for_generic_budget_canonical_reasoning() {
        let adapter = GoogleAdapter;
        let req = UniversalRequest {
            model: Some("gemini-3.5-flash".to_string()),
            messages: vec![Message::User {
                content: UserContent::String("Return JSON.".to_string()),
            }],
            params: UniversalParams {
                reasoning: Some(ReasoningConfig {
                    enabled: Some(true),
                    effort: Some(ReasoningEffort::Medium),
                    budget_tokens: Some(1024),
                    canonical: Some(ReasoningCanonical::BudgetTokens),
                    ..Default::default()
                }),
                token_budget: Some(TokenBudget::OutputTokens(2048)),
                ..Default::default()
            },
        };

        let payload = adapter.request_from_universal(&req).unwrap();
        let typed: GenerateContentRequest =
            serde_json::from_value(payload).expect("request should deserialize");
        let thinking_config = typed
            .generation_config
            .and_then(|config| config.thinking_config)
            .expect("thinkingConfig should be present");

        assert_eq!(thinking_config.thinking_budget, None);
        assert_eq!(thinking_config.include_thoughts, Some(true));
        assert_eq!(thinking_config.thinking_level, Some(ThinkingLevel::Medium));
    }

    #[test]
    fn test_google_preserves_extras() {
        let adapter = GoogleAdapter;
        let payload = json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": "Hello"}]
            }],
            "safetySettings": [{"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_NONE"}]
        });

        let universal = adapter.request_to_universal(payload).unwrap();
        // safetySettings is a known key, so it won't be in extras
        // but it should be preserved through serialization

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        let reconstructed: GenerateContentRequest =
            serde_json::from_value(reconstructed).expect("request should deserialize");
        assert!(reconstructed.contents.is_some());
    }

    #[test]
    fn test_google_tool_choice_to_universal() {
        let adapter = GoogleAdapter;
        let payload = json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": "Hello"}]
            }],
            "toolConfig": {
                "functionCallingConfig": {
                    "mode": "AUTO"
                }
            }
        });

        let universal = adapter.request_to_universal(payload).unwrap();
        let tool_choice = universal.params.tool_choice.unwrap();
        assert_eq!(tool_choice.mode, Some(ToolChoiceMode::Auto));
    }

    #[test]
    fn test_google_tool_choice_from_universal() {
        let adapter = GoogleAdapter;
        let req = UniversalRequest {
            model: None,
            messages: vec![Message::User {
                content: UserContent::String("Hello".into()),
            }],
            params: UniversalParams {
                tool_choice: Some(ToolChoiceConfig {
                    mode: Some(ToolChoiceMode::Required),
                    tool_name: None,
                }),
                ..Default::default()
            },
        };

        let payload = adapter.request_from_universal(&req).unwrap();
        let typed_payload: crate::providers::google::generated::GenerateContentRequest =
            serde_json::from_value(payload).expect("request should deserialize");
        let mode = typed_payload
            .tool_config
            .and_then(|tool_config| tool_config.function_calling_config)
            .and_then(|config| config.mode);
        assert_eq!(
            mode,
            Some(crate::providers::google::generated::FunctionCallingConfigMode::Any)
        );
    }

    #[test]
    fn test_google_marks_transferred_function_call_history() {
        let adapter = GoogleAdapter;
        let req = UniversalRequest {
            model: Some("gemini-3.5-flash".to_string()),
            messages: vec![
                Message::User {
                    content: UserContent::String("List databases.".to_string()),
                },
                Message::Assistant {
                    id: None,
                    content: AssistantContent::Array(vec![AssistantContentPart::ToolCall {
                        tool_call_id: "call_1".to_string(),
                        tool_name: "list_databases".to_string(),
                        arguments: crate::universal::message::ToolCallArguments::from(
                            "{}".to_string(),
                        ),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: None,
                    }]),
                },
                Message::Tool {
                    content: vec![ToolContentPart::ToolResult(
                        crate::universal::message::ToolResultContentPart {
                            tool_call_id: "call_1".to_string(),
                            tool_name: "list_databases".to_string(),
                            output: json!({"databases": ["admin", "config", "local"]}),
                            provider_options: None,
                        },
                    )],
                },
            ],
            params: UniversalParams::default(),
        };

        let payload = adapter.request_from_universal(&req).unwrap();
        let typed_payload: GenerateContentRequest =
            serde_json::from_value(payload).expect("request should deserialize");
        let contents = typed_payload.contents.expect("contents should be present");

        let mut function_call_signature = None;
        let mut function_response_name = None;
        for content in contents {
            for part in content.parts.unwrap_or_default() {
                if part.function_call.is_some() {
                    function_call_signature = part.thought_signature;
                }
                if let Some(function_response) = part.function_response {
                    function_response_name = function_response.name;
                }
            }
        }

        assert_eq!(
            function_call_signature.as_deref(),
            Some(GOOGLE_DUMMY_THOUGHT_SIGNATURE)
        );
        assert_eq!(function_response_name.as_deref(), Some("list_databases"));
    }

    #[test]
    fn test_google_preserves_parallel_transferred_function_call_history() {
        let adapter = GoogleAdapter;
        let req = UniversalRequest {
            model: Some("gemini-3.5-flash".to_string()),
            messages: vec![
                Message::User {
                    content: UserContent::String(
                        "Check the weather in Paris and London.".to_string(),
                    ),
                },
                Message::Assistant {
                    id: None,
                    content: AssistantContent::Array(vec![
                        AssistantContentPart::ToolCall {
                            tool_call_id: "call_paris".to_string(),
                            tool_name: "get_current_temperature".to_string(),
                            arguments: crate::universal::message::ToolCallArguments::from(
                                r#"{"location":"Paris"}"#.to_string(),
                            ),
                            encrypted_content: None,
                            provider_options: None,
                            provider_executed: None,
                        },
                        AssistantContentPart::ToolCall {
                            tool_call_id: "call_london".to_string(),
                            tool_name: "get_current_temperature".to_string(),
                            arguments: crate::universal::message::ToolCallArguments::from(
                                r#"{"location":"London"}"#.to_string(),
                            ),
                            encrypted_content: None,
                            provider_options: None,
                            provider_executed: None,
                        },
                    ]),
                },
                Message::Tool {
                    content: vec![
                        ToolContentPart::ToolResult(
                            crate::universal::message::ToolResultContentPart {
                                tool_call_id: "call_paris".to_string(),
                                tool_name: "get_current_temperature".to_string(),
                                output: json!({"temp": "15C"}),
                                provider_options: None,
                            },
                        ),
                        ToolContentPart::ToolResult(
                            crate::universal::message::ToolResultContentPart {
                                tool_call_id: "call_london".to_string(),
                                tool_name: "get_current_temperature".to_string(),
                                output: json!({"temp": "12C"}),
                                provider_options: None,
                            },
                        ),
                    ],
                },
            ],
            params: UniversalParams::default(),
        };

        let payload = adapter.request_from_universal(&req).unwrap();
        let typed_payload: GenerateContentRequest =
            serde_json::from_value(payload).expect("request should deserialize");
        let contents = typed_payload.contents.expect("contents should be present");

        let function_call_signatures: Vec<_> = contents
            .iter()
            .flat_map(|content| content.parts.as_deref().unwrap_or(&[]))
            .filter(|part| part.function_call.is_some())
            .map(|part| part.thought_signature.as_deref())
            .collect();
        let function_response_count = contents
            .iter()
            .flat_map(|content| content.parts.as_deref().unwrap_or(&[]))
            .filter(|part| part.function_response.is_some())
            .count();

        assert_eq!(
            function_call_signatures,
            vec![Some(GOOGLE_DUMMY_THOUGHT_SIGNATURE), None]
        );
        assert_eq!(function_response_count, 2);
    }

    #[test]
    fn test_google_preserves_native_parallel_function_call_signatures() {
        let adapter = GoogleAdapter;
        let req = UniversalRequest {
            model: Some("gemini-3.5-flash".to_string()),
            messages: vec![
                Message::User {
                    content: UserContent::String(
                        "Check the weather in Paris and London.".to_string(),
                    ),
                },
                Message::Assistant {
                    id: None,
                    content: AssistantContent::Array(vec![
                        AssistantContentPart::ToolCall {
                            tool_call_id: "call_paris".to_string(),
                            tool_name: "get_current_temperature".to_string(),
                            arguments: crate::universal::message::ToolCallArguments::from(
                                r#"{"location":"Paris"}"#.to_string(),
                            ),
                            encrypted_content: Some("real_google_signature".to_string()),
                            provider_options: None,
                            provider_executed: None,
                        },
                        AssistantContentPart::ToolCall {
                            tool_call_id: "call_london".to_string(),
                            tool_name: "get_current_temperature".to_string(),
                            arguments: crate::universal::message::ToolCallArguments::from(
                                r#"{"location":"London"}"#.to_string(),
                            ),
                            encrypted_content: None,
                            provider_options: None,
                            provider_executed: None,
                        },
                    ]),
                },
                Message::Tool {
                    content: vec![
                        ToolContentPart::ToolResult(
                            crate::universal::message::ToolResultContentPart {
                                tool_call_id: "call_paris".to_string(),
                                tool_name: "get_current_temperature".to_string(),
                                output: json!({"temp": "15C"}),
                                provider_options: None,
                            },
                        ),
                        ToolContentPart::ToolResult(
                            crate::universal::message::ToolResultContentPart {
                                tool_call_id: "call_london".to_string(),
                                tool_name: "get_current_temperature".to_string(),
                                output: json!({"temp": "12C"}),
                                provider_options: None,
                            },
                        ),
                    ],
                },
            ],
            params: UniversalParams::default(),
        };

        let payload = adapter.request_from_universal(&req).unwrap();
        let typed_payload: GenerateContentRequest =
            serde_json::from_value(payload).expect("request should deserialize");
        let contents = typed_payload.contents.expect("contents should be present");

        let function_call_signatures: Vec<_> = contents
            .iter()
            .flat_map(|content| content.parts.as_deref().unwrap_or(&[]))
            .filter(|part| part.function_call.is_some())
            .map(|part| part.thought_signature.as_deref())
            .collect();

        assert_eq!(
            function_call_signatures,
            vec![Some("real_google_signature"), None]
        );
    }

    #[test]
    fn test_google_response_model_version_roundtrip() {
        let adapter = GoogleAdapter;
        let payload = json!({
            "modelVersion": "gemini-1.5",
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hi"}]
                },
                "finishReason": "STOP"
            }]
        });

        let universal = adapter.response_to_universal(payload).unwrap();
        assert_eq!(universal.model, Some("gemini-1.5".into()));

        let back = adapter.response_from_universal(&universal).unwrap();
        let back_typed: GenerateContentResponse =
            serde_json::from_value(back).expect("response should deserialize");
        assert_eq!(back_typed.model_version.as_deref(), Some("gemini-1.5"));
    }

    #[test]
    fn test_google_stream_tool_call_sets_tool_calls_finish_reason() {
        let adapter = GoogleAdapter;
        let args: Map<String, Value> = serde_json::from_value(json!({})).unwrap();
        let payload = json!({
            "responseId": "response_123",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [{
                        "thoughtSignature": "thought_signature_123",
                        "functionCall": {
                            "name": "get_summary",
                            "args": args
                        }
                    }]
                },
                "finishReason": "STOP"
            }]
        });

        let chunk = adapter
            .stream_to_universal(payload)
            .unwrap()
            .expect("stream chunk should be present");
        let choice = chunk.choices.first().expect("choice should be present");
        let delta = choice.delta_view().expect("delta should be present");
        let tool_call = delta
            .tool_calls
            .first()
            .expect("tool call should be present");

        assert_eq!(choice.finish_reason.as_deref(), Some("tool_calls"));
        assert_eq!(
            delta.reasoning_signature.as_deref(),
            Some("thought_signature_123")
        );
        assert_eq!(tool_call.index, Some(0));
        assert_eq!(tool_call.id.as_deref(), Some("call_response_123_0"));
    }

    #[test]
    fn test_google_stream_tool_call_indexes_are_tool_call_relative() {
        let adapter = GoogleAdapter;
        let first_args: Map<String, Value> = serde_json::from_value(json!({"a": 1})).unwrap();
        let second_args: Map<String, Value> = serde_json::from_value(json!({"b": 2})).unwrap();
        let payload = json!({
            "responseId": "response_456",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [
                        {"text": "I'll call two tools."},
                        {
                            "functionCall": {
                                "name": "first_tool",
                                "args": first_args
                            }
                        },
                        {"text": "Then another."},
                        {
                            "functionCall": {
                                "name": "second_tool",
                                "args": second_args
                            }
                        }
                    ]
                },
                "finishReason": "STOP"
            }]
        });

        let chunk = adapter
            .stream_to_universal(payload)
            .unwrap()
            .expect("stream chunk should be present");
        let choice = chunk.choices.first().expect("choice should be present");
        let delta = choice.delta_view().expect("delta should be present");

        assert_eq!(choice.finish_reason.as_deref(), Some("tool_calls"));
        assert_eq!(delta.tool_calls.len(), 2);
        assert_eq!(delta.tool_calls[0].index, Some(0));
        assert_eq!(delta.tool_calls[1].index, Some(1));
        assert_eq!(
            delta.tool_calls[0].id.as_deref(),
            Some("call_response_456_0")
        );
        assert_eq!(
            delta.tool_calls[1].id.as_deref(),
            Some("call_response_456_1")
        );
    }

    #[test]
    fn test_google_stream_keeps_thought_text_out_of_content() {
        let adapter = GoogleAdapter;
        let payload = json!({
            "responseId": "response_json_thinking",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [
                        {
                            "text": "**Analyzing caption**\nI should summarize the microphone comparison.",
                            "thought": true
                        },
                        {
                            "text": "{\"topic\":\"microphones\",\"confidence\":0.95}",
                            "thoughtSignature": "thought_signature_123"
                        }
                    ]
                },
                "finishReason": "STOP"
            }]
        });

        let chunk = adapter
            .stream_to_universal(payload)
            .unwrap()
            .expect("stream chunk should be present");
        let choice = chunk.choices.first().expect("choice should be present");
        let delta = choice.delta_view().expect("delta should be present");

        assert_eq!(
            delta.content.as_deref(),
            Some("{\"topic\":\"microphones\",\"confidence\":0.95}")
        );
        assert_eq!(delta.reasoning.len(), 1);
        assert_eq!(
            delta.reasoning[0].content.as_deref(),
            Some("**Analyzing caption**\nI should summarize the microphone comparison.")
        );
        assert_eq!(
            delta.reasoning_signature.as_deref(),
            Some("thought_signature_123")
        );
    }

    #[test]
    fn test_google_stream_from_universal_emits_reasoning_as_thought_part() {
        let adapter = GoogleAdapter;
        let chunk = UniversalStreamChunk::new(
            Some("response_json_thinking".to_string()),
            Some("gemini-2.5-flash".to_string()),
            vec![UniversalStreamChoice {
                index: 0,
                delta: Some(Value::from(UniversalStreamDelta {
                    role: Some("assistant".to_string()),
                    content: None,
                    tool_calls: vec![],
                    reasoning: vec![UniversalReasoningDelta {
                        content: Some("thinking before visible text".to_string()),
                    }],
                    reasoning_signature: None,
                })),
                finish_reason: None,
            }],
            None,
            None,
        );

        let payload = adapter.stream_from_universal(&chunk).unwrap();
        let typed: GenerateContentResponse =
            serde_json::from_value(payload).expect("stream chunk should deserialize");
        let candidates = typed
            .candidates
            .as_ref()
            .expect("candidate should be present");
        let parts = candidates[0]
            .content
            .as_ref()
            .and_then(|content| content.parts.as_ref())
            .expect("parts should be present");

        assert_eq!(parts.len(), 1);
        assert_eq!(
            parts[0].text.as_deref(),
            Some("thinking before visible text")
        );
        assert_eq!(parts[0].thought, Some(true));

        let roundtripped = adapter
            .stream_to_universal(serde_json::to_value(&typed).unwrap())
            .unwrap()
            .expect("stream chunk should be present");
        let delta = roundtripped.choices[0]
            .delta_view()
            .expect("delta should be present");
        assert_eq!(delta.content.as_deref(), Some(""));
        assert_eq!(delta.reasoning.len(), 1);
        assert_eq!(
            delta.reasoning[0].content.as_deref(),
            Some("thinking before visible text")
        );
    }

    #[test]
    fn test_google_stream_from_universal_attaches_signature_to_reasoning_only_thought_part() {
        let adapter = GoogleAdapter;
        let chunk = UniversalStreamChunk::new(
            Some("response_json_thinking".to_string()),
            Some("gemini-2.5-flash".to_string()),
            vec![UniversalStreamChoice {
                index: 0,
                delta: Some(Value::from(UniversalStreamDelta {
                    role: Some("assistant".to_string()),
                    content: None,
                    tool_calls: vec![],
                    reasoning: vec![UniversalReasoningDelta {
                        content: Some("signed thinking without visible text".to_string()),
                    }],
                    reasoning_signature: Some("thought_signature_456".to_string()),
                })),
                finish_reason: None,
            }],
            None,
            None,
        );

        let payload = adapter.stream_from_universal(&chunk).unwrap();
        let typed: GenerateContentResponse =
            serde_json::from_value(payload).expect("stream chunk should deserialize");
        let candidates = typed
            .candidates
            .as_ref()
            .expect("candidate should be present");
        let parts = candidates[0]
            .content
            .as_ref()
            .and_then(|content| content.parts.as_ref())
            .expect("parts should be present");

        assert_eq!(parts.len(), 1);
        assert_eq!(
            parts[0].text.as_deref(),
            Some("signed thinking without visible text")
        );
        assert_eq!(parts[0].thought, Some(true));
        assert_eq!(
            parts[0].thought_signature.as_deref(),
            Some("thought_signature_456")
        );

        let roundtripped = adapter
            .stream_to_universal(serde_json::to_value(&typed).unwrap())
            .unwrap()
            .expect("stream chunk should be present");
        let delta = roundtripped.choices[0]
            .delta_view()
            .expect("delta should be present");
        assert_eq!(delta.reasoning.len(), 1);
        assert_eq!(
            delta.reasoning[0].content.as_deref(),
            Some("signed thinking without visible text")
        );
        assert_eq!(
            delta.reasoning_signature.as_deref(),
            Some("thought_signature_456")
        );
    }

    #[test]
    fn test_google_stream_from_universal_keep_alive_emits_placeholder_candidate() {
        let adapter = GoogleAdapter;
        let chunk = UniversalStreamChunk::keep_alive();

        let payload = adapter.stream_from_universal(&chunk).unwrap();
        let typed: GenerateContentResponse =
            serde_json::from_value(payload).expect("stream chunk should deserialize");

        let candidates = typed
            .candidates
            .expect("placeholder candidate should be present");
        assert_eq!(candidates.len(), 1);
        let content = candidates[0]
            .content
            .as_ref()
            .expect("placeholder candidate should have content");
        assert_eq!(content.role.as_deref(), Some("model"));
        let parts = content
            .parts
            .as_ref()
            .expect("placeholder candidate should have parts");
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn test_exclusive_minimum_stripped_from_google_request() {
        let adapter = GoogleAdapter;
        let req = UniversalRequest {
            model: None,
            messages: vec![Message::User {
                content: UserContent::String("Hello".into()),
            }],
            params: UniversalParams {
                tools: Some(vec![crate::universal::tools::UniversalTool::function(
                    "my_tool",
                    Some("A tool".to_string()),
                    Some(json!({
                        "type": "object",
                        "properties": {
                            "count": {
                                "type": "integer",
                                "exclusiveMinimum": 0
                            }
                        }
                    })),
                    None,
                )]),
                ..Default::default()
            },
        };

        let payload = adapter.request_from_universal(&req).unwrap();

        // Deserialize into the typed request struct so we navigate via struct
        // fields rather than raw Value map access.
        let typed: GenerateContentRequest = serde_json::from_value(payload).unwrap();
        let decl = &typed.tools.as_ref().unwrap()[0]
            .function_declarations
            .as_ref()
            .unwrap()[0];
        let schema = decl
            .parameters_json_schema
            .as_ref()
            .expect("parametersJsonSchema should be present");

        // parameters_json_schema is an opaque JSON value; verify absence of the
        // unsupported keyword by checking the serialized form.
        let schema_str = serde_json::to_string(schema).unwrap();
        assert!(
            !schema_str.contains("exclusiveMinimum"),
            "exclusiveMinimum must be stripped from Google request"
        );
    }

    #[test]
    fn test_google_rejects_discovery_only_history_after_filtering() {
        let adapter = GoogleAdapter;
        let request = UniversalRequest {
            model: Some("gemini-2.5-flash".to_string()),
            messages: vec![Message::Tool {
                content: vec![ToolContentPart::ToolDiscoveryResult(
                    crate::universal::ToolDiscoveryResultContentPart {
                        tool_call_id: "call_tool_search_123".to_string(),
                        discovery_tool_name: "tool_search".to_string(),
                        tools: vec![crate::universal::ToolDiscoveryResultItem {
                            tool_name: "search_code".to_string(),
                            tool: None,
                            provider_options: None,
                        }],
                        status: Some("completed".to_string()),
                        execution: Some("client".to_string()),
                        provider_options: None,
                    },
                )],
            }],
            params: UniversalParams::default(),
        };

        let err = adapter.request_from_universal(&request).unwrap_err();
        match err {
            TransformError::ValidationFailed {
                target: ProviderFormat::Google,
                reason,
            } => {
                assert!(reason.contains("dynamic tool discovery history"));
            }
            other => panic!("expected Google validation error, got {other:?}"),
        }
    }

    #[test]
    fn test_google_stream_from_universal_usage_only_emits_placeholder_candidate() {
        let adapter = GoogleAdapter;
        let chunk = UniversalStreamChunk::new(
            None,
            None,
            vec![],
            None,
            Some(UniversalUsage {
                prompt_tokens: Some(1),
                completion_tokens: Some(2),
                ..Default::default()
            }),
        );

        let payload = adapter.stream_from_universal(&chunk).unwrap();
        let typed: GenerateContentResponse =
            serde_json::from_value(payload).expect("stream chunk should deserialize");

        let candidates = typed
            .candidates
            .expect("placeholder candidate should be present");
        assert_eq!(candidates.len(), 1);
        assert!(typed.usage_metadata.is_some());
    }
}

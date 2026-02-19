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
use crate::providers::google::detect::try_parse_google;
use crate::providers::google::generated::{
    Content as GoogleContent, GenerateContentResponse, GenerationConfig, ThinkingConfig,
    ThinkingLevel, Tool as GoogleTool, ToolConfig, UsageMetadata,
};
use crate::providers::google::params::GoogleParams;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{AssistantContent, AssistantContentPart, Message};
use crate::universal::request::ToolChoiceConfig;
use crate::universal::tools::UniversalTool;
use crate::universal::{
    extract_system_messages, flatten_consecutive_messages, FinishReason, TokenBudget,
    UniversalParams, UniversalRequest, UniversalResponse, UniversalStreamChoice,
    UniversalStreamChunk, UniversalStreamDelta, UniversalToolCallDelta, UniversalToolFunctionDelta,
    UniversalUsage, UserContent,
};

/// Adapter for Google AI GenerateContent API.
pub struct GoogleAdapter;

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
        let (temperature, top_p, top_k, max_tokens, stop, reasoning) = if let Some(config) =
            &typed_params.generation_config
        {
            let max_tokens = config.max_output_tokens;
            // Convert Google's thinkingConfig to ReasoningConfig
            // thinkingLevel: Gemini 3 (effort-based)
            // thinkingBudget: Gemini 2.5 (budget-based), 0 means disabled
            let reasoning = config.thinking_config.as_ref().map(|tc| {
                use crate::providers::google::capabilities::thinking_level_to_effort;

                if let Some(ref level) = tc.thinking_level {
                    // Gemini 3 style: thinkingLevel is canonical (effort-based)
                    let effort = thinking_level_to_effort(level);
                    let budget = crate::universal::reasoning::effort_to_budget(effort, max_tokens);
                    crate::universal::ReasoningConfig {
                        enabled: Some(true),
                        effort: Some(effort),
                        budget_tokens: Some(budget),
                        canonical: Some(crate::universal::ReasoningCanonical::Effort),
                        ..Default::default()
                    }
                } else {
                    // Gemini 2.5 style: thinkingBudget is canonical (budget-based)
                    let is_disabled = tc.thinking_budget == Some(0);
                    let budget_tokens = tc.thinking_budget;
                    let effort = budget_tokens
                        .map(|b| crate::universal::reasoning::budget_to_effort(b, max_tokens));
                    crate::universal::ReasoningConfig {
                        enabled: Some(!is_disabled),
                        effort,
                        budget_tokens,
                        canonical: Some(crate::universal::ReasoningCanonical::BudgetTokens),
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
        let response_format = typed_params
            .generation_config
            .as_ref()
            .map(crate::universal::request::ResponseFormatConfig::from)
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
            service_tier: None,
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

        // Flatten consecutive messages of the same role (Google doesn't allow them)
        flatten_consecutive_messages(&mut messages);

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
                use crate::providers::google::capabilities::{
                    effort_to_thinking_level, GoogleCapabilities, GoogleThinkingStyle,
                };

                if r.is_effectively_disabled() {
                    return None;
                }

                let caps = GoogleCapabilities::detect(req.model.as_deref());

                match caps.thinking_style {
                    GoogleThinkingStyle::ThinkingLevelBased => {
                        // Gemini 3: use thinkingLevel (effort-based)
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
                        // Gemini 2.5: use thinkingBudget (budget-based)
                        let budget = r
                            .budget_tokens
                            .unwrap_or(crate::universal::reasoning::MIN_THINKING_BUDGET);
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

        // Add tools if present
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
        let mut finish_reason = None;

        for candidate in response.candidates.iter().flatten() {
            if let Some(content) = &candidate.content {
                let universal = <Message as TryFromLLM<GoogleContent>>::try_from(content.clone())
                    .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                messages.push(universal);
            }

            if finish_reason.is_none() {
                finish_reason = candidate.finish_reason.as_ref().map(FinishReason::from);
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
            finish_reason
        };

        let usage = response.usage_metadata.as_ref().map(UniversalUsage::from);

        Ok(UniversalResponse {
            model: response.model_version,
            messages,
            usage,
            finish_reason,
        })
    }

    fn response_from_universal(&self, resp: &UniversalResponse) -> Result<Value, TransformError> {
        let finish_reason = resp
            .finish_reason
            .as_ref()
            .map(|r| r.to_provider_string(self.format()).to_string())
            .unwrap_or_else(|| "STOP".to_string());

        let candidates: Vec<Value> = resp
            .messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let content = <GoogleContent as TryFromLLM<Message>>::try_from(msg.clone())
                    .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

                let content_value = serde_json::to_value(&content)
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "index": i,
                    "content": content_value,
                    "finishReason": finish_reason
                }))
            })
            .collect::<Result<Vec<_>, TransformError>>()?;

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

            let text = parts
                .iter()
                .filter_map(|part| part.text.as_deref())
                .collect::<Vec<_>>()
                .join("");

            let tool_calls = parts
                .iter()
                .enumerate()
                .filter_map(|(i, part)| {
                    part.function_call
                        .as_ref()
                        .map(|function_call| UniversalToolCallDelta {
                            index: Some(i as u32),
                            id: function_call.id.clone(),
                            call_type: Some("function".to_string()),
                            function: Some(UniversalToolFunctionDelta {
                                name: function_call.name.clone(),
                                arguments: function_call
                                    .args
                                    .as_ref()
                                    .map(|args| Value::Object(args.clone()).to_string()),
                            }),
                        })
                })
                .collect();

            // Map finish reason using centralized helper
            let finish_reason = candidate
                .finish_reason
                .as_ref()
                .and_then(|reason| serde_json::to_value(reason).ok())
                .and_then(|reason| match reason {
                    Value::String(s) => Some(s),
                    _ => None,
                })
                .map(|reason| {
                    FinishReason::from_provider_string(&reason, self.format()).to_string()
                });

            let delta = UniversalStreamDelta {
                role: Some("assistant".to_string()),
                content: Some(text),
                tool_calls,
                reasoning: vec![],
                reasoning_signature: None,
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
        if chunk.is_keep_alive() {
            // Google doesn't have a keep-alive event, return empty candidates
            return Ok(serde_json::json!({
                "candidates": []
            }));
        }

        let candidates: Vec<Value> = chunk
            .choices
            .iter()
            .map(|c| {
                let delta = c.delta_view();

                // Build parts array from text and tool_calls
                let mut parts: Vec<Value> = Vec::new();

                // Add text part if present
                let text = delta
                    .as_ref()
                    .and_then(|d| d.content.as_deref())
                    .unwrap_or("");
                if !text.is_empty() {
                    parts.push(serde_json::json!({"text": text}));
                }

                // Add functionCall parts from tool_calls
                if let Some(ref d) = delta {
                    for tc in &d.tool_calls {
                        if let Some(ref func) = tc.function {
                            let mut fc_map = serde_json::Map::new();
                            if let Some(ref name) = func.name {
                                fc_map.insert("name".into(), Value::String(name.clone()));
                            }
                            if let Some(ref id) = tc.id {
                                fc_map.insert("id".into(), Value::String(id.clone()));
                            }
                            if let Some(ref args) = func.arguments {
                                if args.is_empty() {
                                    fc_map.insert("args".into(), serde_json::json!({}));
                                } else if let Ok(args_val) = serde_json::from_str::<Value>(args) {
                                    fc_map.insert("args".into(), args_val);
                                }
                            }
                            parts.push(serde_json::json!({"functionCall": fc_map}));
                        }
                    }
                }

                // Ensure at least one empty text part if no parts
                if parts.is_empty() {
                    parts.push(serde_json::json!({"text": ""}));
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

                Value::Object(candidate_map)
            })
            .collect();

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;
    use crate::universal::request::ToolChoiceMode;

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
        assert!(reconstructed.get("contents").is_some());
        assert!(reconstructed.get("generationConfig").is_some());
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
        assert!(reconstructed.get("contents").is_some());
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
                    disable_parallel: None,
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
}

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
use crate::providers::google::convert::{
    apply_response_format_to_generation_config, response_format_from_generation_config,
};
use crate::providers::google::detect::try_parse_google;
use crate::providers::google::generated::{
    Content as GoogleContent, GenerateContentResponse, GenerationConfig, ThinkingConfig,
    Tool as GoogleTool, ToolConfig, UsageMetadata,
};
use crate::providers::google::params::GoogleParams;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::Message;
use crate::universal::request::ToolChoiceConfig;
use crate::universal::tools::UniversalTool;
use crate::universal::{
    extract_system_messages, flatten_consecutive_messages, FinishReason, TokenBudget,
    UniversalParams, UniversalRequest, UniversalResponse, UniversalStreamChoice,
    UniversalStreamChunk, UniversalUsage, UserContent,
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
        let (
            temperature,
            top_p,
            top_k,
            max_tokens,
            stop,
            reasoning,
            seed,
            presence_penalty,
            frequency_penalty,
        ) = if let Some(config) = &typed_params.generation_config {
            let max_tokens = config.max_output_tokens;
            // Convert Google's thinkingConfig to ReasoningConfig
            // thinkingBudget: 0 means disabled
            let reasoning = config.thinking_config.as_ref().map(|tc| {
                let is_disabled = tc.thinking_budget == Some(0);
                let budget_tokens = tc.thinking_budget;
                // Derive effort from budget_tokens
                let effort =
                    budget_tokens.map(|b| crate::universal::reasoning::budget_to_effort(b, None));
                crate::universal::ReasoningConfig {
                    enabled: Some(!is_disabled),
                    effort,
                    budget_tokens,
                    canonical: Some(crate::universal::ReasoningCanonical::BudgetTokens),
                    ..Default::default()
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
                config.seed,
                config.presence_penalty,
                config.frequency_penalty,
            )
        } else {
            (None, None, None, None, None, None, None, None, None)
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
            .and_then(response_format_from_generation_config);

        let mut params = UniversalParams {
            temperature,
            top_p,
            top_k,
            token_budget: max_tokens.map(TokenBudget::OutputTokens),
            stop,
            tools,
            tool_choice,
            response_format,
            seed,
            presence_penalty,
            frequency_penalty,
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

        // Collect Google-specific extras: serde-flatten unknowns + known fields
        // that don't map to universal params.
        let mut google_extras: Map<String, Value> = typed_params.extras.into_iter().collect();

        if let Some(v) = typed_params.safety_settings {
            google_extras.insert("safetySettings".into(), v);
        }
        if let Some(v) = typed_params.cached_content {
            google_extras.insert("cachedContent".into(), Value::String(v));
        }

        // Preserve generationConfig fields that don't have universal equivalents.
        // Serialize the whole config, strip fields we already handle, keep the rest.
        if let Some(config) = &typed_params.generation_config {
            if let Ok(Value::Object(mut config_map)) = serde_json::to_value(config) {
                // Remove fields handled canonically above
                for key in &[
                    "temperature",
                    "topP",
                    "topK",
                    "maxOutputTokens",
                    "stopSequences",
                    "thinkingConfig",
                    "responseMimeType",
                    "responseSchema",
                    "seed",
                    "presencePenalty",
                    "frequencyPenalty",
                ] {
                    config_map.remove(*key);
                }
                // Remove null entries
                config_map.retain(|_, v| !v.is_null());
                if !config_map.is_empty() {
                    google_extras
                        .insert("_generationConfigExtras".into(), Value::Object(config_map));
                }
            }
        }

        if !google_extras.is_empty() {
            params.extras.insert(ProviderFormat::Google, google_extras);
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
        let has_response_format = req.params.response_format.is_some();
        let has_gen_config_extras = req
            .params
            .extras
            .get(&ProviderFormat::Google)
            .and_then(|e| e.get("_generationConfigExtras"))
            .is_some();
        let has_params = req.params.temperature.is_some()
            || req.params.top_p.is_some()
            || req.params.top_k.is_some()
            || req.params.output_token_budget().is_some()
            || req.params.stop.is_some()
            || req.params.seed.is_some()
            || req.params.presence_penalty.is_some()
            || req.params.frequency_penalty.is_some()
            || has_reasoning
            || has_response_format
            || has_gen_config_extras;

        if has_params {
            // Convert ReasoningConfig to Google's thinkingConfig
            let thinking_config = req.params.reasoning.as_ref().and_then(|r| {
                if r.is_effectively_disabled() {
                    return None;
                }
                // Use budget_tokens or default minimum
                let budget = r
                    .budget_tokens
                    .unwrap_or(crate::universal::reasoning::MIN_THINKING_BUDGET);
                Some(ThinkingConfig {
                    include_thoughts: Some(true),
                    thinking_budget: Some(budget),
                    ..Default::default()
                })
            });

            let stop_sequences = req.params.stop.clone();

            let mut config = GenerationConfig {
                temperature: req.params.temperature,
                top_p: req.params.top_p,
                top_k: req.params.top_k,
                max_output_tokens: req.params.output_token_budget(),
                stop_sequences,
                thinking_config,
                seed: req.params.seed,
                presence_penalty: req.params.presence_penalty,
                frequency_penalty: req.params.frequency_penalty,
                ..Default::default()
            };

            // Apply response format to generationConfig
            if let Some(format) = &req.params.response_format {
                apply_response_format_to_generation_config(&mut config, format);
            }

            let mut config_value = serde_json::to_value(config)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

            // Merge back generationConfig extras (candidateCount, speechConfig, etc.)
            if let Some(extras) = req.params.extras.get(&ProviderFormat::Google) {
                if let Some(Value::Object(config_extras)) = extras.get("_generationConfigExtras") {
                    if let Some(config_map) = config_value.as_object_mut() {
                        for (k, v) in config_extras {
                            if !config_map.contains_key(k) {
                                config_map.insert(k.clone(), v.clone());
                            }
                        }
                    }
                }
            }

            obj.insert("generationConfig".into(), config_value);
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
                // _generationConfigExtras is merged into generationConfig above
                if k == "_generationConfigExtras" {
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

    fn detect_response(&self, payload: &Value) -> bool {
        // Google response has candidates[].content structure
        payload
            .get("candidates")
            .and_then(Value::as_array)
            .is_some_and(|arr| arr.first().and_then(|c| c.get("content")).is_some())
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
        let candidates = payload
            .get("candidates")
            .and_then(Value::as_array)
            .ok_or_else(|| TransformError::ToUniversalFailed("missing candidates".to_string()))?;

        let mut choices = Vec::new();

        for candidate in candidates {
            let index = candidate.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;

            // Extract text from content.parts
            let text: String = candidate
                .get("content")
                .and_then(|c| c.get("parts"))
                .and_then(Value::as_array)
                .map(|parts| {
                    parts
                        .iter()
                        .filter_map(|p| p.get("text").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join("")
                })
                .unwrap_or_default();

            // Map finish reason using centralized helper
            let finish_reason = candidate
                .get("finishReason")
                .and_then(Value::as_str)
                .map(|r| FinishReason::from_provider_string(r, self.format()).to_string());

            choices.push(UniversalStreamChoice {
                index,
                delta: Some(serde_json::json!({
                    "role": "assistant",
                    "content": text
                })),
                finish_reason,
            });
        }

        let usage = payload
            .get("usageMetadata")
            .map(|v| serde_json::from_value::<UsageMetadata>(v.clone()))
            .transpose()
            .map_err(|e| TransformError::ToUniversalFailed(format!("usageMetadata: {e}")))?
            .map(|u| UniversalUsage::from(&u));

        let model = payload
            .get("modelVersion")
            .and_then(Value::as_str)
            .map(String::from);

        let id = payload
            .get("responseId")
            .and_then(Value::as_str)
            .map(String::from);

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
                // Extract text content from delta
                let text = c
                    .delta
                    .as_ref()
                    .and_then(|d| d.get("content"))
                    .and_then(Value::as_str)
                    .unwrap_or("");

                let finish_reason = c.finish_reason.as_ref().map(|r| {
                    let fr: FinishReason = r.parse().unwrap_or(FinishReason::Other(r.clone()));
                    fr.to_provider_string(self.format()).to_string()
                });

                let mut candidate_map = serde_json::Map::new();
                candidate_map.insert("index".into(), serde_json::json!(c.index));
                candidate_map.insert(
                    "content".into(),
                    serde_json::json!({
                        "parts": [{"text": text}],
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
        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert!(reconstructed.get("contents").is_some());
        // safetySettings should survive the roundtrip
        assert_eq!(
            reconstructed.get("safetySettings"),
            Some(&json!([{"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_NONE"}]))
        );
    }

    #[test]
    fn test_google_roundtrip_generation_config_extras() {
        let adapter = GoogleAdapter;
        let payload = json!({
            "contents": [{"role": "user", "parts": [{"text": "hi"}]}],
            "generationConfig": {
                "temperature": 0.5,
                "seed": 42,
                "presencePenalty": 0.3,
                "frequencyPenalty": 0.7,
                "candidateCount": 2,
                "responseLogprobs": true,
                "responseModalities": ["TEXT"],
                "mediaResolution": "MEDIA_RESOLUTION_LOW"
            }
        });

        let universal = adapter.request_to_universal(payload).unwrap();
        assert_eq!(universal.params.seed, Some(42));
        assert!((universal.params.presence_penalty.unwrap() - 0.3).abs() < 0.001);
        assert!((universal.params.frequency_penalty.unwrap() - 0.7).abs() < 0.001);

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        let config = reconstructed.get("generationConfig").unwrap();
        assert_eq!(config.get("seed"), Some(&json!(42)));
        assert_eq!(config.get("candidateCount"), Some(&json!(2)));
        assert_eq!(config.get("responseLogprobs"), Some(&json!(true)));
        assert_eq!(config.get("responseModalities"), Some(&json!(["TEXT"])));
        assert_eq!(
            config.get("mediaResolution"),
            Some(&json!("MEDIA_RESOLUTION_LOW"))
        );
    }

    #[test]
    fn test_google_roundtrip_cached_content() {
        let adapter = GoogleAdapter;
        let payload = json!({
            "contents": [{"role": "user", "parts": [{"text": "hi"}]}],
            "cachedContent": "cachedContents/abc123"
        });

        let universal = adapter.request_to_universal(payload).unwrap();
        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert_eq!(
            reconstructed.get("cachedContent"),
            Some(&json!("cachedContents/abc123"))
        );
    }

    #[test]
    fn test_google_openai_google_roundtrip_seed_and_penalties() {
        use crate::processing::adapters::adapter_for_format;

        let google = adapter_for_format(ProviderFormat::Google).unwrap();
        let openai = adapter_for_format(ProviderFormat::ChatCompletions).unwrap();

        let google_payload = json!({
            "model": "gemini-2.0-flash",
            "contents": [{"role": "user", "parts": [{"text": "hello"}]}],
            "generationConfig": {
                "seed": 42,
                "presencePenalty": 0.5,
                "frequencyPenalty": 0.8,
                "candidateCount": 2,
                "responseModalities": ["TEXT"],
                "temperature": 0.7
            },
            "safetySettings": [{"category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_ONLY_HIGH"}],
            "cachedContent": "cachedContents/test123"
        });

        // Google -> Universal
        let universal = google.request_to_universal(google_payload.clone()).unwrap();
        assert_eq!(universal.params.seed, Some(42));
        assert!((universal.params.presence_penalty.unwrap() - 0.5).abs() < 0.001);
        assert!((universal.params.frequency_penalty.unwrap() - 0.8).abs() < 0.001);

        // Universal -> OpenAI ChatCompletions
        let openai_payload = openai.request_from_universal(&universal).unwrap();
        // seed, presence_penalty, frequency_penalty should be in OpenAI format
        assert_eq!(openai_payload.get("seed"), Some(&json!(42)));

        // OpenAI -> Universal (back)
        let universal_2 = openai.request_to_universal(openai_payload).unwrap();
        assert_eq!(universal_2.params.seed, Some(42));

        // Universal -> Google (back)
        let google_out = google.request_from_universal(&universal_2).unwrap();
        let config = google_out.get("generationConfig").unwrap();
        // Universal params survive cross-provider roundtrip
        assert_eq!(config.get("seed"), Some(&json!(42)));
        assert!(config.get("presencePenalty").is_some());
        assert!(config.get("frequencyPenalty").is_some());
        assert!(config.get("temperature").is_some());

        // Google-specific extras (candidateCount, responseModalities, safetySettings,
        // cachedContent) are stored under ProviderFormat::Google in extras, so they
        // survive a Google->Google roundtrip but NOT a cross-provider trip through OpenAI.
        // This is expected: OpenAI doesn't know about Google's candidateCount etc.
    }
}

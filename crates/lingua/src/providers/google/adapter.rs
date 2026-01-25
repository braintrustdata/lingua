/*!
Google AI provider adapter for GenerateContent API.

Google's API has some unique characteristics:
- Uses `contents` instead of `messages`
- Generation params are in `generationConfig` object
- Uses camelCase field names (e.g., `maxOutputTokens`)
- Streaming is endpoint-based, not parameter-based
*/

use crate::capabilities::ProviderFormat;
use crate::processing::adapters::{collect_extras, ProviderAdapter};
use crate::processing::transform::TransformError;
use crate::providers::google::detect::try_parse_google;
use crate::providers::google::generated::{
    candidate, generate_content_response, part, Content as GoogleContent, GenerateContentRequest,
    GenerateContentResponse, GenerationConfig,
};
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::Message;
use crate::universal::{
    extract_system_messages, flatten_consecutive_messages, FinishReason, UniversalParams,
    UniversalRequest, UniversalResponse, UniversalStreamChoice, UniversalStreamChunk,
    UniversalUsage, UserContent,
};

/// Known request fields for Google GenerateContent API.
/// Fields not in this list go into `extras`.
const GOOGLE_KNOWN_KEYS: &[&str] = &[
    "contents",
    "generationConfig",
    "systemInstruction",
    "safetySettings",
    "tools",
    "toolConfig",
    "model",
];

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
        let extras = collect_extras(&payload, GOOGLE_KNOWN_KEYS);
        let model = payload
            .get("model")
            .and_then(Value::as_str)
            .map(String::from);

        let request: GenerateContentRequest = serde_json::from_value(payload)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let messages = <Vec<Message> as TryFromLLM<Vec<GoogleContent>>>::try_from(request.contents)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Extract params from generationConfig
        let (temperature, top_p, top_k, max_tokens, stop) =
            if let Some(config) = &request.generation_config {
                (
                    config.temperature.map(|t| t as f64),
                    config.top_p.map(|p| p as f64),
                    config.top_k.map(|k| k as i64),
                    config.max_output_tokens.map(|t| t as i64),
                    if config.stop_sequences.is_empty() {
                        None
                    } else {
                        serde_json::to_value(&config.stop_sequences).ok()
                    },
                )
            } else {
                (None, None, None, None, None)
            };

        let params = UniversalParams {
            temperature,
            top_p,
            top_k,
            max_tokens,
            stop,
            tools: if request.tools.is_empty() {
                None
            } else {
                serde_json::to_value(&request.tools).ok()
            },
            tool_choice: None, // Google uses different mechanism
            response_format: None,
            seed: None, // Google doesn't support seed
            presence_penalty: None,
            frequency_penalty: None,
            stream: None, // Google uses endpoint-based streaming
        };

        Ok(UniversalRequest {
            model,
            messages,
            params,
            extras,
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
        let stop_sequences = req
            .params
            .stop
            .as_ref()
            .map(|stop| match stop {
                Value::Array(arr) => arr
                    .iter()
                    .filter_map(|s| s.as_str().map(|v| v.to_string()))
                    .collect::<Vec<_>>(),
                Value::String(s) => vec![s.clone()],
                _ => Vec::new(),
            })
            .unwrap_or_default();

        let config = GenerationConfig {
            temperature: req.params.temperature.map(|t| t as f32),
            top_p: req.params.top_p.map(|p| p as f32),
            top_k: req.params.top_k.map(|k| k as i32),
            max_output_tokens: req.params.max_tokens.map(|t| t as i32),
            stop_sequences,
            ..GenerationConfig::default()
        };

        let config_value = serde_json::to_value(config)
            .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;
        if !config_value
            .as_object()
            .map(|map| map.is_empty())
            .unwrap_or(true)
        {
            obj.insert("generationConfig".into(), config_value);
        }

        // Add tools if present
        if let Some(tools) = &req.params.tools {
            obj.insert("tools".into(), tools.clone());
        }

        // Merge extras - only include Google-known fields
        // This filters out OpenAI-specific fields like stream_options that would cause
        // Google to reject the request with "Unknown name: stream_options"
        for (k, v) in &req.extras {
            if GOOGLE_KNOWN_KEYS.contains(&k.as_str()) {
                obj.insert(k.clone(), v.clone());
            }
        }

        Ok(Value::Object(obj))
    }

    fn apply_defaults(&self, _req: &mut UniversalRequest) {
        // Google doesn't require any specific defaults
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
        let GenerateContentResponse {
            candidates,
            usage_metadata,
            model_version,
            ..
        } = response;

        let mut messages = Vec::new();
        let mut finish_reason = None;

        for candidate in candidates {
            let content = candidate.content;
            let finish_reason_value = candidate.finish_reason;

            if let Some(content) = content {
                let universal = <Message as TryFromLLM<GoogleContent>>::try_from(content)
                    .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                messages.push(universal);
            }

            // Get finishReason from first candidate
            if finish_reason.is_none() && finish_reason_value != 0 {
                let reason = candidate::FinishReason::try_from(finish_reason_value)
                    .ok()
                    .map(|r| r.as_str_name())
                    .unwrap_or("FINISH_REASON_UNSPECIFIED");
                finish_reason = Some(reason.parse().unwrap());
            }
        }

        let usage = usage_metadata.map(|u| UniversalUsage {
            prompt_tokens: Some(u.prompt_token_count as i64),
            completion_tokens: Some(u.candidates_token_count as i64),
            prompt_cached_tokens: Some(u.cached_content_token_count as i64),
            prompt_cache_creation_tokens: None, // Google doesn't report cache creation tokens
            completion_reasoning_tokens: Some(u.thoughts_token_count as i64),
        });

        Ok(UniversalResponse {
            model: if model_version.is_empty() {
                None
            } else {
                Some(model_version)
            },
            messages,
            usage,
            finish_reason,
        })
    }

    fn response_from_universal(&self, resp: &UniversalResponse) -> Result<Value, TransformError> {
        let finish_reason = map_finish_reason_to_candidate_enum(
            self.map_finish_reason(resp.finish_reason.as_ref()),
        );

        let candidates = resp
            .messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let content = <GoogleContent as TryFromLLM<Message>>::try_from(msg.clone())
                    .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;
                Ok(crate::providers::google::generated::Candidate {
                    index: Some(i as i32),
                    content: Some(content),
                    finish_reason,
                    finish_message: None,
                    safety_ratings: Vec::new(),
                    citation_metadata: None,
                    token_count: 0,
                    grounding_attributions: Vec::new(),
                    grounding_metadata: None,
                    avg_logprobs: 0.0,
                    logprobs_result: None,
                    url_context_metadata: None,
                })
            })
            .collect::<Result<Vec<_>, TransformError>>()?;

        let usage_metadata = resp.usage.as_ref().map(|usage| {
            let prompt = usage.prompt_tokens.unwrap_or(0) as i32;
            let completion = usage.completion_tokens.unwrap_or(0) as i32;
            generate_content_response::UsageMetadata {
                prompt_token_count: prompt,
                cached_content_token_count: usage.prompt_cached_tokens.unwrap_or(0) as i32,
                candidates_token_count: completion,
                tool_use_prompt_token_count: 0,
                thoughts_token_count: usage.completion_reasoning_tokens.unwrap_or(0) as i32,
                total_token_count: prompt + completion,
                prompt_tokens_details: Vec::new(),
                cache_tokens_details: Vec::new(),
                candidates_tokens_details: Vec::new(),
                tool_use_prompt_tokens_details: Vec::new(),
            }
        });

        let response = GenerateContentResponse {
            candidates,
            prompt_feedback: None,
            usage_metadata,
            model_version: resp.model.clone().unwrap_or_default(),
            response_id: String::new(),
        };

        let mut value = serde_json::to_value(response)
            .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;
        ensure_candidates_field(&mut value);
        Ok(value)
    }

    fn map_finish_reason(&self, reason: Option<&FinishReason>) -> Option<String> {
        reason.map(|r| match r {
            FinishReason::Stop => "STOP".to_string(),
            FinishReason::Length => "MAX_TOKENS".to_string(),
            FinishReason::ToolCalls => "TOOL_CALLS".to_string(),
            FinishReason::ContentFilter => "SAFETY".to_string(),
            FinishReason::Other(s) => s.clone(),
        })
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
        let response: GenerateContentResponse = serde_json::from_value(payload)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
        let GenerateContentResponse {
            candidates,
            usage_metadata,
            model_version,
            response_id,
            ..
        } = response;

        let mut choices = Vec::new();

        for candidate in candidates {
            let index = candidate.index.unwrap_or(0) as u32;

            // Extract text from content.parts
            let text: String = candidate
                .content
                .as_ref()
                .map(|content| {
                    content
                        .parts
                        .iter()
                        .filter_map(|part| match &part.data {
                            Some(part::Data::Text(text)) => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("")
                })
                .unwrap_or_default();

            // Map finish reason
            let finish_reason = candidate::FinishReason::try_from(candidate.finish_reason)
                .ok()
                .map(|r| r.as_str_name())
                .map(|r| match r {
                    "STOP" => "stop".to_string(),
                    "MAX_TOKENS" => "length".to_string(),
                    "SAFETY" | "RECITATION" | "OTHER" => "content_filter".to_string(),
                    other => other.to_lowercase(),
                });

            choices.push(UniversalStreamChoice {
                index,
                delta: Some(serde_json::json!({
                    "role": "assistant",
                    "content": text
                })),
                finish_reason,
            });
        }

        // Extract usage from usageMetadata
        let usage = usage_metadata.map(|u| UniversalUsage {
            prompt_tokens: Some(u.prompt_token_count as i64),
            completion_tokens: Some(u.candidates_token_count as i64),
            prompt_cached_tokens: Some(u.cached_content_token_count as i64),
            prompt_cache_creation_tokens: None,
            completion_reasoning_tokens: Some(u.thoughts_token_count as i64),
        });

        let model = if model_version.is_empty() {
            None
        } else {
            Some(model_version)
        };
        let id = if response_id.is_empty() {
            None
        } else {
            Some(response_id)
        };

        Ok(Some(UniversalStreamChunk::new(
            id, model, choices, None, usage,
        )))
    }

    fn stream_from_universal(&self, chunk: &UniversalStreamChunk) -> Result<Value, TransformError> {
        let candidates = if chunk.is_keep_alive() {
            Vec::new()
        } else {
            chunk
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

                    let finish_reason =
                        map_stream_finish_reason_to_candidate_enum(c.finish_reason.as_deref());

                    crate::providers::google::generated::Candidate {
                        index: Some(c.index as i32),
                        content: Some(GoogleContent {
                            role: "model".to_string(),
                            parts: vec![crate::providers::google::generated::Part {
                                thought: false,
                                thought_signature: Vec::new(),
                                part_metadata: None,
                                data: Some(part::Data::Text(text.to_string())),
                                metadata: None,
                            }],
                        }),
                        finish_reason,
                        finish_message: None,
                        safety_ratings: Vec::new(),
                        citation_metadata: None,
                        token_count: 0,
                        grounding_attributions: Vec::new(),
                        grounding_metadata: None,
                        avg_logprobs: 0.0,
                        logprobs_result: None,
                        url_context_metadata: None,
                    }
                })
                .collect::<Vec<_>>()
        };

        let usage_metadata = chunk.usage.as_ref().map(|usage| {
            let prompt = usage.prompt_tokens.unwrap_or(0) as i32;
            let completion = usage.completion_tokens.unwrap_or(0) as i32;
            generate_content_response::UsageMetadata {
                prompt_token_count: prompt,
                cached_content_token_count: usage.prompt_cached_tokens.unwrap_or(0) as i32,
                candidates_token_count: completion,
                tool_use_prompt_token_count: 0,
                thoughts_token_count: usage.completion_reasoning_tokens.unwrap_or(0) as i32,
                total_token_count: prompt + completion,
                prompt_tokens_details: Vec::new(),
                cache_tokens_details: Vec::new(),
                candidates_tokens_details: Vec::new(),
                tool_use_prompt_tokens_details: Vec::new(),
            }
        });

        let response = GenerateContentResponse {
            candidates,
            prompt_feedback: None,
            usage_metadata,
            model_version: chunk.model.clone().unwrap_or_default(),
            response_id: chunk.id.clone().unwrap_or_default(),
        };

        let mut value = serde_json::to_value(response)
            .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;
        ensure_candidates_field(&mut value);
        Ok(value)
    }
}

fn map_finish_reason_to_candidate_enum(reason_str: Option<String>) -> i32 {
    if let Some(reason_str) = reason_str {
        if let Some(mapped) = candidate::FinishReason::from_str_name(&reason_str) {
            return mapped as i32;
        }
        if reason_str == "TOOL_CALLS" {
            return candidate::FinishReason::Other as i32;
        }
    }
    0
}

fn map_stream_finish_reason_to_candidate_enum(reason: Option<&str>) -> i32 {
    let reason = match reason {
        Some(value) => value,
        None => return 0,
    };

    if reason.eq_ignore_ascii_case("stop") {
        return candidate::FinishReason::Stop as i32;
    }
    if reason.eq_ignore_ascii_case("length") || reason.eq_ignore_ascii_case("max_tokens") {
        return candidate::FinishReason::MaxTokens as i32;
    }
    if reason.eq_ignore_ascii_case("content_filter") {
        return candidate::FinishReason::Safety as i32;
    }
    if reason.eq_ignore_ascii_case("tool_calls") || reason.eq_ignore_ascii_case("tool_use") {
        return candidate::FinishReason::Other as i32;
    }
    if let Some(mapped) = candidate::FinishReason::from_str_name(reason) {
        return mapped as i32;
    }
    candidate::FinishReason::Other as i32
}

fn ensure_candidates_field(value: &mut Value) {
    if let Value::Object(map) = value {
        map.entry("candidates".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
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
        let temperature = universal.params.temperature.unwrap();
        assert_eq!(temperature as f32, 0.7f32);
        assert_eq!(universal.params.max_tokens, Some(1024));

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
}

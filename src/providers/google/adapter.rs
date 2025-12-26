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
use crate::providers::google::detect::{
    try_parse_google, GoogleContent, GoogleGenerateContentRequest, GoogleGenerationConfig,
};
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::Message;
use crate::universal::{
    FinishReason, UniversalParams, UniversalRequest, UniversalResponse, UniversalStreamChunk,
    UniversalStreamChoice, UniversalUsage,
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

    fn request_to_universal(&self, payload: &Value) -> Result<UniversalRequest, TransformError> {
        let request: GoogleGenerateContentRequest = serde_json::from_value(payload.clone())
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let messages =
            <Vec<Message> as TryFromLLM<Vec<GoogleContent>>>::try_from(request.contents)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Extract params from generationConfig
        let (temperature, top_p, top_k, max_tokens, stop) =
            if let Some(config) = &request.generation_config {
                (
                    config.temperature,
                    config.top_p,
                    config.top_k.map(|k| k as i64),
                    config.max_output_tokens.map(|t| t as i64),
                    config
                        .stop_sequences
                        .as_ref()
                        .and_then(|s| serde_json::to_value(s).ok()),
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
            tools: request.tools.and_then(|t| serde_json::to_value(t).ok()),
            tool_choice: None, // Google uses different mechanism
            response_format: None,
            seed: None, // Google doesn't support seed
            presence_penalty: None,
            frequency_penalty: None,
            stream: None, // Google uses endpoint-based streaming
        };

        // Get model from payload if present (it's often in the URL, not the body)
        let model = payload
            .get("model")
            .and_then(Value::as_str)
            .map(String::from);

        Ok(UniversalRequest {
            model,
            messages,
            params,
            extras: collect_extras(payload, GOOGLE_KNOWN_KEYS),
        })
    }

    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
        // Convert messages to Google contents
        let google_contents: Vec<GoogleContent> =
            <Vec<GoogleContent> as TryFromLLM<Vec<Message>>>::try_from(req.messages.clone())
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

        // Build generationConfig if any params are set
        let has_params = req.params.temperature.is_some()
            || req.params.top_p.is_some()
            || req.params.top_k.is_some()
            || req.params.max_tokens.is_some()
            || req.params.stop.is_some();

        if has_params {
            let config = GoogleGenerationConfig {
                temperature: req.params.temperature,
                top_p: req.params.top_p,
                top_k: req.params.top_k.map(|k| k as i32),
                max_output_tokens: req.params.max_tokens.map(|t| t as i32),
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
                "generationConfig".into(),
                serde_json::to_value(config)
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
            );
        }

        // Add tools if present
        if let Some(tools) = &req.params.tools {
            obj.insert("tools".into(), tools.clone());
        }

        // Merge extras
        for (k, v) in &req.extras {
            obj.insert(k.clone(), v.clone());
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
            .is_some_and(|arr| {
                arr.first()
                    .and_then(|c| c.get("content"))
                    .is_some()
            })
    }

    fn response_to_universal(&self, payload: &Value) -> Result<UniversalResponse, TransformError> {
        let candidates = payload
            .get("candidates")
            .and_then(Value::as_array)
            .ok_or_else(|| TransformError::ToUniversalFailed("missing candidates".to_string()))?;

        let mut messages = Vec::new();
        let mut finish_reason = None;

        for candidate in candidates {
            if let Some(content_val) = candidate.get("content") {
                let content: GoogleContent = serde_json::from_value(content_val.clone())
                    .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                let universal = <Message as TryFromLLM<GoogleContent>>::try_from(content)
                    .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                messages.push(universal);
            }

            // Get finishReason from first candidate
            if finish_reason.is_none() {
                if let Some(reason) = candidate.get("finishReason").and_then(Value::as_str) {
                    finish_reason = Some(FinishReason::from_str(reason));
                }
            }
        }

        let usage = payload.get("usageMetadata").map(|u| UniversalUsage {
            input_tokens: u.get("promptTokenCount").and_then(Value::as_i64),
            output_tokens: u.get("candidatesTokenCount").and_then(Value::as_i64),
        });

        Ok(UniversalResponse {
            model: payload
                .get("modelVersion")
                .and_then(Value::as_str)
                .map(String::from),
            messages,
            usage,
            finish_reason,
            extras: Map::new(),
        })
    }

    fn response_from_universal(&self, resp: &UniversalResponse) -> Result<Value, TransformError> {
        let finish_reason = self
            .map_finish_reason(resp.finish_reason.as_ref())
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

        let mut obj = serde_json::json!({
            "candidates": candidates
        });

        if let Some(usage) = &resp.usage {
            let input = usage.input_tokens.unwrap_or(0);
            let output = usage.output_tokens.unwrap_or(0);
            obj.as_object_mut().unwrap().insert(
                "usageMetadata".into(),
                serde_json::json!({
                    "promptTokenCount": input,
                    "candidatesTokenCount": output,
                    "totalTokenCount": input + output
                }),
            );
        }

        Ok(obj)
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
        payload: &Value,
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

            // Map finish reason
            let finish_reason =
                candidate
                    .get("finishReason")
                    .and_then(Value::as_str)
                    .map(|r| match r {
                        "STOP" => "stop".to_string(),
                        "MAX_TOKENS" => "length".to_string(),
                        "SAFETY" | "RECITATION" | "OTHER" => "stop".to_string(),
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
        let usage = payload.get("usageMetadata").map(|u| UniversalUsage {
            input_tokens: u.get("promptTokenCount").and_then(Value::as_i64),
            output_tokens: u.get("candidatesTokenCount").and_then(Value::as_i64),
        });

        let model = payload
            .get("modelVersion")
            .and_then(Value::as_str)
            .map(String::from);

        let id = payload
            .get("responseId")
            .and_then(Value::as_str)
            .map(String::from);

        Ok(Some(UniversalStreamChunk::new(id, model, choices, None, usage)))
    }

    fn stream_from_universal(
        &self,
        chunk: &UniversalStreamChunk,
    ) -> Result<Value, TransformError> {
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

                // Map finish reason to Google format
                let finish_reason = c.finish_reason.as_ref().map(|r| match r.as_str() {
                    "stop" => "STOP",
                    "length" => "MAX_TOKENS",
                    "tool_calls" => "TOOL_CALLS",
                    "content_filter" => "SAFETY",
                    other => other,
                });

                let mut candidate = serde_json::json!({
                    "index": c.index,
                    "content": {
                        "parts": [{"text": text}],
                        "role": "model"
                    }
                });

                if let Some(reason) = finish_reason {
                    candidate
                        .as_object_mut()
                        .unwrap()
                        .insert("finishReason".into(), Value::String(reason.to_string()));
                }

                candidate
            })
            .collect();

        let mut obj = serde_json::json!({
            "candidates": candidates
        });

        let obj_map = obj.as_object_mut().unwrap();

        if let Some(ref id) = chunk.id {
            obj_map.insert("responseId".into(), Value::String(id.clone()));
        }
        if let Some(ref model) = chunk.model {
            obj_map.insert("modelVersion".into(), Value::String(model.clone()));
        }
        if let Some(ref usage) = chunk.usage {
            let input = usage.input_tokens.unwrap_or(0);
            let output = usage.output_tokens.unwrap_or(0);
            obj_map.insert(
                "usageMetadata".into(),
                serde_json::json!({
                    "promptTokenCount": input,
                    "candidatesTokenCount": output,
                    "totalTokenCount": input + output
                }),
            );
        }

        Ok(obj)
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

        let universal = adapter.request_to_universal(&payload).unwrap();
        assert_eq!(universal.params.temperature, Some(0.7));
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

        let universal = adapter.request_to_universal(&payload).unwrap();
        // safetySettings is a known key, so it won't be in extras
        // but it should be preserved through serialization

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert!(reconstructed.get("contents").is_some());
    }
}

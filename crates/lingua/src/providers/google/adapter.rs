/*!
Google AI provider adapter for GenerateContent API.

Google's API has some unique characteristics:
- Uses `contents` instead of `messages`
- Generation params are in `generationConfig` object
- Uses camelCase field names (e.g., `maxOutputTokens`)
- Streaming is endpoint-based, not parameter-based
*/

use crate::capabilities::ProviderFormat;
use crate::error::ConvertError;
use crate::processing::adapters::ProviderAdapter;
use crate::processing::transform::TransformError;
use crate::providers::google::detect::try_parse_google;
use crate::providers::google::generated::{
    Content as GoogleContent, GenerationConfig, ThinkingConfig,
};
use crate::providers::google::params::GoogleParams;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::Message;
use crate::universal::tools::{UniversalTool, UniversalToolType};
use crate::universal::{
    extract_system_messages, flatten_consecutive_messages, FinishReason, UniversalParams,
    UniversalRequest, UniversalResponse, UniversalStreamChoice, UniversalStreamChunk,
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
        let (temperature, top_p, top_k, max_tokens, stop, reasoning) =
            if let Some(config) = &typed_params.generation_config {
                let max_tokens = config.max_output_tokens.map(|t| t as i64);
                // Convert Google's thinkingConfig to ReasoningConfig
                // thinkingBudget: 0 means disabled
                let reasoning = config.thinking_config.as_ref().map(|tc| {
                    let is_disabled = tc.thinking_budget == Some(0);
                    crate::universal::ReasoningConfig {
                        enabled: Some(!is_disabled),
                        budget_tokens: tc.thinking_budget.map(|b| b as i64),
                        ..Default::default()
                    }
                });
                // Generated type has stop_sequences as Vec<String>, convert to Option
                let stop = if config.stop_sequences.is_empty() {
                    None
                } else {
                    Some(config.stop_sequences.clone())
                };
                (
                    config.temperature.map(|t| t as f64),
                    config.top_p.map(|p| p as f64),
                    config.top_k.map(|k| k as i64),
                    max_tokens,
                    stop,
                    reasoning,
                )
            } else {
                (None, None, None, None, None, None)
            };

        let mut params = UniversalParams {
            temperature,
            top_p,
            top_k,
            max_tokens,
            stop,
            tools: typed_params.tools.and_then(|t| {
                // Google uses [{functionDeclarations: [{name, description, parameters}]}]
                // Parse into UniversalTools
                let value = serde_json::to_value(&t).ok()?;
                let tools_arr = value.as_array()?;

                let mut universal_tools = Vec::new();
                for tool_group in tools_arr {
                    if let Some(func_decls) = tool_group.get("functionDeclarations") {
                        if let Some(decls) = func_decls.as_array() {
                            for decl in decls {
                                let name = decl.get("name").and_then(|v| v.as_str())?;
                                let description = decl
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .map(String::from);
                                let parameters = decl.get("parameters").cloned();

                                universal_tools.push(UniversalTool::function(
                                    name,
                                    description,
                                    parameters,
                                    None,
                                ));
                            }
                        }
                    }
                }

                if universal_tools.is_empty() {
                    // Fallback: store as builtin for unknown format
                    Some(vec![UniversalTool::builtin(
                        "google_tools",
                        "google",
                        "unknown",
                        Some(value),
                    )])
                } else {
                    Some(universal_tools)
                }
            }),
            tool_choice: None, // Google uses different mechanism
            response_format: None,
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
        let has_params = req.params.temperature.is_some()
            || req.params.top_p.is_some()
            || req.params.top_k.is_some()
            || req.params.max_tokens.is_some()
            || req.params.stop.is_some()
            || has_reasoning;

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
                    thinking_budget: Some(budget as i32),
                })
            });

            // Generated type has stop_sequences as Vec<String>, not Option
            let stop_sequences = req.params.stop.clone().unwrap_or_default();

            let config = GenerationConfig {
                temperature: req.params.temperature.map(|t| t as f32),
                top_p: req.params.top_p.map(|p| p as f32),
                top_k: req.params.top_k.map(|k| k as i32),
                max_output_tokens: req.params.max_tokens.map(|t| t as i32),
                stop_sequences,
                thinking_config,
                ..Default::default()
            };

            obj.insert(
                "generationConfig".into(),
                serde_json::to_value(config)
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
            );
        }

        // Add tools if present
        // Google uses functionDeclarations format: [{name, description, parameters}]
        if let Some(tools) = &req.params.tools {
            // First check for Google builtins (pass through original config)
            let mut google_builtin_found = false;
            for tool in tools {
                if let UniversalToolType::Builtin {
                    provider, config, ..
                } = &tool.tool_type
                {
                    if provider == "google" {
                        if let Some(config_value) = config {
                            obj.insert("tools".into(), config_value.clone());
                            google_builtin_found = true;
                            break;
                        }
                    }
                }
            }

            // If no Google builtin, convert function tools to Google format
            if !google_builtin_found {
                let function_declarations: Vec<serde_json::Value> = tools
                    .iter()
                    .filter_map(|tool| {
                        if tool.is_function() {
                            Some(serde_json::json!({
                                "name": tool.name,
                                "description": tool.description,
                                "parameters": tool.parameters.clone().unwrap_or(serde_json::json!({}))
                            }))
                        } else {
                            None
                        }
                    })
                    .collect();

                if !function_declarations.is_empty() {
                    obj.insert(
                        "tools".into(),
                        serde_json::json!([{"functionDeclarations": function_declarations}]),
                    );
                }
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

    fn detect_response(&self, payload: &Value) -> bool {
        // Google response has candidates[].content structure
        payload
            .get("candidates")
            .and_then(Value::as_array)
            .is_some_and(|arr| arr.first().and_then(|c| c.get("content")).is_some())
    }

    fn response_to_universal(&self, payload: Value) -> Result<UniversalResponse, TransformError> {
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
                    finish_reason =
                        Some(reason.parse().map_err(|_| ConvertError::InvalidEnumValue {
                            type_name: "FinishReason",
                            value: reason.to_string(),
                        })?);
                }
            }
        }

        let usage = UniversalUsage::extract_from_response(&payload, self.format());

        Ok(UniversalResponse {
            model: payload
                .get("modelVersion")
                .and_then(Value::as_str)
                .map(String::from),
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
            map.insert(
                "usageMetadata".into(),
                usage.to_provider_value(self.format()),
            );
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

        // Extract usage from usageMetadata
        let usage = UniversalUsage::extract_from_response(&payload, self.format());

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

                // Map finish reason to Google format
                let finish_reason = c.finish_reason.as_ref().map(|r| match r.as_str() {
                    "stop" => "STOP",
                    "length" => "MAX_TOKENS",
                    "tool_calls" => "TOOL_CALLS",
                    "content_filter" => "SAFETY",
                    other => other,
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
            map.insert(
                "usageMetadata".into(),
                usage.to_provider_value(self.format()),
            );
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

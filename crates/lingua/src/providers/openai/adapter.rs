/*!
OpenAI Chat Completions API adapter.

This module provides the `OpenAIAdapter` for the standard Chat Completions API,
along with target-specific transformation utilities for providers like Azure,
Vertex, and Mistral.
*/

use crate::capabilities::ProviderFormat;
use crate::error::ConvertError;

use crate::processing::adapters::{
    insert_opt_bool, insert_opt_f64, insert_opt_i64, insert_opt_value, ProviderAdapter,
};
use crate::processing::transform::TransformError;
use crate::providers::openai::capabilities::apply_model_transforms;
use crate::providers::openai::convert::{
    ChatCompletionRequestMessageExt, ChatCompletionResponseMessageExt,
};
use crate::providers::openai::params::OpenAIChatParams;
use crate::providers::openai::tool_parsing::parse_openai_chat_tools_array;
use crate::providers::openai::try_parse_openai;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::Message;
use crate::universal::reasoning::effort_to_budget;
use crate::universal::request::{ReasoningConfig, ReasoningEffort};
use crate::universal::tools::tools_to_openai_chat_value;
use crate::universal::{
    parse_stop_sequences, UniversalParams, UniversalRequest, UniversalResponse,
    UniversalStreamChoice, UniversalStreamChunk, UniversalUsage, PLACEHOLDER_ID, PLACEHOLDER_MODEL,
};
use std::convert::TryInto;

/// Adapter for OpenAI Chat Completions API.
pub struct OpenAIAdapter;

impl ProviderAdapter for OpenAIAdapter {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::ChatCompletions
    }

    fn directory_name(&self) -> &'static str {
        "chat-completions"
    }

    fn display_name(&self) -> &'static str {
        "ChatCompletions"
    }

    fn detect_request(&self, payload: &Value) -> bool {
        try_parse_openai(payload).is_ok()
    }

    fn request_to_universal(&self, payload: Value) -> Result<UniversalRequest, TransformError> {
        // Parse params (messages will be parsed separately to preserve reasoning field)
        let typed_params: OpenAIChatParams = serde_json::from_value(payload.clone())
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Extract and parse messages as extended type to capture reasoning field
        let messages_val = payload
            .get("messages")
            .ok_or_else(|| {
                TransformError::ToUniversalFailed("OpenAI: missing 'messages' field".to_string())
            })?
            .as_array()
            .ok_or_else(|| {
                TransformError::ToUniversalFailed("OpenAI: 'messages' must be an array".to_string())
            })?;

        let provider_messages: Vec<ChatCompletionRequestMessageExt> = messages_val
            .iter()
            .map(|msg_val| {
                serde_json::from_value(msg_val.clone())
                    .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let messages = <Vec<Message> as TryFromLLM<Vec<_>>>::try_from(provider_messages)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Extract max_tokens first - needed for reasoning budget computation
        let max_tokens = typed_params
            .max_tokens
            .or(typed_params.max_completion_tokens);

        // Build ReasoningConfig from all reasoning-related fields
        // Priority: reasoning_enabled: false takes precedence, then reasoning_budget, then reasoning_effort
        let reasoning = build_reasoning_config(
            typed_params.reasoning_enabled,
            typed_params.reasoning_budget,
            typed_params.reasoning_effort,
            max_tokens,
        );

        // Build canonical params from typed fields
        let mut params = UniversalParams {
            temperature: typed_params.temperature,
            top_p: typed_params.top_p,
            top_k: None, // OpenAI doesn't support top_k
            max_tokens,
            stop: typed_params.stop.as_ref().and_then(parse_stop_sequences),
            tools: typed_params
                .tools
                .as_ref()
                .map(parse_openai_chat_tools_array),
            tool_choice: typed_params
                .tool_choice
                .as_ref()
                .and_then(|v| (ProviderFormat::ChatCompletions, v).try_into().ok()),
            response_format: typed_params
                .response_format
                .as_ref()
                .and_then(|v| (ProviderFormat::ChatCompletions, v).try_into().ok()),
            seed: typed_params.seed,
            presence_penalty: typed_params.presence_penalty,
            frequency_penalty: typed_params.frequency_penalty,
            stream: typed_params.stream,
            // New canonical fields
            parallel_tool_calls: typed_params.parallel_tool_calls,
            reasoning,
            metadata: typed_params.metadata,
            store: typed_params.store,
            service_tier: typed_params.service_tier,
            logprobs: typed_params.logprobs,
            top_logprobs: typed_params.top_logprobs,
            extras: Default::default(),
        };

        // Sync parallel_tool_calls with tool_choice.disable_parallel for roundtrip fidelity
        // OpenAI uses parallel_tool_calls at params level, Anthropic uses tool_choice.disable_parallel
        if params.parallel_tool_calls == Some(false) {
            if let Some(ref mut tc) = params.tool_choice {
                if tc.disable_parallel.is_none() {
                    tc.disable_parallel = Some(true);
                }
            }
        }

        // Collect provider-specific extras for round-trip preservation
        // This includes both unknown fields (from serde flatten) and known OpenAI fields
        // that aren't part of UniversalParams
        let mut extras_map: Map<String, Value> = typed_params.extras.into_iter().collect();

        // Add OpenAI-specific known fields that aren't in UniversalParams
        if let Some(user) = typed_params.user {
            extras_map.insert("user".into(), Value::String(user));
        }
        if let Some(n) = typed_params.n {
            extras_map.insert("n".into(), Value::Number(n.into()));
        }
        if let Some(logit_bias) = typed_params.logit_bias {
            extras_map.insert("logit_bias".into(), logit_bias);
        }
        if let Some(stream_options) = typed_params.stream_options {
            extras_map.insert("stream_options".into(), stream_options);
        }
        if let Some(prediction) = typed_params.prediction {
            extras_map.insert("prediction".into(), prediction);
        }
        if let Some(safety_identifier) = typed_params.safety_identifier {
            extras_map.insert("safety_identifier".into(), Value::String(safety_identifier));
        }
        if let Some(prompt_cache_key) = typed_params.prompt_cache_key {
            extras_map.insert("prompt_cache_key".into(), Value::String(prompt_cache_key));
        }

        if !extras_map.is_empty() {
            params
                .extras
                .insert(ProviderFormat::ChatCompletions, extras_map);
        }

        Ok(UniversalRequest {
            model: typed_params.model,
            messages,
            params,
        })
    }

    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
        let model = req.model.as_ref().ok_or(TransformError::ValidationFailed {
            target: ProviderFormat::ChatCompletions,
            reason: "missing model".to_string(),
        })?;

        let openai_messages: Vec<ChatCompletionRequestMessageExt> =
            <Vec<ChatCompletionRequestMessageExt> as TryFromLLM<Vec<Message>>>::try_from(
                req.messages.clone(),
            )
            .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

        let mut obj = Map::new();
        obj.insert("model".into(), Value::String(model.clone()));
        obj.insert(
            "messages".into(),
            serde_json::to_value(openai_messages)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
        );

        insert_opt_f64(&mut obj, "temperature", req.params.temperature);
        insert_opt_f64(&mut obj, "top_p", req.params.top_p);
        insert_opt_i64(&mut obj, "max_completion_tokens", req.params.max_tokens);
        // Output stop sequences as array (OpenAI accepts both string and array)
        if let Some(ref stop) = req.params.stop {
            if !stop.is_empty() {
                obj.insert(
                    "stop".into(),
                    Value::Array(stop.iter().map(|s| Value::String(s.clone())).collect()),
                );
            }
        }
        // Convert tools to OpenAI Chat format
        if let Some(tools) = &req.params.tools {
            if let Some(tools_value) = tools_to_openai_chat_value(tools)? {
                obj.insert("tools".into(), tools_value);
            }
        }
        // Use helper methods to reduce boilerplate
        insert_opt_value(
            &mut obj,
            "tool_choice",
            req.params.tool_choice_for(ProviderFormat::ChatCompletions),
        );
        insert_opt_value(
            &mut obj,
            "response_format",
            req.params
                .response_format_for(ProviderFormat::ChatCompletions),
        );
        insert_opt_i64(&mut obj, "seed", req.params.seed);
        insert_opt_f64(&mut obj, "presence_penalty", req.params.presence_penalty);
        insert_opt_f64(&mut obj, "frequency_penalty", req.params.frequency_penalty);
        insert_opt_bool(&mut obj, "logprobs", req.params.logprobs);
        insert_opt_i64(&mut obj, "top_logprobs", req.params.top_logprobs);
        insert_opt_bool(&mut obj, "stream", req.params.stream);

        // Add parallel_tool_calls from canonical params
        if let Some(parallel) = req.params.parallel_tool_calls {
            obj.insert("parallel_tool_calls".into(), Value::Bool(parallel));
        }

        // Add reasoning_effort from canonical params
        if let Some(effort_value) = req.params.reasoning_for(ProviderFormat::ChatCompletions) {
            obj.insert("reasoning_effort".into(), effort_value);
        }

        // Add metadata from canonical params
        if let Some(metadata) = req.params.metadata.as_ref() {
            obj.insert("metadata".into(), metadata.clone());
        }

        // Add store from canonical params
        if let Some(store) = req.params.store {
            obj.insert("store".into(), Value::Bool(store));
        }

        // Add service_tier from canonical params
        if let Some(ref service_tier) = req.params.service_tier {
            obj.insert("service_tier".into(), Value::String(service_tier.clone()));
        }

        // If streaming, ensure stream_options.include_usage is set for usage reporting
        if req.params.stream == Some(true) {
            let stream_options = obj
                .entry("stream_options")
                .or_insert_with(|| serde_json::json!({}));
            if let Value::Object(opts) = stream_options {
                opts.insert("include_usage".into(), Value::Bool(true));
            }
        }

        // Merge back provider-specific extras (only for OpenAI)
        if let Some(extras) = req.params.extras.get(&ProviderFormat::ChatCompletions) {
            for (k, v) in extras {
                obj.insert(k.clone(), v.clone());
            }
        }

        apply_model_transforms(model, &mut obj);

        Ok(Value::Object(obj))
    }

    fn detect_response(&self, payload: &Value) -> bool {
        // OpenAI chat completion response has choices[].message and object="chat.completion"
        payload.get("choices").and_then(Value::as_array).is_some()
            && payload
                .get("object")
                .and_then(Value::as_str)
                .is_some_and(|o| o == "chat.completion")
    }

    fn response_to_universal(&self, payload: Value) -> Result<UniversalResponse, TransformError> {
        let choices = payload
            .get("choices")
            .and_then(Value::as_array)
            .ok_or_else(|| TransformError::ToUniversalFailed("missing choices".to_string()))?;

        let mut messages = Vec::new();
        let mut finish_reason = None;

        for choice in choices {
            if let Some(msg_val) = choice.get("message") {
                // Deserialize to extended type to capture reasoning field
                let response_msg: ChatCompletionResponseMessageExt =
                    serde_json::from_value(msg_val.clone())
                        .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                let universal =
                    <Message as TryFromLLM<ChatCompletionResponseMessageExt>>::try_from(
                        response_msg,
                    )
                    .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                messages.push(universal);
            }

            // Get finish_reason from first choice
            if finish_reason.is_none() {
                if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
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
                .get("model")
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
            .unwrap_or_else(|| "stop".to_string());

        let choices: Vec<Value> = resp
            .messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                // Use extended type to include reasoning field in output
                let response_msg =
                    <ChatCompletionResponseMessageExt as TryFromLLM<&Message>>::try_from(msg)
                        .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

                let message_value = serde_json::to_value(&response_msg)
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "index": i,
                    "message": message_value,
                    "finish_reason": finish_reason
                }))
            })
            .collect::<Result<Vec<_>, TransformError>>()?;

        let usage = resp
            .usage
            .as_ref()
            .map(|u| u.to_provider_value(self.format()));

        let mut map = serde_json::Map::new();
        map.insert(
            "id".into(),
            Value::String(format!("chatcmpl-{}", PLACEHOLDER_ID)),
        );
        map.insert("object".into(), Value::String("chat.completion".into()));
        map.insert("created".into(), serde_json::json!(0));
        map.insert(
            "model".into(),
            Value::String(resp.model.as_deref().unwrap_or(PLACEHOLDER_MODEL).into()),
        );
        map.insert("choices".into(), Value::Array(choices));

        if let Some(usage_val) = usage {
            map.insert("usage".into(), usage_val);
        }

        Ok(Value::Object(map))
    }

    // =========================================================================
    // Streaming response handling
    // =========================================================================

    fn detect_stream_response(&self, payload: &Value) -> bool {
        // OpenAI streaming chunk has object="chat.completion.chunk"
        // or has choices array with delta field
        if let Some(obj) = payload.get("object").and_then(Value::as_str) {
            if obj == "chat.completion.chunk" {
                return true;
            }
        }

        // Fallback: check for choices with delta
        if let Some(choices) = payload.get("choices").and_then(Value::as_array) {
            if choices.iter().any(|c| c.get("delta").is_some()) {
                return true;
            }
        }

        false
    }

    fn stream_to_universal(
        &self,
        payload: Value,
    ) -> Result<Option<UniversalStreamChunk>, TransformError> {
        // OpenAI is the canonical format, so this is mostly direct mapping
        let choices = payload
            .get("choices")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .map(|c| {
                        let index = c.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;
                        let delta = c.get("delta").cloned();
                        let finish_reason = c
                            .get("finish_reason")
                            .and_then(Value::as_str)
                            .map(String::from);
                        UniversalStreamChoice {
                            index,
                            delta,
                            finish_reason,
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Extract usage if present (usually only on final chunk)
        let usage = UniversalUsage::extract_from_response(&payload, self.format());

        Ok(Some(UniversalStreamChunk::new(
            payload.get("id").and_then(Value::as_str).map(String::from),
            payload
                .get("model")
                .and_then(Value::as_str)
                .map(String::from),
            choices,
            payload.get("created").and_then(Value::as_u64),
            usage,
        )))
    }

    fn stream_from_universal(&self, chunk: &UniversalStreamChunk) -> Result<Value, TransformError> {
        // Convert back to OpenAI streaming format
        if chunk.is_keep_alive() {
            // Return empty chunk for keep-alive
            return Ok(serde_json::json!({
                "object": "chat.completion.chunk",
                "choices": []
            }));
        }

        let choices: Vec<Value> = chunk
            .choices
            .iter()
            .map(|c| {
                let mut choice_map = Map::new();
                choice_map.insert("index".into(), serde_json::json!(c.index));
                choice_map.insert(
                    "delta".into(),
                    c.delta.clone().unwrap_or(Value::Object(Map::new())),
                );
                let finish_reason_val = match &c.finish_reason {
                    Some(reason) => Value::String(reason.clone()),
                    None => Value::Null,
                };
                choice_map.insert("finish_reason".into(), finish_reason_val);
                Value::Object(choice_map)
            })
            .collect();

        let mut map = Map::new();
        map.insert(
            "object".into(),
            Value::String("chat.completion.chunk".into()),
        );
        map.insert("choices".into(), Value::Array(choices));

        if let Some(ref id) = chunk.id {
            map.insert("id".into(), Value::String(id.clone()));
        }
        if let Some(ref model) = chunk.model {
            map.insert("model".into(), Value::String(model.clone()));
        }
        if let Some(created) = chunk.created {
            map.insert("created".into(), Value::Number(created.into()));
        }
        if let Some(ref usage) = chunk.usage {
            map.insert(
                "usage".into(),
                usage.to_provider_value(ProviderFormat::ChatCompletions),
            );
        }

        Ok(Value::Object(map))
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

use crate::providers::openai::generated::ReasoningEffort as OpenAIReasoningEffort;

/// Build ReasoningConfig from OpenAI reasoning-related fields.
///
/// Priority:
/// - `reasoning_enabled: false` OR `reasoning_budget: 0` → disabled
/// - `reasoning_budget: N` (N > 0) → enabled with explicit budget
/// - `reasoning_effort` → enabled with computed budget from effort level
/// - `reasoning_enabled: true` (without budget/effort) → enabled with default
fn build_reasoning_config(
    reasoning_enabled: Option<bool>,
    reasoning_budget: Option<i64>,
    reasoning_effort: Option<OpenAIReasoningEffort>,
    max_tokens: Option<i64>,
) -> Option<ReasoningConfig> {
    use crate::universal::reasoning::budget_to_effort;
    use crate::universal::ReasoningCanonical;

    if reasoning_enabled.is_none() && reasoning_budget.is_none() && reasoning_effort.is_none() {
        return None;
    }

    let is_disabled = reasoning_enabled == Some(false) || reasoning_budget == Some(0);

    if is_disabled {
        return Some(ReasoningConfig {
            enabled: Some(false),
            effort: None,
            budget_tokens: None,
            canonical: None,
            ..Default::default()
        });
    }

    let (effort, budget_tokens, canonical) = if let Some(budget) = reasoning_budget {
        let derived_effort = budget_to_effort(budget, max_tokens);
        (
            Some(derived_effort),
            Some(budget),
            Some(ReasoningCanonical::BudgetTokens),
        )
    } else if let Some(openai_effort) = reasoning_effort {
        let universal_effort = match openai_effort {
            OpenAIReasoningEffort::Low | OpenAIReasoningEffort::Minimal => ReasoningEffort::Low,
            OpenAIReasoningEffort::Medium => ReasoningEffort::Medium,
            OpenAIReasoningEffort::High => ReasoningEffort::High,
        };
        let derived_budget = effort_to_budget(universal_effort, max_tokens);
        (
            Some(universal_effort),
            Some(derived_budget),
            Some(ReasoningCanonical::Effort),
        )
    } else {
        let default_effort = ReasoningEffort::Medium;
        let derived_budget = effort_to_budget(default_effort, max_tokens);
        (
            Some(default_effort),
            Some(derived_budget),
            Some(ReasoningCanonical::Effort),
        )
    };

    Some(ReasoningConfig {
        enabled: Some(true),
        effort,
        budget_tokens,
        canonical,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_openai_detect_request() {
        let adapter = OpenAIAdapter;
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(adapter.detect_request(&payload));
    }

    #[test]
    fn test_openai_passthrough() {
        let adapter = OpenAIAdapter;
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let universal = adapter.request_to_universal(payload).unwrap();
        assert_eq!(universal.model, Some("gpt-4".to_string()));
        assert_eq!(universal.messages.len(), 1);

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert_eq!(reconstructed.get("model").unwrap(), "gpt-4");
        assert!(reconstructed.get("messages").is_some());
    }

    #[test]
    fn test_openai_preserves_extras() {
        let adapter = OpenAIAdapter;
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}],
            "user": "test-user-123",
            "custom_field": "should_be_preserved"
        });

        let universal = adapter.request_to_universal(payload).unwrap();
        let openai_extras = universal
            .params
            .extras
            .get(&ProviderFormat::ChatCompletions)
            .expect("should have OpenAI extras");
        assert!(openai_extras.contains_key("user"));
        assert!(openai_extras.contains_key("custom_field"));

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert_eq!(reconstructed.get("user").unwrap(), "test-user-123");
        assert_eq!(
            reconstructed.get("custom_field").unwrap(),
            "should_be_preserved"
        );
    }

    #[test]
    fn test_openai_reasoning_roundtrip() {
        use crate::universal::message::{AssistantContent, AssistantContentPart, TextContentPart};

        let adapter = OpenAIAdapter;

        // Create universal request with reasoning content
        let universal = UniversalRequest {
            model: Some("gpt-4".to_string()),
            messages: vec![
                Message::User {
                    content: crate::universal::message::UserContent::String("Hello".to_string()),
                },
                Message::Assistant {
                    content: AssistantContent::Array(vec![
                        AssistantContentPart::Reasoning {
                            text: "Let me think about this...".to_string(),
                            encrypted_content: None,
                        },
                        AssistantContentPart::Text(TextContentPart {
                            text: "OK".to_string(),
                            provider_options: None,
                        }),
                    ]),
                    id: None,
                },
                Message::User {
                    content: crate::universal::message::UserContent::String("Thanks".to_string()),
                },
            ],
            params: Default::default(),
        };

        // Convert universal to ChatCompletions format
        let openai_json = adapter.request_from_universal(&universal).unwrap();

        // Verify reasoning field is in the JSON output
        let messages = openai_json.get("messages").unwrap().as_array().unwrap();
        let assistant_msg = &messages[1];
        eprintln!(
            "Assistant message JSON: {}",
            serde_json::to_string_pretty(assistant_msg).unwrap()
        );

        assert!(
            assistant_msg.get("reasoning").is_some(),
            "Assistant message should have reasoning field. Got: {}",
            serde_json::to_string_pretty(assistant_msg).unwrap()
        );
        assert_eq!(
            assistant_msg.get("reasoning").unwrap().as_str().unwrap(),
            "Let me think about this..."
        );

        // Now convert back to universal and verify reasoning is preserved
        let universal2 = adapter.request_to_universal(openai_json.clone()).unwrap();

        // Check that reasoning is preserved in universal format
        let msg = &universal2.messages[1];
        match msg {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    let reasoning_part = parts
                        .iter()
                        .find(|p| matches!(p, AssistantContentPart::Reasoning { .. }));
                    assert!(
                        reasoning_part.is_some(),
                        "Should have reasoning part after roundtrip. Got: {:?}",
                        parts
                    );
                }
                _ => panic!("Expected Array content, got {:?}", content),
            },
            _ => panic!("Expected Assistant message, got {:?}", msg),
        }
    }

    #[test]
    fn test_openai_reasoning_only_roundtrip() {
        // Test case like Responses API where assistant message only has reasoning, no text
        use crate::universal::message::{AssistantContent, AssistantContentPart};

        let adapter = OpenAIAdapter;

        // Create universal request with reasoning-only content (like from Responses API)
        let universal = UniversalRequest {
            model: Some("gpt-4".to_string()),
            messages: vec![
                Message::User {
                    content: crate::universal::message::UserContent::String("Hello".to_string()),
                },
                Message::Assistant {
                    content: AssistantContent::Array(vec![AssistantContentPart::Reasoning {
                        text: "Let me think...".to_string(),
                        encrypted_content: None,
                    }]),
                    id: None,
                },
            ],
            params: Default::default(),
        };

        // Convert universal to ChatCompletions format
        let openai_json = adapter.request_from_universal(&universal).unwrap();
        eprintln!(
            "Full OpenAI JSON: {}",
            serde_json::to_string_pretty(&openai_json).unwrap()
        );

        // Verify reasoning field is in the JSON output
        let messages = openai_json.get("messages").unwrap().as_array().unwrap();
        let assistant_msg = &messages[1];
        eprintln!(
            "Assistant message JSON: {}",
            serde_json::to_string_pretty(assistant_msg).unwrap()
        );

        assert!(
            assistant_msg.get("reasoning").is_some(),
            "Assistant message should have reasoning field. Got: {}",
            serde_json::to_string_pretty(assistant_msg).unwrap()
        );

        // Now convert back to universal and verify reasoning is preserved
        let universal2 = adapter.request_to_universal(openai_json.clone()).unwrap();
        eprintln!("Universal2 messages: {:?}", universal2.messages);

        // Check that reasoning is preserved in universal format
        let msg = &universal2.messages[1];
        match msg {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    eprintln!("Parts: {:?}", parts);
                    let reasoning_part = parts
                        .iter()
                        .find(|p| matches!(p, AssistantContentPart::Reasoning { .. }));
                    assert!(
                        reasoning_part.is_some(),
                        "Should have reasoning part after roundtrip. Got: {:?}",
                        parts
                    );
                }
                AssistantContent::String(s) => {
                    panic!("Expected Array content, got String: {:?}", s)
                }
            },
            _ => panic!("Expected Assistant message, got {:?}", msg),
        }
    }

    #[test]
    fn test_openai_empty_reasoning_roundtrip() {
        // Test case like Responses API where assistant message has empty reasoning summary
        use crate::universal::message::{AssistantContent, AssistantContentPart};

        let adapter = OpenAIAdapter;

        // Create universal request with empty reasoning content (like from Responses API with empty summary)
        let universal = UniversalRequest {
            model: Some("gpt-4".to_string()),
            messages: vec![
                Message::User {
                    content: crate::universal::message::UserContent::String("Hello".to_string()),
                },
                Message::Assistant {
                    content: AssistantContent::Array(vec![AssistantContentPart::Reasoning {
                        text: "".to_string(), // Empty reasoning
                        encrypted_content: None,
                    }]),
                    id: None,
                },
            ],
            params: Default::default(),
        };

        // Convert universal to ChatCompletions format
        let openai_json = adapter.request_from_universal(&universal).unwrap();

        // Verify reasoning field is in the JSON output (even if empty)
        let messages = openai_json.get("messages").unwrap().as_array().unwrap();
        let assistant_msg = &messages[1];

        assert!(
            assistant_msg.get("reasoning").is_some(),
            "Assistant message should have reasoning field (even if empty). Got: {}",
            serde_json::to_string_pretty(assistant_msg).unwrap()
        );

        // Now convert back to universal and verify reasoning is preserved
        let universal2 = adapter.request_to_universal(openai_json.clone()).unwrap();

        // Check that empty reasoning is preserved in universal format
        let msg = &universal2.messages[1];
        match msg {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    let reasoning_part = parts
                        .iter()
                        .find(|p| matches!(p, AssistantContentPart::Reasoning { .. }));
                    assert!(
                        reasoning_part.is_some(),
                        "Should have reasoning part after roundtrip (even if empty). Got: {:?}",
                        parts
                    );
                }
                AssistantContent::String(s) => {
                    panic!("Expected Array content, got String: {:?}", s)
                }
            },
            _ => panic!("Expected Assistant message, got {:?}", msg),
        }
    }

    // =========================================================================
    // Braintrust reasoning extension tests
    // =========================================================================

    #[test]
    fn test_build_reasoning_config_disabled() {
        // reasoning_enabled: false should result in disabled
        let config = build_reasoning_config(Some(false), None, None, None);
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.enabled, Some(false));
    }

    #[test]
    fn test_build_reasoning_config_budget_zero_disabled() {
        // reasoning_budget: 0 should result in disabled
        let config = build_reasoning_config(None, Some(0), None, None);
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.enabled, Some(false));
    }

    #[test]
    fn test_build_reasoning_config_budget_positive() {
        // reasoning_budget: 2000 should result in enabled with explicit budget
        let config = build_reasoning_config(None, Some(2000), None, None);
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.enabled, Some(true));
        assert_eq!(config.budget_tokens, Some(2000));
    }

    #[test]
    fn test_build_reasoning_config_effort_only() {
        // reasoning_effort: high should result in enabled with computed budget
        let config =
            build_reasoning_config(None, None, Some(OpenAIReasoningEffort::High), Some(4096));
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.enabled, Some(true));
        assert_eq!(config.budget_tokens, Some(3072)); // 75% of 4096
    }

    #[test]
    fn test_build_reasoning_config_budget_overrides_effort() {
        // reasoning_budget should take precedence over reasoning_effort
        let config = build_reasoning_config(
            None,
            Some(5000),
            Some(OpenAIReasoningEffort::Low),
            Some(4096),
        );
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.enabled, Some(true));
        assert_eq!(config.budget_tokens, Some(5000)); // Not the computed effort budget
    }

    #[test]
    fn test_build_reasoning_config_enabled_true_budget_zero_disabled() {
        // reasoning_enabled: true with reasoning_budget: 0 should still be disabled
        // (budget: 0 takes precedence)
        let config = build_reasoning_config(Some(true), Some(0), None, None);
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.enabled, Some(false));
    }

    #[test]
    fn test_build_reasoning_config_none() {
        // No reasoning fields should result in None
        let config = build_reasoning_config(None, None, None, None);
        assert!(config.is_none());
    }

    #[test]
    fn test_openai_reasoning_enabled_false_to_anthropic() {
        // Full integration test: OpenAI request with reasoning_enabled: false
        // should produce Anthropic { type: "disabled" }
        let adapter = OpenAIAdapter;
        let payload = json!({
            "model": "claude-3-7-sonnet-20250219",
            "messages": [{"role": "user", "content": "Hello"}],
            "reasoning_enabled": false,
            "max_tokens": 100
        });

        let universal = adapter.request_to_universal(payload).unwrap();

        // Verify ReasoningConfig has enabled: false
        assert!(universal.params.reasoning.is_some());
        let reasoning = universal.params.reasoning.as_ref().unwrap();
        assert_eq!(reasoning.enabled, Some(false));

        // Verify the output for Anthropic
        let anthropic_thinking = universal.params.reasoning_for(ProviderFormat::Anthropic);
        assert!(anthropic_thinking.is_some());
        let thinking = anthropic_thinking.unwrap();
        assert_eq!(thinking.get("type").unwrap(), "disabled");
    }

    #[test]
    fn test_openai_reasoning_budget_to_anthropic() {
        // Full integration test: OpenAI request with reasoning_budget
        // should produce Anthropic { type: "enabled", budget_tokens: N }
        let adapter = OpenAIAdapter;
        let payload = json!({
            "model": "claude-3-7-sonnet-20250219",
            "messages": [{"role": "user", "content": "Hello"}],
            "reasoning_budget": 3000,
            "max_tokens": 100
        });

        let universal = adapter.request_to_universal(payload).unwrap();

        // Verify ReasoningConfig
        assert!(universal.params.reasoning.is_some());
        let reasoning = universal.params.reasoning.as_ref().unwrap();
        assert_eq!(reasoning.enabled, Some(true));
        assert_eq!(reasoning.budget_tokens, Some(3000));

        // Verify the output for Anthropic
        let anthropic_thinking = universal.params.reasoning_for(ProviderFormat::Anthropic);
        assert!(anthropic_thinking.is_some());
        let thinking = anthropic_thinking.unwrap();
        assert_eq!(thinking.get("type").unwrap(), "enabled");
        assert_eq!(thinking.get("budget_tokens").unwrap(), 3000);
    }

    // =========================================================================
    // Temperature stripping for reasoning models
    // =========================================================================

    #[test]
    fn test_openai_omits_temperature_for_reasoning_models() {
        use crate::universal::message::UserContent;

        let adapter = OpenAIAdapter;

        // gpt-5-mini is a reasoning model - temperature should be omitted
        let req = UniversalRequest {
            model: Some("gpt-5-mini".to_string()),
            messages: vec![Message::User {
                content: UserContent::String("Hello".to_string()),
            }],
            params: UniversalParams {
                temperature: Some(0.0), // User specified, but should be omitted
                ..Default::default()
            },
        };

        let result = adapter.request_from_universal(&req).unwrap();

        assert!(
            result.get("temperature").is_none(),
            "Temperature should be omitted for reasoning models (gpt-5-mini)"
        );
    }

    #[test]
    fn test_openai_preserves_temperature_for_non_reasoning_models() {
        use crate::universal::message::UserContent;

        let adapter = OpenAIAdapter;

        // gpt-4 is not a reasoning model - temperature should be preserved
        let req = UniversalRequest {
            model: Some("gpt-4".to_string()),
            messages: vec![Message::User {
                content: UserContent::String("Hello".to_string()),
            }],
            params: UniversalParams {
                temperature: Some(0.7),
                ..Default::default()
            },
        };

        let result = adapter.request_from_universal(&req).unwrap();

        assert_eq!(
            result.get("temperature").unwrap().as_f64().unwrap(),
            0.7,
            "Temperature should be preserved for non-reasoning models"
        );
    }
}

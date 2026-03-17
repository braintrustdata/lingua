/*!
Typed parameter structs for OpenAI APIs.

These structs use `#[serde(flatten)]` to automatically capture unknown fields,
eliminating the need for explicit KNOWN_KEYS arrays.
*/

use crate::capabilities::ProviderFormat;
use crate::providers::openai::generated::{
    ChatCompletionRequestMessage, Instructions, Reasoning, ReasoningEffort,
};
use crate::providers::openai::tool_parsing::parse_openai_responses_tools_array;
use crate::serde_json::{self, Map, Value};
use crate::universal::message::{Message, UserContent};
use crate::universal::request::{ResponseFormatConfig, ToolChoiceConfig};
use crate::universal::tools::tools_to_openai_chat_value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryInto;

/// OpenAI Chat Completions API request parameters.
///
/// All known fields are explicitly typed. Unknown fields automatically
/// go into `extras` via `#[serde(flatten)]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenAIChatParams {
    // === Core fields ===
    pub model: Option<String>,
    pub messages: Option<Vec<ChatCompletionRequestMessage>>,

    // === Sampling parameters ===
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub seed: Option<i64>,
    pub presence_penalty: Option<f64>,
    pub frequency_penalty: Option<f64>,

    // === Output control ===
    pub max_tokens: Option<i64>,
    pub max_completion_tokens: Option<i64>,
    pub stop: Option<Value>,
    pub n: Option<i64>,
    pub logprobs: Option<bool>,
    pub top_logprobs: Option<i64>,
    pub logit_bias: Option<Value>,

    // === Tools and function calling ===
    pub tools: Option<Value>,
    pub tool_choice: Option<Value>,
    pub parallel_tool_calls: Option<bool>,

    // === Response format ===
    pub response_format: Option<Value>,

    // === Streaming ===
    pub stream: Option<bool>,
    pub stream_options: Option<Value>,

    // === Reasoning (o-series models) ===
    pub reasoning_effort: Option<ReasoningEffort>,

    // === Reasoning (Braintrust proxy extensions) ===
    /// Explicitly enable/disable reasoning (Braintrust proxy extension)
    pub reasoning_enabled: Option<bool>,

    /// Token budget for reasoning (Braintrust proxy extension)
    pub reasoning_budget: Option<i64>,

    // === Metadata and identification ===
    pub metadata: Option<Value>,
    pub store: Option<bool>,
    pub service_tier: Option<String>,
    pub user: Option<String>,
    pub safety_identifier: Option<String>,
    pub prompt_cache_key: Option<String>,

    // === Prediction ===
    pub prediction: Option<Value>,

    /// Unknown fields - automatically captured by serde flatten.
    /// These are provider-specific fields not in the canonical set.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Value>,
}

/// OpenAI Responses API request parameters.
///
/// The Responses API has different field names and structure than Chat Completions.
/// All known fields are explicitly typed. Unknown fields automatically
/// go into `extras` via `#[serde(flatten)]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenAIResponsesParams {
    // === Core fields ===
    pub model: Option<String>,
    pub input: Option<Instructions>,
    pub instructions: Option<String>,

    // === Sampling parameters ===
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,

    // === Output control ===
    pub max_output_tokens: Option<i64>,
    pub top_logprobs: Option<i64>,

    // === Tools and function calling ===
    pub tools: Option<Value>,
    pub tool_choice: Option<Value>,
    pub parallel_tool_calls: Option<bool>,

    // === Text/Response format (nested structure) ===
    pub text: Option<Value>,

    // === Streaming ===
    pub stream: Option<bool>,

    // === Reasoning configuration (nested structure) ===
    pub reasoning: Option<Reasoning>,

    // === Context management ===
    pub truncation: Option<Value>,

    // === Metadata and identification ===
    pub metadata: Option<Value>,
    pub store: Option<bool>,
    pub service_tier: Option<String>,
    pub user: Option<String>,
    pub safety_identifier: Option<String>,
    pub prompt_cache_key: Option<String>,

    /// Unknown fields - automatically captured by serde flatten.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Value>,
}

/// Typed view over `UniversalParams.extras[ChatCompletions]` used during
/// universal -> OpenAI Chat reconstruction.
///
/// This is intentionally a partial/loose view:
/// - Extras may contain only a subset of fields.
/// - Values must be preserved as raw JSON when roundtripping.
/// - Generated OpenAPI request types are too strict for this use case because
///   they require full requests (`model`, `messages`, etc.).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct OpenAIChatExtrasView {
    pub messages: Option<Value>,
    pub stop: Option<Value>,
    pub tools: Option<Value>,
    pub tool_choice: Option<Value>,
    pub response_format: Option<Value>,
    pub reasoning_effort: Option<Value>,
    pub max_tokens: Option<Value>,
    pub max_completion_tokens: Option<Value>,
}

/// Typed view over `UniversalParams.extras[Responses]` used during universal
/// -> OpenAI Responses reconstruction.
///
/// This is intentionally not the full generated request type for the same
/// reasons as `OpenAIChatExtrasView`: extras are partial and preserve raw JSON.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct OpenAIResponsesExtrasView {
    pub instructions: Option<String>,
    pub input: Option<Value>,
    pub temperature: Option<Value>,
    pub top_p: Option<Value>,
    pub max_output_tokens: Option<Value>,
    pub top_logprobs: Option<Value>,
    pub stream: Option<Value>,
    pub tools: Option<Value>,
    pub tool_choice: Option<Value>,
    pub text: Option<Value>,
    pub reasoning: Option<Value>,
    pub parallel_tool_calls: Option<Value>,
    pub metadata: Option<Value>,
    pub store: Option<Value>,
    pub service_tier: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct OpenAIResponsesMetadataFingerprint {
    pub object: Option<String>,
    pub id: Option<String>,
    pub tool_choice: Option<Value>,
    pub parallel_tool_calls: Option<Value>,
    pub service_tier: Option<Value>,
    pub top_logprobs: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct OpenAIResponsesTextView {
    pub verbosity: Option<Value>,
    pub format: Option<Value>,
}

fn parse_metadata_object(metadata: &Value) -> Option<Map<String, Value>> {
    match metadata {
        Value::String(metadata_json) => {
            serde_json::from_str::<Map<String, Value>>(metadata_json).ok()
        }
        Value::Object(map) => Some(map.clone()),
        _ => None,
    }
}

fn is_openai_responses_metadata(
    metadata: &Map<String, Value>,
    extras: &OpenAIResponsesExtrasView,
) -> bool {
    let fingerprint: OpenAIResponsesMetadataFingerprint =
        serde_json::from_value(Value::Object(metadata.clone())).unwrap_or_default();

    if fingerprint.object.as_deref() == Some("response") {
        return true;
    }

    if fingerprint
        .id
        .as_deref()
        .is_some_and(|id| id.starts_with("resp_"))
    {
        return true;
    }

    if extras
        .tools
        .as_ref()
        .is_some_and(|tools| !parse_openai_responses_tools_array(tools).is_empty())
    {
        return true;
    }

    extras.instructions.is_some()
        || fingerprint.tool_choice.is_some()
        || fingerprint.parallel_tool_calls.is_some()
        || fingerprint.service_tier.is_some()
        || fingerprint.top_logprobs.is_some()
}

pub(crate) fn extract_openai_responses_metadata_view(
    metadata: &Value,
) -> Option<(Map<String, Value>, OpenAIResponsesExtrasView)> {
    let metadata_object = parse_metadata_object(metadata)?;
    let extras =
        serde_json::from_value::<OpenAIResponsesExtrasView>(Value::Object(metadata_object.clone()))
            .ok()?;

    if !is_openai_responses_metadata(&metadata_object, &extras) {
        return None;
    }

    Some((metadata_object, extras))
}

pub(crate) fn try_system_message_from_openai_metadata(metadata: &Value) -> Option<Message> {
    let (_, extras) = extract_openai_responses_metadata_view(metadata)?;
    let instructions = extras.instructions?;
    if instructions.is_empty() {
        return None;
    }

    Some(Message::System {
        content: UserContent::String(instructions),
    })
}

pub(crate) fn normalize_openai_responses_metadata_for_chat_completions(
    metadata: &Value,
) -> Option<Value> {
    let (mut normalized, extras) = extract_openai_responses_metadata_view(metadata)?;

    if let Some(tools) = extras.tools.as_ref() {
        let parsed_tools = parse_openai_responses_tools_array(tools);
        if let Ok(Some(chat_tools)) = tools_to_openai_chat_value(&parsed_tools) {
            normalized.insert("tools".into(), chat_tools);
        }
    }

    if let Some(tool_choice) = extras.tool_choice.as_ref() {
        if let Ok(config) = <(ProviderFormat, &Value) as TryInto<ToolChoiceConfig>>::try_into((
            ProviderFormat::Responses,
            tool_choice,
        )) {
            if let Ok(Some(chat_tool_choice)) =
                config.to_provider(ProviderFormat::ChatCompletions, None)
            {
                normalized.insert("tool_choice".into(), chat_tool_choice);
            }
        }
    }

    let max_output_tokens = extras
        .max_output_tokens
        .as_ref()
        .and_then(|value| serde_json::from_value::<i64>(value.clone()).ok());

    if let Some(reasoning_value) = extras.reasoning.as_ref() {
        if let Ok(reasoning) = serde_json::from_value::<Reasoning>(reasoning_value.clone()) {
            let config =
                crate::universal::request::ReasoningConfig::from((&reasoning, max_output_tokens));
            if let Ok(Some(Value::String(reasoning_effort))) =
                config.to_provider(ProviderFormat::ChatCompletions, max_output_tokens)
            {
                normalized.insert("reasoning_effort".into(), Value::String(reasoning_effort));
            }
        }
    }

    if let Some(text_value) = extras.text.as_ref() {
        if let Ok(text) = serde_json::from_value::<OpenAIResponsesTextView>(text_value.clone()) {
            if let Some(verbosity) = text.verbosity {
                normalized.insert("verbosity".into(), verbosity);
            }

            if let Some(format) = text.format {
                if let Ok(config) =
                    <(ProviderFormat, &Value) as TryInto<ResponseFormatConfig>>::try_into((
                        ProviderFormat::Responses,
                        &format,
                    ))
                {
                    if let Ok(Some(response_format)) =
                        config.to_provider(ProviderFormat::ChatCompletions)
                    {
                        normalized.insert("response_format".into(), response_format);
                        normalized.remove("text");
                    }
                }
            }
        } else {
            return Some(Value::Object(normalized));
        }
    }

    Some(Value::Object(normalized))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::openai::generated::ReasoningEffort;
    use crate::serde_json;
    use crate::serde_json::json;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct NormalizedChatMetadataView {
        instructions: Option<String>,
        tools: Option<Value>,
        tool_choice: Option<Value>,
        response_format: Option<Value>,
        reasoning_effort: Option<ReasoningEffort>,
        verbosity: Option<String>,
        text: Option<Value>,
    }

    #[test]
    fn test_chat_params_known_fields() {
        let json = json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Hello"}],
            "temperature": 0.7,
            "max_tokens": 100
        });

        let params: OpenAIChatParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.model, Some("gpt-4o".to_string()));
        assert_eq!(params.temperature, Some(0.7));
        assert_eq!(params.max_tokens, Some(100));
        assert!(params.extras.is_empty());
    }

    #[test]
    fn test_chat_params_unknown_fields_go_to_extras() {
        let json = json!({
            "model": "gpt-4o",
            "messages": [],
            "some_future_param": "value",
            "another_unknown": 42
        });

        let params: OpenAIChatParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.model, Some("gpt-4o".to_string()));
        assert_eq!(params.extras.len(), 2);
        assert_eq!(
            params.extras.get("some_future_param"),
            Some(&Value::String("value".to_string()))
        );
        assert_eq!(
            params.extras.get("another_unknown"),
            Some(&Value::Number(42.into()))
        );
    }

    #[test]
    fn test_responses_params_known_fields() {
        let json = json!({
            "model": "gpt-5-nano",
            "input": [{"role": "user", "content": "Hello"}],
            "instructions": "Be helpful",
            "max_output_tokens": 500,
            "reasoning": {"effort": "medium"}
        });

        let params: OpenAIResponsesParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.model, Some("gpt-5-nano".to_string()));
        assert_eq!(params.instructions, Some("Be helpful".to_string()));
        assert_eq!(params.max_output_tokens, Some(500));
        assert!(params.extras.is_empty());
    }

    #[test]
    fn test_roundtrip_preserves_extras() {
        let json = json!({
            "model": "gpt-4o",
            "messages": [],
            "custom_field": {"nested": "data"}
        });

        let params: OpenAIChatParams = serde_json::from_value(json.clone()).unwrap();
        let back: Value = serde_json::to_value(&params).unwrap();

        // Custom field should be preserved
        assert_eq!(back.get("custom_field"), json.get("custom_field"));
    }

    #[test]
    fn test_normalize_openai_responses_metadata_for_chat_completions() {
        let metadata = json!({
            "object": "response",
            "id": "resp_123",
            "instructions": "Be helpful",
            "tools": [{
                "type": "function",
                "name": "lookup_weather",
                "description": "Find weather",
                "parameters": { "type": "object" },
                "strict": true
            }],
            "tool_choice": {
                "type": "function",
                "name": "lookup_weather"
            },
            "reasoning": {
                "effort": "high"
            },
            "text": {
                "verbosity": "low",
                "format": {
                    "type": "json_schema",
                    "name": "forecast",
                    "schema": { "type": "object" },
                    "strict": true
                }
            }
        });

        let normalized =
            normalize_openai_responses_metadata_for_chat_completions(&metadata).unwrap();
        let normalized: NormalizedChatMetadataView = serde_json::from_value(normalized).unwrap();

        assert_eq!(normalized.instructions.as_deref(), Some("Be helpful"));
        assert_eq!(normalized.reasoning_effort, Some(ReasoningEffort::High));
        assert_eq!(normalized.verbosity.as_deref(), Some("low"));
        assert_eq!(
            normalized.response_format,
            Some(json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "forecast",
                    "schema": { "type": "object" },
                    "strict": true
                }
            }))
        );
        assert_eq!(
            normalized.tool_choice,
            Some(json!({
                "type": "function",
                "function": {
                    "name": "lookup_weather"
                }
            }))
        );
        assert_eq!(
            normalized.tools,
            Some(json!([{
                "type": "function",
                "function": {
                    "name": "lookup_weather",
                    "description": "Find weather",
                    "parameters": { "type": "object" },
                    "strict": true
                }
            }]))
        );
        assert_eq!(normalized.text, None);
    }

    #[test]
    fn test_normalize_openai_responses_metadata_requires_responses_fingerprint() {
        let metadata = json!({
            "braintrust": { "integration_name": "langchain-py" },
            "reasoning": { "effort": "medium" },
            "text": { "verbosity": "high" },
            "tools": [{
                "type": "function",
                "function": {
                    "name": "already_normalized"
                }
            }]
        });

        assert!(normalize_openai_responses_metadata_for_chat_completions(&metadata).is_none());
    }
}

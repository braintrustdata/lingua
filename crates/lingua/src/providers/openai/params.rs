/*!
Typed parameter structs for OpenAI APIs.

These structs use `#[serde(flatten)]` to automatically capture unknown fields,
eliminating the need for explicit KNOWN_KEYS arrays.
*/

use crate::providers::openai::generated::{
    ChatCompletionRequestMessage, Instructions, Reasoning, ReasoningEffort,
};
use crate::serde_json::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json;
    use crate::serde_json::json;

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
}
